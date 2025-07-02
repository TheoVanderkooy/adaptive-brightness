/// Wrapper for a single monitor that handles updating its brightness and remembers its state.
use crate::piecewise_linear::PiecewiseLinear;

use ddc;

#[derive(Debug)]
pub struct MonitorState {
    display: ddc::Display,
    curve: PiecewiseLinear,
    brightness: u16,
}

impl MonitorState {
    /// Construct a `MonitorState` with the given brightness curve from a `DisplayInfo`.
    pub fn for_display(display: &ddc::DisplayInfo, curve: PiecewiseLinear) -> anyhow::Result<Self> {
        let display = ddc::Display::from_display_info(display)
            .map_err(|e| anyhow::anyhow!("{}", e.to_string()))?;

        Ok(MonitorState {
            display,
            curve,
            brightness: 0,
        })
    }

    /// Set monitor brightness to the given percentage unconditionally.
    fn set_brightness(&mut self, pct: u16) -> Result<(), anyhow::Error> {
        let pct = pct.clamp(0, 100);

        self.display
            .set_vcp_value(0x10, pct)
            .map_err(|e| anyhow::anyhow!("{}", e.to_string()))?;

        self.brightness = pct;
        Ok(())
    }

    /// Set monitor brightness based on the given lux value unconditionally. Used for initialization.
    pub fn set_brightness_for_lux(&mut self, lux: u32) -> Result<(), anyhow::Error> {
        let target = self.curve.eval(lux) as u16;
        self.set_brightness(target)
    }

    /// Update monitor brightness for the given lux value.
    ///
    /// Returns true if brightness was changed, false otherwise.
    pub fn update_brightness(&mut self, lux: u32) -> Result<bool, anyhow::Error> {
        let target = self.curve.eval(lux) as u16;
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
