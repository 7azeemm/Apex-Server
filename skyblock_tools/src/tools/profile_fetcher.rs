use crate::constants::setups::SetupType;
use crate::item_utils::{decode_items, get_item_id, get_item_name, get_pet_level, get_pet_obj, get_pretty_name};
use crate::structs::item_structs::ItemNbt;
use crate::structs::player_data_structs::{Item, MuseumDonation, PlayerData, PlayerDataResponse, PlayerProfile, PlayerSetup, Storage, StringBuilder};
use crate::utils::get_hypixel_api_key;
use common::extensions::json_ext::JsonExt;
use common::http::send_http_request;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::iter::once;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval_at, Instant};
use tracing::{error, info};

const PROFILES_API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/profiles";
const GARDEN_API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/garden";
const MUSEUM_API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/museum";
const PROFILE_CLEAN_THRESHOLD: u64 = 180;

static PLAYER_PROFILES: LazyLock<RwLock<FxHashMap<String, PlayerData>>> = LazyLock::new(|| RwLock::new(FxHashMap::default()));

pub fn profile_cleaner() {
    tokio::spawn(async {
        let threshold_duration = Duration::from_secs(PROFILE_CLEAN_THRESHOLD);
        let mut interval = interval_at(Instant::now(), threshold_duration);
        loop {
            interval.tick().await;
            let mut player_profiles = PLAYER_PROFILES.write().await;
            let mut profiles_to_remove = Vec::new();

            for (player_uuid, player) in player_profiles.iter() {
                for (id, profile) in player.profiles() {
                    if profile.is_expired(threshold_duration) {
                        profiles_to_remove.push((player_uuid.clone(), id.clone()));
                    }
                }
            }

            let count = profiles_to_remove.len();
            for (player_id, profile_id) in profiles_to_remove {
                if let Some(player) = player_profiles.get_mut(&player_id) {
                    player.remove_profile(&profile_id);
                }
            }
            
            if count > 0 {
                info!("[Profile-Cleaner] Removed {} expired profiles", count);
            }
        }
    });
}

pub async fn get_player_profile(username: &str, player_uuid: &str, profile_name: Option<String>) -> Result<PlayerProfile, String> {
    let mut profile_name = profile_name;

    if let Some(player_data) = PLAYER_PROFILES.read().await.get(player_uuid).cloned() {
        if profile_name.is_none() {
            if let Some(selected_profile) = player_data.selected_profile() {
                profile_name = Some(selected_profile.to_owned());
            }
        }
        if let Some(ref profile_name) = profile_name {
            if let Some((profile_id, _)) = player_data.profiles_info().get(profile_name) {
                if let Some(profile) = player_data.profiles().get(profile_id) {
                    return Ok(profile.clone());
                }
            }
        }
    }

    let url = format!("{PROFILES_API_ENDPOINT}?key={}&uuid={player_uuid}", get_hypixel_api_key());
    let json = match send_http_request(&url).await {
        Ok(json) => json,
        Err(err) => {
            error!(?err, username, player_uuid, profile_name, "Failed to get skyblock data");
            return Err("Failed to get skyblock data".to_owned());
        }
    };
    if !json.get_bool("success").unwrap_or(false) {
        error!(username, player_uuid, profile_name, "Failed to get skyblock data");
        return Err("Failed to get skyblock data".to_owned());
    }

    let profiles = json.get_array("profiles").ok_or("Player does not have any profiles")?;
    let undashed_uuid = player_uuid.replace("-", "");
    let mut target_profile = None;
    let mut profiles_info = HashMap::new();
    let mut selected_profile = None;

    for profile in profiles {
        let Some(profile_id) = profile.get_str("profile_id") else { continue };
        let Some(members) = profile.get_object("members") else { continue };
        let Some(cute_name) = profile.get_str("cute_name") else { continue };
        let game_mode = profile.get_str("game_mode").unwrap_or("normal").to_owned();

        if members.contains_key(&undashed_uuid) {
            let is_selected = profile.get_bool("selected").unwrap_or(false);
            if is_selected {
                selected_profile = Some(cute_name.to_owned())
            }

            if target_profile.is_none() && profile_name.clone().map_or(is_selected, |n| n == cute_name) {
                target_profile = Some(profile);
            }

            profiles_info.insert(cute_name.to_owned(), (profile_id.to_owned(), game_mode));
        }
    }

    let mut player_profiles = PLAYER_PROFILES.write().await;
    let player_data = player_profiles
        .entry(player_uuid.to_owned())
        .or_insert(PlayerData::default());
    player_data.update(profiles_info, selected_profile);

    if let Some(profile) = target_profile {
        let parsed_profile = parse_profile(profile, player_uuid).await;
        if let Some(parsed_profile) = parsed_profile {
            return Ok(player_data.add_profile(parsed_profile));
        }
        error!(username, player_uuid, profile_name, "Couldn't parse skyblock profile");
        return Err("Failed to get player skyblock data".to_string());
    }

    Err("No Matching profile found".into())
}

