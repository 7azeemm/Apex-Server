use crate::structs::mayor_info_structs::{Mayor, MayorInfo, SBDate};
use chrono::{DateTime, Utc};
use common::extensions::json_ext::JsonExt;
use common::http::send_http_request;
use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::{interval_at, Instant};

const MAYOR_ENDPOINT: &str = "https://api.hypixel.net/v2/resources/skyblock/election";
const THRESHOLD: u64 = 300;
const REAL_HOURS_PER_SB_YEAR: i64 = 124;
const ELECTION_OFFSET_HOURS: i64 = 29;
const ELECTION_OFFSET_MINS: i64 = 20;
const SB_START_TIMESTAMP: i64 = 1560275700; // Hypixel SkyBlock release timestamp (2019-06-11 11:15 UTC)

static DATA_WAITER: Notify = Notify::const_new();
static MAYOR_INFO: LazyLock<RwLock<MayorInfo>> = LazyLock::new(|| RwLock::new(MayorInfo::default()));

pub async fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval_at(Instant::now(), Duration::from_secs(THRESHOLD));
        loop {
            ticker.tick().await;
            match update_mayors_info().await {
                Ok(()) => println!("[Mayor-Info] Next update in {} seconds", THRESHOLD),
                Err(err) => eprintln!("[Mayor-Info] Error: {:?}", err),
            }
            DATA_WAITER.notify_waiters()
        }
    });
    DATA_WAITER.notified().await;
}

async fn update_mayors_info() -> Result<(), Box<dyn Error + Send + Sync>> {
    let json = send_http_request(MAYOR_ENDPOINT).await?;

    if !json.get_bool("success").unwrap_or_default() {
        return Err("[Mayor-Info] API Request was not successful".into());
    }

    let mayor_data = &json.get("mayor");

    // Parse current mayor
    let mayor = Mayor::new(
        mayor_data.get_str("name").unwrap_or("Unknown").to_owned(),
        parse_perks(mayor_data.get_array("perks")),
    );

    // Parse minister
    let minister = mayor_data.get_object("minister").map(|minister_data| {
        let mut perks = HashMap::new();
        if let Some(perk) = minister_data.get("perk").and_then(|v| v.as_object()) {
            if let (Some(name), Some(desc)) = (perk.get("name"), perk.get("description")) {
                perks.insert(
                    name.as_str().unwrap_or("Unknown").to_owned(),
                    desc.as_str().unwrap_or("Unknown").to_owned(),
                );
            }
        }

        Mayor::new(
            minister_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_owned(),
            perks,
        )
    });

    // Parse election candidates
    let election = json
        .get_object("current")
        .and_then(|election_data| election_data.get("candidates").and_then(|v| v.as_array()))
        .map(|candidates| {
            let mut candidates_vec: Vec<(Mayor, Option<u64>)> = candidates
                .iter()
                .filter_map(|candidate| {
                    let votes = candidate.get_u64("votes");
                    let name = candidate.get_str("name")?.to_string();
                    let perks = parse_perks(candidate.get_array("perks"));

                    Some((Mayor::new(name, perks), votes))
                }).collect();

            // Sort by votes (highest first)
            candidates_vec.sort_by(|a, b| b.1.cmp(&a.1));
            candidates_vec
        });

    let mut data = MAYOR_INFO.write().await;
    data.update(mayor, minister, election);

    Ok(())
}

fn parse_perks(perks_array: Option<&Vec<serde_json::Value>>) -> HashMap<String, String> {
    perks_array
        .map(|perks| {
            perks
                .iter()
                .filter_map(|perk| {
                    let name = perk.get_str("name")?;
                    let description = perk.get_str("description")?;
                    Some((name.to_string(), description.to_string()))
                }).collect()
        }).unwrap_or_default()
}

pub async fn get_mayor_info() -> MayorInfo {
    MAYOR_INFO.read().await.clone()
}

