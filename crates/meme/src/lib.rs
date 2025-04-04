mod agent;

use std::{io::Cursor, sync::OnceLock};

use crate::agent::init;
use agent::{MetaData, TokenPayload};
use anyhow::anyhow;
use figment::Figment;
use log::info;
use rand::{self, Rng};
use rocket::{
    form::Form,
    futures::future,
    get,
    http::{ContentType, CookieJar, Status},
    post,
    request::{FromRequest, Outcome},
    response::{self, status::NotFound, Redirect, Responder},
    routes,
    serde::json::Json,
    tokio::{self},
    Build, FromForm, Request, Response, Rocket,
};
use uuid::Uuid;

static ENDPOINT: OnceLock<String> = OnceLock::new();
static BUCKET: OnceLock<String> = OnceLock::new();
static HOST: OnceLock<String> = OnceLock::new();
static GATE: OnceLock<String> = OnceLock::new();
static BASE: OnceLock<String> = OnceLock::new();

#[rocket::async_trait]
impl<'a> FromRequest<'a> for TokenPayload {
    type Error = anyhow::Error;

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        fn parse_from_query<'a>(request: &'a Request<'_>) -> anyhow::Result<&'a str> {
            let bearer = request
                .query_value::<&str>("token")
                .ok_or(anyhow!("No token"))?
                .unwrap();
            Ok(bearer)
        }

        fn parse_from_cookies<'a>(request: &'a Request<'_>) -> anyhow::Result<&'a str> {
            let bearer = request
                .cookies()
                .get("token")
                .ok_or(anyhow!("No token"))?
                .value();
            Ok(bearer)
        }

        fn parse_from_header<'a>(request: &'a Request<'_>) -> anyhow::Result<&'a str> {
            let bearer = request
                .headers()
                .get_one("token")
                .ok_or(anyhow!("No bearer token"))?;
            Ok(bearer)
        }

        fn parse_key<'a>(request: &'a Request<'_>) -> anyhow::Result<&'a str> {
            let key = request
                .query_value::<&str>("key")
                .ok_or(anyhow!("No key"))?
                .unwrap();
            Ok(key)
        }

        // check from header, cookies, query
        let bearer = parse_from_header(request)
            .or_else(|_| parse_from_cookies(request))
            .or_else(|_| parse_from_query(request));
        if let Ok(bearer) = bearer {
            if let Ok(claims) = agent::check(bearer.trim()) {
                return Outcome::Success(claims);
            }
        }
        if let Ok(key) = parse_key(request) {
            if let Ok(claims) = agent::check_key(key) {
                return Outcome::Success(claims.clone());
            }
        }
        Outcome::Forward(Status::Unauthorized)
    }
}

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
        let _ = agent::update(&name).await;
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

#[get("/list?<filter>")]
async fn list(data: TokenPayload, filter: Option<&str>) -> Json<Vec<MetaData>> {
    let list = agent::list(&data.name, filter).await;
    Json(list.unwrap_or(Vec::new()))
}

struct FileContent {
    content_type: String,
    body: anyhow::Result<Vec<u8>>,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for FileContent {
    fn respond_to(self, _request: &'r Request<'_>) -> response::Result<'o> {
        if let Ok(body) = self.body {
            Ok(Response::build()
                .status(Status::Ok)
                .raw_header("Content-Type", self.content_type.to_string())
                .raw_header("Content-Length", body.len().to_string())
                .sized_body(body.len(), Cursor::new(body))
                .finalize())
        } else {
            let body = "Not Found".to_string();
            Ok(Response::build()
                .status(Status::NotFound)
                .sized_body(body.len(), Cursor::new(body))
                .finalize())
        }
    }
}

#[get("/random?<t>&<filter>")]
async fn random(data: TokenPayload, t: bool, filter: Option<&str>) -> FileContent {
    let list = agent::list(&data.name, filter).await;
    let list = list.unwrap_or(Vec::new());
    if list.len() == 0 {
        return FileContent {
            content_type: "text/plain".to_string(),
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
            content_type: "image/jpeg".to_string(),
            body: data,
        }
    } else {
        let data = agent::get_content(&format!("{}/{}/{}", "raw", item.owner, item.filename)).await;
        FileContent {
            content_type: item.content_type.clone(),
            body: data,
        }
    }
}

#[derive(FromForm)]
struct UploadedImage<'r> {
    file: &'r [u8],
    tags: Option<&'r str>,
}

#[cfg(debug_assertions)]
#[post("/thumbnail", data = "<data>", format = "multipart/form-data")]
async fn thumbnail(data: Form<UploadedImage<'_>>) -> (ContentType, Vec<u8>) {
    let body = data.file;
    if body.len() == 0 {
        return (ContentType::Text, "failed to read data".as_bytes().to_vec());
    }
    let data = agent::generate_thumbnail(body).await;
    match data {
        Ok(data) => (
            ContentType::parse_flexible(data.2).unwrap_or(ContentType::Any),
            data.0,
        ),
        Err(_) => (
            ContentType::Text,
            "failed to generate thumbnail".as_bytes().to_vec(),
        ),
    }
}

