use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct ManaDisintegratorModifier;
const ID: &str = "MANA_DISINTEGRATOR";

#[async_trait]
impl ModifierHandler for ManaDisintegratorModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::Int(mana_disintegrator_count) = modifier else { return };

        let price = get_item_shared_price(ID).await;
        item_value.add_modifier(ID, Modifier::new(*mana_disintegrator_count, price, ModifierInfo::new("Mana Disintegrator", mana_disintegrator_count.to_string())));
    }
}