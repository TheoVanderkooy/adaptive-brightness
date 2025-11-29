use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::Path,
};

use smol::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::unix::UnixStream as AsyncUnixStream,
};

use tsl2591::TSL2591;

use ftdi_embedded_hal as hal;

/// Abstracted lux sensor
pub(crate) enum Sensor {
    Socket(UnixStream),
    AsyncSocket(AsyncUnixStream),
    Tsl2591(TSL2591<ftdi_embedded_hal::I2c<ftdi::Device>>),
}

impl Sensor {
    /// Open the physical sensor directly.
    fn open_tsl2591() -> anyhow::Result<Self> {
        // vid/pid are for the FTDI device, there are a few others that could be used instead
        // hardcoded for now
        let device = ftdi::find_by_vid_pid(0x0403, 0x6014)
            .interface(ftdi::Interface::Any)
            .open()?;
        let i2c = hal::FtHal::init_default(device)?.i2c()?;
        let sensor = TSL2591::from_i2c(i2c)?;

        Ok(Self::Tsl2591(sensor))
    }

    /// If specified, open the given socket to read brightness values.
    /// If unspecified, open the physical sensor directly.
    pub fn open<T: AsRef<Path>>(socket_path: &Option<T>) -> anyhow::Result<Self> {
        match socket_path {
            None => Self::open_tsl2591(),
            Some(socket_path) => Ok(Self::Socket(UnixStream::connect(socket_path)?)),
        }
    }

    /// If specified, open the given socket to read brightness values.
    /// If unspecified, open the physical sensor directly.
    pub fn open_async<T: AsRef<Path>>(socket_path: &Option<T>) -> anyhow::Result<Self> {
        match socket_path {
            None => Self::open_tsl2591(),
            Some(socket_path) => Ok(Self::AsyncSocket(smol::block_on(
                AsyncUnixStream::connect(socket_path),
            )?)),
        }
    }

    /// Read current lux value from the underlying sensor.
    pub fn read_lux(&mut self) -> anyhow::Result<f64> {
        match self {
            Sensor::Socket(stream) => {
                // Write a single byte to get lux as a response
                stream.write_all(&[0])?;

                // Read and decode response
                let mut buf = [0; 8];
                stream.read_exact(&mut buf)?;

                Ok(f64::from_be_bytes(buf))
            }
            Sensor::AsyncSocket(stream) => {
                smol::block_on(async {
                    // Write a single byte to get lux as a response
                    stream.write_all(&[0]).await?;

                    // Read and decode response
                    let mut buf = [0; 8];
                    stream.read_exact(&mut buf).await?;

                    Ok(f64::from_be_bytes(buf))
                })
            }
            Sensor::Tsl2591(sensor) => sensor.read_lux(),
        }
    }

    pub async fn read_lux_async(&mut self) -> anyhow::Result<f64> {
        match self {
            Sensor::Socket(stream) => {
                // Write a single byte to get lux as a response
                stream.write_all(&[0])?;

                // Read and decode response
                let mut buf = [0; 8];
                stream.read_exact(&mut buf)?;

                Ok(f64::from_be_bytes(buf))
            }
            Sensor::AsyncSocket(stream) => {
                // Write a single byte to get lux as a response
                stream.write_all(&[0]).await?;

                // Read and decode response
                let mut buf = [0; 8];
                stream.read_exact(&mut buf).await?;

                Ok(f64::from_be_bytes(buf))
            }
            Sensor::Tsl2591(sensor) => sensor.read_lux(),
        }
    }
}
