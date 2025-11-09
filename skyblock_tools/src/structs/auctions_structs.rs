use crate::structs::item_structs::{ItemNbt, ItemValue};
use derive_new::new;
use getset::Getters;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use tokio::sync::RwLock;

pub struct AuctionManager {
    pub auctions: RwLock<FxHashMap<String, AuctionItem>>,
    pub lowest_bins: RwLock<FxHashMap<String, LowestBinItem>>,
    pub to_add: RwLock<FxHashMap<String, AuctionItem>>,
    pub to_keep: RwLock<HashSet<String>>,
}

impl AuctionManager {
    pub fn new() -> Self {
        Self {
            auctions: RwLock::new(FxHashMap::with_capacity_and_hasher(60000, Default::default())),
            lowest_bins: RwLock::new(FxHashMap::with_capacity_and_hasher(4000, Default::default())),
            to_add: RwLock::new(FxHashMap::with_capacity_and_hasher(60000, Default::default())),
            to_keep: RwLock::new(HashSet::with_capacity_and_hasher(60000, Default::default())),
        }
    }

    pub async fn start_update(&self) {
        self.to_add.write().await.clear();
        self.to_keep.write().await.clear();
    }
}

#[derive(Deserialize, Debug, Getters)]
#[getset(get = "pub")]
pub struct AuctionsResponse {
    success: bool,
    #[serde(rename = "totalPages")]
    total_pages: u64,
    #[serde(rename = "totalAuctions")]
    total_auctions: u64,
    #[serde(rename = "lastUpdated")]
    last_updated: u64,
    auctions: Vec<Auction>,
}

#[derive(Deserialize, Debug, Getters)]
#[getset(get = "pub")]
pub struct Auction {
    uuid: String,
    auctioneer: String,
    item_name: String,
    item_bytes: String,
    starting_bid: u64,
    bin: bool,
}

#[derive(Clone, Getters)]
#[getset(get = "pub")]
pub struct AuctionItem {
    auction_id: String,
    auctioneer: String,
    item_name: String,
    item_id: String,
    item_nbt: ItemNbt,
    value: ItemValue,
    price: u64,
}

impl AuctionItem {
    pub fn new(id: String, auction: &Auction, item_id: String, item_nbt: ItemNbt) -> Self {
        Self {
            auction_id: id,
            auctioneer: auction.auctioneer.clone(),
            item_name: auction.item_name.clone(),
            value: ItemValue::default(),
            price: auction.starting_bid,
            item_id,
            item_nbt,
        }
    }

    pub fn set_value(&mut self, value: ItemValue) {
        self.value = value
    }
}

#[derive(Clone, new, Getters)]
#[getset(get = "pub")]
pub struct LowestBinItem {
    auction_id: String,
    item_id: String,
    price: u64,
    base_price: u64,
}

impl LowestBinItem {
    pub fn set_base_price(&mut self, base_price: u64) {
        self.base_price = base_price;
    }
}

#[derive(Clone)]
pub enum Budget {
    Low,
    Medium,
    High,
    NoLimit,
}

impl Display for Budget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Budget::Low => write!(f, "Low"),
            Budget::Medium => write!(f, "Medium"),
            Budget::High => write!(f, "High"),
            Budget::NoLimit => write!(f, "NoLimit"),
        }
    }
}

#[derive(Serialize)]
pub struct AuctionItemResponse {
    pub auctioneer: String,
    pub item_name: String,
    pub item_id: String,
    pub price: u64,
    pub total_value: u64,
    pub info: Vec<String>,
}

impl AuctionItemResponse {
    pub async fn from_auction_item(item: &AuctionItem) -> Self {
        Self {
            auctioneer: item.auctioneer.clone(),
            item_name: item.item_name.clone(),
            item_id: item.item_id.clone(),
            price: item.price,
            total_value: item.value.value(),
            info: item.value.info(),
        }
    }
}
