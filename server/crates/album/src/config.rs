use anyhow::anyhow;
use figment::Figment;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct AlbumConfig {
    /// 相对于 data_path 的存储目录
    pub storage_path: String,
    pub device_name: String,
    pub eink_width: u32,
    pub eink_height: u32,
    pub mqtt_device_ip_topic: String,
    pub mqtt_device_state_topic: String,
    pub device_http_endpoint: String,
    pub preserve_original: bool,
}

impl AlbumConfig {
    pub fn from_figment(config: &Figment) -> anyhow::Result<Self> {
        let storage_path = config
            .find_value("album.storage_path")
            .ok()
            .and_then(|v| v.into_string())
            .unwrap_or_else(|| "album".to_string());

        let device_name = config
            .find_value("album.device_name")
            .map_err(|e| anyhow!("album.device_name: {}", e))?
            .into_string()
            .ok_or(anyhow!("album.device_name must be a string"))?;

        let eink_width = config
            .find_value("album.eink_width")
            .ok()
            .and_then(|v| v.to_u128())
            .unwrap_or(800) as u32;

        let eink_height = config
            .find_value("album.eink_height")
            .ok()
            .and_then(|v| v.to_u128())
            .unwrap_or(480) as u32;

        let mqtt_device_ip_topic = config
            .find_value("album.mqtt_device_ip_topic")
            .map_err(|e| anyhow!("album.mqtt_device_ip_topic: {}", e))?
            .into_string()
            .ok_or(anyhow!("album.mqtt_device_ip_topic must be a string"))?;

        let mqtt_device_state_topic = config
            .find_value("album.mqtt_device_state_topic")
            .map_err(|e| anyhow!("album.mqtt_device_state_topic: {}", e))?
            .into_string()
            .ok_or(anyhow!("album.mqtt_device_state_topic must be a string"))?;

        let device_http_endpoint = config
            .find_value("album.device_http_endpoint")
            .map_err(|e| anyhow!("album.device_http_endpoint: {}", e))?
            .into_string()
            .ok_or(anyhow!("album.device_http_endpoint must be a string"))?;

        let preserve_original = config
            .find_value("album.preserve_original")
            .ok()
            .and_then(|v| v.to_bool())
            .unwrap_or(false);

        Ok(AlbumConfig {
            storage_path,
            device_name,
            eink_width,
            eink_height,
            mqtt_device_ip_topic,
            mqtt_device_state_topic,
            device_http_endpoint,
            preserve_original,
        })
    }
}
