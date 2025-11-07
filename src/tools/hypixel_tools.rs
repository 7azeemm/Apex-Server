use crate::constants::misc::ISLAND_NAMES;
use crate::extensions::json_ext::JsonExt;
use crate::http::{get_api_key, send_http_request};
use crate::item_utils::get_pretty_name;
use crate::structs::player_data_structs::{PlayerDataResponse, StringBuilder};
use crate::tools::profile_fetcher::get_profiles_info;
use chrono::Utc;
use serde_json::Value;

const PLAYER_ENDPOINT: &str = "https://api.hypixel.net/v2/player";
const STATUS_ENDPOINT: &str = "https://api.hypixel.net/v2/status";

pub async fn get_player_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let player_uuid = pdr.player_uuid();

    let url = format!("{PLAYER_ENDPOINT}?key={}&uuid={}", get_api_key(), player_uuid);
    let json = match send_http_request(&url).await {
        Ok(json) => json,
        Err(err) => {
            eprintln!("Err: {:?}", err);
            return
        }
    };

    if let Some(player) = json.get("player") {
        if let Some(username) = player.get_str("displayname") {
            sb.push(format!("Player: [{}] {username}", get_hypixel_rank(username, player)));

            let last_logout = player.get_u64("lastLogout").map(|t| format!("Last Active: {}", format_last_active(t)));
            match get_player_status(player_uuid).await {
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

    get_profiles_info(player_uuid, &mut sb).await;

    pdr.set_sb(sb)
}

async fn get_player_status(player_uuid: &str) -> Option<String> {
    let url = format!("{STATUS_ENDPOINT}?key={}&uuid={}", get_api_key(), player_uuid);

    let json = match send_http_request(&url).await {
        Ok(json) => json,
        Err(err) => {
            eprintln!("Err: {:?}", err);
            return None;
        }
    };

    if json.get_bool("success").unwrap_or_default() && let Some(session) = json.get("session") {
        let online = session.get_bool("online").unwrap_or_default();
        let online_str = if online { "Online" } else { "Offline" };
        let mut str = format!("Status: {online_str}");
        if online {
            if let Some(game_type) = session.get_str("gameType") {
                let island_name = ISLAND_NAMES.get(game_type).map(|s| s.to_string()).unwrap_or_else(|| get_pretty_name(game_type));
                str.push_str(&format!(" in {}", island_name));

                if let Some(mode) = session.get_str("mode") {
                    let pretty_mode = get_pretty_name(mode);
                    if !pretty_mode.starts_with(&island_name) {
                        str.push_str(&format!(" {}", pretty_mode));
                    } else {
                        let trimmed_mode = pretty_mode.strip_prefix(&island_name).unwrap_or(&pretty_mode).trim_start();
                        if !trimmed_mode.is_empty() {
                            str.push_str(&format!(" {}", trimmed_mode));
                        }
                    }
                }
            }
        }
        return Some(str);
    }

    None
}

fn get_hypixel_rank(username: &str, value: &Value) -> String {
    // RIP Technoblade :'(
    if username.to_lowercase() == "technoblade" { return "PIG+++".to_owned() }

    if let Some(rank) = value.get_str("rank") {
        if rank == "YOUTUBER" || rank == "STAFF" { return rank.to_owned() }
    }

    if let Some(rank) = value.get_str("monthlyPackageRank") {
        if rank == "SUPERSTAR" { return "MVP++".to_owned() }
    }

    if let Some(rank) = value.get_str("newPackageRank") {
        return rank.replace("_PLUS", "+")
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
        false => str
    }
}