use crate::endpoints::AuctioneerAuctionItem;
use crate::item_utils::{decode_item, get_item_id};
use crate::prices::cosmetic_prices::get_cosmetic_price;
use crate::prices::item_value_calculator::calculate_item_value;
use crate::repos::neu::items::get_id_by_name;
use crate::structs::auctions_structs::{Auction, AuctionItem, AuctionManager, AuctionsResponse, Budget, LowestBinItem};
use crate::structs::player_data_structs::StringBuilder;
use crate::utils::{format_number, get_time_as_secs};
use common::http::send_raw_http_request;
use common::player_fetcher::get_player_username;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use std::cmp::max;
use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::{interval, interval_at, sleep};
use tracing::{error, info, warn};

const API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/auctions";
const THRESHOLD: u64 = 70;
const MIN_DELAY_SECS: u64 = 20;
const MAX_RETRIES: u64 = 3;
const MAX_CONCURRENT_REQUESTS: usize = 10;

static DATA_WAITER: Notify = Notify::const_new();
static AUCTION_MANAGER: LazyLock<AuctionManager> = LazyLock::new(AuctionManager::new);

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

                    ticker = interval_at(
                        tokio::time::Instant::now() + delay,
                        Duration::from_secs(THRESHOLD),
                    );
                    DATA_WAITER.notify_waiters();
                }
                Err(err) => {
                    error!("[Auctions] Failed to update auctions: {:?}", err);
                    ticker = interval_at(
                        tokio::time::Instant::now() + Duration::from_secs(MIN_DELAY_SECS),
                        Duration::from_secs(THRESHOLD),
                    );
                }
            }
        }
    });
    DATA_WAITER.notified().await;
}

async fn update() -> Result<u64, Box<dyn Error + Send + Sync>> {
    let first_page = fetch_page(0).await?;
    let total_pages = *first_page.total_pages();
    let last_updated = *first_page.last_updated();

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
                    Ok(page_data) => process_page(page_data.auctions()).await,
                    Err(e) => error!("[Auctions] Failed to fetch page {}: {}", page, e),
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

    Ok(last_updated)
}

