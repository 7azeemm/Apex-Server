#![allow(warnings)]
mod constants;
mod endpoints;
mod helpers;
mod item_utils;
mod live_data;
mod prices;
mod repos;
mod structs;
mod tools;
mod utils;

use crate::endpoints::{get_auction_by_auction_id, get_auctions_by_auctioneer, get_price};
use crate::live_data::{jacob_contests, mayor_info};
use crate::prices::{auctions, bazaar, cosmetic_prices};
use crate::repos::repo_manager;
use crate::tools::profile_fetcher::profile_cleaner;
use axum::routing::get;
use axum::Router;
use dotenv::dotenv;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::signal;
use common::logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    logger::setup_logging("tools");
    tracing::info!("Starting...");
    let start_time = Instant::now();

    repo_manager::schedule().await;
    mayor_info::schedule().await;
    jacob_contests::schedule().await;
    bazaar::schedule().await;
    cosmetic_prices::schedule().await;
    auctions::schedule().await;
    profile_cleaner();

    tracing::info!("Server took {:.2?} to get ready", start_time.elapsed());

    app().await;
    signal::ctrl_c().await?;
    Ok(())
}

async fn app() {
    let app = Router::new()
        .route("/price", get(get_price))
        .route("/auction/id/{auction_id}", get(get_auction_by_auction_id))
        .route("/auctions/auctioneer/{auctioneer_id}", get(get_auctions_by_auctioneer));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    let listener = TcpListener::bind(addr).await.expect("Failed to bind to address");
    axum::serve(listener, app).await.expect("Failed to start server");
}