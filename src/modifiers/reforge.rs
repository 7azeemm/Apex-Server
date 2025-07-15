use std::collections::HashMap;
use std::ops::Deref;
use async_trait::async_trait;
use fastnbt::Value;
use tokio::sync::RwLock;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::constants::reforges;
use crate::item_value_calculator::{ModifierHandler};
use crate::structs::{ItemValue, Modifier, PriceDataSource, SharedPriceData};
use crate::structs::PriceDataSource::Bazaar;

pub struct ReforgeModifier;

#[async_trait]
impl ModifierHandler for ReforgeModifier {
    async fn calculate_value(&self, item_id: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::String(reforge) = modifier else { return false };

        if reforges::EXECLUDE_REFORGES.contains(reforge) { return false }
        if let Some(&price) = reforges::NPC_REFORGES.get(reforge) {
            let shared_price = SharedPriceData::new(RwLock::new(PriceDataSource::NPC { price }));
            item_value.add_modifier(reforge, Modifier::new_one(shared_price));
            return true
        }

        let id = match reforges::REFORGE_STONES.get(reforge) {
            Some(v) => v,
            None => {
                println!("Couldn't find id of {reforge} in reforges list");
                return false
            }
        };

        //TODO: Apply cost (after release)

        if let Some(shared_price) = get_item_shared_price(id).await {
            item_value.add_modifier(id, Modifier::new_one(shared_price));
            return true
        }
        
        false
    }
}