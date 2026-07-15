use std::path::Path;

use rocket::figment::Figment;
use rocket::get;
use rocket::http::ContentType;
use rocket::post;
use rocket::routes;
use rocket::serde::json::Json;
use rocket::{Build, Rocket};

use crate::indexer;
use crate::models::{ApiResponse, RecipeMeta, RecipeSummary};
use crate::RECIPE_INDEX;

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    Ok(build.mount(base, routes![list_recipes, get_recipe, get_image, reload_recipes]))
}

/// GET /recipes?category=<cat>
#[get("/recipes?<category>")]
async fn list_recipes(category: Option<&str>) -> Json<ApiResponse<Vec<RecipeSummary>>> {
    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    let summaries: Vec<RecipeSummary> = index
        .values()
        .filter(|r| {
            category
                .map(|c| r.category == c)
                .unwrap_or(true)
        })
        .map(|r| RecipeSummary {
            id: r.id.clone(),
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

/// GET /recipes/<id>
#[get("/recipes/<id>")]
async fn get_recipe(id: &str) -> Json<ApiResponse<RecipeMeta>> {
    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    match index.get(id) {
        Some(meta) => Json(ApiResponse::ok(meta.clone())),
        None => Json(ApiResponse::err(format!("Recipe not found: {}", id))),
    }
}

/// GET /image/<id> — serve recipe cover image
#[get("/image/<id>")]
async fn get_image(id: &str) -> Option<(ContentType, Vec<u8>)> {
    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    let cover = index.get(id)?.cover_image.as_ref()?;

    let data_path = utils::get_data_path();
    let recipes_dir = Path::new(data_path).join("recipes");
    let img_path = recipes_dir.join(cover);

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
    let recipes_dir = Path::new(data_path).join("recipes");
    match indexer::scan_recipes(&recipes_dir) {
        Ok(new_index) => {
            let count = new_index.len();
            let mut index = RECIPE_INDEX.get().unwrap().write().unwrap();
            *index = new_index;
            Json(ApiResponse::ok(format!("Reloaded {} recipes", count)))
        }
        Err(e) => Json(ApiResponse::err(format!("Reload failed: {}", e))),
    }
}
