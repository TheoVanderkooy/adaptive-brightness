use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum MonitorId {
    Default,
    Bus(u32),
    Model(String),
    Serial(String),
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
    pub fn from_str(conf: &str) -> Result<Self, anyhow::Error> {
        Ok(ron::from_str::<Config>(conf)?)
    }

    pub fn read_from_file<P: AsRef<Path>>(file: P) -> Result<Self, anyhow::Error> {
        Ok(ron::de::from_reader(BufReader::new(File::open(file)?))?)
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
                identifier: Model("xyz"),
                curve: [
                    (0, 10),
                    (250, 100),
                ],
            ),
            (
                identifier: Bus(6),
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
                        identifier: MonitorId::Model("xyz".to_string()),
                        curve: vec![(0, 10), (250, 100)],
                    },
                    MonitorConfig {
                        identifier: MonitorId::Bus(6),
                        curve: vec![(0, 50)],
                    },
                ]
            }
        );
    }
}
