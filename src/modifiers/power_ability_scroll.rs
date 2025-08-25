use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::auctions::get_shared_lowest_bin;
use crate::item_utils::get_readable_name;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct PowerAbilityScrollModifier;

#[async_trait]
impl ModifierHandler for PowerAbilityScrollModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::String(scroll_id) = modifier else { return };

        let price = get_shared_lowest_bin(scroll_id).await;
        item_value.add_modifier(scroll_id, Modifier::new_one(price, ModifierInfo::new("Power Ability Scroll", get_readable_name(scroll_id))));
    }
}