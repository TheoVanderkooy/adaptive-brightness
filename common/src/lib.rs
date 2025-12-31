use serde::{Deserialize, Serialize};

pub const DEFAULT_BRIGHTNESS_SOCK_PATH: &str = "/tmp/abc-brightness.sock";
pub const DEFAULT_CONTROL_SOCK_PATH: &str = "/tmp/abc-control.sock";

/// Serializable monitor status to be exposed to client apps monitoring the daemon state.
#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorStatus {
    pub display_name: String,
    pub target_brightness: u16,
    pub brightness: u16,
}

/// A snapshot of the current status that can be serialized & shared with other clients.
#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub lux: u32,
    pub monitors: Vec<MonitorStatus>,
    pub unmanaged_monitors: Vec<String>,
}
