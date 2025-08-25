use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::auctions::get_shared_lowest_bin;
use crate::item_utils::get_readable_name;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct BoostersModifier;

#[async_trait]
impl ModifierHandler for BoostersModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::List(boosters) = modifier else { return };

        for booster in boosters {
            if let Value::String(booster) = booster {
                let id = format!("{}_BOOSTER", booster.to_uppercase());
                let price = get_shared_lowest_bin(&id).await;
                item_value.add_modifier(&id, Modifier::new_one(price, ModifierInfo::new("Boosters", get_readable_name(booster))));
            }
        }
    }
}