use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct FarmingForDummiesModifier;
const ID: &str = "FARMING_FOR_DUMMIES";

#[async_trait]
impl ModifierHandler for FarmingForDummiesModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::Int(farming_for_dummies_count) = modifier else { return };
        let price = get_item_shared_price(ID).await;
        item_value.add_modifier(ID, Modifier::new(*farming_for_dummies_count, price, ModifierInfo::new("Farming For Dummies", farming_for_dummies_count.to_string())));
    }
}