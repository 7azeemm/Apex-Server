use crate::api::users::get_or_create_user;
use crate::structs::auth_structs::{Session, TokenRequest, UserInfo};
use crate::structs::user::User;
use crate::utils::validated_json::ValidatedJson;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use common::http::send_http_request;
use common::player_fetcher::get_player_username;
use once_cell::sync::Lazy;
use rsa::pkcs8::DecodePublicKey;
use rsa::sha2::Sha256;
use rsa::signature::digest::Digest;
use rsa::{Pkcs1v15Sign, RsaPublicKey};
use serde_json::{json, Value};
use sha1::Sha1;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use semver::Version;
use tokio::sync::RwLock;
use tokio::time::{interval_at, Instant};
use tracing::{error, info};
use uuid::Uuid;
use crate::constants::{CONTACTS, MAINTENANCE, MIN_VERSION};
use crate::structs::api_structs::ApiResponse;

const MOJANG_KEYS_ENDPOINT: &str = "https://api.minecraftservices.com/publickeys";
const TOKEN_LIFESPAN: i64 = 310;
const CHECK_EXPIRED_TOKENS_THRESHOLD: u64 = 5;
const MOJANG_KEYS_FETCH_THRESHOLD: u64 = 30 * 60;

static SESSIONS: Lazy<RwLock<HashMap<String, Arc<RwLock<Session>>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
static MOJANG_KEYS: Lazy<RwLock<Vec<Vec<u8>>>> = Lazy::new(|| RwLock::new(Vec::new()));

static PENDING_NOTIFICATIONS: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval_at(Instant::now(), std::time::Duration::from_secs(CHECK_EXPIRED_TOKENS_THRESHOLD));
        loop {
            ticker.tick().await;
            remove_expired_profile().await;
        }
    });

    tokio::spawn(async {
        let mut ticker = interval_at(Instant::now(), std::time::Duration::from_secs(MOJANG_KEYS_FETCH_THRESHOLD));
        loop {
            if let Err(error) = fetch_mojang_keys().await {
                error!(error, "Fetching Mojang Keys failed");
            }
            ticker.tick().await;
        }
    });
}

async fn remove_expired_profile() {
    let expired: Vec<String> = {
        let sessions = SESSIONS.read().await;
        let mut to_remove = Vec::new();

        for (token, session_arc) in sessions.iter() {
            let session = session_arc.read().await;
            if session.is_expired() {
                to_remove.push(token.to_owned());
            }
        }

        to_remove
    };

    if !expired.is_empty() {
        let mut sessions = SESSIONS.write().await;
        for token in expired {
            sessions.remove(&token);
        }
    }
}

async fn fetch_mojang_keys() -> Result<(), Box<dyn Error + Send + Sync>> {
    let json = send_http_request(MOJANG_KEYS_ENDPOINT).await?;
    let player_certificate_keys = json
        .get("playerCertificateKeys")
        .ok_or("Mojang Public keys does not contain player certificate keys!")?
        .as_array()
        .ok_or("Player Certificate Keys are not in an array")?;

    let mut keys = Vec::new();
    for k in player_certificate_keys {
        if let Some(Value::String(pk_str)) = k.get("publicKey") {
            if let Ok(decoded) = STANDARD.decode(pk_str) {
                keys.push(decoded);
            }
        }
    }

    if keys.is_empty() {
        return Err("Player Certificate Keys list is empty".into());
    }

    let mut mojang_keys = MOJANG_KEYS.write().await;
    *mojang_keys = keys;

    Ok(())
}

