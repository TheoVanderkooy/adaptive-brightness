use std::{process, thread, time};

use ftdi_embedded_hal as hal;

mod tsl2591;
use tsl2591::TSL2591;

mod brightness_curve;
use brightness_curve::*;

fn set_brightness(pct: u32) -> Result<(), anyhow::Error> {
    let pct = pct.clamp(0, 100);
    let res = process::Command::new("ddcutil")
        .args(["--bus=6", "setvcp", "10", &pct.to_string()])
        .status()?;
    if !res.success() {
        anyhow::bail!("Got unexpected return from ddcutil: {res:?}")
    }
    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    // Connect to the device
    let device: libftd2xx::Ft232h = libftd2xx::Ftdi::new()?.try_into()?;
    let i2c = hal::FtHal::init_default(device)?.i2c()?;
    let mut sensor = TSL2591::from_i2c(i2c)?;

    let curve = BrightnessCurve::from_steps(vec![(0, 10), (250, 100)]);

    // // ... this depends on `ddca_get_display_info_list`, which is not present in the library :/ (it has version2 instead)
    // // see `ldconfig -p` to find dynamic library path, then `nm -D` to find the symbols in the library
    // // https://github.com/arcnmx/ddcutil-rs/issues/2
    // let displays = ddcutil::DisplayInfo::enumerate();
    // for d in &displays? {
    //     println!("{d:#?}")
    // }

    let mut cur_b = curve.target_brightness(sensor.read_lux()? as u32);
    set_brightness(cur_b)?;

    loop {
        let lux = sensor.read_lux()? as u32;
        let target = curve.target_brightness(lux);
        let new_b = if i32::abs(target as i32 - cur_b as i32) <= 1 {
            target
        } else {
            (target + cur_b) / 2
        };
        cur_b = new_b;
        println!("lux={lux}, target={target}, setting={new_b}");
        set_brightness(new_b)?;
        thread::sleep(time::Duration::from_millis(5_000));
    }
}
