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

use crate::endpoints::{get_auction_by_auction_id, get_auction_by_item_uuid, get_auctions_by_auctioneer, get_price};
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    repo_manager::schedule().await;

    mayor_info::schedule().await;
    jacob_contests::schedule().await;
    bazaar::schedule().await;
    auctions::schedule().await;
    cosmetic_prices::schedule().await;
    profile_cleaner();

    // let nbt_str = "H4sIAAAAAAAA/11V3W9iRRQ/lH5Q2rXrx2r0wczqrpZg6+WrpftiKKUtbhc2wNZsjLkZ7h3gymUGZ+a28uiLiY8mJhqjiYkPvPg/mPCnkPhvqGfuQLu7hISZ3/zOmXN+58whDbAJiSANAIkVWAn8xF4C1qoi4jqRhqSm/U1YZdwbgPkkYOsZ70pGh7QbskQSNs8Dn52GtK/w9N80bPiBGod0gkYXQrIUoh/Bh7Pp4TmjoR48IrOply2WHPxlu9mik8HF0W72MAMPkHTCeowrZlg0myuWLSv/CqutJeP9G2cFe1goGZa/my1n4D6yqjLQpDqg3Iv9HWVzpYeWiYsMfLCknNAR7S8pB86CUygiB5eHda5ZGAZ9tvDTzRbKlpIvZeCdm8xIiyHHhpRz4K6Jc8yYb5BetuDA+2bxJUY4//UnXH31yhaFeh3dIjCbhhesHwhOLuEthE4Z1QMmySnFOHgf0S1Ez6S4xlsvYQc3T6XQzNPWxgAt9nV0xTjVDIE3EEDNBNo+oZySy3od7iDWGQjJFanjFuC92bQ0//l7UuM+k4rTISOtiLPFKVaw3AmYZD45FjxSj8j5xJeUoNsAmbvOp8UMFDBn/NaumJyYCjr7eYX7TwiVGCyhWlNvqEhfUq6NTjk8jI2UOSCih+DB/LsfX/KNx/uIly+EYiRHYmocDu1pFKWojB0XmvRpwI081HL2Ibl0b2Ki3sDi9noV1wkbwlYfGYRy35b3oRGsW4ljjktoQrDePrPeKjZ8x5gpm5QaCKHjts3Hzoy1InpANfGxPwwfewvexF/reXlxeR/lxW45qnAvYChMrC9k0MvZMlJ8C6bl5r/8QV5oWdOdYybh3Tgor0oxEjHqGoPDkF2x0Hi+h701/+1v0ubiuheaqp5MmA9HsU1x/vufJrIW+yYKJFPkcRQZ5eu8xyTHqE3JSVWMxiEzzYXUfcjaFp1Nh3Q2lcTunjzvnNer5LjZ7LTJLSEJa54IhYS//vkvBasNOmLw8SIiZC1Tvr2PoTYSJRBaQRp2at9qSSsaG6EbaaaSsCMpCjBxozGW0Wdm6OAQujsQ2h0LTbVwPTO5EE6nYU1iA6NRutY4qbXajcrjGh7gHlb7bKRSkKo2nxxXOq4Da63ms7PzTXgt4qHwhsx3VYgxmPG18gJtucqlYHu5dNEXrH9eaT+ttW5h52X4xnBxEyA0En7QQ4Fhg1odUmb8wr1647TWalQu3E6t1Wq23FhUnLRdoZSrjQEmkYZtM5SxQUZoiUluqfiFuyN84UjYSMK6jt+3YaNwUaiDEc4DN4xnC6JrSdjuSaG0e03DYex2BYk9O27cnh03lpiWNwPFAuv9eP4sTsc388cAmNxqFGEqD5xS3usdlIp7XYfSvaJTPtqj3Xxpr5BnucOcky93aR6V8CfMDTTqdefkec1tN5pfnF5gsVZhE2Nm+MJGY6zzD/ffnsb1WLf9b/6P/geLaPjkvgYAAA\u{003d}\u{003d}";
    // let item_nbt = decode_item(nbt_str)?;
    // let item_id = get_item_id(&item_nbt).unwrap_or("".to_string());
    // let value = calculate_item_value(&item_id, &item_nbt).await;
    // println!("{:#?}", value.info());

    //Some("Tomato".to_owned())
    // let mut pdr = PlayerDataResponse::new("7azem_".to_owned(), None).await.ok_or("PDR Error")?;
    // get_missing_accessories(&mut pdr, 1).await;
    // println!("{}", pdr.get_resp().unwrap_or("No Text".to_owned()));

    // let id = "10be71d9-2a0d-4ed1-874a-ac6ddd256d40";

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
    let listener = TcpListener::bind(addr).await.expect("Failed to bind to address");
    axum::serve(listener, app).await.expect("Failed to start server");
}