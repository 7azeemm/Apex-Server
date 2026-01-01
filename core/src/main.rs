#![allow(warnings)]
mod api;
mod structs;
mod constants;
mod workers;
mod utils;

use crate::api::chats::{delete_chat, get_chat, get_chats};
use crate::api::completions::completions_handler;
use utils::rate_limiter::{rate_limit_middleware, RateLimiter};
use axum::routing::{get, post};
use axum::{middleware, Router};
use dotenvy::dotenv;
use api::auth;
use api::auth::{auth, auth_middleware};
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use common::logger;
use utils::database;
use crate::api::interest::interest;
use crate::api::users::upgrade_plan;
use crate::structs::plan::Plan;
use crate::workers::{daily_token_reset, plan_expiration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    logger::setup_logging("core");
    tracing::info!("Starting...");

    database::connect().await;
    auth::schedule();
    daily_token_reset::schedule();
    plan_expiration::schedule();

    tracing::info!("Server is ready");

    app().await;
    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn app() {
    let rate_limiter = RateLimiter::new(300, Duration::from_secs(60));

    let api_router = Router::new()
        .route("/chats", get(get_chats))
        .route("/chat/{id}", get(get_chat).delete(delete_chat))
        .route("/chat/completions", post(completions_handler))
        .layer(middleware::from_fn(auth_middleware));

    let app = Router::new()
        .route("/auth", post(auth))
        .route("/interest", post(interest))
        .nest("/api", api_router)
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        ))
        .with_state(rate_limiter);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(addr).await.expect("Failed to bind to address");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.expect("Failed to start server");
}