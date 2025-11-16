use once_cell::sync::Lazy;
use serde_json::Value;
use std::error::Error;
use std::time::Duration;

pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .gzip(true)
        .build()
        .expect("Failed to build HTTP client")
});

pub async fn send_raw_http_request(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    Ok(HTTP_CLIENT.get(url).send().await?.text().await?)
}

pub async fn send_http_request(url: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let resp = send_raw_http_request(url).await?;
    let json: Value = serde_json::from_str(&resp)?;
    Ok(json)
}