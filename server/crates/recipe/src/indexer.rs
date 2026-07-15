use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use log::{info, warn};

use crate::markdown;
use crate::models::RecipeMeta;

/// Scan a directory for recipe markdown files, parse them, and return a HashMap index.
pub fn scan_recipes(dir: &Path) -> anyhow::Result<HashMap<String, RecipeMeta>> {
    let mut index = HashMap::new();

    if !dir.exists() {
        return Ok(index);
    }

    let entries = std::fs::read_dir(dir).context("Failed to read recipe directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" {
            continue;
        }

        let raw = std::fs::read_to_string(&path).context("Failed to read recipe file")?;
        match markdown::parse_recipe(&raw) {
            Ok(meta) => {
                let cover = find_cover_image(dir, &meta.id);
                let mut meta = meta;
                meta.cover_image = cover;
                info!("Loaded recipe: {} ({})", meta.name, meta.id);
                index.insert(meta.id.clone(), meta);
            }
            Err(e) => {
                warn!("Failed to parse recipe {:?}: {}", path, e);
            }
        }
    }

    info!("Indexed {} recipes", index.len());
    Ok(index)
}

/// Look for a cover image for a recipe: `<id>.jpg`, `<id>.png`, `<id>.webp`,
/// or `<id>/cover.jpg|png|webp`.
fn find_cover_image(dir: &Path, id: &str) -> Option<String> {
    for ext in &["jpg", "png", "webp", "jpeg"] {
        let candidate = dir.join(format!("{}.{}", id, ext));
        if candidate.exists() {
            return Some(format!("{}.{}", id, ext));
        }
        let candidate = dir.join(id).join(format!("cover.{}", ext));
        if candidate.exists() {
            return Some(format!("{}/cover.{}", id, ext));
        }
    }
    None
}
