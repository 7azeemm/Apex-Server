#![allow(warnings)]
mod constants;
mod item_utils;
mod repos;
mod endpoints;
mod tools;
mod live_data;
mod utils;
mod http;
mod structs;
mod extensions;
mod prices;
mod helpers;

use crate::endpoints::{get_auction_by_auction_id, get_auctions_by_auctioneer, get_price};
use crate::live_data::{jacob_contests, mayor_info};
use crate::prices::auctions::{search_in_auction_house, Budget};
use crate::prices::{auctions, bazaar, cosmetic_prices};
use crate::repos::repo_manager;
use crate::structs::player_data_structs::StringBuilder;
use crate::tools::profile_fetcher::profile_cleaner;
use axum::routing::get;
use axum::Router;
use dotenv::dotenv;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    repo_manager::schedule().await;

    mayor_info::schedule().await;
    jacob_contests::schedule().await;
    bazaar::schedule().await;
    cosmetic_prices::schedule().await;
    auctions::schedule().await;
    profile_cleaner();

    let mut sb = StringBuilder::new();

    for budget in vec![Budget::Low, Budget::Medium, Budget::High, Budget::NoLimit] {
        search_in_auction_house(&mut sb, "Strong Dragon Chestplate", false, budget.clone()).await;
        search_in_auction_house(&mut sb, "Necron's chestplate", false, budget.clone()).await;
        search_in_auction_house(&mut sb, "Giant's sword", false, budget.clone()).await;
        search_in_auction_house(&mut sb, "Shadow assassin boots", false, budget.clone()).await;
        search_in_auction_house(&mut sb, "Livid dagger", false, budget.clone()).await;
        search_in_auction_house(&mut sb, "Shadow fury", false, budget.clone()).await;
        search_in_auction_house(&mut sb, "Glacial scythe", true, budget.clone()).await;
        search_in_auction_house(&mut sb, "Bonzo mask", false, budget.clone()).await;
    }

    println!("{}", sb.get_response());

    app().await;
    signal::ctrl_c().await?;
    Ok(())
}

async fn app() {
    let app = Router::new()
        .route("/price", get(get_price))
        .route("/auction/id/{auction_id}", get(get_auction_by_auction_id))
        .route("/auctions/auctioneer/{auctioneer_id}", get(get_auctions_by_auctioneer));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = TcpListener::bind(addr).await.expect("Failed to bind to address");
    axum::serve(listener, app).await.expect("Failed to start server");
}