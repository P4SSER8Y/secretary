mod indexer;
mod ingredient;
mod markdown;
mod menus;
pub mod models;
mod recipes;
pub mod sync;

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use rocket::figment::Figment;
use rocket::get;
use rocket::http::ContentType;
use rocket::routes;
use rocket::{Build, Rocket};

use crate::models::RecipeMeta;

/// Global in-memory recipe index
pub static RECIPE_INDEX: std::sync::OnceLock<RwLock<HashMap<String, RecipeMeta>>> =
    std::sync::OnceLock::new();

/// S3 sync config — initialized at startup if `[recipe.s3]` is configured.
pub static S3_CONFIG: std::sync::OnceLock<Option<sync::S3Config>> =
    std::sync::OnceLock::new();

/// Tracks the last sync timestamp for auto-refresh throttling.
pub static SYNC_STATE: std::sync::OnceLock<RwLock<sync::SyncState>> =
    std::sync::OnceLock::new();

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    // Ensure data directories exist
    let data_path = utils::get_data_path();
    let recipes_dir = Path::new(data_path).join("recipes");
    let raw_dir = recipes_dir.join("raw");
    let saved_dir = recipes_dir.join("saved");
    tokio::fs::create_dir_all(&raw_dir).await?;
    tokio::fs::create_dir_all(&saved_dir).await?;

    // Build the recipe index from raw/ at startup
    let index = indexer::scan_recipes(&raw_dir).unwrap_or_default();
    RECIPE_INDEX.get_or_init(|| RwLock::new(index));

    // Read optional S3 sync config
    let s3_config: Option<sync::S3Config> = config
        .extract_inner("recipe.s3")
        .unwrap_or(None);
    if s3_config.as_ref().map_or(false, |c| c.enabled) {
        SYNC_STATE.get_or_init(|| RwLock::new(sync::SyncState::new()));
        log::info!("S3 sync enabled: bucket '{}'",
            s3_config.as_ref().unwrap().bucket);
    }
    S3_CONFIG.get_or_init(|| s3_config);

    // Mount routes
    let build = recipes::build(base, build, config).await?;
    let build = menus::build(base, build, config).await?;
    let build = build.mount("/recipe", routes![skill_md]);
    Ok(build)
}

/// Serve the recipe authoring guide at /recipe/SKILL.md
#[get("/SKILL.md")]
fn skill_md() -> (ContentType, &'static str) {
    (ContentType::Plain, include_str!("../SKILL.md"))
}
