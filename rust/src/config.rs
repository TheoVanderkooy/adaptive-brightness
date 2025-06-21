use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum MonitorId {
    Default,
    Bus(u32),
    Model(String),
    Serial(String),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct MonitorConfig {
    identifier: MonitorId,
    curve: Vec<(u32, u32)>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    monitors: Vec<MonitorConfig>,
}

/*
TODO:
 - reading from config file
*/

#[test]
fn test_deserialize_config() {
    let parsed: Config = ron::from_str(
        r#"
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
    "#,
    )
    .unwrap();

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
