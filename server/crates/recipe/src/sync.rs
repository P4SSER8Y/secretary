use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use log::{info, warn};
use serde::Deserialize;

/// S3 sync configuration, loaded from Rocket.toml via Figment.
#[derive(Deserialize, Clone)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    #[serde(default)]
    pub enabled: bool,
}

/// Tracks last sync time for auto-refresh throttling.
pub struct SyncState {
    pub last_sync: Instant,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            last_sync: Instant::now() - std::time::Duration::from_secs(4 * 3600),
        }
    }
}

// ── S3 helpers ──

fn build_bucket(config: &S3Config) -> anyhow::Result<Box<s3::Bucket>> {
    use s3::creds::Credentials;
    use s3::Region;

    let region = Region::Custom {
        region: config.region.clone(),
        endpoint: config.endpoint.clone(),
    };
    let credentials = Credentials {
        access_key: Some(config.access_key.clone()),
        secret_key: Some(config.secret_key.clone()),
        security_token: None,
        session_token: None,
        expiration: None,
    };
    let mut bucket = s3::Bucket::new(&config.bucket, region, credentials)
        .with_context(|| "Failed to create S3 bucket handle")?
        .with_path_style();
    // Garage needs ListObjects v1 when listing without prefix; v2 may return
    // incomplete results with certain parameter combinations.
    bucket.set_listobjects_v1();
    Ok(bucket)
}

fn recipes_dir() -> PathBuf {
    Path::new(utils::get_data_path()).join("recipes").join("raw")
}

// ── ETag cache (sled) ──

fn etag_key(s3_key: &str) -> Vec<u8> {
    format!("recipe/s3_etag/{}", s3_key).into_bytes()
}

fn get_cached_etag(db: &sled::Db, s3_key: &str) -> Option<String> {
    db.get(etag_key(s3_key)).ok().flatten()
        .and_then(|v| String::from_utf8(v.to_vec()).ok())
}

fn set_cached_etag(db: &sled::Db, s3_key: &str, etag: &str) {
    let _ = db.insert(etag_key(s3_key), etag.as_bytes());
}

fn remove_cached_etag(db: &sled::Db, s3_key: &str) {
    let _ = db.remove(etag_key(s3_key));
}

/// Delete all sled ETag entries whose keys no longer exist in S3.
fn purge_stale_etags(db: &sled::Db, known_keys: &HashSet<String>) {
    let prefix = "recipe/s3_etag/";
    let mut stale: Vec<String> = Vec::new();
    for item in db.scan_prefix(prefix) {
        if let Ok((k, _)) = item {
            if let Some(s3_key) = std::str::from_utf8(&k)
                .ok()
                .and_then(|s| s.strip_prefix(prefix))
            {
                if !known_keys.contains(s3_key) {
                    stale.push(s3_key.to_string());
                }
            }
        }
    }
    for key in &stale {
        remove_cached_etag(db, key);
    }
    if !stale.is_empty() {
        info!("Purged {} stale ETag entries", stale.len());
    }
}

// ── Core sync logic ──

/// Pull all files from S3 and write them into `recipes_dir`,
/// preserving the directory structure and filenames from S3 keys.
/// Uses sled-stored ETags for incremental sync: only downloads files
/// whose ETag has changed since last sync.
async fn sync_from_s3(
    config: &S3Config,
    recipes_dir: &Path,
) -> anyhow::Result<usize> {
    let bucket = build_bucket(config)?;
    let db = utils::database::get_db();
    info!("S3 sync: bucket='{}'", config.bucket);

    let mut written = 0usize;
    let mut known_keys: HashSet<String> = HashSet::new();

    // Garage returns empty contents when listing with empty prefix and no delimiter.
    // Workaround: first list with delimiter "/" to discover top-level directory
    // prefixes, then list each directory individually with a non-empty prefix.
    let root_list = bucket
        .list("".to_string(), Some("/".to_string()))
        .await
        .context("Failed to list S3 root with delimiter")?;

    let mut dir_prefixes: Vec<String> = Vec::new();
    for page in root_list {
        for obj in page.contents {
            written += sync_one(&bucket, &db, recipes_dir, &obj, &mut known_keys).await?;
        }
        if let Some(cp) = page.common_prefixes {
            for p in cp {
                dir_prefixes.push(p.prefix.clone());
            }
        }
    }

    for dir_prefix in &dir_prefixes {
        info!("S3 sync: listing '{}'", dir_prefix);
        let dir_list = bucket
            .list(dir_prefix.clone(), None)
            .await
            .with_context(|| format!("Failed to list S3 dir '{}'", dir_prefix))?;

        for page in dir_list {
            for obj in page.contents {
                written += sync_one(&bucket, &db, recipes_dir, &obj, &mut known_keys).await?;
            }
        }
    }

    // Clean up local files no longer present in S3
    info!("S3 sync done: {} files downloaded, cleaning stale", written);
    let known_set: HashSet<&str> = known_keys.iter().map(|s| s.as_str()).collect();
    clean_stale_files(recipes_dir, recipes_dir, &known_set);
    purge_stale_etags(&db, &known_keys);

    Ok(written)
}

