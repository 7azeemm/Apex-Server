use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use fastnbt::Value;
use sea_orm::Iden;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::item_utils::extract_modifiers;

// Bazaar
#[derive(Deserialize, Debug)]
pub struct BazaarResponse {
    success: bool,
    #[serde(rename = "lastUpdated")]
    last_updated: u64,
    products: HashMap<String, Product>,
}

#[derive(Deserialize, Debug)]
pub struct Product {
    #[serde(rename = "quick_status")]
    quick_status: PriceData,
}

#[derive(Deserialize, Debug)]
pub struct PriceData {
    #[serde(rename = "sellPrice")]
    sell_price: f64,
    #[serde(rename = "buyPrice")]
    buy_price: f64,
}

impl BazaarResponse {
    pub fn is_successful(&self) -> bool { self.success }
    pub fn last_updated(&self) -> u64 { self.last_updated }
    pub fn get_products(&self) -> &HashMap<String, Product> { &self.products }
}

impl Product {
    pub fn sell_price(&self) -> f64 { self.quick_status.sell_price }
    pub fn buy_price(&self)  -> f64 { self.quick_status.buy_price }
}

impl PriceData {
    pub fn new(buy_price: f64, sell_price: f64) -> Self {
        Self { buy_price, sell_price }
    }
    pub fn set_prices(&mut self, sell_price: f64, buy_price: f64) {
        self.sell_price = sell_price;
        self.buy_price = buy_price;
    }
}

// Auctions
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

#[derive(Deserialize, Debug)]
pub struct Auction {
    uuid: String,
    auctioneer: String,
    item_name: String,
    item_bytes: String,
    starting_bid: u64,
    bin: bool,
}

#[derive(Clone)]
pub struct AuctionItem {
    auctioneer: String,
    item_uuid: String,
    item_name: String,
    item_id: String,
    item_nbt: ItemNbt,
    value: ItemValue,
    price: f64
}

#[derive(Serialize)]
pub struct ModifierResponse {
    pub count: i32,
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingredients: Option<HashMap<String, SimpleModifier>>,
}

#[derive(Serialize)]
pub struct SimpleModifier {
    pub count: i32,
    pub price: Option<f64>,
}

impl ModifierResponse {
    pub async fn from_modifier(modifier: &Modifier) -> Self {
        let price = if let Some(price_data) = &modifier.price {
            Some(price_data.read().await.get_price())
        } else {
            None
        };

        let ingredients = if let Some(ingredients) = &modifier.ingredients {
            let mut ingredient_map = HashMap::new();
            for (key, ingredient) in ingredients {
                let ingredient_price = if let Some(price_data) = &ingredient.price {
                    Some(price_data.read().await.get_price())
                } else {
                    None
                };

                ingredient_map.insert(key.clone(), SimpleModifier {
                    count: ingredient.count,
                    price: ingredient_price,
                });
            }
            Some(ingredient_map)
        } else {
            None
        };

        Self {
            count: modifier.count,
            price,
            ingredients,
        }
    }
}

#[derive(Serialize)]
pub struct AuctionItemResponse {
    pub auctioneer: String,
    pub item_uuid: String,
    pub item_name: String,
    pub item_id: String,
    pub price: f64,
    pub total_value: f64,
    pub modifiers: HashMap<String, ModifierResponse>,
}

impl AuctionItemResponse {
    pub async fn from_auction_item(item: &AuctionItem) -> Self {
        let mut modifiers = HashMap::new();
        for (key, modifier) in &item.value.modifiers {
            modifiers.insert(key.clone(), ModifierResponse::from_modifier(modifier).await);
        }

        Self {
            auctioneer: item.auctioneer.clone(),
            item_uuid: item.item_uuid.clone(),
            item_name: item.item_name.clone(),
            item_id: item.item_id.clone(),
            price: item.price,
            total_value: item.value.total_value,
            modifiers,
        }
    }
}

impl AuctionsResponse {
    pub fn is_successful(&self) -> bool { self.success }
    pub fn total_pages(&self) -> u64 { self.total_pages }
    pub fn total_auctions(&self) -> u64 { self.total_auctions }
    pub fn last_updated(&self) -> u64 { self.last_updated }
    pub fn get_auctions(&self) -> &[Auction] { &self.auctions }
}

