#![allow(warnings)]
mod constants;
pub mod item_utils;
mod repos;
mod endpoints;
mod player_data;
mod live_data;
mod utils;
mod http;
mod structs;
mod extensions;
mod prices;
mod helpers;

use crate::endpoints::{get_auction_by_auction_id, get_auctions_by_auctioneer, get_price};
use crate::live_data::{jacob_contests, mayor_info};
use crate::player_data::profile_fetcher::profile_cleaner;
use crate::prices::{auctions, bazaar, cosmetic_prices};
use crate::repos::repo_manager;
use axum::routing::get;
use axum::Router;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use crate::structs::player_data_structs::{PlayerDataResponse};

macro_rules! run_tools {
    ($pdr:expr, $($tool:expr),+ $(,)?) => {
        $(
            $tool(&mut $pdr).await;
            println!("{}\n------", $pdr.get_resp().unwrap_or("No Text".to_owned()));
        )+
    };
}

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

    let mut pdr = PlayerDataResponse::new("7azem_".to_owned(), None).await?;

    // run_tools!(
    //     pdr,
    //     get_player_info,
    //     get_player_overview,
    //     get_mining_info,
    //     get_garden_info,
    //     get_foraging_info,
    //     get_fishing_info,
    //     get_slayer_info,
    //     get_dungeons_info,
    //     get_events_info,
    //     get_misc_info,
    //     get_inventory,
    //     get_profile_networth,
    // );

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