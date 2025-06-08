use std::{thread, time};

use ftdi_embedded_hal as hal;

mod tsl2591;
use tsl2591::TSL2591;

mod brightness_curve;
use brightness_curve::*;

// TODO monitor control -- libddcutil?

fn main() -> Result<(), anyhow::Error> {
    // Connect to the device
    let device: libftd2xx::Ft232h = libftd2xx::Ftdi::new()?.try_into()?;
    let i2c = hal::FtHal::init_default(device)?.i2c()?;
    let mut sensor = TSL2591::from_i2c(i2c)?;

    let curve = BrightnessCurve::from_steps(vec![(0, 10), (250, 100)]);

    // ... this depends on `ddca_get_display_info_list`, which is not present in the library :/ (it has version2 instead)
    // see `ldconfig -p` to find dynamic library path, then `nm -D` to find the symbols in the library
    // https://github.com/arcnmx/ddcutil-rs/issues/2
    let displays = ddcutil::DisplayInfo::enumerate()?;


    for d in &displays {
        println!("{d:#?}")
    }

    // ddcutil::Display::capabilities(&self);

    loop {
        let (ch0, ch1) = sensor.read_brightness()?;
        let lux = sensor.read_lux()? as u32;
        let b = curve.target_brightness(lux);
        println!("read: {ch0}, {ch1}, lux={lux}, target={b}");
        thread::sleep(time::Duration::from_millis(5_000));
    }
}
