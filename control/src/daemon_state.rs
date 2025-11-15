use crate::monitor::MonitorState;

pub(crate) struct DaemonState {
    pub lux: u32,
    pub monitors: Vec<MonitorState>,
}
