use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct PolarvoidModifier;
const ID: &str = "POLARVOID_BOOK";

#[async_trait]
impl ModifierHandler for PolarvoidModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::Int(polarvoid_count) = modifier else { return };

        let price = get_item_shared_price(ID).await;
        item_value.add_modifier(ID, Modifier::new(*polarvoid_count, price, ModifierInfo::new("Polarvoid", polarvoid_count.to_string())));
    }
}