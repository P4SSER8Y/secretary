use serde::{Deserialize, Serialize};

// ── Ingredient (from recipe frontmatter) ──

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Ingredient {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default)]
    pub optional: bool,
    /// Nested sub-ingredients for compound items (e.g. "浓盐葱姜水" → [盐, 葱, 姜, 水]).
    /// When present, the parent ingredient acts as a group header and its own
    /// amount/unit/hint are informational (the sub-ingredients carry the real quantities).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_ingredients: Option<Vec<Ingredient>>,
}

// ── ScaledIngredient (after diners/servings scaling) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaledIngredient {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub used_in: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_ingredients: Option<Vec<ScaledIngredient>>,
}

// ── Recipe (parsed from markdown) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeMeta {
    pub name: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
    pub prep_time: String,
    pub cook_time: String,
    pub servings: u32,
    pub difficulty: String,
    #[serde(default = "default_true")]
    pub adjustable: bool,
    #[serde(default)]
    pub ingredients: Vec<Ingredient>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub body: String,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSummary {
    pub name: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
    pub cook_time: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub servings: u32,
    pub adjustable: bool,
}

// ── Menu recipe entry ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuRecipe {
    pub name: String,
    #[serde(default = "default_portions")]
    pub portions: f64,
}

fn default_portions() -> f64 { 1.0 }

// ── Custom dish (freeform, not from recipe index) ──

/// A freeform dish added directly to the menu without a backing recipe file.
/// Displayed in the order list and cooking tabs with adjustable portions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDish {
    pub name: String,
    #[serde(default = "default_portions")]
    pub portions: f64,
}

// ── Menu (saved menu file) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuFrontmatter {
    pub date: String,
    pub name: String,
    pub diners: u32,
    #[serde(default)]
    pub menu_recipes: Vec<MenuRecipe>,
    #[serde(default)]
    pub ingredients: Vec<ScaledIngredient>,
    #[serde(default)]
    pub custom_dishes: Vec<CustomDish>,
}

/// Shared menu state — all users share one recipe list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuState {
    pub filename: String,
    pub name: String,
    pub date: String,
    pub diners: u32,
    pub menu_recipes: Vec<MenuRecipe>,
    pub ingredients: Vec<ScaledIngredient>,
    #[serde(default)]
    pub recipes: Vec<RecipeMeta>,
    #[serde(default)]
    pub custom_dishes: Vec<CustomDish>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuSummary {
    pub filename: String,
    pub name: String,
    pub date: String,
    pub diners: u32,
    pub dish_count: usize,
}

// ── API Request types ──

#[derive(Debug, Deserialize)]
pub struct CreateMenuRequest {
    #[serde(default)]
    pub creator: String,
    pub diners: u32,
    pub name: String,
}

/// Toggle a single recipe in the shared order list.
#[derive(Debug, Deserialize)]
pub struct ToggleRequest {
    #[serde(alias = "recipe_id")]
    pub recipe_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMenuRequest {
    #[serde(default)]
    pub diners: Option<u32>,
    #[serde(default)]
    pub menu_recipes: Option<Vec<MenuRecipe>>,
}

#[derive(Debug, Deserialize)]
pub struct SetCurrentRequest {
    pub filename: String,
}

// ── API Response types ──

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            message: None,
            data: Some(data),
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: Some(msg.into()),
            data: None,
        }
    }
}
