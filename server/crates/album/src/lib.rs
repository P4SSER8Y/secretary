mod api;
mod config;
mod converter;
mod discovery;
mod sender;
mod storage;

use rocket::{Build, Rocket};

pub use config::AlbumConfig;

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    config: &rocket::figment::Figment,
) -> anyhow::Result<Rocket<Build>> {
    let cfg = AlbumConfig::from_figment(config)?;

    storage::init(&cfg).await?;
    discovery::set_runtime_handle(tokio::runtime::Handle::current());
    discovery::init(&cfg)?;

    let build = api::build(build, base, cfg).await?;
    Ok(build)
}
