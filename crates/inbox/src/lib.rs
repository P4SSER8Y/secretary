mod api;
mod drop;
mod get;
mod list;
mod metadata;
mod new;

extern crate rocket;
use drop::remove_expired;
use rocket::{figment::Figment, Build, Rocket};

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    let data = std::path::Path::new(utils::get_data_path());
    let path = data.join(api::PREFIX);
    tokio::fs::create_dir_all(&path).await?;
    api::init_path(path.to_str().unwrap());
    remove_expired().await;
    let build = new::build(base, build, config).await?;
    let build = drop::build(base, build, config).await?;
    let build = list::build(base, build, config).await?;
    let build = get::build(base, build, config).await?;
    Ok(build)
}
