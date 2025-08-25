use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use sea_orm::Iden;
use crate::bazaar::{get_item_price, get_item_shared_price};
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct ArtOfPieceModifier;
const ID: &str = "THE_ART_OF_PEACE";

#[async_trait]
impl ModifierHandler for ArtOfPieceModifier {
    async fn calculate_value(&self, _: &str, _: &Value, item_value: &mut ItemValue) {
        let price = get_item_shared_price(ID).await;
        item_value.add_modifier(ID, Modifier::new_one(price, ModifierInfo::new("Art Of Peace", "applied".to_string())));
    }
}