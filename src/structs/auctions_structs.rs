use crate::structs::item_structs::{ItemNbt, ItemValue};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::RwLock;

pub struct AuctionManager {
    pub auctions: RwLock<FxHashMap<String, AuctionItem>>,
    pub lowest_bins: RwLock<FxHashMap<String, LowestBinItem>>,
    pub sorted_item_values: RwLock<FxHashMap<String, Vec<String>>>,
    pub player_auctions: RwLock<FxHashMap<String, HashSet<String>>>,
    pub to_add: RwLock<FxHashMap<String, AuctionItem>>,
    pub to_keep: RwLock<HashSet<String>>,
}

impl AuctionManager {
    pub fn new() -> Self {
        Self {
            auctions: RwLock::new(FxHashMap::with_capacity_and_hasher(60000, Default::default())),
            lowest_bins: RwLock::new(FxHashMap::with_capacity_and_hasher(12000, Default::default())),
            sorted_item_values: RwLock::new(FxHashMap::with_capacity_and_hasher(60000, Default::default())),
            player_auctions: RwLock::new(FxHashMap::with_capacity_and_hasher(25000, Default::default())),
            to_add: RwLock::new(FxHashMap::with_capacity_and_hasher(60000, Default::default())),
            to_keep: RwLock::new(HashSet::with_capacity_and_hasher(60000, Default::default())),
        }
    }

    pub async fn start_update(&self) {
        self.to_add.write().await.clear();
        self.to_keep.write().await.clear();
    }
}

#[derive(Deserialize, Debug)]
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

impl AuctionsResponse {
    pub fn is_successful(&self) -> bool { self.success }
    pub fn total_pages(&self) -> u64 { self.total_pages }
    pub fn total_auctions(&self) -> u64 { self.total_auctions }
    pub fn last_updated(&self) -> u64 { self.last_updated }
    pub fn get_auctions(&self) -> &[Auction] { &self.auctions }
}

#[derive(Deserialize, Debug)]
pub struct Auction {
    uuid: String,
    auctioneer: String,
    item_name: String,
    item_bytes: String,
    starting_bid: u64,
    bin: bool,
}

impl Auction {
    pub fn uuid(&self) -> &str { &self.uuid }
    pub fn auctioneer(&self) -> &str { &self.auctioneer }
    pub fn item_name(&self) -> &str { &self.item_name }
    pub fn item_bytes(&self) -> &str { &self.item_bytes }
    pub fn starting_bid(&self) -> u64 { self.starting_bid }
    pub fn is_bin(&self) -> bool { self.bin }
}

#[derive(Clone)]
pub struct AuctionItem {
    auctioneer: String,
    item_uuid: String,
    item_name: String,
    item_id: String,
    item_nbt: ItemNbt,
    value: ItemValue,
    price: u64,
}

impl AuctionItem {
    pub fn new(auction: &Auction, item_uuid: String, item_id: String, item_nbt: ItemNbt) -> Self {
        Self {
            auctioneer: (*auction.auctioneer).to_string(),
            item_name: (*auction.item_name).to_string(),
            value: ItemValue::new(),
            price: auction.starting_bid,
            item_uuid,
            item_id,
            item_nbt,
        }
    }

    pub fn auctioneer(&self) -> &str { &self.auctioneer }
    pub fn item_name(&self) -> &str { &self.item_name }
    pub fn item_uuid(&self) -> &str { &self.item_uuid }
    pub fn item_id(&self) -> &str { &self.item_id }
    pub fn item_nbt(&self) -> &ItemNbt { &self.item_nbt }
    pub fn item_value(&self) -> &ItemValue { &self.value }
    pub fn price(&self) -> u64 { self.price }

    pub fn set_value(&mut self, value: ItemValue) { self.value = value }
}

#[derive(Clone)]
pub struct LowestBinItem {
    auction_id: String,
    item_id: String,
    price: u64,
    base_price: u64,
}

impl LowestBinItem {
    pub fn new(auction_id: String, item_id: String, price: u64) -> Self {
        Self { auction_id, item_id, price, base_price: price }
    }
    pub fn auction_id(&self) -> &str { &self.auction_id }
    pub fn item_id(&self) -> &str { &self.item_id }
    pub fn price(&self) -> u64 { self.price }
    pub fn base_price(&self) -> u64 { self.base_price }

    pub fn set_base_price(&mut self, base_price: u64) {
        self.base_price = base_price;
    }
}

#[derive(Serialize)]
pub struct AuctionItemResponse {
    pub auctioneer: String,
    pub item_uuid: String,
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
            item_uuid: item.item_uuid.clone(),
            item_name: item.item_name.clone(),
            item_id: item.item_id.clone(),
            price: item.price,
            total_value: item.value.value(),
            info: item.value.info(),
        }
    }
}