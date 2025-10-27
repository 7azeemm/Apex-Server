use crate::repos::neu::neu_repo::load_file;
use rustc_hash::FxHashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use tokio::time::Instant;

static GEMSTONE_SLOT_COSTS: LazyLock<RwLock<FxHashMap<String, FxHashMap<String, Vec<String>>>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

pub async fn load_gemstone_slot_costs() {
    let start_time = Instant::now();

    match load_file("constants/gemstonecosts.json").await {
        Ok(serde_json::Value::Object(items)) => {
            let mut list = GEMSTONE_SLOT_COSTS.write().await;
            list.clear();

            for (item, slots) in items {
                let slots = match slots.as_object() {
                    Some(s) => s,
                    None => continue,
                };

                let mut slots_map = FxHashMap::with_capacity_and_hasher(slots.len(), Default::default());

                for (slot, cost) in slots {
                    let cost = match cost.as_array() {
                        Some(c) => c,
                        None => continue,
                    };

                    let cost_list: Vec<String> = cost
                        .iter()
                        .filter_map(|c| c.as_str().map(|s| s.to_owned()))
                        .collect();

                    if !cost_list.is_empty() {
                        slots_map.insert(slot.clone(), cost_list);
                    }
                }

                if !slots_map.is_empty() {
                    list.insert(item, slots_map);
                }
            }

            println!("[NEU-Repo] Loaded gemstone slot costs in {:.2?}", start_time.elapsed());
        }
        Err(err) => println!("[NEU-Repo] Error occurred while loading gemstone slot costs: {:?}", err),
        _ => println!("[NEU-Repo] Error occurred while loading gemstone slot costs: Invalid JSON format")
    }
}

pub async fn get_item_gemstone_slots(item_id: &str) -> Option<FxHashMap<String, Vec<String>>> {
    GEMSTONE_SLOT_COSTS.read().await.get(item_id).cloned()
}