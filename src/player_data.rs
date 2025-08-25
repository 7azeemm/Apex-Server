use std::cmp::min;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::format;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use fastnbt::nbt;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use rustc_hash::FxHashMap;
use sea_orm::Iden;
use sea_orm::sea_query::ExprTrait;
use serde_json::{Map, Value};
use strsim::jaro_winkler;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tokio::time::sleep;
use crate::auctions::{decode_base64, get_lowest_bin};
use crate::bazaar::get_item_price;
use crate::constants::garden::{CANE_CACTUS_MILESTONE_XP, CARROT_POTATO_MILESTONE_XP, COCOA_WART_MILESTONE_XP, CROP_NAMES, GARDEN_LEVELS_XP, MAX_COMPOSTER_UPGRADE_LEVEL, MAX_CROP_MILESTONE, MAX_CROP_UPGRADE_LEVEL, MAX_GARDEN_LEVEL, MAX_PLOTS, MELON_MILESTONE_XP, WHEAT_PUMPKIN_MUSHROOM_MILESTONE_XP};
use crate::constants::misc::{BESTIARY_MAX_LEVEL, MAX_ENIGMA_SOULS, MAX_FAIRY_SOULS, MAX_TIMECHARMS};
use crate::constants::pets::{PET_LEVELS_XP, RARITY_OFFSETS};
use crate::constants::reforges::{EXECLUDE_REFORGES, REFORGE_STONES};
use crate::constants::skills::{DUNGEONEERING_SKILL_XP, RUNECRAFTING_SKILL_XP, SKILLS_XP, SKILL_MAX_LEVELS, SOCIAL_SKILL_XP};
use crate::endpoints::get_price;
use crate::item_utils::{decode_inventory_base64, format_number, get_readable_name};
use crate::item_value_calculator::calculate_item_value;
use crate::live_data::{get_mayor_info, get_skyblock_year, get_upcoming_contests};
use crate::statics::HTTP_CLIENT;
use crate::structs::{Donation, Item, ItemNbt, ItemValue, Pet, PlayerProfile, Storage};

const PROFILES_API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/profiles";
const GARDEN_API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/garden";
const MUSEUM_API_ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/museum";
const API_KEY: &str = "0024033b-1366-4de9-b094-52c5db7a6500";
const CLEAN_THRESHOLD: u64 = 300;

pub static PLAYER_PROFILES: LazyLock<RwLock<FxHashMap<String, (Vec<PlayerProfile>, Instant)>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));

pub async fn fetch_profiles(player_uuid: &str) -> Result<(), Box<dyn Error>> {
    // let resp = HTTP_CLIENT
    //     .get(PROFILES_API_ENDPOINT)
    //     .query(&[("key", API_KEY), ("uuid", player_uuid)])
    //     .send()
    //     .await?
    //     .text()
    //     .await?;
    //
    // let value: Value = serde_json::from_str(&resp)?;
    // let success = value.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    // if !success {
    //     return Err(format!("Couldn't fetch profiles of player {player_uuid}").into());
    // }
    //
    // let profiles = value.get("profiles").and_then(|v| v.as_array());
    // let profiles = match profiles {
    //     None => return Err("Player does not have any profiles".into()),
    //     Some(v) => v
    // };
    //
    // if profiles.is_empty() {
    //     return Err("Player does not have any profiles".into());
    // }
    //
    // let mut profiles_list: Vec<PlayerProfile> = vec![];
    //
    // for profile in profiles {
    //     let profile_id = profile.get("profile_id").and_then(|v| v.as_str()).unwrap_or("");
    //     if profile_id.is_empty() { continue };
    //     let profile_name = profile.get("cute_name").and_then(|v| v.as_str()).unwrap_or("");
    //     let selected = profile.get("selected").and_then(|v| v.as_bool()).unwrap_or(false);
    //     let game_mode = profile.get("game_mode").and_then(|v| v.as_str()).unwrap_or("");
    //
    //     let members = profile.get("members").and_then(|v| v.as_object());
    //     let members = match members {
    //         None => { continue },
    //         Some(v) => v
    //     };
    //
    //     let undashed_uuid = player_uuid.replace("-", "");
    //     let player_data = match members.get(&undashed_uuid) {
    //         None => { continue }
    //         Some(v) => v
    //     };
    //
    //     let player_profile = PlayerProfile::new(
    //         profile_id.to_string(),
    //         profile_name.to_string(),
    //         game_mode.to_string(),
    //         selected,
    //         player_data.clone()
    //     );
    //
    //     if selected {
    //         save_json_to_file(&serde_json::to_string_pretty(&player_data)?, "test.json").await?;
    //     }
    //
    //     profiles_list.push(player_profile);
    // }

    let mut profiles_list: Vec<PlayerProfile> = vec![];

    let data = load_json_from_file("test.json").await?;
    let storage = scan_storage(&data);
    let first_join = data.get("profile").and_then(|v| v.get("first_join")).and_then(|v| v.as_u64());
    let cookie_buff_active = data.get("profile").and_then(|v| v.get("cookie_buff_active")).and_then(|v| v.as_bool()).unwrap_or(false);

    let player_profile = PlayerProfile::new(
        "1d94c517-57eb-4765-a464-7436a61367b6".to_string(),
        String::default(),
        String::default(),
        true,
        data,
        None,
        storage,
        None,
        (110_000_000, 0),
        0,
        first_join,
        cookie_buff_active,
        Vec::new()
    );
    profiles_list.push(player_profile);

    let mut write = PLAYER_PROFILES.write().await;
    write.insert(player_uuid.to_string(), (profiles_list, Instant::now()));

    Ok(())
}

pub async fn get_garden_data(profile: &mut PlayerProfile) -> &Option<Value> {
    if profile.garden().is_some() {
        return profile.garden();
    }

    match fetch_garden_data(profile.id()).await {
        Ok(data) => { profile.set_garden_data(data); },
        Err(err) => { eprintln!("{}", err); }
    }

    profile.garden()
}

