use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use fastnbt::Value;
use sea_orm::Iden;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::auctions::{get_base_price, get_shared_lowest_bin};
use crate::item_utils::{format_number};
use crate::item_value_calculator::MODIFIERS;

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
pub struct AuctionItemResponse {
    pub auctioneer: String,
    pub item_uuid: String,
    pub item_name: String,
    pub item_id: String,
    pub price: f64,
    pub total_value: f64,
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
            total_value: item.value.total_value,
            info: item.value.build_info_string(&item.item_id, true).await,
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
            value: ItemValue::new(),
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
    pub display: Option<Value>,
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
    total_value: f64,
}

#[derive(Debug, Clone)]
pub struct Modifier {
    count: i32,
    price: Option<SharedPriceData>,
    ingredients: Option<HashMap<String, Modifier>>,
    info: ModifierInfo
}

#[derive(Debug, Clone)]
pub struct ModifierInfo {
    group: String,
    info: String,
}

impl ModifierInfo {
    pub fn new (group: &str, info: String) -> Self {
        Self { group: group.to_string(), info }
    }

    pub fn group(&self) -> &str { &self.group }
    pub fn info(&self) -> &str { &self.info }
}

impl Modifier {
    pub fn new (count: i32, price: Option<SharedPriceData>, info: ModifierInfo) -> Self {
        Self { count, price, ingredients: None, info }
    }
    pub fn new_one (price: Option<SharedPriceData>, info: ModifierInfo) -> Self {
        Self { count: 1, price, ingredients: None, info }
    }
    pub fn new_craftable (ingredients: HashMap<String, Modifier>, info: ModifierInfo) -> Self {
        Self { count: 1, price: None, ingredients: Some(ingredients), info }
    }
    pub fn count(&self) -> i32 { self.count }
    pub fn ingredients(&self) -> &Option<HashMap<String, Modifier>> { &self.ingredients }
    pub fn info(&self) -> &ModifierInfo { &self.info }
    pub async fn get_price(&self) -> Option<f64> {
        if let Some(price) = &self.price {
            let price = price.read().await.get_price();
            return Some(price * (self.count as f64))
        };

        None
    }
    pub async fn calculate_price(&self) -> f64 {
        let mut total = 0.0;
        if let Some(price) = &self.get_price().await {
            total += price;
        }
        total
    }
}

impl ItemValue {
    pub fn new() -> Self {
        Self {
            modifiers: HashMap::new(),
            total_value: -1.0
        }
    }

    pub fn total_value(&self) -> &f64 { &self.total_value }
    pub fn modifiers(&self) -> &HashMap<String, Modifier> { &self.modifiers }
    pub fn add_modifier(&mut self, id: &str, modifier: Modifier) {
        self.modifiers.insert(id.to_string(), modifier);
    }
    pub async fn build_info_string(&self, item_id: &str, include_price_info: bool) -> Vec<String> {
        let modifiers = &self.modifiers;
        let mut lines = Vec::new();
        let mut groups: HashMap<&str, Vec<(String, bool)>> = HashMap::new();
        let mut total_value = 0.0;

        if include_price_info && !modifiers.is_empty() {
            if let Some(base_value) = get_base_price(item_id).await {
                lines.push(format!("Base Price: {}", format_number(base_value as u64)));
                total_value += base_value;
            }
        }

        for modifier in modifiers.values() {
            let mut ingredients_lines = Vec::new();
            let info = modifier.info();
            let mut text = info.info().to_string();
            if include_price_info && let Some(price) = modifier.get_price().await && price > 1.0 {
                text = format!("{} ({})", text, format_number(price as u64));
                total_value += price;
            }

            if let Some(ingredients) = modifier.ingredients() {
                for ingredient in ingredients.values() {
                    let mut ingredient_text = ingredient.info().info().to_string();
                    if include_price_info && let Some(price) = ingredient.get_price().await && price > 1.0 {
                        ingredient_text = format!("{} ({})", ingredient_text, format_number(price as u64));
                        total_value += price;
                    }
                    ingredients_lines.push(ingredient_text)
                }
            }

            if groups.contains_key(info.group()) {
                let mut existing_list: &mut Vec<(String, bool)> = groups.get_mut(info.group()).unwrap();
                existing_list.push((text, false));
                for ingredient in ingredients_lines {
                    existing_list.push((ingredient, true));
                }
            } else {
                let mut list = vec![(text, false)];
                for ingredient in ingredients_lines {
                    list.push((ingredient, true));
                }
                groups.insert(info.group(), list);
            }
        }

        for (group, list) in groups {
            let mut group_line = group.to_string() + ":";
            if list.len() == 1 {
                lines.push(group_line + " " + &*list.get(0).unwrap().0)
            } else {
                lines.push(group_line);
                for element in list {
                    let prefix = if element.1 { " - " } else { "- " }.to_string();
                    lines.push(prefix + &*element.0);
                }
            }
        }

        if include_price_info && total_value > 0.0 {
            lines.push(format!("Estimated Item Value: {}", format_number(total_value as u64)));
        }

        lines
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
    pub price: f64,
}

//TODO: maybe don't use new() a lot, normal creating is cool :>

#[derive(Debug, Clone)]
pub struct Donation {
    pub id: String,
    pub slot: String,
    pub borrowing: bool,
    pub items: Vec<Item>
}

#[derive(Debug, Clone)]
pub struct PlayerProfile {
    id: String,
    name: String,
    game_mode: String,
    selected: bool,
    data: serde_json::Value,
    garden: Option<serde_json::Value>,
    storage: Storage,
    museum: Option<Vec<Donation>>,
    purse: u64,
    bank: (u64, u64),
    first_join: Option<u64>,
    cookie_buff_active: bool,
    members: Vec<String>
}

#[derive(Debug, Clone)]
pub struct Storage {
    pub inventory: Vec<Item>,
    pub ender_chest: Vec<Item>,
    pub backpacks: Vec<Item>,
    pub armor: Vec<Item>,
    pub equipment: Vec<Item>,
    pub wardrobe: Vec<Item>,
    pub accessories: Vec<Item>,
    pub vault: Vec<Item>,
    pub sacks: HashMap<String, u64>,
    pub pets: Vec<Pet>
}

#[derive(Debug, Clone)]
pub struct Pet {
    pub name: String,
    pub tier: String,
    pub xp: f64,
    pub held_item: Option<String>,
    pub skin: Option<String>,
    pub active: bool,
}

impl Pet {
    pub fn new(name: String, tier: String, xp: f64, held_item: Option<String>, skin: Option<String>, active: bool) -> Self {
        Self { name, tier, xp, held_item, skin, active }
    }
}

impl Storage {
    pub fn empty() -> Self {
        Self { inventory: Vec::new(), ender_chest: Vec::new(), backpacks: Vec::new(), armor: Vec::new(), equipment: Vec::new(), wardrobe: Vec::new(), accessories: Vec::new(), vault: Vec::new(), sacks: HashMap::new(), pets: Vec::new() }
    }

