use once_cell::sync::Lazy;
use reqwest;
use std::time::Duration;

pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .gzip(true)
        .build()
        .expect("Failed to build http client")
});