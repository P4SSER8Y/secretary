use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug)]
struct Config {
    location: Option<String>,
    key: Option<String>,
}

static CONFIG: Lazy<Mutex<Config>> = Lazy::new(|| {
    return Mutex::new(Config {
        location: None,
        key: None,
    });
});

// cf. https://dev.qweather.com/docs/api/weather/weather-daily-forecast/
const API_URL_3D: &str = "https://devapi.qweather.com/v7/weather/3d";
// cf. https://dev.qweather.com/docs/api/weather/weather-now/
const API_URL_24H: &str = "https://devapi.qweather.com/v7/weather/24h";
static DATA_24H: Lazy<Mutex<Vec<HourForcase>>> = Lazy::new(|| Mutex::new(Vec::new()));
static RAW_DATA_3D: Lazy<Mutex<serde_json::Value>> =
    Lazy::new(|| Mutex::new(serde_json::Value::Null));

pub struct Forcast24H {
    pub min_temp: i32,
    pub max_temp: i32,
    pub texts: Vec<String>,
}

pub fn get_24h_forcast() -> Result<Forcast24H> {
    let data = DATA_24H.lock().or(Err(anyhow!("cannot acquire lock")))?;
    if data.len() == 0 {
        return Err(anyhow!("no data yet"));
    }
    let min_temp = data.iter().map(|x| x.temp).min().ok_or(anyhow!("wtf"))?;
    let max_temp = data.iter().map(|x| x.temp).max().ok_or(anyhow!("wtf"))?;
    let mut texts = vec![data[0].text.clone()];
    for i in 1..data.len() {
        if data[i].text.ne(texts.last().unwrap()) {
            texts.push(data[i].text.clone());
        }
    }
    return Ok(Forcast24H {
        min_temp,
        max_temp,
        texts,
    });
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct HourForcastRaw {
    fxTime: String,
    temp: String,
    humidity: String,
    text: String,
}

#[derive(Debug)]
pub struct HourForcase {
    pub fx_time: DateTime<Local>,
    pub temp: i32,
    pub humidity: i32,
    pub text: String,
}

impl std::convert::TryFrom<&HourForcastRaw> for HourForcase {
    type Error = anyhow::Error;

    fn try_from(value: &HourForcastRaw) -> Result<HourForcase> {
        let fx_time = DateTime::parse_from_str(&value.fxTime, "%Y-%m-%dT%H:%M%z")
            .or(Err(anyhow!("failed to parse fxTime")))?;
        let fx_time = DateTime::<Local>::try_from(fx_time)?;
        let temp: i32 = value.temp.parse()?;
        let humidity: i32 = value.temp.parse()?;
        return Ok(HourForcase {
            fx_time: fx_time,
            temp: temp,
            humidity: humidity,
            text: value.text.clone(),
        });
    }
}

pub fn set_location(location: String) {
    let mut cfg = CONFIG.lock().unwrap();
    if location.len() > 0 {
        cfg.location = Some(location);
    } else {
        cfg.location = None;
    }
    info!("update {:?}", cfg);
}

pub fn set_key(key: String) {
    let mut cfg = CONFIG.lock().unwrap();
    if key.len() > 0 {
        cfg.key = Some(key);
    } else {
        cfg.key = None;
    }
    info!("update {:?}", cfg);
}

#[allow(dead_code)]
pub fn update_24h() -> Result<()> {
    let cfg = CONFIG.lock().or(Err(anyhow!("failed to acquire lock")))?;
    let params = &[
        ("location", cfg.location.as_ref().unwrap()),
        ("key", cfg.key.as_ref().unwrap()),
    ];
    let url = reqwest::Url::parse_with_params(API_URL_24H, params).unwrap();
    info!("{}", url);
    let query = reqwest::blocking::get(url).or(Err(anyhow!("cannot fetch {:#?}", API_URL_24H)))?;
    let bytes = query
        .bytes()
        .or(Err(anyhow!("failed to turn content into bytes")))?
        .to_vec();
    let json = serde_json::from_slice::<serde_json::Value>(&bytes)
        .or(Err(anyhow!("failed to parse into json")))?;
    if !json.is_object() {
        return Err(anyhow!("json is not an valid object"));
    }
    let code = json
        .get("code")
        .ok_or(anyhow!("code not found"))?
        .as_str()
        .ok_or(anyhow!("code is not a string"))?;
    if code != "200" {
        return Err(anyhow!("query failed"));
    }
    let mut raw = DATA_24H.lock().or(Err(anyhow!("failed to acquire lock")))?;

    let hourly = json.get("hourly").ok_or(anyhow!("hourly not found"))?;
    let hour_forcast_raw: Vec<HourForcastRaw> = serde_json::from_value(hourly.clone())?;
    *raw = hour_forcast_raw
        .iter()
        .map(|x| TryFrom::try_from(x))
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect::<Vec<HourForcase>>();
    return Ok(());
}

#[allow(dead_code)]
pub fn update_3d() -> Result<()> {
    let cfg = CONFIG.lock().or(Err(anyhow!("failed to acquire lock")))?;
    let params = &[
        ("location", cfg.location.as_ref().unwrap()),
        ("key", cfg.key.as_ref().unwrap()),
    ];
    let url = reqwest::Url::parse_with_params(API_URL_3D, params).unwrap();
    let query = reqwest::blocking::get(url).or(Err(anyhow!("cannot fetch {:#?}", API_URL_24H)))?;
    let bytes = query
        .bytes()
        .or(Err(anyhow!("failed to turn content into bytes")))?
        .to_vec();
    let json = serde_json::from_slice::<serde_json::Value>(&bytes)
        .or(Err(anyhow!("failed to parse into json")))?;
    if !json.is_object() {
        return Err(anyhow!("json is not an valid object"));
    }
    let code = json
        .get("code")
        .ok_or(anyhow!("code not found"))?
        .as_str()
        .ok_or(anyhow!("code is not a string"))?;
    if code != "200" {
        return Err(anyhow!("query failed"));
    }
    let mut raw = RAW_DATA_3D
        .lock()
        .or(Err(anyhow!("failed to acquire lock")))?;
    *raw = json;
    return Ok(());
}