/// Sync a single S3 object: check ETag, skip if unchanged, otherwise download.
/// Returns 1 if downloaded, 0 if skipped.
async fn sync_one(
    bucket: &s3::Bucket,
    db: &sled::Db,
    recipes_dir: &Path,
    obj: &s3::serde_types::Object,
    known_keys: &mut HashSet<String>,
) -> anyhow::Result<usize> {
    let key = &obj.key;
    known_keys.insert(key.clone());

    // Check ETag — skip download if unchanged AND local file exists
    if let Some(ref remote_etag) = obj.e_tag {
        if let Some(cached_etag) = get_cached_etag(db, key) {
            if cached_etag == *remote_etag && recipes_dir.join(key).exists() {
                info!("Skipped (unchanged): {}", key);
                return Ok(0);
            }
        }
    }

    // Download and write
    let data = bucket
        .get_object(key)
        .await
        .with_context(|| format!("Failed to get S3 object: {}", key))?;

    let file_path = recipes_dir.join(key);
    if let Some(parent_dir) = file_path.parent() {
        std::fs::create_dir_all(parent_dir)
            .with_context(|| format!("Failed to create dir: {:?}", parent_dir))?;
    }

    std::fs::write(&file_path, data.as_slice())
        .with_context(|| format!("Failed to write file: {:?}", file_path))?;
    info!("Synced: {}", key);

    // Cache the new ETag
    if let Some(ref etag) = obj.e_tag {
        set_cached_etag(db, key, etag);
    }

    Ok(1)
}

/// Recursively remove files under `base_dir` whose relative path (from `base_dir`)
/// is not in `known_set`.
fn clean_stale_files(base_dir: &Path, current: &Path, known: &HashSet<&str>) {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            clean_stale_files(base_dir, &path, known);
            if std::fs::read_dir(&path).map_or(false, |mut d| d.next().is_none()) {
                if let Err(e) = std::fs::remove_dir(&path) {
                    warn!("Failed to remove empty dir {:?}: {}", path, e);
                } else {
                    info!("Removed empty dir: {:?}", path);
                }
            }
        } else if let Ok(rel) = path.strip_prefix(base_dir) {
            let rel_str = rel.to_string_lossy();
            if !known.contains(rel_str.as_ref()) {
                if let Err(e) = std::fs::remove_file(&path) {
                    warn!("Failed to remove stale file {:?}: {}", path, e);
                } else {
                    info!("Removed stale file: {:?}", path);
                }
            }
        }
    }
}

fn rebuild_index(recipes_dir: &Path) -> anyhow::Result<()> {
    let new_index = crate::indexer::scan_recipes(recipes_dir)?;
    if let Some(index) = crate::RECIPE_INDEX.get() {
        let mut w = index.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        *w = new_index;
    }
    Ok(())
}

// ── Public API ──

/// Called at the beginning of recipe GET endpoints.
/// Triggers a background S3 sync if more than 3 hours have passed since the last sync.
pub async fn maybe_auto_sync() {
    let config = match crate::S3_CONFIG.get().and_then(|c| c.as_ref()) {
        Some(c) if c.enabled => c,
        _ => return,
    };

    let state = match crate::SYNC_STATE.get() {
        Some(s) => s,
        None => return,
    };

    {
        let s = state.read().unwrap();
        if s.last_sync.elapsed() < std::time::Duration::from_secs(3 * 3600) {
            return;
        }
    }

    info!("Auto-sync triggered (last sync > 3h ago)");

    let config = config.clone();
    let dir = recipes_dir();

    tokio::spawn(async move {
        match sync_from_s3(&config, &dir).await {
            Ok(n) => {
                info!("Auto-sync completed: {} files from S3", n);
                if let Err(e) = rebuild_index(&dir) {
                    warn!("Failed to rebuild index after auto-sync: {}", e);
                }
                if let Some(state) = crate::SYNC_STATE.get() {
                    if let Ok(mut s) = state.write() {
                        s.last_sync = Instant::now();
                    }
                }
            }
            Err(e) => warn!("Auto-sync failed: {}", e),
        }
    });
}

/// Manual sync + index rebuild. Returns the number of files synced.
pub async fn manual_sync_and_reload() -> anyhow::Result<usize> {
    let config = crate::S3_CONFIG
        .get()
        .and_then(|c| c.as_ref())
        .context("S3 config not present")?;

    if !config.enabled {
        anyhow::bail!("S3 sync is not enabled");
    }

    let dir = recipes_dir();
    let count = sync_from_s3(config, &dir).await?;
    rebuild_index(&dir)?;

    if let Some(state) = crate::SYNC_STATE.get() {
        if let Ok(mut s) = state.write() {
            s.last_sync = Instant::now();
        }
    }

    Ok(count)
}
