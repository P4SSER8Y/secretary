mod agent;
mod api;
mod data;

use figment::Figment;
#[allow(unused_imports)]
use log::info;
use rocket::{Build, Rocket};

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    agent::init(config).await?;

    let build = api::build(build,  &base, config).await?;
    Ok(build)
}
