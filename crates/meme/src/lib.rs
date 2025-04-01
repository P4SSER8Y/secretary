mod agent;

use crate::agent::init;
use agent::{MetaData, TokenPayload};
use anyhow::anyhow;
use figment::Figment;
use log::{debug, info};
use rocket::{
    get, http::Status, request::{FromRequest, Outcome}, response::status::NotFound, routes, serde::json::Json, Build, Request, Rocket
};

#[rocket::async_trait]
impl<'a> FromRequest<'a> for TokenPayload {
    type Error = anyhow::Error;
    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        let bearer = request
            .headers()
            .get_one("Authorization")
            .ok_or(anyhow!("No bearer token"));
        if let Err(_) = bearer {
            return Outcome::Forward(Status::Unauthorized);
        }
        let bearer = bearer.unwrap();
        let bearer = bearer.replace("Bearer ", "");
        match agent::check(bearer.trim()) {
            Ok(claims) => Outcome::Success(claims),
            Err(_e) => Outcome::Forward(Status::Unauthorized),
        }
    }
}

#[get("/check")]
async fn check(data: TokenPayload) -> Result<String, NotFound<()>> {
    Ok(format!("Hello {} of {}", data.name, data.family))
}

#[get("/check", rank=1)]
async fn check_failed() -> (Status, &'static str) {
    (Status::Unauthorized, "WTF")
}

#[get("/list")]
async fn list(data: TokenPayload) -> Json<Vec<MetaData>> {
    let list = agent::list(&data.name).await;
    Json(list.unwrap_or(Vec::new()))
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
    Ok(build.mount(base, routes![check, check_failed, list]))
}
