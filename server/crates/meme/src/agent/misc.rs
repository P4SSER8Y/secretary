use anyhow::{anyhow, Context, Result};
use figment::Figment;
#[allow(unused_imports)]
use log::{debug, info, warn};
use rocket::{
    futures::future::join_all,
    tokio::{
        self,
        sync::{Mutex, RwLock, Semaphore},
    },
};
use s3::Bucket;
use s3_sync::SyncStore;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};
use std::{path::Path, sync::Arc};

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
    #[serde(default)]
    pub encrypted: bool,
    /// S3 subfolder for this meta file. Not serialized — the S3 key is the source of truth.
    /// - `None` → legacy flat path `meta/{owner}/uuid.yml`
    /// - `Some("_plain")` → `meta/{owner}/_plain/uuid.yml`
    /// - `Some("<hex>")` → `meta/{owner}/{hex}/uuid.yml`
    #[serde(skip)]
    pub path_key: Option<String>,
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

/// Compute the S3 key for a meta YAML file based on its `path_key`.
/// Falls back to legacy flat path when `path_key` is `None`.
pub fn meta_s3_path(meta: &MetaData) -> String {
    match &meta.path_key {
        Some(hash) => format!("meta/{}/{}/{}.yml", meta.owner, hash, meta.uuid),
        None => format!("meta/{}/{}.yml", meta.owner, meta.uuid),
    }
}

/// Parse the S3 subfolder from a meta key.
/// - `meta/alice/abc123/uuid.yml` → `Some("abc123")`
/// - `meta/alice/uuid.yml` → `None`
fn parse_path_key_from_s3_key(key: &str, owner: &str) -> Option<String> {
    let prefix = format!("meta/{}/", owner);
    let rest = key.strip_prefix(&prefix)?;
    let parts: Vec<&str> = rest.split('/').collect();
    match parts.len() {
        1 => None,                          // uuid.yml → legacy flat
        2 => Some(parts[0].to_string()),    // hash/uuid.yml → hash
        _ => None,
    }
}

#[derive(Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ReencryptResponse {
    pub processed: usize,
    pub errors: Vec<ReencryptError>,
}

#[derive(Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ReencryptError {
    pub uuid: String,
    pub error: String,
}

pub struct FilterTerm {
    pub text: String,
    pub exact: bool,
    pub negative: bool,
}

static BUCKET: OnceLock<Box<s3::Bucket>> = OnceLock::new();
static S3_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
static HIDE: OnceLock<Vec<String>> = OnceLock::new();

type UuidToMetaListT = HashMap<String, Arc<MetaData>>;
type OwnerToUuidListT = HashMap<String, UuidToMetaListT>;
static META_BUFFERS: OnceLock<RwLock<OwnerToUuidListT>> = OnceLock::new();
static LAST_KEY_HASH: OnceLock<RwLock<HashMap<String, [u8; 8]>>> = OnceLock::new();
static META_SYNC_STORE: OnceLock<SyncStore> = OnceLock::new();

