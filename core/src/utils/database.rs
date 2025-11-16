use once_cell::sync::OnceCell;
use sqlx::PgPool;
use tracing::info;

static DB_POOL: OnceCell<PgPool> = OnceCell::new();

pub async fn connect() {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let pool = PgPool::connect(&db_url).await.expect("Failed to connect to the database");
    DB_POOL.set(pool).unwrap();
    info!("Connected to database!");
}

pub fn get_db_pool() -> &'static PgPool {
    DB_POOL.get().unwrap()
}