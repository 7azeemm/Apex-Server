use crate::extensions::json_ext::JsonExt;
use crate::http::send_http_request;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

const NAME_TO_UUID_URL: &str = "https://api.mojang.com/users/profiles/minecraft";
const NAME_TO_UUID_URL_FALLBACK: &str = "https://playerdb.co/api/player/minecraft";
const UUID_TO_NAME_URL: &str = "https://api.minecraftservices.com/minecraft/profile/lookup";
static UUID_CACHE: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| RwLock::new(HashMap::new()));
static NAME_CACHE: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub async fn get_player_uuid(player_name: &str) -> Result<String, Box<dyn Error>> {
    {
        let cache = UUID_CACHE.read().await;
        if let Some(uuid) = cache.get(player_name) {
            return Ok(uuid.clone());
        }
    }

    let url = format!("{NAME_TO_UUID_URL}/{player_name}");
    let json = match send_http_request(&url).await {
        Ok(json) => Some(json),
        Err(err) => match err.downcast_ref::<reqwest::Error>() {
            None => return Err(err),
            Some(reqwest_err) => {
                if reqwest_err.is_timeout() || reqwest_err.is_connect() {
                    match reqwest_err.is_timeout() {
                        true => eprintln!("Mojang uuid lookup request timed out"),
                        false => eprintln!("Mojang uuid lookup connection error: {}", reqwest_err)
                    }
                    let url = format!("{NAME_TO_UUID_URL_FALLBACK}/{player_name}");
                    send_http_request(&url).await?.get("data").and_then(|v| v.get("player")).cloned()
                } else { return Err(err) }
            }
        }
    };

    if let Some(json) = json {
        if let Some(uuid) = json.get_str("id") {
            let mut cache = UUID_CACHE.write().await;
            cache.insert(player_name.to_owned(), uuid.to_owned());
            return Ok(uuid.to_owned());
        }
    }

    Err(format!("Couldn't find any player with name {player_name}").into())
}

pub async fn get_player_username(player_uuid: &str) -> Result<String, Box<dyn Error>> {
    {
        let cache = NAME_CACHE.read().await;
        if let Some(username) = cache.get(player_uuid) {
            return Ok(username.clone());
        }
    }

    let url = format!("{UUID_TO_NAME_URL}/{player_uuid}");
    let json = send_http_request(&url).await?;

    if let Some(username) = json.get_str("name") {
        {
            let mut cache = NAME_CACHE.write().await;
            cache.insert(player_uuid.to_owned(), username.to_owned());
        }
        return Ok(username.to_owned());
    }

    Err(format!("Couldn't find any player with uuid {player_uuid}").into())
}

pub fn get_time_as_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn format_number_with_commas(num: u64) -> String {
    let s = num.to_string();
    let mut result = String::new();

    let chars: Vec<char> = s.chars().rev().collect();
    for (i, c) in chars.iter().enumerate() {
        if i != 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result.chars().rev().collect()
}

pub fn format_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}b", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}m", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }.replace(".0", "").to_string()
}

pub fn strip_formatting(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '§' {
            chars.next();
        } else {
            result.push(ch);
        }
    }

    result
}