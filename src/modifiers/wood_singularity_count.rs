use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::{ModifierHandler};
use crate::structs::{ItemValue, Modifier};

pub struct WoodSingularityModifier;
const ID: &str = "WOOD_SINGULARITY";

#[async_trait]
impl ModifierHandler for WoodSingularityModifier {
    async fn calculate_value(&self, _: &str, _: &Value, item_value: &mut ItemValue) -> bool {
        if let Some(shared_price) = get_item_shared_price(ID).await {
            item_value.add_modifier(ID, Modifier::new_one(shared_price));
        }

        true
    }
}