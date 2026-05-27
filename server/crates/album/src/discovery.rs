use std::sync::RwLock;

use rand::Rng;

use crate::config::AlbumConfig;
use crate::{sender, storage};

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

    // Subscribe to button random command topic — triggers random photo switch
    let button_topic = cfg.mqtt_button_random_topic.clone();
    if !button_topic.is_empty() {
        let cfg = cfg.clone();
        mqtt::subscribe(&button_topic, move |_payload| {
            let images = storage::list_images();
            if images.is_empty() {
                log::warn!("mqtt button: no images to switch to");
                return;
            }
            let idx = rand::rng().random_range(0..images.len());
            let id = &images[idx].id;
            log::info!("mqtt button: switching to random image {}", id);

            match storage::load_raw_binary(&cfg, id) {
                Ok(raw) => {
                    if let Err(e) = sender::send_to_device_blocking(&cfg, &raw) {
                        log::error!("mqtt button: send failed: {}", e);
                    } else {
                        storage::set_current(id).ok();
                    }
                }
                Err(e) => log::error!("mqtt button: load raw failed: {}", e),
            }
        })?;
    }

    log::info!(
        "album discovery initialized: ip_topic={}, state_topic={}",
        ip_topic,
        state_topic
    );
    Ok(())
}
