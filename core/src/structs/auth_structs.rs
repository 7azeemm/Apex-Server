use crate::structs::chat_structs::Chat;
use crate::structs::user_structs::{Plan, User};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

fn from_base64_string<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    STANDARD.decode(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct TokenRequest {
    key_pair: KeyPairInfo,
    signed_data: SignedData,
    minecraft_version: String,
    mod_version: String,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct KeyPairInfo {
    uuid: Uuid,
    #[serde(deserialize_with = "from_base64_string")]
    public_key: Vec<u8>,
    #[serde(deserialize_with = "from_base64_string")]
    public_key_signature: Vec<u8>,
    expires_at: i64,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct SignedData {
    #[serde(deserialize_with = "from_base64_string")]
    original: Vec<u8>,
    #[serde(deserialize_with = "from_base64_string")]
    signed: Vec<u8>,
}

impl TokenRequest {
    pub fn context(&self) -> Vec<(&str, String)> {
        vec![
            ("player_uuid", self.key_pair.uuid.to_string()),
            ("minecraft_version", self.minecraft_version.clone()),
            ("mod_version", self.mod_version.clone()),
        ]
    }
}

#[derive(Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(flatten)]
    payload: T,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(payload: T) -> Response {
        (
            StatusCode::OK,
            Json(ApiResponse {
                error: None,
                payload,
            }),
        ).into_response()
    }
}

impl ApiResponse<()> {
    fn log(msg: String, error: impl Display, context: &[(&str, String)]) {
        eprintln!("Api error: {msg}, details: {error}");
        for (k, v) in context {
            eprintln!("  {k} = {v}");
        }
    }

    pub fn internal_err(msg: impl Into<String> + Display, error: impl Display, context: &[(&str, String)]) -> Response {
        Self::log(msg.to_string(), error, context);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                error: Some(msg.into()),
                payload: (),
            }),
        ).into_response()
    }

    pub fn err(msg: impl Into<String> + Display, status: StatusCode) -> Response {
        (
            status,
            Json(ApiResponse {
                error: Some(msg.into()),
                payload: (),
            }),
        ).into_response()
    }

    pub fn err_and_log(msg: impl Into<String> + Display, status: StatusCode, error: impl Display, context: &[(&str, String)]) -> Response {
        Self::log(msg.to_string(), error, context);
        Self::err(msg, status)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    plan: Plan,
    plan_owned_at: i64,
    plan_duration: Option<i64>,
    daily_max_tokens: i64,
    used_tokens_today: i64,
}

impl UserInfo {
    pub fn from_user(user: &User) -> Self {
        Self {
            plan: user.plan().clone(),
            plan_owned_at: *user.plan_owned_at(),
            plan_duration: user.plan().duration(),
            daily_max_tokens: user.plan().daily_max_tokens(),
            used_tokens_today: *user.used_tokens_today(),
        }
    }
}

#[derive(Debug, Clone, new, Getters)]
#[getset(get = "pub")]
pub struct Session {
    user: User,
    chats: HashMap<String, Chat>,
    minecraft_version: String,
    mod_version: String,
    expires_at: i64,
}

impl Session {
    pub fn user_mut(&mut self) -> &mut User {
        &mut self.user
    }
    pub fn chats_mut(&mut self) -> &mut HashMap<String, Chat> {
        &mut self.chats
    }
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() >= self.expires_at
    }

    pub fn context(&self) -> Vec<(&'static str, String)> {
        vec![
            ("player_uuid", self.user.player_uuid().to_owned()),
            ("player_name", self.user.player_name().to_owned()),
            ("plan", self.user.plan().to_string()),
            ("used_tokens_today", self.user.used_tokens_today().to_string()),
            ("minecraft_version", self.minecraft_version().to_owned()),
            ("mod_version", self.mod_version().to_owned()),
        ]
    }
}
