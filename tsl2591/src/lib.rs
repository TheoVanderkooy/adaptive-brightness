/// Represents a TSL2591 sensor and provides convenience methods to control & read from it over I2C.
///
/// Datasheet for the sensor: https://cdn-shop.adafruit.com/datasheets/TSL25911_Datasheet_EN_v1.pdf
use embedded_hal::i2c::{I2c, SevenBitAddress};

pub struct TSL2591<I: I2c> {
    i2c: I,

    // config values:
    /// gain: default to "low gain" = field value 0b00 = 1x
    gain: u16,
    /// integration time: defaults to field value 0b000 = 100ms
    atime: u16,
}

const I2C_ADDR: SevenBitAddress = 0x29;

const COMMAND_BIT: u8 = 0xA0;

#[allow(unused)]
pub mod register {
    // Configuration registers
    pub const ENABLE: u8 = 0x00;
    pub const CONFIG: u8 = 0x01;

    // ALS interrupt related registers
    pub const AILTL: u8 = 0x04;
    pub const AILTH: u8 = 0x05;
    pub const AIHTL: u8 = 0x06;
    pub const AIHTH: u8 = 0x07;
    pub const NPAILTL: u8 = 0x08;
    pub const NPAILTH: u8 = 0x09;
    pub const NPAIHTL: u8 = 0x0A;
    pub const NPAIHTH: u8 = 0x0B;
    pub const PERSIST: u8 = 0x0C;

    // Status registers
    pub const PID: u8 = 0x11;
    pub const ID: u8 = 0x12;
    pub const STATUS: u8 = 0x13;

    // Data registers
    pub const CH0_LO: u8 = 0x14;
    pub const CH0_HI: u8 = 0x15;
    pub const CH1_LO: u8 = 0x16;
    pub const CH1_HI: u8 = 0x17;
}

pub mod config {
    pub const GAIN_LOW: u16 = 1;
    pub const GAIN_MED: u16 = 25;
    pub const GAIN_HIGH: u16 = 428;
    pub const GAIN_MAX: u16 = 9876;
}

impl<I: I2c> TSL2591<I> {
    pub fn from_i2c(mut i2c: I) -> Result<Self, anyhow::Error> {
        // Check the chip is what we expect
        let res = Self::read8_from_i2c(&mut i2c, register::ID)?;
        if res != 0x50 {
            anyhow::bail!("Expected TSL2591 device ID = 0x50, got {:#x}", res);
        }

        // Check how it's currently configured
        let config = Self::read8_from_i2c(&mut i2c, register::CONFIG)?;
        let gain = match config & 0b0011_0000 {
            0b0000_0000 => config::GAIN_LOW,
            0b0001_0000 => config::GAIN_MED,
            0b0010_0000 => config::GAIN_HIGH,
            0b0011_0000 => config::GAIN_MAX,
            _ => unreachable!(),
        };
        let atime = match config & 0b0000_0111 {
            val @ 0..=5 => 100 * val as u16 + 100,
            val => anyhow::bail!("unexpected integration time value {val}"),
        };

        // TODO it might make more sense to _write_ the configuration (& turn it on) instead

        Ok(TSL2591 {
            i2c: i2c,
            gain: gain,
            atime: atime,
        })
    }

    fn read8_from_i2c(i2c: &mut I, register: u8) -> Result<u8, anyhow::Error> {
        let mut buf = [0u8, 1];
        i2c.write_read(I2C_ADDR, &[COMMAND_BIT | register], &mut buf)
            .map_err(|e| anyhow::anyhow!("I2C read failed! register={register:#x}, error={e:?}"))?;
        Ok(buf[0])
    }

    #[allow(dead_code)]
    fn read8(&mut self, register: u8) -> Result<u8, anyhow::Error> {
        Self::read8_from_i2c(&mut self.i2c, register)
    }

    pub fn read_brightness(&mut self) -> Result<(u16, u16), anyhow::Error> {
        let mut buf = [0u8; 4];
        I2c::write_read(
            &mut self.i2c,
            I2C_ADDR,
            &[COMMAND_BIT | register::CH0_LO],
            &mut buf,
        )
        .map_err(|e| anyhow::anyhow!("I2C read of brighness failed! error={e:?}"))?;
        let ch0: u16 = (buf[1] as u16) << 8 | (buf[0] as u16);
        let ch1: u16 = (buf[3] as u16) << 8 | (buf[2] as u16);

        Ok((ch0, ch1))
    }

    /// Calculate lux from the values of the two sensor channels
    ///  - ch0 = "full spectrum"
    ///  - ch1 = ifrared
    ///
    /// See the datasheet for the responsiveness characteristics
    ///
    /// Based on the adafruit circuitpython library:
    ///     https://github.com/adafruit/Adafruit_CircuitPython_TSL2591/blob/main/adafruit_tsl2591.py
    /// which is in turn based on their arduino library:
    ///     https://github.com/adafruit/Adafruit_TSL2591_Library/blob/master/Adafruit_TSL2591.cpp
    fn calculate_lux(&self, ch0: u16, ch1: u16) -> f64 {
        let ch0 = ch0 as f64;
        let ch1 = ch1 as f64;
        let cpl = (self.atime as f64 * self.gain as f64) / 408.0;

        f64::max(ch0 - 1.64 * ch1, 0.59 * ch0 - 0.86 * ch1) / cpl
    }

    /// Read current brightness value from the sensor
    pub fn read_lux(&mut self) -> Result<f64, anyhow::Error> {
        let (ch0, ch1) = self.read_brightness()?;
        Ok(self.calculate_lux(ch0, ch1))
    }
}
