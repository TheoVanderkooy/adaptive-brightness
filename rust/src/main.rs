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
use ddc;
use xdg_dirs::{dirs, xdg_location_of, xdg_user_dir};

// STD
use std::fs::File;
use std::path::PathBuf;
use std::{fs, thread, time};

// 3rd party libraries
use anyhow::Context;
use clap::{Parser, Subcommand, command};
use ftdi_embedded_hal as hal;

const CONFIG_PATH: &str = "adaptive-brightness/config.ron";

const DEFAULT_CONFIG: &str = r#"
(
monitors: [
    (
        identifier: Default,
        curve: [
            (0, 10),
            (250, 100),
        ],
    ),
]
)
"#;

#[derive(Debug, Subcommand, PartialEq)]
enum Command {
    #[command(
        about = "(default) Poll brightness sensor value and periodically update monitor brightness based on the config file."
    )]
    Run,

    #[command(
        about = "Check configuration file syntax and print out the settings that will be applied for each display device."
    )]
    Check,

    #[command(about = "Generate a default config file")]
    GenConfig,

    // TODO:
    //  - detecting monitors (... or at least tell them (how to) to use ddcutil)
    //  - directly set brightness?

    // TODO remove
    #[command(about = "for testing")]
    Test,
}

#[derive(Debug, Parser, PartialEq)]
#[command(
    about = "A tool for adaptive brightness on devices that wouldn't otherwise have it built in",
    author = "Theo Vanderkooy",
    version
)]
struct Args {
    #[arg(
        global = true,
        short,
        long = "config",
        help = format!("Path to configuration file. Defaults to `{CONFIG_PATH}` under the user's config directory."),
    )]
    config_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

impl Args {
    /// Get the config path, and verify the file exists. This is the either the path passed as an arg, or from the XDG directory if not specified.
    ///
    /// This returns error if the path does not exist.
    fn get_config_path(&self) -> anyhow::Result<PathBuf> {
        match &self.config_path {
            Some(path) => {
                let path = path
                    .canonicalize()
                    .with_context(|| format!("Could not open config file `{0}`", path.display()));

                path
            }
            None => xdg_location_of(&dirs::CONFIG, CONFIG_PATH)
                .with_context(|| "Could not open config file"),
        }
    }
}

/// Load the configuration based on arguments.
/// Uses the file supplied to the CLI, or in the default location if not specified, or the default config if there is no file.
fn get_config(args: &Args) -> anyhow::Result<Config> {
    match args.get_config_path() {
        Ok(path) => {
            println!("Reading config from {path}", path = path.display());
            Config::read_from_file(path)
        }
        Err(err) => {
            eprintln!(
                "Config file not found in any standard locations, using default configuration."
            );
            eprintln!("  Config search error: {err}");
            Config::from_str(DEFAULT_CONFIG)
        }
    }
}

/// Get list of displays from the DDC library, and wrapp the error because they aren't sync so anyhow doesn't like them.
fn get_displays() -> anyhow::Result<ddc::DisplayInfoList> {
    ddc::get_display_info_list(false).map_err(|e| anyhow::anyhow!("{}", e.to_string()))
}

/// Match up display configuration to the detected displays.
fn match_displays_to_config<'d, 'c>(
    displays: &'d ddc::DisplayInfoList,
    config: &'c Config,
) -> anyhow::Result<Vec<(&'d ddc::DisplayInfo, Option<&'c MonitorConfig>)>> {
    let ret = displays
        .into_iter()
        .map(|d| {
            let matching = config.monitors.iter().find(|&m| match &m.identifier {
                // default always applies
                MonitorId::Default => true,

                // compare physical path of the display
                MonitorId::I2cBus(busno) => {
                    d.path() == ddc::DisplayPath::I2C { bus: *busno as i32 }
                }

                // compare identifiers of the display
                MonitorId::Model(manufacturer, model) => {
                    d.manufacturer() == manufacturer && d.model() == model
                }
                MonitorId::ModelSerial(manufacturer, model, serial) => {
                    d.manufacturer() == manufacturer
                        && d.model() == model
                        && d.serial_number() == serial
                }
                MonitorId::Serial(serial) => d.serial_number() == serial,
            });

            (d, matching)
        })
        .collect();

    Ok(ret)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("args = {args:?}");

    // TODO initialize libddcutil

    // process commands
    match args.command {
        // Primary behaviour: releatedly read brightness and update monitors
        None | Some(Command::Run) => main_loop(&args),

        // Test config file: make sure it exists, can be read, and can be parsed
        Some(Command::Check) => check_config(&args),

        // Generate config file: if the file does not already exist, write
        Some(Command::GenConfig) => gen_config_file(&args),

        Some(Command::Test) => test(&args),
    }
}

