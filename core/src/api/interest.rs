use std::path::Path;
use axum::Json;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::fs;

const FILE_PATH: &str = "interest.json";

#[derive(Deserialize)]
pub struct InterestRequest {
    username: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct InterestData {
    users: Vec<String>,
}

pub async fn interest(Json(payload): Json<InterestRequest>) -> impl IntoResponse {
    let path = Path::new(FILE_PATH);

    let mut data: InterestData = match path.exists() {
        false => InterestData::default(),
        true => {
            let content = fs::read_to_string(path).await.unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        }
    };

    // Avoid duplicates
    if !data.users.contains(&payload.username) {
        data.users.push(payload.username);
    }

    // Write back
    let json = serde_json::to_string_pretty(&data).unwrap_or_default();
    fs::write(path, json).await.unwrap_or_default();

    Json("ok")
}