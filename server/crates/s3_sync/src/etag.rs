use std::collections::HashSet;

use log::info;

// ── ETag sled helpers ──
//
// sled key:   "{db_prefix}{s3_key}"
// sled value: etag string (raw bytes, UTF-8)

fn etag_key(db_prefix: &str, s3_key: &str) -> Vec<u8> {
    format!("{}{}", db_prefix, s3_key).into_bytes()
}

pub fn get_cached_etag(db_prefix: &str, s3_key: &str) -> Option<String> {
    let db = utils::database::get_db();
    db.get(etag_key(db_prefix, s3_key))
        .ok()
        .flatten()
        .and_then(|v| String::from_utf8(v.to_vec()).ok())
}

pub fn set_cached_etag(db_prefix: &str, s3_key: &str, etag: &str) {
    let db = utils::database::get_db();
    let _ = db.insert(etag_key(db_prefix, s3_key), etag.as_bytes());
}

pub fn remove_cached_etag(db_prefix: &str, s3_key: &str) {
    let db = utils::database::get_db();
    let _ = db.remove(etag_key(db_prefix, s3_key));
}

/// Delete all sled ETag entries under `db_prefix` whose s3_key is NOT in `known_keys`.
pub fn purge_stale_etags(db_prefix: &str, known_keys: &HashSet<String>) {
    let db = utils::database::get_db();
    let prefix = db_prefix;
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
        remove_cached_etag(db_prefix, key);
    }
    if !stale.is_empty() {
        info!("s3_sync: purged {} stale ETags for prefix '{}'", stale.len(), db_prefix);
    }
}
