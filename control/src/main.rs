// in-crate modules
mod args;
mod config;
mod daemon;
mod monitor;
mod piecewise_linear;
mod sensor;

// in-crate imports
use args::*;
use common::{DEFAULT_BRIGHTNESS_SOCK_PATH, DEFAULT_CONTROL_SOCK_PATH, DaemonStatus};
use config::*;
use daemon::*;
use sensor::*;

// my libraries
use ddc::{self, ConvertToAnyhow};
use xdg_dirs::{dirs, xdg_user_dir};

// STD
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    str::FromStr,
    thread,
};

// 3rd party libraries
use anyhow::Context;
use clap::Parser;

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
    ddc::get_display_info_list(false).anyhow()
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

fn init_ddcutil() -> anyhow::Result<()> {
    ddc::sys::DDCA_Init_Options(0);
    ddc::lib_init(None, ddc::SysLogLevel::DDCA_SYSLOG_WARNING, 0.into()).anyhow()?;
    ddc::lib_set_dynamic_sleep(false);

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // process commands
    match args.command {
        // Repeatedly read brightness and update monitors
        None | Some(Command::Daemon) => {
            println!("args = {args:?}");
            daemon_main(&args)
        }

        Some(Command::Read) => read_brightness(&args),

        Some(Command::Status) => read_status(&args),

        // Test config file: make sure it exists, can be read, and can be parsed
        Some(Command::Check) => {
            init_ddcutil()?;
            check_config(&args)
        }

        // Generate config file: if the file does not already exist, write
        Some(Command::GenConfig) => {
            init_ddcutil()?;
            gen_config_file(&args)
        }

        // Periodically poll brightness and write it to a file
        Some(Command::CollectBrightness(ref collect_args)) => {
            collect_brightness(&args, &collect_args)
        }

        // testing
        Some(Command::Test) => test(&args),
    }
}

/// Simply read and print out the current brightness
fn read_brightness(args: &Args) -> anyhow::Result<()> {
    let sock_path = &args.brightness_socket_path;

    let sensor = Sensor::open(sock_path);
    let mut sensor = if let Err(e) = &sensor
        && sock_path.is_none()
    {
        println!("Couldn't open sensor ({e}), trying default socket path...");
        // if no path specified, and loading the device fails, check for default system socket
        Sensor::open(&Some(DEFAULT_BRIGHTNESS_SOCK_PATH))?
    } else {
        sensor?
    };
    let lux = sensor.read_lux()? as i32;

    println!("{lux}");

    Ok(())
}

/// Read status from the status socket & print it out
fn read_status(args: &Args) -> anyhow::Result<()> {
    // Connect to the status socket
    let sock_path = match &args.control_socket_path {
        Some(p) => p,
        None => &PathBuf::from_str(DEFAULT_CONTROL_SOCK_PATH)?,
    };

    let mut stream = UnixStream::connect(sock_path)?;

    // Send 's' to the socket to get the status
    stream.write_all(&[b's'])?;

    let status = serde_json::Deserializer::from_reader(stream)
        .into_iter::<DaemonStatus>()
        .next()
        .with_context(|| "Didn't get a response from the daemon")??;

    let max_monitor_name_len = status
        .monitors
        .iter()
        .map(|ms| ms.display_name.len())
        .max()
        .unwrap_or(0);

    println!("Current lux:  {0}", status.lux);
    println!("\nMonitor brightness:");
    for ms in status.monitors {
        println!(
            "  {1:<0$}:  {2}  (target={3})",
            max_monitor_name_len, ms.display_name, ms.brightness, ms.target_brightness
        );
    }
    println!("\nUnmanaged monitors:");
    for m in status.unmanaged_monitors {
        println!("  {0}", m);
    }

    Ok(())
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

/// Periodically measure the current brightness, and write the result to a file or stdout.
fn collect_brightness(args: &Args, collect_args: &CollectBrightnessArgs) -> anyhow::Result<()> {
    let to_file = collect_args.out_path.is_some();

    let mut sensor = Sensor::open(&args.brightness_socket_path)?;

    let mut writer: csv::Writer<Box<dyn io::Write>> = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(match collect_args.out_path {
            Some(ref p) => Box::new(OpenOptions::new().append(true).create(true).open(p)?),
            None => Box::new(io::stdout()),
        });

    // Loop to collect data and output CSV to out_path.
    loop {
        let lux = sensor.read_lux()?;
        let now = chrono::Local::now().naive_local();

        writer.serialize((now.date().to_string(), now.time().to_string(), lux))?;
        writer.flush()?;

        // If writing to a file, pretty-print to stdout
        if to_file {
            println!("{0} {1} -- lux={lux}", now.date(), now.time());
        }

        thread::sleep(collect_args.period);
    }
}

// TODO remove this once no longer needed
fn test(_args: &Args) -> anyhow::Result<()> {
    // ...

    Ok(())
}
