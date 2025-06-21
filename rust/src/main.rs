mod piecewise_linear;
use piecewise_linear::*;
mod config;
use config::*;

use std::{process, thread, time};

use ftdi_embedded_hal as hal;

mod tsl2591;
use tsl2591::TSL2591;

struct MonitorState {
    bus: u32,
    curve: PiecewiseLinear,

    brightness: u32,
}

impl MonitorState {
    fn for_bus(bus: u32, curve: PiecewiseLinear) -> Self {
        MonitorState {
            bus,
            curve,
            brightness: 0,
        }
    }
}

fn set_brightness(monitor: &mut MonitorState, pct: u32) -> Result<(), anyhow::Error> {
    let pct = pct.clamp(0, 100);
    let res = process::Command::new("ddcutil")
        .args([
            "--bus",
            &monitor.bus.to_string(),
            "setvcp",
            "10",
            &pct.to_string(),
        ])
        .status()?;

    if !res.success() {
        anyhow::bail!("Got unexpected return from ddcutil: {res:?}")
    }

    monitor.brightness = pct;
    Ok(())
}

/// Update monitor brightness for the given lux value.
///
/// Returns true if brightness was changed, false otherwise.
fn update_brightness(monitor: &mut MonitorState, lux: u32) -> Result<bool, anyhow::Error> {
    let target = monitor.curve.eval(lux);
    let cur = monitor.brightness;
    let change = target as i32 - cur as i32;

    let new_b;
    if i32::abs(change) <= 1 {
        new_b = target;
    } else {
        new_b = if target > cur { cur + 1 } else { cur - 1 };
    }

    if new_b != cur {
        println!("lux={lux}, target={target}, setting={new_b}");
        set_brightness(monitor, new_b)?;

        Ok(true)
    } else {
        Ok(false)
    }
}

fn main() -> Result<(), anyhow::Error> {
    // Connect to the device
    let device = ftdi::find_by_vid_pid(0x0403, 0x6014)
        .interface(ftdi::Interface::A)
        .open()?;
    let i2c = hal::FtHal::init_default(device)?.i2c()?;
    let mut sensor = TSL2591::from_i2c(i2c)?;

    // VendorId = 0x0403
    // ProductId = 0x6014
    // Description = USB <-> Serial Converter
    // SerialNumber = FTA3Q3CS

    // TODO read brightness curve from a config file
    let curve = PiecewiseLinear::from_steps(vec![(0, 10), (250, 100)]).unwrap();
    let mut monitors: Vec<MonitorState> = vec![MonitorState::for_bus(6, curve)];

    // // ... this depends on `ddca_get_display_info_list`, which is not present in the library :/ (it has version2 instead)
    // // see `ldconfig -p` to find dynamic library path, then `nm -D` to find the symbols in the library
    // // https://github.com/arcnmx/ddcutil-rs/issues/2
    // let displays = ddcutil::DisplayInfo::enumerate();
    // for d in &displays? {
    //     println!("{d:#?}")
    // }

    let lux = sensor.read_lux()? as u32;
    for m in &mut monitors {
        update_brightness(m, lux)?;
    }

    loop {
        let mut updated = false;

        for m in &mut monitors {
            updated = updated || update_brightness(m, lux)?;
        }

        thread::sleep(time::Duration::from_millis(if updated {
            100
        } else {
            5_000
        }));
    }
}

#[ignore]
#[test]
fn test_testing() {
    panic!()
}
