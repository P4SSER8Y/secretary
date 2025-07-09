use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use crypto::{digest::Digest, sha2::Sha256};
use jsonwebtoken::{decode, DecodingKey, Validation};
#[allow(unused_imports)]
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, path::PathBuf, sync::OnceLock
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenPayload {
    #[serde(alias = "n")]
    pub name: String,
    #[serde(alias = "f")]
    pub family: String,
    #[serde(default)]
    pub raw: String,
}

static KEY_DB: OnceLock<HashMap<String, TokenPayload>> = OnceLock::new();
static KEY_SALT: OnceLock<String> = OnceLock::new();
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
    let mut claims = decode::<TokenPayload>(token, key, validation)?.claims;
    claims.raw = token.to_string();
    Ok(claims)
}

pub fn check_key(key: &str) -> Result<&TokenPayload> {
    let mut hasher = Sha256::new();
    hasher.input_str(key);
    hasher.input_str(KEY_SALT.get().unwrap());
    let hash = hasher.result_str();
    let db = KEY_DB.get().unwrap();
    match db.get(&hash) {
        Some(value) => Ok(value),
        None => Err(anyhow!("not matched")),
    }
}

pub async fn init(
    jwt_secret_key: &str,
    key_salt: &str,
    key_file: &PathBuf,
) -> anyhow::Result<()> {
    let key = BASE64.decode(jwt_secret_key)?;
    let key = DecodingKey::from_ec_pem(&key)?;
    JWT_SECRET_KEY.get_or_init(|| key);
    JWT_VALIDATION.get_or_init(|| Validation::new(jsonwebtoken::Algorithm::ES256));

    let key_db = std::fs::read_to_string(key_file);
    if key_db.is_err() {
        log::error!("read {:?} failed", key_file);
    } else {
        let key_db = key_db.unwrap();
        let key_db = serde_yaml::from_str::<HashMap<String, TokenPayload>>(&key_db);
        if key_db.is_err() {
            log::error!("parse {:?} failed: {:?}", key_file, key_db.err());
        } else {
            let key_db = key_db.unwrap();
            KEY_DB.get_or_init(|| key_db);
        }
    }
    KEY_DB.get_or_init(|| HashMap::new());
    KEY_SALT.get_or_init(|| key_salt.to_string());

    return Ok(());
}
