use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use fastnbt::Value;
use tokio::sync::RwLock;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::constants::enchantments::{NPC_ENCHANTS, STACKING_ENCHANTS, UPGRADABLE_ENCHANTS};
use crate::item_utils::get_readable_name;
use crate::item_value_calculator::{ModifierHandler};
use crate::structs::{ItemValue, Modifier, ModifierInfo, PriceDataSource, SharedPriceData};

pub struct EnchantmentsModifier;
const SILEX_ID: &str = "SIL_EX";
const STONK_PICKAXE: &str = "STONK_PICKAXE";
const PROMISING_SPADE: &str = "PROMISING_SPADE";

#[async_trait]
impl ModifierHandler for EnchantmentsModifier {
    async fn calculate_value(&self, item_id: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::Compound(enchantments) = modifier else { return };

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
                let price = SharedPriceData::new(RwLock::new(PriceDataSource::NPC { price: *price }));
                enchants_map.insert(name.to_string(), Modifier::new_one(Some(price), ModifierInfo::new("Enchantments", get_readable_name(name))));
                continue;
            }

            if let Some(required_item) = UPGRADABLE_ENCHANTS.get(&format!("{}_{}", name, level)) {
                upgradable_enchants.insert(name, (level, required_item));
                continue;
            }

            if name == "efficiency" && level > 5 {
                if item_id != STONK_PICKAXE && item_id != PROMISING_SPADE {
                    let price = get_item_shared_price(SILEX_ID).await;
                    enchants_map.insert(SILEX_ID.to_string(), Modifier::new(level - 5, price, ModifierInfo::new("Enchantments", format!("{}x {}", level - 5, get_readable_name(SILEX_ID)))));
                }
            }

            if (name == "sharpness" && level == 10) || (name == "thorns" && level == 5) { continue };
            enchantments_ids.push(format!("ENCHANTMENT_{}_{level}", name.to_uppercase()));
        }

        for enchant in &enchantments_ids {
            let price = get_item_shared_price(enchant).await;
            enchants_map.insert(enchant.to_string(), Modifier::new_one(price, ModifierInfo::new("Enchantments", get_readable_name(enchant))));
        }

        for (enchant, (level, required)) in upgradable_enchants.iter() {
            let id = format!("ENCHANTMENT_{}_{}", enchant.to_uppercase(), level);
            let downgrade_id = format!("ENCHANTMENT_{}_{}", enchant.to_uppercase(), level - 1);

            let downgrade_price = get_item_shared_price(&downgrade_id).await;
            let required_item_price = get_item_shared_price(required).await;

            let mut ingredients = HashMap::new();
            ingredients.insert(downgrade_id.to_string(), Modifier::new_one(downgrade_price, ModifierInfo::new("Enchantments", get_readable_name(&*downgrade_id))));
            ingredients.insert(required.to_string(), Modifier::new_one(required_item_price, ModifierInfo::new("Enchantments", get_readable_name(required))));
            enchants_map.insert(id.to_string(), Modifier::new_craftable(ingredients, ModifierInfo::new("Enchantments", get_readable_name(&*id))));
        }

        for (id, modifier) in enchants_map {
            item_value.add_modifier(&*id, modifier);
        }
    }
}