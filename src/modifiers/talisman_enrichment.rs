use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use sea_orm::Iden;
use crate::auctions::get_shared_lowest_bin;
use crate::bazaar::get_item_price;
use crate::item_utils::get_readable_name;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, ModifierInfo};

pub struct TalismanEnrichmentModifier;

#[async_trait]
impl ModifierHandler for TalismanEnrichmentModifier {
    async fn calculate_value(&self, _: &str, modifier: &Value, item_value: &mut ItemValue) {
        let Value::String(talisman_enrichment) = modifier else { return };
        let id = format!("TALISMAN_ENRICHMENT_{}", talisman_enrichment.to_uppercase());
        let price = get_shared_lowest_bin(&id).await;
        item_value.add_modifier(&id, Modifier::new_one(price, ModifierInfo::new("Enrichment", get_readable_name(&*id))));
    }
}