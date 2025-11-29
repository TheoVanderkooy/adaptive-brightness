use std::os::unix::net::UnixListener as StdUnixListener;
use std::{os::fd::FromRawFd, time};

use anyhow::Context;
use ddc::ConvertToAnyhow;
use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::stream::StreamExt;
use smol::{LocalExecutor, Timer, lock::Mutex, net::unix::UnixListener};
use systemd::daemon::listen_fds;

use crate::{
    args::Args, get_config, get_displays, init_ddcutil, match_displays_to_config,
    monitor::MonitorState, piecewise_linear::PiecewiseLinear, sensor::Sensor,
};

pub(crate) struct DaemonState {
    pub lux: u32,
    pub monitors: Vec<MonitorState>,
}


/// Main daemon process:
/// periodically wakes up to update all monitors if the brightness has changed
async fn main_daemon_loop(mut sensor: Sensor, state: &Mutex<DaemonState>) -> anyhow::Result<()> {
    let mut iters_since_last_update = 0;

    // Main loop: periodically wake up to update all monitors
    loop {
        let mut updated = false;
        let lux;
        {
            lux = sensor.read_lux_async().await? as u32;
            let mut s = state.lock().await;
            s.lux = lux;

            for m in &mut s.monitors {
                updated = updated || m.update_brightness(lux)?;
            }
        }

        if updated {
            iters_since_last_update = 0;
        } else {
            iters_since_last_update += 1;
            if iters_since_last_update >= 100 {
                iters_since_last_update = 0;
                println!("lux={lux}");
            }
        }

        // Don't sleep as long if we may be off-target
        Timer::after(time::Duration::from_millis(if updated {
            100
        } else {
            5_000
        }))
        .await;
    }
}

/// Control loop:
/// Listens on the control socket and responds to external queries/commands
async fn control_loop<'a>(
    listener: &UnixListener,
    exec: &'a LocalExecutor<'a>,
    state: &'a Mutex<DaemonState>,
) -> anyhow::Result<()> {
    let mut incoming = listener.incoming();
    println!("control loop starting on {0:?}", listener.local_addr());

    while let Some(stream) = incoming.next().await {
        println!("getting a new connection...");

// TODO properly implement this :)

        match stream {
            Ok(mut stream) => {
                let t = exec.spawn(async move {
                    println!("new diagnostic connection");
                    let mut read_buf = [0];
                    loop {
                        match stream.read_exact(&mut read_buf).await {
                            Ok(_) => {
                                let lux = { state.lock().await.lux };
                                let mut buf = [0u8; 5];
                                buf[4] = b'\n';
                                buf[3] = b'0' + ((lux % 10) as u8);
                                buf[2] = b'0' + (((lux / 10) % 10) as u8);
                                buf[1] = b'0' + (((lux / 100) % 10) as u8);
                                buf[0] = b'0' + (((lux / 1000) % 10) as u8);
                                match stream.write_all(&buf).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        println!("Write error {e:?}");
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Read error {e:?}");
                                return;
                            }
                        }
                    }
                });
                t.detach();
            }
            Err(e) => println!("incoming connection error: {e:?}"),
        }
    }

    anyhow::bail!("control loop ended unexpectedly!");
}

/// Main daemon logic:
/// Collect config & monitors,
pub(crate) fn daemon_main(args: &Args) -> anyhow::Result<()> {
    init_ddcutil()?;

    // Open listener for the control socket, if applicable
    let control_listener = match &args.control_socket_path {
        // If a socket path was passed, bind it, removing it first if the file already exists
        // "not found" is fine, anything else means we didn't remove the file
        Some(p) => {
            match std::fs::remove_file(p) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                res => res?,
            }
            Some(UnixListener::bind(p)?)
        }
        // systemd uses $LISTEN_FDS to communicate how many sockets have been passed in
        // https://www.freedesktop.org/software/systemd/man/latest/systemd.socket.html
        None => {
            let lfds = listen_fds(false).with_context(|| "error checking for systemd sockets")?;
            match lfds.len() {
                0 => None,
                1 => Some(unsafe { StdUnixListener::from_raw_fd(3) }.try_into()?),
                _ => anyhow::bail!(
                    "unexpected number of sockets provided by systemd: {0:?}",
                    lfds.len()
                ),
            }
        }
    };

    loop {
        // Read in configuration, or load default configuration
        let config = get_config(args)?;
        println!("Loaded configuration: {config:?}");

        // Detect displays and match them up with configuration settings
        let displays = get_displays()?;
        let config_mapping = match_displays_to_config(&displays, &config)?;

        println!("Detected displays:");
        for (d, conf) in &config_mapping {
            print!(
                "    {0:<3}  {1:<13}  {2:<13}: ",
                d.manufacturer(),
                d.model(),
                d.serial_number()
            );
            match conf {
                None => println!("no matching config"),
                Some(mc) => println!("curve={0:?}", mc.curve),
            }
        }

        // Construct internal state for each device
        let monitors: Vec<MonitorState> = config_mapping
            .iter()
            .filter_map(|&(ref d, mc)| {
                // filter out monitors that don't match any config
                if let Some(mc) = mc {
                    Some((*d, mc))
                } else {
                    None
                }
            })
            .map(|(d, mc)| {
                // Open each display and build their state
                let curve = PiecewiseLinear::from_steps(mc.curve.clone()).ok_or_else(|| {
                    anyhow::anyhow!("Invalid brightness curve for monitor {0:?}", mc.identifier)
                })?;
// TODO need to remember the name of the display
                let d = ddc::Display::from_display_info(d).anyhow()?;

                Ok(MonitorState::for_display(d, curve))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Sanity check: if no monitors, there's nothing to do
        if monitors.len() < 1 {
            anyhow::bail!("no monitors detected matching any configuration values, exiting ...");
        }
// TODO consider "required" monitors

        // Connect to the brightness sensor
        let mut sensor = Sensor::open_async(&args.brightness_socket_path)?;

        let mut state = Mutex::new(DaemonState {
            lux: sensor.read_lux()? as u32,
            monitors: monitors,
        });

        // Set initial brightness based on current state
        let s = state.get_mut();
        for m in &mut s.monitors {
            m.set_brightness_for_lux(s.lux)?;
        }

        let exec = Box::leak(Box::new(LocalExecutor::new()));
        let t_main = main_daemon_loop(sensor, &state);

        if let Some(control_listener) = &control_listener {
            let t_control = exec.spawn(control_loop(control_listener, exec, &state));
            if let Err(e) = smol::block_on(exec.run(t_main)) {
                println!("Main loop got an error! {e:?}");
            }
            // after the main loop exits: cancel the control loop
            if let Some(Err(e)) = smol::block_on(t_control.cancel()) {
                println!("Error from control loop!: {e:?}");
            }
        } else {
            if let Err(e) = smol::block_on(t_main) {
                println!("Main loop got an error! {e:?}");
            }
        }
    }
}