#[post("/upload", data = "<data>", format = "multipart/form-data")]
async fn upload(data: Form<UploadedImage<'_>>, token: TokenPayload) -> (Status, String) {
    let body = data.file;
    if body.len() == 0 {
        return (Status::NoContent, "empty file".to_string());
    }
    let uuid = Uuid::new_v4().to_string();
    let thumbnail = agent::generate_thumbnail(body).await;
    if thumbnail.is_err() {
        return (Status::InternalServerError, "failed to convert".to_string());
    }
    let thumbnail = thumbnail.unwrap();
    let raw_format = agent::guess_image_mime_type(data.file)
        .await
        .unwrap_or(("application/octet-stream", ""));
    let meta = MetaData {
        timestamp: chrono::Utc::now().to_rfc3339(),
        content_type: raw_format.0.to_string(),
        filename: format!("{}.{}", uuid, raw_format.1),
        thumbnail_content_type: Some(thumbnail.2.to_string()),
        thumbnail: format!("{}.{}", uuid, thumbnail.1),
        uuid: uuid.clone(),
        owner: token.name.to_string(),
        size: data.file.len(),
        tags: agent::split_tags(data.tags)
            .iter()
            .map(|s| s.to_string())
            .collect(),
    };

    let raw_key = format!("raw/{}/{}", meta.owner, meta.filename);
    let thumbnail_key = format!("thumbnail/{}/{}", meta.owner, meta.thumbnail);
    let meta_key = format!("meta/{}/{}.yml", meta.owner, meta.uuid);
    let meta_raw = serde_yaml::to_string(&meta).unwrap();
    {
        let u_raw = agent::upload(&raw_key, data.file);
        let u_thumbnail = agent::upload(&thumbnail_key, &thumbnail.0);
        let u_meta = agent::upload(&meta_key, meta_raw.as_bytes());

        let result = future::join_all(vec![u_raw, u_thumbnail, u_meta])
            .await
            .iter()
            .all(|r| r.is_ok());
        if result {
            (Status::Ok, serde_json::to_string(&meta).unwrap())
        } else {
            (Status::InternalServerError, "upload failed".to_string())
        }
    }
}

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    let host = config
        .find_value("host")?
        .as_str()
        .ok_or(anyhow!("host not found"))?
        .to_owned();
    HOST.get_or_init(move || host);
    let gate = config
        .find_value("meme.jwt_gate")?
        .as_str()
        .ok_or(anyhow!("meme.jwt_gate not found"))?
        .to_owned();
    GATE.get_or_init(move || gate);
    BASE.get_or_init(|| base.to_string());
    let endpoint = config
        .find_value("meme.endpoint")?
        .as_str()
        .ok_or(anyhow!("endpoint not found"))?
        .to_owned();
    ENDPOINT.get_or_init(|| endpoint.clone());
    let region_name = config
        .find_value("meme.region")?
        .as_str()
        .ok_or(anyhow!("region not found"))?
        .to_owned();
    let bucket_name = config
        .find_value("meme.bucket")?
        .as_str()
        .ok_or(anyhow!("bucket not found"))?
        .to_owned();
    BUCKET.get_or_init(|| bucket_name.clone());
    info!("{} {} {}", endpoint, region_name, bucket_name);

    let access_key = config
        .find_value("meme.access_key")?
        .as_str()
        .ok_or(anyhow!("access_key not found"))?
        .to_owned();
    let secret_key = config
        .find_value("meme.secret_key")?
        .as_str()
        .ok_or(anyhow!("secret_key not found"))?
        .to_owned();
    let jwt_secret_key = config
        .find_value("meme.jwt_secret_key")?
        .as_str()
        .ok_or(anyhow!("jwt_secret_key not found"))?
        .to_owned();
    let hide = config.find_value("meme.hide")?;
    let hide = hide
        .as_array()
        .ok_or(anyhow!("hide not found"))?
        .iter()
        .map(|item| item.as_str())
        .filter(|item| item.is_some() && item.unwrap().len() > 0)
        .map(|item| item.unwrap())
        .collect();
    let key_file = config
        .find_value("meme.key_file")?
        .as_str()
        .ok_or(anyhow!("key_file not found"))?
        .to_owned();
    let key_salt = config
        .find_value("meme.key_salt")?
        .as_str()
        .ok_or(anyhow!("key_salt not found"))?
        .to_owned();

    init(
        &endpoint,
        &region_name,
        &bucket_name,
        &access_key,
        &secret_key,
        &jwt_secret_key,
        &hide,
        &key_salt,
        &key_file,
    )
    .await?;

    let build = build.mount(
        base,
        routes![
            check,
            check_with_token,
            everything,
            list,
            random,
            login,
            logout,
            upload,
        ],
    );
    #[cfg(debug_assertions)]
    let build = build.mount(base, routes![thumbnail]);
    Ok(build)
}
