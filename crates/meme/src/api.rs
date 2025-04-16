use crate::{
    agent::{self, get_content, MetaData, RawImage, TokenPayload},
    data::{BriefMetaData, FileContent, ListInfo, UploadedImage},
};
use anyhow::{anyhow, Context};
#[allow(unused_imports)]
use log::{debug, info};
use rand::Rng;
use rocket::{
    form::Form,
    futures::future,
    get,
    http::{ContentType, CookieJar, Status},
    post,
    response::{status::NotFound, Redirect},
    routes,
    serde::json::Json,
    tokio::{self, sync::RwLock},
    Build, Rocket,
};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime},
};
use uuid::Uuid;

struct ToDeleteRecord {
    ts: SystemTime,
    meta: Arc<MetaData>,
}
static TO_DELETE_BUFFER: OnceLock<RwLock<HashMap<String, ToDeleteRecord>>> = OnceLock::new();

static HOST: OnceLock<String> = OnceLock::new();
static GATE: OnceLock<String> = OnceLock::new();
static BASE: OnceLock<String> = OnceLock::new();
static EALBUM_WIDTH: OnceLock<u32> = OnceLock::new();
static EALBUM_HEIGHT: OnceLock<u32> = OnceLock::new();

#[get("/check?<token>")]
async fn check_with_token(
    token: &str,
    _data: TokenPayload,
    cookies: &CookieJar<'_>,
) -> (ContentType, String) {
    cookies.add(("token", token.to_string()));
    (
        ContentType::HTML,
        format!(
            r#"<html><head><meta http-equiv="refresh" content="0;url={}"></head><body></body></html>"#,
            BASE.get().unwrap().clone() + "check",
        ),
    )
}

#[get("/check")]
async fn check(data: TokenPayload) -> Result<String, NotFound<()>> {
    let name = data.name.clone();
    tokio::spawn(async move {
        let _ = agent::force_update(&name).await;
    });
    Ok(format!("Hello {} of {}", data.name, data.family))
}

#[get("/login")]
async fn login() -> Redirect {
    let host = HOST.get();
    let host = if let Some(host) = host {
        host
    } else {
        "http://localhost"
    };

    let callback = url::Url::parse(host).unwrap();
    let callback = callback.join(BASE.get().unwrap()).unwrap();
    let callback = callback.join("check").unwrap();
    debug!("redirect to {}", callback);

    let gate = url::Url::parse_with_params(
        GATE.get().unwrap(),
        &[("c", callback.as_str()), ("f", "meme"), ("t", "1")],
    )
    .unwrap();

    info!("{:}", gate);
    Redirect::to(gate.to_string())
}

