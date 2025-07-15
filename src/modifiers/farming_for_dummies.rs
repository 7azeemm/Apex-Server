use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier};

pub struct FarmingForDummiesModifier;
const ID: &str = "FARMING_FOR_DUMMIES";

#[async_trait]
impl ModifierHandler for FarmingForDummiesModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::Int(farming_for_dummies_count) = modifier else { return false };

        if let Some(shared_price) = get_item_shared_price(ID).await {
            item_value.add_modifier(ID, Modifier::new(*farming_for_dummies_count, shared_price));
        }

        true
    }
}