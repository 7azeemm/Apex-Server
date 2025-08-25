use std::collections::HashSet;
use std::error::Error;
use std::io::Cursor;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use fastnbt::{from_reader, from_value, Value};
use flate2::read::GzDecoder;
use crate::item_value_calculator::MODIFIERS;
use crate::structs::ItemNbt;

pub fn get_readable_name(text: &str) -> String {
    text.to_lowercase()
        .replace("enchantment", "")
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
    let extra_attributes = item_nbt.tag.as_ref()?.extra_attributes.as_ref()?;

    let extra_map = match extra_attributes {
        Value::Compound(map) => map,
        _ => return None,
    };

    match extra_map.get("uuid")? {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    }
}

pub fn get_item_id(item_nbt: &ItemNbt) -> Option<String> {
    let extra_attributes = item_nbt.tag.as_ref()?.extra_attributes.as_ref()?;

    let extra_map = match extra_attributes {
        Value::Compound(map) => map,
        _ => return None,
    };

    let id_value = extra_map.get("id")?;
    let id = match id_value {
        Value::String(s) => s,
        _ => return None,
    };

    match id.as_str() {
        "PET" => {
            let pet_info_json = match extra_map.get("petInfo")? {
                Value::String(s) => s,
                _ => return None,
            };

            let extract_value = |key: &str| -> Option<&str> {
                let key_pattern = format!("\"{}\":\"", key);
                let start = pet_info_json.find(&key_pattern)? + key_pattern.len();
                let end = pet_info_json[start..].find('"')? + start;
                Some(&pet_info_json[start..end])
            };

            let pet_type = extract_value("type")?;
            let pet_tier = extract_value("tier")?;

            Some(format!("{}_{}", pet_tier, pet_type))
        }
        "POTION" => {
            let potion_name = match extra_map.get("potion")? {
                Value::String(s) => s,
                _ => return None,
            };
            let potion_level = match extra_map.get("potion_level")? {
                Value::Int(level) => level,
                _ => return None,
            };
            Some(format!("{}_POTION_{}", potion_name.to_uppercase(), potion_level))
        }
        "RUNE" => {
            let runes_value = extra_map.get("runes")?;
            let runes_map = match runes_value {
                Value::Compound(map) => map,
                _ => return None,
            };
            let (rune_type, rune_level_value) = runes_map.iter().next()?;
            let rune_level = match rune_level_value {
                Value::Int(level) => level,
                _ => return None,
            };
            Some(format!("{}_RUNE_{}", rune_type.to_uppercase(), rune_level))
        }
        _ => Some(id.to_string()),
    }
}

pub fn decode_base64(encoded: &str) -> Result<ItemNbt, Box<dyn Error>> {
    let bytes = STANDARD.decode(encoded)?;
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

//TODO: rename to decode multiple or smth
pub fn decode_inventory_base64(encoded: &str) -> Result<Vec<(ItemNbt, u64)>, Box<dyn Error>> {
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
    let mut slot = 0;
    for item_value in items_list {
        match from_value::<ItemNbt>(&item_value) {
            Ok(item_nbt) => items.push((item_nbt, slot)),
            Err(_) => {}, // Skip Empty slots
        }
        slot += 1;
    }

    Ok(items)
}

pub fn format_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}b", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}m", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }.replace(".0", "").to_string()
}