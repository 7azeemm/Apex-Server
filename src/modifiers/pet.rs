use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use sea_orm::JsonValue;
use crate::auctions::get_shared_lowest_bin;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier};

pub struct PetModifier;

#[async_trait]
impl ModifierHandler for PetModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::String(pet_info) = modifier else { return false };

        let pet_data: JsonValue = match serde_json::from_str(pet_info) {
            Ok(data) => data,
            Err(_) => return false,
        };

        if let Some(skin) = pet_data.get("skin") {
            if let Some(skin_name) = skin.as_str() {
                let id = format!("PET_SKIN_{}", skin_name);
                if let Some(shared_price) = get_shared_lowest_bin(&id).await {
                    item_value.add_modifier(&*id, Modifier::new_one(shared_price));
                }
            }
        }

        if let Some(held_item) = pet_data.get("heldItem") {
            if let Some(held_item_name) = held_item.as_str() {
                if let Some(shared_price) = get_shared_lowest_bin(&held_item_name).await {
                    item_value.add_modifier(&*held_item_name, Modifier::new_one(shared_price));
                }
            }
        }

        true
    }
}