pub fn split_tags(tags: Option<&str>) -> Vec<&str> {
    tags.unwrap_or("")
        .split(&[',', '，', ';', '；'][..])
        .filter(|s| s.len() > 0)
        .map(|s| s.trim())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn is_delimiter(c: char) -> bool {
    matches!(c, ',' | '，' | ';' | '；')
}

pub fn split_filter(filter: Option<&str>) -> Vec<FilterTerm> {
    let input = filter.unwrap_or("");
    let chars: Vec<char> = input.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        let negative = chars[i] == '-';
        if negative {
            i += 1;
        }

        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        if i < chars.len() && (chars[i] == '+' || chars[i] == '＋') {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
        }
        if i >= chars.len() {
            break;
        }

        if chars[i] == '"' {
            i += 1;
            let mut text = String::new();
            while i < chars.len() && chars[i] != '"' {
                text.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            let text = text.trim().to_ascii_lowercase();
            if !text.is_empty() {
                result.push(FilterTerm { text, exact: true, negative });
            }
        } else {
            let mut text = String::new();
            while i < chars.len() && !is_delimiter(chars[i]) {
                text.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            let text = text.trim().to_ascii_lowercase();
            if !text.is_empty() {
                result.push(FilterTerm { text, exact: false, negative });
            }
        }
    }

    result
}

#[cfg(feature = "avif")]
pub async fn format_into_avif(src: Arc<MetaData>, key: Option<&[u8; 32]>) -> Result<()> {
    async fn convert(
        path: &str,
        mime: Option<&str>,
        encrypted: bool,
        key: Option<&[u8; 32]>,
        owner: &str,
    ) -> Result<RawImage<'static>> {
        if mime.unwrap_or("").to_ascii_lowercase() == "image/avif" {
            return Err(anyhow!("already avif"));
        }
        info!("format {}", path);
        let raw = get_content(path).await?;
        let raw = if encrypted {
            let k = key.ok_or_else(|| anyhow!("encrypted but no key for avif conversion"))?;
            super::crypto::decrypt(&raw, k)?
        } else {
            raw
        };
        let raw_len = raw.len();
        let ts = std::time::SystemTime::now();
        let compressed = tokio::task::spawn_blocking(move || img::convert_to_avif(&raw)).await??;
        info!(
            "compress {} rate:{:.2}%, cost: {}s",
            path,
            compressed.data.len() as f32 / raw_len as f32 * 100.0,
            std::time::SystemTime::now()
                .duration_since(ts)
                .unwrap()
                .as_secs_f32()
        );
        Ok(compressed)
    }
    async fn update_meta(meta: MetaData, encrypted: bool, key: Option<&[u8; 32]>) -> Result<()> {
        let meta_bytes = serde_yaml::to_string(&meta)?.into_bytes();
        let to_upload: Vec<u8> = if encrypted {
            super::crypto::encrypt(&meta_bytes, key.unwrap())?
        } else {
            meta_bytes
        };
        let _ = upload(&meta_s3_path(&meta), &to_upload).await?;

        let buffer = META_BUFFERS
            .get()
            .with_context(|| anyhow!("META_BUFFERS not set"))?;
        let mut buffer = buffer.write().await;
        buffer
            .get_mut(&meta.owner)
            .unwrap()
            .insert(meta.uuid.to_ascii_lowercase(), Arc::new(meta));
        Ok(())
    }
    async fn upload_avif(
        path: &str,
        data: &[u8],
        encrypted: bool,
        key: Option<&[u8; 32]>,
    ) -> Result<()> {
        let to_upload: Vec<u8> = if encrypted {
            super::crypto::encrypt(data, key.unwrap())?
        } else {
            data.to_vec()
        };
        upload(path, &to_upload).await
    }

    let mut meta = MetaData::clone(&src);
    let thumbnail = convert(
        &format!("thumbnail/{}/{}", meta.owner, meta.thumbnail),
        meta.thumbnail_content_type.as_deref(),
        src.encrypted,
        key,
        &meta.owner,
    )
    .await;
    if let Ok(result) = thumbnail {
        meta.thumbnail = format!("{}.{}", meta.uuid, result.extension);
        meta.thumbnail_content_type = Some(result.mime_type.to_string());
        let _ = upload_avif(
            &format!("thumbnail/{}/{}", meta.owner, meta.thumbnail),
            &result.data,
            src.encrypted,
            key,
        )
        .await?;
        update_meta(meta.clone(), src.encrypted, key).await?;
        if meta.thumbnail != src.thumbnail {
            remove(&format!("thumbnail/{}/{}", src.owner, src.thumbnail)).await?;
        }
    }
    let raw = convert(
        &format!("raw/{}/{}", meta.owner, meta.filename),
        Some(&meta.content_type),
        src.encrypted,
        key,
        &meta.owner,
    )
    .await;
    if let Ok(result) = raw {
        meta.filename = format!("{}.{}", meta.uuid, result.extension);
        meta.content_type = result.mime_type.to_string();
        let _ = upload_avif(
            &format!("raw/{}/{}", meta.owner, meta.filename),
            &result.data,
            src.encrypted,
            key,
        )
        .await?;
        update_meta(meta.clone(), src.encrypted, key).await?;
        if meta.filename != src.filename {
            remove(&format!("raw/{}/{}", src.owner, src.filename)).await?;
        }
    }
    Ok(())
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

pub async fn update_tags(
    owner: &str,
    uuid: &str,
    new_tags: Vec<String>,
    password: Option<&str>,
) -> Result<()> {
    let buffer = META_BUFFERS
        .get()
        .with_context(|| anyhow!("META_BUFFERS not set"))?;

    let meta = {
        let buffer = buffer.read().await;
        let owner_map = buffer
            .get(owner)
            .with_context(|| anyhow!("owner {} not found", owner))?;
        let meta = owner_map
            .get(&uuid.to_ascii_lowercase())
            .with_context(|| anyhow!("uuid {} not found", uuid))?;
        (**meta).clone()
    };

    let mut updated = meta;
    updated.tags = new_tags
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    updated.lower_tags = updated
        .tags
        .iter()
        .map(|t| t.to_ascii_lowercase())
        .collect();

    let meta_bytes = serde_yaml::to_string(&updated)?.into_bytes();

    let to_upload: Vec<u8> = if updated.encrypted {
        let pwd = password.with_context(|| anyhow!("password required for encrypted item"))?;
        let key = super::crypto::derive_key(pwd, owner);
        super::crypto::encrypt(&meta_bytes, &key)?
    } else {
        meta_bytes
    };

    let _permit = S3_SEMAPHORE
        .get()
        .with_context(|| anyhow!("S3_SEMAPHORE not set"))?
        .acquire()
        .await;
    let meta_key = meta_s3_path(&updated);
    upload(&meta_key, &to_upload).await?;

    // Write-through to local cache
    if let (Some(store), Some(bucket)) = (
        META_SYNC_STORE.get(),
        BUCKET.get(),
    ) {
        let _ = store.push(bucket, &meta_key, &to_upload).await;
    }

    let mut buffer = buffer.write().await;
    if let Some(owner_map) = buffer.get_mut(owner) {
        owner_map.insert(uuid.to_ascii_lowercase(), Arc::new(updated));
    }

    Ok(())
}

pub async fn get_content(path: &str) -> Result<Vec<u8>> {
    debug!("fetch {}", path);
    let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
    let data = bucket.get_object(path).await?;
    Ok(data.to_vec())
}

pub async fn get_content_decrypted(
    s3_path: &str,
    encrypted: bool,
    password: Option<&str>,
    owner: &str,
) -> Result<Vec<u8>> {
    let data = get_content(s3_path).await?;
    if encrypted {
        let pwd = password.ok_or_else(|| anyhow!("password required for encrypted content"))?;
        let key = super::crypto::derive_key(pwd, owner);
        super::crypto::decrypt(&data, &key)
    } else {
        Ok(data)
    }
}

async fn full_update(name: &str, password: Option<&str>) -> Result<()> {
    let key: Option<[u8; 32]> = password.map(|p| super::crypto::derive_key(p, name));
    let target_subfolder = super::crypto::compute_subfolder(password.is_some(), password, name);
    let ts = target_subfolder.as_str();

    /// Process a single meta object from S3. Returns Ok(Some(meta)) if successfully
    /// parsed, Ok(None) if skipped (wrong key, etc.), or Err on hard failure.
    async fn update_one(
        bucket: &Box<Bucket>,
        s3_key: String,
        name: &str,
        aes_key: Option<&[u8; 32]>,
    ) -> Result<Option<MetaData>> {
        let _permit = S3_SEMAPHORE
            .get()
            .with_context(|| anyhow!("S3_SEMAPHORE not set"))?
            .acquire()
            .await;
        let raw = bucket.get_object(s3_key.clone()).await?;
        let raw_bytes = raw.as_slice();

        // Determine path_key from the S3 key
        let pk = parse_path_key_from_s3_key(&s3_key, name);

        if super::crypto::is_encrypted(raw_bytes) {
            let k = aes_key.ok_or_else(|| anyhow!("skipped encrypted {}", s3_key))?;
            let decrypted = match super::crypto::decrypt(raw_bytes, k) {
                Ok(d) => d,
                Err(_) => {
                    // Failed to decrypt — if it's in a subfolder, move back to flat
                    if pk.is_some() {
                        let fallback = format!("meta/{}/{}", name,
                            s3_key.rsplit('/').next().unwrap_or("unknown.yml"));
                        warn!("decrypt failed for {}, moving back to {}", s3_key, fallback);
                        let _ = bucket.put_object(fallback, raw_bytes).await;
                        let _ = bucket.delete_object(s3_key).await;
                    }
                    return Err(anyhow!("decrypt failed"));
                }
            };
            let mut meta = serde_yaml::from_slice::<MetaData>(&decrypted)
                .map_err(|_| anyhow!("parse failed"))?;
            meta.owner = name.to_string();
            meta.lower_tags = meta
                .tags
                .iter()
                .map(|t| t.trim().to_ascii_lowercase())
                .collect();
            meta.path_key = pk;
            insert(Arc::new(meta.clone())).await?;
            Ok(Some(meta))
        } else if aes_key.is_some() {
            // Plaintext data but we have a key — skip
            Err(anyhow!("skipped plaintext {}", s3_key))
        } else {
            let mut meta = serde_yaml::from_slice::<MetaData>(raw_bytes).map_err(|e| {
                log::warn!("parse {} failed: {}", s3_key, e);
                anyhow!("parse failed")
            })?;
            meta.owner = name.to_string();
            meta.lower_tags = meta
                .tags
                .iter()
                .map(|t| t.trim().to_ascii_lowercase())
                .collect();
            meta.path_key = pk;
            insert(Arc::new(meta.clone())).await?;
            Ok(Some(meta))
        }
    }

    /// Migrate a meta file from flat path to the target subfolder.
    async fn migrate_meta(
        bucket: &Box<Bucket>,
        meta: &MetaData,
        target_subfolder: &str,
        aes_key: Option<&[u8; 32]>,
    ) -> Result<()> {
        let new_path = format!("meta/{}/{}/{}.yml", meta.owner, target_subfolder, meta.uuid);
        let old_path = format!("meta/{}/{}.yml", meta.owner, meta.uuid);
        if new_path == old_path {
            return Ok(());
        }
        let _permit = S3_SEMAPHORE
            .get()
            .with_context(|| anyhow!("S3_SEMAPHORE not set"))?
            .acquire()
            .await;

        let meta_yaml = serde_yaml::to_string(meta)?;
        let to_upload = if let Some(k) = aes_key {
            super::crypto::encrypt(meta_yaml.as_bytes(), k)?
        } else {
            meta_yaml.into_bytes()
        };

        let store = META_SYNC_STORE.get().with_context(|| anyhow!("META_SYNC_STORE not set"))?;
        store.push(bucket, &new_path, &to_upload).await?;
        bucket.delete_object(&old_path).await?;
        info!("migrated meta {} -> {}", old_path, new_path);
        Ok(())
    }

    #[cfg(feature = "avif")]
    async fn format_all(name: String, key: Option<[u8; 32]>) -> Result<()> {
        let buffer = META_BUFFERS
            .get()
            .with_context(|| anyhow!("META_BUFFERS not set"))?;
        let buffer = buffer.read().await;
        let data = buffer.get(&name).unwrap().clone();
        drop(buffer);
        for (_, item) in data {
            let _ = format_into_avif(item, key.as_ref()).await;
        }
        Ok(())
    }

    info!("update buffer for {}", name);
    let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
    let store = META_SYNC_STORE.get().with_context(|| anyhow!("META_SYNC_STORE not set"))?;

    // Clear existing buffer for this owner
    {
        let buffer = META_BUFFERS
            .get()
            .with_context(|| anyhow!("META_BUFFERS not set"))?;
        buffer.write().await.remove(name);
    }

    // Phase 1: incremental sync from S3 to local via SyncStore (ETag-based)
    let prefix = format!("meta/{}/{}/", name, ts);
    let sync_result = store.sync(bucket, &prefix, None).await?;
    info!(
        "synced meta for {}/{}: downloaded={} skipped={}",
        name, ts, sync_result.downloaded, sync_result.skipped
    );

    // Phase 2: scan flat legacy dir (meta/{name}/*.yml, not inside subfolders)
    // These files still need per-object handling for auto-migration
    {
        let legacy_prefix = format!("meta/{}/", name);
        let list = bucket.list(legacy_prefix, Some("/".to_string())).await?;
        let mut results = Vec::new();
        for page in list {
            for object in page.contents {
                results.push(update_one(bucket, object.key, name, key.as_ref()));
            }
        }
        let mut metas_to_migrate: Vec<MetaData> = Vec::new();
        for r in join_all(results).await {
            match r {
                Ok(Some(meta)) if meta.path_key.is_none() => {
                    metas_to_migrate.push(meta);
                }
                _ => {}
            }
        }
        // Run migrations concurrently
        let migrations: Vec<_> = metas_to_migrate
            .iter()
            .map(|m| migrate_meta(bucket, m, ts, key.as_ref()))
            .collect();
        for r in join_all(migrations).await {
            if let Err(e) = r {
                warn!("migration failed: {}", e);
            }
        }

        // Update path_key in buffer for successfully migrated items
        {
            let buffer = META_BUFFERS
                .get()
                .with_context(|| anyhow!("META_BUFFERS not set"))?;
            let mut buffer = buffer.write().await;
            if let Some(map) = buffer.get_mut(name) {
                for (_, v) in map.iter_mut() {
                    if v.path_key.is_none() && v.encrypted == key.is_some() {
                        Arc::make_mut(v).path_key = Some(ts.to_string());
                    }
                }
            }
        }
    }

    // Phase 3: Load buffer from local files (if sync brought nothing new, load from existing local)
    let any_success = load_buffer_from_local(name, password).await?;

    if any_success {
        info!("finish update buffer for {}", name);
        #[cfg(feature = "avif")]
        if let Some(k) = key {
            tokio::spawn(format_all(name.to_string(), Some(k)));
        }
        Ok(())
    } else {
        Err(anyhow!("no such name: {}", name))
    }
}

/// Load META_BUFFERS from local encrypted/plaintext meta files.
/// Returns true if any entries were loaded.
async fn load_buffer_from_local(name: &str, password: Option<&str>) -> Result<bool> {
    let store = META_SYNC_STORE.get().with_context(|| anyhow!("META_SYNC_STORE not set"))?;
    let subfolder = super::crypto::compute_subfolder(password.is_some(), password, name);
    let key: Option<[u8; 32]> = password.map(|p| super::crypto::derive_key(p, name));

    let dir = store.local_path(&format!("meta/{}/{}/", name, subfolder));
    if !dir.exists() {
        return Ok(false);
    }

    let mut any_success = false;
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(false),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "yml") {
            continue;
        }

        let raw = match std::fs::read(&path) {
            Ok(r) => r,
            Err(e) => {
                warn!("failed to read local meta {:?}: {}", path, e);
                continue;
            }
        };

        if super::crypto::is_encrypted(&raw) {
            let k = match &key {
                Some(k) => k,
                None => {
                    warn!("skipped encrypted local file {:?}: no password", path);
                    continue;
                }
            };
            let decrypted = match super::crypto::decrypt(&raw, k) {
                Ok(d) => d,
                Err(e) => {
                    warn!("failed to decrypt local meta {:?}: {}", path, e);
                    continue;
                }
            };
            let mut meta = match serde_yaml::from_slice::<MetaData>(&decrypted) {
                Ok(m) => m,
                Err(e) => {
                    warn!("failed to parse local meta {:?}: {}", path, e);
                    continue;
                }
            };
            meta.owner = name.to_string();
            meta.lower_tags = meta.tags.iter().map(|t| t.trim().to_ascii_lowercase()).collect();
            meta.path_key = Some(subfolder.clone());
            insert(Arc::new(meta)).await?;
            any_success = true;
        } else if key.is_none() {
            let mut meta = match serde_yaml::from_slice::<MetaData>(&raw) {
                Ok(m) => m,
                Err(e) => {
                    warn!("failed to parse local meta {:?}: {}", path, e);
                    continue;
                }
            };
            meta.owner = name.to_string();
            meta.lower_tags = meta.tags.iter().map(|t| t.trim().to_ascii_lowercase()).collect();
            meta.path_key = Some(subfolder.clone());
            insert(Arc::new(meta)).await?;
            any_success = true;
        }
    }

    Ok(any_success)
}

