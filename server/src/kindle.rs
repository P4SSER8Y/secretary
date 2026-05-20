use chrono::{self, Local, NaiveDate};
use kindle::Context;
use once_cell::sync::OnceCell;
use rocket::figment::Figment;
use rocket::response::status::NotFound;
use rocket::{http::ContentType, Build, Rocket};
use std::collections::HashMap;
use std::io::Cursor;
use std::vec;

static DEVICE_NAME: OnceCell<String> = OnceCell::new();
static DEVICE_ID: OnceCell<String> = OnceCell::new();
static DASHBOARD_PATH: OnceCell<Option<String>> = OnceCell::new();

pub fn build(base: &'static str, build: Rocket<Build>, config: &Figment) -> Rocket<Build> {
    kindle::set_default_style(
        config
            .find_value("kindle.style")
            .ok()
            .and_then(|x| x.to_i128().and_then(|x| Some(x as usize))),
    );
    let mut font_map = HashMap::new();
    let list = config.find_value("kindle.fonts");
    if let Ok(list) = list {
        if let Some(list) = list.as_dict() {
            for item in list {
                let name = item.0;
                if let Some(path) = item.1.as_str() {
                    let path = std::path::Path::new(utils::get_data_path()).join(path);
                    font_map.insert(name.clone(), path.to_str().unwrap().to_string());
                }
            }
        }
    }
    kindle::load_fonts(font_map);

    let device_name = config
        .find_value("kindle.device_name")
        .ok()
        .and_then(|x| x.into_string())
        .unwrap_or_else(|| "kindle".to_string());
    let device_id = device_name.clone();
    let state_topic = format!("secretary/{}/state", device_id);
    let availability_topic = format!("secretary/{}/availability", device_id);
    DEVICE_NAME.set(device_name.clone()).ok();
    DEVICE_ID.set(device_id.clone()).ok();

    let dashboard_path = config
        .find_value("kindle.dashboard_path")
        .ok()
        .and_then(|x| x.into_string());
    DASHBOARD_PATH.set(dashboard_path).ok();

    mqtt::publish_discovery(&mqtt::HassDeviceConfig {
        name: device_name.to_string(),
        device_id: device_id.clone(),
        state_topic,
        availability_topic: Some(availability_topic),
    });
    mqtt::publish_availability(&device_id, "online");

    build.mount(base, routes![main])
}

async fn save_battery(battery: usize) -> Result<(), anyhow::Error> {
    use influxdb2::models::DataPoint;
    tsdb::write(vec![DataPoint::builder("device")
        .tag("name", "kindle")
        .field("power", battery as f64)
        .build()?])
    .await?;
    Ok(())
}

#[get("/?<battery>&<style>&<now>")]
async fn main(
    battery: Option<usize>,
    style: Option<usize>,
    now: Option<String>,
) -> Result<(ContentType, Vec<u8>), NotFound<()>> {
    info!("{:?}", now);
    let now = match now {
        Some(raw) => {
            let date = NaiveDate::parse_from_str(&raw, "%Y-%m-%d");
            if let Ok(date) = date {
                let naive_datetime = date.and_hms_opt(0, 0, 0).unwrap();
                naive_datetime.and_local_timezone(Local).unwrap()
            } else {
                Local::now()
            }
        }
        None => Local::now(),
    };
    let mut battery = battery;
    if let Some(battery) = battery {
        let db = utils::database::Db::new();
        tokio::spawn(save_battery(battery));
        let _ = db.set("kindle/battery", &battery);
        if battery < 20 {
            bark::send(bark::Message {
                body: &format!("kindle's battery is low: {}%", battery),
                ..Default::default()
            })
            .await;
        }
    } else {
        let db = utils::database::Db::new();
        battery = db.get("kindle/battery").unwrap_or(None);
    }

    let now_local = Local::now();
    let db = utils::database::Db::new();
    let _ = db.set("kindle/last_seen", &now_local.to_rfc3339());

    if let (Some(device_id), Some(_device_name)) = (DEVICE_ID.get(), DEVICE_NAME.get()) {
        mqtt::publish_state(device_id, battery, &now_local.to_rfc3339());
        mqtt::publish_availability(device_id, "online");
    }

    let context = Context {
        battery: battery,
        now: Some(now),
        dashboard_path: DASHBOARD_PATH.get().and_then(|x| x.clone()),
    };
    info!("style={:?}", style);
    info!("now={:?}", context.now);
    info!("battery={:?}", context.battery);

    match kindle::factory(style, &context).await {
        Ok(img) => {
            let mut buffer: Vec<u8> = Vec::new();
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageOutputFormat::Png)
                .expect("failed to encoded image");
            return Ok((ContentType::PNG, buffer));
        }
        Err(e) => {
            error!("{:?}", e);
            return Err(NotFound(()));
        }
    }
}
