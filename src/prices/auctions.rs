use crate::endpoints::AuctioneerAuctionItem;
use crate::http::send_raw_http_request;
use crate::item_utils::{decode_item, get_item_id};
use crate::prices::item_value_calculator;
use crate::structs::auctions_structs::{Auction, AuctionItem, AuctionManager, AuctionsResponse, LowestBinItem};
use crate::utils::get_time_as_secs;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use std::cmp::max;
use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::{interval, interval_at, sleep};
use crate::prices::cosmetic_prices::get_cosmetic_price;

const API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/auctions";
const THRESHOLD: u64 = 70;
const MIN_DELAY_SECS: u64 = 20;
const MAX_RETRIES: u64 = 3;
const MAX_CONCURRENT_REQUESTS: usize = 10;

static DATA_WAITER: Notify = Notify::const_new();
static AUCTION_MANAGER: LazyLock<AuctionManager> = LazyLock::new(|| AuctionManager::new());

pub async fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval(Duration::from_secs(THRESHOLD));
        loop {
            ticker.tick().await;
            match update().await {
                Ok(last_updated) => {
                    let next_update_time = (last_updated / 1000) + THRESHOLD;
                    let now = get_time_as_secs();

                    let delay = Duration::from_secs(match next_update_time > now {
                        true => max(next_update_time - now, MIN_DELAY_SECS),
                        false => MIN_DELAY_SECS,
                    });

                    ticker = interval_at(tokio::time::Instant::now() + delay, Duration::from_secs(THRESHOLD));
                    DATA_WAITER.notify_waiters();
                }
                Err(err) => {
                    eprintln!("[Auctions] Error: {:?}", err);
                    ticker = interval_at(tokio::time::Instant::now() + Duration::from_secs(MIN_DELAY_SECS), Duration::from_secs(THRESHOLD));
                }
            }
        }
    });
    DATA_WAITER.notified().await;
}

async fn update() -> Result<u64, Box<dyn Error + Send + Sync>> {
    println!("[Auctions] Starting Auctions update...");
    let start_time = Instant::now();

    let first_page = fetch_page(0).await?;
    let total_pages = *first_page.total_pages();
    let total_auctions = *first_page.total_auctions();
    let last_updated = *first_page.last_updated();

    println!("[Auctions] Fetching {} auctions in {} pages...", total_auctions, total_pages);

    AUCTION_MANAGER.start_update().await;
    process_page(first_page.auctions()).await;

    let mut tasks = FuturesUnordered::new();
    let mut next_page = 1;

    while next_page < total_pages || !tasks.is_empty() {
        // Fill up the buffer
        while tasks.len() < MAX_CONCURRENT_REQUESTS && next_page < total_pages {
            let page = next_page;
            tasks.push(tokio::spawn(async move {
                match fetch_page(page).await {
                    Ok(page_data) => { process_page(page_data.auctions()).await; }
                    Err(e) => eprintln!("[Auctions] Failed to fetch page {}: {}", page, e)
                }
            }));
            next_page += 1;
        }
        tasks.next().await;
    }

    update_auctions_list().await;
    update_lowest_bin_list().await;
    calculate_base_prices().await;
    update_auctions_values().await;

    println!("[Auctions] Successfully updated auctions in {:.2?}", start_time.elapsed());
    Ok(last_updated)
}

async fn fetch_page(page: u64) -> Result<AuctionsResponse, Box<dyn Error + Send + Sync>> {
    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let delay = 1 * attempt.pow(2);
            println!("[Auctions] Retrying fetching page {page} in {:.1}sc", delay);
            sleep(Duration::from_secs(delay)).await;
        }

        if let Ok(resp) = send_raw_http_request(&format!("{API_ENDPOINT}?page={page}")).await {
            let page_response: AuctionsResponse = serde_json::from_str(&resp)?;

            if *page_response.success() {
                return Ok(page_response);
            }
        }
    }

    Err(format!("[Auctions] Couldn't fetch page {page} after {MAX_RETRIES} attempts.").into())
}

async fn process_page(auctions: &[Auction]) {
    for auction in auctions.iter() {
        process_auction(auction).await;
    }
}

