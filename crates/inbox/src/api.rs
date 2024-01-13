use crate::metadata;

use super::metadata::Metadata;
use rocket::{
    data::ToByteUnit,
    form::Form,
    fs::TempFile,
    get, post,
    response::{self, status::NotFound, Responder},
    serde::json::Json,
    Data, FromForm, Response,
};
use serde::Serialize;
use sled::IVec;
use std::{path::Path, sync::OnceLock, time::SystemTime};
use tokio::fs::File;
use utils::database::get_db;

pub const PREFIX: &str = "inbox/";
static PATH: OnceLock<String> = OnceLock::new();

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Status {
    Ok { ok: bool, message: String },
    NewItem { ok: bool, id: String },
    CommonError { ok: bool, message: String },
}

pub fn init_path(path: &str) {
    PATH.get_or_init(|| path.to_string());
}

#[get("/list")]
pub async fn list() -> Json<Vec<Metadata>> {
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

#[derive(FromForm)]
pub struct NewData<'r> {
    file: TempFile<'r>,
}

#[post(
    "/new?<public>&<expire>",
    data = "<data>",
    format = "multipart/form-data",
    rank = 1
)]
pub async fn new_file(
    mut data: Form<NewData<'_>>,
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
    let name = data
        .file
        .raw_name()
        .and_then(|x| Some(x.dangerous_unsafe_unsanitized_raw().to_string()))
        .unwrap_or(String::new());
    let expiration = SystemTime::now() + humantime::parse_duration(&expire).unwrap();
    let meta = Metadata {
        id: id,
        kind: metadata::Kind::File,
        name: name,
        expiration: expiration,
        size: data.file.len(),
        public: public,
        file: None,
    };
    log::debug!("id={} key={} meta={:?}", meta.id, key, meta);
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
pub async fn new_text(
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
        kind: metadata::Kind::Text,
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

async fn drop_item(id: &str) {
    log::warn!("drop {}", id);
    let db = get_db();
    let key = format!("{}{}", PREFIX, id);
    let _ = db.remove(key);
    let path = Path::new(PATH.get().unwrap()).join(id);
    let _ = tokio::fs::remove_file(path).await;
}

#[get("/drop/<id>")]
pub async fn drop(id: &str) -> Json<Status> {
    drop_item(id).await;
    Json(Status::Ok {
        ok: true,
        message: format!("dropped {}", id),
    })
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for Metadata {
    fn respond_to(self, _request: &'r rocket::Request<'_>) -> response::Result<'static> {
        let filename = match self.kind {
            metadata::Kind::File => self.name,
            metadata::Kind::Text => self.id,
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
