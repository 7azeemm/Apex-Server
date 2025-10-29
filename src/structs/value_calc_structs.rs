use crate::prices::bazaar::get_buy_price;
use crate::structs::item_structs::{ItemValue, ModifierContext};
use async_trait::async_trait;
use fastnbt::Value;

#[async_trait]
pub trait ModifierHandler: Send + Sync {
    async fn calculate_value(&self, ctx: &ModifierContext<'_>, attr: &Value, item_value: &mut ItemValue);
}

pub struct SingleItemModifier {
    label: &'static str,
    item_id: &'static str,
}

impl SingleItemModifier {
    pub const fn new(label: &'static str, item_id: &'static str) -> Self {
        Self { label, item_id }
    }
}

#[async_trait]
impl ModifierHandler for SingleItemModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, _attr: &Value, value: &mut ItemValue) {
        let price = get_buy_price(self.item_id).await;
        value.add(&format!("{}: Applied", self.label), price, 1);
    }
}

pub struct CountedItemModifier {
    label: &'static str,
    item_id: &'static str,
    max_count: u64,
}

impl CountedItemModifier {
    pub const fn new(label: &'static str, item_id: &'static str, max_count: u64) -> Self {
        Self { label, item_id, max_count }
    }
}

#[async_trait]
impl ModifierHandler for CountedItemModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(mut count) = attr.as_u64() else { return };

        count = count.min(self.max_count);
        let label = format!("{}: {}/{}", self.label, count, self.max_count);
        let price = get_buy_price(self.item_id).await;
        value.add(&label, price, count);
    }
}