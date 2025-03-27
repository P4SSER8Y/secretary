mod agent;

use crate::agent::init;
use anyhow::anyhow;
use log::{self, debug, info, warn};
use rocket::{
    figment::Figment, form::Form, http::ContentType, post, response::status::NotFound, routes,
    Build, FromForm, Rocket,
};

#[derive(Debug, FromForm)]
struct JWT {
    bearer: String,
}

#[post("/check", data = "<data>")]
async fn check(data: Form<JWT>) -> Result<String, NotFound<()>> {
    if let Ok(claims) = agent::check(&data.bearer) {
        debug!("{:?}", claims);
        return Ok(format!("{:?}", claims));
    }
    warn!("Invalid token: {}", data.bearer);
    Err(NotFound(()))
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
    Ok(build.mount(base, routes![check]))
}
