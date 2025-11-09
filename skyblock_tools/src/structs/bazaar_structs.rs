use derive_new::new;
use getset::Getters;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Getters)]
#[getset(get = "pub")]
pub struct BazaarResponse {
    success: bool,
    #[serde(rename = "lastUpdated")]
    last_updated: u64,
    products: HashMap<String, Product>,
}

#[derive(Deserialize, Debug)]
pub struct Product {
    #[serde(rename = "quick_status")]
    quick_status: PriceData,
}

impl Product {
    pub fn sell_price(&self) -> f64 {
        self.quick_status.sell_price
    }
    pub fn buy_price(&self) -> f64 {
        self.quick_status.buy_price
    }
}

#[derive(Deserialize, Debug, new, Getters)]
#[getset(get = "pub")]
pub struct PriceData {
    #[serde(rename = "sellPrice")]
    sell_price: f64,
    #[serde(rename = "buyPrice")]
    buy_price: f64,
}
