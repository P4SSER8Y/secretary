use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use base64::Engine;
use log::{info, warn};
use serde::Deserialize;

/// CouchDB sync configuration, loaded from Rocket.toml via Figment.
#[derive(Deserialize, Clone)]
pub struct CouchDbConfig {
    pub url: String,
    pub db: String,
    pub prefix: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub enabled: bool,
}

/// Tracks last sync time for auto-refresh throttling.
pub struct SyncState {
    pub last_sync: Instant,
}

impl SyncState {
    pub fn new() -> Self {
        // Initialize far in the past so the first access triggers a sync.
        Self {
            last_sync: Instant::now() - std::time::Duration::from_secs(4 * 3600),
        }
    }
}

// ── Internal deserialization helpers ──

#[derive(Deserialize)]
struct CouchDoc {
    #[serde(rename = "_id")]
    #[allow(dead_code)]
    id: String,
    path: Option<String>,
    children: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ChunkDoc {
    #[serde(rename = "_id")]
    id: String,
    data: Option<String>,
}

// ── HTTP helpers ──

fn build_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .context("Failed to build HTTP client")
}

fn auth_url(config: &CouchDbConfig) -> String {
    let scheme = if config.url.starts_with("https") {
        "https"
    } else {
        "http"
    };
    let host = config
        .url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    format!("{}://{}:{}@{}", scheme, config.username, config.password, host)
}

fn recipes_dir() -> PathBuf {
    Path::new(utils::get_data_path()).join("recipes").join("raw")
}

/// Strip the configured CouchDB prefix from a document path, returning the relative path.
/// e.g. prefix="300-life/302-食谱/", path="300-life/302-食谱/菜/蚝油生菜.md" → "菜/蚝油生菜.md"
fn strip_prefix<'a>(prefix: &str, path: &'a str) -> &'a str {
    path.strip_prefix(prefix).unwrap_or(path)
}

// ── CouchDB queries ──

