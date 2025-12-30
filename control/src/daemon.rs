use std::io::ErrorKind;
use std::ops::Deref;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::rc::Rc;
use std::{os::fd::FromRawFd, time};

use anyhow::Context;
use ddc::ConvertToAnyhow;
use serde::{Deserialize, Serialize};
use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::net::unix::UnixStream;
use smol::stream::StreamExt;
use smol::{LocalExecutor, Timer, lock::Mutex, net::unix::UnixListener};
use systemd::daemon::listen_fds;

use crate::monitor::{DisplayInfoDisplayName, MonitorStatus};
use crate::{
    args::Args, get_config, get_displays, init_ddcutil, match_displays_to_config,
    monitor::MonitorState, piecewise_linear::PiecewiseLinear, sensor::Sensor,
};

pub(crate) struct DaemonState {
    pub lux: u32,
    pub monitors: Vec<MonitorState>,
    pub unmanaged_monitors: Vec<String>,
}

/// A snapshot of the current status that can be serialized & shared with other clients.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DaemonStatus {
    pub lux: u32,
    pub monitors: Vec<MonitorStatus>,
    pub unmanaged_monitors: Vec<String>,
}

impl DaemonState {
    pub fn get_status(&self) -> DaemonStatus {
        let monitors = self.monitors.iter().map(|m| m.get_status()).collect();
        DaemonStatus {
            lux: self.lux,
            monitors,
            unmanaged_monitors: self.unmanaged_monitors.clone(),
        }
    }
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
            let ret = sensor.read_lux_async().await;
            if let Err(e) = &ret {
                println!("error reading lux: {e:?}");
            }
            lux = ret? as u32;
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

async fn control_connection_handler_inner(
    state: &Mutex<DaemonState>,
    command: u8,
    stream: &mut UnixStream,
) -> anyhow::Result<()> {
    match command {
        // `s`: serializes and sends over the current daemon state
        b's' => {
            let status = { state.lock().await.get_status() };

            let buf = serde_json::to_vec(&status).with_context(|| "error serializing status")?;

            stream
                .write_all(&buf)
                .await
                .with_context(|| "write error")?;
        }
        // TODO: `r`: restart the daemon
        // TODO: `p`: trigger panic?
        // TODO: other commands to e.g. reload config?
        _ => {
            anyhow::bail!("unknown command {command}");
        }
    }
    Ok(())
}

async fn control_connection_handler(cid: u32, state: &Mutex<DaemonState>, mut stream: UnixStream) {
    println!("[{cid}] new diagnostic connection");
    let mut read_buf = [0];
    loop {
        match stream.read_exact(&mut read_buf).await {
            Ok(_) => {
                let command = read_buf[0];
                if let Err(e) = control_connection_handler_inner(state, command, &mut stream).await
                {
                    println!("[{cid}] Error while handling command {command}: {e:?}");
                    return;
                }
            }
            Err(e) => {
                // Differentiate expected (stream closed) vs unexpected errors & end the task
                match e.kind() {
                    // Client closed the the connection after reading everything we sent
                    ErrorKind::UnexpectedEof
                    // Client closed the connection after reading only part of what we sent
                    | ErrorKind::ConnectionReset
                    => {
                        println!("[{cid}] Diagnostic connection closed")
                    }
                    // Anything else is unexpected
                    _ => println!("[{cid}] Read error `{e:?}`, closing connection"),
                };
                return;
            }
        }
    }
}

/// Control loop:
/// Listens on the control socket and responds to external queries/commands
async fn control_loop<'a, 'e>(
    listener: &'a UnixListener,
    exec: impl Deref<Target = LocalExecutor<'e>>,
    state: &'a Mutex<DaemonState>,
) -> anyhow::Result<()>
where
    'a: 'e,
{
    let mut incoming = listener.incoming();
    println!("control loop starting on {0:?}", listener.local_addr());

    let mut cid = 0u32;

    while let Some(stream) = incoming.next().await {
        println!("getting a new connection...");
        cid += 1;

        match stream {
            Ok(stream) => {
                let t = exec.spawn(control_connection_handler(cid, state, stream));
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

                let display_name = d.display_name();
                let d = ddc::Display::from_display_info(d).anyhow()?;

                Ok(MonitorState::for_display(d, display_name, curve))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Remember monitors we're not doing anything with, just for displaying via UI
        let unmanaged_monitors = config_mapping
            .iter()
            .filter_map(|&(ref d, mc)| {
                if let None = mc {
                    Some(d.display_name())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Sanity check: if no monitors, there's nothing to do
        if monitors.len() < 1 {
            anyhow::bail!("no monitors detected matching any configuration values, exiting ...");
        }
        // TODO consider "required" monitors

        // Connect to the brightness sensor
        let mut sensor = Sensor::open_async(&args.brightness_socket_path)?;

        let mut state = Mutex::new(DaemonState {
            lux: sensor.read_lux()? as u32,
            monitors,
            unmanaged_monitors,
        });

        // Set initial brightness based on current state
        let s = state.get_mut();
        for m in &mut s.monitors {
            m.set_brightness_for_lux(s.lux)?;
        }

        // Set up async machinery for the various "threads"
        let exec = Rc::new(LocalExecutor::new());
        let t_main = main_daemon_loop(sensor, &state);

        if let Some(control_listener) = &control_listener {
            let fc = control_loop(control_listener, exec.clone(), &state);
            let t_control = exec.spawn(fc);
            smol::block_on(exec.run(t_main))?;
            // after the main loop exits: cancel the control loop
            if let Some(Err(e)) = smol::block_on(t_control.cancel()) {
                println!("Error from control loop!: {e:?}");
            }
            println!("control loop done, restarting...");
        } else {
            if let Err(e) = smol::block_on(t_main) {
                println!("Main loop got an error! {e:?}");
            }
        }
    }
}
