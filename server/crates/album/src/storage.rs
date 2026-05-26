use std::path::Path;

use crate::config::AlbumConfig;
use crate::converter::{ConversionParams, ConvertedImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub id: String,
    pub original_filename: String,
    pub mime_type: String,
    pub created_at: String,
    pub params: ConversionParams,
}

fn db() -> sled::Db {
    utils::database::get_db()
}

pub fn storage_dir(cfg: &AlbumConfig) -> std::path::PathBuf {
    Path::new(utils::get_data_path()).join(&cfg.storage_path)
}

pub async fn init(cfg: &AlbumConfig) -> anyhow::Result<()> {
    let dir = storage_dir(cfg);
    tokio::fs::create_dir_all(&dir).await?;
    Ok(())
}

pub fn save_image(
    cfg: &AlbumConfig,
    id: &str,
    converted: &ConvertedImage,
    meta: &ImageMetadata,
) -> anyhow::Result<()> {
    let dir = storage_dir(cfg).join(id);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("preview.png"), &converted.clean_png)?;
    std::fs::write(dir.join("raw.bin"), &converted.raw_4bpp)?;

    let key = format!("album/{}", id);
    let value = serde_json::to_vec(meta)?;
    db().insert(key.as_bytes(), value)?;

    Ok(())
}

pub fn load_preview(cfg: &AlbumConfig, id: &str) -> anyhow::Result<Vec<u8>> {
    let path = storage_dir(cfg).join(id).join("preview.png");
    std::fs::read(&path).map_err(|e| anyhow::anyhow!("preview not found for {}: {}", id, e))
}

pub fn load_raw_binary(cfg: &AlbumConfig, id: &str) -> anyhow::Result<Vec<u8>> {
    let path = storage_dir(cfg).join(id).join("raw.bin");
    std::fs::read(&path).map_err(|e| anyhow::anyhow!("raw binary not found for {}: {}", id, e))
}

pub fn list_images() -> Vec<ImageMetadata> {
    let mut images: Vec<ImageMetadata> = db()
        .scan_prefix("album/")
        .filter_map(|item| item.ok())
        .filter_map(|(key, value)| {
            let key_str = String::from_utf8_lossy(&key);
            if key_str == "album/current" {
                return None;
            }
            serde_json::from_slice::<ImageMetadata>(&value).ok()
        })
        .collect();
    images.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    images
}

pub fn delete_image(cfg: &AlbumConfig, id: &str) -> anyhow::Result<()> {
    let d = db();
    d.remove(format!("album/{}", id).as_bytes())?;

    // If this was the current image, clear it
    if let Some(current) = get_current()? {
        if current == id {
            clear_current()?;
        }
    }

    let dir = storage_dir(cfg).join(id);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

pub fn get_metadata(id: &str) -> anyhow::Result<ImageMetadata> {
    let key = format!("album/{}", id);
    match db().get(key.as_bytes())? {
        Some(value) => Ok(serde_json::from_slice(&value)?),
        None => Err(anyhow::anyhow!("image {} not found", id)),
    }
}

// ---- Current image tracking ----

const CURRENT_KEY: &[u8] = b"album/current";

pub fn set_current(id: &str) -> anyhow::Result<()> {
    db().insert(CURRENT_KEY, id.as_bytes())?;
    Ok(())
}

pub fn get_current() -> anyhow::Result<Option<String>> {
    match db().get(CURRENT_KEY)? {
        Some(v) => Ok(Some(String::from_utf8_lossy(&v).to_string())),
        None => Ok(None),
    }
}

pub fn clear_current() -> anyhow::Result<()> {
    db().remove(CURRENT_KEY)?;
    Ok(())
}
