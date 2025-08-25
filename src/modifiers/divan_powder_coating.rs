use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct DivanPowderCoatingModifier;
const ID: &str = "DIVAN_POWDER_COATING";

#[async_trait]
impl ModifierHandler for DivanPowderCoatingModifier {
    async fn calculate_value(&self, _: &str, _: &Value, item_value: &mut ItemValue) {
        let price = get_item_shared_price(ID).await;
        item_value.add_modifier(ID, Modifier::new_one(price, ModifierInfo::new("Divan Powder Coating", "applied".to_string())));
    }
}