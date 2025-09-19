use crate::constants::setups::SetupType;
use crate::player_data::profile_fetcher::get_player_profile;
use crate::structs::item_structs::ItemNbt;
use crate::utils::get_player_uuid;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::Instant;

pub struct PlayerDataResponse {
    username: String,
    profile_name: Option<String>,
    player_uuid: String,
    profile: PlayerProfile,
    resp: Option<String>,
}

impl PlayerDataResponse {
    pub async fn new(username: String, profile_name: Option<String>) -> Option<Self> {
        let player_uuid = get_player_uuid(&username).await.ok()?;
        let profile = get_player_profile(&username, &player_uuid, profile_name.clone()).await.ok()?;
        Some(Self { username, profile_name, player_uuid, profile, resp: None })
    }
    pub fn username(&self) -> &str { &self.username }
    pub fn profile_name(&self) -> &Option<String> { &self.profile_name }
    pub fn player_uuid(&self) -> &str { &self.player_uuid }
    pub fn profile(&self) -> &PlayerProfile { &self.profile }
    pub fn profile_mut(&mut self) -> &mut PlayerProfile { &mut self.profile }
    pub fn profile_data(&self) -> &Value { &self.profile.data }
    pub fn get_resp(&self) -> Option<String> { self.resp.clone() }

    pub fn set_resp(&mut self, resp: StringBuilder) { self.resp = Some(resp.lines.join("\n")) }
}

pub struct StringBuilder {
    lines: Vec<String>,
}

impl StringBuilder {
    pub fn new() -> Self { Self { lines: Vec::new() } }
    pub fn lines(&self) -> &Vec<String> { &self.lines }
    pub fn push(&mut self, line: String) { self.lines.push(line) }
    pub fn pushln(&mut self) { self.lines.push("".to_owned()) }
    pub fn push_option(&mut self, line: Option<String>) {
        if let Some(line) = line {
            self.lines.push(line)
        }
    }
}

#[derive(Clone)]
pub struct PlayerData {
    profiles: HashMap<String, PlayerProfile>,
    profiles_info: HashMap<String, (String, String)>,
    selected_profile: Option<String>,
}

impl PlayerData {
    pub fn new() -> Self {
        Self { profiles: HashMap::new(), profiles_info: HashMap::new(), selected_profile: None }
    }

    pub fn profiles(&self) -> &HashMap<String, PlayerProfile> { &self.profiles }
    pub fn profiles_info(&self) -> &HashMap<String, (String, String)> { &self.profiles_info }
    pub fn selected_profile(&self) -> &Option<String> { &self.selected_profile }

    pub fn update(&mut self, profiles_info: HashMap<String, (String, String)>, selected: Option<String>) {
        self.profiles_info = profiles_info;
        self.selected_profile = selected;
    }

    pub fn add_profile(&mut self, profile: PlayerProfile) -> PlayerProfile {
        let profile_id = profile.id().to_owned();
        self.profiles.insert(profile_id.clone(), profile);
        self.profiles.get(&profile_id).unwrap().clone()
    }

    pub fn remove_profile(&mut self, profile_id: &str) {
        self.profiles.remove(profile_id);
    }
}

#[derive(Clone)]
pub struct PlayerProfile {
    id: String,
    name: String,
    game_mode: String,
    selected: bool,
    data: Value,
    garden: Option<Value>,
    storage: Storage,
    setups: HashMap<SetupType, PlayerSetup>,
    museum: Option<Vec<Donation>>,
    purse: u64,
    bank: u64,
    first_join: Option<u64>,
    cookie_buff_active: bool,
    members: Vec<String>,
    fetch_time: Instant,
}

impl PlayerProfile {
    pub fn new(id: String, name: String, game_mode: String, selected: bool, data: Value, storage: Storage,
               setups: HashMap<SetupType, PlayerSetup>, bank: u64, purse: u64, first_join: Option<u64>,
               cookie_buff_active: bool, members: Vec<String>) -> Self {
        Self {
            id,
            name,
            game_mode,
            selected,
            data,
            garden: None,
            storage,
            setups,
            museum: None,
            bank,
            purse,
            first_join,
            cookie_buff_active,
            members,
            fetch_time: Instant::now(),
        }
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn name(&self) -> &str { &self.name }
    pub fn game_mode(&self) -> &str { &self.game_mode }
    pub fn is_selected(&self) -> bool { self.selected }
    pub fn data(&self) -> &Value { &self.data }
    pub fn garden(&self) -> &Option<Value> { &self.garden }
    pub fn storage(&self) -> &Storage { &self.storage }
    pub fn setups(&self) -> &HashMap<SetupType, PlayerSetup> { &self.setups }
    pub fn museum(&self) -> &Option<Vec<Donation>> { &self.museum }
    pub fn bank(&self) -> u64 { self.bank }
    pub fn purse(&self) -> u64 { self.purse }
    pub fn first_join(&self) -> &Option<u64> { &self.first_join }
    pub fn cookie_buff_active(&self) -> bool { self.cookie_buff_active }
    pub fn members(&self) -> &Vec<String> { &self.members }
    pub fn is_expired(&self, threshold: Duration) -> bool {
        Instant::now().duration_since(self.fetch_time) > threshold
    }

    pub fn set_garden_data(&mut self, data: Value) { self.garden = Some(data); }
    pub fn set_museum_data(&mut self, data: Vec<Donation>) { self.museum = Some(data); }
}

#[derive(Clone)]
pub struct Donation {
    pub id: String,
    pub slot: String,
    pub borrowing: bool,
    pub items: Vec<Item>,
}

#[derive(Clone)]
pub struct Storage {
    inventory: Vec<Item>,
    ender_chest: Vec<Item>,
    backpacks: Vec<Item>,
    armor: Vec<Item>,
    equipment: Vec<Item>,
    wardrobe: Vec<[Option<Item>; 4]>,
    accessories: Vec<Item>,
    vault: Vec<Item>,
    sacks: HashMap<String, u64>,
    pets: Vec<Pet>,
}

impl Storage {
    pub fn empty() -> Self {
        Self { inventory: Vec::new(), ender_chest: Vec::new(), backpacks: Vec::new(), armor: Vec::new(), equipment: Vec::new(), wardrobe: Vec::new(), accessories: Vec::new(), vault: Vec::new(), sacks: HashMap::new(), pets: Vec::new() }
    }

