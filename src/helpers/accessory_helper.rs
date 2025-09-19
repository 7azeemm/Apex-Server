use crate::constants::misc::RARITIES;
use crate::item_utils::{get_item_rarity, get_rarity_index};
use crate::repo::items::ACCESSORIES;
use crate::repo::talisman_upgrades::{get_talisman_upgrades, IGNORED_TALISMANS};
use crate::structs::player_data_structs::{PlayerDataResponse, StringBuilder};
use std::collections::{HashMap, HashSet};

pub async fn get_missing_accessories(pdr: &mut PlayerDataResponse, page: u64) {
    let mut sb = StringBuilder::new();
    let player_accessories: HashMap<String, (String, bool)> = pdr.profile().storage().accessories()
        .iter()
        .map(|i| {
            let item_id = i.item_id().to_owned();
            let rarity = get_item_rarity(i.nbt()).unwrap();
            let is_recombed = i.nbt().get_extra_map().unwrap().contains_key("rarity_upgrades");
            (item_id, (rarity, is_recombed))
        })
        .collect();

    let mut skip = HashSet::new();
    let mut missing = HashMap::new();

    let all_accessories = ACCESSORIES.read().await;
    for (talisman_id, rarity) in all_accessories.iter() {
        if skip.contains(talisman_id) { continue; }
        if player_accessories.contains_key(talisman_id) {
            skip.insert(talisman_id.to_owned());
        } else if let Some(upgrade_line) = get_talisman_upgrades(talisman_id).await {
            let mut owned_talisman = None;
            for (i, upgrade) in upgrade_line.iter().enumerate() {
                skip.insert(upgrade.to_owned());
                if upgrade != talisman_id && player_accessories.contains_key(upgrade) {
                    owned_talisman = Some((i, upgrade))
                }
            }
            let mut last_rarity_index: i64 = -1;
            let mut last_added = None;
            for (i, upgrade) in upgrade_line.iter().enumerate() {
                let rarity = all_accessories.get(upgrade).unwrap();
                let rarity_index = get_rarity_index(rarity).unwrap() as i64;
                let replace_last = rarity_index == last_rarity_index && last_added.is_some();
                if replace_last || rarity_index > last_rarity_index {
                    if replace_last {
                        missing.remove(last_added.unwrap());
                    }
                    if let Some((index, talisman)) = owned_talisman {
                        if i > index {
                            missing.insert(upgrade.to_owned(), (rarity.to_owned(), Some(talisman.to_owned())));
                        }
                    } else {
                        missing.insert(upgrade.to_owned(), (rarity.to_owned(), None));
                    }
                }
                last_rarity_index = rarity_index;
                last_added = Some(upgrade);
            }
        } else {
            missing.insert(talisman_id.to_owned(), (rarity.to_owned(), None));
        }
    }

    for ignored in IGNORED_TALISMANS.read().await.iter() {
        if missing.contains_key(ignored) {
            missing.remove(ignored);
        }
    }

    let player_has_hat = player_accessories.iter().any(|(id, _)| id.starts_with("PARTY_HAT_") || id.starts_with("BALLOON_HAT_"));
    let mut to_remove = Vec::new();
    let mut to_add = Vec::new();
    let mut kept_one_hat = false;

    for (missing_talis, _) in missing.iter() {
        if missing_talis == "CRACKED_PIGGY_BANK" || missing_talis == "BROKEN_PIGGY_BANK" {
            to_remove.push(missing_talis.to_owned());
            to_add.push(("PIGGY_BANK", ("UNCOMMON", None)));
        }

        if missing_talis.starts_with("PARTY_HAT_") || missing_talis.starts_with("BALLOON_HAT_") {
            if !player_has_hat && !kept_one_hat {
                kept_one_hat = true;
                continue;
            }
            to_remove.push(missing_talis.to_owned());
        }
    }

    for item in to_remove {
        missing.remove(&item);
    }

    for (item, (rarity, downgrade)) in to_add {
        missing.insert(item.to_owned(), (rarity.to_owned(), downgrade));
    }

    let other_upgrades = vec![
        ("PULSE_RING", vec!["UNCOMMON", "RARE", "EPIC", "LEGENDARY"]),
        ("BOOK_OF_PROGRESSION", vec!["COMMON", "UNCOMMON", "RARE", "EPIC", "LEGENDARY", "MYTHIC"]),
        ("RUNEBOOK", vec!["COMMON", "UNCOMMON", "RARE", "EPIC", "LEGENDARY"]),
        ("PANDORAS_BOX", vec!["COMMON", "UNCOMMON", "RARE", "EPIC", "LEGENDARY", "MYTHIC"]),
        ("TRAPPER_CREST", vec!["COMMON", "UNCOMMON"]),
    ];

    for (id, rarities) in other_upgrades {
        if let Some((rarity, is_recomb)) = player_accessories.get(id) {
            let rarity_index = rarities.iter().position(|r| r == rarity).unwrap();
            let downgrade = format!("{id}_{rarity}");
            for (i, talis_rarity) in rarities.iter().enumerate() {
                if (*is_recomb && i > rarity_index - 1) || i > rarity_index {
                    let mut talis_rarity = match is_recomb {
                        false => talis_rarity,
                        true => {
                            let rarity_pos = RARITIES.iter().position(|r| r == talis_rarity).unwrap();
                            RARITIES.get(rarity_pos + 1).unwrap_or_else(|| talis_rarity)
                        }
                    };
                    missing.insert(format!("{id}_{talis_rarity}"), (talis_rarity.to_string(), Some(downgrade.clone())));
                }
            }
        } else {
            missing.remove(id);
            for rarity in rarities {
                missing.insert(format!("{id}_{rarity}"), (rarity.to_owned(), None));
            }
        }
    }

    sb.push(format!("Total Missing Accessories: {}", missing.len()));

    pdr.set_resp(sb);
}