use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::auctions::get_shared_lowest_bin;
use crate::bazaar::get_item_price;
use crate::constants::reforges;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier};

pub struct RodSinkerModifier;

#[async_trait]
impl ModifierHandler for RodSinkerModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::Compound(sinker) = modifier else { return false };

        if let Some(Value::String(part)) = sinker.get("part") {
            let id = &part.to_uppercase();
            if let Some(shared_price) = get_shared_lowest_bin(id).await {
                item_value.add_modifier(id, Modifier::new_one(shared_price));
            }
        }

        true
    }
}