use anyhow::{anyhow, Context};
use crate::models::{MenuFrontmatter, RecipeMeta, ScaledIngredient};

/// Parse a recipe markdown file into RecipeMeta.
/// Frontmatter is delimited by `---`, the rest is markdown body.
pub fn parse_recipe(raw: &str) -> anyhow::Result<RecipeMeta> {
    let parts: Vec<&str> = raw.splitn(3, "---").collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid recipe format: no frontmatter found"));
    }

    let frontmatter_str = if parts[0].trim().is_empty() {
        // Standard: starts with ---
        if parts.len() < 3 {
            return Err(anyhow!("Invalid recipe format: unclosed frontmatter"));
        }
        parts[1]
    } else {
        return Err(anyhow!("Recipe must start with --- frontmatter"));
    };

    let body = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default();

    let mut meta: RecipeMeta = serde_yaml::from_str(frontmatter_str)
        .context("Failed to parse recipe frontmatter")?;
    meta.body = body;

    Ok(meta)
}

/// Generate markdown frontmatter for a saved menu file.
pub fn generate_menu_frontmatter(fm: &MenuFrontmatter) -> String {
    let yaml = serde_yaml::to_string(fm).unwrap_or_default();
    format!("---\n{}---\n", yaml)
}

/// Serialize ingredients to YAML for menu frontmatter.
pub fn ingredients_to_yaml(ingredients: &[ScaledIngredient]) -> String {
    serde_yaml::to_string(ingredients).unwrap_or_default()
}
