use std::collections::HashMap;
use std::sync::LazyLock;
use rustc_hash::FxHashMap;
use tokio::sync::RwLock;
use tokio::time::Instant;
use crate::extensions::json_ext::JsonExt;
use crate::repos::neu::neu_repo::load_file;

static REFORGE_STONES: LazyLock<RwLock<FxHashMap<String, ReforgeStone>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

#[derive(Clone)]
pub struct ReforgeStone {
    pub id: String,
    pub apply_cost: HashMap<String, u64>
}

pub async fn load_reforge_stones() {
    let start_time = Instant::now();

    match load_file("constants/reforgestones.json").await {
        Ok(serde_json::Value::Object(items)) => {
            let mut reforge_stones = REFORGE_STONES.write().await;
            reforge_stones.clear();

            for (stone_id, data) in items {
                let reforge_name = data.get_str("nbtModifier").or_else(|| data.get_str("reforgeName"));
                let apply_cost = data.get_object("reforgeCosts");

                if let (Some(reforge_name), Some(apply_cost)) = (reforge_name, apply_cost) {
                    let apply_cost: HashMap<String, u64> = apply_cost
                        .iter()
                        .filter_map(|(k, v)| v.as_u64().map(|val| (k.to_owned(), val)))
                        .collect();
                    let clean_name = reforge_name.to_lowercase();
                    let reforge_stone = ReforgeStone { id: stone_id, apply_cost };
                    reforge_stones.insert(clean_name.clone(), reforge_stone.clone());

                    if clean_name == "blood_shot" {
                        reforge_stones.insert("bloodshot".to_owned(), reforge_stone);
                    }
                }
            }

            println!("[NEU-Repo] Loaded reforge stones in {:.2?}", start_time.elapsed());
        }
        Err(err) => println!("[NEU-Repo] Error occurred while loading reforge stones: {:?}", err),
        _ => println!("[NEU-Repo] Error occurred while loading reforge stones: Invalid JSON format")
    }
}

pub async fn get_reforge_stone(reforge_name: &str) -> Option<ReforgeStone> {
    REFORGE_STONES.read().await.get(reforge_name).cloned()
}