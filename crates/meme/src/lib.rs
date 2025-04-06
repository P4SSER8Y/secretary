mod agent;
mod api;
mod data;

use anyhow::anyhow;
use figment::Figment;
#[allow(unused_imports)]
use log::info;
use rocket::{Build, Rocket};

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    let host = config
        .find_value("host")?
        .as_str()
        .ok_or(anyhow!("host not found"))?
        .to_owned();
    let gate = config
        .find_value("meme.jwt_gate")?
        .as_str()
        .ok_or(anyhow!("meme.jwt_gate not found"))?
        .to_owned();
    let endpoint = config
        .find_value("meme.endpoint")?
        .as_str()
        .ok_or(anyhow!("endpoint not found"))?
        .to_owned();
    let region_name = config
        .find_value("meme.region")?
        .as_str()
        .ok_or(anyhow!("region not found"))?
        .to_owned();
    let bucket_name = config
        .find_value("meme.bucket")?
        .as_str()
        .ok_or(anyhow!("bucket not found"))?
        .to_owned();

    let access_key = config
        .find_value("meme.access_key")?
        .as_str()
        .ok_or(anyhow!("access_key not found"))?
        .to_owned();
    let secret_key = config
        .find_value("meme.secret_key")?
        .as_str()
        .ok_or(anyhow!("secret_key not found"))?
        .to_owned();
    let jwt_secret_key = config
        .find_value("meme.jwt_secret_key")?
        .as_str()
        .ok_or(anyhow!("jwt_secret_key not found"))?
        .to_owned();
    let hide = config.find_value("meme.hide")?;
    let hide = hide
        .as_array()
        .ok_or(anyhow!("hide not found"))?
        .iter()
        .map(|item| item.as_str())
        .filter(|item| item.is_some() && item.unwrap().len() > 0)
        .map(|item| item.unwrap())
        .collect();
    let key_file = config
        .find_value("meme.key_file")?
        .as_str()
        .ok_or(anyhow!("key_file not found"))?
        .to_owned();
    let key_salt = config
        .find_value("meme.key_salt")?
        .as_str()
        .ok_or(anyhow!("key_salt not found"))?
        .to_owned();

    agent::init(
        &endpoint,
        &region_name,
        &bucket_name,
        &access_key,
        &secret_key,
        &jwt_secret_key,
        &hide,
        &key_salt,
        &key_file,
    )
    .await?;

    let build = api::build(build, &host, &gate, &base).await?;
    Ok(build)
}
