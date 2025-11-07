use crate::constants::misc::{MAGICAL_POWER, RARITIES, SPECIAL_TALISMANS};
use crate::item_utils::{get_item_rarity, get_pretty_name, get_rarity_index};
use crate::prices::auctions::get_lowest_bin;
use crate::repos::neu::items::ACCESSORIES;
use crate::repos::neu::talisman_upgrades::{get_talisman_upgrades, IGNORED_TALISMANS};
use crate::structs::player_data_structs::{PlayerDataResponse, StringBuilder};
use crate::utils::format_number;
use std::collections::{HashMap, HashSet};

const ITEMS_PER_PAGE: usize = 10;

#[derive(Clone, Debug)]
struct Talisman {
    id: String,
    rarity: String,
    recombed: bool,
    downgrade: Option<Box<Talisman>>,
    mp: Option<u64>,
    price: Option<u64>,
    coins_per_mp: Option<u64>
}

impl Talisman {
    fn new(id: &str, rarity: &str, recombed: bool) -> Self {
        Self {
            id: id.to_owned(), rarity: rarity.to_owned(), recombed, downgrade: None, mp: None, price: None, coins_per_mp: None
        }
    }

    fn set_downgrade(&mut self, downgrade: Talisman) {
        self.downgrade = Some(Box::new(downgrade))
    }
    fn set_mp(&mut self, mp: u64) {
        self.mp = Some(mp)
    }
    fn set_price(&mut self, price: u64) {
        self.price = Some(price)
    }
    fn set_coins_per_mp(&mut self, coins_per_mp: u64) {
        self.coins_per_mp = Some(coins_per_mp)
    }
}

