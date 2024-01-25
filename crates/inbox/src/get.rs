use std::{time::SystemTime, path::Path};

use rocket::{Rocket, Build, figment::Figment, get, routes, Response, response::{self, Responder, status::NotFound}};
use tokio::fs::File;
use utils::database::get_db;

use crate::{metadata::{Metadata, Kind}, api::{PREFIX, PATH}, drop::drop_item};

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    Ok(build.mount(base, routes![get]))
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for Metadata {
    fn respond_to(self, _request: &'r rocket::Request<'_>) -> response::Result<'static> {
        let filename = match self.kind {
            Kind::File => self.name,
            Kind::Text => self.id,
        };
        Response::build()
            .streamed_body(self.file.unwrap())
            .raw_header(
                "Content-Disposition",
                format!("attachment;filename=\"{}\"", filename),
            )
            .ok()
    }
}

#[get("/get/<id>")]
pub async fn get(id: &str) -> Result<Metadata, NotFound<()>> {
    let db = get_db();
    let key = format!("{}{}", PREFIX, id);
    if let Ok(Some(value)) = db.get(key) {
        if let Ok(meta) = Metadata::try_from(&value) {
            let now = SystemTime::now();
            if now > meta.expiration {
                drop_item(id).await;
            } else {
                if let Ok(file) = File::open(Path::new(PATH.get().unwrap()).join(id)).await {
                    let meta = Metadata {
                        file: Some(file),
                        ..meta
                    };
                    return Ok(meta);
                }
            }
        }
    }
    Err(NotFound(()))
}
