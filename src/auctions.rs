use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::Deref;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use rustc_hash::FxHashMap;
use sea_orm::Iden;
use sea_orm::sea_query::ExprTrait;
use sea_orm::sqlx::types::chrono::Local;
use tokio::sync::RwLock;
use tokio::time::{interval, interval_at, sleep};
use crate::bazaar::BAZAAR;
pub(crate) use crate::item_utils::{decode_base64, get_item_id};
use crate::item_utils::get_item_uuid;
use crate::item_value_calculator;
use crate::statics::HTTP_CLIENT;
use crate::structs::{Auction, AuctionItem, AuctionsResponse, ItemNbt, PriceDataSource, SharedPriceData};
use crate::structs::PriceDataSource::LowestBin;

const API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/auctions";
const THRESHOLD: u64 = 70;
const MIN_DELAY_SECS: u64 = 20;
const MAX_RETRIES: u64 = 3;
const MAX_CONCURRENT_REQUESTS: usize = 10;


pub struct AuctionManager {
    auctions: RwLock<FxHashMap<String, AuctionItem>>,
    lowest_bins: RwLock<FxHashMap<String, (String, SharedPriceData)>>,
    sorted_item_values: RwLock<FxHashMap<String, Vec<String>>>,
    player_auctions: RwLock<FxHashMap<String, HashSet<String>>>,

    to_add: RwLock<FxHashMap<String, AuctionItem>>,
    to_remove: RwLock<FxHashMap<String, String>>,
    to_keep: RwLock<HashSet<String>>,
}

impl AuctionManager {
    pub fn new() -> Self {
        Self {
            auctions: RwLock::new(FxHashMap::with_capacity_and_hasher(60000, Default::default())),
            lowest_bins: RwLock::new(FxHashMap::with_capacity_and_hasher(12000, Default::default())),
            sorted_item_values: RwLock::new(FxHashMap::with_capacity_and_hasher(60000, Default::default())),
            player_auctions: RwLock::new(FxHashMap::with_capacity_and_hasher(25000, Default::default())),
            to_add: RwLock::new(FxHashMap::with_capacity_and_hasher(60000, Default::default())),
            to_remove: RwLock::new(FxHashMap::with_capacity_and_hasher(60000, Default::default())),
            to_keep: RwLock::new(HashSet::with_capacity_and_hasher(60000, Default::default()))
        }
    }

    pub async fn start_update(&self) {
        self.to_add.write().await.clear();
        self.to_remove.write().await.clear();
        self.to_keep.write().await.clear();
    }
}

static AUCTION_MANAGER: LazyLock<AuctionManager> = LazyLock::new(|| AuctionManager::new());

pub fn schedule() {
    tokio::spawn(async {
        let mut ticker = interval(Duration::from_secs(THRESHOLD));
        loop {
            ticker.tick().await;
            match update().await {
                Ok(last_updated) => {
                    let next_update_time = (last_updated / 1000) + THRESHOLD;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    let delay = Duration::from_secs(match next_update_time > now {
                        true => max(next_update_time - now, MIN_DELAY_SECS),
                        false => MIN_DELAY_SECS,
                    });

                    ticker = interval_at(tokio::time::Instant::now() + delay, Duration::from_secs(THRESHOLD));
                    let formatted = Local::now() + Duration::from_secs(delay.as_secs());
                    println!("[Auctions] Next update in {:.1} seconds (at {})", delay.as_secs(), formatted.format("%H:%M:%S"));
                },
                Err(err) => {
                    eprintln!("[Auctions] Error: {:?}", err);
                    ticker = interval_at(tokio::time::Instant::now() + Duration::from_secs(MIN_DELAY_SECS), Duration::from_secs(THRESHOLD));
                }
            }
        }
    });
}

