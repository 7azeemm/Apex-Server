use once_cell::sync::Lazy;
use serde_json::Value;
use std::error::Error;
use std::time::Duration;

const API_KEY: &str = "d1086173-a977-4fc3-993a-5b1781e8e13b";

pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .gzip(true)
        .build()
        .expect("Failed to build http client")
});

pub async fn send_raw_http_request(url: &str) -> Result<String, Box<dyn Error>> {
    let resp = HTTP_CLIENT
        .get(url)
        .send().await?
        .text().await?;
    Ok(resp)
}

pub async fn send_http_request(url: &str) -> Result<Value, Box<dyn Error>> {
    let resp = send_raw_http_request(url).await?;
    let json: Value = serde_json::from_str(&*resp)?;
    Ok(json)
}

pub fn get_api_key() -> &'static str { API_KEY }