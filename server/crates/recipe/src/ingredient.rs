use std::collections::HashMap;

use crate::models::{Ingredient, MenuRecipe, RecipeMeta, ScaledIngredient};

/// Scale a single ingredient from recipe servings to target portions.
fn scale(ingredient: &Ingredient, recipe_servings: u32, portions: f64) -> ScaledIngredient {
    let ratio = portions / recipe_servings as f64;
    ScaledIngredient {
        name: ingredient.name.clone(),
        amount: ingredient.amount.map(|a| (a * ratio * 100.0).round() / 100.0),
        unit: ingredient.unit.clone(),
        hint: ingredient.hint.clone(),
        optional: ingredient.optional,
        used_in: Vec::new(),
    }
}

/// Given a list of recipe metas and their per-dish portions, compute the
/// aggregated, scaled ingredient list. Each recipe's ingredient amounts are
/// scaled by `portions / recipe.servings`, then summed by name+unit.
pub fn aggregate(
    recipes: &[&RecipeMeta],
    menu_recipes: &[MenuRecipe],
) -> Vec<ScaledIngredient> {
    let portion_map: HashMap<&str, f64> = menu_recipes
        .iter()
        .map(|m| (m.id.as_str(), m.portions))
        .collect();

    let mut groups: HashMap<(String, Option<String>), (ScaledIngredient, Vec<String>)> = HashMap::new();

    for recipe in recipes {
        let portions = portion_map.get(recipe.id.as_str()).copied().unwrap_or(1.0);

        for ing in &recipe.ingredients {
            let key = (ing.name.clone(), ing.unit.clone());
            let scaled = scale(ing, recipe.servings, portions);

            groups
                .entry(key)
                .and_modify(|(acc, used)| {
                    if let (Some(a), Some(b)) = (acc.amount, scaled.amount) {
                        acc.amount = Some(((a + b) * 100.0).round() / 100.0);
                    }
                    if !used.contains(&recipe.name) {
                        used.push(recipe.name.clone());
                    }
                })
                .or_insert_with(|| {
                    let used_in = vec![recipe.name.clone()];
                    let mut s = scaled;
                    s.used_in = used_in.clone();
                    (s, used_in)
                });
        }
    }

    let mut result: Vec<ScaledIngredient> = groups
        .into_values()
        .map(|(mut ing, used)| {
            ing.used_in = used;
            ing
        })
        .collect();

    result.sort_by(|a, b| {
        a.optional
            .cmp(&b.optional)
            .then_with(|| a.name.cmp(&b.name))
    });

    result
}
