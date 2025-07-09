use std::{path::Path, time::SystemTime};

use rocket::{figment::Figment, get, routes, serde::json::Json, Build, Rocket};
use utils::database::get_db;

use crate::{api::{Status, PATH, PREFIX}, metadata::Metadata};

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    Ok(build.mount(base, routes![drop, empty]))
}

pub async fn drop_item(id: &str) {
    log::warn!("drop {}", id);
    let db = get_db();
    let key = format!("{}{}", PREFIX, id);
    let _ = db.remove(key);
    let path = Path::new(PATH.get().unwrap()).join(id);
    let _ = tokio::fs::remove_file(path).await;
}

#[get("/drop/<id>")]
async fn drop(id: &str) -> Json<Status> {
    drop_item(id).await;
    Json(Status::Ok {
        ok: true,
        message: format!("dropped {}", id),
    })
}

pub async fn remove_expired() {
    let db = get_db();
    let now = SystemTime::now();
    for item in db
        .scan_prefix(PREFIX)
        .filter(|x| x.is_ok())
        .map(|x| Metadata::try_from(&(x.unwrap().1)))
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
    {
        if now > item.expiration {
            drop_item(&item.id).await;
        }
    }
}

pub async fn auto_empty() -> u32 {
    let db = get_db();
    let mut count: u32 = 0;
    for k in db.scan_prefix(PREFIX).keys() {
        if let Ok(k) = k {
            if let Ok(k) = String::from_utf8(k.to_vec()) {
                let id = k.split_at(PREFIX.len()).1;
                drop_item(&id).await;
                count += 1;
            }
        }
    }
    count
}

#[get("/empty")]
pub async fn empty() -> Json<Status> {
    Json(Status::Ok {
        ok: true,
        message: format!("clean up {} items", auto_empty().await),
    })
}
