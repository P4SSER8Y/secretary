use once_cell::sync::OnceCell;
use rumqttc::{Client, Connection, Event, Incoming, MqttOptions, QoS};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

static CLIENT: OnceCell<Client> = OnceCell::new();
static CONNECTION: OnceCell<Mutex<Connection>> = OnceCell::new();
type TopicCallback = Arc<dyn Fn(Vec<u8>) + Send + Sync + 'static>;
type SubscriberMap = HashMap<String, Vec<TopicCallback>>;
static SUBSCRIBERS: OnceCell<Arc<RwLock<SubscriberMap>>> = OnceCell::new();

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
    pub availability_topic: Option<String>,
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

    let (client, connection) = Client::new(options, 10);
    CLIENT.set(client).ok();
    SUBSCRIBERS
        .set(Arc::new(RwLock::new(HashMap::new())))
        .ok();
    CONNECTION.set(Mutex::new(connection)).ok();

    let subscribers = SUBSCRIBERS.get().unwrap().clone();
    std::thread::spawn(move || {
        let conn = CONNECTION.get().unwrap();
        let mut conn = match conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        for notification in conn.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let payload = publish.payload.to_vec();
                    let topic = publish.topic;
                    if let Ok(registry) = subscribers.read() {
                        for (filter, callbacks) in registry.iter() {
                            if rumqttc::matches(&topic, filter) {
                                for cb in callbacks {
                                    cb(payload.clone());
                                }
                            }
                        }
                    }
                }
                Err(e) => log::error!("mqtt event loop error: {}", e),
                _ => {}
            }
        }
    });
    log::info!(
        "mqtt connected to {}:{} as {}",
        config.host,
        config.port,
        config.client_id
    );
}

pub fn subscribe(
    topic_filter: &str,
    callback: impl Fn(Vec<u8>) + Send + Sync + 'static,
) -> anyhow::Result<()> {
    let cb: TopicCallback = Arc::new(callback);
    let subs = match SUBSCRIBERS.get() {
        Some(s) => s,
        None => {
            log::warn!("mqtt not initialized, skipping subscribe to {}", topic_filter);
            return Ok(());
        }
    };
    subs.write()
        .map_err(|e| anyhow::anyhow!("lock error: {}", e))?
        .entry(topic_filter.to_string())
        .or_insert_with(Vec::new)
        .push(cb);

    if let Some(client) = client() {
        client.subscribe(topic_filter, QoS::AtMostOnce)?;
    }
    Ok(())
}

pub fn publish_discovery(config: &HassDeviceConfig) {
    let mut battery_config = serde_json::json!({
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
    if let Some(ref topic) = config.availability_topic {
        battery_config["availability_topic"] = serde_json::json!(topic);
    }
    let topic = format!(
        "homeassistant/sensor/{}/battery/config",
        config.device_id
    );
    publish(&topic, &battery_config.to_string(), true);

    let mut last_seen_config = serde_json::json!({
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
    if let Some(ref topic) = config.availability_topic {
        last_seen_config["availability_topic"] = serde_json::json!(topic);
    }
    let topic = format!(
        "homeassistant/sensor/{}/last_seen/config",
        config.device_id
    );
    publish(&topic, &last_seen_config.to_string(), true);
}

pub fn publish_state(device_id: &str, battery: Option<usize>, last_seen: &str) {
    let state = DeviceState {
        battery,
        last_seen: last_seen.to_string(),
    };
    if let Ok(payload) = serde_json::to_string(&state) {
        publish(&format!("secretary/{}/state", device_id), &payload, true);
    }
}

fn publish(topic: &str, payload: &str, retain: bool) {
    if let Some(client) = client() {
        if let Err(e) = client.publish(topic, QoS::AtMostOnce, retain, payload) {
            log::error!("mqtt publish failed: {}", e);
        }
    }
}

pub fn publish_availability(device_id: &str, status: &str) {
    publish(
        &format!("secretary/{}/availability", device_id),
        status,
        true,
    );
}