pub async fn auth(ValidatedJson(request): ValidatedJson<TokenRequest>) -> Response {
    let key_pair = request.key_pair();
    let player_uuid = key_pair.uuid().to_string();

    let mut error_context = request.context();

    let min_version = Version::parse(MIN_VERSION).unwrap();
    let Ok(mod_version) = Version::parse(request.mod_version()) else {
        return ApiResponse::err_and_log(
            "Invalid mod version",
            StatusCode::BAD_REQUEST,
            format!("version: {}", request.mod_version()),
            &error_context
        );
    };

    if mod_version < min_version {
        return ApiResponse::err("Unsupported mod version, please update the mod.", StatusCode::UNAUTHORIZED);
    }

    let player_name = match get_player_username(&player_uuid).await {
        Some(name) => name,
        None => return ApiResponse::err_and_log(
            "Failed to auth",
            StatusCode::BAD_REQUEST,
            "Couldn't get player username",
            &error_context,
        )
    };

    error_context.insert(1, ("player_name", player_name.clone()));
    let cert_keys = MOJANG_KEYS.read().await.clone();

    let is_key_valid = verify_player_public_key(
        key_pair.public_key(),
        key_pair.public_key_signature(),
        key_pair.uuid(),
        *key_pair.expires_at(),
        &cert_keys,
    );

    // if let Err(err) = is_key_valid {
    //     return ApiResponse::err_and_log(
    //         "Failed to auth",
    //         StatusCode::UNAUTHORIZED,
    //         format!("Failed to verify player public key: {}", err),
    //         &error_context
    //     );
    // }

    let signed_data = request.signed_data();
    let owns_private = verify_client_signature(
        key_pair.public_key(),
        signed_data.original(),
        signed_data.signed(),
    );

    // if let Err(err) = owns_private {
    //     return ApiResponse::err_and_log(
    //         "Failed to auth",
    //         StatusCode::UNAUTHORIZED,
    //         format!("Client did not sign challenge correctly: {err}"),
    //         &error_context
    //     );
    // }

    match get_or_create_user(player_uuid.clone(), player_name.clone()).await {
        Err((api_err, sys_err)) => ApiResponse::internal_err(api_err, sys_err, &error_context),
        Ok(user) => {
            let minecraft_version = request.minecraft_version().to_owned();
            let mod_version = request.mod_version().to_owned();
            let contacts: HashMap<&str, &str> = CONTACTS.iter().copied().collect();

            let user_info = UserInfo::from_user(&user);
            let token = generate_token(user, minecraft_version, mod_version).await;

            let mut json = json!({
                "token": token,
                "user_info": user_info,
                "maintenance": MAINTENANCE,
                "contacts": contacts,
            });

            if let Some(notification) = PENDING_NOTIFICATIONS.write().await.remove(&player_uuid) {
                json["notification"] = Value::String(notification.clone());
            }

            ApiResponse::ok(json)
        }
    }
}

fn verify_player_public_key(
    player_public_key_der: &[u8],
    signature: &[u8],
    uuid: &Uuid,
    expires_at: i64,
    mojang_root_keys_der: &[Vec<u8>],
) -> Result<(), String> {
    // Validate inputs
    if player_public_key_der.len() < 270 || player_public_key_der.len() > 300 {
        return Err(format!("Invalid player public key length: {}", player_public_key_der.len()));
    }
    if signature.len() != 512 {
        return Err(format!("Invalid signature length: {}", signature.len()));
    }
    if mojang_root_keys_der.is_empty() {
        return Err("No Mojang root keys provided".to_owned());
    }

    // Check expiration
    let expires_at_date = DateTime::<Utc>::from_timestamp_millis(expires_at)
        .ok_or_else(|| "Invalid expires_at timestamp".to_owned())?;
    if expires_at_date < Utc::now() {
        return Err(format!("Key expired at {}", expires_at_date));
    }

    // Construct signed data: UUID (16 bytes) + expiresAt (8 bytes) + X.509 DER public key
    let mut signed_data = Vec::with_capacity(16 + 8 + player_public_key_der.len());
    signed_data.extend_from_slice(uuid.as_bytes());
    signed_data.extend_from_slice(&expires_at.to_be_bytes());
    signed_data.extend_from_slice(player_public_key_der);

    // Try each Mojang key
    for (i, root_der) in mojang_root_keys_der.iter().enumerate() {
        let root_pub = RsaPublicKey::from_public_key_der(root_der)
            .map_err(|e| format!("Failed to parse Mojang key {}: {}", i, e))?;

        if verify_signature::<Sha1>(&root_pub, &signed_data, signature, "Mojang key").is_ok() {
            return Ok(());
        }
    }

    Err("No Mojang key could verify the signature".to_owned())
}

