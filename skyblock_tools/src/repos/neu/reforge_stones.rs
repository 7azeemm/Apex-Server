use crate::repos::neu::neu_repo::load_file;
use common::extensions::json_ext::JsonExt;
use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use tokio::sync::RwLock;

static REFORGE_STONES: LazyLock<RwLock<FxHashMap<String, ReforgeStone>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

#[derive(Clone)]
pub struct ReforgeStone {
    pub id: String,
    pub apply_cost: HashMap<String, u64>,
}

pub async fn load_reforge_stones() -> Result<(), Box<dyn Error + Send + Sync>> {
    let json = load_file("constants/reforgestones.json").await?;
    let items = json.as_object().ok_or("Json is not an object")?;

    let mut map = FxHashMap::default();

    for (stone_id, data) in items {
        let reforge_name = data
            .get_str("nbtModifier")
            .or_else(|| data.get_str("reforgeName"));
        let apply_cost = data.get_object("reforgeCosts");

        if let (Some(reforge_name), Some(apply_cost)) = (reforge_name, apply_cost) {
            let apply_cost: HashMap<String, u64> = apply_cost
                .iter()
                .filter_map(|(k, v)| v.as_u64().map(|val| (k.to_owned(), val)))
                .collect();
            let clean_name = reforge_name.to_lowercase();
            let reforge_stone = ReforgeStone {
                id: stone_id.to_owned(),
                apply_cost,
            };
            map.insert(clean_name.clone(), reforge_stone.clone());

            if clean_name == "blood_shot" {
                map.insert("bloodshot".to_owned(), reforge_stone);
            }
        }
    }

    let mut reforge_stones = REFORGE_STONES.write().await;
    *reforge_stones = map;

    Ok(())
}

pub async fn get_reforge_stone(reforge_name: &str) -> Option<ReforgeStone> {
    REFORGE_STONES.read().await.get(reforge_name).cloned()
}