pub async fn force_update(name: &str, password: Option<&str>) -> Result<()> {
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
                    info!("waited buffer for {}", name);
                    break;
                }
            }
        }
    }
    {
        let mut set = LOCK.get_or_init(|| Mutex::new(HashSet::new())).lock().await;
        set.insert(name.to_string());
    }
    let t = full_update(name, password).await;
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
            force_update(name, None).await
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

pub async fn list(
    name: &str,
    filter: Option<&str>,
    password: Option<&str>,
) -> Result<Vec<Arc<MetaData>>> {
    if let Some(pwd) = password {
        let key = super::crypto::derive_key(pwd, name);
        let key_hash: [u8; 8] = key[..8].try_into().unwrap();
        let hash_map = LAST_KEY_HASH
            .get_or_init(|| RwLock::new(HashMap::new()))
            .read()
            .await;
        if hash_map.get(name) != Some(&key_hash) {
            drop(hash_map);
            force_update(name, Some(pwd)).await?;
            LAST_KEY_HASH
                .get()
                .unwrap()
                .write()
                .await
                .insert(name.to_string(), key_hash);
        }
    } else {
        let _ = update(name).await;
    }
    let buffer = META_BUFFERS
        .get()
        .with_context(|| anyhow!("BUFFERS not set"))?;
    let buffer = buffer.read().await;
    let buffer = buffer.get(name);
    if buffer.is_none() {
        return Err(anyhow!("{} not found", name));
    }
    let buffer = buffer.unwrap();
    let mut result: Vec<Arc<MetaData>> = buffer.values().map(|v| v.clone()).collect();
    let filters = split_filter(filter);
    debug!("{}", result.len());
    for hide in HIDE.get().unwrap_or(&Vec::new()) {
        if !filters.iter().any(|f| f.text == *hide) {
            debug!("remove {}", hide);
            result.retain(|v| v.lower_tags.iter().all(|s| !s.contains(hide)));
            debug!("{}", result.len());
        }
    }
    for f in &filters {
        let key = &f.text;
        if f.negative {
            debug!("remove {}", key);
            if f.exact {
                result.retain(|v| {
                    v.lower_tags.iter().all(|s| s != key) && v.content_type != *key
                });
            } else {
                result.retain(|v| {
                    v.lower_tags.iter().all(|s| !s.contains(key))
                        && !v.content_type.contains(key)
                });
            }
        } else {
            debug!("keep {}", key);
            if f.exact {
                result.retain(|v| {
                    v.lower_tags.iter().any(|s| s == key) || v.content_type == *key
                });
            } else {
                result.retain(|v| {
                    v.lower_tags.iter().any(|s| s.contains(key))
                        || v.content_type.contains(key)
                });
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

/// Write-through helper: push meta bytes to local cache via SyncStore.
/// Called after successful S3 upload. Errors are logged but not propagated
/// (local cache is best-effort, S3 is source of truth).
pub async fn push_meta_to_local_cache(meta_key: &str, encrypted_bytes: &[u8]) {
    if let (Some(store), Some(bucket)) = (META_SYNC_STORE.get(), BUCKET.get()) {
        if let Err(e) = store.push(bucket, meta_key, encrypted_bytes).await {
            warn!("push_meta_to_local_cache failed for {}: {}", meta_key, e);
        }
    }
}

pub async fn remove(key: &str) -> anyhow::Result<()> {
    warn!("remove {}", key);
    let bucket = BUCKET.get().with_context(|| anyhow!("BUCKET not set"))?;
    bucket.delete_object(key).await?;
    Ok(())
}

pub async fn remove_meta(meta: Arc<MetaData>) -> anyhow::Result<()> {
    warn!("remove meta {}", meta.uuid);
    let mut buffer = META_BUFFERS
        .get()
        .with_context(|| anyhow!("META_BUFFERS not set"))?
        .write()
        .await;
    if let Some(map) = buffer.get_mut(&meta.owner) {
        map.remove(&meta.uuid);
    }
    // Invalidate local cache
    if let Some(store) = META_SYNC_STORE.get() {
        store.invalidate(&meta_s3_path(&meta));
    }
    let keys = vec![
        meta_s3_path(&meta),
        format!("raw/{}/{}", meta.owner, meta.filename),
        format!("thumbnail/{}/{}", meta.owner, meta.thumbnail),
    ];
    join_all(keys.iter().map(|key| async {
        let _permit = S3_SEMAPHORE
            .get()
            .with_context(|| anyhow!("S3_SEMAPHORE not set"))?
            .acquire()
            .await;
        remove(key).await
    }))
    .await;

    Ok(())
}

pub async fn reencrypt(
    name: &str,
    filter: Option<&str>,
    old_password: Option<&str>,
    new_password: Option<&str>,
) -> Result<ReencryptResponse> {
    let list = list(name, filter, old_password).await?;
    let old_key: Option<[u8; 32]> = old_password.map(|p| super::crypto::derive_key(p, name));
    let new_key: Option<[u8; 32]> = new_password.map(|p| super::crypto::derive_key(p, name));
    let mut processed = 0usize;
    let mut errors = Vec::new();

    for item in &list {
        let uuid = item.uuid.clone();
        match reencrypt_one(item, &old_key, new_key.as_ref(), new_password).await {
            Ok(()) => processed += 1,
            Err(e) => errors.push(ReencryptError {
                uuid,
                error: format!("{}", e),
            }),
        }
    }

    if processed > 0 {
        if let Some(ref nk) = new_key {
            let new_key_hash: [u8; 8] = nk[..8].try_into().unwrap();
            LAST_KEY_HASH
                .get()
                .unwrap()
                .write()
                .await
                .insert(name.to_string(), new_key_hash);
        }
    }

    Ok(ReencryptResponse { processed, errors })
}

async fn reencrypt_one(
    item: &MetaData,
    old_key: &Option<[u8; 32]>,
    new_key: Option<&[u8; 32]>,
    new_password: Option<&str>,
) -> Result<()> {
    let was_encrypted = item.encrypted;
    let old_meta_key = meta_s3_path(item);
    let raw_key = format!("raw/{}/{}", item.owner, item.filename);
    let thumb_key = format!("thumbnail/{}/{}", item.owner, item.thumbnail);

    // Determine new meta path
    let new_subfolder = if new_key.is_some() {
        super::crypto::compute_path_hash(
            new_password.ok_or_else(|| anyhow!("new password required for encrypted"))?,
            &item.owner,
        )
    } else {
        super::crypto::PLAIN_FOLDER.to_string()
    };
    let new_meta_key = format!("meta/{}/{}/{}.yml", item.owner, new_subfolder, item.uuid);

    // Fetch and update meta
    let meta_bytes = get_content(&old_meta_key).await?;
    let mut meta: MetaData = if super::crypto::is_encrypted(&meta_bytes) {
        let k = old_key
            .as_ref()
            .ok_or_else(|| anyhow!("encrypted meta but no old password"))?;
        let decrypted = super::crypto::decrypt(&meta_bytes, k)?;
        serde_yaml::from_slice(&decrypted)?
    } else {
        serde_yaml::from_slice(&meta_bytes)?
    };

    // Fetch and process raw
    let raw_bytes = get_content(&raw_key).await?;
    let raw_plain = if was_encrypted {
        let k = old_key
            .as_ref()
            .ok_or_else(|| anyhow!("encrypted raw but no old password"))?;
        super::crypto::decrypt(&raw_bytes, k)?
    } else {
        raw_bytes
    };
    let raw_to_upload = if let Some(nk) = new_key {
        meta.encrypted = true;
        super::crypto::encrypt(&raw_plain, nk)?
    } else {
        meta.encrypted = false;
        raw_plain
    };
    upload(&raw_key, &raw_to_upload).await?;

    // Fetch and process thumbnail
    let thumb_bytes = get_content(&thumb_key).await?;
    let thumb_plain = if was_encrypted {
        let k = old_key
            .as_ref()
            .ok_or_else(|| anyhow!("encrypted thumbnail but no old password"))?;
        super::crypto::decrypt(&thumb_bytes, k)?
    } else {
        thumb_bytes
    };
    let thumb_to_upload = if let Some(nk) = new_key {
        super::crypto::encrypt(&thumb_plain, nk)?
    } else {
        thumb_plain
    };
    upload(&thumb_key, &thumb_to_upload).await?;

    // Update meta's path_key and upload to new path
    meta.path_key = Some(new_subfolder);

    // Upload updated meta to NEW path
    let meta_yaml = serde_yaml::to_string(&meta)?;
    let meta_to_upload = if let Some(nk) = new_key {
        super::crypto::encrypt(meta_yaml.as_bytes(), nk)?
    } else {
        meta_yaml.into_bytes()
    };
    upload(&new_meta_key, &meta_to_upload).await?;

    // Delete old meta if path changed
    if old_meta_key != new_meta_key {
        remove(&old_meta_key).await?;
    }

    // Write-through to local cache
    if let (Some(store), Some(bucket)) = (
        META_SYNC_STORE.get(),
        BUCKET.get(),
    ) {
        let _ = store.push(bucket, &new_meta_key, &meta_to_upload).await;
        if old_meta_key != new_meta_key {
            store.invalidate(&old_meta_key);
        }
    }

    // Update in-memory buffer
    let buffer = META_BUFFERS
        .get()
        .with_context(|| anyhow!("META_BUFFERS not set"))?;
    let mut buffer = buffer.write().await;
    if let Some(map) = buffer.get_mut(&meta.owner) {
        map.insert(meta.uuid.to_ascii_lowercase(), Arc::new(meta));
    }

    Ok(())
}

pub async fn init(config: &Figment) -> anyhow::Result<()> {
    let endpoint = config
        .find_value("meme.endpoint")?
        .as_str()
        .ok_or(anyhow!("endpoint not found"))?
        .to_owned();
    let region = config
        .find_value("meme.region")?
        .as_str()
        .ok_or(anyhow!("region not found"))?
        .to_owned();
    let bucket = config
        .find_value("meme.bucket")?
        .as_str()
        .ok_or(anyhow!("bucket not found"))?
        .to_owned();

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
    let hide: Vec<_> = hide
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
    let data_path = config
        .find_value("data_path")?
        .as_str()
        .ok_or(anyhow!("data_path not set"))?
        .to_owned();
    let key_file = Path::new(&data_path).join(key_file);
    let key_salt = config
        .find_value("meme.key_salt")?
        .as_str()
        .ok_or(anyhow!("key_salt not found"))?
        .to_owned();
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

    let bucket = Bucket::new(&bucket, region, credentials)?.with_path_style();
    BUCKET.get_or_init(|| bucket);
    let s3_concurrency: usize = config
        .find_value("meme.s3_concurrency")
        .ok()
        .and_then(|v| v.to_u128())
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(16);
    S3_SEMAPHORE.get_or_init(|| Arc::new(Semaphore::new(s3_concurrency)));
    META_BUFFERS.get_or_init(|| RwLock::new(HashMap::new()));
    LAST_KEY_HASH.get_or_init(|| RwLock::new(HashMap::new()));

    let meme_meta_dir = Path::new(&data_path).join("meme_meta");
    let _ = std::fs::create_dir_all(&meme_meta_dir);
    META_SYNC_STORE.get_or_init(|| SyncStore::new("meme/s3_etag/", meme_meta_dir));

    HIDE.get_or_init(|| hide.iter().map(|s| s.trim().to_ascii_lowercase()).collect());

    super::validation::init(&jwt_secret_key, &key_salt, &key_file).await?;

    return Ok(());
}