    pub fn inventory(&self) -> &Vec<Item> { &self.inventory }
    pub fn ender_chest(&self) -> &Vec<Item> { &self.ender_chest }
    pub fn backpacks(&self) -> &Vec<Item> { &self.backpacks }
    pub fn armor(&self) -> &Vec<Item> { &self.armor }
    pub fn equipment(&self) -> &Vec<Item> { &self.equipment }
    pub fn wardrobe(&self) -> &Vec<[Option<Item>; 4]> { &self.wardrobe }
    pub fn accessories(&self) -> &Vec<Item> { &self.accessories }
    pub fn vault(&self) -> &Vec<Item> { &self.vault }
    pub fn sacks(&self) -> &HashMap<String, u64> { &self.sacks }
    pub fn pets(&self) -> &Vec<Pet> { &self.pets }

    pub fn add_inventory(&mut self, inventory: Vec<Item>) { self.inventory.extend(inventory); }
    pub fn add_ender_chest(&mut self, ender_chest: Vec<Item>) { self.ender_chest.extend(ender_chest); }
    pub fn add_backpacks(&mut self, backpacks: Vec<Item>) { self.backpacks.extend(backpacks); }
    pub fn add_armor(&mut self, armor: Vec<Item>) { self.armor.extend(armor); }
    pub fn add_equipment(&mut self, equipment: Vec<Item>) { self.equipment.extend(equipment); }
    pub fn add_wardrobe(&mut self, wardrobe: Vec<[Option<Item>; 4]>) { self.wardrobe.extend(wardrobe); }
    pub fn add_accessories(&mut self, accessories: Vec<Item>) { self.accessories.extend(accessories); }
    pub fn add_vault(&mut self, vault: Vec<Item>) { self.vault.extend(vault); }
    pub fn add_sacks(&mut self, sacks: HashMap<String, u64>) { self.sacks.extend(sacks); }
    pub fn add_pets(&mut self, pets: Vec<Pet>) { self.pets.extend(pets); }

    pub fn get_wardrobe_items(&self) -> Vec<&Item> {
        let mut items = Vec::new();
        for set in self.wardrobe.iter() {
            for piece in set.iter() {
                if let Some(piece) = piece {
                    items.push(piece);
                }
            }
        }
        items
    }

    pub fn get_items_list(&self) -> Vec<&Item> {
        self.inventory.iter()
            .chain(self.ender_chest.iter())
            .chain(self.backpacks.iter())
            .chain(self.armor.iter())
            .chain(self.equipment.iter())
            .chain(self.vault.iter())
            .chain(self.get_wardrobe_items())
            .collect()
    }
}

#[derive(Clone)]
pub struct Pet {
    name: String,
    tier: String,
    xp: f64,
    held_item: Option<String>,
    skin: Option<String>,
    active: bool,
}

impl Pet {
    pub fn new(name: String, tier: String, xp: f64, held_item: Option<String>, skin: Option<String>, active: bool) -> Self {
        Self { name, tier, xp, held_item, skin, active }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn tier(&self) -> &str { &self.tier }
    pub fn xp(&self) -> f64 { self.xp }
    pub fn held_item(&self) -> &Option<String> { &self.held_item }
    pub fn skin(&self) -> &Option<String> { &self.skin }
    pub fn active(&self) -> bool { self.active }
}

#[derive(Clone)]
pub struct Item {
    custom_id: String,
    item_id: String,
    name: String,
    count: u64,
    nbt: ItemNbt,
}

impl Item {
    pub fn new(custom_id: String, item_id: String, name: String, count: u64, nbt: ItemNbt) -> Self {
        Self { custom_id, item_id, name, count, nbt }
    }

    pub fn id(&self) -> &str { &self.custom_id }
    pub fn item_id(&self) -> &str { &self.item_id }
    pub fn name(&self) -> &str { &self.name }
    pub fn count(&self) -> &u64 { &self.count }
    pub fn nbt(&self) -> &ItemNbt { &self.nbt }
}

#[derive(Clone)]
pub struct PlayerSetup {
    armor: Vec<String>,
    equipment: Vec<String>,
    tools: Vec<String>,
    pet: String,
}

impl PlayerSetup {
    pub fn new() -> PlayerSetup {
        PlayerSetup { armor: Vec::new(), equipment: Vec::new(), tools: Vec::new(), pet: String::new() }
    }

    pub fn armor(&self) -> &Vec<String> { &self.armor }
    pub fn equipment(&self) -> &Vec<String> { &self.equipment }
    pub fn tools(&self) -> &Vec<String> { &self.tools }
    pub fn pet(&self) -> &String { &self.pet }

    pub fn add_armor(&mut self, armor: Vec<String>) { &self.armor.extend(armor); }
    pub fn add_equipment(&mut self, equipment: Vec<String>) { &self.equipment.extend(equipment); }
    pub fn add_tool(&mut self, tool: String) { &self.tools.push(tool); }
    pub fn add_pet(&mut self, pet: String) { &self.pet.push_str(&pet); }
}

//TODO: remove all pubs