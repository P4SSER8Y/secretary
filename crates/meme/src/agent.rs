use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use jsonwebtoken::{decode, DecodingKey, Validation};
use log::{debug, info};
use rocket::{serde::Deserialize, tokio::sync::RwLock};
use s3::Bucket;
use serde::Serialize;
use std::{collections::HashMap, result, sync::OnceLock};

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct MetaData {
    #[serde(rename = "content-type")]
    pub content_type: String,
    pub timestamp: String,
    pub filename: String,
    pub thumbnail: String,
    pub uuid: String,
    pub owner: String,
    pub size: usize,
    pub tags: Vec<String>,
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
    let key = JWT_SECRET_KEY
        .get()
        .with_context(|| anyhow!("JWT_SECRET_KEY not set"))?;
    let validation = JWT_VALIDATION
        .get()
        .with_context(|| anyhow!("JWT_VALIDATION not set"))?;
    let claims = decode::<TokenPayload>(token, key, validation)?.claims;
    Ok(claims)
}

pub async fn list(name: &str) -> Result<Vec<MetaData>> {
    static BUFFERS: OnceLock<RwLock<HashMap<String, Vec<MetaData>>>> = OnceLock::new();
    let buffer = BUFFERS.get_or_init(|| RwLock::new(HashMap::new())).read().await;
    if !buffer.contains_key(name) {
        drop(buffer);
        info!("update buffer for {}", name);
        let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
        let list = bucket.list(format!("meta/{}", name), None).await?;
        let mut result = Vec::new();
        for list in list {
            for object in list.contents {
                let data = bucket.get_object(object.key).await?;
                let meta: MetaData = serde_yaml::from_slice(data.as_slice())?;
                result.push(meta);
            }
        }
        let mut buffer = BUFFERS.get_or_init(|| RwLock::new(HashMap::new())).write().await;
        buffer.insert(name.to_owned(), result);
    }
    let buffer = BUFFERS.get_or_init(|| RwLock::new(HashMap::new())).read().await;
    match buffer.get(name) {
        Some(result) => Ok(result.clone()),
        None => Err(anyhow!("no such name")),
    }
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
    BUCKET.get_or_init(|| bucket);

    return Ok(());
}
