mod api;
mod metadata;

extern crate rocket;
use rocket::{figment::Figment, routes, Build, Rocket};

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    let data = std::path::Path::new(utils::get_data_path());
    let path = data.join(api::PREFIX);
    tokio::fs::create_dir_all(&path).await?;
    api::init_path(path.to_str().unwrap());
    api::remove_expired().await;
    Ok(build.mount(
        base,
        routes![
            api::new_file,
            api::new_text,
            api::drop,
            api::list,
            api::get,
            api::empty
        ],
    ))
}
