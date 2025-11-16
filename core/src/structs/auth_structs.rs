use crate::structs::chat::Chat;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;
use crate::structs::plan::Plan;
use crate::structs::user::User;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    plan: Plan,
    plan_color: i64,
    daily_tokens: i64,
    tokens_used_today: i64,
}

impl UserInfo {
    pub fn from_user(user: &User) -> Self {
        Self {
            plan: user.plan().clone(),
            plan_color: user.plan().color(),
            daily_tokens: user.plan().daily_tokens(),
            tokens_used_today: *user.tokens_used_today(),
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
            ("tokens_used_today", self.user.tokens_used_today().to_string()),
            ("minecraft_version", self.minecraft_version().to_owned()),
            ("mod_version", self.mod_version().to_owned()),
        ]
    }
}
