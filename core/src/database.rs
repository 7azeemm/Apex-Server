use once_cell::sync::OnceCell;
use sqlx::PgPool;

static DB_POOL: OnceCell<PgPool> = OnceCell::new();

pub async fn connect() {
    let db_url = std::env::var("DATABASE_URL").expect("Couldn't find DB url in .env file");
    let pool = PgPool::connect(&db_url).await.expect("Failed to connect to the database");
    DB_POOL.set(pool).unwrap();
    println!("Connected to database!");
}

pub fn get_db_pool() -> &'static PgPool {
    DB_POOL.get().expect("DB pool is not initialized yet")
}
