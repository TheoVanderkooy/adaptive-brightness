use std::path::PathBuf;
use std::time;

use anyhow::Context;
use clap::{Parser, Subcommand};
use xdg_dirs::{dirs, xdg_location_of};

use crate::config::CONFIG_PATH;

#[derive(Debug, PartialEq, clap::Args)]
#[command(flatten_help = true, about = "Collect brightness data to ")]
pub(crate) struct CollectBrightnessArgs {
    #[arg(
        short,
        long = "out",
        help = "Path to write brightness data to. Defaults to stdout. If a path is specified, the data will also be pretty-printed to stdout. Format is: date,time,lux"
    )]
    pub out_path: Option<PathBuf>,

    #[arg(
        short, long = "period",
        help = "How frequently to poll brightness.",
        value_parser = humantime::parse_duration,
        default_value = "5m",
    )]
    pub period: time::Duration,
}

#[derive(Debug, Subcommand, PartialEq)]
pub(crate) enum Command {
    #[command(
        about = "Poll brightness sensor value and periodically update monitor brightness based on the config file."
    )]
    Daemon,

    #[command(about = "Read current lux value.")]
    Read,

    #[command(
        about = "Check configuration file syntax and print out the settings that will be applied for each display device."
    )]
    Check,

    #[command(about = "Generate a default config file")]
    GenConfig,

    CollectBrightness(CollectBrightnessArgs),

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
pub(crate) struct Args {
    #[arg(
        global = true,
        short,
        long = "config",
        help = format!("Path to configuration file. Defaults to `{CONFIG_PATH}` under the user's config directory."),
    )]
    pub config_path: Option<PathBuf>,

    #[arg(
        global = true,
        short = 's',
        long = "brightness-socket",
        help = "Path of the brightness server's socket from which to read brightness values. If not specified, the device will be opened directly without writing brightness to any socket."
    )]
    pub socket_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Args {
    /// Get the config path, and verify the file exists. This is the either the path passed as an arg, or from the XDG directory if not specified.
    ///
    /// This returns error if the path does not exist.
    pub(crate) fn get_config_path(&self) -> anyhow::Result<PathBuf> {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_arg_parsing() {
        assert_eq!(
            Args {
                config_path: None,
                command: None,
                socket_path: None,
            },
            Args::try_parse_from(&["executable"]).unwrap()
        );

        assert_eq!(
            Args {
                config_path: Some(PathBuf::from("/some/file")),
                command: None,
                socket_path: None,
            },
            Args::try_parse_from(&["executable", "--config", "/some/file"]).unwrap()
        );

        assert_eq!(
            Args {
                config_path: Some(PathBuf::from("/some/file")),
                command: Some(Command::Check),
                socket_path: None,
            },
            Args::try_parse_from(&["executable", "check", "--config", "/some/file"]).unwrap()
        );

        assert_eq!(
            Args {
                config_path: Some(PathBuf::from("/some/file")),
                command: Some(Command::Daemon),
                socket_path: Some(PathBuf::from("/some/socket")),
            },
            Args::try_parse_from(&[
                "executable",
                "--config",
                "/some/file",
                "daemon",
                "-s",
                "/some/socket"
            ])
            .unwrap()
        );
    }
}
