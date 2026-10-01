/// Reprsents a LTR390 sensor with convenience methods to control & read from it over I2C.
///
/// Datasheet for the sensor: https://optoelectronics.liteon.com/upload/download/DS86-2015-0004/LTR-390UV_Final_%20DS_V1%201.pdf
use embedded_hal::i2c::{I2c, SevenBitAddress};

use core::time::Duration;

#[allow(unused)]
pub struct LTR390<I: I2c> {
    i2c: I,
    gain: config::Gain,
    resolution: config::Resolution,
    measure_rate: config::MeasureRate,
    window_factor: f32,
}

const I2C_ADDR: SevenBitAddress = 0x53;

#[allow(unused)]
pub mod register {
    pub const MAIN_CTRL: u8 = 0x00;

    pub const ALS_UVS_MEAS_RATE: u8 = 0x04;
    pub const ALS_UVS_GAIN: u8 = 0x05;
    pub const PART_ID: u8 = 0x06; // high bits should be 0b1011
    pub const MAIN_STATUS: u8 = 0x07;

    pub const ALS_DATA_0: u8 = 0x0d;
    pub const ALS_DATA_1: u8 = 0x0e;
    pub const ALS_DATA_2: u8 = 0x0f;
    pub const UVS_DATA_0: u8 = 0x10;
    pub const UVS_DATA_1: u8 = 0x11;
    pub const UVS_DATA_2: u8 = 0x12;
    // 0x13-0x18 are "resered"

    pub const INT_CFG: u8 = 0x19;
    pub const INT_PST: u8 = 0x1a;

    pub const ALS_UVS_THRES_UP_0: u8 = 0x21;
    pub const ALS_UVS_THRES_UP_1: u8 = 0x22;
    pub const ALS_UVS_THRES_UP_2: u8 = 0x23;
    pub const ALS_UVS_THRES_LOW_0: u8 = 0x24;
    pub const ALS_UVS_THRES_LOW_1: u8 = 0x25;
    pub const ALS_UVS_THRES_LOW_2: u8 = 0x26;
}

#[allow(unused)]
pub mod config {
    use core::time::Duration;

    // MAIN_CTRL register bits
    pub const SW_RESET_BIT: u8 = 1 << 4;
    pub const UVS_MODE_BIT: u8 = 1 << 3;
    pub const ENABLE_BIT: u8 = 1 << 1;

    // ALS_UVS_MEAS_RATE resolution options
    pub enum Resolution {
        Res20,
        Res19,
        Res18,
        Res17,
        Res16,
        Res13,
    }

    impl Resolution {
        /// Number of bits in the result
        pub fn bits(&self) -> u8 {
            match self {
                Resolution::Res20 => 20,
                Resolution::Res19 => 19,
                Resolution::Res18 => 18,
                Resolution::Res17 => 17,
                Resolution::Res16 => 16,
                Resolution::Res13 => 13,
            }
        }

        /// Conversion time (?)
        pub fn integration_time(&self) -> Duration {
            match self {
                Resolution::Res20 => Duration::from_millis(400),
                Resolution::Res19 => Duration::from_millis(200),
                Resolution::Res18 => Duration::from_millis(100),
                Resolution::Res17 => Duration::from_millis(500),
                Resolution::Res16 => Duration::from_millis(25),
                Resolution::Res13 => Duration::from_micros(12500),
            }
        }

        /// bits set in the register value
        pub fn reg_bits(&self) -> u8 {
            let base_val: u8 = match self {
                Resolution::Res20 => 0,
                Resolution::Res19 => 1,
                Resolution::Res18 => 2,
                Resolution::Res17 => 3,
                Resolution::Res16 => 4,
                Resolution::Res13 => 5,
            };

            // bits 6-4
            base_val << 4
        }

