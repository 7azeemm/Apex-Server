use common::http::send_http_request;
use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::{interval_at, Instant};
use tracing::error;

const ENDPOINT: &str = "https://raw.githubusercontent.com/SkyHelperBot/Prices/main/pricesV2.json";
const THRESHOLD: u64 = 300;

static DATA_WAITER: Notify = Notify::const_new();
static DATA: LazyLock<RwLock<FxHashMap<String, u64>>> = LazyLock::new(|| RwLock::new(HashMap::default()));

pub async fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval_at(Instant::now(), Duration::from_secs(THRESHOLD));
        loop {
            ticker.tick().await;
            if let Err(err) = update().await {
                error!(?err, "[Cosmetic-Prices] Failed to update cosmetic prices");
            }
            DATA_WAITER.notify_waiters()
        }
    });
    DATA_WAITER.notified().await;
}

async fn update() -> Result<(), Box<dyn Error + Send + Sync>> {
    let json = send_http_request(ENDPOINT).await?;

    if let Some(map) = json.as_object() {
        let mut data = DATA.write().await;
        data.clear();
        data.extend(map.iter().filter_map(|(k, v)| {
            v.as_u64()
                .or_else(|| v.as_f64().map(|f| f as u64))
                .map(|val| (k.clone(), val))
        }));
    }

    Ok(())
}

pub async fn get_cosmetic_price(id: &str) -> Option<u64> {
    DATA.read().await.get(id).copied()
}