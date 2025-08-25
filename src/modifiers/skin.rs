use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::auctions::get_shared_lowest_bin;
use crate::bazaar::get_item_price;
use crate::item_utils::get_readable_name;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct SkinModifier;

#[async_trait]
impl ModifierHandler for SkinModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::String(skin) = modifier else { return };

        let price = get_shared_lowest_bin(skin).await;
        item_value.add_modifier(skin, Modifier::new_one(price, ModifierInfo::new("Skin", get_readable_name(skin))));
    }
}