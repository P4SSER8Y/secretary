use std::path::Path;
use std::time::SystemTime;

use crate::api::{NewData, Status, PATH, PREFIX};
use crate::metadata::{Kind, Metadata};
use rocket::data::ToByteUnit;
use rocket::form::Form;
use rocket::serde::json::Json;
use rocket::Data;
use rocket::{figment::Figment, post, routes, Build, Rocket};
use sled::IVec;

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    Ok(build.mount(base, routes![new_file, new_text]))
}

#[post(
    "/new",
    data = "<data>",
    format = "multipart/form-data",
    rank = 1
)]
async fn new_file(
    data: Form<NewData<'_>>,
) -> Result<Json<Status>, Json<Status>> {
    let public = data.public.unwrap_or(true);
    let expire = data.expire.clone().unwrap_or_else(|| "7d".to_string());
    let db = utils::database::get_db();
    let (key, id) = loop {
        let id = rand::random::<u32>() % 10_000;
        let id = format!("{:04}", id);
        let key = format!("{}{}", PREFIX, id);
        if !db.contains_key(&key).unwrap() {
            break (key, id);
        }
    };
    let name = data
        .file
        .raw_name()
        .and_then(|x| Some(x.dangerous_unsafe_unsanitized_raw().to_string()))
        .unwrap_or(String::new());
    let expiration = SystemTime::now() + humantime::parse_duration(&expire).unwrap();
    let meta = Metadata {
        id: id,
        kind: Kind::File,
        name: name,
        expiration: expiration,
        size: data.file.len(),
        public: public,
        file: None,
    };
    log::debug!("id={} key={} meta={:?}", meta.id, key, meta);
    let mut data = data;
    match data
        .file
        .move_copy_to(Path::new(PATH.get().unwrap()).join(&meta.id))
        .await
    {
        Ok(_) => {
            let id = meta.id.clone();
            let value: IVec = IVec::try_from(&meta).unwrap();
            db.insert(key, value).unwrap();
            Ok(Json(Status::NewItem { ok: true, id: id }))
        }
        Err(err) => Err(Json(Status::CommonError {
            ok: false,
            message: err.to_string(),
        })),
    }
}

#[post("/new?<public>&<expire>", data = "<data>", rank = 2)]
async fn new_text(
    data: Data<'_>,
    public: Option<bool>,
    expire: Option<String>,
) -> Result<Json<Status>, Json<Status>> {
    let public = public.unwrap_or(true);
    let expire = expire.unwrap_or_else(|| "7d".to_string());
    let db = utils::database::get_db();
    let (key, id) = loop {
        let id = rand::random::<u32>() % 10_000;
        let id = format!("{:04}", id);
        let key = format!("{}{}", PREFIX, id);
        if !db.contains_key(&key).unwrap() {
            break (key, id);
        }
    };
    let expiration = SystemTime::now() + humantime::parse_duration(&expire).unwrap();
    let meta = Metadata {
        id: id,
        kind: Kind::Text,
        name: "".to_string(),
        expiration: expiration,
        size: 0,
        public: public,
        file: None,
    };
    log::debug!("id={} key={} meta={:?}", meta.id, key, meta);
    match data
        .open(32.mebibytes())
        .into_file(Path::new(PATH.get().unwrap()).join(&meta.id))
        .await
    {
        Ok(_) => {
            let id = meta.id.clone();
            let value: IVec = IVec::try_from(&meta).unwrap();
            db.insert(key, value).unwrap();
            Ok(Json(Status::NewItem { ok: true, id: id }))
        }
        Err(err) => Err(Json(Status::CommonError {
            ok: false,
            message: err.to_string(),
        })),
    }
}
