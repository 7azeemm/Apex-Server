use crate::repos::neu::neu_repo::load_file;
use common::extensions::json_ext::JsonExt;
use getset::Getters;
use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use tokio::sync::RwLock;

static ESSENCE_COSTS: LazyLock<RwLock<FxHashMap<String, ItemUpgradeCosts>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

#[derive(Clone, Getters)]
#[getset(get = "pub")]
pub struct ItemUpgradeCosts {
    essence_type: String,
    stars: HashMap<u64, StarCost>,
    dungeonize_cost: Option<u64>,
}

#[derive(Clone, Getters)]
#[getset(get = "pub")]
pub struct StarCost {
    essence: u64,
    items: Vec<(String, u64)>,
}

pub async fn load_essence_costs() -> Result<(), Box<dyn Error + Send + Sync>> {
    let json = load_file("constants/essencecosts.json").await?;
    let items = json.as_object().ok_or("Json is not an object")?;

    let mut map = FxHashMap::default();

    for (id, data) in items {
        if let Some(upgrade_costs) = get_upgrade_costs(data) {
            map.insert(id.to_owned(), upgrade_costs);
        }
    }

    let mut current_map = ESSENCE_COSTS.write().await;
    *current_map = map;

    Ok(())
}

fn get_upgrade_costs(map: &serde_json::Value) -> Option<ItemUpgradeCosts> {
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
