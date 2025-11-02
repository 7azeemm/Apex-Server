use std::collections::{HashMap, HashSet};
use crate::item_utils::get_pretty_name;
use crate::tools::profile_fetcher::get_museum_items;
use crate::prices::auctions::get_lowest_bin;
use crate::repos::neu::museum_donations::{Donation, DONATIONS, SET_EXCEPTIONS, UPGRADES};
use crate::structs::player_data_structs::{PlayerDataResponse, StringBuilder};
use crate::utils::format_number;

const ITEMS_PER_PAGE: usize = 15;

#[derive(Clone, Debug)]
struct DonationItem {
    donation: Donation,
    price: Option<u64>,
    coins_per_xp: Option<f64>,
}

pub async fn get_missing_museum_donations(pdr: &mut PlayerDataResponse, page: u64, soulbound: bool) {
    let mut sb = StringBuilder::new();
    let player_uuid = pdr.player_uuid().to_string();

    // Collect donated items
    let mut donated_items = HashSet::new();
    if let Some(museum_donations) = get_museum_items(&player_uuid, pdr.profile_mut()).await {
        for donation in museum_donations.iter() {
            donated_items.insert(donation.id().to_owned());
        }
    }

    // Collect upgrade lines (to clear donated items from things weren't in the API)
    let mut item_to_line: HashMap<String, Vec<String>> = HashMap::new();
    for line in UPGRADES.read().await.iter() {
        for item in line {
            item_to_line.insert(item.to_owned(), line.clone());
        }
    }

    let mut missing_with_prices = Vec::new();
    let mut no_price_donations = Vec::new();

    for (id, donation) in DONATIONS.read().await.iter() {
        if donated_items.contains(id) { continue }

        if let Some(line) = item_to_line.get(id) {
            let pos = line.iter().position(|x| x == id).unwrap();
            let has_better = (0..pos).any(|i| donated_items.contains(&line[i]));
            if has_better { continue }
        }

        // Handle sets or single items
        let mut price = None;
        match donation.set() {
            None => price = get_lowest_bin(id).await,
            Some(set) => {
                let mut total = 0;
                let mut has_no_price = false;

                for piece_id in set {
                    match get_lowest_bin(piece_id).await {
                        Some(p) => total += p,
                        None => {
                            has_no_price = true;
                            break;
                        }
                    }
                }

                if !has_no_price {
                    price = Some(total);
                }
            }
        }

        let coins_per_xp = price.map(|p| p as f64 / *donation.xp() as f64);
        let item = DonationItem {
            donation: donation.clone(),
            price,
            coins_per_xp,
        };

        match price.is_some() {
            true => missing_with_prices.push(item),
            false => no_price_donations.push(item)
        }
    }

    // Sort by coins per XP
    missing_with_prices.sort_by(|a, b| {
        a.coins_per_xp.unwrap().partial_cmp(&b.coins_per_xp.unwrap()).unwrap()
    });

    // Sort no_price_donations by XP descending
    no_price_donations.sort_by(|a, b| b.donation.xp().cmp(a.donation.xp()));

    let list = if soulbound { &no_price_donations } else { &missing_with_prices };
    let total_pages = (list.len() + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE;
    let page_index = (page as usize).saturating_sub(1);
    let page_index = if page_index >= total_pages { total_pages.saturating_sub(1) } else { page_index };
    let start = page_index * ITEMS_PER_PAGE;
    let effective_page = page_index + 1;

    sb.push(format!("Page: {}/{}", effective_page, total_pages));
    sb.push(format!("Total Missing Donations: {}", missing_with_prices.len() + no_price_donations.len()));
    sb.push(format!("Missing Soulbound Donations: {}", no_price_donations.len()));
    sb.pushln();
    sb.push("Donations:".to_string());

    for (i, item) in list.iter().skip(start).take(ITEMS_PER_PAGE).enumerate() {
        let xp = item.donation.xp();
        let mut id = item.donation.id().to_owned();
        if let Some(set_id) = SET_EXCEPTIONS.read().await.get(&id) {
            id = set_id.to_owned();
        }

        let name = match item.donation.is_set() {
            true => format!("{} set", get_pretty_name(&id)),
            false => get_pretty_name(&id)
        };

        match soulbound {
            true => sb.push(format!("{}. {} ({} XP)", i + 1, name, xp)),
            false => {
                let price = item.price.unwrap_or_default();
                sb.push(format!("{}. {} ({} coins, {} XP)", i + 1, name, format_number(price), xp));
            }
        }
    }

    if !soulbound {
        sb.pushln();
        sb.push("Notes:".to_string());
        sb.push("- Some donations may be sets with multiple pieces.".to_string());
        sb.push("- Prices represent the current Lowest BIN value for single items or the sum of Lowest BIN values for set pieces.".to_string());
        sb.push("- Donations are sorted by lowest coins per XP (most efficient first).".to_string());
    }

    pdr.set_resp(sb);
}