use crate::extensions::json_ext::JsonExt;
use crate::http::send_http_request;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::error;

const NAME_TO_UUID_URL: &str = "https://api.mojang.com/users/profiles/minecraft";
const UUID_TO_NAME_URL: &str = "https://api.minecraftservices.com/minecraft/profile/lookup";
const FALLBACK_URL: &str = "https://playerdb.co/api/player/minecraft";

static UUID_CACHE: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| RwLock::new(HashMap::new()));
static NAME_CACHE: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub async fn get_player_uuid(player_name: &str) -> Option<String> {
    {
        let cache = UUID_CACHE.read().await;
        if let Some(uuid) = cache.get(player_name) {
            return Some(uuid.clone());
        }
    }

    let url = format!("{NAME_TO_UUID_URL}/{player_name}");
    let id = match send_http_request(&url).await.map(|s| s.get_str("id").map(|s| s.to_owned())) {
        Ok(Some(id)) => Some(id),
        _ => match send_http_request(&format!("{FALLBACK_URL}/{player_name}")).await {
            Ok(json) => json.get_str("data/player/id").map(|s| s.to_owned()),
            Err(error) => {
                error!(error, "Player UUID fetch failed");
                return None;
            }
        },
    };

    if let Some(id) = id {
        let mut cache = UUID_CACHE.write().await;
        cache.insert(player_name.to_string(), id.to_owned());
        return Some(id.to_owned());
    }

    None
}

pub async fn get_player_username(player_uuid: &str) -> Option<String> {
    {
        let cache = NAME_CACHE.read().await;
        if let Some(username) = cache.get(player_uuid) {
            return Some(username.clone());
        }
    }

    let url = format!("{UUID_TO_NAME_URL}/{player_uuid}");
    let username = match send_http_request(&url).await.map(|v| v.get_str("name").map(|s| s.to_owned())) {
        Ok(Some(username)) => Some(username),
        _ => match send_http_request(&format!("{FALLBACK_URL}/{player_uuid}")).await {
            Ok(json) => json.get_str("data/player/username").map(|s| s.to_owned()),
            Err(error) => {
                error!(error, "Player Username fetch failed");
                return None;
            }
        },
    };

    if let Some(username) = username {
        let mut cache = NAME_CACHE.write().await;
        cache.insert(player_uuid.to_owned(), username.to_owned());
        return Some(username.to_owned());
    }

    None
}