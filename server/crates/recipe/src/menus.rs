use std::path::Path;

use chrono::Local;
use rocket::delete;
use rocket::figment::Figment;
use rocket::get;
use rocket::post;
use rocket::put;
use rocket::routes;
use rocket::serde::json::Json;
use rocket::{Build, Rocket};

use crate::ingredient;
use crate::markdown;
use crate::models::{
    ApiResponse, CreateMenuRequest, CustomDish, MenuFrontmatter, MenuRecipe, MenuState,
    MenuSummary, RecipeMeta, SetCurrentRequest, ToggleRequest, UpdateMenuRequest,
};
use crate::RECIPE_INDEX;

const SAVED_DIR: &str = "saved";

fn saved_dir() -> std::path::PathBuf {
    Path::new(utils::get_data_path())
        .join("recipes")
        .join(SAVED_DIR)
}

fn set_current_menu(db: &sled::Db, filename: &str) {
    let _ = db.insert(b"recipe/current_menu", filename.as_bytes());
}

fn get_current_menu(db: &sled::Db) -> Option<String> {
    db.get(b"recipe/current_menu")
        .ok()
        .flatten()
        .and_then(|v| String::from_utf8(v.to_vec()).ok())
}

fn build_menu_state(filename: &str, fm: &MenuFrontmatter) -> MenuState {
    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    let recipes: Vec<RecipeMeta> = fm
        .menu_recipes
        .iter()
        .filter_map(|mr| index.get(&mr.id).cloned())
        .collect();
    MenuState {
        filename: filename.to_string(),
        name: fm.name.clone(),
        date: fm.date.clone(),
        diners: fm.diners,
        menu_recipes: fm.menu_recipes.clone(),
        ingredients: fm.ingredients.clone(),
        recipes,
        custom_dishes: fm.custom_dishes.clone(),
    }
}

fn save_menu_file(filename: &str, fm: &MenuFrontmatter) {
    let filepath = saved_dir().join(filename);
    let content = markdown::generate_menu_frontmatter(fm);
    let _ = std::fs::write(&filepath, &content);
}

fn recompute_and_save(filename: &str, fm: &mut MenuFrontmatter) {
    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    let recipes: Vec<&RecipeMeta> = fm
        .menu_recipes
        .iter()
        .filter_map(|mr| index.get(&mr.id))
        .collect();
    fm.ingredients = ingredient::aggregate(&recipes, &fm.menu_recipes);
    save_menu_file(filename, fm);
}

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    let dir = saved_dir();
    tokio::fs::create_dir_all(&dir).await?;

    Ok(build.mount(
        base,
        routes![
            current_menu,
            set_current,
            create_menu,
            list_menus,
            get_menu,
            toggle_recipe,
            adjust_portion,
            update_menu,
            delete_menu,
            add_custom_dish,
            remove_custom_dish,
            adjust_custom_portion,
        ],
    ))
}

// ── GET /current ──

#[get("/current")]
async fn current_menu() -> Json<ApiResponse<MenuState>> {
    let db = utils::database::get_db();

    if let Some(filename) = get_current_menu(&db) {
        let filepath = saved_dir().join(&filename);
        if filepath.exists() {
            let raw = std::fs::read_to_string(&filepath).unwrap_or_default();
            let parts: Vec<&str> = raw.splitn(3, "---").collect();
            if parts.len() >= 3 && parts[0].trim().is_empty() {
                if let Ok(fm) = serde_yaml::from_str::<MenuFrontmatter>(parts[1]) {
                    return Json(ApiResponse::ok(build_menu_state(&filename, &fm)));
                }
            }
        }
    }

    // Find most recent
    let dir = saved_dir();
    if dir.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .into_iter()
            .flat_map(|d| d.flatten())
            .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
            .collect();
        entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
        if let Some(entry) = entries.first() {
            let filename = entry.file_name().to_string_lossy().to_string();
            set_current_menu(&db, &filename);
            let raw = std::fs::read_to_string(entry.path()).unwrap_or_default();
            let parts: Vec<&str> = raw.splitn(3, "---").collect();
            if parts.len() >= 3 && parts[0].trim().is_empty() {
                if let Ok(fm) = serde_yaml::from_str::<MenuFrontmatter>(parts[1]) {
                    return Json(ApiResponse::ok(build_menu_state(&filename, &fm)));
                }
            }
        }
    }

    Json(ApiResponse::ok(MenuState {
        filename: String::new(),
        name: String::new(),
        date: String::new(),
        diners: 0,
        menu_recipes: vec![],
        ingredients: vec![],
        recipes: vec![],
        custom_dishes: vec![],
    }))
}

