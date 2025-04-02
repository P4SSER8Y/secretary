mod agent;

use std::{io::Cursor, sync::OnceLock};

use crate::agent::init;
use agent::{MetaData, TokenPayload};
use anyhow::anyhow;
use figment::Figment;
use log::info;
use rand::{self, Rng};
use rocket::{
    get,
    http::{CookieJar, Status},
    request::{FromRequest, Outcome},
    response::{self, status::NotFound, Responder},
    routes,
    serde::json::Json,
    Build, Request, Response, Rocket,
};

static ENDPOINT: OnceLock<String> = OnceLock::new();
static BUCKET: OnceLock<String> = OnceLock::new();

#[rocket::async_trait]
impl<'a> FromRequest<'a> for TokenPayload {
    type Error = anyhow::Error;

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        fn parse_from_query<'a>(request: &'a Request<'_>) -> anyhow::Result<String> {
            let bearer = request
                .query_value::<String>("token")
                .ok_or(anyhow!("No token"))?
                .unwrap();
            Ok(bearer)
        }

        fn parse_from_cookies<'a>(request: &'a Request<'_>) -> anyhow::Result<String> {
            let bearer = request
                .cookies()
                .get("token")
                .ok_or(anyhow!("No token"))?
                .value()
                .to_string();
            Ok(bearer)
        }

        fn parse_from_header<'a>(request: &'a Request<'_>) -> anyhow::Result<String> {
            let bearer = request
                .headers()
                .get_one("Authorization")
                .ok_or(anyhow!("No bearer token"))?;
            let bearer = bearer.replace("Bearer ", "");
            Ok(bearer)
        }

        // check from header, cookies, query
        let bearer = parse_from_header(request)
            .or_else(|_| parse_from_cookies(request))
            .or_else(|_| parse_from_query(request));

        if let Err(_) = bearer {
            return Outcome::Forward(Status::Unauthorized);
        }
        match agent::check(bearer.unwrap().trim()) {
            Ok(claims) => Outcome::Success(claims),
            Err(_e) => Outcome::Forward(Status::Unauthorized),
        }
    }
}

#[get("/check")]
async fn check(data: TokenPayload, cookies: &CookieJar<'_>) -> Result<String, NotFound<()>> {
    cookies.add(("token", data.raw.clone()));
    Ok(format!("Hello {} of {}", data.name, data.family))
}

#[get("/<_..>", rank = 99)]
async fn everything() -> (Status, &'static str) {
    (Status::Unauthorized, "WTF")
}

#[get("/list")]
async fn list(data: TokenPayload) -> Json<Vec<MetaData>> {
    let list = agent::list(&data.name).await;
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

#[get("/random?<t>")]
async fn random(data: TokenPayload, t: bool) -> FileContent {
    let list = agent::list(&data.name).await;
    let list = list.unwrap_or(Vec::new());
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

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
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

    init(
        &endpoint,
        &region_name,
        &bucket_name,
        &access_key,
        &secret_key,
        &jwt_secret_key,
    )
    .await?;
    Ok(build.mount(base, routes![check, everything, list, random]))
}
