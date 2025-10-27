use crate::constants::misc::RARITIES;
use crate::extensions::json_ext::JsonExt;
use crate::item_utils::strip_formatting;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::fs;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use tokio::time::Instant;

static ITEMS: LazyLock<RwLock<FxHashMap<String, Value>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));
pub static ACCESSORIES: LazyLock<RwLock<FxHashMap<String, String>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));
static ACCESSORY_RARITIES: [&str; 3] = ["ACCESSORY", "HATCESSORY", "DUNGEON ACCESSORY"];

pub async fn load_items(dir: &str) {
    let start_time = Instant::now();
    let mut map = FxHashMap::default();

    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();

                        if path.extension().and_then(|e| e.to_str()) != Some("json") {
                            continue;
                        }

                        match fs::read_to_string(&path) {
                            Ok(buf) => match serde_json::from_str::<Value>(&buf) {
                                Ok(value) => {
                                    if let Some(item_id) = value.get_str("internalname") {
                                        map.insert(item_id.to_owned(), value);
                                    }
                                }
                                Err(err) => eprintln!("[NEU-Repo] Failed to parse JSON in {}: {err}", path.display()),
                            },
                            Err(err) => eprintln!("[NEU-Repo] Failed to read {}: {err}", path.display()),
                        }
                    }
                    Err(err) => eprintln!("[NEU-Repo] Failed to read entry in dir {dir}: {err}"),
                }
            }
        }
        Err(err) => {
            eprintln!("[NEU-Repo] Failed to read dir {dir}: {err}");
            return;
        }
    }

    {
        let mut items = ITEMS.write().await;
        *items = map;
        println!("[NEU-Repo] Loaded {} items in {:.2?}", items.len(), start_time.elapsed());
    }
}

pub async fn load_accessories() {
    let start_time = Instant::now();
    let mut accessories = FxHashMap::default();

    let items = ITEMS.read().await;
    for (item_id, value) in items.iter() {
        if let Some(lore) = value.get_array("lore") {
            if let Some(Value::String(last_line)) = lore.last() {
                if ACCESSORY_RARITIES.iter().any(|r| last_line.contains(r)) {
                    let stripped_line = strip_formatting(last_line);
                    let rarity = RARITIES.iter().find(|r| stripped_line.starts_with(*r)).unwrap();
                    accessories.insert(item_id.to_owned(), rarity.to_string());
                }
            }
        }
    }

    let mut map = ACCESSORIES.write().await;
    *map = accessories;
    println!("[NEU-Repo] Loaded {} accessories in {:.2?}", map.len(), start_time.elapsed());
}