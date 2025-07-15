use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, SharedPriceData};

pub struct AbilityScrollModifier;

#[async_trait]
impl ModifierHandler for AbilityScrollModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::List(scrolls) = modifier else { return false };

        for scroll in scrolls {
            if let Value::String(id) = scroll {
                if let Some(shared_price) = get_item_shared_price(&id).await {
                    item_value.add_modifier(id, Modifier::new_one(shared_price));
                }
            }
        }
        
        true
    }
}