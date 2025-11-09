use crate::repos::repo_manager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::{interval_at, Instant};

#[derive(Clone)]
pub struct Repo {
    pub name: &'static str,
    pub url: &'static str,
    pub branch: &'static str,
    pub path: &'static str,
    pub threshold: u64, // seconds
}

impl Repo {
    pub async fn schedule<F, Fut>(&self, reload_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output=()> + Send + 'static,
    {
        let waiter = Arc::new(Notify::new());
        let waiter_clone = waiter.clone();
        let config = self.clone();
        tokio::spawn(async move {
            let mut ticker = interval_at(Instant::now(), Duration::from_secs(config.threshold));
            let mut force_update = true;

            loop {
                ticker.tick().await;
                let updated = repo_manager::fetch_repo(&config).await;
                if updated || force_update {
                    reload_fn().await;
                }
                force_update = false;
                waiter_clone.notify_waiters();
            }
        });

        waiter.notified().await;
    }
}