async fn process_auction(auction: &Auction) {
    if !auction.bin() { return; }
    let auction_id = auction.uuid();

    AUCTION_MANAGER.to_keep.write().await.insert(auction_id.to_owned());
    if AUCTION_MANAGER.auctions.read().await.contains_key(auction_id) { return; };

    let item_nbt = match decode_item(auction.item_bytes()) {
        Ok(nbt) => nbt,
        Err(err) => {
            eprintln!("Failed to decode {}, err: {err}", auction.item_name());
            return
        }
    };

    if let Some(id) = get_item_id(&item_nbt) {
        let auction_item = AuctionItem::new(auction, id, item_nbt);
        let mut to_add = AUCTION_MANAGER.to_add.write().await;
        to_add.insert(auction_id.to_owned(), auction_item);
    }
}

async fn update_auctions_list() {
    let new_auctions: Vec<_> = {
        let mut to_add = AUCTION_MANAGER.to_add.write().await;
        to_add.drain().collect()
    };

    let mut auctions = AUCTION_MANAGER.auctions.write().await;
    let to_keep = AUCTION_MANAGER.to_keep.read().await;
    auctions.retain(|auction_id, _| to_keep.contains(auction_id));

    auctions.extend(new_auctions);
}

async fn update_lowest_bin_list() {
    let mut updates = HashMap::new();

    let auctions = AUCTION_MANAGER.auctions.read().await;
    for (auction_id, auction) in auctions.iter() {
        let item_id = auction.item_id();
        updates.entry(item_id)
            .and_modify(|existing: &mut (&str, u64)| {
                let &price = auction.price();
                if price < existing.1 {
                    *existing = (auction_id, price);
                }
            })
            .or_insert((auction_id, *auction.price()));
    }

    let mut lowest_bins = AUCTION_MANAGER.lowest_bins.write().await;
    lowest_bins.clear();

    for (item_id, (auction_id, price)) in updates {
        lowest_bins.insert(
            item_id.to_owned(),
            LowestBinItem::new(auction_id.to_owned(), item_id.to_owned(), price, price),
        );
    }
}

async fn calculate_base_prices() {
    let mut to_update = Vec::new();

    {
        let lowest_bins = AUCTION_MANAGER.lowest_bins.read().await;
        for (_, lowest_bin_item) in lowest_bins.iter() {
            to_update.push((
                lowest_bin_item.auction_id().to_owned(),
                *lowest_bin_item.price(),
            ));
        }
    }

    for (auction_id, price) in to_update {
        if let Some(mut auction) = AUCTION_MANAGER.auctions.write().await.get_mut(&auction_id) {
            calc_auction_value(&mut auction).await;
            let modifiers_value = auction.value().modifiers_value();

            if let Some(mut lowest_bin_item) = AUCTION_MANAGER.lowest_bins.write().await.get_mut(&auction_id) {
                lowest_bin_item.set_base_price(price - modifiers_value);
            }
        }
    }
}

async fn update_auctions_values() {
    let mut auctions = AUCTION_MANAGER.auctions.write().await;
    for (_, auction) in auctions.iter_mut() {
        calc_auction_value(auction).await;
    }
}

async fn calc_auction_value(auction: &mut AuctionItem) {
    let item_id = auction.item_id();
    let item_nbt = auction.item_nbt();
    let item_value = item_value_calculator::calculate_item_value(item_id, item_nbt, false).await;
    auction.set_value(item_value);
}

pub async fn get_base_price(item_id: &str) -> Option<u64> {
    AUCTION_MANAGER.lowest_bins.read().await.get(item_id).map(|i| *i.base_price())
}

pub async fn get_lowest_bin(item_id: &str) -> Option<u64> {
    match AUCTION_MANAGER.lowest_bins.read().await.get(item_id).map(|i| *i.price()) {
        Some(p) => Some(p),
        None => get_cosmetic_price(item_id).await
    }
}

pub async fn get_lowest_bin_and_id(item_id: &str) -> Option<(u64, String)> {
    AUCTION_MANAGER.lowest_bins.read().await.get(item_id).map(|i| (*i.price(), i.auction_id().to_owned()))
}

pub async fn get_auction_by_id(auction_id: &str) -> Option<AuctionItem> {
    AUCTION_MANAGER.auctions.read().await.get(auction_id).cloned()
}

pub async fn get_auctions_by_player(auctioneer_id: &str) -> Vec<AuctioneerAuctionItem> {
    AUCTION_MANAGER.auctions.read().await.iter()
        .filter(|(_, auction)| auction.auctioneer() == auctioneer_id)
        .map(|(id, auction)| AuctioneerAuctionItem {
            auction_id: id.clone(),
            item_name: auction.item_name().to_owned(),
            price: *auction.price(),
        })
        .collect()
}