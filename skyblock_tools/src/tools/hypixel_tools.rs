use std::sync::LazyLock;
use std::time::{Duration, Instant};
use crate::constants::misc::ISLAND_NAMES;
use crate::item_utils::get_pretty_name;
use crate::structs::player_data_structs::{PlayerDataResponse, StringBuilder};
use crate::tools::profile_fetcher::get_profiles_info;
use crate::utils::get_hypixel_api_key;
use chrono::Utc;
use rustc_hash::FxHashMap;
use common::extensions::json_ext::JsonExt;
use common::http::send_http_request;
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::error;

const PLAYER_ENDPOINT: &str = "https://api.hypixel.net/v2/player";
const STATUS_ENDPOINT: &str = "https://api.hypixel.net/v2/status";
const CACHE_DURATION: Duration = Duration::from_secs(60);

static PLAYERS_DATA: LazyLock<RwLock<FxHashMap<String, (Value, Instant)>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

pub async fn clear_expired_cache() {
    let mut cache = PLAYERS_DATA.write().await;
    let now = Instant::now();

    cache.retain(|_uuid, (_player, timestamp)| now.duration_since(*timestamp) < CACHE_DURATION);
}

pub async fn get_player_status(pdr: &PlayerDataResponse, sb: &mut StringBuilder) {
    clear_expired_cache().await;
    let player_uuid = pdr.player_uuid();

    if let Some((cached_player, _)) = PLAYERS_DATA.read().await.get(player_uuid).cloned() {
        process_player(&cached_player, player_uuid, sb).await;
        return;
    }

    let url = format!("{PLAYER_ENDPOINT}?key={}&uuid={player_uuid}", get_hypixel_api_key());
    match send_http_request(&url).await {
        Err(err) => error!(?err, "Failed to get player info"),
        Ok(json) => {
            if let Some(player) = json.get("player") {
                PLAYERS_DATA.write().await.insert(player_uuid.to_string(), (player.clone(), Instant::now()));
                process_player(player, player_uuid, sb).await;
            }
        },
    };

    get_profiles_info(player_uuid, sb).await;
}

async fn process_player(player: &Value, player_uuid: &str, sb: &mut StringBuilder) {
    if let Some(username) = player.get_str("displayname") {
        sb.push(format!("Player: [{}] {}", get_hypixel_rank(username, player), username));

        let last_logout = player
            .get_u64("lastLogout")
            .map(|t| format!("Last Active: {}", format_last_active(t)));

        match get_status(player_uuid).await {
            None => sb.push_option(last_logout),
            Some(status) => {
                if !status.contains("Online") {
                    sb.push(status);
                    sb.push_option(last_logout)
                } else {
                    sb.push(status)
                }
            }
        }
    }
}

async fn get_status(player_uuid: &str) -> Option<String> {
    let url = format!("{STATUS_ENDPOINT}?key={}&uuid={player_uuid}", get_hypixel_api_key());
    let json = match send_http_request(&url).await {
        Ok(json) => json,
        Err(err) => {
            error!(?err, "Failed to get player status");
            return None;
        }
    };

    if json.get_bool("success").unwrap_or_default() && let Some(session) = json.get("session") {
        let online = session.get_bool("online").unwrap_or_default();
        let online_str = if online { "Online" } else { "Offline" };
        let mut str = format!("Status: {online_str}");
        if online && let Some(game_type) = session.get_str("gameType") {
            str.push_str(&format!(" in {}", get_pretty_name(game_type)));

            if let Some(mode) = session.get_str("mode") {
                let mode = ISLAND_NAMES
                    .get(mode)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| get_pretty_name(mode));

                str.push_str(&format!(" {mode}"))
            }
        }
        return Some(str);
    }

    None
}

fn get_hypixel_rank(username: &str, value: &Value) -> String {
    // RIP Technoblade :'(
    if username.to_lowercase() == "technoblade" {
        return "PIG+++".to_owned();
    }

    if let Some(rank) = value.get_str("rank") {
        if rank == "YOUTUBER" || rank == "STAFF" {
            return rank.to_owned();
        }
    }

    if let Some(rank) = value.get_str("monthlyPackageRank") {
        if rank == "SUPERSTAR" {
            return "MVP++".to_owned();
        }
    }

    if let Some(rank) = value.get_str("newPackageRank") {
        return rank.replace("_PLUS", "+");
    }

    "Default".to_owned()
}

fn format_last_active(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let now = Utc::now().timestamp();
    let diff = now - secs;

    let str = match diff {
        d if d < 60 => format!("{d} seconds ago"),
        d if d < 3600 => format!("{} minutes ago", d / 60),
        d if d < 86400 => format!("{} hours ago", d / 3600),
        d if d < 604800 => format!("{} days ago", d / 86400),
        d if d < 2592000 => format!("{} weeks ago", d / 604800),
        d if d < 31536000 => format!("{} months ago", d / 2592000),
        _ => format!("{} years ago", diff / 31536000),
    };

    match str.starts_with("1") {
        true => str.replace("s ", " "),
        false => str,
    }
}