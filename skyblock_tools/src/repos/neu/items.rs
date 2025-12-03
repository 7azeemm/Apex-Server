use std::collections::HashMap;
use crate::constants::misc::{ACCESSORY_RARITIES, RARITIES};
use crate::repos::neu::neu_repo::REPO_PATH;
use crate::utils::strip_formatting;
use common::extensions::json_ext::JsonExt;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use crate::item_utils::get_pretty_name;

static ITEMS: LazyLock<RwLock<FxHashMap<String, Value>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));
static ITEM_NAMES: LazyLock<RwLock<FxHashMap<String, String>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));
static PET_NAMES: LazyLock<RwLock<FxHashMap<String, String>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));
pub static RECIPES: LazyLock<RwLock<FxHashMap<String, HashMap<String, String>>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));
pub static ACCESSORIES: LazyLock<RwLock<FxHashMap<String, String>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

pub async fn load_items() -> Result<(), Box<dyn Error + Send + Sync>> {
    let entries = fs::read_dir(format!("{REPO_PATH}/items"))?;

    let mut items_map = FxHashMap::default();
    let mut item_names_map = FxHashMap::default();
    let mut pet_names_map = FxHashMap::default();
    let mut recipes_map = FxHashMap::default();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let string = fs::read_to_string(&path)?;
        let value = serde_json::from_str::<Value>(&string)?;

        if let Some(item_id) = value.get_str("internalname") {
            if let Some(display_name) = value.get_str("displayname") {
                if display_name.contains("[Lvl {LVL}]") {
                    let pet_name = display_name.replace("[Lvl {LVL}] ", "");
                    let mut parts = item_id.split(";");
                    if let (Some(pet), Some(rarity_index)) = (parts.next(), parts.next()) {
                        if let Ok(rarity_index) = rarity_index.parse::<usize>() {
                            if let Some(rarity) = RARITIES.get(rarity_index) {
                                let pet_id = format!("{}_{}", rarity, pet);
                                let pet_name = format!("{} {}", get_pretty_name(rarity), strip_formatting(&pet_name));
                                pet_names_map.insert(pet_id.clone(), pet_name);
                                items_map.insert(pet_id, value);
                                continue;
                            }
                        }
                    }
                }
                item_names_map.insert(item_id.to_owned(), strip_formatting(display_name));

                let recipe = value.get_array("recipes")
                    .map(|a| a.first())
                    .unwrap_or_else(|| value.get("recipe"));

                if let Some(recipe) = recipe {
                    let recipe_type = recipe.get_str("type");
                    if recipe_type == None || recipe_type == Some("crafting") {
                        let mut recipe_map = HashMap::new();
                        for (k, v) in recipe.as_object().unwrap() {
                            if let Some(v) = v.as_str() {
                                if k.len() == 2 || k == "type" || k == "count" {
                                    recipe_map.insert(k.to_owned(), v.to_owned());
                                }
                            }
                        }
                        recipes_map.insert(item_id.to_owned(), recipe_map);
                    }
                }
            }

            items_map.insert(item_id.to_owned(), value);
        }
    }

    let mut pet_names = PET_NAMES.write().await;
    *pet_names = pet_names_map;

    let mut item_names = ITEM_NAMES.write().await;
    *item_names = item_names_map;

    let mut items = ITEMS.write().await;
    *items = items_map;

    let mut recipes = RECIPES.write().await;
    *recipes = recipes_map;

    Ok(())
}

pub async fn load_accessories() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut accessories = FxHashMap::default();

    let items = ITEMS.read().await;
    for (item_id, value) in items.iter() {
        if let Some(lore) = value.get_array("lore") {
            if let Some(Value::String(last_line)) = lore.last() {
                if ACCESSORY_RARITIES.iter().any(|r| last_line.contains(r)) {
                    let stripped_line = strip_formatting(last_line);
                    let rarity = RARITIES
                        .iter()
                        .find(|r| stripped_line.starts_with(*r))
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    accessories.insert(item_id.to_owned(), rarity);
                }
            }
        }
    }

    let mut map = ACCESSORIES.write().await;
    *map = accessories;

    Ok(())
}

pub async fn get_item_display_name(item_id: &str) -> Option<String> {
    ITEM_NAMES.read().await.get(item_id).cloned()
}

pub async fn get_id_by_name(name: &str) -> Vec<String> {
    let mut items = ITEM_NAMES.read().await.clone();
    items.extend(PET_NAMES.read().await.clone());
    let names: Vec<String> = items.values().cloned().collect();

    let matches = find_best_matches(name, &names);

    if let Some(best_name) = matches.first() {
        return items
            .iter()
            .filter(|(_, n)| *n == *best_name)
            .map(|(id, _)| id.clone())
            .collect();
    }

    vec![]
}

pub fn find_best_matches<'a>(query: &'a str, list: &'a [String]) -> Vec<&'a str> {
    fn score(candidate: &str, query: &str) -> usize {
        let query_lowercase = query.to_lowercase();
        let candidate_lowercase = candidate.to_lowercase();
        let query_words: Vec<_> = query_lowercase.split_whitespace().collect();
        let candidate_words: Vec<_> = candidate_lowercase.split_whitespace().collect();
        let mut score = 0;

        for qw in &query_words {
            score += match () {
                _ if candidate_words.iter().any(|cw| cw == qw) => 5, // Exact match
                _ if candidate_words.iter().any(|cw| cw.starts_with(qw)) => 3, // Prefix match
                _ if candidate_words.iter().any(|cw| cheap_distance(cw, qw) <= 2) => 2, // Fuzzy match: allow 1-2 typos
                _ => 0,
            };
        }

        score
    }

    // char-difference distance
    fn cheap_distance(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let len = a.len().max(b.len());
        let mut dif = 0;
        for i in 0..len {
            if a.get(i) != b.get(i) {
                dif += 1;
            }
        }
        dif
    }

    let mut scored: Vec<(&str, usize)> = list
        .iter()
        .map(|item| (item.as_str(), score(item, query)))
        .filter(|(_, s)| *s > 0)
        .collect();

    // Sort descending by score, then shorter names
    scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.len().cmp(&b.0.len())));

    // Return top 5
    scored.into_iter().take(5).map(|(s, _)| s).collect()
}