        /// multiplier in LUX calculation
        pub fn integration_factor(&self) -> f32 {
            match self {
                Resolution::Res20 => 4.0,
                Resolution::Res19 => 2.0,
                Resolution::Res18 => 1.0,
                Resolution::Res17 => 0.5,
                Resolution::Res16 => 0.25,
                Resolution::Res13 => 0.125, // TODO this one is not in the data sheet, not sure what it's supposed to be..
            }
        }
    }

    // ALS_UVS_MEAS_RATE measurement rate options
    pub enum MeasureRate {
        MeasureRate25,
        MeasureRate50,
        MeasureRate100,
        MeasureRate200,
        MeasureRate500,
        MeasureRate1000,
        MeasureRate2000,
    }

    impl MeasureRate {
        /// Period between measurements
        pub fn period(&self) -> Duration {
            match self {
                MeasureRate::MeasureRate25 => Duration::from_millis(25),
                MeasureRate::MeasureRate50 => Duration::from_millis(50),
                MeasureRate::MeasureRate100 => Duration::from_millis(100),
                MeasureRate::MeasureRate200 => Duration::from_millis(200),
                MeasureRate::MeasureRate500 => Duration::from_millis(500),
                MeasureRate::MeasureRate1000 => Duration::from_secs(1),
                MeasureRate::MeasureRate2000 => Duration::from_secs(2),
            }
        }

        /// bits set in the configuration register
        pub fn reg_bits(&self) -> u8 {
            match self {
                MeasureRate::MeasureRate25 => 0,
                MeasureRate::MeasureRate50 => 1,
                MeasureRate::MeasureRate100 => 2,
                MeasureRate::MeasureRate200 => 3,
                MeasureRate::MeasureRate500 => 4,
                MeasureRate::MeasureRate1000 => 5,
                MeasureRate::MeasureRate2000 => 6,
            }
        }
    }

    // ALS_UVS_GAIN
    pub enum Gain {
        GainX1,
        GainX3,
        GainX6,
        GainX9,
        GainX18,
    }

    impl Gain {
        /// Gain multiplier for lux conversion
        pub fn gain(&self) -> u8 {
            match self {
                Gain::GainX1 => 1,
                Gain::GainX3 => 3,
                Gain::GainX6 => 6,
                Gain::GainX9 => 9,
                Gain::GainX18 => 18,
            }
        }

        /// bit value used for device configuration
        pub fn reg_bits(&self) -> u8 {
            match self {
                Gain::GainX1 => 0,
                Gain::GainX3 => 1,
                Gain::GainX6 => 2,
                Gain::GainX9 => 3,
                Gain::GainX18 => 4,
            }
        }
    }

    // part ID should be 0b1011, low bits are the revision ID
    pub const PART_ID_LTR390: u8 = 0b1011 << 4;
    pub const PART_ID_PART_ID_MASK: u8 = 0xF0;
    pub const PART_ID_REVISION_MASK: u8 = 0x0F;

    // MAIN_STATUS:
    //  - power_on = bit 5
    //  - interrupt status = bit 4
    //  - data status = bit 3 (indicates if data has been read or not)

    // INT_CFG
    pub const INT_CFG_ENABLE: u8 = 1 << 2;
    pub const INT_CFG_ALS: u8 = 1 << 4;
    pub const INT_CFG_UVS: u8 = 3 << 4;

    // INT_PST: (N-1) << 4 for N consecutive values out of range to trigger an interrupt
    // Threshold for above stored in UVS_ALS_THRES
}

