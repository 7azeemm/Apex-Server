use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use fastnbt::Value;
use tokio::sync::RwLock;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::constants::enchantments::{NPC_ENCHANTS, STACKING_ENCHANTS, UPGRADABLE_ENCHANTS};
use crate::item_value_calculator::{ModifierHandler};
use crate::structs::{ItemValue, Modifier, PriceDataSource, SharedPriceData};

pub struct EnchantmentsModifier;
const SILEX_ID: &str = "SIL_EX";
const STONK_PICKAXE: &str = "STONK_PICKAXE";
const PROMISING_SPADE: &str = "PROMISING_SPADE";

#[async_trait]
impl ModifierHandler for EnchantmentsModifier {
    async fn calculate_value(&self, item_id: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::Compound(enchantments) = modifier else { return false };

        let mut enchantments_ids = Vec::new();
        let mut enchants_map: HashMap<String, Modifier> = HashMap::new();
        let mut upgradable_enchants = HashMap::new();

        for (name, level) in enchantments {
            let level = match level {
                Value::Int(i) => *i,
                _ => continue
            };
            
            if STACKING_ENCHANTS.contains(name) {
                enchantments_ids.push(format!("ENCHANTMENT_{}_1", name.to_uppercase()));
                continue;
            }
            if let Some(price) = NPC_ENCHANTS.get(name) {
                let shared_price = SharedPriceData::new(RwLock::new(PriceDataSource::NPC { price: *price }));
                enchants_map.insert(name.to_string(), Modifier::new(1, shared_price));
                continue;
            }

            if let Some(required_item) = UPGRADABLE_ENCHANTS.get(&format!("{}_{}", name, level)) {
                upgradable_enchants.insert(name, (level, required_item));
                continue;
            }

            if name == "efficiency" && level > 5 {
                if item_id != STONK_PICKAXE && item_id != PROMISING_SPADE {
                    if let Some(shared_price) = get_item_shared_price(SILEX_ID).await {
                        enchants_map.insert(SILEX_ID.to_string(), Modifier::new(level - 5, shared_price));
                    }
                }
            }

            if (name == "sharpness" && level == 10) || (name == "thorns" && level == 5) { continue };
            enchantments_ids.push(format!("ENCHANTMENT_{}_{level}", name.to_uppercase()));
        }

        for enchant in &enchantments_ids {
            if let Some(shared_price) = get_item_shared_price(enchant).await {
                let price = shared_price.read().await.get_price();
                if price == 0.0 {
                    println!("{enchant}: {:?}", shared_price.read().await.get_price());
                }
                enchants_map.insert(enchant.to_string(), Modifier::new(1, shared_price));
            }
        }

        for (enchant, (level, required)) in upgradable_enchants.iter() {
            let id = format!("ENCHANTMENT_{}_{}", enchant.to_uppercase(), level);
            let downgrade_id = format!("ENCHANTMENT_{}_{}", enchant.to_uppercase(), level - 1);

            let downgrade_price = get_item_shared_price(&downgrade_id).await;
            let required_item_price = get_item_shared_price(required).await;

            if let (Some(d_price), Some(r_price)) = (downgrade_price, required_item_price) {
                let mut ingredients = HashMap::new();
                ingredients.insert(downgrade_id, Modifier::new_one(d_price));
                ingredients.insert(required.to_string(), Modifier::new_one(r_price));
                enchants_map.insert(id, Modifier::new_craftable(ingredients));
            }
        }

        for (id, modifier) in enchants_map {
            item_value.add_modifier(&*id, modifier);
        }
        true
    }
}