// ── POST /current ──

#[post("/current", data = "<req>")]
async fn set_current(req: Json<SetCurrentRequest>) -> Json<ApiResponse<String>> {
    let filepath = saved_dir().join(&req.filename);
    if !filepath.exists() {
        return Json(ApiResponse::err("Menu not found"));
    }
    let db = utils::database::get_db();
    set_current_menu(&db, &req.filename);
    Json(ApiResponse::ok(format!("Switched to {}", req.filename)))
}

// ── POST /menus ──

#[post("/menus", data = "<req>")]
async fn create_menu(req: Json<CreateMenuRequest>) -> Json<ApiResponse<MenuState>> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let filename = format!("{}-{}.md", today, req.name);

    let dir = saved_dir();
    tokio::fs::create_dir_all(&dir).await.ok();

    let filepath = dir.join(&filename);
    if filepath.exists() {
        return Json(ApiResponse::err(format!("Menu '{}' already exists for today", req.name)));
    }

    let fm = MenuFrontmatter {
        date: today.clone(),
        name: req.name.clone(),
        diners: req.diners,
        menu_recipes: vec![],
        ingredients: vec![],
        custom_dishes: vec![],
    };

    save_menu_file(&filename, &fm);

    let db = utils::database::get_db();
    set_current_menu(&db, &filename);

    Json(ApiResponse::ok(MenuState {
        filename: filename.clone(),
        name: req.name.clone(),
        date: today,
        diners: req.diners,
        menu_recipes: vec![],
        ingredients: vec![],
        recipes: vec![],
        custom_dishes: vec![],
    }))
}

// ── GET /menus ──

#[get("/menus")]
async fn list_menus() -> Json<ApiResponse<Vec<MenuSummary>>> {
    let dir = saved_dir();
    let mut menus = Vec::new();
    if !dir.exists() {
        return Json(ApiResponse::ok(menus));
    }

    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .into_iter()
        .flat_map(|d| d.flatten())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    for entry in entries {
        let raw = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let parts: Vec<&str> = raw.splitn(3, "---").collect();
        if parts.len() < 3 || !parts[0].trim().is_empty() {
            continue;
        }
        if let Ok(fm) = serde_yaml::from_str::<MenuFrontmatter>(parts[1]) {
            menus.push(MenuSummary {
                filename: entry.file_name().to_string_lossy().to_string(),
                name: fm.name,
                date: fm.date,
                diners: fm.diners,
                dish_count: fm.menu_recipes.len() + fm.custom_dishes.len(),
            });
        }
    }

    Json(ApiResponse::ok(menus))
}

// ── GET /menus/<filename> ──

#[get("/menus/<filename>")]
async fn get_menu(filename: &str) -> Json<ApiResponse<MenuState>> {
    let filepath = saved_dir().join(filename);
    let raw = match std::fs::read_to_string(&filepath) {
        Ok(r) => r,
        Err(_) => return Json(ApiResponse::err("Menu not found")),
    };
    let parts: Vec<&str> = raw.splitn(3, "---").collect();
    if parts.len() < 3 || !parts[0].trim().is_empty() {
        return Json(ApiResponse::err("Invalid menu format"));
    }
    let fm: MenuFrontmatter = match serde_yaml::from_str(parts[1]) {
        Ok(f) => f,
        Err(e) => return Json(ApiResponse::err(format!("Parse error: {}", e))),
    };
    Json(ApiResponse::ok(build_menu_state(filename, &fm)))
}

