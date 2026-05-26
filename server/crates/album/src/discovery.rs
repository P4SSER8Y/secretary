use std::sync::RwLock;

use crate::config::AlbumConfig;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceState {
    pub ip: Option<String>,
    pub online: bool,
    pub status: Option<String>,
    pub last_seen: Option<String>,
}

static DEVICE_STATE: RwLock<DeviceState> = RwLock::new(DeviceState {
    ip: None,
    online: false,
    status: None,
    last_seen: None,
});

pub fn get_device_state() -> DeviceState {
    DEVICE_STATE
        .read()
        .ok()
        .map(|s| s.clone())
        .unwrap_or(DeviceState {
            ip: None,
            online: false,
            status: None,
            last_seen: None,
        })
}

pub fn init(cfg: &AlbumConfig) -> anyhow::Result<()> {
    // Subscribe to device IP topic
    let ip_topic = cfg.mqtt_device_ip_topic.clone();
    mqtt::subscribe(&ip_topic, move |payload| {
        let ip = String::from_utf8_lossy(&payload).trim().to_string();
        if let Ok(mut state) = DEVICE_STATE.write() {
            state.ip = Some(ip);
            state.last_seen = Some(chrono::Local::now().to_rfc3339());
            state.online = true;
        }
    })?;

    // Subscribe to device status topic (idle/fetching/updating/done/error/restarted)
    let state_topic = cfg.mqtt_device_state_topic.clone();
    mqtt::subscribe(&state_topic, move |payload| {
        let status = String::from_utf8_lossy(&payload).trim().to_string();
        if let Ok(mut state) = DEVICE_STATE.write() {
            state.status = Some(status);
            state.last_seen = Some(chrono::Local::now().to_rfc3339());
            state.online = true;
        }
    })?;

    log::info!(
        "album discovery initialized: ip_topic={}, state_topic={}",
        ip_topic,
        state_topic
    );
    Ok(())
}
