use std::time::Duration;
use once_cell::sync::OnceCell;
use sqlx::{ConnectOptions, PgPool};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tracing::{info, log};

static DB_POOL: OnceCell<PgPool> = OnceCell::new();

pub async fn connect() {
    let connect_opts = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set")
        .parse::<PgConnectOptions>()
        .expect("Invalid DB URL")
        .log_statements(log::LevelFilter::Debug)
        .options([("statement_timeout", "10s")]);

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(8 * 60))
        .max_lifetime(Duration::from_secs(30 * 60))
        .connect_with(connect_opts)
        .await
        .expect("Failed to connect to DB");

    DB_POOL.set(pool).unwrap();
    info!("Connected to database!");
}

pub fn get_db_pool() -> &'static PgPool {
    DB_POOL.get().unwrap()
}