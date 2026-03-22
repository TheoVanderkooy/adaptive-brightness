use std::sync::atomic::AtomicU32;

use prometheus_client::{
    metrics::gauge::Gauge,
    registry::{Registry, Unit},
};

/// Metrics for the brightness daemon
#[derive(Debug)]
pub struct Metrics {
    pub registry: Registry,

    pub brightness: Gauge<u32, AtomicU32>,
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

        Self {
            registry,
            brightness,
        }
    }
}
