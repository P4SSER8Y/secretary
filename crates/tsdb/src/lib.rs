use anyhow::anyhow;
use figment::Figment;
use influxdb2::{models::DataPoint, Client};
use once_cell::sync::OnceCell;
use futures::prelude::stream;

struct Config {
    host: String,
    org: String,
    token: String,
    bucket: String,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

fn parse_string(figment: &Figment, key: &str) -> Result<String, anyhow::Error> {
    figment.find_value(key)?.into_string().ok_or_else(|| {
        anyhow!(format!("unknown {}", key))
    })
}

fn init_config(figment: &Figment) -> Result<Config, anyhow::Error> {
    let host = parse_string(figment, "tsdb.host")?;
    let org = parse_string(figment, "tsdb.org")?;
    let token = parse_string(figment, "tsdb.token")?;
    let bucket = parse_string(figment, "tsdb.bucket")?;
    Ok(Config {
        host,
        org,
        token,
        bucket,
    })
}

pub async fn init(figment: &Figment) {
    let cfg = init_config(figment);
    if cfg.is_err() {
        log::error!("initialize TSDB failed");
        return;
    }
    let cfg = cfg.unwrap();
    CONFIG.get_or_init(|| cfg);
}

pub async fn write(points: Vec<DataPoint>) -> Result<(), anyhow::Error> {
    let config = CONFIG.get().ok_or(anyhow!("TSDB not initialized"))?;
    let client = Client::new(&config.host, &config.org, &config.token);
    client.write(&config.bucket, stream::iter(points)).await?;
    Ok(())
}