pub async fn fetch_garden_data(profile_uuid: &str) -> Result<Value, Box<dyn Error>> {
    let resp = HTTP_CLIENT
        .get(GARDEN_API_ENDPOINT)
        .query(&[("key", API_KEY), ("profile", profile_uuid)])
        .send()
        .await?
        .text()
        .await?;

    let value: Value = serde_json::from_str(&resp)?;
    let success = value.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    if !success {
        return Err(format!("Couldn't fetch garden data of profile {}", profile_uuid).into());
    }

    Ok(value)
}

pub async fn get_museum_items<'a>(player_uuid: &str, profile: &'a mut PlayerProfile) -> &'a Option<Vec<Donation>> {
    if profile.museum().is_some() {
        return profile.museum()
    }

    match fetch_museum_data(profile.id()).await {
        Ok(data) => {
            let undashed_player_uuid = player_uuid.replace("-", "");
            let donations = data.get("members").and_then(|v| v.get(undashed_player_uuid)).and_then(|v| v.get("items"));
            if let Some(donations) = donations.and_then(|v| v.as_object()) {
                let mut donations_list = Vec::new();
                for (id, data) in donations {
                    let mut items = Vec::new();
                    if let Some(items_data) = data.get("items").and_then(|v| v.get("data")).and_then(|v| v.as_str()) {
                        if let Ok(decoded_items) = decode_inventory_base64(items_data) {
                            for (item, _) in decoded_items {
                                if let Some(item) = get_item_obj(item) {
                                    items.push(item);
                                }
                            }
                        }
                    }

                    let slot = data.get("featured_slot").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let borrowing = data.get("borrowing").and_then(|v| v.as_bool()).unwrap_or(false);
                    let donation = Donation {
                        id: id.to_string(),
                        slot,
                        borrowing,
                        items
                    };
                    donations_list.push(donation);
                }
                profile.set_museum_data(donations_list);
            }
        },
        Err(err) => { eprintln!("{}", err); }
    }

    profile.museum()
}

pub async fn fetch_museum_data(profile_uuid: &str) -> Result<Value, Box<dyn Error>> {
    let resp = HTTP_CLIENT
        .get(MUSEUM_API_ENDPOINT)
        .query(&[("key", API_KEY), ("profile", profile_uuid)])
        .send()
        .await?
        .text()
        .await?;

    let value: Value = serde_json::from_str(&resp)?;
    let success = value.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    if !success {
        return Err(format!("Couldn't fetch museum data of profile {}", profile_uuid).into());
    }

    Ok(value)
}

pub async fn get_selected_profile(player_uuid: &str) -> Option<PlayerProfile> {
    async fn find_selected(player_uuid: &str) -> Option<PlayerProfile> {
        let read = PLAYER_PROFILES.read().await;
        read.get(player_uuid)
            .and_then(|(profiles, _)| profiles.iter().find(|p| p.is_selected()).cloned())
    }

    if let Some(profile) = find_selected(player_uuid).await {
        return Some(profile);
    }

    if fetch_profiles(player_uuid).await.is_ok() {
        return find_selected(player_uuid).await;
    }

    None
}

pub fn spawn_profile_cleanup() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let mut write = PLAYER_PROFILES.write().await;
            let now = Instant::now();
            write.retain(|_, (_, timestamp)| now.duration_since(*timestamp) < Duration::from_secs(CLEAN_THRESHOLD));
        }
    });
}

pub async fn save_json_to_file(json_str: &str, file_path: &str) -> std::io::Result<()> {
    let mut file = File::create(file_path).await?;
    file.write_all(json_str.as_bytes()).await?;
    Ok(())
}

pub async fn load_json_from_file(file_path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(file_path).await?;
    let value: Value = serde_json::from_str(&data)?;
    Ok(value)
}

pub async fn get_basic_info(player_uuid: &str) -> Option<String> {
    let profile = get_selected_profile(player_uuid).await?;
    let data = profile.data();

    let mut result = String::new();
    let mut first = true;

    let sections = vec![
        // get_sb_level(data),
        // get_skills(data),
        // get_currencies(data),
        // get_fairy_souls(data),
        // get_accessories_info(data),
        // get_pets_info(data),
        // get_mining_info(data),
        // get_slayer_info(data),
        // get_dungeons_info(data),
        // get_bestiary_info(data),
        // get_crimson_info(data),
        // get_rift_info(data),
        get_essence(data),
        // get_events_info().await
    ];

    for section in sections {
        if let Some(text) = section {
            if !first {
                result.push_str("\n\n");
            }
            result.push_str(&text);
            first = false;
        }
    }

    println!("{}", result);
    Some(result)
}

