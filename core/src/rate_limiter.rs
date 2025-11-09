use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(rate_limiter): State<RateLimiter>,
    req: Request,
    next: Next,
) -> impl IntoResponse {
    let ip = addr.ip();
    if !rate_limiter.check_rate(ip).await {
        println!("Too Many Requests for ip {ip}");
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    next.run(req).await
}

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
    limit: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(limit: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            limit,
            window,
        }
    }

    async fn check_rate(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut requests = self.requests.lock().await;

        let history = requests.entry(ip).or_insert_with(Vec::new);
        history.retain(|&t| now.duration_since(t) <= self.window);
        requests.retain(|_, h| !h.is_empty());

        let history = requests.entry(ip).or_insert_with(Vec::new);
        match history.len() >= self.limit {
            true => false,
            false => {
                history.push(now);
                true
            }
        }
    }
}
