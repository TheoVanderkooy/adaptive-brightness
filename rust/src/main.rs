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
use xdg_dirs::{dirs, xdg_location_of, xdg_user_dir};

use std::fs::File;
use std::io::Write;
// STD
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
        identifier: Bus(6),
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

    #[command(about = "Verify the config file")]
    TestConfig,

    #[command(about = "Generate a default config file")]
    GenConfig,
    // TODO:
    //  - detecting monitors (... or at least tell them (how to) to use ddcutil)
    //  - directly set brightness?
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
    fn get_config_path(&self) -> Result<PathBuf, anyhow::Error> {
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

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    println!("args = {args:?}");

    // process commands
    match args.command {
        // Primary behaviour: releatedly read brightness and update monitors
        None | Some(Command::Run) => main_loop(&args),

        // Test config file: make sure it exists, can be read, and can be parsed
        Some(Command::TestConfig) => check_config(&args),

        // Generate config file: if the file does not already exist, write
        Some(Command::GenConfig) => return gen_config_file(&args),
    }
}

/// Verify the config file: Make sure it can be found at the expected location (passed through CLI or using XDG config location), and parses properly.
fn check_config(args: &Args) -> Result<(), anyhow::Error> {
    // Try to _find_ the config file
    let path = args
        .get_config_path()
        .with_context(|| "Failed to find config file")?;

    // Try to _parse_ the config file
    println!("Attempting to load config from `{0}`", path.display());
    let conf = Config::read_from_file(path).with_context(|| "Failed to parse configuration")?;

    println!("Successfully read config: {conf:#?}");
    Ok(())
}

/// Generate a default configuration file, at the expected location based on args or environment variables.
fn gen_config_file(args: &Args) -> Result<(), anyhow::Error> {
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

    // Create the new file and write the default contents
    let mut file = File::create_new(&path)
        .with_context(|| format!("Failed to create new config file {0}", path.display()))?;

    // Create the new file and write the default contents
    write!(file, "{}", DEFAULT_CONFIG)
        .with_context(|| format!("Failed to write the new config file {0}", path.display()))?;

    Ok(())
}

/// Default daemon behaviour: Read config file, then read brightness and update each monitor forever.
fn main_loop(args: &Args) -> Result<(), anyhow::Error> {
    // Read in configuration, or load default configuration
    let config = match args.get_config_path() {
        Ok(path) => {
            println!("Reading config from {path}", path = path.display());
            Config::read_from_file(path)?
        }
        Err(err) => {
            eprintln!(
                "Config file not found in any standard locations, using default configuration."
            );
            eprintln!("  Config search error: {err}");
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
                command: Some(Command::TestConfig),
            },
            Args::try_parse_from(&["executable", "test-config", "--config", "/some/file"]).unwrap()
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