pub fn get_skyblock_date() -> String {
    let sb_date = get_sb_time();

    format!(
        "Year {} Month {} Day {} {:02}:{:02}:{:02}",
        sb_date.get_year(),
        sb_date.get_month(),
        sb_date.get_day(),
        sb_date.get_hour(),
        sb_date.get_min(),
        sb_date.get_sec()
    )
}

pub fn get_election_over_time_left() -> String {
    let sb_time = get_sb_time();
    let mut sb_year = sb_time.get_year();
    if (sb_time.get_month(), sb_time.get_day(), sb_time.get_hour()) >= (3, 27, 12) {
        sb_year += 1;
    }

    let election_time = get_election_time(sb_year);

    let duration = chrono::Duration::seconds(election_time - Utc::now().timestamp());
    format!(
        "Election Over in {}d {}h {}m {}s",
        duration.num_days(),
        duration.num_hours() % 24,
        duration.num_minutes() % 60,
        duration.num_seconds() % 60
    )
}

pub fn get_special_mayors_info() -> String {
    let sb_date = get_sb_time();
    let sb_year = sb_date.get_year();

    let mut infos: Vec<(i64, String)> = ["Derpy", "Jerry", "Scorpius"]
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            let (year, dur) = next_special_mayor(Utc::now(), sb_year, idx as i64);
            let (d, h, m, s) = (
                dur.num_days(),
                dur.num_hours() % 24,
                dur.num_minutes() % 60,
                dur.num_seconds() % 60,
            );
            (year, format!("- Next {name} -> Year {year} in ~{d}d {h}h {m}m {s}s"))
        }).collect();

    infos.sort_by_key(|(year, _)| *year);
    infos
        .iter()
        .map(|(_, s)| s.clone())
        .collect::<Vec<String>>()
        .join("\n")
}

fn next_special_mayor_year(current: i64, mayor_index: i64) -> i64 {
    let rem = current % 8;
    let y0 = current + ((8 - rem) % 8);
    let k0 = y0 / 8;

    let offset = (mayor_index - ((k0 + 2) % 3) + 3) % 3;
    let k = k0 + offset;
    k * 8
}

fn get_sb_time() -> SBDate {
    let now = Utc::now();
    let elapsed = now.timestamp() - SB_START_TIMESTAMP;

    let sb_years = elapsed / (REAL_HOURS_PER_SB_YEAR * 3600);
    let sb_year_progress = elapsed % (REAL_HOURS_PER_SB_YEAR * 3600);

    let sb_days_total = sb_year_progress / 1000;

    let year = sb_years;
    let month = (sb_days_total / 31) + 1;
    let day = (sb_days_total % 31) + 1;

    let sb_seconds_in_day = sb_year_progress % 1000;
    let hour = sb_seconds_in_day * 24 / 1000;
    let min = (sb_seconds_in_day * 1440 / 1000) % 60;
    let sec = (sb_seconds_in_day * 86400 / 1000) % 60;

    SBDate::new(year, month, day, hour, min, sec)
}

fn next_special_mayor(now: DateTime<Utc>, sb_year: i64, idx: i64) -> (i64, chrono::Duration) {
    let rem = sb_year % 8;
    let y0 = sb_year + ((8 - rem) % 8);
    let k0 = y0 / 8;

    let offset = (idx - ((k0 + 2) % 3) + 3) % 3;
    let k = k0 + offset;
    let next_year = k * 8;

    let election_time = get_election_time(next_year);
    let duration = chrono::Duration::seconds(election_time - now.timestamp());
    (next_year, duration)
}

fn get_election_time(year: i64) -> i64 {
    let year_start = SB_START_TIMESTAMP + year * REAL_HOURS_PER_SB_YEAR * 3600;
    year_start + (ELECTION_OFFSET_HOURS * 3600) + (ELECTION_OFFSET_MINS * 60)
}
