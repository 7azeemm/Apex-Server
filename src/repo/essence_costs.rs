use crate::extensions::json_ext::JsonExt;
use crate::repo::neu_repo::load_repo_file;
use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use tokio::time::Instant;

static ESSENCE_COSTS: LazyLock<RwLock<FxHashMap<String, ItemUpgradeCosts>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

#[derive(Clone)]
pub struct ItemUpgradeCosts {
    pub essence_type: String,
    pub stars: HashMap<u64, StarCost>,
    pub dungeonize_cost: Option<u64>,
}

#[derive(Clone)]
pub struct StarCost {
    pub essence: u64,
    pub items: Vec<(String, u64)>,
}

pub async fn load_essence_costs() {
    let start_time = Instant::now();

    match load_repo_file("constants/essencecosts.json").await {
        Ok(serde_json::Value::Object(items)) => {
            let mut essence_costs = ESSENCE_COSTS.write().await;
            essence_costs.clear();

            for (id, data) in items {
                if let Some(upgrade_costs) = get_upgrade_costs(data) {
                    essence_costs.insert(id, upgrade_costs);
                }
            }

            println!("[NEU-Repo] Successfully extracted essence costs in {:.2?}", start_time.elapsed());
        }
        Err(err) => println!("[NEU-Repo] Error occurred while extracting essence costs: {:?}", err),
        _ => println!("[NEU-Repo] Error occurred while extracting essence costs: Invalid JSON format")
    }
}

fn get_upgrade_costs(map: serde_json::Value) -> Option<ItemUpgradeCosts> {
    let essence_type = map.get_str("type")?.to_owned();
    let dungeonize_cost = map.get_u64("dungeonize");
    let items = map.get("items");
    let mut stars = HashMap::new();

    for (k, v) in map.as_object()? {
        if let Ok(num) = k.parse::<u64>() {
            let essence = v.as_u64()?;
            let mut star_items = Vec::new();
            if let Some(items) = items.get_array(k) {
                for item in items {
                    if let Some(item_str) = item.as_str() {
                        let mut parts = item_str.splitn(2, ':');
                        if let (Some(item), Some(amount)) = (parts.next(), parts.next()) {
                            let amount = amount.parse::<u64>().unwrap_or(0);
                            star_items.push((item.to_owned(), amount))
                        }
                    }
                }
            }

            stars.insert(num, StarCost { essence, items: star_items });
        }
    }

    Some(ItemUpgradeCosts { essence_type, stars, dungeonize_cost })
}

pub async fn get_essence_costs(item_id: &str) -> Option<ItemUpgradeCosts> {
    ESSENCE_COSTS.read().await.get(item_id).cloned()
}