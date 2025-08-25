use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use crate::auctions;
use crate::auctions::get_lowest_bin;
use crate::bazaar::get_item_price;
use crate::structs::{AuctionItemResponse, AuctioneerAuctionItem, PriceQuery, PriceResp};

pub async fn get_price(axum::extract::Query(q): axum::extract::Query<PriceQuery>) -> Result<Json<PriceResp>, StatusCode> {
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

pub async fn get_auction_by_item_uuid(Path(item_uuid): Path<String>) -> Result<Json<AuctionItemResponse>, StatusCode> {
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

pub async fn get_auction_by_auction_id(Path(auction_id): Path<String>) -> Result<Json<AuctionItemResponse>, StatusCode> {
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

pub async fn get_auctions_by_auctioneer(Path(auctioneer_id): Path<String>) -> Result<Json<Vec<AuctioneerAuctionItem>>, StatusCode> {
    match auctions::get_auction_ids_by_auctioneer(&auctioneer_id).await {
        Some(auction_items) => Ok(Json(auction_items)),
        None => {
            println!("couldn't find auctions by auctioneer id {auctioneer_id}");
            Err(StatusCode::NOT_FOUND)
        }
    }
}