use std::collections::{HashMap, HashSet};
use async_trait::async_trait;
use fastnbt::Value;
use phf::{phf_set, Set};
use sea_orm::{ColIdx, Iden};
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_utils::get_readable_name;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct GemstonesModifier;
static SPECIAL_GEMS: Set<&'static str> = phf_set! {
    "JADE", "ONYX", "AMBER", "RUBY", "SAPPHIRE", "AMETHYST", "JASPER", "TOPAZ",
    "PERIDOT", "AQUAMARINE", "CITRINE", "OPAL"
};

#[async_trait]
impl ModifierHandler for GemstonesModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::Compound(gems) = modifier else { return };

        let mut result: HashMap<String, usize> = HashMap::new();

        for (k, v) in gems {
            if k == "unlocked_slots" { continue }
            if k.ends_with("_gem") {
                let base_key = &k[..k.len() - 4];
                if let Some(base_val) = gems.get(base_key) {
                    let gem = extract_field_str(v, "gem").or_else(|| extract_field_str(v, ""));
                    let quality = extract_field_str(base_val, "quality").or_else(|| extract_field_str(base_val, ""));
                    if let (Some(quality), Some(gem)) = (quality, gem) {
                        let key = format!("{}_{}_GEM", quality, gem);
                        *result.entry(key).or_insert(0) += 1;
                    }
                }
            } else if let Some(pos) = k.find('_') {
                let gem_name = &k[..pos];
                if SPECIAL_GEMS.contains(gem_name) {
                    let quality = extract_field_str(v, "quality").or_else(|| extract_field_str(v, ""));
                    if let Some(quality) = quality {
                        let key = format!("{}_{}_GEM", quality, gem_name);
                        *result.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }

        for (id, count) in result {
            let price = get_item_shared_price(&id).await;
            item_value.add_modifier(&id, Modifier::new(count as i32, price, ModifierInfo::new("Gemstones", get_readable_name(&*id))));
        }

        //Todo: Apply cost
        //https://github.com/NotEnoughUpdates/NotEnoughUpdates-REPO/blob/master/.github/scripts/updateGemstoneCosts.py
    }
}

fn extract_field_str<'a>(val: &'a Value, field: &str) -> Option<&'a str> {
    match val {
        Value::String(s) if field == "" => Some(s.as_str()),
        Value::Compound(map) => match map.get(field) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}