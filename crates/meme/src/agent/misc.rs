use anyhow::{anyhow, Context, Result};
#[allow(unused_imports)]
use log::{debug, info};
use rocket::{
    futures::future::join_all,
    tokio::{
        self,
        sync::{Mutex, RwLock},
    },
};
use s3::Bucket;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

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
    #[serde(skip_serializing, skip_deserializing)]
    pub lower_tags: Vec<String>,
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

static BUCKET: OnceLock<Box<s3::Bucket>> = OnceLock::new();
static HIDE: OnceLock<Vec<String>> = OnceLock::new();

type UuidToMetaListT = HashMap<String, Arc<MetaData>>;
type OwnerToUuidListT = HashMap<String, UuidToMetaListT>;
static META_BUFFERS: OnceLock<RwLock<OwnerToUuidListT>> = OnceLock::new();

pub fn split_tags(tags: Option<&str>) -> Vec<&str> {
    tags.unwrap_or("")
        .split(&[',', '，', ';', '；'][..])
        .filter(|s| s.len() > 0)
        .map(|s| s.trim())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

pub async fn insert(meta: Arc<MetaData>) -> Result<usize> {
    let buffer = META_BUFFERS
        .get()
        .with_context(|| anyhow!("META_BUFFERS not set"))?;
    let mut buffer = buffer.write().await;
    if let Some(list) = buffer.get_mut(&meta.owner) {
        list.insert(meta.uuid.to_ascii_lowercase(), meta.clone());
    } else {
        let mut list = HashMap::new();
        list.insert(meta.uuid.clone(), meta.clone());
        buffer.insert(meta.owner.to_string(), list);
    }
    Ok(buffer.get(&meta.owner).unwrap().len())
}

pub async fn get_content(path: &str) -> Result<Vec<u8>> {
    let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
    let data = bucket.get_object(path).await?;
    Ok(data.to_vec())
}

async fn full_update(name: &str) -> Result<()> {
    async fn update_one(bucket: &Box<Bucket>, key: String) -> Result<usize> {
        let data = bucket.get_object(key.clone()).await?;
        let data = serde_yaml::from_slice::<MetaData>(data.as_slice());
        if data.is_err() {
            log::error!("cannot parse {}", key);
            return Err(anyhow!("cannot parse {}", key));
        }
        let mut meta = data.unwrap();
        meta.lower_tags = meta
            .tags
            .iter()
            .map(|t| t.trim().to_ascii_lowercase())
            .collect();
        let result = insert(Arc::new(meta)).await?;
        Ok(result)
    }
    info!("update buffer for {}", name);
    let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
    let list = bucket.list(format!("meta/{}", name), None).await?;
    let mut results = Vec::new();
    for list in list {
        for object in list.contents {
            results.push(update_one(bucket, object.key));
        }
    }
    let todo_result = join_all(results).await;
    if todo_result
        .into_iter()
        .any(|item| item.is_ok() && item.unwrap() > 0)
    {
        info!("finish update buffer for {}", name);
        Ok(())
    } else {
        Err(anyhow!("no such name: {}", name))
    }
}

pub async fn force_update(name: &str) -> Result<()> {
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
    let t = full_update(name).await;
    if t.is_err() {
        log::error!("update {} failed: {:?}", name, t);
    }
    {
        let mut set = LOCK.get_or_init(|| Mutex::new(HashSet::new())).lock().await;
        set.remove(name);
    }
    t
}

async fn update(name: &str) -> Result<()> {
    let buffer = META_BUFFERS
        .get()
        .with_context(|| anyhow!("BUFFER not set"))?
        .read()
        .await;
    match !buffer.contains_key(name) {
        true => {
            drop(buffer);
            force_update(name).await
        }
        false => Ok(()),
    }
}

pub async fn get_meta_by_uuid(name: &str, uuid: &str) -> Result<Arc<MetaData>> {
    let buffer = META_BUFFERS.get().unwrap().read().await;
    let buffer = buffer.get(name);
    if buffer.is_none() {
        return Err(anyhow!("no such name"));
    }
    let buffer = buffer.unwrap();
    let buffer = buffer.get(uuid);
    if buffer.is_none() {
        return Err(anyhow!("no such uuid"));
    }
    let buffer = buffer.unwrap();
    Ok(buffer.clone())
}

pub async fn list(name: &str, filter: Option<&str>) -> Result<Vec<Arc<MetaData>>> {
    let buffer = META_BUFFERS
        .get()
        .with_context(|| anyhow!("BUFFERS not set"))?;
    let _ = update(name).await;
    let buffer = buffer.read().await;
    let buffer = buffer.get(name);
    if buffer.is_none() {
        return Err(anyhow!("{} not found", name));
    }
    let buffer = buffer.unwrap();
    let mut result: Vec<Arc<MetaData>> = buffer.values().map(|v| v.clone()).collect();
    let filters = split_tags(filter)
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect::<Vec<_>>();
    debug!("{}", result.len());
    for hide in HIDE.get().unwrap_or(&Vec::new()) {
        if !filters.contains(hide) {
            debug!("remove {}", hide);
            result.retain(|v| v.lower_tags.iter().all(|s| !s.contains(hide)));
            debug!("{}", result.len());
        }
    }
    for filter in filters {
        let key = filter.trim_start_matches(&['+', '-', ' ']);
        match filter.chars().nth(0) {
            Some('-') => {
                debug!("remove {}", key);
                result.retain(|v| v.lower_tags.iter().all(|s| !s.contains(key)));
            }
            _ => {
                debug!("keep {}", key);
                result.retain(|v| v.lower_tags.iter().any(|s| s.contains(key)));
            }
        }
        debug!("{}", result.len());
    }
    Ok(result)
}

pub async fn upload(key: &str, data: &[u8]) -> anyhow::Result<()> {
    info!("upload {} bytes to {}", data.len(), key);
    let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
    bucket.put_object(key, data).await?;
    info!("finish upload {}", key);
    Ok(())
}

pub async fn init(
    endpoint: &str,
    region: &str,
    bucket: &str,
    access_key: &str,
    secret_key: &str,
    jwt_secret_key: &str,
    hide: &Vec<&str>,
    key_salt: &str,
    key_file: &str,
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

    let bucket = Bucket::new(bucket, region, credentials)?.with_path_style();
    BUCKET.get_or_init(|| bucket);
    META_BUFFERS.get_or_init(|| RwLock::new(HashMap::new()));

    HIDE.get_or_init(|| hide.iter().map(|s| s.trim().to_ascii_lowercase()).collect());

    super::validation::init(jwt_secret_key, key_salt, key_file).await?;

    return Ok(());
}
