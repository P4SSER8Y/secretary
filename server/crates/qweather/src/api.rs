use anyhow::{anyhow, Context, Ok, Result};
use chrono::{DateTime, FixedOffset, Local, NaiveTime};
use log::info;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

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
const API_URL_NOW: &str = "https://devapi.qweather.com/v7/weather/now";

static DATA_24H: Lazy<Mutex<Vec<HourlyForecast>>> = Lazy::new(|| Mutex::new(Vec::new()));
static DATA_3D: Lazy<Mutex<Vec<DailyForecast>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub struct Forcast24H {
    pub min_temp: i32,
    pub max_temp: i32,
    pub texts: Vec<String>,
}

pub async fn get_24h_forcast() -> Result<Forcast24H> {
    let data = DATA_24H.lock().await;
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

pub async fn get_3d_forecast() -> Result<Vec<DailyForecast>> {
    let data = DATA_3D.lock().await;
    if data.len() == 0 {
        return Err(anyhow!("no data yet"));
    }
    return Ok(data.clone());
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct HourlyForecastRaw {
    fxTime: String,
    temp: String,
    humidity: String,
    text: String,
}

#[derive(Debug, Clone)]
pub struct HourlyForecast {
    pub fx_time: DateTime<Local>,
    pub temp: i32,
    pub humidity: i32,
    pub text: String,
}

impl std::convert::TryFrom<&HourlyForecastRaw> for HourlyForecast {
    type Error = anyhow::Error;
    fn try_from(value: &HourlyForecastRaw) -> Result<HourlyForecast> {
        let fx_time = DateTime::parse_from_str(&value.fxTime, "%Y-%m-%dT%H:%M%z")
            .or(Err(anyhow!("failed to parse fxTime")))?;
        let fx_time = DateTime::<Local>::try_from(fx_time)?;
        let temp: i32 = value.temp.parse()?;
        let humidity: i32 = value.temp.parse()?;
        return Ok(HourlyForecast {
            fx_time: fx_time,
            temp: temp,
            humidity: humidity,
            text: value.text.clone(),
        });
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct DailyForecastRaw {
    fxDate: String,
    tempMax: String,
    tempMin: String,
    textDay: String,
    iconDay: String,
}

#[derive(Debug, Clone)]
pub struct DailyForecast {
    pub date: DateTime<Local>,
    pub temp_min: i32,
    pub temp_max: i32,
    pub text: String,
    pub icon: String,
}

impl std::convert::TryFrom<&DailyForecastRaw> for DailyForecast {
    type Error = anyhow::Error;
    fn try_from(value: &DailyForecastRaw) -> Result<DailyForecast> {
        let date = chrono::NaiveDate::parse_from_str(&value.fxDate, "%Y-%m-%d")
            .or(Err(anyhow!("failed to parse fxDate")))?
            .and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap())
            .and_local_timezone(Local)
            .single()
            .ok_or(anyhow!("failed to parse fxDate"))?;
        let temp_min: i32 = value.tempMin.parse()?;
        let temp_max: i32 = value.tempMax.parse()?;
        return Ok(DailyForecast {
            date: date,
            temp_max: temp_max,
            temp_min: temp_min,
            text: value.textDay.clone(),
            icon: value.iconDay.clone(),
        });
    }
}

pub async fn set_location(location: String) {
    let mut cfg = CONFIG.lock().await;
    if location.len() > 0 {
        cfg.location = Some(location);
    } else {
        cfg.location = None;
    }
    info!("update {:?}", cfg);
}

pub async fn set_key(key: String) {
    let mut cfg = CONFIG.lock().await;
    if key.len() > 0 {
        cfg.key = Some(key);
    } else {
        cfg.key = None;
    }
    info!("update {:?}", cfg);
}

#[allow(dead_code)]
pub async fn update_24h() -> Result<()> {
    info!("start fetch 24h");
    let cfg = CONFIG.lock().await;
    let params = &[
        ("location", cfg.location.as_ref().unwrap()),
        ("key", cfg.key.as_ref().unwrap()),
    ];
    let url = reqwest::Url::parse_with_params(API_URL_24H, params).unwrap();
    info!("{}", url);
    let query = reqwest::get(url)
        .await
        .or(Err(anyhow!("cannot fetch {:#?}", API_URL_24H)))?;
    let bytes = query
        .bytes()
        .await
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
    let mut raw = DATA_24H.lock().await;

    let hourly = json.get("hourly").ok_or(anyhow!("hourly not found"))?;
    let hour_forcast_raw: Vec<HourlyForecastRaw> = serde_json::from_value(hourly.clone())?;
    *raw = hour_forcast_raw
        .iter()
        .map(|x| TryFrom::try_from(x))
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect::<Vec<HourlyForecast>>();
    return Ok(());
}

#[allow(dead_code)]
pub async fn update_3d() -> Result<()> {
    info!("start update 3d");
    let cfg = CONFIG.lock().await;
    let params = &[
        ("location", cfg.location.as_ref().unwrap()),
        ("key", cfg.key.as_ref().unwrap()),
    ];
    let url = reqwest::Url::parse_with_params(API_URL_3D, params).unwrap();
    let query = reqwest::get(url)
        .await
        .or(Err(anyhow!("cannot fetch {:#?}", API_URL_3D)))?;
    let bytes = query
        .bytes()
        .await
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
    let mut raw = DATA_3D.lock().await;

    let daily = json.get("daily").ok_or(anyhow!("daily not found"))?;
    let daily_forecast_raw: Vec<DailyForecastRaw> = serde_json::from_value(daily.to_owned())?;
    *raw = daily_forecast_raw
        .iter()
        .map(|x| TryFrom::try_from(x))
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect::<Vec<DailyForecast>>();
    return Ok(());
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct CurrentWeatherRaw {
    obsTime: String,
    temp: Option<String>,
    feelsLike: Option<String>,
    text: Option<String>,
    wind360: Option<String>,
    windDir: Option<String>,
    windScale: Option<String>,
    windSpeed: Option<String>,
    humidity: Option<String>,
    precip: Option<String>,
    pressure: Option<String>,
    vis: Option<String>,
    cloud: Option<String>,
    dew: Option<String>,
}

#[derive(Debug)]
pub struct CurrentWeather {
    pub observe_time: DateTime<FixedOffset>,
    pub temperature: Option<f64>,
    pub feels_like: Option<f64>,
    pub text: Option<String>,
    pub wind360: Option<i64>,
    pub wind_direction: Option<String>,
    pub wind_scale: Option<f64>,
    pub wind_speed: Option<f64>,
    pub humidity: Option<f64>,
    pub precip: Option<f64>,
    pub pressure: Option<f64>,
    pub visiblity: Option<f64>,
    pub cloud: Option<f64>,
    pub dew: Option<f64>,
}

impl std::convert::TryFrom<&CurrentWeatherRaw> for CurrentWeather {
    type Error = anyhow::Error;
    fn try_from(v: &CurrentWeatherRaw) -> Result<CurrentWeather> {
        let observe_time = chrono::DateTime::parse_from_str(&v.obsTime, "%FT%R%:z")
            .with_context(|| anyhow!("cannot decode datetime"))?;
        let temperature = v.temp.as_ref().and_then(|x| x.parse().ok());
        let feels_like = v.feelsLike.as_ref().and_then(|x| x.parse().ok());
        let text = v.text.as_ref().and_then(|x| Some(x.clone()));
        let wind360 = v.wind360.as_ref().and_then(|x| x.parse().ok());
        let wind_direction = v.windDir.as_ref().and_then(|x| Some(x.clone()));
        let wind_scale = v.windScale.as_ref().and_then(|x| x.parse().ok());
        let wind_speed = v.windSpeed.as_ref().and_then(|x| x.parse().ok());
        let humidity = v.humidity.as_ref().and_then(|x| x.parse().ok());
        let precip = v.precip.as_ref().and_then(|x| x.parse().ok());
        let pressure = v.pressure.as_ref().and_then(|x| x.parse().ok());
        let visiblity = v.vis.as_ref().and_then(|x| x.parse().ok());
        let cloud = v.cloud.as_ref().and_then(|x| x.parse().ok());
        let dew = v.dew.as_ref().and_then(|x| x.parse().ok());
        Ok(CurrentWeather {
            observe_time,
            temperature,
            feels_like,
            text,
            wind360,
            wind_direction,
            wind_scale,
            wind_speed,
            humidity,
            precip,
            pressure,
            visiblity,
            cloud,
            dew,
        })
    }
}

pub async fn get_now() -> Result<CurrentWeather> {
    info!("fetch current weather");
    let cfg = CONFIG.lock().await;
    let params = &[
        ("location", cfg.location.as_ref().unwrap()),
        ("key", cfg.key.as_ref().unwrap()),
    ];
    let url = reqwest::Url::parse_with_params(API_URL_NOW, params).unwrap();
    let query = reqwest::get(url)
        .await
        .or(Err(anyhow!("cannot fetch {:#?}", API_URL_NOW)))?;
    let bytes = query
        .bytes()
        .await
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

    let now = json.get("now").ok_or(anyhow!("now not found"))?;
    let raw: CurrentWeatherRaw = serde_json::from_value(now.to_owned())?;
    CurrentWeather::try_from(&raw)
}