async fn update() -> Result<u64, Box<dyn Error + Send + Sync>> {
    println!("[Auctions] Starting Auctions update...");
    let total_start = Instant::now();

    let first_page = fetch_page(0).await?;
    let total_pages = first_page.total_pages();
    let total_auctions = first_page.total_auctions();
    let last_updated = first_page.last_updated();
    println!("[Auctions] Fetching {} auctions in {} pages...", total_auctions, total_pages);

    AUCTION_MANAGER.start_update().await;

    process_page(first_page.get_auctions()).await;

    let mut tasks = FuturesUnordered::new();
    let mut next_page = 1;

    while next_page < total_pages || !tasks.is_empty() {
        // Fill up the buffer
        while tasks.len() < MAX_CONCURRENT_REQUESTS && next_page < total_pages {
            let page = next_page;
            tasks.push(tokio::spawn(async move {
                match fetch_page(page).await {
                    Ok(page_data) => {
                        println!("[Auctions] Fetched page {}, found {} auctions", page, page_data.get_auctions().len());
                        process_page(page_data.get_auctions()).await;
                    }
                    Err(e) => eprintln!("[Auctions] Failed to fetch page {}: {}", page, e)
                }
            }));
            next_page += 1;
        }

        tasks.next().await;
    }

    let process_start = Instant::now();
    let edited_items = update_auctions_list().await;
    println!("[Auctions] Auctions Processing time: {:.2?}", process_start.elapsed());

    let process_lowest_bin_start = Instant::now();
    update_lowest_bin_list(edited_items).await;
    println!("[Auctions] Auctions Processing LowestBIN time: {:.2?}", process_lowest_bin_start.elapsed());

    let calculate_base_prices_start = Instant::now();
    calculate_base_prices().await;
    println!("[Auctions] Auctions Calculating base values time: {:.2?}", calculate_base_prices_start.elapsed());

    let updating_values_start = Instant::now();
    update_auctions_values().await;
    println!("[Auctions] Auctions Updating values time: {:.2?}", updating_values_start.elapsed());

    // let lowest_bins = AUCTION_MANAGER.lowest_bins.read().await;
    // for (k, v) in lowest_bins.iter() {
    //     let i = v.1.read().await;
    //     if let LowestBin { price, clean, base_price } = *i {
    //         if price != base_price {
    //             println!("{k}: {price} | {base_price}")
    //         }
    //     }
    // }

    // let read = AUCTION_MANAGER.auctions.read().await;
    // for (k, v) in read.iter().take(100) {
    //     let value = v.value();
    //     let modifiers = value.modifiers();
    //     if !modifiers.is_empty() {
    //         println!("{}: total Value: {}", v.item_id(), value.total_value());
    //         for modifier in modifiers {
    //             if let Some(price) = modifier.1.price() {
    //                 let read_price = price.read().await;
    //                 let the_price = read_price.get_price();
    //                 println!("  {}: {}", modifier.0, the_price);
    //             } else {
    //                 println!("Modifier {} got ingredients", modifier.0);
    //                 if let Some(ingredients) = modifier.1.ingredients() {
    //                     for ingredient in ingredients {
    //                         if let Some(price) = ingredient.1.price() {
    //                             let read_price = price.read().await;
    //                             let the_price = read_price.get_price();
    //                             println!("  ingredient: {}: {}", ingredient.0, the_price);
    //                         } else {
    //                             println!("ingredient does not have a price");
    //                         }
    //                     }
    //                 } else {
    //                     println!("Modifier does not have any ingredients SOMEHOW")
    //                 }
    //             }
    //         }
    //         println!("-------------");
    //     }
    // }

    println!("[Auctions] Auctions Total time: {:.2?}", total_start.elapsed());
    Ok(last_updated)
}

async fn fetch_page(page: u64) -> Result<AuctionsResponse, Box<dyn Error + Send + Sync>> {
    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let delay = 1 * attempt.pow(2);
            println!("[Auctions] Retrying fetching page {page} in {:.1}sc", delay);
            sleep(Duration::from_secs(delay)).await;
        }

        let resp = HTTP_CLIENT.get(format!("{API_ENDPOINT}?page={page}"))
            .send().await?.text().await?;

        let page_response: AuctionsResponse = serde_json::from_str(&resp)?;

        if page_response.is_successful() {
            return Ok(page_response)
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
    if !auction.is_bin() { return }

    let auction_id = auction.uuid();

    AUCTION_MANAGER.to_keep.write().await.insert(auction_id.to_string());
    if AUCTION_MANAGER.auctions.read().await.contains_key(auction_id) { return; };

    let item_nbt = match decode_base64(auction.item_bytes()) {
        Ok(v) => v,
        _ => return
    };

    let item_id = match get_item_id(&item_nbt) {
        Some(v) => v,
        _ => return
    };

    let item_uuid = match get_item_uuid(&item_nbt) {
        Some(v) => v,
        _ => return
    };

    let auction_item = AuctionItem::new(auction, item_uuid, item_id, item_nbt);

    {
        let mut to_add = AUCTION_MANAGER.to_add.write().await;
        to_add.insert(auction_id.to_string(), auction_item);
    }
}

async fn update_auctions_list() -> HashSet<String> {
    let mut auctions = AUCTION_MANAGER.auctions.write().await;
    let mut to_add = AUCTION_MANAGER.to_add.write().await;
    let to_keep = AUCTION_MANAGER.to_keep.read().await;
    let mut edited_items: HashSet<String> = HashSet::new();

    for (id, auction_item) in to_add.drain() {
        edited_items.insert(auction_item.item_id().to_string());
        auctions.insert(id, auction_item);
    }

    let mut to_remove = AUCTION_MANAGER.to_remove.write().await;
    for (auction_id, auction) in auctions.iter() {
        if to_keep.contains(auction_id) { continue };
        edited_items.insert(auction.item_id().to_string());
        to_remove.insert(auction_id.to_string(), auction.item_id().to_string());
    }

    for k in to_remove.keys() {
        auctions.remove(k);
    }

    edited_items
}

