use std::collections::HashSet;
use std::path::Path;

use anyhow::Context;
use log::{info, warn};

use crate::etag;
use crate::SyncStore;

/// Result of a sync operation.
pub struct SyncResult {
    pub downloaded: usize,
    pub skipped: usize,
    /// All S3 keys discovered during this sync (for later cleanup aggregation)
    pub known_keys: HashSet<String>,
    /// Common prefixes returned by S3 (for callers that need to discover subdirectories)
    pub common_prefixes: Vec<String>,
}

// ── Core sync ──

/// List S3 objects under `prefix`, compare ETags with local cache,
/// download only changed/new objects.
///
/// Does NOT clean stale files or ETags — call `store.cleanup(&all_known_keys)`
/// after all sync calls are complete (useful when syncing multiple prefixes).
pub async fn sync_from_s3(
    store: &SyncStore,
    bucket: &s3::Bucket,
    prefix: &str,
    delimiter: Option<&str>,
) -> anyhow::Result<SyncResult> {
    let mut known_keys = HashSet::new();
    let mut downloaded = 0usize;
    let mut skipped = 0usize;
    let mut common_prefixes = Vec::new();

    let delimiter_str = delimiter.map(|d| d.to_string());
    let list = bucket
        .list(prefix.to_string(), delimiter_str)
        .await
        .with_context(|| format!("Failed to list S3 prefix '{}'", prefix))?;

    for page in &list {
        for obj in &page.contents {
            known_keys.insert(obj.key.clone());

            // Check ETag cache
            if let Some(ref remote_etag) = obj.e_tag {
                if let Some(cached_etag) = etag::get_cached_etag(&store.db_prefix, &obj.key) {
                    if cached_etag == *remote_etag {
                        let local_path = store.local_path(&obj.key);
                        if local_path.exists() {
                            skipped += 1;
                            continue;
                        }
                    }
                }
            }

            // Download and write locally
            let data = bucket
                .get_object(&obj.key)
                .await
                .with_context(|| format!("Failed to get S3 object: {}", &obj.key))?;

            let local_path = store.local_path(&obj.key);
            if let Some(parent_dir) = local_path.parent() {
                std::fs::create_dir_all(parent_dir)
                    .with_context(|| format!("Failed to create dir: {:?}", parent_dir))?;
            }

            // Atomic write: .tmp → rename
            let tmp_path = local_path.with_extension("tmp");
            std::fs::write(&tmp_path, data.as_slice())
                .with_context(|| format!("Failed to write file: {:?}", tmp_path))?;
            std::fs::rename(&tmp_path, &local_path)
                .with_context(|| format!("Failed to rename {:?} → {:?}", tmp_path, local_path))?;
            info!("s3_sync: downloaded {}", &obj.key);

            // Cache the new ETag
            if let Some(ref e_tag) = obj.e_tag {
                etag::set_cached_etag(&store.db_prefix, &obj.key, e_tag);
            }

            downloaded += 1;
        }

        // Collect common prefixes for callers that need to discover subdirectories
        if let Some(ref cp) = page.common_prefixes {
            for p in cp {
                common_prefixes.push(p.prefix.clone());
            }
        }
    }

    info!(
        "s3_sync: prefix='{}' downloaded={} skipped={}",
        prefix, downloaded, skipped
    );

    Ok(SyncResult {
        downloaded,
        skipped,
        known_keys,
        common_prefixes,
    })
}

// ── Push ──

/// Upload to S3, write local copy, and cache the ETag from the response.
pub async fn push_to_s3(
    store: &SyncStore,
    bucket: &s3::Bucket,
    key: &str,
    data: &[u8],
) -> anyhow::Result<()> {
    let response = bucket
        .put_object(key, data)
        .await
        .with_context(|| format!("Failed to push to S3: {}", key))?;

    // Write local copy atomically
    let local_path = store.local_path(key);
    if let Some(parent_dir) = local_path.parent() {
        std::fs::create_dir_all(parent_dir)
            .with_context(|| format!("Failed to create dir: {:?}", parent_dir))?;
    }
    let tmp_path = local_path.with_extension("tmp");
    std::fs::write(&tmp_path, data)?;
    std::fs::rename(&tmp_path, &local_path)?;

    // Cache ETag from response headers
    let headers = response.headers();
    if let Some(e_tag) = headers.get("ETag").or_else(|| headers.get("etag")) {
        etag::set_cached_etag(&store.db_prefix, key, e_tag);
    } else {
        // ETag not returned — invalidate so next sync re-downloads to be safe
        warn!("s3_sync: push '{}' — no ETag in response, invalidating cache entry", key);
        etag::remove_cached_etag(&store.db_prefix, key);
    }

    info!("s3_sync: pushed {} ({} bytes)", key, data.len());
    Ok(())
}

// ── Clean stale files ──

/// Recursively remove files under `base_dir` whose relative path from `base_dir`
/// is not in `known_keys`. Also removes empty directories.
pub fn clean_stale_files(base_dir: &Path, current: &Path, known_keys: &HashSet<String>) {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            clean_stale_files(base_dir, &path, known_keys);
            // Remove empty directories
            if std::fs::read_dir(&path).map_or(false, |mut d| d.next().is_none()) {
                if let Err(e) = std::fs::remove_dir(&path) {
                    warn!("s3_sync: failed to remove empty dir {:?}: {}", path, e);
                }
            }
        } else if let Ok(rel) = path.strip_prefix(base_dir) {
            let rel_str = rel.to_string_lossy();
            if !known_keys.contains(rel_str.as_ref()) {
                if let Err(e) = std::fs::remove_file(&path) {
                    warn!("s3_sync: failed to remove stale file {:?}: {}", path, e);
                } else {
                    info!("s3_sync: removed stale file {:?}", path);
                }
            }
        }
    }
}
