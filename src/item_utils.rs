use std::collections::HashSet;
use std::error::Error;
use std::io::Cursor;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use fastnbt::{from_reader, from_value, Value};
use flate2::read::GzDecoder;
use crate::item_value_calculator::MODIFIERS;
use crate::structs::ItemNbt;

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
                _ => return Err("[Auctions] Missing 'i' list".into()),
            }
        }
        _ => return Err("[Auctions] Root is not a compound".into()),
    };

    let item_nbt: ItemNbt = from_value(&i0)?;

    Ok(item_nbt)
}

pub fn extract_modifiers(item_nbt: &ItemNbt) -> HashSet<String> {
    let mut found = HashSet::new();
    let Some(Value::Compound(attributes)) = item_nbt.tag.as_ref().and_then(|tag| tag.extra_attributes.as_ref()) else { return found };
    for modifier in MODIFIERS.keys() {
        if attributes.contains_key(*modifier) {
            found.insert(modifier.to_string());
        }
    }
    found
}