use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use utils::database::Db;

const CREDENTIAL_PREFIX: &str = "gate/credential/";
const META_KEY: &str = "gate/meta";

fn invites_dir() -> PathBuf {
    PathBuf::from(utils::get_data_path()).join("gate/invites")
}

// ── Invite (filesystem-based — allows CLI + server concurrent access) ─

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Invite {
    pub family: String,
    pub created_at: i64,
    pub expires_at: i64,
}

pub fn create_invite(code_hash: &str, family: &str, ttl: u64) -> Result<()> {
    let dir = invites_dir();
    std::fs::create_dir_all(&dir)?;
    let now = chrono::Utc::now().timestamp();
    let invite = Invite {
        family: family.to_string(),
        created_at: now,
        expires_at: if ttl == 0 { i64::MAX } else { now + ttl as i64 },
    };
    let path = dir.join(format!("{}.json", code_hash));
    let json = serde_json::to_string_pretty(&invite)?;
    std::fs::write(&path, json)?;
    log::info!("Invite created: {}", path.display());
    Ok(())
}

pub fn get_invite(code_hash: &str) -> Result<Option<Invite>> {
    let path = invites_dir().join(format!("{}.json", code_hash));
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path)?;
    let invite: Invite = serde_json::from_str(&json)?;
    Ok(Some(invite))
}

pub fn delete_invite(code_hash: &str) -> Result<()> {
    let path = invites_dir().join(format!("{}.json", code_hash));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// Remove all expired invite files. Returns count of removed files.
pub fn cleanup_expired_invites() -> Result<usize> {
    let dir = invites_dir();
    if !dir.exists() {
        return Ok(0);
    }
    let now = chrono::Utc::now().timestamp();
    let mut removed = 0;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(invite) = serde_json::from_str::<Invite>(&json) {
                    if now > invite.expires_at {
                        std::fs::remove_file(&path)?;
                        removed += 1;
                    }
                } else {
                    // corrupt file — remove it
                    std::fs::remove_file(&path)?;
                    removed += 1;
                }
            }
        }
    }
    if removed > 0 {
        log::info!("Cleaned up {} expired invite(s)", removed);
    }
    Ok(removed)
}

/// Whether any pending (non-expired) invite exists — used by the frontend to
/// decide whether to show a registration entry.
pub fn has_invites() -> Result<bool> {
    let dir = invites_dir();
    if !dir.exists() {
        return Ok(false);
    }
    let now = chrono::Utc::now().timestamp();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(invite) = serde_json::from_str::<Invite>(&json) {
                    if now <= invite.expires_at {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}

// ── Credential ──────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredCredential {
    pub name: String,
    pub family: String,
    pub credential_id: String,
    pub passkey_bytes: Vec<u8>, // serde_json-serialized webauthn_rs::Passkey
    pub origin: String,
    pub enabled: bool,
    pub created_at: String,
}

pub fn store_credential(db: &Db, cred: &StoredCredential) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let key = format!("{}{}", CREDENTIAL_PREFIX, id);
    db.set(&key, cred)
}

pub fn get_credential(db: &Db, id: &str) -> Result<Option<StoredCredential>> {
    let key = format!("{}{}", CREDENTIAL_PREFIX, id);
    db.get(&key)
}

/// Check if a credential already exists for the given name + family
pub fn credential_exists(db: &Db, name: &str, family: &str) -> Result<bool> {
    let all = list_credentials(db, Some(family))?;
    Ok(all.iter().any(|(_, c)| c.name == name && c.family == family))
}

pub fn list_credentials(
    db: &Db,
    family_filter: Option<&str>,
) -> Result<Vec<(String, StoredCredential)>> {
    let mut results = Vec::new();
    for item in db.scan_prefix(CREDENTIAL_PREFIX) {
        let item = item?;
        let key = item.0;
        let value = item.1;
        let key_str = String::from_utf8_lossy(&key).to_string();
        // Envelope is bincode (Db::set); only passkey_bytes inside is serde_json.
        // Skip stale/foreign entries that fail to parse.
        let cred: StoredCredential = match bincode::deserialize(&value) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(f) = family_filter {
            if cred.family != f {
                continue;
            }
        }
        let id = key_str.strip_prefix(CREDENTIAL_PREFIX).unwrap_or(&key_str).to_string();
        results.push((id, cred));
    }
    Ok(results)
}

pub fn disable_credential(db: &Db, id: &str) -> Result<()> {
    let key = format!("{}{}", CREDENTIAL_PREFIX, id);
    if let Some(mut cred) = db.get::<StoredCredential>(&key)? {
        cred.enabled = false;
        db.set(&key, &cred)?;
    }
    Ok(())
}

pub fn delete_credential(db: &Db, id: &str) -> Result<()> {
    let key = format!("{}{}", CREDENTIAL_PREFIX, id);
    db.delete(&key)
}

// ── Meta ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GateMeta {
    pub rp_name: String,
    pub rp_id: String,
    pub origin: String,
}

pub fn set_meta(db: &Db, meta: &GateMeta) -> Result<()> {
    db.set(META_KEY, meta)
}

pub fn get_meta(db: &Db) -> Result<Option<GateMeta>> {
    db.get(META_KEY)
}

// ── Stats ────────────────────────────────────────────────

pub fn stats(db: &Db) -> Result<(usize, usize, Vec<String>)> {
    let creds = list_credentials(db, None)?;
    let total = creds.len();
    let enabled = creds.iter().filter(|(_, c)| c.enabled).count();
    let mut families: Vec<String> = creds
        .iter()
        .map(|(_, c)| c.family.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    families.sort();
    Ok((total, enabled, families))
}
