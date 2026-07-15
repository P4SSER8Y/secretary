mod indexer;
mod ingredient;
mod markdown;
mod menus;
pub mod models;
mod recipes;

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use rocket::figment::Figment;
use rocket::{Build, Rocket};

use crate::models::RecipeMeta;

/// Global in-memory recipe index
pub static RECIPE_INDEX: std::sync::OnceLock<RwLock<HashMap<String, RecipeMeta>>> =
    std::sync::OnceLock::new();

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    // Ensure data directories exist
    let data_path = utils::get_data_path();
    let recipes_dir = Path::new(data_path).join("recipes");
    let saved_dir = recipes_dir.join("saved");
    tokio::fs::create_dir_all(&recipes_dir).await?;
    tokio::fs::create_dir_all(&saved_dir).await?;

    // Build the recipe index at startup
    let index = indexer::scan_recipes(&recipes_dir).unwrap_or_default();
    RECIPE_INDEX.get_or_init(|| RwLock::new(index));

    // Mount routes
    let build = recipes::build(base, build, _config).await?;
    let build = menus::build(base, build, _config).await?;
    Ok(build)
}