    pub fn add_inventory(&mut self, inventory: Vec<Item>) { self.inventory.extend(inventory); }
    pub fn add_ender_chest(&mut self, ender_chest: Vec<Item>) { self.ender_chest.extend(ender_chest); }
    pub fn add_backpacks(&mut self, backpacks: Vec<Item>) { self.backpacks.extend(backpacks); }
    pub fn add_armor(&mut self, armor: Vec<Item>) { self.armor.extend(armor); }
    pub fn add_equipment(&mut self, equipment: Vec<Item>) { self.equipment.extend(equipment); }
    pub fn add_wardrobe(&mut self, wardrobe: Vec<Item>) { self.wardrobe.extend(wardrobe); }
    pub fn add_accessories(&mut self, accessories: Vec<Item>) { self.accessories.extend(accessories); }
    pub fn add_vault(&mut self, vault: Vec<Item>) { self.vault.extend(vault); }
    pub fn add_sacks(&mut self, sacks: HashMap<String, u64>) { self.sacks.extend(sacks); }
    pub fn add_pets(&mut self, pets: Vec<Pet>) { self.pets.extend(pets); }

    pub fn get_items_list(&self) -> Vec<&Item> {
        self.inventory.iter()
            .chain(self.ender_chest.iter())
            .chain(self.backpacks.iter())
            .chain(self.armor.iter())
            .chain(self.equipment.iter())
            .chain(self.wardrobe.iter())
            .chain(self.accessories.iter())
            .chain(self.vault.iter())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    id: String,
    item_id: String,
    name: String,
    count: u64,
    nbt: ItemNbt
}

impl Item {
    pub fn new(item_id: String, name: String, count: u64, nbt: ItemNbt) -> Self {
        Self { id: "".to_string(), item_id, name, count, nbt }
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn item_id(&self) -> &str { &self.item_id }
    pub fn name(&self) -> &str { &self.name }
    pub fn count(&self) -> &u64 { &self.count }
    pub fn nbt(&self) -> &ItemNbt { &self.nbt }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

impl PlayerProfile {
    pub fn new(id: String, name: String, game_mode: String, selected: bool, data: serde_json::Value, garden: Option<serde_json::Value>, storage: Storage, museum: Option<Vec<Donation>>, bank: (u64, u64), purse: u64, first_join: Option<u64>, cookie_buff_active: bool, members: Vec<String>) -> Self {
        Self { id, name, game_mode, selected, data, garden, storage, museum, bank, purse, first_join, cookie_buff_active, members }
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn name(&self) -> &str { &self.name }
    pub fn game_mode(&self) -> &str { &self.game_mode }
    pub fn is_selected(&self) -> bool { self.selected }
    pub fn data(&self) -> &serde_json::Value { &self.data }
    pub fn garden(&self) -> &Option<serde_json::Value> { &self.garden }
    pub fn storage(&self) -> &Storage { &self.storage }
    pub fn museum(&self) -> &Option<Vec<Donation>> { &self.museum }
    pub fn bank(&self) -> (u64, u64) { self.bank }
    pub fn purse(&self) -> u64 { self.purse }
    pub fn first_join(&self) -> &Option<u64> { &self.first_join }
    pub fn cookie_buff_active(&self) -> bool { self.cookie_buff_active }
    pub fn members(&self) -> &Vec<String> { &self.members }

    pub fn set_garden_data(&mut self, data: serde_json::Value) { self.garden = Some(data); }
    pub fn set_museum_data(&mut self, data: Vec<Donation>) { self.museum = Some(data); }
}