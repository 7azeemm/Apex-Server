use std::cmp::min;
use derive_new::new;
use common::extensions::json_ext::JsonExt;
use crate::item_utils::get_pretty_name;
use crate::prices::bazaar::BAZAAR;
use crate::structs::player_data_structs::StringBuilder;
use crate::utils::format_number_with_commas;

const ITEMS_PER_PAGE: usize = 3;
const MAX_PAGES: usize = 25;

#[derive(new)]
struct BazaarFlip {
    id: String,
    sell_price: f64,
    buy_price: f64,
    one_hour_instasells: u64,
    one_hour_instabuys: u64,
    profit: f64,
    coins_per_hour: u64
}

pub async fn get_bazaar_flips(sb: &mut StringBuilder, page: u64) {
    let mut flips = Vec::new();

    {
        let products = BAZAAR.read().await;
        for (id, product) in products.iter() {
            let one_hour_instasells = (product.sell_moving_week() / 7) / 24;
            let one_hour_instabuys = (product.buy_moving_week() / 7) / 24;
            let sell_order = product.sell_summary().first()
                .map(|v| v.get_f64("pricePerUnit").unwrap_or_default())
                .unwrap_or_else(|| product.sell_price());
            let buy_order = product.buy_summary().first()
                .map(|v| v.get_f64("pricePerUnit").unwrap_or_default())
                .unwrap_or_else(|| product.buy_price());
            let tax = ((buy_order / 100.0) * 1.25) * 2.0;
            let profit = buy_order - sell_order - tax;
            if profit <= 0f64 { continue }
            let coins_per_hour = min(one_hour_instasells, one_hour_instabuys) * profit as u64;

            flips.push(BazaarFlip::new(
                id.to_owned(), sell_order, buy_order,
                one_hour_instasells, one_hour_instabuys, profit, coins_per_hour
            ));
        }
    }

    flips.sort_by(|a, b| b.coins_per_hour.cmp(&a.coins_per_hour));

    let total_pages = min(flips.len().div_ceil(ITEMS_PER_PAGE), MAX_PAGES);
    let page_index = (page as usize).saturating_sub(1);
    let page_index = if page_index >= total_pages { total_pages.saturating_sub(1) } else { page_index };
    let start = page_index.saturating_mul(ITEMS_PER_PAGE);
    let effective_page = page_index + 1;

    sb.push(format!("Page: {effective_page}/{total_pages}"));
    sb.pushln();
    sb.push("Bazaar Flips:".to_string());

    for (i, flip) in flips.iter().skip(start).take(ITEMS_PER_PAGE).enumerate() {
        sb.push(format!("{}. {}:", i + 1, get_pretty_name(&flip.id)));
        sb.push(format!("  Buy Price: {} coins", format_number_with_commas(flip.buy_price as u64)));
        sb.push(format!("  Sell Price: {} coins", format_number_with_commas(flip.sell_price as u64)));
        sb.push(format!("  Hour Insta-Buys: {}", format_number_with_commas(flip.one_hour_instabuys)));
        sb.push(format!("  Hour Insta-Sells: {}", format_number_with_commas(flip.one_hour_instasells)));
        sb.push(format!("  Profit: {} coins", format_number_with_commas(flip.profit as u64)));
        sb.push(format!("  Coins per Hour: {} coins", format_number_with_commas(flip.coins_per_hour)));
    }
}