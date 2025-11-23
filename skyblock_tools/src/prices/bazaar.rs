use crate::structs::bazaar_structs::{BazaarResponse, PriceData};
use crate::utils::get_time_as_secs;
use common::http::send_raw_http_request;
use rustc_hash::FxHashMap;
use std::cmp::max;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::{interval, interval_at, Instant};
use tracing::{error, info};

const API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/bazaar";
const THRESHOLD: u64 = 70;
const MIN_DELAY_SECS: u64 = 20;

static BAZAAR: LazyLock<RwLock<FxHashMap<String, PriceData>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));
static DATA_WAITER: Notify = Notify::const_new();

pub async fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval(Duration::from_secs(THRESHOLD));
        loop {
            ticker.tick().await;
            match update().await {
                Ok(last_updated) => {
                    let next_update_time = (last_updated / 1000) + THRESHOLD;
                    let now = get_time_as_secs();

                    let delay = Duration::from_secs(match next_update_time > now {
                        true => max(next_update_time - now, MIN_DELAY_SECS),
                        false => MIN_DELAY_SECS,
                    });

                    ticker = interval_at(Instant::now() + delay, Duration::from_secs(THRESHOLD));
                    DATA_WAITER.notify_waiters();
                }
                Err(err) => {
                    error!(?err, "[Bazaar] Failed to update bazaar items");
                    ticker = interval_at(
                        Instant::now() + Duration::from_secs(MIN_DELAY_SECS),
                        Duration::from_secs(THRESHOLD),
                    );
                }
            }
        }
    });
    DATA_WAITER.notified().await;
}

async fn update() -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let resp = send_raw_http_request(API_ENDPOINT).await?;
    let bazaar_response: BazaarResponse = serde_json::from_str(&resp)?;

    if !*bazaar_response.success() {
        return Err("API Request was unsuccessful".into());
    }

    let products = bazaar_response.products();
    let &last_updated = bazaar_response.last_updated();

    let mut bazaar = BAZAAR.write().await;
    for (id, data) in products {
        bazaar.insert(id.clone(), PriceData::new(data.buy_price(), data.sell_price()));
    }

    Ok(last_updated)
}

pub async fn get_buy_price(id: &str) -> Option<u64> {
    BAZAAR.read().await.get(id).map(|p| *p.buy_price() as u64)
}

pub async fn get_price(id: &str) -> Option<(u64, u64)> {
    BAZAAR.read().await.get(id).map(|p| (*p.buy_price() as u64, *p.sell_price() as u64))
}