pub async fn update_player_profile(player_uuid: &str, profile: PlayerProfile) {
    let mut players = PLAYER_PROFILES.write().await;
    if let Some(player) = players.get_mut(player_uuid) {
        player.update_profile(profile);
    }
}

pub async fn get_garden_data(pdr: &PlayerDataResponse) -> Option<Value> {
    let profile = pdr.profile();
    if profile.garden().is_some() {
        return profile.garden().clone();
    }

    let context = pdr.context();
    let url = &format!("{GARDEN_API_ENDPOINT}?key={}&profile={}", get_hypixel_api_key(), profile.id());
    match send_http_request(url).await {
        Ok(value) => match value.get_bool("success").unwrap_or(false) {
            true => {
                let data = value.get("garden").unwrap_or(&Value::default()).clone();
                profile.cache_garden_data(pdr.player_uuid(), data.clone()).await;
                return Some(data);
            },
            false => error!(?context, "Failed to get garden data"),
        },
        Err(err) => error!(?err, ?context, "Failed to get garden data"),
    };

    None
}

pub async fn get_museum_items(pdr: &PlayerDataResponse) -> Option<Vec<MuseumDonation>> {
    let profile = pdr.profile();
    if profile.museum().is_some() {
        return profile.museum().clone();
    }

    let context = pdr.context();
    let url = &format!("{MUSEUM_API_ENDPOINT}?key={}&profile={}", get_hypixel_api_key(), profile.id());
    match send_http_request(url).await {
        Err(err) => error!(?err, ?context, "Failed to get museum data"),
        Ok(value) => match value.get_bool("success").unwrap_or(false) {
            false => error!(?context, "Failed to get museum data"),
            true => {
                let player_uuid = pdr.player_uuid();
                let undashed_player_uuid = player_uuid.replace("-", "");
                if let Some(donations) = value.get_object(&format!("members/{undashed_player_uuid}/items")) {
                    let mut donations_list = Vec::new();
                    for (id, data) in donations {
                        let mut items = Vec::new();
                        if let Some(items_data) = data.get("items") {
                            items.extend(get_container_items(items_data, &format!("MUSEUM_{}", id)));
                        }

                        let slot = data.get_str("featured_slot").unwrap_or_default().to_owned();
                        let borrowing = data.get_bool("borrowing").unwrap_or(false);
                        let donation = MuseumDonation::new(id.to_owned(), slot, borrowing, items);
                        donations_list.push(donation);
                    }

                    profile.cache_museum_data(pdr.player_uuid(), donations_list.clone()).await;
                    return Some(donations_list)
                }
            }
        }
    };

    None
}