pub async fn get_missing_accessories(pdr: &mut PlayerDataResponse, page: u64, soulbound: bool) {
    let mut sb = StringBuilder::new();
    let player_talismans: HashMap<String, Talisman> = pdr.profile().storage().accessories()
        .iter()
        .map(|i| {
            let id = i.item_id().to_owned();
            let rarity = get_item_rarity(i.nbt()).unwrap_or_default();
            let is_recombed = i.nbt().get_extra_map()
                .map(|m| m.contains_key("rarity_upgrades"))
                .unwrap_or_default();
            (id.clone(), Talisman::new(&id, &rarity, is_recombed))
        })
        .collect();

    let mut talismans = HashMap::new();

    {
        let mut skip = HashSet::new();
        let all_accessories = ACCESSORIES.read().await;
        for (id, rarity) in all_accessories.iter() {
            // If player has the talisman, or it's already processed continue
            if skip.contains(id) { continue; }
            if player_talismans.contains_key(id) {
                skip.insert(id.to_owned());
                continue;
            }

            // Get talisman upgrade line or insert to missing list and continue if unavailable
            let mut upgrade_line = match get_talisman_upgrades(id).await {
                Some(upgrade_line) => upgrade_line,
                None => {
                    talismans.insert(id.to_owned(), Talisman::new(id, rarity, false));
                    continue;
                }
            };

            // Try to find the best owned talisman upgrade if available
            let mut owned_talisman = None;
            for (i, upgrade) in upgrade_line.iter().enumerate() {
                skip.insert(upgrade.to_owned());
                if upgrade != id && let Some(talisman) = player_talismans.get(upgrade) {
                    owned_talisman = Some((i, talisman.clone()))
                }
            }

            // Slice the upgrade_line to only include upgrades after the owned one
            if let Some((index, talisman)) = owned_talisman {
                upgrade_line = upgrade_line[index..].to_vec();
                owned_talisman = Some((0, talisman));
            }

            // Add missing talismans
            let mut last_rarity_index: i64 = -1;
            let mut last_added = None;
            for (i, upgrade) in upgrade_line.iter().enumerate() {
                let Some(rarity) = all_accessories.get(upgrade) else { continue };
                let Some(rarity_index) = get_rarity_index(rarity).map(|i| i as i64) else { continue };

                let replace_last = rarity_index == last_rarity_index && last_added.is_some();
                if replace_last || rarity_index > last_rarity_index {
                    if replace_last {
                        talismans.remove(last_added.unwrap());
                    }

                    match owned_talisman.as_ref() {
                        None => { talismans.insert(upgrade.to_owned(), Talisman::new(upgrade, rarity, false)); }
                        Some((index, downgrade)) => {
                            if i > *index {
                                let mut talisman = Talisman::new(upgrade, rarity, false);
                                talisman.set_downgrade(downgrade.clone());
                                talismans.insert(upgrade.to_owned(), talisman);
                            }
                        }
                    }
                }
                last_rarity_index = rarity_index;
                last_added = Some(upgrade);
            }
        }
    }

    // Removes ignored talismans (rift talismans...)
    for ignored in IGNORED_TALISMANS.read().await.iter() {
        talismans.remove(ignored);
    }

    // Removes duplicated hats
    let player_has_hat = player_talismans.iter().any(|(id, _)| id.starts_with("PARTY_HAT_") || id.starts_with("BALLOON_HAT_"));
    let mut kept_one_hat = false;
    let mut to_remove = Vec::new();
    let mut to_add = Vec::new();

    for talisman in talismans.keys() {
        let piggy_banks = vec!["PIGGY_BANK", "CRACKED_PIGGY_BANK", "BROKEN_PIGGY_BANK"];
        if piggy_banks.contains(&talisman.as_str()) {
            to_remove.push(talisman.to_owned());
            let player_has_piggy = piggy_banks.iter().any(|piggy| player_talismans.contains_key(*piggy));
            if !player_has_piggy {
                to_add.push(Talisman::new("PIGGY_BANK", "UNCOMMON", false));
            }
        }

        if talisman.starts_with("PARTY_HAT_") || talisman.starts_with("BALLOON_HAT_") {
            if !player_has_hat && !kept_one_hat {
                kept_one_hat = true;
                continue;
            }
            to_remove.push(talisman.to_owned());
        }
    }

    for talisman in to_remove {
        talismans.remove(&talisman);
    }

    for talisman in to_add {
        talismans.insert(talisman.id.clone(), talisman);
    }

    // Process Special Talismans (Book Of Progression, Pandora's box...)
    for (id, rarities) in SPECIAL_TALISMANS {
        let talisman = match player_talismans.get(*id) {
            Some(t) => t,
            None => {
                talismans.remove(*id);
                for rarity in *rarities {
                    let custom_id = format!("{rarity}_{id}");
                    let talisman = Talisman::new(&custom_id, rarity, false);
                    talismans.insert(custom_id, talisman);
                }
                continue
            }
        };

        let rarity_index = rarities.iter().position(|r| r == &talisman.rarity).unwrap_or_default();
        let downgrade_custom_id = format!("{}_{}", talisman.rarity, talisman.id);
        let downgrade = Talisman::new(&downgrade_custom_id, &talisman.rarity, talisman.recombed);

        for (i, talis_rarity) in rarities.iter().enumerate() {
            if (talisman.recombed && i > rarity_index - 1) || i > rarity_index {
                let custom_id = format!("{talis_rarity}_{id}");
                let mut talisman = Talisman::new(&custom_id, talis_rarity, false);
                talisman.set_downgrade(downgrade.clone());
                talismans.insert(custom_id, talisman);
            }
        }
    }

    let mut valuable_talismans = Vec::new();
    let mut no_price_talismans = Vec::new();

    // Set magical power, prices and coins per mp
    for (id, mut talisman) in talismans {
        let mut downgrade_mp = 0;
        let mut downgrade_price = 0;

        let talisman_mp = get_magical_power(&talisman, false);
        talisman.set_mp(talisman_mp);

        if let Some(downgrade) = talisman.downgrade.as_mut() {
            let mp = get_magical_power(downgrade, false);
            downgrade.set_mp(mp);
            downgrade_mp = mp;
        }

        let Some(talisman_price) = get_accessory_price(&id).await else {
            no_price_talismans.push(talisman);
            continue
        };
        talisman.set_price(talisman_price);

        if let Some(downgrade) = talisman.downgrade.as_mut() {
            if let Some(price) = get_accessory_price(&downgrade.id).await {
                downgrade.set_price(price);
                downgrade_price = price;
            }
        }

        let eff_mp = talisman_mp - downgrade_mp;
        let eff_price = talisman_price - downgrade_price;

        match eff_price > 0 && eff_mp > 0 {
            true => talisman.set_coins_per_mp((eff_price as f64 / eff_mp as f64) as u64),
            false => {
                no_price_talismans.push(talisman);
                continue
            }
        }

        valuable_talismans.push(talisman);
    }

    // Sorting lists
    no_price_talismans.sort_by(|a, b| b.mp.unwrap_or(0).cmp(&a.mp.unwrap_or(0)));
    valuable_talismans.sort_by(|a, b| {
        match (a.coins_per_mp, b.coins_per_mp) {
            (Some(a_val), Some(b_val)) => a_val.partial_cmp(&b_val).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal
        }
    });

    let list = if soulbound { &no_price_talismans } else { &valuable_talismans };
    let total_pages = (list.len() + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE;
    let page_index = (page as usize).saturating_sub(1);
    let page_index = if page_index >= total_pages { total_pages.saturating_sub(1) } else { page_index };
    let start = page_index.saturating_mul(ITEMS_PER_PAGE);
    let effective_page = page_index + 1;

    sb.push(format!("Page: {effective_page}/{total_pages}"));
    sb.push(format!("Total Missing accessories/upgrades: {}", valuable_talismans.len() + no_price_talismans.len()));
    sb.push(format!("Missing Soulbound accessories: {}", no_price_talismans.len()));
    sb.pushln();
    sb.push("Accessories:".to_string());

    for (i, talisman) in list.iter().skip(start).take(ITEMS_PER_PAGE).enumerate() {
        let name = get_pretty_name(&talisman.id);
        let rarity = get_pretty_name(&talisman.rarity);
        match soulbound {
            true => sb.push(format!("{}. {} ({})", i + 1, name, rarity)),
            false => {
                let price = talisman.price.unwrap_or(0);
                sb.push(format!("{}. {} ({}, {} coins)", i + 1, name, rarity, format_number(price)));
                if let Some(downgrade) = &talisman.downgrade {
                    if let Some(downgrade_price) = &downgrade.price {
                        let name = get_pretty_name(&downgrade.id);
                        let rarity = get_pretty_name(&downgrade.rarity);
                        sb.push(format!("   Upgrades From: {} ({}, {} coins)", name, rarity, format_number(*downgrade_price)));
                        sb.push(format!("   Net Cost: {} coins", format_number(price.saturating_sub(*downgrade_price))));
                    }
                }
            }
        }
    }

    if !soulbound {
        sb.pushln();
        sb.push("Notes:".to_string());
        sb.push("- Prices represent the current Lowest BIN value or NPC sell price.".to_string());
        sb.push("- Net Cost = Accessory Price - Previous Tier Price.".to_string());
        sb.push("- Some accessories may have multiple upgrade tiers not shown on this page.".to_string());
    }

    pdr.set_sb(sb);
}

fn get_magical_power(talisman: &Talisman, with_recomb: bool) -> u64 {
    let rarity = match with_recomb {
        false => talisman.rarity.clone(),
        true => {
            let rarity_index = get_rarity_index(&talisman.rarity).unwrap_or_default();
            match RARITIES.get(rarity_index + 1) {
                Some(next_rarity) => next_rarity.to_string(),
                None => talisman.rarity.clone()
            }
        }
    };

    let mp = MAGICAL_POWER.get(&rarity).map(|n| *n).unwrap_or(1);

    match talisman.id.as_str() {
        "HEGEMONY_ARTIFACT" => mp * 2,
        "RIFT_PRISM" => mp + 3,
        _ => mp
    }
}

async fn get_accessory_price(id: &str) -> Option<u64> {
    if id.contains("_TRAPPER_CREST") {
        return get_lowest_bin("TRAPPER_CREST").await;
    }

    if id.contains("_PULSE_RING") {
        let ring_price = get_lowest_bin("PULSE_RING").await?;
        let rarity = id.split('_').next().unwrap_or("UNCOMMON");
        let thunder_in_a_bottle_price = get_lowest_bin("THUNDER_IN_A_BOTTLE").await.unwrap_or_default();
        let bottles_needed = match rarity {
            "UNCOMMON" => 0,
            "RARE" => 3,
            "EPIC" => 20,
            "LEGENDARY" => 100,
            _ => 0
        };
        return Some(ring_price + (bottles_needed * thunder_in_a_bottle_price))
    }

    get_lowest_bin(id).await
}