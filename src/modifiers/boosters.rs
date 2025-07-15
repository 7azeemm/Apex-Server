use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::auctions::get_shared_lowest_bin;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier};

pub struct BoostersModifier;

#[async_trait]
impl ModifierHandler for BoostersModifier {
    async fn calculate_value(&self, item_id: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::List(boosters) = modifier else { return false };

        for booster in boosters {
            if let Value::String(booster) = booster {
                let id = format!("{}_BOOSTER", booster.to_uppercase());
                if let Some(shared_price) = get_shared_lowest_bin(&id).await {
                    item_value.add_modifier(&id, Modifier::new_one(shared_price));
                }
            }
        }

        true
    }
}