async fn parse_profile(profile: &Value, player_uuid: &str) -> Option<PlayerProfile> {
    let profile_id = profile.get_str("profile_id")?;
    let profile_name = profile.get_str("cute_name")?;
    let game_mode = profile.get_str("game_mode").unwrap_or("normal");
    let members = profile.get_object("members")?;
    let player_data = members.get(&player_uuid.replace("-", ""))?;
    let selected = profile.get_bool("selected").unwrap_or(false);

    let storage = scan_storage(player_data);
    let setups = scan_setups(&storage);
    let first_join = player_data.get_u64("profile/first_join");
    let cookie_buff_active = player_data
        .get_bool("profile/cookie_buff_active")
        .unwrap_or(false);
    let purse = player_data.get_f64("currencies/coin_purse").unwrap_or(0.0) as u64;
    let bank_balance = profile.get_f64("banking/balance");
    let personal_bank = player_data.get_f64("profile/bank_account");
    let bank = match bank_balance.is_none() && personal_bank.is_none() {
        false => Some((bank_balance.unwrap_or_default() + personal_bank.unwrap_or_default()) as u64),
        true => None,
    };

    let player_profile = PlayerProfile::new(
        profile_id.to_owned(),
        profile_name.to_owned(),
        game_mode.to_owned(),
        selected,
        player_data.clone(),
        storage,
        setups,
        bank,
        purse,
        first_join,
        cookie_buff_active,
    );

    Some(player_profile)
}

pub async fn get_profiles_info(player_uuid: &str, sb: &mut StringBuilder) {
    let player_profiles = PLAYER_PROFILES.read().await;
    if let Some(player) = player_profiles.get(player_uuid) {
        let selected_profile = player.selected_profile();
        sb.push("Profiles:".to_owned());
        for (name, (_, game_mode)) in player.profiles_info() {
            let mut line = format!("- {name} ({})", get_pretty_name(game_mode));
            if let Some(selected) = selected_profile && selected == name {
                line.push_str(" [Selected]");
            }
            sb.push(line);
        }
    }
}

fn scan_storage(data: &Value) -> Storage {
    let mut storage = Storage::default();
    let Some(inventory) = data.get_object("inventory") else {
        return storage;
    };

    let get_items = |key: &str, path: &str| -> Vec<Item> {
        inventory.get(key).map_or_else(Vec::new, |v| get_container_items(v, path))
    };

    storage.add_inventory(get_items("inv_contents", "INVENTORY"));
    storage.add_ender_chest(get_items("ender_chest_contents", "ENDER_CHEST"));

    if let Some(Value::Object(backpacks)) = &inventory.get("backpack_contents") {
        for (i, bp) in backpacks.iter() {
            storage.add_backpacks(get_container_items(bp, &format!("BACKPACK{i}")));
        }
    }

    if let Some(inv_armor) = inventory.get("inv_armor") {
        storage.add_armor(get_container_items(inv_armor, "ARMOR").into_iter().rev().collect());
    }

    storage.add_equipment(get_items("equipment_contents", "EQUIPMENT"));

    if let Some(accessories) = &inventory.get("bag_contents").and_then(|v| v.get("talisman_bag")) {
        storage.add_accessories(get_container_items(accessories, "ACCESSORY"));
    }

    storage.add_vault(get_items("personal_vault_contents", "VAULT"));

    if let Some(wardrobe_contents) = inventory.get("wardrobe_contents") {
        let wardrobe_items = get_all_container_items(wardrobe_contents, "WARDROBE");
        if !wardrobe_items.is_empty() {
            storage.add_wardrobe(organize_wardrobe_sets(wardrobe_items));
        }
    }

    if let Some(Value::Object(sacks_counts)) = inventory.get("sacks_counts") {
        let map: HashMap<String, u64> = sacks_counts
            .iter()
            .filter_map(|(k, v)| v.as_u64().map(|count| (k.clone(), count)))
            .collect();
        storage.add_sacks(map);
    }

    if let Some(pets) = data.get_array("pets_data/pets") {
        let mut pets_list = Vec::new();
        for pet in pets {
            if let Some(pet) = get_pet_obj(pet) {
                pets_list.push(pet);
            }
        }
        storage.add_pets(pets_list);
    }

    storage
}