// ── POST /menus/<filename>/toggle ──

#[post("/menus/<filename>/toggle", data = "<req>")]
async fn toggle_recipe(filename: &str, req: Json<ToggleRequest>) -> Json<ApiResponse<MenuState>> {
    let filepath = saved_dir().join(filename);
    if !filepath.exists() {
        return Json(ApiResponse::err("Menu not found"));
    }

    let raw = std::fs::read_to_string(&filepath).unwrap_or_default();
    let parts: Vec<&str> = raw.splitn(3, "---").collect();
    if parts.len() < 3 || !parts[0].trim().is_empty() {
        return Json(ApiResponse::err("Invalid menu format"));
    }
    let mut fm: MenuFrontmatter = match serde_yaml::from_str(parts[1]) {
        Ok(f) => f,
        Err(e) => return Json(ApiResponse::err(format!("Parse error: {}", e))),
    };

    // Toggle: add with default portions = recipe servings, or remove
    if let Some(pos) = fm.menu_recipes.iter().position(|mr| mr.id == req.recipe_id) {
        fm.menu_recipes.remove(pos);
    } else {
        let index = RECIPE_INDEX.get().unwrap().read().unwrap();
        let portions = index
            .get(&req.recipe_id)
            .map(|r| r.servings as f64)
            .unwrap_or(1.0);
        fm.menu_recipes.push(MenuRecipe {
            id: req.recipe_id.clone(),
            portions,
        });
    }

    recompute_and_save(filename, &mut fm);
    Json(ApiResponse::ok(build_menu_state(filename, &fm)))
}

// ── POST /menus/<filename>/portion ──

#[derive(Debug, serde::Deserialize)]
struct PortionRequest {
    recipe_id: String,
    portions: f64,
}

#[post("/menus/<filename>/portion", data = "<req>")]
async fn adjust_portion(filename: &str, req: Json<PortionRequest>) -> Json<ApiResponse<MenuState>> {
    let filepath = saved_dir().join(filename);
    if !filepath.exists() {
        return Json(ApiResponse::err("Menu not found"));
    }

    let raw = std::fs::read_to_string(&filepath).unwrap_or_default();
    let parts: Vec<&str> = raw.splitn(3, "---").collect();
    if parts.len() < 3 || !parts[0].trim().is_empty() {
        return Json(ApiResponse::err("Invalid menu format"));
    }
    let mut fm: MenuFrontmatter = match serde_yaml::from_str(parts[1]) {
        Ok(f) => f,
        Err(e) => return Json(ApiResponse::err(format!("Parse error: {}", e))),
    };

    // Check if recipe is adjustable
    let index = RECIPE_INDEX.get().unwrap().read().unwrap();
    if let Some(recipe) = index.get(&req.recipe_id) {
        if !recipe.adjustable {
            return Json(ApiResponse::err("This recipe's portion cannot be adjusted"));
        }
    }

    // Clamp portions to valid range (0.5, 1.0, 1.5, 2.0, ...)
    let clamped = (req.portions * 2.0).round() / 2.0;
    let clamped = clamped.max(0.5);

    if let Some(mr) = fm.menu_recipes.iter_mut().find(|mr| mr.id == req.recipe_id) {
        mr.portions = clamped;
    }

    recompute_and_save(filename, &mut fm);
    Json(ApiResponse::ok(build_menu_state(filename, &fm)))
}

// ── PUT /menus/<filename> ──

#[put("/menus/<filename>", data = "<req>")]
async fn update_menu(filename: &str, req: Json<UpdateMenuRequest>) -> Json<ApiResponse<MenuState>> {
    let filepath = saved_dir().join(filename);
    if !filepath.exists() {
        return Json(ApiResponse::err("Menu not found"));
    }

    let raw = std::fs::read_to_string(&filepath).unwrap_or_default();
    let parts: Vec<&str> = raw.splitn(3, "---").collect();
    if parts.len() < 3 || !parts[0].trim().is_empty() {
        return Json(ApiResponse::err("Invalid menu format"));
    }
    let mut fm: MenuFrontmatter = match serde_yaml::from_str(parts[1]) {
        Ok(f) => f,
        Err(e) => return Json(ApiResponse::err(format!("Parse error: {}", e))),
    };

    if let Some(diners) = req.diners {
        fm.diners = diners;
    }
    if let Some(menu_recipes) = &req.menu_recipes {
        fm.menu_recipes = menu_recipes.clone();
    }

    recompute_and_save(filename, &mut fm);
    Json(ApiResponse::ok(build_menu_state(filename, &fm)))
}

