use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::auctions::get_shared_lowest_bin;
use crate::bazaar::get_item_price;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct EtherwarpConduitModifier;
const ID: &str = "ETHERWARP_CONDUIT";

#[async_trait]
impl ModifierHandler for EtherwarpConduitModifier {
    async fn calculate_value(&self, _: &str, _: &Value, item_value: &mut ItemValue) {
        let price = get_shared_lowest_bin(ID).await;
        item_value.add_modifier(ID, Modifier::new_one(price, ModifierInfo::new("Etherwarp Conduit", "applied".to_string())));
    }
}