mod statics;
mod structs;
mod bazaar;
mod auctions;
pub mod item_value_calculator;
mod constants;
mod modifiers;
pub mod item_utils;
mod neu_repo;
mod endpoints;
mod player_data;
mod live_data;

use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use axum::{Json, Router};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::get;
use sea_orm::Iden;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::time::sleep;
use crate::auctions::get_lowest_bin;
use crate::bazaar::get_item_price;
use crate::endpoints::{get_auction_by_auction_id, get_auction_by_item_uuid, get_auctions_by_auctioneer, get_price};
use crate::player_data::{fetch_profiles, get_basic_info, get_armor, get_garden_data, get_garden_info, get_inventory, get_item_info, get_profile_networth, get_selected_profile, search_item, search_pet, spawn_profile_cleanup};
use crate::structs::{AuctionItemResponse, AuctioneerAuctionItem, PriceQuery, PriceResp};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    neu_repo::fetch_repo();
    live_data::schedule();
    bazaar::schedule();
    bazaar::BAZAAR_READY.notified().await;

    // auctions::schedule();
    //TODO: move to schedule? (in the end of fn)
    // auctions::AUCTIONS_READY.notified().await;

    spawn_profile_cleanup();
    let id = "10be71d9-2a0d-4ed1-874a-ac6ddd256d40";
    // get_basic_info(id).await;
    println!("{}", get_profile_networth(id).await.unwrap());


    app().await;

    signal::ctrl_c().await?;
    Ok(())
}

async fn app() {
    let app = Router::new()
        .route("/price", get(get_price))
        .route("/auction/item/{item_uuid}", get(get_auction_by_item_uuid))
        .route("/auction/id/{auction_id}", get(get_auction_by_auction_id))
        .route("/auctions/auctioneer/{auctioneer_id}", get(get_auctions_by_auctioneer));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("Listening on {addr}");

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}