// ── DELETE /menus/<filename> ──

#[delete("/menus/<filename>")]
async fn delete_menu(filename: &str) -> Json<ApiResponse<String>> {
    let filepath = saved_dir().join(filename);
    if let Err(e) = std::fs::remove_file(&filepath) {
        return Json(ApiResponse::err(format!("Failed to delete: {}", e)));
    }
    Json(ApiResponse::ok(format!("Deleted {}", filename)))
}

// ── Custom dish helpers ──

fn read_menu_frontmatter(filename: &str) -> Result<MenuFrontmatter, String> {
    let filepath = saved_dir().join(filename);
    let raw =
        std::fs::read_to_string(&filepath).map_err(|_| "Menu not found".to_string())?;
    let parts: Vec<&str> = raw.splitn(3, "---").collect();
    if parts.len() < 3 || !parts[0].trim().is_empty() {
        return Err("Invalid menu format".to_string());
    }
    serde_yaml::from_str::<MenuFrontmatter>(parts[1])
        .map_err(|e| format!("Parse error: {}", e))
}

// ── POST /menus/<filename>/custom ──

#[derive(Debug, serde::Deserialize)]
struct CustomDishRequest {
    name: String,
}

#[post("/menus/<filename>/custom", data = "<req>")]
async fn add_custom_dish(
    filename: &str,
    req: Json<CustomDishRequest>,
) -> Json<ApiResponse<MenuState>> {
    let mut fm = match read_menu_frontmatter(filename) {
        Ok(f) => f,
        Err(e) => return Json(ApiResponse::err(e)),
    };

    fm.custom_dishes.push(CustomDish {
        name: req.name.clone(),
        portions: 1.0,
    });
    save_menu_file(filename, &fm);
    Json(ApiResponse::ok(build_menu_state(filename, &fm)))
}

// ── DELETE /menus/<filename>/custom/<index> ──

#[delete("/menus/<filename>/custom/<index>")]
async fn remove_custom_dish(
    filename: &str,
    index: usize,
) -> Json<ApiResponse<MenuState>> {
    let mut fm = match read_menu_frontmatter(filename) {
        Ok(f) => f,
        Err(e) => return Json(ApiResponse::err(e)),
    };

    if index >= fm.custom_dishes.len() {
        return Json(ApiResponse::err("Custom dish index out of range"));
    }
    fm.custom_dishes.remove(index);
    save_menu_file(filename, &fm);
    Json(ApiResponse::ok(build_menu_state(filename, &fm)))
}

// ── POST /menus/<filename>/custom/<index>/portion ──

#[derive(Debug, serde::Deserialize)]
struct CustomPortionRequest {
    portions: f64,
}

#[post("/menus/<filename>/custom/<index>/portion", data = "<req>")]
async fn adjust_custom_portion(
    filename: &str,
    index: usize,
    req: Json<CustomPortionRequest>,
) -> Json<ApiResponse<MenuState>> {
    let mut fm = match read_menu_frontmatter(filename) {
        Ok(f) => f,
        Err(e) => return Json(ApiResponse::err(e)),
    };

    if index >= fm.custom_dishes.len() {
        return Json(ApiResponse::err("Custom dish index out of range"));
    }

    let clamped = (req.portions * 2.0).round() / 2.0;
    fm.custom_dishes[index].portions = clamped.max(0.5);
    save_menu_file(filename, &fm);
    Json(ApiResponse::ok(build_menu_state(filename, &fm)))
}
