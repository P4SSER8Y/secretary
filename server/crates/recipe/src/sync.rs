use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use log::{info, warn};
use serde::Deserialize;

use s3_sync::SyncStore;

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

fn sync_store() -> SyncStore {
    SyncStore::new("recipe/s3_etag/", recipes_dir())
}

// ── Core sync logic ──

/// Pull all files from S3 and write them into `recipes_dir`,
/// preserving the directory structure and filenames from S3 keys.
/// Uses sled-stored ETags for incremental sync: only downloads files
/// whose ETag has changed since last sync.
async fn sync_from_s3(config: &S3Config) -> anyhow::Result<usize> {
    let bucket = build_bucket(config)?;
    let store = sync_store();
    info!("S3 sync: bucket='{}'", config.bucket);

    let mut total_downloaded = 0usize;
    let mut all_known_keys: HashSet<String> = HashSet::new();

    // Garage returns empty contents when listing with empty prefix and no delimiter.
    // Workaround: first list with delimiter "/" to discover top-level directory
    // prefixes, then sync each directory individually with a non-empty prefix.
    //
    // Phase 1: list root with delimiter → sync root-level objects + discover dirs
    let root_result = store.sync(&bucket, "", Some("/")).await?;
    total_downloaded += root_result.downloaded;
    all_known_keys.extend(root_result.known_keys);
    let dir_prefixes = root_result.common_prefixes;

    // Phase 2: sync each directory recursively
    for dir_prefix in &dir_prefixes {
        let result = store
            .sync(&bucket, dir_prefix, None)
            .await
            .with_context(|| format!("Failed to sync S3 dir '{}'", dir_prefix))?;
        total_downloaded += result.downloaded;
        all_known_keys.extend(result.known_keys);
    }

    // Clean up local files and ETags no longer present in S3
    info!(
        "S3 sync done: {} files downloaded, cleaning stale",
        total_downloaded
    );
    store.cleanup(&all_known_keys);

    Ok(total_downloaded)
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

/// Upload a single object to S3 + local copy + ETag cache.
pub async fn push_to_s3(key: &str, data: &[u8]) -> anyhow::Result<()> {
    let config = crate::S3_CONFIG
        .get()
        .and_then(|c| c.as_ref())
        .context("S3 config not present")?;

    if !config.enabled {
        anyhow::bail!("S3 sync is not enabled");
    }

    let bucket = build_bucket(config)?;
    let store = sync_store();
    store.push(&bucket, key, data).await?;

    Ok(())
}

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
        match sync_from_s3(&config).await {
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
    let count = sync_from_s3(config).await?;
    rebuild_index(&dir)?;

    if let Some(state) = crate::SYNC_STATE.get() {
        if let Ok(mut s) = state.write() {
            s.last_sync = Instant::now();
        }
    }

    Ok(count)
}
