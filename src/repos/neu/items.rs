use crate::constants::misc::RARITIES;
use crate::extensions::json_ext::JsonExt;
use crate::item_utils::strip_formatting;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::fs;
use std::io::Error;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use crate::repos::neu::neu_repo::REPO_PATH;

const ACCESSORY_RARITIES: [&str; 3] = ["ACCESSORY", "HATCESSORY", "DUNGEON ACCESSORY"];
static ITEMS: LazyLock<RwLock<FxHashMap<String, Value>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));
pub static ACCESSORIES: LazyLock<RwLock<FxHashMap<String, String>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

pub async fn load_items() -> Result<(), Error> {
    let entries = fs::read_dir(&format!("{REPO_PATH}/items"))?;

    let mut map = FxHashMap::default();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue }

        let string = fs::read_to_string(&path)?;
        let value = serde_json::from_str::<Value>(&string)?;

        if let Some(item_id) = value.get_str("internalname") {
            map.insert(item_id.to_owned(), value);
        }
    }

    let mut items = ITEMS.write().await;
    *items = map;

    Ok(())
}

pub async fn load_accessories() -> Result<(), Error> {
    let mut accessories = FxHashMap::default();

    let items = ITEMS.read().await;
    for (item_id, value) in items.iter() {
        if let Some(lore) = value.get_array("lore") {
            if let Some(Value::String(last_line)) = lore.last() {
                if ACCESSORY_RARITIES.iter().any(|r| last_line.contains(r)) {
                    let stripped_line = strip_formatting(last_line);
                    let rarity = RARITIES.iter()
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
    if let Some(item) = ITEMS.read().await.get(item_id) {
        if let Some(display_name) = item.get_str("displayname") {
            return Some(strip_formatting(display_name))
        }
    }
    None
}