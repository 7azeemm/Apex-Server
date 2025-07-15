use std::cmp::{max, min};
use std::collections::HashMap;
use async_trait::async_trait;
use fastnbt::Value;
use phf::{phf_map, phf_set, Map, Set};
use tokio::sync::RwLock;
use crate::auctions::get_shared_lowest_bin;
use crate::bazaar::get_item_shared_price;
use crate::item_value_calculator::ModifierHandler;
use crate::structs::{ItemValue, Modifier, PriceDataSource, SharedPriceData};

pub struct UpgradeLevelModifier;

pub static REGULAR_STARS: Map<&'static str, i32> = phf_map! { // TODO: real prices
    "1" => 10_000,
    "2" => 50_000,
    "3" => 150_000,
    "4" => 300_000,
    "5" => 600_000,
};

pub static MASTER_STARS: Map<&'static str, &'static str> = phf_map! {
    "1" => "FIRST_MASTER_STAR",
    "2" => "SECOND_MASTER_STAR",
    "3" => "THIRD_MASTER_STAR",
    "4" => "FOURTH_MASTER_STAR",
    "5" => "FIFTH_MASTER_STAR",
};

pub static SKIP_FOR_NOW: Set<&'static str> = phf_set! { //TODO: crimson essence
    "CRIMSON",
    "AURORA",
    "TERROR",
    "HOLLOW",
    "FERVOR",
    "MOLTEN",
    "MAGMA_ROD",
    "INFERNO_ROD",
    "HELLFIRE_ROD",
};

#[async_trait]
impl ModifierHandler for UpgradeLevelModifier {
    async fn calculate_value(&self, item_id: &str, modifier: &Value, item_value: &mut ItemValue) -> bool {
        let Value::Int(level) = modifier else { return false };
        for skip in SKIP_FOR_NOW.iter() {
            if item_id.contains(skip) {
                return true;
            }
        }

        let regular_stars = min(5, *level);
        let master_stars = max(0, *level - 5);
        let mut regular_stars_value = 0.0;

        for (star, cost) in REGULAR_STARS.entries() {
            if let Ok(star_num) = star.parse::<i32>() {
                if star_num <= regular_stars {
                    regular_stars_value += (*cost as f64);
                }
            }
        }

        if regular_stars_value != 0.0 {
            let shared_price = SharedPriceData::new(RwLock::new(PriceDataSource::NPC { price: regular_stars_value }));
            item_value.add_modifier(&format!("{regular_stars}_STARS"), Modifier::new_one(shared_price));
        }

        for (star, id) in MASTER_STARS.entries() {
            if let Ok(star_num) = star.parse::<i32>() {
                if star_num <= master_stars {
                    if let Some(shared_price) = get_item_shared_price(&id).await {
                        item_value.add_modifier(&id, Modifier::new_one(shared_price));
                    }
                }
            }
        }

        true
    }
}