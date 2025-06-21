// in-crate modules
mod config;
mod monitor;
mod piecewise_linear;
mod tsl2591;

// in-crate imports
use config::*;
use monitor::*;
use piecewise_linear::*;
use tsl2591::TSL2591;

// my libraries
use xdg_dirs::{dirs, xdg_location_of};

// STD
use std::{thread, time};

// 3rd party libraries
use ftdi_embedded_hal as hal;

const DEFAULT_CONFIG: &str = r#"
    (
    monitors: [
        (
            identifier: Bus(6),
            curve: [
                (0, 10),
                (250, 100),
            ],
        ),
    ]
    )
"#;

fn main() -> Result<(), anyhow::Error> {
    // // ... this depends on `ddca_get_display_info_list`, which is not present in the library :/ (it has version2 instead)
    // // see `ldconfig -p` to find dynamic library path, then `nm -D` to find the symbols in the library
    // // https://github.com/arcnmx/ddcutil-rs/issues/2
    // let displays = ddcutil::DisplayInfo::enumerate();
    // for d in &displays? {
    //     println!("{d:#?}")
    // }

    // return Ok(());
    // #[allow(unreachable_code)]

    // Read in configuration, or load default configuration
    let config_location = xdg_location_of(&dirs::CONFIG, "adaptive-brightness/config.ron");
    let config = match config_location {
        Ok(path) => {
            println!("Reading config from {path}", path = path.display());
            Config::read_from_file(path)?
        }
        Err(err) => {
            println!(
                "Config file not found in any standard locations, using default configuration."
            );
            println!("  Config search error: {err}");
            Config::from_str(DEFAULT_CONFIG)?
        }
    };
    println!("Loaded configuration: {config:?}");

    // Construct monitor states based on the configuration
    let mut monitors: Vec<MonitorState> = config
        .monitors
        .into_iter()
        .map(|mc| -> Result<MonitorState, anyhow::Error> {
            let curve = PiecewiseLinear::from_steps(mc.curve).ok_or_else(|| {
                anyhow::anyhow!("Invalid brightness curve for monitor {0:?}", mc.identifier)
            })?;

            Ok(match mc.identifier {
                MonitorId::Bus(bus_id) => MonitorState::for_bus(bus_id, curve),
                // TODO: match monitors by other properties
                MonitorId::Default => todo!(),
                MonitorId::Model(_) => todo!(),
                MonitorId::Serial(_) => todo!(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Connect to the brightness sensor
    let device = ftdi::find_by_vid_pid(0x0403, 0x6014)
        .interface(ftdi::Interface::A)
        .open()?;
    let i2c = hal::FtHal::init_default(device)?.i2c()?;
    let mut sensor = TSL2591::from_i2c(i2c)?;

    // VendorId = 0x0403
    // ProductId = 0x6014
    // Description = USB <-> Serial Converter
    // SerialNumber = FTA3Q3CS

    // Set initial brightness based on current state
    let lux = sensor.read_lux()? as u32;
    for m in &mut monitors {
        m.set_brightness_for_lux(lux)?;
    }

    // Main loop: periodically wake up to update all monitors
    loop {
        let mut updated = false;

        for m in &mut monitors {
            updated = updated || m.update_brightness(lux)?;
        }

        // Don't sleep as long if we may be off-target
        thread::sleep(time::Duration::from_millis(if updated {
            100
        } else {
            5_000
        }));
    }
}

// TODO remove test code
#[ignore]
#[test]
fn test_testing() {
    panic!()
}
