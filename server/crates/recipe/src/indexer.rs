use std::collections::HashMap;
use std::path::Path;

use log::{info, warn};

use crate::markdown;
use crate::models::RecipeMeta;

/// Scan a directory recursively for recipe markdown files, parse them, and return a HashMap index.
pub fn scan_recipes(dir: &Path) -> anyhow::Result<HashMap<String, RecipeMeta>> {
    let mut index = HashMap::new();
    if !dir.exists() {
        return Ok(index);
    }
    scan_recipes_recursive(dir, dir, &mut index);
    info!("Indexed {} recipes", index.len());
    Ok(index)
}

fn scan_recipes_recursive(root: &Path, current: &Path, index: &mut HashMap<String, RecipeMeta>) {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read dir {:?}: {}", current, e);
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_recipes_recursive(root, &path, index);
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" {
            continue;
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to read {:?}: {}", path, e);
                continue;
            }
        };
        match markdown::parse_recipe(&raw) {
            Ok(mut meta) => {
                if meta.cover_image.is_none() {
                    meta.cover_image = find_cover_image_recursive(root, &meta.id);
                }
                info!("Loaded recipe: {} ({})", meta.name, meta.id);
                index.insert(meta.id.clone(), meta);
            }
            Err(e) => {
                warn!("Failed to parse recipe {:?}: {}", path, e);
            }
        }
    }
}

/// Look for a cover image for a recipe anywhere under root.
fn find_cover_image_recursive(root: &Path, id: &str) -> Option<String> {
    for ext in &["jpg", "png", "webp", "jpeg"] {
        // Check root level
        let candidate = root.join(format!("{}.{}", id, ext));
        if candidate.exists() {
            return Some(format!("{}.{}", id, ext));
        }
        // Check {id}/cover.ext
        let candidate = root.join(id).join(format!("cover.{}", ext));
        if candidate.exists() {
            return Some(format!("{}/cover.{}", id, ext));
        }
        // Check assets/{id}.ext (Obsidian subdirectory pattern)
        let candidate = root.join("assets").join(format!("{}.{}", id, ext));
        if candidate.exists() {
            return Some(format!("assets/{}.{}", id, ext));
        }
    }
    None
}
