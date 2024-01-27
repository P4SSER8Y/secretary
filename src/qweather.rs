use influxdb2::models::DataPoint;
use once_cell::sync::OnceCell;
use qweather::CurrentWeather;
use rocket::{figment::Figment, Build, Rocket};
use std::str::FromStr;

static SCHEDULER: OnceCell<cron::Schedule> = OnceCell::new();

pub async fn build(build: Rocket<Build>, figment: &Figment) -> Rocket<Build> {
    qweather::init(figment).await;
    SCHEDULER.get_or_init(|| {
        let value = figment
            .find_value("weather.cron")
            .expect("weather.cron not configured");
        let exp = value.as_str().expect("weather.cron format is invalid");
        info!("weather.cron is {}", exp);
        return cron::Schedule::from_str(exp).expect("weather.cron format is invalid");
    });
    tokio::spawn(main());
    return build;
}

fn build_datapoint_from_currentweather(value: &CurrentWeather) -> Result<DataPoint, anyhow::Error> {
    let mut p = DataPoint::builder("weather").tag("location", "home");
    if let Some(v) = value.temperature {
        p = p.field("temperature", v);
    }
    if let Some(v) = value.feels_like {
        p = p.field("feels_like", v);
    }
    if let Some(v) = value.text.as_ref() {
        p = p.field("status", v.as_str());
    }
    if let Some(v) = value.wind360 {
        p = p.field("wind360", v);
    }
    if let Some(v) = value.wind_direction.as_ref() {
        p = p.field("wind_direction", v.as_str());
    }
    if let Some(v) = value.wind_scale {
        p = p.field("wind_scale", v);
    }
    if let Some(v) = value.wind_speed {
        p = p.field("wind_speed", v);
    }
    if let Some(v) = value.humidity {
        p = p.field("humidity", v);
    }
    if let Some(v) = value.precip {
        p = p.field("precip", v);
    }
    if let Some(v) = value.pressure {
        p = p.field("pressure", v);
    }
    if let Some(v) = value.visiblity {
        p = p.field("visiblity", v);
    }
    if let Some(v) = value.cloud {
        p = p.field("cloud", v);
    }
    if let Some(v) = value.dew {
        p = p.field("dew", v);
    }
    Ok(p.build()?)
}

async fn main() {
    info!("start cron");
    let mut iter = SCHEDULER.get().unwrap().upcoming(chrono::Local);
    loop {
        match qweather::get_now().await {
            Ok(now) => match build_datapoint_from_currentweather(&now) {
                Ok(point) => {
                    if let Err(err) = tsdb::write(vec![point]).await {
                        error!("cannot save data: {:}", err);
                    }
                }
                Err(err) => error!("cannot parse returned result: {:}", err),
            },
            Err(err) => error!("failed to get current weather: {:}", err),
        }
        if let Err(err) = qweather::update_24h().await {
            error!("failed to update weather of 24 hours: {:}", err.to_string());
        }
        if let Err(err) = qweather::update_3d().await {
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
            ))
            .await;
        } else {
            break;
        }
    }
}
