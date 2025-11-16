use std::time::Duration;
use chrono::Local;
use tokio::time::sleep;
use tracing::{error, info};
use crate::utils::database::get_db_pool;

pub fn schedule() {
    tokio::spawn(async move {
        loop {
            sleep(time_until_midnight()).await;

            match sqlx::query!("UPDATE users SET tokens_used_today = 0").execute(get_db_pool()).await {
                Err(error) => error!(?error, "Failed to reset daily tokens"),
                Ok(_) => info!("Daily token reset completed")
            }
        }
    });
}

fn time_until_midnight() -> Duration {
    let now = Local::now();

    let next_midnight = (now + chrono::Duration::days(1))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .single()
        .unwrap();

    let diff = next_midnight - now;
    Duration::from_secs(diff.num_seconds() as u64)
}