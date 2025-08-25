use std::cmp::max;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use sea_orm::Iden;
use tokio::sync::{Notify, RwLock};
use tokio::time::{interval, interval_at, Instant};
use crate::statics::HTTP_CLIENT;
use crate::structs::{BazaarResponse, PriceDataSource, SharedPriceData};
use crate::structs::PriceDataSource::Bazaar;

const API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/bazaar";
const THRESHOLD: u64 = 70;
const MIN_DELAY_SECS: u64 = 20;

pub static BAZAAR: LazyLock<RwLock<FxHashMap<String, SharedPriceData>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));
pub static BAZAAR_READY: Notify = Notify::const_new();

pub fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval(Duration::from_secs(THRESHOLD));
        loop {
            ticker.tick().await;
            match update().await {
                Ok(last_updated) => {
                    let next_update_time = (last_updated / 1000) + THRESHOLD;
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    let delay = Duration::from_secs(match next_update_time > now {
                        true => max(next_update_time - now, MIN_DELAY_SECS),
                        false => MIN_DELAY_SECS,
                    });

                    BAZAAR_READY.notify_waiters();
                    ticker = interval_at(Instant::now() + delay, Duration::from_secs(THRESHOLD));
                    println!("[Bazaar] Next update in {:.1} seconds", delay.as_secs());
                },
                Err(err) => {
                    eprintln!("[Bazaar] Error: {:?}", err);
                    ticker = interval_at(Instant::now() + Duration::from_secs(MIN_DELAY_SECS), Duration::from_secs(THRESHOLD));
                }
            }
        }
    });
}

async fn update() -> Result<u64, Box<dyn std::error::Error>> {
    // println!("Starting Bazaar update...");
    let total_start = Instant::now();

    let network_start = Instant::now();
    let resp = HTTP_CLIENT.get(API_ENDPOINT).send().await?;
    // println!("Network (headers): {:.2?}", network_start.elapsed());

    let download_start = Instant::now();
    let text = resp.text().await?;
    // println!("Download (body): {:.2?}", download_start.elapsed());
    // println!("Response size: {:.2} Mbs", text.len() as f32 / 1_000_000.0);

    let parse_start = Instant::now();
    let bazaar_response: BazaarResponse = serde_json::from_str(&text)?;
    // println!("Parse: {:.2?}", parse_start.elapsed());

    if !bazaar_response.is_successful() {
        return Err("Bazaar API Request was unsuccessful".into());
    }

    let process_start = Instant::now();
    for (id, data) in bazaar_response.get_products() {
        if data.buy_price() == 0.0 { continue; }
        let bazaar = BAZAAR.read().await;
        if let Some(price_ref) = bazaar.get(id) {
            let mut price = price_ref.write().await;
            *price = Bazaar {
                buy_price: data.buy_price(),
                sell_price: data.sell_price(),
            };
        } else {
            drop(bazaar);
            let mut bazaar = BAZAAR.write().await;
            bazaar.insert(
                id.parse().unwrap(),
                SharedPriceData::new(RwLock::new(Bazaar {
                    buy_price: data.buy_price(),
                    sell_price: data.sell_price(),
                }))
            );
        }
    }

    Ok(bazaar_response.last_updated())
}

pub async fn get_item_price(id: &str) -> Option<f64> {
    let bazaar_map = BAZAAR.read().await;
    
    let value = match bazaar_map.get(id) {
        Some(price_data) => price_data,
        None => {
            // println!("Couldn't find bazaar price of {id}");
            return None
        }
    };

    match value.read().await.deref() {
        Bazaar { buy_price, .. } => Some(*buy_price),
        _ => {
            println!("{id} price is registered as LowestBIN?");
            None
        }
    }
}

pub async fn get_item_shared_price(id: &str) -> Option<SharedPriceData> {
    let bazaar_map = BAZAAR.read().await;
    bazaar_map.get(id).cloned()
}