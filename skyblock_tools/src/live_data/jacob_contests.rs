use crate::utils::get_time_as_secs;
use common::extensions::json_ext::JsonExt;
use common::http::send_http_request;
use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::{interval_at, Instant};
use tracing::error;

const CONTESTS_ENDPOINT: &str = "https://api.elitebot.dev/contests/at/now";
const THRESHOLD: u64 = 300;

static DATA_WAITER: Notify = Notify::const_new();
static CONTESTS: LazyLock<RwLock<HashMap<String, Vec<String>>>> = LazyLock::new(|| RwLock::new(HashMap::default()));

pub async fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval_at(Instant::now(), Duration::from_secs(THRESHOLD));
        loop {
            ticker.tick().await;
            if let Err(err) = update_contests().await {
                error!(?err, "[Jacob-Contests] Failed to update data");
            }
            DATA_WAITER.notify_waiters()
        }
    });
    DATA_WAITER.notified().await;
}

async fn update_contests() -> Result<(), Box<dyn Error + Send + Sync>> {
    let json = send_http_request(CONTESTS_ENDPOINT).await?;

    let contests = json
        .get_object("contests")
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    let contests_vec = v
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        }).unwrap_or_default();
                    (k.clone(), contests_vec)
                }).collect::<HashMap<String, Vec<String>>>()
        }).unwrap_or_default();

    let mut data = CONTESTS.write().await;
    *data = contests;

    Ok(())
}

pub async fn get_upcoming_contests() -> Vec<(String, Vec<String>)> {
    let data = CONTESTS.read().await;
    let mut upcoming: Vec<_> = data
        .iter()
        .filter_map(|(time_str, contests)| {
            time_str
                .parse::<u64>()
                .ok()
                .filter(|&event_time| event_time > get_time_as_secs())
                .map(|event_time| (event_time, time_str.to_string(), contests.clone()))
        }).collect();

    // Sort by timestamp (earliest first)
    upcoming.sort_by_key(|(timestamp, _, _)| *timestamp);
    upcoming
        .into_iter()
        .take(5)
        .map(|(_, time_str, contests)| (time_str, contests))
        .collect()
}