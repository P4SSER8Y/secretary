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
        sub_ingredients: ingredient.sub_ingredients.as_ref().map(|subs| {
            subs.iter()
                .map(|sub| scale(sub, recipe_servings, portions))
                .collect()
        }),
    }
}

/// Given a list of recipe metas and their per-dish portions, compute the
/// aggregated, scaled ingredient list. Each recipe's ingredient amounts are
/// scaled by `portions / recipe.servings`, then summed by name+unit.
///
/// Compound ingredients (those with `sub_ingredients`) are grouped by parent
/// name across recipes, and their sub-ingredients are aggregated by name+unit
/// within each group.
pub fn aggregate(
    recipes: &[&RecipeMeta],
    menu_recipes: &[MenuRecipe],
) -> Vec<ScaledIngredient> {
    let portion_map: HashMap<&str, f64> = menu_recipes
        .iter()
        .map(|m| (m.id.as_str(), m.portions))
        .collect();

    // Flat (non-compound) ingredients: keyed by (name, unit)
    let mut flat_groups: HashMap<(String, Option<String>), (ScaledIngredient, Vec<String>)> =
        HashMap::new();

    // Compound ingredients: keyed by parent name.
    // Value: (parent_ingredient, parent_used_in, sub_map keyed by (sub_name, sub_unit))
    type SubMap = HashMap<(String, Option<String>), (ScaledIngredient, Vec<String>)>;
    let mut compound_groups: HashMap<String, (ScaledIngredient, Vec<String>, SubMap)> =
        HashMap::new();

    for recipe in recipes {
        let portions = portion_map.get(recipe.id.as_str()).copied().unwrap_or(1.0);

        for ing in &recipe.ingredients {
            if let Some(ref subs) = ing.sub_ingredients {
                // ── Compound ingredient ──
                let entry = compound_groups
                    .entry(ing.name.clone())
                    .or_insert_with(|| {
                        // Parent is a group label — amount/unit come from sub-ingredients.
                        let parent = ScaledIngredient {
                            name: ing.name.clone(),
                            amount: None,
                            unit: None,
                            hint: ing.hint.clone(),
                            optional: ing.optional,
                            used_in: Vec::new(),
                            sub_ingredients: Some(Vec::new()),
                        };
                        (parent, Vec::new(), HashMap::new())
                    });

                if !entry.1.contains(&recipe.name) {
                    entry.1.push(recipe.name.clone());
                }

                // Aggregate sub-ingredients within this compound group
                for sub in subs {
                    let sub_key = (sub.name.clone(), sub.unit.clone());
                    let scaled_sub = scale(sub, recipe.servings, portions);
                    let sub_entry = entry.2.entry(sub_key).or_insert_with(|| {
                        let used = vec![recipe.name.clone()];
                        let mut s = scaled_sub.clone();
                        s.used_in = used.clone();
                        (s, used)
                    });
                    if let (Some(a), Some(b)) = (sub_entry.0.amount, scaled_sub.amount) {
                        sub_entry.0.amount = Some(((a + b) * 100.0).round() / 100.0);
                    }
                    if !sub_entry.1.contains(&recipe.name) {
                        sub_entry.1.push(recipe.name.clone());
                    }
                }
            } else {
                // ── Flat ingredient (existing logic) ──
                let key = (ing.name.clone(), ing.unit.clone());
                let scaled = scale(ing, recipe.servings, portions);

                flat_groups
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
    }

    // Build flat results
    let mut result: Vec<ScaledIngredient> = flat_groups
        .into_values()
        .map(|(mut ing, used)| {
            ing.used_in = used;
            ing
        })
        .collect();

    // Build compound results
    for (_, (mut parent, used, sub_map)) in compound_groups {
        parent.used_in = used;
        let mut subs: Vec<ScaledIngredient> = sub_map
            .into_values()
            .map(|(mut sub, sub_used)| {
                sub.used_in = sub_used;
                sub
            })
            .collect();
        subs.sort_by(|a, b| {
            a.optional
                .cmp(&b.optional)
                .then_with(|| a.name.cmp(&b.name))
        });
        parent.sub_ingredients = Some(subs);
        result.push(parent);
    }

    result.sort_by(|a, b| {
        a.optional
            .cmp(&b.optional)
            .then_with(|| a.name.cmp(&b.name))
    });

    result
}