impl<I: I2c> LTR390<I> {
    pub fn from_i2c(mut i2c: I) -> Result<Self, anyhow::Error> {
        // Confirm part ID matches
        let res = Self::read8_from_i2c(&mut i2c, register::PART_ID)?;
        if (res & config::PART_ID_PART_ID_MASK) != config::PART_ID_LTR390 {
            anyhow::bail!("Expected LTR390 device ID = 0b1011, got {res:#b}");
        }

        let gain = config::Gain::GainX3;
        let resolution = config::Resolution::Res16;
        let meas_rate = config::MeasureRate::MeasureRate100;

        Self::write_to_i2c(
            &mut i2c,
            register::ALS_UVS_MEAS_RATE,
            resolution.reg_bits() | meas_rate.reg_bits(),
        )?;
        Self::write_to_i2c(&mut i2c, register::ALS_UVS_GAIN, gain.reg_bits())?;

        Self::write_to_i2c(&mut i2c, register::MAIN_CTRL, config::ENABLE_BIT)?;

        Ok(Self {
            i2c: i2c,
            gain: gain,
            window_factor: 1.0,
            resolution: resolution,
            measure_rate: meas_rate,
        })
    }

    fn read_from_i2c_with_retries(
        i2c: &mut I,
        register: u8,
        buf: &mut [u8],
    ) -> Result<(), anyhow::Error> {
        const RETRIES: u8 = 10;

        let mut res = i2c.write_read(I2C_ADDR, &[register], buf);
        for _ in 0..RETRIES {
            if res.is_ok() {
                return Ok(());
            }

            std::thread::sleep(Duration::from_millis(10));
            res = i2c.write_read(I2C_ADDR, &[register], buf);
        }

        res.map_err(|e| {
            anyhow::anyhow!(
                "I2C read failed after {RETRIES} retries! register={register:#x}, error={e:?}"
            )
        })?;
        Ok(())
    }

    fn read8_from_i2c(i2c: &mut I, register: u8) -> Result<u8, anyhow::Error> {
        let mut buf = [0u8, 1];
        Self::read_from_i2c_with_retries(i2c, register, &mut buf)?;
        Ok(buf[0])
    }

    fn write_to_i2c(i2c: &mut I, register: u8, val: u8) -> Result<(), anyhow::Error> {
        i2c.write(I2C_ADDR, &[register, val])
            .map_err(|e| anyhow::anyhow!("I2C write failed! register={register:#x}, error={e:?}"))
    }

    fn read8(&mut self, register: u8) -> Result<u8, anyhow::Error> {
        Self::read8_from_i2c(&mut self.i2c, register)
    }

    fn read_status(&mut self) -> Result<u8, anyhow::Error> {
        self.read8(register::MAIN_STATUS)
    }

    /// Check whether the sensor data is ready.
    /// Note: read_brightness waits for this before reading.
    pub fn is_ready(&mut self) -> Result<bool, anyhow::Error> {
        let status = self.read_status()?;

        Ok(status & (1 << 3) != 0)
    }

    /// Read the raw brightness value from the sensor
    pub fn read_brightness(&mut self) -> Result<u32, anyhow::Error> {
        const READY_RETRIES: u8 = 100;
        let mut buf = [0u8; 3];

        for _ in 0..READY_RETRIES {
            if self.is_ready()? {
                Self::read_from_i2c_with_retries(&mut self.i2c, register::ALS_DATA_0, &mut buf)?;
                let res = (buf[2] as u32) << 16 | (buf[1] as u32) << 8 | (buf[0] as u32);

                return Ok(res);
            }

            std::thread::sleep(Duration::from_millis(10));
        }

        Err(anyhow::anyhow!(
            "Failed to read brightness after {READY_RETRIES} retries"
        ))
    }

    /// Callculate lux based on the sensor reading and current device configuration
    pub fn read_lux(&mut self) -> Result<f64, anyhow::Error> {
        let res = 0.6 * (self.read_brightness()? as f64) * (self.window_factor as f64)
            / (self.gain.gain() as f64)
            / (self.resolution.integration_factor() as f64);

        Ok(res)
    }

    /*
    TODO:
     - UV accessors
     - settings updates
     - ...
    */

    /// Debugging: print out all the registers in the given range
    pub fn debug_read_all(&mut self, registers: impl Iterator<Item = u8>) {
        for i in registers {
            let res = self.read8(i);
            if let Ok(res) = res {
                println!("read8({i:x}): {res}   {res:x}");
            }
        }
    }
}
