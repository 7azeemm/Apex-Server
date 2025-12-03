use getset::Getters;
use serde::Deserialize;
use std::collections::HashMap;
use serde_json::Value;

#[derive(Deserialize, Debug, Getters)]
#[getset(get = "pub")]
pub struct BazaarResponse {
    success: bool,
    #[serde(rename = "lastUpdated")]
    last_updated: u64,
    products: HashMap<String, Product>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Product {
    #[serde(rename = "quick_status")]
    quick_status: QuickStatus,
    sell_summary: Vec<Value>,
    buy_summary: Vec<Value>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct QuickStatus {
    #[serde(rename = "sellPrice")]
    sell_price: f64,
    #[serde(rename = "buyPrice")]
    buy_price: f64,
    #[serde(rename = "sellMovingWeek")]
    sell_moving_week: u64,
    #[serde(rename = "buyMovingWeek")]
    buy_moving_week: u64,
}

impl Product {
    pub fn sell_summary(&self) -> &Vec<Value> {
        &self.sell_summary
    }

    pub fn buy_summary(&self) -> &Vec<Value> {
        &self.buy_summary
    }

    pub fn sell_price(&self) -> f64 {
        self.quick_status.sell_price
    }
    pub fn buy_price(&self) -> f64 {
        self.quick_status.buy_price
    }
    pub fn sell_moving_week(&self) -> u64 {
        self.quick_status.sell_moving_week
    }
    pub fn buy_moving_week(&self) -> u64 {
        self.quick_status.buy_moving_week
    }
}