async fn update_lowest_bin_list(edited_items: HashSet<String>) {
    let edited_items: HashSet<&str> = edited_items.iter().map(|s| s.as_str()).collect();
    let auctions = AUCTION_MANAGER.auctions.read().await;
    let mut temp_map = HashMap::new();

    for (auction_id, auction) in auctions.iter() {
        let item_id = auction.item_id();
        if !edited_items.contains(item_id) {
            continue;
        }

        match temp_map.get(item_id) {
            None => { temp_map.insert(item_id, (auction_id, auction.price())); },
            Some(existing_item) => {
                if existing_item.1 <= auction.price() { continue };
                temp_map.insert(item_id, (auction_id, auction.price()));
            }
        }
    }

    let mut lowest_bins = AUCTION_MANAGER.lowest_bins.write().await;

    for (item_id, (auction_id, price)) in temp_map {
        match lowest_bins.get(item_id) {
            None => {
                lowest_bins.insert(
                    item_id.to_string(),
                    (
                        auction_id.to_string(),
                        SharedPriceData::new(RwLock::new(LowestBin {
                            price,
                            clean: false,
                            base_price: price
                        }))
                    )
                );
            },
            Some(existing_item) => {
                if existing_item.0 == auction_id.to_string() { continue };
                let shared_price_data = Arc::clone(&existing_item.1);
                {
                    let mut data = shared_price_data.write().await;
                    *data = LowestBin {
                        price,
                        clean: false,
                        base_price: price,
                    };
                }

                lowest_bins.insert(item_id.to_string(), (auction_id.to_string(), shared_price_data));
            }
        }
    }
}

async fn calculate_base_prices() {
    let lowest_bins = AUCTION_MANAGER.lowest_bins.write().await;
    let auctions = AUCTION_MANAGER.auctions.read().await;
    for (id, (auction_id, price)) in lowest_bins.iter() {
        let mut shared_price = price.write().await;
        if let LowestBin { price, clean, ..} = *shared_price {
            if clean { continue };
            if let Some(auction_item) = auctions.get(auction_id) {
                let value = auction_item.value();
                if value.modifiers_to_process().is_empty() && value.modifiers().is_empty() {
                    *shared_price = LowestBin {
                        price,
                        clean: true,
                        base_price: price,
                    };
                    continue;
                }
                let mut modifiers_price = 0.0;
                for modifier in value.modifiers() {
                    modifiers_price += modifier.1.calculate_price().await;
                    if let Some(ingredients) = &modifier.1.ingredients() {
                        for ingredient in ingredients {
                            modifiers_price += ingredient.1.calculate_price().await;
                        }
                    }
                }
                *shared_price = LowestBin {
                    price,
                    clean: true,
                    base_price: price - modifiers_price,
                };
            }
        }
    }
}

async fn update_auctions_values() {
    let mut auctions = AUCTION_MANAGER.auctions.write().await;
    let lowest_bins = AUCTION_MANAGER.lowest_bins.read().await;
    for auction in auctions.iter_mut() {
        let auction_item = auction.1;
        item_value_calculator::calculate_auction_value(auction_item).await;
        if let Some(i) = lowest_bins.get(auction_item.item_id()) {
            let shared_price = i.1.read().await;
            if let LowestBin { base_price, ..} = *shared_price {
                auction_item.value_mut().calculate_total(base_price).await;
            }
        }
    }
}


pub async fn get_shared_lowest_bin(item_id: &str) -> Option<SharedPriceData> {
    let lowest_bin_list = AUCTION_MANAGER.lowest_bins.read().await;
    let (_, price_data) = lowest_bin_list.get(item_id)?;
    Some(Arc::clone(price_data))
}

pub async fn get_lowest_bin(item_id: &str, return_id: bool) -> Option<(f64, Option<String>)> {
    let lowest_bin_list = AUCTION_MANAGER.lowest_bins.read().await;
    let (auction_id, price_data) = lowest_bin_list.get(item_id)?;
    let price_data = price_data.read().await;
    if let LowestBin { price, .. } = *price_data {
        let id = if return_id { Some(auction_id.clone()) } else { None };
        return Some((price, id));
    }
    None
}

pub async fn get_auction_by_auction_id(auction_id: &str) -> Option<AuctionItem> {
    let auctions = AUCTION_MANAGER.auctions.read().await;
    auctions.get(auction_id).cloned()
}

pub async fn get_auction_by_item_uuid(item_uuid: &str) -> Option<AuctionItem> {
    let auctions = AUCTION_MANAGER.auctions.read().await;
    auctions.values()
        .find(|auction| auction.item_uuid() == item_uuid)
        .cloned()
}

pub async fn get_auction_ids_by_auctioneer(auctioneer_id: &str) -> Option<Vec<crate::AuctioneerAuctionItem>> {
    let auctions = AUCTION_MANAGER.auctions.read().await;
    let mut result = Vec::new();

    for (id, auction) in auctions.iter() {
        if auction.auctioneer() == auctioneer_id {
            result.push(crate::AuctioneerAuctionItem {
                auction_id: id.to_string(),
                item_name: auction.item_name().to_string(),
                price: auction.price(),
            });
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}



pub fn get_auction_manager() -> &'static AuctionManager {
    &AUCTION_MANAGER
}