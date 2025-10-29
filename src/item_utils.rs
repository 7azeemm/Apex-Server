use crate::constants::misc::RARITIES;
use crate::constants::pets::{PET_LEVELS_XP, RARITY_OFFSETS};
use crate::extensions::fastnbt_ext::ValueExt;
use crate::extensions::json_ext::JsonExt;
use crate::structs::item_structs::ItemNbt;
use crate::structs::player_data_structs::Pet;
pub(crate) use crate::utils::strip_formatting;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use fastnbt::{from_reader, from_value, Value};
use flate2::read::GzDecoder;
use std::cmp::min;
use std::error::Error;
use std::io::Cursor;

pub fn get_pretty_name(text: &str) -> String {
    text.to_lowercase()
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

pub fn get_item_uuid(item_nbt: &ItemNbt) -> Option<String> {
    item_nbt.get_extra_map()?.get("uuid")?.as_str().map(|s| s.to_string())
}

pub fn get_item_id(item_nbt: &ItemNbt) -> Option<String> {
    let extra_map = item_nbt.get_extra_map()?;

    let id = extra_map.get("id")?.as_str()?;
    match id {
        "PET" => {
            let pet_info_str = extra_map.get("petInfo")?.as_str()?;
            let pet_info: serde_json::Value = serde_json::from_str(pet_info_str).ok()?;

            let pet_type = pet_info.get("type")?.as_str()?;
            let pet_tier = pet_info.get("tier")?.as_str()?;

            Some(format!("{}_{}", pet_tier, pet_type))
        }
        "POTION" => {
            let potion_name = extra_map.get("potion")?.as_str()?;
            let potion_level = extra_map.get("potion_level")?.as_u64()?;
            Some(format!("{}_POTION_{}", potion_name.to_uppercase(), potion_level))
        }
        "RUNE" => {
            let runes = extra_map.get("runes")?.as_compound()?;
            let (rune_type, rune_level) = runes.iter().next()?;
            let rune_level = rune_level.as_u64()?;
            Some(format!("{}_RUNE_{}", rune_type.to_uppercase(), rune_level))
        }
        _ => Some(id.to_string()),
    }
}

pub fn get_item_name(item_nbt: &ItemNbt) -> Option<String> {
    let display = item_nbt.get_display_map()?;
    let name = display.get("Name")?.as_str().map(|s| s.to_string())?;

    let clean_name = strip_formatting(&name);
    match clean_name.as_str() {
        "SkyBlock Menu (Click)" => None,
        "Enchanted Book" => {
            if let Some(Value::Compound(enchantments)) = item_nbt.get_extra_map()?.get("enchantments") {
                let mut enchants = Vec::new();
                for (name, level) in enchantments {
                    enchants.push(format!("{} {}", name, level.as_u64()?));
                }
                return Some(format!("Enchanted Book ({})", enchants.join(", ")))
            };
            Some(clean_name)
        }
        _ => Some(clean_name)
    }
}

pub fn get_item_rarity(item_nbt: &ItemNbt) -> Option<String> {
    let display = item_nbt.get_display_map()?;
    let lore = display.get("Lore")?.as_list()?;
    let last_line = lore.last()?.as_str()?.replace("SHINY ", "");
    let mut stripped_line = strip_formatting(&last_line);

    while let Some(first_char) = stripped_line.chars().next() {
        if first_char.is_uppercase() { break }
        stripped_line = stripped_line.chars().skip(1).collect();
    }

    for rarity in RARITIES {
        if stripped_line.starts_with(rarity) {
            return Some(rarity.to_string());
        }
    }

    None
}

pub fn get_rarity_index(rarity: &str) -> Option<usize> {
    RARITIES.iter().position(|&x| x == rarity)
}

pub fn decode_item(encoded_str: &str) -> Result<ItemNbt, Box<dyn Error>> {
    let bytes = STANDARD.decode(encoded_str)?;
    let mut decoder = GzDecoder::new(Cursor::new(bytes));
    let root: Value = from_reader(&mut decoder)?;

    let i0 = match root {
        Value::Compound(mut root_map) => {
            match root_map.remove("i") {
                Some(Value::List(mut list)) => list.remove(0),
                _ => return Err("[Decoder] Missing 'i' list".into()),
            }
        }
        _ => return Err("[Decoder] Root is not a compound".into()),
    };

    let item_nbt: ItemNbt = from_value(&i0)?;
    Ok(item_nbt)
}

pub fn decode_items(encoded: &str, keep_empty_slots: bool) -> Result<Vec<Option<ItemNbt>>, Box<dyn Error>> {
    let bytes = STANDARD.decode(encoded)?;
    let mut decoder = GzDecoder::new(Cursor::new(bytes));
    let root: Value = from_reader(&mut decoder)?;

    let items_list = match root {
        Value::Compound(mut root_map) => {
            match root_map.remove("i") {
                Some(Value::List(list)) => list,
                _ => return Err("[Decoder] Missing 'i' list in inventory".into()),
            }
        }
        _ => return Err("[Decoder] Root is not a compound".into()),
    };

    let mut items = Vec::new();
    for item_value in items_list {
        match from_value::<ItemNbt>(&item_value) {
            Ok(item_nbt) => items.push(Some(item_nbt)),
            Err(_) => { if keep_empty_slots { items.push(None) } },
        }
    }

    Ok(items)
}

pub fn get_pet_level(name: &str, rarity: &str, pet_xp: u64) -> (u64, Option<u64>) {
    let rarity = match name {
        "BINGO" => "COMMON",
        _ => rarity
    };

    let level_max = match name {
        "GOLDEN_DRAGON" | "JADE_DRAGON" => 200,
        _ => 100
    };

    let offset = *RARITY_OFFSETS.get(rarity).unwrap_or(&0) as usize;
    let pet_levels = &PET_LEVELS_XP[offset..offset + level_max - 1];

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

pub fn get_pet_info(pet: &Pet) -> Option<String> {
    let pet_name = pet.name();
    let pet_tier = pet.tier();
    let (level, _) = get_pet_level(pet_name, pet_tier, *pet.xp() as u64);

    Some(format!("[Lvl {level}] {} {}", get_pretty_name(pet_tier), get_pretty_name(pet_name)))
}

pub fn get_pet_obj(pet: &serde_json::Value) -> Option<Pet> {
    let pet_type = pet.get_str("type")?;
    let pet_tier = pet.get_str("tier")?;
    let pet_xp = pet.get_f64("exp")?;
    let held_item = pet.get_str("heldItem").map(|s| s.to_owned());
    let skin = pet.get_str("skin").map(|s| s.to_owned());
    let active = pet.get_bool("active").unwrap_or(false);

    Some(Pet::new(pet_type.to_owned(), pet_tier.to_owned(), pet_xp, held_item, skin, active))
}