use std::env::var;
use once_cell::sync::Lazy;
use serde_json::json;
use crate::http::HTTP_CLIENT;

static DISCORD_WEBHOOK: Lazy<String> = Lazy::new(|| {
    var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL must be set")
});

struct DiscordLogger;

impl DiscordLogger {
    pub async fn send_embed(embed: serde_json::Value) {
        let payload = json!({ "embeds": [embed] });

        let _ = HTTP_CLIENT
            .post(&*DISCORD_WEBHOOK)
            .json(&payload)
            .send()
            .await;
    }
}

pub fn log_error(description: String) {
    let embed = json!({
        "title": "⛔ Error Logged",
        "color": 0xD32F2F,
        "description": description,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "footer": {"text": "Backend Server"}
    });

    tokio::spawn(async move {
        DiscordLogger::send_embed(embed).await;
    });
}

pub fn log_new_user(player_name: &str) {
    let embed = json!({
        "title": "🟩 New User",
        "color": 0x4CAF50,
        "description": format!("**Player:** {player_name}"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "footer": {"text": "Backend Server"}
    });

    tokio::spawn(async move {
        DiscordLogger::send_embed(embed).await;
    });
}

pub fn log_plan_upgrade(player_name: &str, old_plan: &str, new_plan: &str) {
    let mut description = String::new();
    description.push_str(&format!("**Player:** {}\n", player_name));
    description.push_str(&format!("**Plan:** {} ➜ {}", old_plan, new_plan));

    let embed = json!({
        "title": "⚡ Plan Upgraded",
        "color": 0x6A1B9A,
        "description": description,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "footer": {"text": "Backend Server"}
    });

    tokio::spawn(async move {
        DiscordLogger::send_embed(embed).await;
    });
}