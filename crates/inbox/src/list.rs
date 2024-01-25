use crate::drop::remove_expired;

use super::api::PREFIX;
use super::metadata::Metadata;
use rocket::{figment::Figment, get, routes, serde::json::Json, Build, Rocket};
use utils::database::get_db;

#[get("/list")]
async fn list() -> Json<Vec<Metadata>> {
    let db = get_db();
    remove_expired().await;
    let mut v: Vec<Metadata> = db
        .scan_prefix(PREFIX)
        .filter(|x| x.is_ok())
        .map(|x| Metadata::try_from(&(x.unwrap().1)))
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .filter(|x| x.public)
        .collect();
    v.sort_unstable_by_key(|x| x.expiration);
    Json::from(v)
}

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    Ok(build.mount(base, routes![list]))
}
