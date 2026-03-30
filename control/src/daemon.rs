use std::io::ErrorKind;
use std::net::SocketAddr;
use std::ops::Deref;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::sync::Arc;
use std::{os::fd::FromRawFd, time};

use anyhow::Context;
use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Response, StatusCode};
use axum::routing::get;
use common::DaemonStatus;
use ddc::ConvertToAnyhow;
use prometheus_client::encoding::text::encode as prometheus_encode;
use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::net::unix::UnixStream;
use smol::stream::StreamExt;
use smol::{Async, Executor};
use smol::{Timer, lock::Mutex, net::unix::UnixListener};
use systemd::daemon::listen_fds;

use crate::metrics::Metrics;
use crate::monitor::DisplayInfoDisplayName;
use crate::{
    args::Args, get_config, get_displays, init_ddcutil, match_displays_to_config,
    monitor::MonitorState, piecewise_linear::PiecewiseLinear, sensor::Sensor,
};

pub(crate) struct DaemonState {
    lux: u32,
    pub monitors: Vec<MonitorState>,
    pub unmanaged_monitors: Vec<String>,
    metrics: Metrics,
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

    /// Update the current lux.
    /// Using this wrapper is preferred over direct update as this also updates metrics.
    pub fn set_lux(&mut self, lux: u32) {
        self.lux = lux;
        self.metrics.brightness.set(lux);
    }
}

/// DaemonState isn't Send by default because of the void* fields from ddcutil.
/// I'm not sure about ddcutil's thread safety but we're only updating brightness from one task so hopefully this is fine.
unsafe impl Send for DaemonState {}

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

            // TODO can get "reset by peer" or "unexpected EOF" (see control_connection_handler)
            //  -- should do that handling inside sensor.read_lux[_async] with retries

            if let Err(e) = &ret {
                println!("error reading lux: {e:?}");
            }
            lux = ret? as u32;
            let mut s = state.lock().await;
            s.set_lux(lux);

            let s = &mut *s;
            let monitors = &mut s.monitors;
            let metrics = &s.metrics;

            for m in monitors {
                let did_update = m.update_brightness(lux)?;

                if did_update {
                    updated = true;

                    let ms = m.get_status();
                    metrics.set_monitor_brightness(ms.display_name, ms.brightness);
                }
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
        // TODO: `w`: stream stuff over the connection
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
    listener: UnixListener,
    exec: impl Deref<Target = Executor<'e>>,
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

/// Metric loop:
/// Serves prometheus/OpenMetrics metrics on the specified port
async fn metric_loop<'e>(
    port: u16,
    exec: Arc<Executor<'e>>,
    state: Arc<Mutex<DaemonState>>,
) -> anyhow::Result<()> {
    let listener = Async::<std::net::TcpListener>::bind(SocketAddr::from(([0, 0, 0, 0], port)))?;

    let app = Router::new().route(
        "/metrics",
        get(async move || {
            let mut buffer = String::new();

            {
                let s = state.lock().await;
                prometheus_encode(&mut buffer, &s.metrics.registry).unwrap();
            }

            Response::builder()
                .status(StatusCode::OK)
                .header(
                    CONTENT_TYPE,
                    "application/openmetrics-text; version=1.0.0; charset=utf-8",
                )
                .body(Body::from(buffer))
                .expect("Something went wrong sending metrics response!")
        }),
    );

    smol_axum::serve(exec, listener, app).await?;

    Ok(())
}

/// Main daemon logic:
/// Collect config & monitors,
pub(crate) fn daemon_main(args: &Args) -> anyhow::Result<()> {
    init_ddcutil()?;

    let metrics = Metrics::new();

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
            metrics,
        });

        // Set initial brightness based on current state
        let s = state.get_mut();
        s.metrics.brightness.set(s.lux);
        for m in &mut s.monitors {
            m.set_brightness_for_lux(s.lux)?;

            let ms = m.get_status();
            s.metrics
                .set_monitor_brightness(ms.display_name, ms.brightness);
        }

        let state = Arc::new(state);

        // Set up async machinery for the various "threads"
        // TODO smol_axum doesn't support LocalExecutor, but that would be nice..
        let exec = Arc::new(Executor::new());
        let t_main = main_daemon_loop(sensor, &state);

        // control socket handler
        let t_control =
            control_listener.map(|cl| exec.spawn(control_loop(cl, exec.clone(), &state)));

        // metric handler
        let t_metric = args
            .metric_port
            .map(|port| exec.spawn(metric_loop(port, exec.clone(), state.clone())));

        // wait for main thread to exit and shut everything else down
        smol::block_on(exec.run(t_main))?;
        if let Some(t_control) = t_control
            && let Some(Err(e)) = smol::block_on(t_control.cancel())
        {
            println!("Error from control loop!: {e:#?}");
        }

        if let Some(t_metric) = t_metric
            && let Some(Err(e)) = smol::block_on(t_metric.cancel())
        {
            println!("Error from metric loop!: {e:#?}");
        }

        // TODO figure out if this is needed. Have systemd restart the whole thing for now .. but would be nice to just restart the executor locally
        println!("restarting ...");
        anyhow::bail!("restart");
    }
}