fn scan_storage(data: &Value) -> Storage {
    let mut storage = Storage::empty();
    let inventory = match data["inventory"].as_object() {
        None => return storage,
        Some(inventory) => inventory
    };

    storage.add_inventory(get_container_items(&inventory["inv_contents"], "INVENTORY"));
    storage.add_ender_chest(get_container_items(&inventory["ender_chest_contents"], "ENDER_CHEST"));

    if let Some(backpacks) = &inventory["backpack_contents"].as_object() {
        for (e, bp) in backpacks.iter() {
            storage.add_backpacks(get_container_items(&bp, &*("BACKPACK".to_owned() + e)));
        }
    }

    storage.add_armor(get_container_items(&inventory["inv_armor"], "ARMOR"));
    storage.add_equipment(get_container_items(&inventory["equipment_contents"], "EQUIPMENT"));
    storage.add_wardrobe(get_container_items(&inventory["wardrobe_contents"], "WARDROBE"));
    storage.add_accessories(get_container_items(&inventory["bag_contents"]["talisman_bag"], "ACCESSORY"));
    storage.add_vault(get_container_items(&inventory["personal_vault_contents"], "VAULT"));

    if let Some(sacks_counts) = &inventory["sacks_counts"].as_object() {
        let map: HashMap<String, u64> = sacks_counts
            .iter()
            .filter_map(|(k, v)| v.as_u64().map(|count| (k.clone(), count)))
            .collect();
        storage.add_sacks(map);
    }

    if let Some(pets) = data.get("pets_data").and_then(|v| v.get("pets")).and_then(|v| v.as_array()) {
        let mut pets_list = Vec::new();
        for pet in pets {
            let pet_type = pet.get("type").and_then(|v| v.as_str());
            let pet_tier = pet.get("tier").and_then(|v| v.as_str());
            let pet_xp = pet.get("exp").and_then(|v| v.as_f64());
            let held_item = pet.get("heldItem").and_then(|v| v.as_str()).map(|s| s.to_string());
            let skin = pet.get("skin").and_then(|v| v.as_str()).map(|s| s.to_string());
            let active = pet.get("active").and_then(|v| v.as_bool());

            if let (Some(name), Some(tier), Some(xp), Some(active)) = (pet_type, pet_tier, pet_xp, active) {
                pets_list.push(Pet::new(name.to_string(), tier.to_string(), xp, held_item, skin, active))
            }
        }
        storage.add_pets(pets_list);
    }

    storage
}

fn get_container_items(container: &Value, path: &str) -> Vec<Item> {
    let mut items = Vec::new();
    if let Some(contents) = container["data"].as_str() {
        if let Ok(items_data) = decode_inventory_base64(contents) {
            for (item, slot) in items_data {
                if let Some(mut item) = get_item_obj(item) {
                    let id = format!("{}-{}_{}", item.item_id(), path, slot);
                    item.set_id(id);
                    items.push(item);
                }
            }
        }
    }

    items
}

pub async fn search_item(player_uuid: &str, item_name: &str) -> Option<String> {
    let profile = get_selected_profile(player_uuid).await?;
    let storage = profile.storage();

    let item_id = item_name.to_uppercase().replace(" ", "_");
    if let Some(amount) = storage.sacks.get(&item_id) {
        return Some(format!("Found Item in sacks: {}x {}", amount, item_id))
    }

    // Collect all items and their names
    let all_items = storage.get_items_list();

    // Get item names for fuzzy matching
    let mut item_names: Vec<String> = all_items.iter()
        .map(|item| item.name().to_string())
        .collect();
    item_names.dedup();

    // Find the best match using fuzzy search
    if let Some(best_match) = find_best_match(item_name, &item_names) {
        let item = all_items.iter()
            .find(|item| item.name() == best_match)
            .map(|item| (*item).clone());

        if let Some(ref item) = item {
            let mut result = format!("Item: {} (id: {})", item.name(), item.id());

            // Check if this item has duplicates
            let duplicates = all_items.iter()
                .filter(|i| i.item_id() == item.item_id() && i.id() != item.id())
                .collect::<Vec<_>>();
            if duplicates.len() > 0 {
                let max = 3;
                let ids: Vec<&str> = duplicates.iter().take(max).map(|i| i.id()).collect();
                result.push_str(&format!(" (similar items: {}", ids.join(", ")));
                if duplicates.len() > max {
                    result.push_str(&format!(", {} more", duplicates.len() - max));
                }
                result.push_str(")");
            }

            return Some(result);
        }
    }

    None
}

pub async fn get_profile_networth(player_uuid: &str) -> Option<String> {
    let mut profile = get_selected_profile(player_uuid).await?;
    let storage = profile.storage();
    let mut result = Vec::new();

    let mut museum = 0;
    let mut total_value = 0;

    let containers = vec![
        ("Inventory", storage.inventory.iter()),
        ("Enderchest", storage.ender_chest.iter()),
        ("Backpacks", storage.backpacks.iter()),
        ("Armor", storage.armor.iter()),
        ("Equipment", storage.equipment.iter()),
        ("Wardrobe", storage.wardrobe.iter()),
        ("accessories", storage.accessories.iter()),
    ];

    for (name, items) in containers {
        let mut value = 0;
        for item in items {
            let mut item_value = ItemValue::new();
            let item_id = item.item_id();
            calculate_item_value(item_id.to_string(), item.nbt().clone(), &mut item_value).await;
            value += *item_value.total_value() as u64;
        }
        result.push(format!("{}: {}", name, format_number(value)));
        total_value += value;
    }

    let mut sacks_value = 0;
    for (item, amount) in storage.sacks.iter() {
        let price = get_item_price(item).await.unwrap_or(0.0) as u64;
        sacks_value += price * amount;
    }
    result.push(format!("Sacks: {}", format_number(sacks_value)));
    total_value += sacks_value;

    let mut pets_value = 0;
    for pet in storage.pets.iter() {
        let id = format!("{}_{}", pet.tier, pet.name);
        let price = get_lowest_bin(&id, false).await.unwrap_or((0.0, None));
        pets_value += price.0 as u64;//TODO: make all prices use u64
    }
    result.push(format!("Pets: {}", format_number(pets_value)));
    total_value += pets_value;

    let mut museum_value = 0;
    if let Some(museum_donations) = get_museum_items(player_uuid, &mut profile).await {
        for donation in museum_donations.iter() {
            if donation.borrowing { continue };
            for item in donation.items.iter() {
                //TODO: make fn for this, (used above)
                let mut item_value = ItemValue::new();
                let item_id = item.item_id();
                calculate_item_value(item_id.to_string(), item.nbt().clone(), &mut item_value).await;
                museum_value += *item_value.total_value() as u64;
            }
        }
    }
    result.push(format!("Museum: {}", format_number(museum_value)));
    total_value += museum_value;

    result.push(format!("Purse: {}", format_number(profile.purse())));
    result.push(format!("Bank: {}/{}", format_number(profile.bank().0), format_number(profile.bank().1)));

    let mut essence_value = 0;
    if let Some(essence) = get_essence_map(profile.data()) {
        for (name, amount) in essence {
            let price = get_item_price(&format!("ESSENCE_{}", name)).await.unwrap_or(0.0) as u64;
            essence_value += price * amount;
        }
    }
    total_value += essence_value;
    result.push(format!("Essence: {}", format_number(essence_value)));

    result.push(format!("\nProfile Networth: {}", format_number(total_value)));

    Some(result.join("\n"))
}

