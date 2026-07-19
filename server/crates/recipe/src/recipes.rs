use std::path::{Path, PathBuf};

use rocket::data::{Data, ToByteUnit};
use rocket::figment::Figment;
use rocket::get;
use rocket::http::ContentType;
use rocket::post;
use rocket::put;
use rocket::routes;
use rocket::serde::json::Json;
use rocket::{Build, Rocket};

use crate::indexer;
use crate::markdown;
use crate::models::{ApiResponse, RecipeMeta, RecipeSummary};
use crate::sync;
use crate::RECIPE_INDEX;

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    Ok(build.mount(base, routes![
        list_recipes,
        get_recipe,
        get_image,
        reload_recipes,
        sync_recipes,
        get_raw_recipe,
        put_raw_recipe,
        upload_cover_image,
    ]))
}

/// GET /recipes?category=<cat>
#[get("/recipes?<category>")]
async fn list_recipes(category: Option<&str>) -> Json<ApiResponse<Vec<RecipeSummary>>> {
    sync::maybe_auto_sync().await;

    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    let summaries: Vec<RecipeSummary> = index
        .values()
        .filter(|r| {
            category
                .map(|c| r.category == c)
                .unwrap_or(true)
        })
        .map(|r| RecipeSummary {
            name: r.name.clone(),
            category: r.category.clone(),
            cover_image: r.cover_image.clone(),
            cook_time: r.cook_time.clone(),
            difficulty: r.difficulty.clone(),
            tags: r.tags.clone(),
            servings: r.servings,
            adjustable: r.adjustable,
        })
        .collect();
    Json(ApiResponse::ok(summaries))
}

/// GET /recipes/<name>
#[get("/recipes/<name>")]
async fn get_recipe(name: &str) -> Json<ApiResponse<RecipeMeta>> {
    sync::maybe_auto_sync().await;

    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    match index.get(name) {
        Some(meta) => Json(ApiResponse::ok(meta.clone())),
        None => Json(ApiResponse::err(format!("Recipe not found: {}", name))),
    }
}

/// GET /image/<name> — serve recipe cover image
#[get("/image/<name>")]
async fn get_image(name: &str) -> Option<(ContentType, Vec<u8>)> {
    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    let cover = index.get(name)?.cover_image.as_ref()?;

    let data_path = utils::get_data_path();
    let raw_dir = Path::new(data_path).join("recipes").join("raw");

    // S3 sync preserves folder structure under raw/,
    // so cover may be "filename.jpg" or "assets/filename.jpg"
    let img_path = if cover.contains('/') {
        raw_dir.join(cover)
    } else {
        // Check root level first, then assets/ (both are valid locations)
        let root_path = raw_dir.join(cover);
        if root_path.exists() {
            root_path
        } else {
            raw_dir.join("assets").join(cover)
        }
    };

    let data = std::fs::read(&img_path).ok()?;
    let ct = ContentType::from_extension(
        img_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg"),
    )
    .unwrap_or(ContentType::JPEG);
    Some((ct, data))
}

/// POST /recipes/reload — hot-reload recipe index
#[post("/recipes/reload")]
async fn reload_recipes() -> Json<ApiResponse<String>> {
    let data_path = utils::get_data_path();
    let raw_dir = Path::new(data_path).join("recipes").join("raw");
    match indexer::scan_recipes(&raw_dir) {
        Ok(new_index) => {
            let count = new_index.len();
            let mut index = RECIPE_INDEX.get().unwrap().write().unwrap();
            *index = new_index;
            Json(ApiResponse::ok(format!("Reloaded {} recipes", count)))
        }
        Err(e) => Json(ApiResponse::err(format!("Reload failed: {}", e))),
    }
}

/// POST /recipes/sync — pull recipes from S3 and rebuild index
#[post("/recipes/sync")]
async fn sync_recipes() -> Json<ApiResponse<String>> {
    match sync::manual_sync_and_reload().await {
        Ok(count) => Json(ApiResponse::ok(format!("Synced {} files from S3", count))),
        Err(e) => Json(ApiResponse::err(format!("Sync failed: {}", e))),
    }
}

