use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum MonitorId {
    Default,
    I2cBus(u32),
    Model(String, String), // manufacturer, model
    Serial(String),
    ModelSerial(String, String, String), // manufacturer, model, serial#
    // TODO USB device, hiddev
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct MonitorConfig {
    pub identifier: MonitorId,
    pub curve: Vec<(u32, u32)>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    pub monitors: Vec<MonitorConfig>,
    // TODO: could configure brightness sensor (different intermediate chips (vid,pid), maybe implement different sensors)
}


impl Config {
    fn validate_and_normalize(mut self) -> Result<Self, anyhow::Error> {

        // Sort by priority. Sorting is stable, so position is the tie-breaker if multiple categories apply
        self.monitors.sort_by_key(|m| match m.identifier {
            MonitorId::I2cBus(_) => 0,
            MonitorId::ModelSerial(_, _, _) => 10,
            MonitorId::Serial(_) => 11,
            MonitorId::Model(_, _) => 20,
            MonitorId::Default => 100,
        });

        // TODO validation?

        Ok(self)
    }

    pub fn from_str(conf: &str) -> Result<Self, anyhow::Error> {
        let opts = ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        let parsed = opts.from_str::<Config>(conf)?;
        parsed.validate_and_normalize()
    }

    pub fn read_from_file<P: AsRef<Path>>(file: P) -> Result<Self, anyhow::Error> {
        let opts = ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        let parsed = opts.from_reader::<_, Config>(BufReader::new(File::open(file)?))?;
        parsed.validate_and_normalize()
    }
}

/*
TODO:
 - reading from config file
*/

#[cfg(test)]
mod test {
    use super::*;
    const TEST_CONFIG: &str = r#"
        (
        monitors: [
            (
                identifier: Model("abc", "xyz"),
                curve: [
                    (0, 10),
                    (250, 100),
                ],
            ),
            (
                identifier: I2cBus(6),
                curve: [
                    (0, 50),
                ],
            ),
        ]
        )
    "#;

    #[test]
    fn test_deserialize_config() {
        let parsed: Config = ron::from_str(TEST_CONFIG).unwrap();

        assert_eq!(
            parsed,
            Config {
                monitors: vec![
                    MonitorConfig {
                        identifier: MonitorId::Model("abc".to_string(), "xyz".to_string()),
                        curve: vec![(0, 10), (250, 100)],
                    },
                    MonitorConfig {
                        identifier: MonitorId::I2cBus(6),
                        curve: vec![(0, 50)],
                    },
                ]
            }
        );
    }
}