fn scan_setups(storage: &Storage) -> HashMap<SetupType, PlayerSetup> {
    let mut setups = HashMap::new();
    let player_items: HashMap<String, String> = storage
        .get_items_list()
        .iter()
        .map(|i| (i.item_id().to_owned(), i.name().to_owned()))
        .collect();
    let pet_ids: Vec<String> = storage.pets().iter().map(|p| p.name().to_owned()).collect();

    let wardrobe_sets = storage.wardrobe().iter().map(|set| {
        [
            set[0].as_ref(),
            set[1].as_ref(),
            set[2].as_ref(),
            set[3].as_ref(),
        ]
    });

    let player_armor = storage.armor();
    let armor_set = [
        player_armor.first(),
        player_armor.get(1),
        player_armor.get(2),
        player_armor.get(3),
    ];

    let player_sets: Vec<[Option<&Item>; 4]> = wardrobe_sets.chain(once(armor_set)).collect();

    let setups_list = [
        SetupType::Mining,
        SetupType::Farming,
        SetupType::Foraging,
        SetupType::Fishing,
        SetupType::Archer,
        SetupType::Berserker,
        SetupType::Mage,
        SetupType::Tank,
        SetupType::Healer,
    ];

    for setup_type in setups_list {
        let mut player_setup = PlayerSetup::default();
        let setup = setup_type.get_setup();

        // Armor
        let mut armor = scan_gear(setup.armor, &player_items);
        let armor_ids: HashSet<String> = armor
            .iter()
            .filter_map(|item| item.as_ref())
            .map(|(item_id, _)| item_id.clone())
            .collect();
        let mut frozen_blaze_pieces = 0;

        if !armor_ids.is_empty() {
            let mut score = 0;
            let mut wardrobe_set = None;

            for set in player_sets.iter() {
                let set_score = set
                    .iter()
                    .filter(|piece| {
                        piece.as_ref().map_or(false, |p| armor_ids.contains(p.item_id()))
                    })
                    .count();

                if set_score > score {
                    score = set_score;
                    wardrobe_set = Some(set);
                }
            }

            if let Some(set) = wardrobe_set {
                let mut simplified_set: [Option<(String, String)>; 4] = [None, None, None, None];
                for (i, piece) in set.iter().enumerate() {
                    simplified_set[i] = piece.map(|item| (item.item_id().to_owned(), item.name().to_owned()));
                }
                armor = simplified_set;
            }

            for piece in armor.iter() {
                if let Some((_, id)) = piece {
                    if id.contains("FROZEN_BLAZE") {
                        frozen_blaze_pieces += 1;
                    }
                }
            }
        }

        let mut armor_set = Vec::new();
        for piece in armor {
            let piece_name = piece.map(|(_, name)| name).unwrap_or("N/A".to_owned());
            armor_set.push(piece_name);
        }
        player_setup.add_armor(armor_set);

        let equipment = scan_gear(setup.equipment, &player_items);
        let mut equipment_set = Vec::new();
        for piece in equipment {
            let piece_name = piece.map(|(_, name)| name).unwrap_or("N/A".to_owned());
            equipment_set.push(piece_name);
        }
        player_setup.add_equipment(equipment_set);

        for tool_group in setup.tools {
            let item_name = scan_item(tool_group, &player_items);
            player_setup.add_tool(item_name.unwrap_or("N/A".to_owned()));
        }

        let mut pet = None;
        if frozen_blaze_pieces >= 2 {
            const BLAZE_PET_ID: &str = "BLAZE";
            if setup.pets.contains(&BLAZE_PET_ID) {
                if pet_ids.contains(&BLAZE_PET_ID.to_owned()) {
                    pet = Some(BLAZE_PET_ID)
                }
            }
        }

        if pet.is_none() {
            for pet_id in setup.pets.iter() {
                if pet_ids.contains(&pet_id.to_string()) {
                    pet = Some(pet_id);
                    break;
                }
            }
        }

        let mut pet_info = None;

        if let Some(pet) = pet {
            if let Some(pet) = storage.pets().iter().find(|p| p.name() == pet) {
                let (level, _) = get_pet_level(pet.name(), pet.tier(), *pet.xp() as u64);
                pet_info = Some(format!(
                    "[Lvl {}] {} {}",
                    level,
                    get_pretty_name(pet.tier()),
                    get_pretty_name(pet.name())
                ));
            }
        }

        player_setup.add_pet(pet_info.unwrap_or("N/A".to_owned()));
        setups.insert(setup_type, player_setup);
    }

    setups
}

