use std::collections::HashMap;
use std::ops::Deref;
use async_trait::async_trait;
use fastnbt::Value;
use tokio::sync::RwLock;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::constants::reforges;
use crate::item_utils::get_readable_name;
use crate::item_value_calculator::{ModifierHandler};
use crate::structs::{ItemValue, Modifier, ModifierInfo, PriceDataSource, SharedPriceData};
use crate::structs::PriceDataSource::Bazaar;

pub struct ReforgeModifier;

#[async_trait]
impl ModifierHandler for ReforgeModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::String(reforge) = modifier else { return };

        if reforges::EXECLUDE_REFORGES.contains(reforge) { return }
        if let Some(&price) = reforges::NPC_REFORGES.get(reforge) {
            let price = SharedPriceData::new(RwLock::new(PriceDataSource::NPC { price }));
            item_value.add_modifier(reforge, Modifier::new_one(Some(price), ModifierInfo::new("Reforge", get_readable_name(reforge))));
            return
        }

        let id = match reforges::REFORGE_STONES.get(reforge) {
            Some(v) => v,
            None => {
                println!("Couldn't find id of {reforge} in reforges list");
                return
            }
        };

        //TODO: Apply cost (after release)

        let price = get_item_shared_price(id).await;
        item_value.add_modifier(id, Modifier::new_one(price, ModifierInfo::new("Reforge", get_readable_name(id))));
    }
}