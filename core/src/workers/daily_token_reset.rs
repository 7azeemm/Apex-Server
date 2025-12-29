use std::time::Duration;
use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::time::sleep;
use tracing::{error, info};
use crate::utils::database::get_db_pool;

const RESET_FILE: &str = "daily_reset.json";

#[derive(Serialize, Deserialize)]
struct ResetState {
    last_reset: String, // YYYY-MM-DD (UTC)
}

pub fn schedule() {
    tokio::spawn(async move {
        startup_reset_if_needed().await;
        loop {
            sleep(time_until_midnight()).await;
            reset_tokens().await;
        }
    });
}

async fn load_reset_state() -> Option<ResetState> {
    fs::read_to_string(RESET_FILE).await.ok().and_then(|s| serde_json::from_str(&s).ok())
}

async fn save_reset_state(state: &ResetState) -> std::io::Result<()> {
    let tmp = format!("{}.tmp", RESET_FILE);
    fs::write(&tmp, serde_json::to_string(state).unwrap()).await?;
    fs::rename(tmp, RESET_FILE).await?;
    Ok(())
}

async fn reset_tokens() {
    match sqlx::query!("UPDATE users SET tokens_used_today = 0").execute(get_db_pool()).await {
        Err(error) => error!(?error, "Failed to reset daily tokens"),
        Ok(_) => {
            let today = Utc::now().date_naive().to_string();
            let _ = save_reset_state(&ResetState {
                last_reset: today,
            }).await;
            info!("Daily token reset completed")
        }
    }
}

async fn startup_reset_if_needed() {
    let today = Utc::now().date_naive().to_string();

    let last = load_reset_state().await
        .map(|s| s.last_reset)
        .unwrap_or_default();

    if last != today {
        reset_tokens().await;
    }
}

fn time_until_midnight() -> Duration {
    let now = Utc::now();
    let tomorrow = (now.date_naive() + chrono::Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let diff = tomorrow - now.naive_utc();
    Duration::from_secs(diff.num_seconds().max(0) as u64)
}