async fn fetch_page(page: u64) -> Result<AuctionsResponse, Box<dyn Error + Send + Sync>> {
    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let delay = attempt.pow(2);
            warn!("[Auctions] Failed fetching page {page}. Retrying in {:.1}sc", delay);
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
    if !auction.bin() {
        return;
    }
    let auction_id = auction.uuid();

    AUCTION_MANAGER.to_keep.write().await.insert(auction_id.to_owned());
    if AUCTION_MANAGER.auctions.read().await.contains_key(auction_id) { return; };

    let item_nbt = match decode_item(auction.item_bytes()) {
        Ok(nbt) => nbt,
        Err(err) => {
            error!("Failed to decode {}, err: {err}", auction.item_name());
            return;
        }
    };

    if let Some(id) = get_item_id(&item_nbt) {
        let auction_item = AuctionItem::new(auction_id.to_owned(), auction, id, item_nbt);
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
        updates
            .entry(item_id)
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
        if let Some(auction) = AUCTION_MANAGER.auctions.write().await.get_mut(&auction_id) {
            calc_auction_value(auction).await;
            let modifiers_value = auction.value().modifiers_value();

            if let Some(lowest_bin_item) = AUCTION_MANAGER.lowest_bins.write().await.get_mut(&auction_id) {
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
    let item_value = calculate_item_value(item_id, item_nbt, false, true).await;
    auction.set_value(item_value);
}

fn get_max_price(lowest_bin: u64, budget: &Budget) -> u64 {
    let additions = match lowest_bin {
        0..=5_000_000 => [3_000_000, 8_000_000, 16_000_000, u64::MAX],
        5_000_001..=25_000_000 => [8_000_000, 40_000_000, 60_000_000, u64::MAX],
        25_000_001..=100_000_000 => [15_000_000, 40_000_000, 60_000_000, u64::MAX],
        100_000_001..=200_000_000 => [15_000_000, 50_000_000, 80_000_000, u64::MAX],
        200_000_001..=1_000_000_000 => [30_000_000, 75_000_000, 150_000_000, u64::MAX],
        _ => [50_000_000, 150_000_000, 300_000_000, u64::MAX],
    };

    let addition = match budget {
        Budget::Low => additions[0],
        Budget::Medium => additions[1],
        Budget::High => additions[2],
        Budget::NoLimit => additions[3],
    };

    lowest_bin.saturating_add(addition)
}

pub async fn search_in_auction_house(sb: &mut StringBuilder, name: &str, pet: bool, budget: Budget) {
    let item_ids = get_id_by_name(name, pet).await;

    if item_ids.is_empty() {
        sb.push("Couldn't find any auctions by that name".to_owned());
        return;
    }

    // Collect auctions by item_id
    let mut auctions_by_id: HashMap<String, Vec<AuctionItem>> = HashMap::new();
    {
        let auctions_list = AUCTION_MANAGER.auctions.read().await;
        for auction in auctions_list.values() {
            let auction_item_id = auction.item_id();
            if item_ids.contains(auction_item_id) {
                auctions_by_id
                    .entry(auction_item_id.to_owned())
                    .or_default()
                    .push(auction.clone());
            }
        }
    }

    if auctions_by_id.is_empty() {
        sb.push("No auctions available for these items".to_owned());
        return;
    }

    let mut lowest_bins = HashMap::new();
    for item_id in &item_ids {
        if let Some(lowest_bin) = get_lowest_bin(item_id).await {
            lowest_bins.insert(item_id.clone(), lowest_bin);
        }
    }

    if lowest_bins.is_empty() {
        sb.push("No lowest bin data available for these items".to_owned());
        return;
    }

    let mut current_budget = budget;
    let mut auctions = HashMap::new();
    let mut switched_budget = false;

    loop {
        let mut found_auctions = false;

        for (item_id, auction_list) in &auctions_by_id {
            if let Some(&lowest_bin) = lowest_bins.get(item_id) {
                let max_price = get_max_price(lowest_bin, &current_budget);
                for auction in auction_list {
                    if *auction.price() < max_price {
                        auctions.insert(auction.auction_id().to_owned(), auction.clone());
                        found_auctions = true;
                    }
                }
            }
        }

        if found_auctions {
            break;
        }

        // Escalate to next budget or break
        current_budget = match current_budget {
            Budget::Low => Budget::Medium,
            Budget::Medium => Budget::High,
            Budget::High => Budget::NoLimit,
            Budget::NoLimit => {
                sb.push("Couldn't find any auctions by that name".to_owned());
                return;
            }
        };
        switched_budget = true;
    }

    let mut valued_items = Vec::new();
    for (auction_id, auction) in auctions {
        let value = calculate_item_value(auction.item_id(), auction.item_nbt(), true, false).await;
        let price = *auction.price();
        let estimated_value = value.value();
        if price > 0 {
            valued_items.push((auction_id, auction, value, estimated_value as f64 - price as f64));
        }
    }

    valued_items.sort_by(|a, b| {
        b.3.partial_cmp(&a.3)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.price().cmp(b.1.price()))
    });

    if let Some((auction_id, auction, value, profit)) = valued_items.first() {
        let auctioneer_username = get_player_username(auction.auctioneer()).await.unwrap_or("Unknown".to_owned());
        let price = *auction.price();
        let lowest_bin = *lowest_bins.get(auction.item_id()).unwrap_or(&0);
        let estimated_value = value.value();
        sb.push(format!("Item Name: {}", auction.item_name()));
        sb.push(format!("Auction ID: {auction_id}"));
        sb.push(format!("Auctioneer Username: {auctioneer_username}"));
        sb.push(format!("LowestBIN: {} coins", format_number(lowest_bin)));
        sb.push(format!("Price: {} coins", format_number(price)));
        sb.push(format!("Estimated Item Value: {} coins", format_number(estimated_value)));
        if switched_budget {
            sb.push(format!("No matches were found in your budget, so the search was expanded to {current_budget}."));
        }
        match price == lowest_bin {
            true => sb.push("This item is the lowestBIN!".to_owned()),
            false => sb.push(format!("Profit: {} coins", format_number(*profit as u64))),
        }
        if value.info().len() > 3 {
            sb.push("Item Details:".to_owned());
            for line in value.info().iter().skip(1) {
                if line.contains("Estimated Item Value") {
                    continue;
                };
                sb.push(format!("- {line}"));
            }
        }
    }

    let others = valued_items.into_iter().skip(1).take(3);
    if others.len() > 0 {
        sb.pushln();
        sb.push("Other Auctions:".to_owned());
    }

    for (auction_id, auction, value, profit) in others {
        sb.push(format!("- Item Name: {} (Auction ID: {auction_id})", auction.item_name()));
        sb.push(format!(
            "  Price: {} coins, Estimated Item Value: {} coins, Profit: {} coins",
            format_number(*auction.price()),
            format_number(value.value()),
            format_number(profit as u64)
        ));
    }
}

pub async fn get_base_price(item_id: &str) -> Option<u64> {
    AUCTION_MANAGER.lowest_bins.read().await.get(item_id).map(|i| *i.base_price())
}

pub async fn get_lowest_bin(item_id: &str) -> Option<u64> {
    match AUCTION_MANAGER.lowest_bins.read().await.get(item_id).map(|i| *i.price()) {
        Some(p) => Some(p),
        None => get_cosmetic_price(item_id).await,
    }
}

pub async fn get_lowest_bin_and_id(item_id: &str) -> Option<(u64, String)> {
    AUCTION_MANAGER.lowest_bins.read().await.get(item_id).map(|i| (*i.price(), i.auction_id().to_owned()))
}

pub async fn get_auction_by_id(auction_id: &str) -> Option<AuctionItem> {
    AUCTION_MANAGER.auctions.read().await.get(auction_id).cloned()
}

pub async fn get_auctions_by_player(auctioneer_id: &str) -> Vec<AuctioneerAuctionItem> {
    AUCTION_MANAGER.auctions.read().await
        .iter()
        .filter(|(_, auction)| auction.auctioneer() == auctioneer_id)
        .map(|(id, auction)| AuctioneerAuctionItem {
            auction_id: id.clone(),
            item_name: auction.item_name().to_owned(),
            price: *auction.price(),
        })
        .collect()
}