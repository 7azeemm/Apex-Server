use crate::repos::neu::neu_repo::load_file;
use rustc_hash::FxHashMap;
use std::error::Error;
use std::sync::LazyLock;
use tokio::sync::RwLock;

static GEMSTONE_SLOT_COSTS: LazyLock<RwLock<FxHashMap<String, FxHashMap<String, Vec<String>>>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

pub async fn load_gemstone_slot_costs() -> Result<(), Box<dyn Error + Send + Sync>> {
    let json = load_file("constants/gemstonecosts.json").await?;
    let items = json.as_object().ok_or("Json is not an object")?;

    let mut map = FxHashMap::default();

    for (item, slots) in items {
        let Some(slots) = slots.as_object() else { continue };
        let mut slots_map = FxHashMap::with_capacity_and_hasher(slots.len(), Default::default());

        for (slot, cost) in slots {
            let Some(cost) = cost.as_array() else { continue };

            let cost_list: Vec<String> = cost
                .iter()
                .filter_map(|c| c.as_str().map(|s| s.to_owned()))
                .collect();

            if !cost_list.is_empty() {
                slots_map.insert(slot.clone(), cost_list);
            }
        }

        if !slots_map.is_empty() {
            map.insert(item.to_owned(), slots_map);
        }
    }

    let mut current_map = GEMSTONE_SLOT_COSTS.write().await;
    *current_map = map;

    Ok(())
}

pub async fn get_item_gemstone_slots(item_id: &str) -> Option<FxHashMap<String, Vec<String>>> {
    GEMSTONE_SLOT_COSTS.read().await.get(item_id).cloned()
}
