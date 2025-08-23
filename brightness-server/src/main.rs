use std::env;
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::time::Duration;

use anyhow::Context;
use smol::LocalExecutor;
use smol::lock::Mutex;
use smol::net::unix::{UnixListener, UnixStream};
use smol::prelude::*;

use ftdi_embedded_hal as hal;
use tsl2591::{CachedTsl2591, TSL2591};

type CachedSensor = CachedTsl2591<ftdi_embedded_hal::I2c<ftdi::Device>>;

/// Open the brightness sensor
fn open_brightness_sensor() -> anyhow::Result<CachedSensor> {
    // vid/pid are for the FTDI device, there are a few others that could be used instead
    // hardcoded for now
    let device = ftdi::find_by_vid_pid(0x0403, 0x6014)
        .interface(ftdi::Interface::Any)
        .open()?;
    let i2c = hal::FtHal::init_default(device)?.i2c()?;
    let sensor = TSL2591::from_i2c(i2c)?;
    let sensor = CachedSensor::for_sensor(sensor, Duration::from_secs(4))?;

    Ok(sensor)
}

async fn async_main<'a>(
    exec: &'a LocalExecutor<'a>,
    listener: UnixListener,
    sensor: &'a Mutex<CachedSensor>,
) -> anyhow::Result<()> {
    let mut tasks = vec![];

    let mut incoming = listener.incoming();
    let mut i = 0;
    while let Some(stream) = incoming.next().await {
        i += 1;

        // Spawn a task for each inbound connection
        let t = exec.spawn(conn_handler(stream?, &sensor, i));
        tasks.push(t);

        // Every time we spawn a new task, go through and check if any previous tasks are done
        // This is mainly to propagate errors back. Note that errors won't show up right away, only on the next connection
        // TODO find a better way to do this
        let mut i = tasks.len();
        while i > 0 {
            i -= 1;
            if tasks[i].is_finished() {
                tasks.swap_remove(i).await?;
            }
        }
    }

    Ok(())
}

async fn conn_handler(
    mut stream: UnixStream,
    sensor: &Mutex<CachedSensor>,
    i: i32,
) -> anyhow::Result<()> {
    println!("got connection {i}");

    let mut read_buf = [0];
    while stream.read_exact(&mut read_buf).await.is_ok() {
        let lux = sensor
            .lock()
            .await
            .get_lux()
            .with_context(|| "error reading lux")?;

        println!("got {read_buf:?},  current lux = {lux}");

        stream
            .write_all(&lux.to_be_bytes())
            .await
            .with_context(|| "error writing response")?;
    }

    println!("connection {i} ended");

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = env::args();
    let args: Vec<_> = args.collect();
    let sock_path = args.get(1);

    // We either need a socket specified, or if launched by systemd a socket is transferred as a file descriptor
    let listener = match sock_path {
        Some(p) => {
            // Bind the socket, removing it first if the file already exists

            match std::fs::remove_file(p) {
                Ok(_) => {}
                // "not found" is fine, anything else means we didn't remove the file
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                res => res?,
            }
            UnixListener::bind(p)?
        }
        None => {
            // systemd uses this env var to communicate how many sockets
            // TODO: use systemd crate instead of doing this manually
            // https://docs.rs/systemd/latest/systemd/daemon/struct.ListenFds.html
            // https://www.freedesktop.org/software/systemd/man/latest/systemd.socket.html
            let listen_fds = env::var("LISTEN_FDS");
            if listen_fds != Ok("1".to_string()) {
                anyhow::bail!(
                    "no socket path provided and unexpected value of LISTEN_FDS if managed by systemd\nLISTEN_FDS: {0:?}",
                    listen_fds
                );
            }

            unsafe { StdUnixListener::from_raw_fd(3) }.try_into()?
        }
    };

    // open the sensor and wrap it with all the stuff we need
    let sensor = open_brightness_sensor()?;
    let sensor = Mutex::new(sensor);

    let exec = Box::leak(Box::new(LocalExecutor::new()));
    let task = exec.spawn(async_main(&exec, listener, &sensor));
    smol::block_on(exec.run(task))
}
