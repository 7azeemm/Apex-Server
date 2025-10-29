use crate::prices::auctions;
use crate::prices::auctions::get_lowest_bin_and_id;
use crate::prices::bazaar::get_buy_price_as_float;
use crate::structs::auctions_structs::AuctionItemResponse;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PriceQuery {
    pub item_id: String,
    pub source: Option<String>, // Optional parameter: "bazaar", "auction", or None
}

#[derive(Serialize)]
pub struct PriceResp {
    pub item_id: String,
    pub auction_id: String,
    pub price: f64,
    pub source: String,
    pub timestamp: u64,
}

#[derive(Serialize)]
pub struct AuctioneerAuctionItem {
    pub auction_id: String,
    pub item_name: String,
    pub price: u64,
}

pub async fn get_price(axum::extract::Query(q): axum::extract::Query<PriceQuery>) -> Result<Json<PriceResp>, StatusCode> {
    let item_id = q.item_id.as_str();

    let (price, auction_id, source) = match q.source.as_deref() {
        Some("bazaar") => {
            // Check only bazaar
            match get_buy_price_as_float(item_id).await {
                Some(bazaar_price) => (bazaar_price, None, "bazaar"),
                None => return Err(StatusCode::NOT_FOUND),
            }
        }
        Some("auction") => {
            // Check only auction
            match get_lowest_bin_and_id(item_id).await {
                Some((auction_price, auction_id)) => (auction_price as f64, Some(auction_id), "auction"),
                None => return Err(StatusCode::NOT_FOUND),
            }
        }
        _ => {
            // Default behavior: check both (bazaar first, then auction)
            match get_buy_price_as_float(item_id).await {
                Some(bazaar_price) => (bazaar_price, None, "bazaar"),
                None => {
                    match get_lowest_bin_and_id(item_id).await {
                        Some((auction_price, auction_id)) => (auction_price as f64, Some(auction_id), "auction"),
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

pub async fn get_auction_by_auction_id(Path(auction_id): Path<String>) -> Result<Json<AuctionItemResponse>, StatusCode> {
    match auctions::get_auction_by_id(&auction_id).await {
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

pub async fn get_auctions_by_auctioneer(Path(auctioneer_id): Path<String>) -> Result<Json<Vec<AuctioneerAuctionItem>>, StatusCode> {
    Ok(Json(auctions::get_auctions_by_player(&auctioneer_id).await))
}