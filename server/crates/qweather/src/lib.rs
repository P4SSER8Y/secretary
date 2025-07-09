mod api;

use anyhow::anyhow;
use figment::Figment;

pub use api::*;

pub async fn init(figment: &Figment) {
    api::set_location(
        figment
            .find_value("weather.location")
            .expect("weather.location not found")
            .into_string()
            .ok_or(anyhow!("weather.location not found"))
            .unwrap(),
    )
    .await;
    api::set_key(
        figment
            .find_value("weather.key")
            .expect("weather.key not found")
            .into_string()
            .ok_or(anyhow!("weather.key not found"))
            .unwrap(),
    )
    .await;
}