#[get("/logout")]
async fn logout(cookies: &CookieJar<'_>) -> &'static str {
    cookies.remove("token");
    "Bye"
}

#[get("/<_..>", rank = 99)]
async fn everything() -> (Status, &'static str) {
    (Status::Unauthorized, "WTF")
}

#[get("/list?<filter>&<sort>&<asc>&<s>&<e>")]
async fn list(
    data: TokenPayload,
    filter: Option<&str>,
    sort: Option<&str>,
    asc: bool,
    s: Option<usize>,
    e: Option<usize>,
) -> Json<ListInfo> {
    debug!(
        "filter={:?} sort={:?} asc={} range={:?}:{:?}",
        filter, sort, asc, s, e
    );
    let list = agent::list(&data.name, filter).await;
    let mut list = list.unwrap_or(Vec::new());
    let s = s.unwrap_or(0);
    let e = e.unwrap_or(list.len());
    if let Some(sort) = sort {
        let sort = sort.trim().to_ascii_lowercase();
        let sort = sort.as_str();
        if asc {
            list.sort_by(|a, b| match sort {
                "ts" => a.timestamp.cmp(&b.timestamp),
                _ => a.uuid.cmp(&b.uuid),
            });
        } else {
            list.sort_by(|b, a| match sort {
                "ts" => a.timestamp.cmp(&b.timestamp),
                _ => a.uuid.cmp(&b.uuid),
            });
        }
    }
    let list = list.iter().skip(s).take(e - s);
    let list: Vec<_> = list.map(|v| v.as_ref().into()).collect();
    Json(ListInfo { meta: list })
}

#[get("/latest/<n>?<filter>")]
async fn latest(data: TokenPayload, n: Option<usize>, filter: Option<&str>) -> FileContent {
    let n = n.unwrap_or(0);
    let mut list = agent::list(&data.name, filter).await.unwrap_or(Vec::new());
    list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let result = list.get(n);
    match result {
        Some(meta) => FileContent {
            content_type: meta.content_type.to_string(),
            name: meta.filename.to_owned(),
            body: get_content(&format!("raw/{}/{}", meta.owner, meta.filename)).await,
        },
        None => FileContent {
            content_type: "text/plain".to_string(),
            name: "".to_owned(),
            body: Err(anyhow!("not found")),
        },
    }
}

#[get("/random?<t>&<filter>")]
async fn random(data: TokenPayload, t: bool, filter: Option<&str>) -> FileContent {
    let list = agent::list(&data.name, filter).await;
    let list = list.unwrap_or(Vec::new());
    if list.len() == 0 {
        return FileContent {
            content_type: "text/plain".to_string(),
            name: "".to_owned(),
            body: Err(anyhow!("WTF")),
        };
    }
    let idx = rand::rng().random_range(0..list.len());
    let item = &list[idx];
    if t {
        let data = agent::get_content(&format!(
            "{}/{}/{}",
            "thumbnail", item.owner, item.thumbnail
        ))
        .await;
        FileContent {
            content_type: match &item.thumbnail_content_type {
                Some(v) => v.clone(),
                None => "image/jpeg".to_string(),
            },
            name: item.thumbnail.clone(),
            body: data,
        }
    } else {
        let data = agent::get_content(&format!("{}/{}/{}", "raw", item.owner, item.filename)).await;
        FileContent {
            content_type: item.content_type.clone(),
            name: item.filename.clone(),
            body: data,
        }
    }
}

#[cfg(debug_assertions)]
#[post("/thumbnail", data = "<data>", format = "multipart/form-data")]
async fn preview_thumbnail(data: Form<UploadedImage<'_>>) -> (ContentType, Vec<u8>) {
    let body = data.file;
    if body.len() == 0 {
        return (ContentType::Text, "failed to read data".as_bytes().to_vec());
    }
    let data = agent::generate_thumbnail(body).await;
    match data {
        Ok(data) => (
            ContentType::parse_flexible(data.mime_type).unwrap_or(ContentType::Any),
            data.data,
        ),
        Err(_) => (
            ContentType::Text,
            "failed to generate thumbnail".as_bytes().to_vec(),
        ),
    }
}

#[get("/raw/<uuid>")]
async fn get_raw(uuid: &str, token: TokenPayload) -> FileContent {
    let meta = agent::get_meta_by_uuid(&token.name, uuid).await;
    if meta.is_err() {
        return FileContent {
            content_type: "text/plain".to_owned(),
            name: "".to_string(),
            body: Err(meta.err().unwrap()),
        };
    }
    let meta = meta.unwrap();
    let data = agent::get_content(&format!("raw/{}/{}", meta.owner, meta.filename)).await;
    if data.is_err() {
        return FileContent {
            content_type: "text/plain".to_owned(),
            name: "".to_owned(),
            body: data,
        };
    }
    let data = data.unwrap();
    FileContent {
        content_type: meta.content_type.to_owned(),
        name: meta.filename.to_owned(),
        body: Ok(data),
    }
}

#[get("/thumbnail/<uuid>")]
async fn get_thumbnail(uuid: &str, token: TokenPayload) -> FileContent {
    let meta = agent::get_meta_by_uuid(&token.name, uuid).await;
    if meta.is_err() {
        return FileContent {
            content_type: "text/plain".to_owned(),
            name: "".to_owned(),
            body: Err(meta.err().unwrap()),
        };
    }
    let meta = meta.unwrap();
    let data = agent::get_content(&format!("thumbnail/{}/{}", meta.owner, meta.thumbnail)).await;
    if data.is_err() {
        return FileContent {
            content_type: "text/plain".to_owned(),
            name: "".to_owned(),
            body: data,
        };
    }
    let data = data.unwrap();
    let content_type = match &meta.thumbnail_content_type {
        Some(content_type) => content_type.clone(),
        None => "image/jpeg".to_owned(),
    };
    FileContent {
        content_type: content_type,
        name: meta.thumbnail.clone(),
        body: Ok(data),
    }
}

#[post("/upload", data = "<data>", format = "multipart/form-data")]
async fn upload(data: Form<UploadedImage<'_>>, token: TokenPayload) -> (Status, String) {
    let body = data.file;
    if body.len() == 0 {
        return (Status::NoContent, "empty file".to_string());
    }
    let uuid = uuid::Uuid::new_v4().to_string();
    let thumbnail = agent::generate_thumbnail(body).await;
    if thumbnail.is_err() {
        return (Status::InternalServerError, "failed to convert".to_string());
    }
    let thumbnail = thumbnail.unwrap();
    let raw_format = agent::guess_image_mime_type(data.file)
        .await
        .unwrap_or(RawImage {
            data: Vec::new(),
            mime_type: "application/octet-stream",
            extension: "",
        });
    let tags = agent::split_tags(data.tags);
    let meta = MetaData {
        timestamp: chrono::Utc::now().to_rfc3339(),
        content_type: raw_format.mime_type.to_string(),
        filename: format!("{}.{}", uuid, raw_format.extension),
        thumbnail_content_type: Some(thumbnail.mime_type.to_string()),
        thumbnail: format!("{}.{}", uuid, thumbnail.extension),
        uuid: uuid.clone(),
        owner: token.name.to_string(),
        size: data.file.len(),
        tags: tags.iter().map(|v| v.to_string()).collect(),
        lower_tags: tags.iter().map(|v| v.to_ascii_lowercase()).collect(),
    };

    let raw_key = format!("raw/{}/{}", meta.owner, meta.filename);
    let thumbnail_key = format!("thumbnail/{}/{}", meta.owner, meta.thumbnail);
    let meta_key = format!("meta/{}/{}.yml", meta.owner, meta.uuid);
    let meta_raw = serde_yaml::to_string(&meta).unwrap();
    {
        let u_raw = agent::upload(&raw_key, data.file);
        let u_thumbnail = agent::upload(&thumbnail_key, &thumbnail.data);
        let u_meta = agent::upload(&meta_key, meta_raw.as_bytes());

        let result = future::join_all(vec![u_raw, u_thumbnail, u_meta])
            .await
            .iter()
            .all(|r| r.is_ok());
        if result {
            let brief = BriefMetaData::from(&meta);
            let meta = Arc::new(meta);
            #[cfg(feature = "avif")]
            tokio::spawn(agent::format_into_avif(meta.clone()));
            let _ = agent::insert(meta.clone()).await;
            (Status::Ok, serde_json::to_string(&brief).unwrap())
        } else {
            (Status::InternalServerError, "upload failed".to_string())
        }
    }
}

#[get("/delete?<id>&<code>")]
pub async fn delete_item(
    id: Option<&str>,
    code: Option<&str>,
    token: TokenPayload,
) -> (Status, String) {
    async fn request(id: &str, token: TokenPayload) -> (Status, String) {
        let uuid = id.trim().to_ascii_lowercase();
        if let Ok(meta) = agent::get_meta_by_uuid(&token.name, &uuid).await {
            let code = Uuid::new_v4().to_string();
            let mut map = TO_DELETE_BUFFER.get().unwrap().write().await;
            map.insert(
                code.clone(),
                ToDeleteRecord {
                    ts: SystemTime::now(),
                    meta: meta,
                },
            );
            (Status::Ok, code)
        } else {
            (Status::NotFound, "not found".to_string())
        }
    }
    async fn confirm(code: &str, token: TokenPayload) -> (Status, String) {
        let code = code.trim().to_ascii_lowercase();
        let record = TO_DELETE_BUFFER.get().unwrap().write().await.remove(&code);
        let earlist_ts = SystemTime::now() - Duration::from_secs(60);
        if let Some(record) = record {
            if record.meta.owner == token.name {
                if record.ts >= earlist_ts {
                    if agent::remove_meta(record.meta.clone()).await.is_ok() {
                        return (Status::Ok, format!("{}, goodbye!", record.meta.uuid));
                    }
                }
            }
        }
        (Status::BadRequest, "invalid code".to_owned())
    }
    if let Some(id) = id {
        request(id, token).await
    } else if let Some(code) = code {
        confirm(code, token).await
    } else {
        (Status::BadRequest, "WTF".to_owned())
    }
}

#[get("/dither?<filter>&<preview>")]
async fn dither_random(data: TokenPayload, filter: Option<&str>, preview: bool) -> FileContent {
    let w = EALBUM_WIDTH.get().unwrap();
    let h = EALBUM_HEIGHT.get().unwrap();
    let list = agent::list(&data.name, filter).await;
    let mut list = list.unwrap_or(Vec::new());
    while list.len() > 0 {
        let idx = rand::rng().random_range(0..list.len());
        let item = list.remove(idx);
        let data =
            agent::get_content(&format!("{}/{}/{}", "raw", item.owner, item.filename)).await;
        if let Ok(data) = data {
            let result = agent::dither(&data, &item.content_type, *w, *h, preview);
            if let Ok(result) = result {
                return FileContent {
                    content_type: result.mime_type.to_string(),
                    name: "dither".to_string(),
                    body: Ok(result.data),
                };
            } else {
                debug!(
                    "failed to dither {}, mime={}, error={:?}",
                    item.uuid,
                    item.content_type,
                    result.err().unwrap()
                );
            }
        } else {
            debug!("failed to get {}, mime={}", item.uuid, item.content_type);
        }
    }
    return FileContent {
        content_type: "text/plain".to_string(),
        name: "".to_owned(),
        body: Err(anyhow!("WTF")),
    };
}

#[get("/dither/<uuid>?<preview>")]
async fn dither_uuid(data: TokenPayload, uuid: &str, preview: bool) -> FileContent {
    let w = EALBUM_WIDTH.get().unwrap();
    let h = EALBUM_HEIGHT.get().unwrap();
    let item = agent::get_meta_by_uuid(&data.name, uuid).await;
    if item.is_err() {
        return FileContent {
            content_type: "text/plain".to_string(),
            name: "".to_owned(),
            body: Err(anyhow!("{} not found", uuid)),
        };
    }
    let item = item.unwrap();
    let data = agent::get_content(&format!("{}/{}/{}", "raw", item.owner, item.filename)).await;
    if let Ok(data) = data {
        let result = agent::dither(&data, &item.content_type, *w, *h, preview);
        if let Ok(result) = result {
            return FileContent {
                content_type: result.mime_type.to_string(),
                name: "dither".to_string(),
                body: Ok(result.data),
            };
        } else {
            debug!(
                "failed to dither {}, mime={}, error={:?}",
                item.uuid,
                item.content_type,
                result.err().unwrap()
            );
        }
    } else {
        debug!("failed to get {}, mime={}", item.uuid, item.content_type);
    }
    FileContent {
        content_type: "text/plain".to_string(),
        name: "".to_owned(),
        body: Err(anyhow!("{} dithering failed", uuid)),
    }
}

pub async fn build(
    build: Rocket<Build>,
    base: &str,
    config: &figment::Figment,
) -> anyhow::Result<Rocket<Build>> {
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
    let w = config
        .find_value("meme.eink_album_width")
        .with_context(|| anyhow!("meme.eink_album_width not found"))?
        .to_num()
        .ok_or(anyhow!("meme.eink_album_width not a number"))?;
    let h = config
        .find_value("meme.eink_album_height")
        .with_context(|| anyhow!("meme.eink_album_width not found"))?
        .to_num()
        .ok_or(anyhow!("meme.eink_album_height not a number"))?;
    
    EALBUM_WIDTH.get_or_init(|| w.to_i128().unwrap() as u32);
    EALBUM_HEIGHT.get_or_init(|| h.to_i128().unwrap() as u32);
    HOST.get_or_init(move || host.to_string());
    GATE.get_or_init(move || gate.to_string());
    TO_DELETE_BUFFER.get_or_init(|| RwLock::new(HashMap::new()));

    let base = regex::Regex::new(r"/{2,}")
        .unwrap()
        .replace_all(&format!("{}/i/", base), "/")
        .to_string();
    BASE.get_or_init(|| base.clone());
    #[cfg(debug_assertions)]
    let build = build.mount(&base, routes![preview_thumbnail]);
    Ok(build.mount(
        &base,
        routes![
            check,
            check_with_token,
            everything,
            list,
            random,
            login,
            logout,
            upload,
            get_raw,
            get_thumbnail,
            latest,
            delete_item,
            dither_random,
            dither_uuid,
        ],
    ))
}
