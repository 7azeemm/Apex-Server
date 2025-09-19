use crate::extensions::json_ext::JsonExt;
use crate::http::{get_api_key, send_http_request};
use crate::item_utils::get_pretty_name;
use crate::player_data::profile_fetcher::get_profiles_info;
use crate::structs::player_data_structs::{PlayerDataResponse, StringBuilder};
use chrono::Utc;
use serde_json::Value;

const PLAYER_ENDPOINT: &str = "https://api.hypixel.net/v2/player";
const STATUS_ENDPOINT: &str = "https://api.hypixel.net/v2/status";

//TODO: check rank with mvp++ / pig / None
pub async fn get_player_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let player_uuid = pdr.player_uuid();

    match send_http_request(&format!("{PLAYER_ENDPOINT}?key={}&uuid={}", get_api_key(), player_uuid)).await {
        Err(err) => eprintln!("Err: {:?}", err),
        Ok(json) => {
            if let Some(player) = json.get("player") {
                if let Some(username) = player.get_str("displayname") {
                    sb.push(match get_hypixel_rank(username, player) {
                        None => format!("Player: {username}"),
                        Some(rank) => format!("Player: [{rank}] {username}")
                    });

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
        }
    }

    get_profiles_info(player_uuid, &mut sb).await;
    pdr.set_resp(sb)
}

async fn get_player_status(player_uuid: &str) -> Option<String> {
    match send_http_request(&format!("{STATUS_ENDPOINT}?key={}&uuid={}", get_api_key(), player_uuid)).await {
        Err(err) => eprintln!("Err: {:?}", err),
        Ok(json) => {
            if json.get_bool("success").unwrap_or(false) && let Some(session) = json.get("session") {
                let online = session.get_bool("online").unwrap_or(false);
                let online_str = match online {
                    true => "Online",
                    false => "Offline"
                };
                let mut str = format!("Status: {online_str}");
                if online {
                    if let Some(game_type) = session.get_str("gameType") {
                        let pretty_game = get_pretty_name(game_type);
                        str.push_str(&format!(" in {}", pretty_game));
                        if let Some(mode) = session.get_str("mode") {
                            let pretty_mode = get_pretty_name(mode);
                            if !pretty_mode.starts_with(&pretty_game) {
                                str.push_str(&format!(" {}", pretty_mode));
                            } else {
                                let trimmed_mode = pretty_mode.strip_prefix(&pretty_game).unwrap_or(&pretty_mode);
                                let trimmed_mode = trimmed_mode.trim_start();
                                if !trimmed_mode.is_empty() {
                                    str.push_str(&format!(" {}", trimmed_mode));
                                }
                            }
                        }
                    }
                }
                return Some(str);
            }
        }
    }

    eprintln!("Couldn't get player status");
    None
}

fn get_hypixel_rank(username: &str, value: &Value) -> Option<String> {
    // Rest In Peace Techno :'(
    if username.to_lowercase() == "technoblade" { return Some("PIG+++".to_owned()); }

    let candidates = [
        value.get_str("rank"),
        value.get_str("monthlyPackageRank").filter(|s| *s != "NONE"),
        value.get_str("newPackageRank"),
    ];

    candidates.into_iter().flatten().next().map(|s| {
        s.replace("SUPERSTAR", "MVP++").replace("_PLUS", "+")
    })
}

fn format_last_active(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let now = Utc::now().timestamp();
    let diff = now - secs;

    match diff {
        d if d < 60 => format!("{d} seconds ago"),
        d if d < 3600 => format!("{} minutes ago", d / 60),
        d if d < 86400 => format!("{} hours ago", d / 3600),
        d if d < 604800 => format!("{} days ago", d / 86400),
        d if d < 2592000 => format!("{} weeks ago", d / 604800),
        d if d < 31536000 => format!("{} months ago", d / 2592000),
        _ => format!("{} years ago", diff / 31536000),
    }
}