fn scan_item(item_ids: &[&str], player_items: &HashMap<String, String>) -> Option<String> {
    for item_id in item_ids {
        if let Some(item_name) = player_items.get(item_id.to_owned()) {
            return Some(item_name.to_owned());
        }
    }

    None
}

fn scan_gear(sets: &[&[&str]], player_items: &HashMap<String, String>) -> [Option<(String, String)>; 4] {
    let mut found: [Option<(String, String)>; 4] = [None, None, None, None];

    for set in sets {
        for (slot_index, item_id) in set.iter().enumerate() {
            if !item_id.is_empty() && found[slot_index].is_none() {
                if let Some(item_name) = player_items.get(item_id.to_owned()) {
                    found[slot_index] = Some((item_id.to_string(), item_name.to_owned()));
                }
            }
        }
    }

    found
}

fn organize_wardrobe_sets(wardrobe_items: Vec<Option<Item>>) -> Vec<[Option<Item>; 4]> {
    let mut pages = Vec::new();

    // Split into two pages of 36 items each
    for page_start in (0..wardrobe_items.len()).step_by(36) {
        // Each page has 9 sets
        for set_index in 0..9 {
            let mut armor_set: [Option<Item>; 4] = [None, None, None, None];
            let mut is_empty = true;

            // Fill slots
            for slot in 0..4 {
                let item_index = page_start + (slot * 9) + set_index;
                if item_index < wardrobe_items.len() {
                    let item = wardrobe_items[item_index].clone();
                    if item.is_some() {
                        is_empty = false;
                    }
                    armor_set[slot] = item;
                }
            }

            if !is_empty {
                pages.push(armor_set);
            }
        }
    }

    pages
}

fn get_container_items(container: &Value, path: &str) -> Vec<Item> {
    let mut items = Vec::new();
    if let Some(contents) = container.get_str("data") {
        if let Ok(items_data) = decode_items(contents, false) {
            for (slot, item) in items_data.iter().enumerate() {
                if let Some(item) = item {
                    if let Some(item) = get_item_obj(item.clone(), path, slot) {
                        items.push(item);
                    }
                }
            }
        }
    }

    items
}

fn get_all_container_items(container: &Value, path: &str) -> Vec<Option<Item>> {
    let mut items = Vec::new();
    if let Some(contents) = container.get_str("data") {
        if let Ok(items_data) = decode_items(contents, true) {
            for (slot, item) in items_data.iter().enumerate() {
                match item {
                    None => items.push(None),
                    Some(item) => {
                        if let Some(item) = get_item_obj(item.clone(), path, slot) {
                            items.push(Some(item));
                        }
                    }
                }
            }
        }
    }

    items
}

fn get_item_obj(item: ItemNbt, path: &str, slot: usize) -> Option<Item> {
    let item_id = get_item_id(&item)?;
    let item_name = get_item_name(&item)?;
    let custom_id = format!("{item_id}-{path}_{slot}");

    Some(Item::new(custom_id, item_id, item_name, item.count(), item))
}