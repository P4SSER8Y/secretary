use chrono::{self, Local, NaiveDate};
use kindle::Context;
use serde::{Deserialize, Serialize};
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

    build.mount(base, routes![main, get_cfg, set_cfg])
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


// ---------- Kindle 配置端点 ----------
//
//   GET  /kindle/cfg                -> JSON
//   GET  /kindle/cfg?format=text    -> key=value（Kindle 端解析这个，省得解析 JSON）
//   POST /kindle/cfg                -> form: sleep=0&interval=600&style=2（只改传了的字段）
//
// 存在 sled 的 kindle/config 键里，和 kindle/battery 同一个库。

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cfg {
    sleep: bool,
    interval: u64,
    style: Option<usize>,
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            sleep: true,
            interval: 10800,
            style: None,
        }
    }
}

const CFG_KEY: &str = "kindle/config";

fn load_cfg() -> Cfg {
    utils::database::Db::new()
        .get::<Cfg>(CFG_KEY)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn save_cfg(cfg: &Cfg) {
    let _ = utils::database::Db::new().set(CFG_KEY, cfg);
}

fn now_epoch() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

fn cfg_json(cfg: &Cfg) -> String {
    serde_json::json!({
        "sleep": cfg.sleep,
        "interval": cfg.interval,
        "style": cfg.style,
        "next_refresh_in": if cfg.sleep { cfg.interval } else { 0 },
        "server_time": now_epoch(),
    })
    .to_string()
}

fn cfg_text(cfg: &Cfg) -> String {
    format!(
        "sleep={}\ninterval={}\nstyle={}\nnext_refresh_in={}\nserver_time={}\n",
        if cfg.sleep { 1 } else { 0 },
        cfg.interval,
        cfg.style.map(|s| s.to_string()).unwrap_or_default(),
        if cfg.sleep { cfg.interval } else { 0 },
        now_epoch(),
    )
}

fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let hex = |x: u8| -> Option<u8> {
                    match x {
                        b'0'..=b'9' => Some(x - b'0'),
                        b'a'..=b'f' => Some(x - b'a' + 10),
                        b'A'..=b'F' => Some(x - b'A' + 10),
                        _ => None,
                    }
                };
                if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                    out.push(h * 16 + l);
                    i += 3;
                    continue;
                }
                out.push(b[i]);
            }
            x => out.push(x),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

// 只认 sleep / interval / style 三个字段，非法值忽略
fn apply_form(cfg: &mut Cfg, body: &str) {
    for pair in body.split('&') {
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("").trim();
        let v = url_decode(it.next().unwrap_or("").trim());
        match k {
            "sleep" => cfg.sleep = !(v == "0" || v == "false" || v.is_empty()),
            "interval" => {
                if let Ok(n) = v.parse::<u64>() {
                    cfg.interval = n.min(30 * 24 * 3600);
                }
            }
            "style" => {
                if v.is_empty() {
                    cfg.style = None;
                } else if let Ok(n) = v.parse::<usize>() {
                    cfg.style = Some(n);
                }
            }
            _ => {}
        }
    }
}

#[get("/cfg?<format>")]
fn get_cfg(format: Option<String>) -> (ContentType, String) {
    let cfg = load_cfg();
    match format.as_deref() {
        Some("text") => (ContentType::Plain, cfg_text(&cfg)),
        _ => (ContentType::JSON, cfg_json(&cfg)),
    }
}

#[post("/cfg", data = "<body>")]
fn set_cfg(body: String) -> (ContentType, String) {
    let mut cfg = load_cfg();
    apply_form(&mut cfg, &body);
    save_cfg(&cfg);
    info!("kindle config updated: {:?}", cfg);
    (ContentType::JSON, cfg_json(&cfg))
}
