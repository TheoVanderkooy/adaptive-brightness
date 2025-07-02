/// Wrapper for a single monitor that handles updating its brightness and remembers its state.
use crate::piecewise_linear::PiecewiseLinear;

use ddc;

#[derive(Debug)]
pub struct MonitorState {
    // Configuration
    display: ddc::Display,
    curve: PiecewiseLinear,

    // State
    target: u16,
    brightness: u16,
}

impl MonitorState {
    /// Brightness target is always rounded to a multiple of this constant.
    const ROUND_TO_NEAREST: u16 = 5;

    /// Construct a `MonitorState` with the given brightness curve from a `DisplayInfo`.
    pub fn for_display(display: &ddc::DisplayInfo, curve: PiecewiseLinear) -> anyhow::Result<Self> {
        let display = ddc::Display::from_display_info(display)
            .map_err(|e| anyhow::anyhow!("{}", e.to_string()))?;

        Ok(MonitorState {
            display,
            curve,
            target: 0,
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

    /// Calculate a change in brightness target.
    ///
    /// Round to the multiple of `ROUND_TO_NEAREST` that is closest to current value.
    ///
    /// This prevents the target from moving too erratically, reducing how often we make updates
    /// oscillating over a small range
    fn new_target_brightness(cur: u16, new: u16) -> u16 {
        if new == cur {
            new
        } else if new < cur {
            (new + Self::ROUND_TO_NEAREST - 1) / Self::ROUND_TO_NEAREST * Self::ROUND_TO_NEAREST
        } else {
            new / Self::ROUND_TO_NEAREST * Self::ROUND_TO_NEAREST
        }
    }

    /// Update monitor brightness for the given lux value.
    ///
    /// Returns true if brightness was changed, false otherwise.
    pub fn update_brightness(&mut self, lux: u32) -> Result<bool, anyhow::Error> {
        let cur = self.brightness;

        self.target = Self::new_target_brightness(cur, self.curve.eval(lux) as u16);
        let target = self.target;

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

#[test]
fn test_new_target_brightness() {
    for new in 46..=54 {
        assert_eq!(50, MonitorState::new_target_brightness(50, new));
    }
    assert_eq!(50, MonitorState::new_target_brightness(0, 54));
    assert_eq!(50, MonitorState::new_target_brightness(100, 46));
    assert_eq!(55, MonitorState::new_target_brightness(50, 55));
    assert_eq!(45, MonitorState::new_target_brightness(50, 45));

    for new in 51..=59 {
        assert_eq!(55, MonitorState::new_target_brightness(55, new));
    }
}