impl Auction {
    pub fn uuid(&self) -> &str { &self.uuid }
    pub fn auctioneer(&self) -> &str { &self.auctioneer }
    pub fn item_name(&self) -> &str { &self.item_name }
    pub fn item_bytes(&self) -> &str { &self.item_bytes }
    pub fn starting_bid(&self) -> u64 { self.starting_bid }
    pub fn is_bin(&self) -> bool { self.bin }
}

impl AuctionItem {
    pub fn new(auction: &Auction, item_uuid: String, item_id: String, item_nbt: ItemNbt) -> Self {
        Self {
            auctioneer: (*auction.auctioneer).to_string(),
            item_name: (*auction.item_name).to_string(),
            value: ItemValue::new(&item_nbt),
            price: auction.starting_bid as f64,
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
    pub fn value(&self) -> &ItemValue { &self.value }
    pub fn value_mut(&mut self) -> &mut ItemValue { &mut self.value }
    pub fn price(&self) -> f64 { self.price }
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct ItemNbt {
    #[serde(rename = "Count")]
    pub count: u8,
    pub tag: Option<ItemTag>,
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct ItemTag {
    #[serde(rename = "ExtraAttributes")]
    pub extra_attributes: Option<Value>,
}

pub type SharedPriceData = Arc<RwLock<PriceDataSource>>;

#[derive(Debug, Clone)]
pub enum PriceDataSource {
    Bazaar { buy_price: f64, sell_price: f64 },
    LowestBin { price: f64, clean: bool, base_price: f64 },
    NPC { price: f64 }
}

impl PriceDataSource {
    pub fn get_price(&self) -> f64 {
        match self {
            PriceDataSource::Bazaar { buy_price, .. } => *buy_price,
            PriceDataSource::LowestBin { price, .. } => *price,
            PriceDataSource::NPC { price } => *price,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ItemValue {
    modifiers: HashMap<String, Modifier>,
    modifiers_to_process: HashSet<String>,
    total_value: f64,
}

#[derive(Debug, Clone)]
pub struct Modifier {
    count: i32,
    price: Option<SharedPriceData>,
    ingredients: Option<HashMap<String, Modifier>>
}

impl Modifier {
    pub fn new (count: i32, price: SharedPriceData) -> Self {
        Self { count, price: Some(price), ingredients: None }
    }
    pub fn new_one (price: SharedPriceData) -> Self {
        Self { count: 1, price: Some(price), ingredients: None }
    }
    pub fn new_craftable (ingredients: HashMap<String, Modifier>) -> Self {
        Self { count: 1, price: None, ingredients: Some(ingredients) }
    }
    pub fn count(&self) -> i32 { self.count }
    pub fn price(&self) -> &Option<SharedPriceData> { &self.price }
    pub fn ingredients(&self) -> &Option<HashMap<String, Modifier>> { &self.ingredients }
    pub async fn calculate_price(&self) -> f64 {
        let mut total = 0.0;
        if let Some(price) = &self.price {
            let read_price = price.read().await;
            total += read_price.get_price() * (self.count as f64);
        }
        total
    }
}

impl ItemValue {
    pub fn new(item_nbt: &ItemNbt) -> Self {
        Self {
            modifiers: HashMap::new(),
            modifiers_to_process: extract_modifiers(item_nbt),
            total_value: -1.0
        }
    }

    pub fn total_value(&self) -> &f64 { &self.total_value }
    pub fn modifiers(&self) -> &HashMap<String, Modifier> { &self.modifiers }
    pub fn modifiers_mut(&mut self) -> &mut HashMap<String, Modifier> { &mut self.modifiers }
    pub fn modifiers_to_process(&self) -> &HashSet<String> { &self.modifiers_to_process }
    pub fn modifiers_to_process_mut(&mut self) -> &mut HashSet<String> { &mut self.modifiers_to_process }
    pub fn add_modifier(&mut self, id: &str, modifier: Modifier) {
        self.modifiers.insert(id.to_string(), modifier);
    }
    pub async fn calculate_total(&mut self, base_value: f64) {
        let mut total = base_value;
        for modifier in self.modifiers.values() {
            total += modifier.calculate_price().await;
            if let Some(ingredients) = &modifier.ingredients {
                for ingredient in ingredients {
                    total += ingredient.1.calculate_price().await;
                }
            }
        }
        self.total_value = total;
    }
}