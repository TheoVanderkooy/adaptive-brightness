use std::env;
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::time::{Duration, SystemTime};

use smol::LocalExecutor;
use smol::lock::Mutex;
use smol::net::unix::{UnixListener, UnixStream};
use smol::prelude::*;

use ftdi_embedded_hal as hal;
use tsl2591::TSL2591;

/// Caching wrapper for tsl2591::TSL2591 that will remember the most recent value for up to 5s
struct CachedTsl2591 {
    tsl2591: TSL2591<ftdi_embedded_hal::I2c<ftdi::Device>>,
    cached_lux: f64,
    last_read: SystemTime,
}

impl CachedTsl2591 {
    /// Wrap a TSL2591 sensor with a cache of the most recent value (up to 5s)
    fn for_sensor(
        mut sensor: TSL2591<ftdi_embedded_hal::I2c<ftdi::Device>>,
    ) -> anyhow::Result<Self> {
        let lux = sensor.read_lux()?;
        let now = SystemTime::now();

        Ok(Self {
            tsl2591: sensor,
            cached_lux: lux,
            last_read: now,
        })
    }

    /// Get the current lux value.
    /// This is the cached value if last read <5s ago, else read the current value
    fn get_lux(&mut self) -> anyhow::Result<f64> {
        // update cached brightness if enough time has passed
        if self.last_read.elapsed()? > Duration::from_secs(5) {
            self.last_read = SystemTime::now();
            self.cached_lux = self.tsl2591.read_lux()?;
        }

        Ok(self.cached_lux)
    }
}

fn open_brightness_sensor() -> anyhow::Result<TSL2591<ftdi_embedded_hal::I2c<ftdi::Device>>> {
    let device = ftdi::find_by_vid_pid(0x0403, 0x6014)
        .interface(ftdi::Interface::A)
        .open()?;
    let i2c = hal::FtHal::init_default(device)?.i2c()?;
    let sensor = TSL2591::from_i2c(i2c)?;

    Ok(sensor)
}

async fn async_main<'a>(
    exec: &'a LocalExecutor<'a>,
    listener: UnixListener,
    sensor: &'a Mutex<CachedTsl2591>,
) -> anyhow::Result<()> {
    // TODO need to open/connect to brightness sensor here

    // also need:
    // mutex (async?)
    // maybe a cached value + time of update

    let mut incoming = listener.incoming();
    let mut i = 0;
    while let Some(stream) = incoming.next().await {
        i += 1;

        // TODO how to get errors back from the handler threads?
        exec.spawn(conn_handler(stream?, &sensor, i)).detach();
    }

    Ok(())
}

async fn conn_handler(mut stream: UnixStream, sensor: &Mutex<CachedTsl2591>, i: i32) {
    println!("got connection {i}");

    let mut read_buf = [0];
    while stream.read_exact(&mut read_buf).await.is_ok() {
        let lux = match sensor.lock().await.get_lux() {
            Ok(lux) => lux,
            Err(e) => {
                println!("Error reading lux! {e}");
                break;
            }
        };

        println!("got {read_buf:?},  current lux = {lux}");

        match stream.write_all(&lux.to_be_bytes()).await {
            Ok(()) => {}
            Err(e) => {
                println!("Error writing lux! {e}");
                break;
            }
        };
    }

    println!("connection {i} ended");
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
            if env::var("LISTEN_FDS") != Ok("3".to_string()) {
                anyhow::bail!(
                    "no socket path provided and unexpected value of LISTEN_FDS if managed by systemd"
                );
            }

            unsafe { StdUnixListener::from_raw_fd(3) }.try_into()?
        }
    };

    let sensor = open_brightness_sensor()?;
    let sensor = CachedTsl2591::for_sensor(sensor)?;
    let sensor = Mutex::new(sensor);

    let exec = Box::leak(Box::new(LocalExecutor::new()));
    let task = exec.spawn(async_main(&exec, listener, &sensor));
    smol::block_on(exec.run(task))
}
