use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use jsonwebtoken::{decode, DecodingKey, Validation};
use log::{debug, info};
use rocket::serde::Deserialize;
use s3::Bucket;
use serde::Serialize;
use std::sync::OnceLock;

#[derive(Deserialize, Debug)]
struct MetaData {
    #[serde(rename = "content-type")]
    content_type: String,
    timestamp: String,
    filename: String,
    thumbnail: String,
    uuid: String,
    owner: String,
    size: usize,
    tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPayload {
    #[serde(alias = "n")]
    pub name: String,
    #[serde(alias = "f")]
    pub family: String,
}

static BUCKET: OnceLock<Box<s3::Bucket>> = OnceLock::new();

static JWT_SECRET_KEY: OnceLock<DecodingKey> = OnceLock::new();
static JWT_VALIDATION: OnceLock<Validation> = OnceLock::new();

pub fn check(token: &str) -> Result<TokenPayload> {
    // check if bearer is valid JWT token with AES256 algorithm
    let key = JWT_SECRET_KEY.get().with_context(|| anyhow!("JWT_SECRET_KEY not set"))?;
    let validation = JWT_VALIDATION.get().with_context(|| anyhow!("JWT_VALIDATION not set"))?;
    let claims = decode::<TokenPayload>(token, key, validation)?.claims;
    Ok(claims)
}

pub async fn init(
    endpoint: &str,
    region: &str,
    bucket: &str,
    access_key: &str,
    secret_key: &str,
    jwt_secret_key: &str,
) -> anyhow::Result<()> {
    let region = s3::Region::Custom {
        region: region.to_string(),
        endpoint: endpoint.to_string(),
    };

    let credentials = s3::creds::Credentials {
        access_key: Some(access_key.to_string()),
        secret_key: Some(secret_key.to_string()),
        security_token: None,
        session_token: None,
        expiration: None,
    };

    // base64 解码出 pem key
    let key = BASE64.decode(jwt_secret_key)?;
    let key = DecodingKey::from_ec_pem(&key)?;
    JWT_SECRET_KEY.get_or_init(|| key);
    JWT_VALIDATION.get_or_init(|| Validation::new(jsonwebtoken::Algorithm::ES256));

    let bucket = Bucket::new(bucket, region, credentials)?.with_path_style();
    info!("list keys");
    let list = bucket.list("meta/ooxx".to_owned(), None).await?;

    let key = &list[0].contents[500].key;
    info!("get object");
    let data = bucket.get_object(key).await?;
    info!("{} {:#?}", key, data.bytes());
    let meta: MetaData = serde_yaml::from_slice(data.as_slice())?;
    info!("{} {:#?}", key, meta);

    BUCKET.get_or_init(|| bucket);

    return Ok(());
}
