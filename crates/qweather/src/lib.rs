mod api;

use std::str::FromStr;

use anyhow::anyhow;
use figment::Figment;
use log::{error, info};
use once_cell::sync::OnceCell;

static SCHEDULER: OnceCell<cron::Schedule> = OnceCell::new();

pub use api::get_24h_forcast;
pub use api::get_3d_forecast;

pub async fn init(figment: &Figment) {
    SCHEDULER.get_or_init(|| {
        let value = figment
            .find_value("weather.cron")
            .expect("weather.cron not configured");
        let exp = value.as_str().expect("weather.cron format is invalid");
        info!("weather.cron is {}", exp);
        return cron::Schedule::from_str(exp).expect("weather.cron format is invalid");
    });
    api::set_location(
        figment
            .find_value("weather.location")
            .expect("weather.location not found")
            .into_string()
            .ok_or(anyhow!("weather.location not found"))
            .unwrap(),
    ).await;
    api::set_key(
        figment
            .find_value("weather.key")
            .expect("weather.key not found")
            .into_string()
            .ok_or(anyhow!("weather.key not found"))
            .unwrap(),
    ).await;
    tokio::spawn(main());
}

async fn main() {
    info!("start cron");
    let mut iter = SCHEDULER.get().unwrap().upcoming(chrono::Local);
    loop {
        if let Err(err) = api::update_24h().await {
            error!("failed to update weather of 24 hours: {:}", err.to_string());
        }
        if let Err(err) = api::update_3d().await {
            error!("failed to update weather of 3 days: {:}", err.to_string());
        }
        let next = iter.next();
        if let Some(next) = next {
            let now = chrono::Local::now();
            let duration = next - now;
            info!(
                "wait until: {}, aka wait for {}s",
                next,
                duration.num_seconds()
            );
            tokio::time::sleep(std::time::Duration::from_secs(
                duration.num_seconds().try_into().unwrap(),
            )).await;
        } else {
            break;
        }
    }
}
