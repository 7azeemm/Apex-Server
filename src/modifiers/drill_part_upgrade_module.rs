use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::auctions::get_shared_lowest_bin;
use crate::bazaar::get_item_price;
use crate::item_utils::get_readable_name;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct DrillPartUpgradeModuleModifier;

#[async_trait]
impl ModifierHandler for DrillPartUpgradeModuleModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::String(part) = modifier else { return };

        let id = part.to_uppercase();
        let price = get_shared_lowest_bin(&id).await;
        item_value.add_modifier(&id, Modifier::new_one(price, ModifierInfo::new("Drill Upgrade", get_readable_name(part))));
    }
}