/// Verify the config file: Make sure it can be found at the expected location (passed through CLI or using XDG config location), and parses properly.
fn check_config(args: &Args) -> anyhow::Result<()> {
    // Try to _find_ the config file
    let path = args
        .get_config_path()
        .with_context(|| "Failed to find config file")?;

    // Try to _parse_ the config file
    println!("Attempting to load config from `{0}`", path.display());
    let config = Config::read_from_file(path).with_context(|| "Failed to parse configuration")?;

    println!("Successfully read config: {config:#?}");

    // Detect monitors and match them up with configuration rules
    println!("\nDetecting displays...");
    let displays = get_displays()?;
    let config_mapping = match_displays_to_config(&displays, &config)?;

    for (display, conf) in config_mapping {
        println!(
            "Display {0}: {1} {2} {3}",
            display.display_no(),
            display.manufacturer(),
            display.model(),
            display.serial_number()
        );
        match conf {
            None => println!("  No matching configuration!"),
            Some(conf) => println!("  Matched: {0:?}", conf),
        }
    }

    // TODO: compare configuration against list of displays, list brightness curve for each detected display

    Ok(())
}

/// Generate a default configuration file, at the expected location based on args or environment variables.
fn gen_config_file(args: &Args) -> anyhow::Result<()> {
    // CLI arg path, or default from environment
    let path = args
        .config_path
        .clone()
        .map_or_else(|| xdg_user_dir(&dirs::CONFIG, CONFIG_PATH), Ok)
        .with_context(|| "Could not determine location for config file")?;

    // Create parent directory path if applicable
    match path.parent() {
        Some(parent) => fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create parent directory of the new config file {0}",
                path.display()
            )
        })?,
        _ => { /* do nothing if no parent */ }
    };

    // Detect displays to write default config
    let displays = get_displays()?;

    let monitors = displays
        .into_iter()
        .map(|d| MonitorConfig {
            identifier: MonitorId::ModelSerial(
                d.manufacturer().to_string(),
                d.model().to_string(),
                d.serial_number().to_string(),
            ),
            curve: vec![(0, 10), (250, 100)],
        })
        .collect::<Vec<_>>();
    let conf = Config { monitors: monitors };

    // Create the new file and write the default contents
    let file = File::create_new(&path)
        .with_context(|| format!("Failed to create new config file {0}", path.display()))?;

    let format_opts = ron::ser::PrettyConfig::new().indentor("  ");
    ron::Options::default().to_io_writer_pretty(file, &conf, format_opts)?;

    Ok(())
}

/// Default daemon behaviour: Read config file, then read brightness and update each monitor forever.
fn main_loop(args: &Args) -> anyhow::Result<()> {
    // Read in configuration, or load default configuration
    let config = get_config(args)?;
    println!("Loaded configuration: {config:?}");

    // Detect displays and match them up with configuration settings
    let displays = get_displays()?;
    let config_mapping = match_displays_to_config(&displays, &config)?;

    // Construct internal state for each device
    let mut monitors: Vec<MonitorState> = config_mapping
        .iter()
        .filter_map(|&(ref d, mc)| {
            // filter out monitors that don't match any config
            if let Some(mc) = mc {
                Some((*d, mc))
            } else {
                None
            }
        })
        .map(|(d, mc)| -> anyhow::Result<MonitorState> {
            // Open each display and build a monitor config for them
            let curve = PiecewiseLinear::from_steps(mc.curve.clone()).ok_or_else(|| {
                anyhow::anyhow!("Invalid brightness curve for monitor {0:?}", mc.identifier)
            })?;

            Ok(MonitorState::for_display(&d, curve)?)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

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
        let lux = sensor.read_lux()? as u32;

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

// TODO remove this once no longer needed
fn test(_args: &Args) -> anyhow::Result<()> {
    // ...

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arg_parsing() {
        assert_eq!(
            Args {
                config_path: None,
                command: None,
            },
            Args::try_parse_from(&["executable"]).unwrap()
        );

        assert_eq!(
            Args {
                config_path: Some(PathBuf::from("/some/file")),
                command: None,
            },
            Args::try_parse_from(&["executable", "--config", "/some/file"]).unwrap()
        );

        assert_eq!(
            Args {
                config_path: Some(PathBuf::from("/some/file")),
                command: Some(Command::Check),
            },
            Args::try_parse_from(&["executable", "check", "--config", "/some/file"]).unwrap()
        );

        assert_eq!(
            Args {
                config_path: Some(PathBuf::from("/some/file")),
                command: Some(Command::Run),
            },
            Args::try_parse_from(&["executable", "--config", "/some/file", "run"]).unwrap()
        );
    }
}
