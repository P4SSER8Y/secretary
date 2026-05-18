use once_cell::sync::OnceCell;
use rumqttc::{Client, MqttOptions, QoS};
use serde::Serialize;
use std::time::Duration;

static CLIENT: OnceCell<Client> = OnceCell::new();

pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub client_id: String,
}

pub struct HassDeviceConfig {
    pub name: String,
    pub device_id: String,
    pub state_topic: String,
}

#[derive(Serialize)]
struct DeviceState {
    battery: Option<usize>,
    last_seen: String,
}

fn client() -> Option<&'static Client> {
    CLIENT.get()
}

pub fn init(config: MqttConfig) {
    let mut options = MqttOptions::new(&config.client_id, &config.host, config.port);
    options.set_keep_alive(Duration::from_secs(30));
    if let (Some(u), Some(p)) = (&config.username, &config.password) {
        if !u.is_empty() {
            options.set_credentials(u.as_str(), p.as_str());
        }
    }

    let (client, mut connection) = Client::new(options, 10);
    CLIENT.set(client).ok();
    std::thread::spawn(move || {
        for _ in connection.iter() {}
    });
    log::info!(
        "mqtt connected to {}:{} as {}",
        config.host,
        config.port,
        config.client_id
    );
}

pub fn publish_discovery(config: &HassDeviceConfig) {
    let battery_config = serde_json::json!({
        "name": format!("{} Battery", config.name),
        "device_class": "battery",
        "state_topic": config.state_topic,
        "unit_of_measurement": "%",
        "value_template": "{{ value_json.battery }}",
        "unique_id": format!("{}_battery", config.device_id),
        "device": {
            "name": config.name,
            "identifiers": [config.device_id]
        }
    });
    let topic = format!(
        "homeassistant/sensor/{}/battery/config",
        config.device_id
    );
    publish(&topic, &battery_config.to_string());

    let last_seen_config = serde_json::json!({
        "name": format!("{} Last Seen", config.name),
        "device_class": "timestamp",
        "state_topic": config.state_topic,
        "value_template": "{{ value_json.last_seen }}",
        "unique_id": format!("{}_last_seen", config.device_id),
        "device": {
            "name": config.name,
            "identifiers": [config.device_id]
        }
    });
    let topic = format!(
        "homeassistant/sensor/{}/last_seen/config",
        config.device_id
    );
    publish(&topic, &last_seen_config.to_string());
}

pub fn publish_state(device_id: &str, battery: Option<usize>, last_seen: &str) {
    let state = DeviceState {
        battery,
        last_seen: last_seen.to_string(),
    };
    if let Ok(payload) = serde_json::to_string(&state) {
        publish(&format!("secretary/{}/state", device_id), &payload);
    }
}

fn publish(topic: &str, payload: &str) {
    if let Some(client) = client() {
        if let Err(e) = client.publish(topic, QoS::AtMostOnce, false, payload) {
            log::error!("mqtt publish failed: {}", e);
        }
    }
}