pub async fn get_inventory(player_uuid: &str) -> Option<String> {
    let profile = get_selected_profile(player_uuid).await?;
    let storage = profile.storage();
    let inventory = &storage.inventory;

    if inventory.is_empty() {
        return None;
    }

    let mut result = Vec::new();
    result.push("Inventory:".to_string());

    for item in inventory {
        result.push(format!("- {} (id: {})", item.name(), item.id()))
    }

    Some(result.join("\n"))
}

pub async fn get_armor(player_uuid: &str) -> Option<String> {
    let profile = get_selected_profile(player_uuid).await?;
    let storage = profile.storage();
    let mut result = Vec::new();

    let armor = &storage.armor;
    let equipment = &storage.equipment;

    if !armor.is_empty() {
        result.push("Armor:".to_string());
        for piece in armor {
            result.push(format!("- {} (id: {})", piece.name(), piece.id()));
        }
    }

    if !equipment.is_empty() {
        result.push("Equipments:".to_string());
        for equip in equipment {
            result.push(format!("- {} (id: {})", equip.name(), equip.id()));
        }
    }

    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn find_best_match<'a>(query: &'a str, list: &'a [String]) -> Option<&'a str> {
    // 1. Exact match
    if let Some(exact) = list.iter().find(|s| s.eq_ignore_ascii_case(query)) {
        return Some(exact);
    }

    // 2. Word-overlap priority
    let mut best: Option<(&str, usize, f64)> = None;
    for item in list {
        let overlap = word_overlap_score(item, query);
        let jw = jaro_winkler(item, query);
        match best {
            None => best = Some((item, overlap, jw)),
            Some((_, best_overlap, best_jw)) => {
                if overlap > best_overlap || (overlap == best_overlap && jw > best_jw) {
                    best = Some((item, overlap, jw));
                }
            }
        }
    }

    best.map(|(s, _, _)| s)
}

fn word_overlap_score(candidate: &str, query: &str) -> usize {
    let lowercase = query.to_lowercase();
    let query_words: Vec<_> = lowercase.split_whitespace().collect();
    let candidate = candidate.to_lowercase();

    query_words.iter().filter(|w| candidate.contains(*w)).count()
}

pub async fn get_item_info(player_uuid: &str, item_id: &str, price_info: bool) -> Option<String> {
    let profile = get_selected_profile(player_uuid).await?;
    let storage = profile.storage();

    let all_items = storage.get_items_list();
    let item = all_items.iter()
        .find(|item| item.id() == item_id)
        .cloned();

    let item = match item {
        None => return None,
        Some(i) => i
    };

    let mut item_value = ItemValue::new();
    let item_id = item.item_id();
    calculate_item_value(item_id.to_string(), item.nbt().clone(), &mut item_value).await;

    let info = item_value.build_info_string(item_id, price_info).await;
    if info.is_empty() { None } else { Some(info.join("\n")) }
}

pub async fn search_pet(player_uuid: &str, pet_name: &str) -> Option<String> {
    let profile = get_selected_profile(player_uuid).await?;
    let storage = profile.storage();

    let pet_names: Vec<String> = storage.pets.iter()
        .map(|pet| get_readable_name(&pet.name).to_string())
        .collect();

    if let Some(best_match) = find_best_match(pet_name, &pet_names) {
        let pet = storage.pets.iter()
            .find(|pet| get_readable_name(&pet.name) == best_match)
            .map(|pet| (*pet).clone());
        if let Some(pet) = pet {
            let mut result = String::new();
            let (level, progress) = get_pet_level(&pet.name, &pet.tier, pet.xp as u64);

            result.push_str(&format!("[Lvl {}] {} {}", level, get_readable_name(&pet.tier), get_readable_name(&pet.name)));

            if let Some(progress) = progress { result.push_str(&format!(" (Progress {}%)", progress)); }
            if pet.active { result.push_str(" (Active)"); }
            if let Some(held_item) = pet.held_item { result.push_str(&format!(" (Item Pet: {})", get_readable_name(&held_item))); }
            if let Some(skin) = pet.skin { result.push_str(&format!(" (Skin: {})", get_readable_name(&skin))); }

            return Some(result)
        }
    }

    None
}

fn get_pet_level(name: &str, rarity: &str, pet_xp: u64) -> (u64, Option<u64>) {
    let rarity = match name {
        "BINGO" => "COMMON",
        _ => rarity
    };

    let level_max = match name {
        "GOLDEN_DRAGON" | "JADE_DRAGON" => 200,
        _ => 100
    };

    let offset = *RARITY_OFFSETS.get(rarity).unwrap_or(&0) as usize;
    let pet_levels = &PET_LEVELS_XP[offset .. offset + level_max - 1];

    let mut level = 1;
    let mut total_exp = 0;
    let mut progress = None;

    for &xp in pet_levels.iter().take(level_max) {
        total_exp += xp;
        if total_exp > pet_xp {
            total_exp -= xp;
            progress = Some((((pet_xp - total_exp) as f64 / xp as f64) * 100.0) as u64);
            break;
        }
        level += 1;
    }

    (min(level, level_max as u64), progress)
}

fn get_item_obj(item: ItemNbt) -> Option<Item> {
    let count = item.count;

    if let Some(ref tag) = item.tag {
        let name = get_item_name(&item)?;
        if let Some(fastnbt::Value::Compound(attrs)) = tag.extra_attributes.as_ref() {
            if let Some(fastnbt::Value::String(item_id)) = attrs.get("id") {
                let item_obj = Item::new(item_id.clone(), name, count as u64, item);
                return Some(item_obj);
            }
        }
    }

    None
}

fn get_item_name(item: &ItemNbt) -> Option<String> {
    let tag = item.tag.as_ref()?;

    let display = match tag.display.as_ref() {
        Some(fastnbt::Value::Compound(v)) => v,
        _ => return None,
    };

    let name = match display.get("Name") {
        Some(fastnbt::Value::String(v)) => v,
        _ => return None,
    };

    let mut clean_name = remove_colors(name);
    if clean_name == "SkyBlock Menu (Click)" {
        return None;
    }
    if clean_name == "Enchanted Book" {
        if let fastnbt::Value::Compound(attrs) = tag.extra_attributes.as_ref()? {
            if let fastnbt::Value::Compound(enchant) = attrs.get("enchantments")? {
                let mut enchants = Vec::new();
                for (name, level) in enchant {
                    enchants.push(format!("{} {}", name, level.as_u64()?));
                }
                clean_name = format!("Enchanted Book ({})", enchants.join(", "))
            }
        }
    }

    Some(clean_name)
}

fn remove_colors(text: &str) -> String {
    text.chars()
        .scan(false, |skip_next, c| {
            if *skip_next {
                *skip_next = false;
                Some(None)
            } else if c == '§' {
                *skip_next = true;
                Some(None)
            } else {
                Some(Some(c))
            }
        })
        .filter_map(|c| c)
        .collect::<String>()
}

fn get_sb_level(data: &Value) -> Option<String> {
    let experience = data.get("leveling")?.get("experience")?.as_u64()?;
    let level = experience / 100;
    let progress = experience % 100;
    Some(format!("SkyBlock Level {} ({}/100)", level, progress))
}

fn get_skills(data: &Value) -> Option<String> {
    let skills = data.get("player_data")?.get("experience")?.as_object()?;

    let taming_cap = || {
        data.get("pets_data")
            .and_then(|v| v.get("pet_care"))
            .and_then(|v| v.get("pet_types_sacrificed"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.len() as u64)
            .unwrap_or(0)
    };

    let farming_cap = || {
        data.get("jacobs_contest")
            .and_then(|v| v.get("perks"))
            .and_then(|v| v.get("farming_level_cap"))
            .and_then(|v| v.as_i64())
            .map(|v| v as u64)
            .unwrap_or(0)
    };

    let cap_functions: &[(&str, &dyn Fn() -> u64)] = &[
        ("SKILL_TAMING", &taming_cap),
        ("SKILL_FARMING", &farming_cap),
    ];

    fn pretty_skill_name(skill: &str) -> String {
        skill.strip_prefix("SKILL_")
            .unwrap_or(skill)
            .to_ascii_lowercase()
            .split('_')
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    let mut result = String::from_str("Skills:\n").unwrap();
    let mut total_level = 0u64;
    let mut count = 0u64;

    for (skill, xp_value) in skills {
        let xp = xp_value.as_f64().unwrap_or(0.0) as u64;
        let xp_table: &[u64] = match skill.as_str() {
            "SKILL_RUNECRAFTING" => &RUNECRAFTING_SKILL_XP,
            "SKILL_SOCIAL" => &SOCIAL_SKILL_XP,
            _ => &SKILLS_XP,
        };

        let level = xp_table.iter().enumerate()
            .take_while(|&(_, &threshold)| xp >= threshold)
            .map(|(i, _)| i as u64)
            .last()
            .unwrap_or(0);

        let max_level = *SKILL_MAX_LEVELS.get(skill.as_str()).unwrap_or(&60);
        let mut cap_info = None;
        let mut cap_limit = max_level;

        for &(name, cap_fn) in cap_functions {
            if skill == name {
                let cap_int = cap_fn();
                let cap = (50 + cap_int).min(max_level);
                if cap_int > 0 {
                    cap_info = Some(cap);
                    cap_limit = cap;
                }
                break;
            }
        }

        let display_level = level.min(cap_limit);

        // Calculate percentage to next level if not maxed/capped
        let mut percent_to_next = None;
        if display_level < cap_limit {
            let curr_lvl_xp = xp_table.get(display_level as usize).copied().unwrap_or(0);
            let next_lvl_xp = xp_table.get(display_level as usize + 1).copied().unwrap_or(curr_lvl_xp);
            let gained = xp.saturating_sub(curr_lvl_xp);
            let needed = next_lvl_xp.saturating_sub(curr_lvl_xp);
            if needed > 0 {
                let percent = (gained as f64 / needed as f64 * 100.0).floor() as u64;
                percent_to_next = Some(
                    format!("{}/{} {}% to next level",
                            format_number(gained),
                            format_number(needed),
                            percent
                    )
                );
            }
        }

        // Exclude cosmetic skills from average
        if skill != "SKILL_RUNECRAFTING" && skill != "SKILL_SOCIAL" {
            total_level += display_level;
            count += 1;
        }

        let mut line = String::new();
        line.push_str(&format!("{}: {} (max: {}", pretty_skill_name(skill), display_level, max_level));

        if let Some(cap) = cap_info {
            line.push_str(&format!(", cap: {}", cap));
        }
        if let Some(progress) = percent_to_next {
            line.push_str(&format!(", {}", progress));
        }
        line.push(')');

        result.push_str(&line);
        result.push('\n');
    }

    if count > 0 {
        let avg = total_level as f64 / count as f64;
        result.push_str(&format!("Average Skill Level: {:.2}", avg));
    }

    Some(result)
}

//TODO: use profile.purse() and bank
fn get_currencies(data: &Value) -> Option<String> {
    let currencies = data.get("currencies")?;
    let purse = currencies.get("coin_purse")?.as_f64()?;

    Some(format!("Purse: {}", format_number(purse as u64)))
}

fn get_fairy_souls(data: &Value) -> Option<String> {
    let fairy_souls = data.get("fairy_soul")?.get("total_collected")?.as_i64()?;

    Some(format!("FairySouls: {}/{}", fairy_souls, MAX_FAIRY_SOULS))
}

fn get_accessories_info(data: &Value) -> Option<String> {
    let mut result = String::new();
    let storage = data.get("accessory_bag_storage")?;

    if let Some(selected_power) = storage.get("selected_power") {
        result.push_str(&format!("Selected Power: {}\n", selected_power.as_str().unwrap_or("Nothing")));
    }

    if let Some(mp) = storage.get("highest_magical_power") {
        result.push_str(&format!("Magical Power: {}\n", mp));
    }

    if let Some(unlocked_powers) = storage.get("unlocked_powers") {
        if let Some(powers_list) = unlocked_powers.as_array() {
            let values = powers_list
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            result.push_str(&format!("Unlocked Powers: [{}]", values));
        }
    }

    Some(result)
}

fn get_pets_info(data: &Value) -> Option<String> {
    let mut result = String::new();
    let pets = data.get("pets_data")?.get("pets")?.as_array()?;
    if let Some(active_pet) = pets.iter().find(|pet| pet.get("active").and_then(|v| v.as_bool()) == Some(true)) {
        let pet_type = active_pet.get("type").and_then(|v| v.as_str())?;
        let pet_tier = active_pet.get("tier").and_then(|v| v.as_str())?;
        let xp = active_pet.get("xp").and_then(|v| v.as_f64())?;
        let (level, _) = get_pet_level(pet_type, pet_tier, xp as u64);

        result.push_str(&format!("Active Pet: [Lvl {}] {} {}\n", level, get_readable_name(pet_tier), get_readable_name(pet_type)));
    }

    if let Some(pet_score) = data.get("leveling")?.get("highest_pet_score")?.as_i64() {
        result.push_str(&format!("Pet Score: {}", pet_score));
    }

    Some(result)
}

fn get_mining_info(data: &Value) -> Option<String> {
    let mut result = Vec::new();
    if let Some(mining_core) = data.get("mining_core") {
        let powders = [
            ("powder_mithril_total", "Mithril powder"),
            ("powder_gemstone_total", "Gemstone powder"),
            ("powder_glacite_total", "Glacite powder"),
        ];

        for (key, label) in powders.iter() {
            if let Some(powder) = mining_core.get(*key).and_then(|v| v.as_u64()) {
                result.push(format!("{}: {}", label, format_number(powder)));
            }
        }
    }

    if let Some(nucleus_runs) = data.get("leveling")
        .and_then(|v| v.get("completions"))
        .and_then(|v| v.get("NUCLEUS_RUNS"))
        .and_then(|v| v.as_u64()) {
        result.push(format!("Nucleus runs: {}", nucleus_runs));
    }

    if let Some(tutorial) = data.get("objectives").and_then(|v| v.get("tutorial")).and_then(|v| v.as_array()) {
        for lvl in (1..=6).rev() {
            let quest_id = format!("commission_milestone_reward_skyblock_xp_tier_{}", lvl);
            if tutorial.iter().any(|v| v.as_str() == Some(&quest_id)) {
                result.push(format!("Commission milestone: {}", lvl));
                break;
            }
        }
    }

    if let Some(glacite_core) = data.get("glacite_player_data") {
        if let Some(mineshafts_entered) = glacite_core.get("mineshafts_entered").and_then(|v| v.as_u64()) {
            result.push(format!("Mineshafts entered: {}", mineshafts_entered));
        }
        if let Some(corpses_looted) = glacite_core.get("corpses_looted").and_then(|v| v.as_object()) {
            result.push("Corpses looted:".to_string());
            for (corpse, count) in corpses_looted.iter() {
                result.push(format!("- {}: {}", get_readable_name(corpse), count));
            }
        }
        if let Some(fossils_donated) = glacite_core.get("fossils_donated").and_then(|v| v.as_array()) {
            result.push("Fossils donated:".to_string());
            for fossil in fossils_donated.iter() {
                result.push(format!("- {}", fossil.as_str().map(get_readable_name).unwrap_or_default()));
            }
        }
    }

    if result.is_empty() { None } else { Some(result.join("\n")) }
}

pub async fn get_garden_info(player_uuid: &str) -> Option<String> {
    let mut profile = get_selected_profile(player_uuid).await?;
    let garden_data = get_garden_data(&mut profile).await;
    let mut result = Vec::new();

    if let Some(garden_data) = garden_data {
        if let Some(garden) = garden_data.get("garden") {
            if let Some(xp) = garden.get("garden_experience").and_then(|v| v.as_f64()) {
                let level = GARDEN_LEVELS_XP.iter().enumerate()
                    .take_while(|&(_, &threshold)| xp >= threshold as f64)
                    .map(|(i, _)| i as u64)
                    .last()
                    .unwrap_or(1);

                result.push(format!("Garden Level: {}/{}", level, MAX_GARDEN_LEVEL));
            }

            let commission_data = garden.get("commission_data");
            let visitors_served = commission_data.and_then(|v| v.get("total_completed")).and_then(|v| v.as_u64()).unwrap_or(0);
            let unique_visitors = commission_data.and_then(|v| v.get("unique_npcs_served")).and_then(|v| v.as_u64()).unwrap_or(0);
            let plots = garden.get("unlocked_plots_ids").and_then(|v| v.as_array()).and_then(|v| Some(v.len())).unwrap_or(0);

            result.push(format!("Visitors served: {}", visitors_served));
            result.push(format!("Unique visitors served: {}", unique_visitors));
            result.push(format!("Unlocked Plots: {}/{}", plots, MAX_PLOTS));

            if let Some(composter_upgrades) = garden.get("composter_data").and_then(|v| v.get("upgrades")).and_then(|v| v.as_object()) {
                result.push("Composter upgrades:".to_string());
                for (upgrade, level) in composter_upgrades {
                    result.push(format!("- {}: {}/{}", get_readable_name(upgrade), level, MAX_COMPOSTER_UPGRADE_LEVEL));
                }
            }

            if let Some(crop_milestones) = garden.get("resources_collected").and_then(|v| v.as_object()) {
                result.push("Crop Milestones:".to_string());
                for (crop, crop_xp) in crop_milestones {
                    let crop_xp = crop_xp.as_u64()?;
                    let crop = crop.as_str();

                    let xp_table = match crop {
                        "WHEAT" | "PUMPKIN" | "MUSHROOM_COLLECTION" => WHEAT_PUMPKIN_MUSHROOM_MILESTONE_XP,
                        "CARROT_ITEM" | "POTATO_ITEM" => CARROT_POTATO_MILESTONE_XP,
                        "SUGAR_CANE" | "CACTUS" => CANE_CACTUS_MILESTONE_XP,
                        "MELON" => MELON_MILESTONE_XP,
                        "INK_SACK:3" | "NETHER_STALK" => COCOA_WART_MILESTONE_XP,
                        _ => {continue}
                    };

                    //TODO: transfer this to stand-alone fn and use it for all levels
                    //TODO: also check on all xp tables
                    let mut level = 0;
                    let mut total_exp = 0;
                    let mut progress = None;

                    for &xp in xp_table.iter() {
                        total_exp += xp;
                        if total_exp > crop_xp {
                            total_exp -= xp;
                            progress = Some((((crop_xp - total_exp) as f64 / xp as f64) * 100.0) as u64);
                            break;
                        }
                        level += 1;
                    }

                    let mut line = format!("- {}: {}/{}", CROP_NAMES.get(crop)?, level, MAX_CROP_MILESTONE);
                    if progress.is_some() { line.push_str(&format!(" (Progress {}%)", progress?))}
                    result.push(line);
                }
            }

            //TODO: default values? for all, data may not exist if 0
            if let Some(crop_upgrades) = garden.get("crop_upgrade_levels").and_then(|v| v.as_object()) {
                result.push("Crop Upgrades:".to_string());
                for (crop, level) in crop_upgrades {
                    let level = level.as_u64()?;
                    result.push(format!("- {}: {}/{}", CROP_NAMES.get(crop)?, level, MAX_CROP_UPGRADE_LEVEL));
                }
            }
        }
    }

    if let Some(medals_inv) = profile.data().get("jacobs_contest").and_then(|v| v.get("medals_inv")) {
        result.push("Jacob Medals:".to_string());
        let brackets = vec!["bronze", "silver", "gold", "platinum", "diamond"];
        for bracket in brackets {
            let amount = match medals_inv.get(bracket) {
                None => 0,
                Some(v) => v.as_u64()?
            };
            result.push(format!("- {}: {}", get_readable_name(bracket), amount))
        }
    }

    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn get_slayer_info(data: &Value) -> Option<String> {
    let mut result = Vec::new();
    if let Some(slayer_bosses) = data.get("slayer").and_then(|v| v.get("slayer_bosses")).and_then(|v| v.as_object()) {
        result.push("Slayers:".to_string());
        for (slayer, data) in slayer_bosses.iter() {
            let level = data.get("claimed_levels").and_then(|v| v.as_object()).map(|v| v.len());
            let xp = data.get("xp").and_then(|v| v.as_u64());
            if let (Some(level), Some(xp)) = (level, xp) {
                result.push(format!("- {} level: {} (xp: {})", slayer, level, xp));
            }
        }
    }

    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn get_dungeons_info(data: &Value) -> Option<String> {
    let mut result = Vec::new();

    let max_level = SKILL_MAX_LEVELS.get("SKILL_DUNGEONEERING")?;
    let dungeons = data.get("dungeons")?;
    let dungeon_types = dungeons.get("dungeon_types")?;

    let catacombs = dungeon_types.get("catacombs")?;
    let xp = catacombs.get("experience")?.as_f64()?;
    let highest_tier_completed = catacombs.get("highest_tier_completed")?.as_u64()?;
    let milestone_completions = catacombs.get("milestone_completions")?.as_object()?;

    fn get_level(xp: f64) -> f64 {
        DUNGEONEERING_SKILL_XP.iter().enumerate()
            .take_while(|&(_, &threshold)| xp >= threshold as f64)
            .map(|(i, _)| i as f64)
            .last()
            .unwrap_or(0.0)
    }

    result.push(format!("Dungeons Level: {}/{}", get_level(xp), max_level));
    if let Some(classes) = dungeons.get("player_classes").and_then(|v| v.as_object()) {
        result.push("Dungeon Classes:".to_string());
        for (class, xp) in classes.iter() {
            let level = get_level(xp.get("experience")?.as_f64()?);
            result.push(format!("- {}: {}/{}", get_readable_name(class), level, max_level));
        }
    }

    result.push("Catacombs:".to_string());
    result.push(format!("- Highest floor completed: {}", highest_tier_completed));
    for (floor, completions) in milestone_completions {
        if floor == "total" { continue };
        let floor_name = if floor == "0" { "Entrance" } else {
            &*format!("Floor {}", floor)
        };
        result.push(format!("- {}: {} runs", floor_name, completions.as_f64()?))
    }

    if let Some(master_catacombs) = dungeon_types.get("master_catacombs") {
        result.push("Master Mode:".to_string());
        if let Some(highest_tier_completed) = master_catacombs.get("highest_tier_completed").and_then(|v| v.as_u64()) {
            result.push(format!("- Highest floor completed: {}", highest_tier_completed));
        }
        if let Some(milestone_completions) = master_catacombs.get("milestone_completions").and_then(|v| v.as_object()) {
            for (floor, completions) in milestone_completions {
                if floor == "total" { continue };
                result.push(format!("- Floor {}: {} runs", floor, completions.as_f64()?))
            }
        }
    }

    if let Some(secrets) = dungeons.get("secrets").and_then(|v| v.as_u64()) {
        result.push(format!("Secrets: {}", secrets));
    }

    if let Some(selected_class) = dungeons.get("selected_dungeon_class") {
        result.push(format!("Selected Dungeon Class: {}", get_readable_name(selected_class.as_str()?)));
    }

    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn get_bestiary_info(data: &Value) -> Option<String> {
    let bestiary = data.get("bestiary")?;
    let milestone = bestiary.get("milestone")?;
    let last_claimed_milestone = milestone.get("last_claimed_milestone")?.as_u64()?;

    Some(format!("Bestiary Level: {}/{}", last_claimed_milestone, BESTIARY_MAX_LEVEL))
}

fn get_crimson_info(data: &Value) -> Option<String> {
    let mut result = Vec::new();
    let crimson_data = data.get("nether_island_player_data")?;

    if let Some(selected_faction) = crimson_data.get("selected_faction").and_then(|v| v.as_str()) {
        result.push(format!("Selected Faction: {}", get_readable_name(selected_faction)));
    }

    if let Some(mages_rep) = crimson_data.get("mages_reputation").and_then(|v| v.as_f64()) {
        result.push(format!("Mages reputation: {}", mages_rep as u64));
    }

    if let Some(barb_rep) = crimson_data.get("barbarians_reputation").and_then(|v| v.as_f64()) {
        result.push(format!("Barbarians reputation: {}", barb_rep as u64));
    }

    if let Some(kuudra) = crimson_data.get("kuudra_completed_tiers").and_then(|v| v.as_object()) {
        let tiers = vec!["none", "hot", "burning", "fiery", "infernal"];
        result.push("Kuudra:".to_string());
        for tier in tiers {
            let comp = kuudra.get(tier).and_then(|v| v.as_u64()).unwrap_or(0);
            let tier_name = if tier == "none" { "Basic".to_string() } else { get_readable_name(tier) };
            result.push(format!("- {}: {} runs", tier_name, comp))
        }
    }

    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn get_rift_info(data: &Value) -> Option<String> {
    let mut result = Vec::new();
    let rift = data.get("rift")?;

    if let Some(enigma_souls) = rift.get("enigma").and_then(|v| v.get("found_souls")).and_then(|v| v.as_array()) {
        result.push(format!("Enigma souls: {}/{}", enigma_souls.len(), MAX_ENIGMA_SOULS));
    }

    if let Some(motes) = data.get("currencies").and_then(|v| v.get("motes_purse")).and_then(|v| v.as_f64()) {
        result.push(format!("Motes: {}", motes));
    }

    if let Some(timecharms) = rift.get("gallery").and_then(|v| v.get("secured_trophies")).and_then(|v| v.as_array()) {
        result.push(format!("Timecharms: {}/{}", timecharms.len(), MAX_TIMECHARMS));
    }

    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn get_essence_map(data: &Value) -> Option<Vec<(&str, u64)>> {
    let essence = data.get("currencies")?.get("essence")?.as_object()?;
    let mut list = Vec::new();

    for (name, amount) in essence.iter() {
        let amount = amount.get("current").and_then(|v| v.as_u64()).unwrap_or(0);
        list.push((name.as_str(), amount));
    }

    Some(list)
}

fn get_essence(data: &Value) -> Option<String> {
    let mut result = Vec::new();

    let essence = get_essence_map(data)?;
    result.push("Essence:".to_string());
    for (name, amount) in essence.iter() {
        result.push(format!("- {}: {}", get_readable_name(name), amount));
    }

    if result.is_empty() { None } else { Some(result.join("\n")) }
}

async fn get_events_info() -> Option<String> {
    let mut result = Vec::new();

    result.push(format!("SkyBlock Year: {}", get_skyblock_year().await));

    let mayor_info = get_mayor_info().await;
    let mayor = mayor_info.get_mayor();
    let mayor_perks = mayor.get_perks().keys().map(|s| s.as_str()).collect::<Vec<_>>().join("/");
    result.push(format!("Mayor: {} (perks: {})", mayor.get_name(), mayor_perks));

    if let Some(minister) = mayor_info.get_minister() {
        let minister_perk = minister.get_perks().keys().map(|s| s.as_str()).collect::<Vec<_>>().join("/");
        result.push(format!("Minister: {} (perks: {})", minister.get_name(), minister_perk));
    }

    let upcoming_contests = get_upcoming_contests().await;
    if !upcoming_contests.is_empty() {
        result.push("Jacob Contests:".to_string());
        for (time, crops) in upcoming_contests.iter() {
            if let Ok(timestamp) = time.parse::<u64>() {
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let time_diff = timestamp.saturating_sub(current_time);
                let total_minutes = time_diff / 60;

                // Format crops joined by "/"
                let crops_str = crops.join("/");

                // Convert timestamp to HH:MM:SS format
                let hours = (timestamp % 86400) / 3600;
                let minutes = (timestamp % 3600) / 60;
                let seconds = timestamp % 60;
                let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

                result.push(format!("- {} after {}mins (at {})", crops_str, total_minutes, formatted_time));
            }
        }
    }

    if result.is_empty() { None } else { Some(result.join("\n")) }
}