async fn fetch_parent_docs(
    client: &reqwest::Client,
    config: &CouchDbConfig,
) -> anyhow::Result<Vec<CouchDoc>> {
    let base = auth_url(config);
    let startkey = percent_encode(&config.prefix);
    let endkey = format!("{}%EF%BF%BF", startkey);

    let url = format!(
        "{}/{}/_all_docs?startkey=%22{}%22&endkey=%22{}%22&include_docs=true",
        base, config.db, startkey, endkey
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch parent docs")?;

    #[derive(Deserialize)]
    struct AllDocsResponse {
        rows: Vec<RowDoc>,
    }
    #[derive(Deserialize)]
    struct RowDoc {
        doc: Option<CouchDoc>,
    }

    let body: AllDocsResponse = resp.json().await.context("Failed to parse parent docs")?;
    let docs: Vec<CouchDoc> = body.rows.into_iter().filter_map(|r| r.doc).collect();

    info!("Fetched {} recipe parent docs from CouchDB", docs.len());
    Ok(docs)
}

async fn fetch_chunks(
    client: &reqwest::Client,
    config: &CouchDbConfig,
    chunk_ids: &[String],
) -> anyhow::Result<Vec<ChunkDoc>> {
    if chunk_ids.is_empty() {
        return Ok(vec![]);
    }

    let base = auth_url(config);
    let url = format!("{}/{}/_all_docs?include_docs=true", base, config.db);

    #[derive(serde::Serialize)]
    struct KeysReq {
        keys: Vec<String>,
    }

    let resp = client
        .post(&url)
        .json(&KeysReq {
            keys: chunk_ids.to_vec(),
        })
        .send()
        .await
        .context("Failed to fetch chunk docs")?;

    #[derive(Deserialize)]
    struct AllDocsResponse {
        rows: Vec<RowDoc>,
    }
    #[derive(Deserialize)]
    struct RowDoc {
        doc: Option<ChunkDoc>,
    }

    let body: AllDocsResponse = resp.json().await.context("Failed to parse chunk docs")?;
    let chunks: Vec<ChunkDoc> = body.rows.into_iter().filter_map(|r| r.doc).collect();

    info!("Fetched {} chunk docs", chunks.len());
    Ok(chunks)
}

// ── Utility ──

fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match b {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                result.push(*b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}

// ── Core sync logic ──

/// Recognised image file extensions.
fn is_image_ext(ext: &str) -> bool {
    matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "gif" | "svg")
}

/// Pull all files from CouchDB and write them into `recipes_dir`,
/// preserving the exact directory structure and filenames from CouchDB.
/// Returns the total number of files written.
async fn sync_from_couchdb(
    config: &CouchDbConfig,
    recipes_dir: &Path,
) -> anyhow::Result<usize> {
    let cl = build_client()?;

    let parents = fetch_parent_docs(&cl, config).await?;
    let chunk_ids: Vec<String> = parents
        .iter()
        .filter_map(|p| p.children.as_ref())
        .flatten()
        .cloned()
        .collect();

    if chunk_ids.is_empty() {
        info!("No chunks found under CouchDB prefix");
        return Ok(0);
    }

    let chunks = fetch_chunks(&cl, config, &chunk_ids).await?;
    let chunk_map: HashMap<&str, &ChunkDoc> = chunks.iter().map(|c| (c.id.as_str(), c)).collect();

    let mut written = 0usize;
    let mut known_relpaths: Vec<String> = Vec::new();

    for parent in &parents {
        let path = parent.path.as_deref().unwrap_or("");
        let rel = strip_prefix(&config.prefix, path);
        if rel.is_empty() || rel == path {
            continue; // path doesn't start with prefix, skip
        }

        let children_ids = match parent.children.as_ref() {
            Some(ids) if !ids.is_empty() => ids,
            _ => continue,
        };
        // Concatenate all chunks' data (handles both single and multi-chunk files)
        let mut combined_data = String::new();
        for child_id in children_ids {
            if let Some(chunk) = chunk_map.get(child_id.as_str()) {
                if let Some(d) = &chunk.data {
                    combined_data.push_str(d);
                }
            }
        }
        if combined_data.is_empty() {
            continue;
        }
        let data = &combined_data;

        let file_path = recipes_dir.join(rel);

        // Ensure parent directories exist
        if let Some(parent_dir) = file_path.parent() {
            std::fs::create_dir_all(parent_dir)
                .with_context(|| format!("Failed to create dir: {:?}", parent_dir))?;
        }

        // Determine if this is a binary (image) or text file by extension
        let ext = Path::new(rel)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if is_image_ext(ext) {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(data)
                .with_context(|| format!("Failed to decode base64: {}", rel))?;
            std::fs::write(&file_path, &decoded)
                .with_context(|| format!("Failed to write image: {:?}", file_path))?;
            info!("Synced image: {}", rel);
        } else {
            std::fs::write(&file_path, data)
                .with_context(|| format!("Failed to write file: {:?}", file_path))?;
            info!("Synced file: {}", rel);
        }

        known_relpaths.push(rel.to_string());
        written += 1;
    }

    // ── Clean up local files no longer present in CouchDB ──
    let known_set: std::collections::HashSet<&str> = known_relpaths.iter().map(|s| s.as_str()).collect();
    clean_stale_files(recipes_dir, recipes_dir, &known_set);

    Ok(written)
}

/// Recursively remove files under `base_dir` whose relative path (from `base_dir`)
/// is not in `known_set`.
fn clean_stale_files(base_dir: &Path, current: &Path, known: &std::collections::HashSet<&str>) {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            clean_stale_files(base_dir, &path, known);
            // Remove empty directory
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
/// Triggers a background sync if more than 3 hours have passed since the last sync.
pub async fn maybe_auto_sync() {
    let config = match crate::COUCHDB_CONFIG.get().and_then(|c| c.as_ref()) {
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
        match sync_from_couchdb(&config, &dir).await {
            Ok(n) => {
                info!("Auto-sync completed: {} recipes", n);
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

/// Manual sync + index rebuild. Returns the number of recipes synced.
pub async fn manual_sync_and_reload() -> anyhow::Result<usize> {
    let config = crate::COUCHDB_CONFIG
        .get()
        .and_then(|c| c.as_ref())
        .context("CouchDB config not present")?;

    if !config.enabled {
        anyhow::bail!("CouchDB sync is not enabled");
    }

    let dir = recipes_dir();
    let count = sync_from_couchdb(config, &dir).await?;
    rebuild_index(&dir)?;

    // Update last sync timestamp
    if let Some(state) = crate::SYNC_STATE.get() {
        if let Ok(mut s) = state.write() {
            s.last_sync = Instant::now();
        }
    }

    Ok(count)
}
