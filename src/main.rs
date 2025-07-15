mod statics;
mod structs;
mod bazaar;
mod auctions;
pub mod item_value_calculator;
mod constants;
mod modifiers;
pub mod item_utils;

use std::error::Error;
use std::net::SocketAddr;
use axum::{Json, Router};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::get;
use sea_orm::Iden;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::signal;
use crate::auctions::get_lowest_bin;
use crate::bazaar::get_item_price;
use crate::structs::AuctionItemResponse;

#[derive(Deserialize)]
struct PriceQuery {
    item_id: String,
    source: Option<String>, // Optional parameter: "bazaar", "auction", or None
}

#[derive(Serialize)]
struct PriceResp {
    item_id: String,
    auction_id: String,
    price: f64,
    source: String,
    timestamp: u64,
}

#[derive(Serialize)]
struct AuctioneerAuctionItem {
    auction_id: String,
    item_name: String,
    price: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    bazaar::schedule();
    bazaar::BAZAAR_READY.notified().await;

    auctions::schedule();

    let app = Router::new()
        .route("/price", get(get_price))
        .route("/auction/item/{item_uuid}", get(get_auction_by_item_uuid))
        .route("/auction/id/{auction_id}", get(get_auction_by_auction_id))
        .route("/auctions/auctioneer/{auctioneer_id}", get(get_auctions_by_auctioneer));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Listening on {addr}");

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");

    signal::ctrl_c().await?;
    Ok(())
}

async fn get_price(axum::extract::Query(q): axum::extract::Query<PriceQuery>) -> Result<Json<PriceResp>, StatusCode> {
    let item_id = q.item_id.as_str();

    let (price, auction_id, source) = match q.source.as_deref() {
        Some("bazaar") => {
            // Check only bazaar
            match get_item_price(item_id).await {
                Some(bazaar_price) => (bazaar_price, None, "bazaar"),
                None => return Err(StatusCode::NOT_FOUND),
            }
        }
        Some("auction") => {
            // Check only auction
            match get_lowest_bin(item_id, true).await {
                Some((auction_price, auction_id)) => (auction_price, auction_id, "auction"),
                None => return Err(StatusCode::NOT_FOUND),
            }
        }
        _ => {
            // Default behavior: check both (bazaar first, then auction)
            match get_item_price(item_id).await {
                Some(bazaar_price) => (bazaar_price, None, "bazaar"),
                None => {
                    match get_lowest_bin(item_id, true).await {
                        Some((auction_price, auction_id)) => (auction_price, auction_id, "auction"),
                        None => return Err(StatusCode::NOT_FOUND),
                    }
                }
            }
        }
    };

    Ok(Json(PriceResp {
        item_id: q.item_id,
        auction_id: auction_id.unwrap_or_else(|| "None".to_string()),
        price,
        source: source.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    }))
}

async fn get_auction_by_item_uuid(Path(item_uuid): Path<String>) -> Result<Json<AuctionItemResponse>, StatusCode> {
    match auctions::get_auction_by_item_uuid(&item_uuid).await {
        Some(auction_item) => {
            let response = AuctionItemResponse::from_auction_item(&auction_item).await;
            Ok(Json(response))
        }
        None => {
            println!("couldn't find auction by item uuid {item_uuid}");
            Err(StatusCode::NOT_FOUND)
        }
    }
}

async fn get_auction_by_auction_id(Path(auction_id): Path<String>) -> Result<Json<AuctionItemResponse>, StatusCode> {
    match auctions::get_auction_by_auction_id(&auction_id).await {
        Some(auction_item) => {
            let response = AuctionItemResponse::from_auction_item(&auction_item).await;
            Ok(Json(response))
        }
        None => {
            println!("couldn't find auction by auction id {auction_id}");
            Err(StatusCode::NOT_FOUND)
        }
    }
}

async fn get_auctions_by_auctioneer(Path(auctioneer_id): Path<String>) -> Result<Json<Vec<AuctioneerAuctionItem>>, StatusCode> {
    match auctions::get_auction_ids_by_auctioneer(&auctioneer_id).await {
        Some(auction_items) => Ok(Json(auction_items)),
        None => {
            println!("couldn't find auctions by auctioneer id {auctioneer_id}");
            Err(StatusCode::NOT_FOUND)
        }
    }
}