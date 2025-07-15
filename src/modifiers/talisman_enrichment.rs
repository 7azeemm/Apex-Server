use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use sea_orm::Iden;
use crate::auctions::get_shared_lowest_bin;
use crate::bazaar::get_item_price;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier};

pub struct TalismanEnrichmentModifier;

#[async_trait]
impl ModifierHandler for TalismanEnrichmentModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::String(talisman_enrichment) = modifier else { return false };

        let id = format!("TALISMAN_ENRICHMENT_{}", talisman_enrichment.to_uppercase());
        if let Some(shared_price) = get_shared_lowest_bin(&id).await {
            item_value.add_modifier(&id, Modifier::new_one(shared_price));
        }

        true
    }
}