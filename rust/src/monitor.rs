/// Wrapper for a single monitor that handles updating its brightness and remembers its state.
use crate::piecewise_linear::PiecewiseLinear;

use std::process;

#[derive(Debug)]
pub struct MonitorState {
    bus: u32,
    curve: PiecewiseLinear,

    brightness: u32,
}

impl MonitorState {
    /// Construct a MonitorState with the given brightness curve on the specified I2C bus
    pub fn for_bus(bus: u32, curve: PiecewiseLinear) -> Self {
        MonitorState {
            bus,
            curve,
            brightness: 0,
        }
    }

    /// Set monitor brightness to the given percentage unconditionally.
    fn set_brightness(&mut self, pct: u32) -> Result<(), anyhow::Error> {
        let pct = pct.clamp(0, 100);
        // TODO: use libddcutil directly instead of calling the command-line process.
        let res = process::Command::new("ddcutil")
            .args([
                "--bus",
                &self.bus.to_string(),
                "setvcp",
                "10",
                &pct.to_string(),
            ])
            .status()?;

        if !res.success() {
            anyhow::bail!("Got unexpected return from ddcutil: {res:?}")
        }

        self.brightness = pct;
        Ok(())
    }

    /// Set monitor brightness based on the given lux value unconditionally. Used for initialization.
    pub fn set_brightness_for_lux(&mut self, lux: u32) -> Result<(), anyhow::Error> {
        let target = self.curve.eval(lux);
        self.set_brightness(target)
    }

    /// Update monitor brightness for the given lux value.
    ///
    /// Returns true if brightness was changed, false otherwise.
    pub fn update_brightness(&mut self, lux: u32) -> Result<bool, anyhow::Error> {
        let target = self.curve.eval(lux);
        let cur = self.brightness;
        let change = target as i32 - cur as i32;

        let new_b;
        if i32::abs(change) <= 1 {
            new_b = target;
        } else {
            new_b = if target > cur { cur + 1 } else { cur - 1 };
        }

        if new_b != cur {
            println!("lux={lux}, target={target}, setting={new_b}");
            self.set_brightness(new_b)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