fn verify_client_signature(player_public_key_der: &[u8], challenge: &[u8], signed_challenge: &[u8]) -> Result<(), String> {
    // Validate inputs
    if challenge.len() != 16 {
        return Err(format!("Invalid challenge length: {}", challenge.len()));
    }
    if signed_challenge.len() != 256 {
        return Err(format!("Invalid signed challenge length: {}", signed_challenge.len()));
    }

    // Parse public key
    let player_pub = RsaPublicKey::from_public_key_der(player_public_key_der)
        .map_err(|e| format!("Failed to parse player public key: {}", e))?;

    verify_signature::<Sha256>(&player_pub, challenge, signed_challenge, "client signature")
}

fn verify_signature<T: Digest + rsa::pkcs8::AssociatedOid>(public_key: &RsaPublicKey, data: &[u8], signature: &[u8], algo_name: &str) -> Result<(), String> {
    let mut hasher = T::new();
    hasher.update(data);
    let hashed_data = hasher.finalize();

    let padding = Pkcs1v15Sign::new::<T>();
    public_key
        .verify(padding, &hashed_data, signature)
        .map_err(|e| format!("Signature verification failed for {}: {}", algo_name, e))?;

    Ok(())
}

pub async fn auth_middleware(mut req: Request, next: Next) -> impl IntoResponse {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    match token {
        Some(t) => match validate_token(&t).await {
            Some(session) => {
                req.extensions_mut().insert(session);
                next.run(req).await
            }
            None => ApiResponse::err_and_log("Invalid token", StatusCode::UNAUTHORIZED, "", &[]),
        },
        None => ApiResponse::err_and_log("Missing token", StatusCode::UNAUTHORIZED, "", &[]),
    }
}

async fn generate_token(user: User, minecraft_version: String, mod_version: String) -> String {
    // Remove all existing tokens for this user
    let player_uuid = user.player_uuid();
    let keys_to_remove: Vec<String> = {
        let sessions = SESSIONS.read().await;
        let mut to_remove = Vec::new();
        for (token, session_arc) in sessions.iter() {
            let session = session_arc.read().await;
            if session.user().player_uuid() == player_uuid {
                to_remove.push(token.clone());
            }
        }
        to_remove
    };

    {
        let mut sessions = SESSIONS.write().await;
        for key in keys_to_remove {
            sessions.remove(&key);
        }
    }

    // Generate new token
    let token = Uuid::new_v4().to_string();
    let expires_at = (Utc::now() + Duration::seconds(TOKEN_LIFESPAN)).timestamp();

    let session = Session::new(user, HashMap::default(), minecraft_version, mod_version, expires_at);
    SESSIONS.write().await.insert(token.clone(), Arc::new(RwLock::new(session)));

    token
}

pub async fn validate_token(token: &str) -> Option<Arc<RwLock<Session>>> {
    SESSIONS.read().await.get(token).cloned()
}

pub async fn remove_user_session(player_uuid: &str) {
    let mut sessions = SESSIONS.write().await;
    let mut found = None;
    for (token, session) in sessions.iter() {
        if session.read().await.user().player_uuid() == player_uuid {
            found = Some(token.to_owned());
            break;
        }
    }

    if let Some(token) = found {
        sessions.remove(&token);
    }
}

pub async fn add_pending_notification(player_uuid: &str, message: String) {
    PENDING_NOTIFICATIONS.write().await.insert(player_uuid.to_owned(), message);
}