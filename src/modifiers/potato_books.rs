use std::cmp::{max, min};
use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::{ModifierHandler};
use crate::structs::{ItemValue, Modifier};

pub struct PotatoBooksModifier;
const POTATO_ID: &str = "HOT_POTATO_BOOK";
const FUMING_ID: &str = "FUMING_POTATO_BOOK";

#[async_trait]
impl ModifierHandler for PotatoBooksModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::Int(count) = modifier else { return false };

        let hot_potato_books = min(10, *count);
        let fuming_books = max(0, *count - 10);

        if let Some(shared_price) = get_item_shared_price(POTATO_ID).await {
            item_value.add_modifier(POTATO_ID, Modifier::new(hot_potato_books, shared_price));
        }

        if fuming_books > 0 {
            if let Some(shared_price) = get_item_shared_price(FUMING_ID).await {
                item_value.add_modifier(FUMING_ID, Modifier::new(fuming_books, shared_price));
            }
        }
        
        true
    }
}