use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct TransmissionTunerModifier;
const ID: &str = "TRANSMISSION_TUNER";

#[async_trait]
impl ModifierHandler for TransmissionTunerModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::Int(tuner_count) = modifier else { return };
        let price = get_item_shared_price(ID).await;
        item_value.add_modifier(ID, Modifier::new(*tuner_count, price, ModifierInfo::new("Transmission Tuners", tuner_count.to_string())));
    }
}