use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct WetBookModifier;
const ID: &str = "WET_BOOK";

#[async_trait]
impl ModifierHandler for WetBookModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::Int(wet_book_count) = modifier else { return };
        let price = get_item_shared_price(ID).await;
        item_value.add_modifier(ID, Modifier::new(*wet_book_count, price, ModifierInfo::new("Wet Books", wet_book_count.to_string())));
    }
}