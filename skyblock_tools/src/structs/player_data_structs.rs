use crate::constants::setups::SetupType;
use crate::structs::item_structs::ItemNbt;
use crate::tools::profile_fetcher::get_player_profile;
use common::player_fetcher::get_player_uuid;
use derive_new::new;
use getset::Getters;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use tokio::time::Instant;

pub struct PlayerDataResponse {
    username: String,
    profile_name: Option<String>,
    player_uuid: String,
    profile: PlayerProfile,
    sb: Option<StringBuilder>,
}

impl PlayerDataResponse {
    pub async fn new(username: String, profile_name: Option<String>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let player_uuid = get_player_uuid(&username).await.ok_or("Couldn't get player_uuid")?;
        let profile = get_player_profile(&username, &player_uuid, profile_name.clone()).await?;
        Ok(Self { username, profile_name, player_uuid, profile, sb: None })
    }
    pub fn username(&self) -> &str {
        &self.username
    }
    pub fn profile_name(&self) -> &Option<String> {
        &self.profile_name
    }
    pub fn player_uuid(&self) -> &str {
        &self.player_uuid
    }
    pub fn profile(&self) -> &PlayerProfile {
        &self.profile
    }
    pub fn profile_mut(&mut self) -> &mut PlayerProfile {
        &mut self.profile
    }
    pub fn profile_data(&self) -> &Value {
        &self.profile.data
    }
    pub fn get_sb(&self) -> &Option<StringBuilder> {
        &self.sb
    }

    pub fn set_sb(&mut self, sb: StringBuilder) {
        self.sb = Some(sb)
    }
}

pub struct StringBuilder {
    lines: Vec<String>,
}

impl StringBuilder {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }
    pub fn lines(&self) -> &Vec<String> {
        &self.lines
    }
    pub fn push(&mut self, line: String) {
        self.lines.push(line)
    }
    pub fn pushln(&mut self) {
        self.lines.push("".to_owned())
    }
    pub fn push_option(&mut self, line: Option<String>) {
        if let Some(line) = line {
            self.lines.push(line)
        }
    }

    pub fn get_response(&self) -> String {
        self.lines.join("\n")
    }
}

#[derive(Default, Clone)]
pub struct PlayerData {
    profiles: HashMap<String, PlayerProfile>,
    profiles_info: HashMap<String, (String, String)>,
    selected_profile: Option<String>,
}

impl PlayerData {
    pub fn profiles(&self) -> &HashMap<String, PlayerProfile> {
        &self.profiles
    }
    pub fn profiles_info(&self) -> &HashMap<String, (String, String)> {
        &self.profiles_info
    }
    pub fn selected_profile(&self) -> &Option<String> {
        &self.selected_profile
    }

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
    museum: Option<Vec<MuseumDonation>>,
    purse: u64,
    bank: Option<u64>,
    first_join: Option<u64>,
    cookie_buff_active: bool,
    fetch_time: Instant,
}

