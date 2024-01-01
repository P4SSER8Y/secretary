use log::{info, error};
use once_cell::sync::OnceCell;
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub enum PushLevel {
    Active,
    TimeSensitive,
    Passive,
}

impl Serialize for PushLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PushLevel::Active => serializer.serialize_str("active"),
            PushLevel::TimeSensitive => serializer.serialize_str("timeSensitive"),
            PushLevel::Passive => serializer.serialize_str("passive"),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Message<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<&'a str>,
    pub body: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<PushLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<i32>,
    #[serde(rename= "autoCopy", skip_serializing_if = "Option::is_none")]
    pub auto_copy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<&'a str>,
}

impl Default for Message<'_> {
    fn default() -> Self {
        Self {
            title: None,
            body: "",
            level: None,
            badge: None,
            auto_copy: None,
            copy: None,
            icon: None,
            url: None,
        }
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    url: String,
}

static CONFIG: OnceCell<Config> = OnceCell::new();
pub fn build(figment: &figment::Figment) {
    if let Ok(cfg) = figment.find_value("bark") {
        if let Ok(cfg) = cfg.deserialize::<Config>() {
            info!("{:?}", cfg);
            CONFIG.get_or_init(|| cfg);
        }
    }
    send(&Message {
        body: "Hello World",
        title: Some("Lighter"),
        ..Default::default()
    });
}

pub fn send(msg: &Message) {
    let url = &CONFIG.get().unwrap().url;
    let client = reqwest::blocking::Client::new();
    let response = client.post(url).json(msg).send();
    if let Err(response) = response {
        error!("failed to bark: {:?}", response);
    }
}
