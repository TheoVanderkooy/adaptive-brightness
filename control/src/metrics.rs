use std::sync::atomic::AtomicU32;

use prometheus_client::{
    encoding::EncodeLabelSet,
    metrics::{family::Family, gauge::Gauge},
    registry::{Registry, Unit},
};

/// Labels for monitor-specific metrics
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct MonitorLabels {
    monitor: String,
}

type MonitorMetric = Family<MonitorLabels, Gauge<u32, AtomicU32>>;

/// Metrics for the brightness daemon
#[derive(Debug)]
pub struct Metrics {
    pub registry: Registry,

    pub brightness: Gauge<u32, AtomicU32>,
    monitor_settings: MonitorMetric,
}

impl Metrics {
    pub fn new() -> Self {
        let mut registry = Registry::default();
        let brightness = Gauge::default();
        registry.register_with_unit(
            "brightness",
            "brightness (in lux) measured by the sensor on my desk",
            Unit::Other("lux".into()),
            brightness.clone(),
        );

        let monitor_settings = MonitorMetric::default();
        registry.register_with_unit(
            "brightness_setting",
            "brightness setting (percentage) of the given monitor",
            Unit::Other("pct".into()),
            monitor_settings.clone(),
        );

        Self {
            registry,
            brightness,
            monitor_settings,
        }
    }

    pub fn set_monitor_brightness(&self, name: impl ToString, val: u16) {
        self.monitor_settings
            .get_or_create(&MonitorLabels {
                monitor: name.to_string(),
            })
            .set(val as u32);
    }
}