impl PlayerProfile {
    pub fn new(
        id: String,
        name: String,
        game_mode: String,
        selected: bool,
        data: Value,
        storage: Storage,
        setups: HashMap<SetupType, PlayerSetup>,
        bank: Option<u64>,
        purse: u64,
        first_join: Option<u64>,
        cookie_buff_active: bool,
    ) -> Self {
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
            fetch_time: Instant::now(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn game_mode(&self) -> &str {
        &self.game_mode
    }
    pub fn is_selected(&self) -> bool {
        self.selected
    }
    pub fn data(&self) -> &Value {
        &self.data
    }
    pub fn garden(&self) -> &Option<Value> {
        &self.garden
    }
    pub fn storage(&self) -> &Storage {
        &self.storage
    }
    pub fn museum(&self) -> &Option<Vec<MuseumDonation>> {
        &self.museum
    }
    pub fn bank(&self) -> Option<u64> {
        self.bank
    }
    pub fn purse(&self) -> u64 {
        self.purse
    }
    pub fn first_join(&self) -> &Option<u64> {
        &self.first_join
    }
    pub fn cookie_buff_active(&self) -> bool {
        self.cookie_buff_active
    }
    pub fn is_expired(&self, threshold: Duration) -> bool {
        Instant::now().duration_since(self.fetch_time) > threshold
    }

    pub fn add_setup_info(&self, setup_type: SetupType, sb: &mut StringBuilder) {
        match self.setups.get(&setup_type) {
            None => sb.push("Gear: unavailable".to_owned()),
            Some(setup) => setup.add_info(setup_type, sb),
        }
    }

    pub fn set_garden_data(&mut self, data: Value) {
        self.garden = Some(data);
    }
    pub fn set_museum_data(&mut self, data: Vec<MuseumDonation>) {
        self.museum = Some(data);
    }
}

#[derive(Clone, new, Getters)]
#[getset(get = "pub")]
pub struct MuseumDonation {
    id: String,
    slot: String,
    borrowing: bool,
    items: Vec<Item>,
}

#[derive(Default, Clone, Getters)]
#[getset(get = "pub")]
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
    pub fn add_inventory(&mut self, inventory: Vec<Item>) {
        self.inventory.extend(inventory);
    }
    pub fn add_ender_chest(&mut self, ender_chest: Vec<Item>) {
        self.ender_chest.extend(ender_chest);
    }
    pub fn add_backpacks(&mut self, backpacks: Vec<Item>) {
        self.backpacks.extend(backpacks);
    }
    pub fn add_armor(&mut self, armor: Vec<Item>) {
        self.armor.extend(armor);
    }
    pub fn add_equipment(&mut self, equipment: Vec<Item>) {
        self.equipment.extend(equipment);
    }
    pub fn add_wardrobe(&mut self, wardrobe: Vec<[Option<Item>; 4]>) {
        self.wardrobe.extend(wardrobe);
    }
    pub fn add_accessories(&mut self, accessories: Vec<Item>) {
        self.accessories.extend(accessories);
    }
    pub fn add_vault(&mut self, vault: Vec<Item>) {
        self.vault.extend(vault);
    }
    pub fn add_sacks(&mut self, sacks: HashMap<String, u64>) {
        self.sacks.extend(sacks);
    }
    pub fn add_pets(&mut self, pets: Vec<Pet>) {
        self.pets.extend(pets);
    }

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
        self.inventory
            .iter()
            .chain(self.ender_chest.iter())
            .chain(self.backpacks.iter())
            .chain(self.armor.iter())
            .chain(self.equipment.iter())
            .chain(self.vault.iter())
            .chain(self.get_wardrobe_items())
            .collect()
    }
}

#[derive(Clone, new, Getters)]
#[getset(get = "pub")]
pub struct Pet {
    name: String,
    tier: String,
    xp: f64,
    held_item: Option<String>,
    skin: Option<String>,
    active: bool,
}

#[derive(Clone, new, Getters)]
#[getset(get = "pub")]
pub struct Item {
    id: String,
    item_id: String,
    name: String,
    count: u64,
    nbt: ItemNbt,
}

#[derive(Default, Clone, Getters)]
#[getset(get = "pub")]
pub struct PlayerSetup {
    armor: Vec<String>,
    equipment: Vec<String>,
    tools: Vec<String>,
    pet: String,
}

impl PlayerSetup {
    pub fn add_armor(&mut self, armor: Vec<String>) {
        self.armor.extend(armor);
    }
    pub fn add_equipment(&mut self, equipment: Vec<String>) {
        self.equipment.extend(equipment);
    }
    pub fn add_tool(&mut self, tool: String) {
        self.tools.push(tool);
    }
    pub fn add_pet(&mut self, pet: String) {
        self.pet.push_str(&pet);
    }

    pub fn add_info(&self, setup_type: SetupType, sb: &mut StringBuilder) {
        let armor = &self.armor;
        match armor.iter().any(|p| p != "N/A") {
            false => sb.push("Armor: N/A".to_owned()),
            true => {
                sb.push("Armor:".to_owned());
                for piece in armor {
                    sb.push(format!(" - {piece}"));
                }
            }
        }

        let equipment = &self.equipment;
        match equipment.iter().any(|p| p != "N/A") {
            false => sb.push("Equipment: N/A".to_owned()),
            true => {
                sb.push("Equipment:".to_owned());
                for piece in equipment {
                    sb.push(format!(" - {piece}"));
                }
            }
        }

        let tools = &self.tools;
        if setup_type != SetupType::Fishing {
            if setup_type != SetupType::Farming {
                let tool_name = match setup_type {
                    SetupType::Mining => "Mining Tool",
                    SetupType::Foraging => "Axe",
                    _ => "Weapon", // Dungeon classes
                };
                let tool = tools.first().map(|s| s.as_str()).unwrap_or("N/A");
                sb.push(format!("{tool_name}: {tool}"))
            }
        } else {
            match tools.len() == 1 {
                true => sb.push(format!("Rod: {}", tools.first().unwrap())),
                false => {
                    sb.push(format!("Water Rod: {}", tools.first().unwrap()));
                    sb.push(format!("Lava Rod: {}", tools.get(1).unwrap()));
                }
            }
        }

        sb.push(format!("Pet: {}", &self.pet))
    }
}
