use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::{guess_format, ImageReader};
use jsonwebtoken::{decode, DecodingKey, Validation};
#[allow(unused_imports)]
use log::{debug, info};
use rocket::{
    serde::Deserialize,
    tokio::{
        self,
        sync::{Mutex, RwLock},
    },
};
use s3::Bucket;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};
use std::{io::Cursor, sync::Arc};

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct MetaData {
    #[serde(rename = "content-type")]
    pub content_type: String,
    pub timestamp: String,
    pub filename: String,
    #[serde(rename = "thumbnail-content-type")]
    pub thumbnail_content_type: Option<String>,
    pub thumbnail: String,
    pub uuid: String,
    pub owner: String,
    pub size: usize,
    pub tags: Vec<String>,
}

impl PartialEq for MetaData {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl std::hash::Hash for MetaData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state)
    }
}

impl Eq for MetaData {}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPayload {
    #[serde(alias = "n")]
    pub name: String,
    #[serde(alias = "f")]
    pub family: String,
    #[serde(default)]
    pub raw: String,
}

static BUCKET: OnceLock<Box<s3::Bucket>> = OnceLock::new();
static HIDE: OnceLock<Vec<String>> = OnceLock::new();

type MetaListT = Vec<Arc<MetaData>>;
type TagToMetalistT = HashMap<String, MetaListT>; // HashMap<tag, list>
type OwnerListT = HashMap<String, TagToMetalistT>; // HashMap<owner, full_list>
static BUFFERS: OnceLock<RwLock<OwnerListT>> = OnceLock::new();
static JWT_SECRET_KEY: OnceLock<DecodingKey> = OnceLock::new();
static JWT_VALIDATION: OnceLock<Validation> = OnceLock::new();

pub fn split_tags(tags: Option<&str>) -> Vec<&str> {
    tags.unwrap_or("")
        .split(&[',', '，', ';', '；'][..])
        .filter(|s| s.len() > 0)
        .map(|s| s.trim())
        .collect()
}

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

pub async fn get_content(path: &str) -> Result<Vec<u8>> {
    let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
    let data = bucket.get_object(path).await?;
    Ok(data.to_vec())
}

pub async fn update(name: &str) -> Result<()> {
    async fn main(name: &str) -> Result<()> {
        info!("update buffer for {}", name);
        let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
        let list = bucket.list(format!("meta/{}", name), None).await?;
        let mut result = HashMap::new();
        for list in list {
            for object in list.contents {
                let data = bucket.get_object(object.key.clone()).await?;
                let data = serde_yaml::from_slice::<MetaData>(data.as_slice());
                if data.is_err() {
                    log::error!("cannot parse {}", object.key);
                    continue;
                }
                let meta = Arc::new(data.unwrap());
                for tag in meta
                    .tags
                    .iter()
                    .map(|item| item.trim().to_ascii_lowercase())
                {
                    result
                        .entry(tag)
                        .or_insert_with(Vec::new)
                        .push(meta.clone());
                }
            }
        }
        if result.len() > 0 {
            let mut buffer = BUFFERS
                .get_or_init(|| RwLock::new(HashMap::new()))
                .write()
                .await;
            buffer.insert(name.to_owned(), result);
            info!("finish update buffer for {}", name);
            Ok(())
        } else {
            Err(anyhow!("no such name: {}", name))
        }
    }
    static LOCK: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let mut updating = false;
    {
        let set = LOCK.get_or_init(|| Mutex::new(HashSet::new())).lock().await;
        if set.contains(name) {
            updating = true;
        }
    }
    if updating {
        info!("updating buffer for {}", name);
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            {
                let set = LOCK.get_or_init(|| Mutex::new(HashSet::new())).lock().await;
                if !set.contains(name) {
                    info!("updated buffer for {}", name);
                    return Ok(());
                }
            }
        }
    }
    {
        let mut set = LOCK.get_or_init(|| Mutex::new(HashSet::new())).lock().await;
        set.insert(name.to_string());
    }
    let t = main(name).await;
    if t.is_err() {
        log::error!("update {} failed: {:?}", name, t);
    }
    {
        let mut set = LOCK.get_or_init(|| Mutex::new(HashSet::new())).lock().await;
        set.remove(name);
    }
    t
}

pub async fn list(name: &str, filter: Option<&str>) -> Result<Vec<MetaData>> {
    let buffer = BUFFERS
        .get()
        .with_context(|| anyhow!("BUFFERS not set"))?
        .read()
        .await;
    if !buffer.contains_key(name) {
        drop(buffer);
        update(name).await?;
    }
    let buffer = BUFFERS
        .get_or_init(|| RwLock::new(HashMap::new()))
        .read()
        .await;
    let buffer = buffer.get(name);
    if buffer.is_none() {
        return Err(anyhow!("no such name"));
    }
    let buffer = buffer.unwrap();
    let mut result: HashSet<Arc<MetaData>> = HashSet::new();
    let filter: Vec<_> = split_tags(filter)
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect();
    if filter.len() > 0 {
        for key in buffer.keys() {
            if !key.contains(&filter[0]) {
                continue;
            }
            for item in buffer.get(key).unwrap() {
                if filter
                    .iter()
                    .all(|f| item.tags.iter().any(|tag| tag.contains(f)))
                {
                    result.insert(item.clone());
                }
            }
        }
    } else {
        for item in buffer {
            item.1.iter().for_each(|item| {
                result.insert(item.clone());
            });
        }
        for filter in HIDE.get().unwrap_or(&Vec::new()) {
            for key in buffer.keys() {
                if key.contains(filter) {
                    buffer.get(key).unwrap().iter().for_each(|item| {
                        result.remove(item);
                    });
                }
            }
        }
    }
    Ok(result.drain().map(|x| MetaData::clone(&x)).collect())
}

pub async fn upload(key: &str, data: &[u8]) -> anyhow::Result<()> {
    info!("upload {} bytes to {}", data.len(), key);
    let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
    bucket.put_object(key, data).await?;
    info!("finish upload {}", key);
    Ok(())
}

pub async fn generate_thumbnail(data: &[u8]) -> anyhow::Result<(Vec<u8>, &str, &str)> {
    let format = image::ImageFormat::WebP;
    let mut img = ImageReader::new(Cursor::new(data))
        .with_guessed_format()?
        .decode()?;
    let nheight = (img.height() as f32 * 512.0 / img.width() as f32) as u32;
    img = img.thumbnail(512, nheight);
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::WebP)?;
    Ok((buf, format.extensions_str()[0], format.to_mime_type()))
}

#[allow(dead_code)]
pub async fn compress(data: &[u8]) -> anyhow::Result<(Vec<u8>, &str)> {
    let img = ImageReader::new(Cursor::new(data))
        .with_guessed_format()?
        .decode()?;
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::WebP)?;
    Ok((buf, image::ImageFormat::WebP.to_mime_type()))
}

pub async fn guess_image_mime_type(data: &[u8]) -> anyhow::Result<(&str, &str)> {
    let format = guess_format(data);
    match format {
        Ok(format) => Ok((format.to_mime_type(), format.extensions_str()[0])),
        Err(_) => Ok(("application/octet-stream", "")),
    }
}

pub async fn init(
    endpoint: &str,
    region: &str,
    bucket: &str,
    access_key: &str,
    secret_key: &str,
    jwt_secret_key: &str,
    hide: &Vec<&str>,
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
    BUFFERS.get_or_init(|| RwLock::new(HashMap::new()));

    HIDE.get_or_init(|| hide.iter().map(|s| s.to_ascii_lowercase()).collect());

    return Ok(());
}