/// GET /recipes/<name>/raw — return raw markdown for editing
#[get("/recipes/<name>/raw")]
async fn get_raw_recipe(name: &str) -> Option<(ContentType, String)> {
    let data_path = utils::get_data_path();
    let raw_dir = Path::new(data_path).join("recipes").join("raw");

    // Try filename first: {name}.md
    let name_file = raw_dir.join(format!("{}.md", name));
    if name_file.exists() {
        let raw = std::fs::read_to_string(&name_file).ok()?;
        return Some((ContentType::Plain, raw));
    }

    // Full recursive scan — parse frontmatter of each .md file
    let found = find_recipe_file(&raw_dir, name)?;
    let raw = std::fs::read_to_string(&found).ok()?;
    Some((ContentType::Plain, raw))
}

/// Walk directory tree to find a .md file whose frontmatter `name` matches.
fn find_recipe_file(dir: &Path, target_name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_recipe_file(&path, target_name) {
                return Some(found);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if let Ok(meta) = markdown::parse_recipe(&raw) {
                    if meta.name == target_name {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

/// Push data to S3, then sync back to keep local cache consistent.
async fn save_and_sync(key: &str, data: &[u8]) -> Result<(), String> {
    sync::push_to_s3(key, data)
        .await
        .map_err(|e| format!("S3 upload failed: {}", e))?;
    sync::manual_sync_and_reload()
        .await
        .map_err(|e| format!("Sync after save failed: {}", e))?;
    Ok(())
}

/// PUT /recipes/<name>/raw — save recipe (push S3 → sync back → rebuild index)
#[put("/recipes/<name>/raw", data = "<body>")]
async fn put_raw_recipe(
    name: &str,
    body: Data<'_>,
) -> Json<ApiResponse<RecipeMeta>> {
    let content = match body.open(1_i32.mebibytes()).into_string().await {
        Ok(s) => s.into_inner(),
        Err(e) => return Json(ApiResponse::err(format!("Failed to read body: {}", e))),
    };

    // Parse to validate frontmatter and extract canonical name
    let mut meta = match markdown::parse_recipe(&content) {
        Ok(m) => m,
        Err(e) => return Json(ApiResponse::err(format!("Invalid recipe: {}", e))),
    };

    // Use frontmatter name if present, otherwise fall back to URL name
    let canonical_name = if !meta.name.is_empty() {
        meta.name.clone()
    } else {
        meta.name = name.to_string();
        meta.name.clone()
    };

    let key = format!("{}.md", canonical_name);

    if let Err(e) = save_and_sync(&key, content.as_bytes()).await {
        return Json(ApiResponse::err(e));
    }

    // Re-read the parsed recipe from the refreshed index
    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    match index.get(&canonical_name) {
        Some(recipe) => Json(ApiResponse::ok(recipe.clone())),
        None => Json(ApiResponse::err(format!(
            "Recipe '{}' saved but not found after sync — check file syntax",
            canonical_name
        ))),
    }
}

/// POST /recipes/<name>/image — upload cover image (push S3 → sync back → rebuild index)
#[post("/recipes/<name>/image", data = "<data>")]
async fn upload_cover_image(
    name: &str,
    ct: &ContentType,
    data: Data<'_>,
) -> Json<ApiResponse<String>> {
    let ext = match ct.extension() {
        Some(e) if matches!(e.as_str(), "jpg" | "jpeg" | "png" | "webp") => e.as_str(),
        _ => return Json(ApiResponse::err("Unsupported image type (use jpg/png/webp)")),
    };

    let bytes = match data.open(10_i32.mebibytes()).into_bytes().await {
        Ok(b) => b.into_inner(),
        Err(e) => return Json(ApiResponse::err(format!("Failed to read image: {}", e))),
    };

    let data_path = utils::get_data_path();
    let raw_dir = Path::new(data_path).join("recipes").join("raw");
    let key = format!("{}.{}", name, ext);

    // Delete old cover images for this recipe (any extension)
    for old_ext in &["jpg", "jpeg", "png", "webp"] {
        let old_path = raw_dir.join(format!("{}.{}", name, old_ext));
        if old_path.exists() {
            let _ = std::fs::remove_file(&old_path);
        }
    }

    if let Err(e) = save_and_sync(&key, &bytes).await {
        return Json(ApiResponse::err(e));
    }

    Json(ApiResponse::ok(key))
}
