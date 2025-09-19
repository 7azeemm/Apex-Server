use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct PriceData {
    #[serde(rename = "sellPrice")]
    sell_price: f64,
    #[serde(rename = "buyPrice")]
    buy_price: f64,
}

impl BazaarResponse {
    pub fn is_successful(&self) -> bool { self.success }
    pub fn last_updated(&self) -> u64 { self.last_updated }
    pub fn get_products(self) -> HashMap<String, Product> { self.products }
    pub fn get_products_len(&self) -> usize { self.products.len() }
}

impl Product {
    pub fn sell_price(&self) -> f64 { self.quick_status.sell_price }
    pub fn buy_price(&self) -> f64 { self.quick_status.buy_price }
}

impl PriceData {
    pub fn new(buy_price: f64, sell_price: f64) -> Self {
        Self { buy_price, sell_price }
    }
    pub fn get_buy_price(&self) -> f64 { self.buy_price }
    pub fn get_sell_price(&self) -> f64 { self.sell_price }
}