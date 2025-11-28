use axum::Json;
use axum::response::IntoResponse;
use serde::Deserialize;
use common::extensions::json_ext::JsonExt;
use common::player_fetcher::get_player_uuid;
use crate::helpers::accessory_helper::get_accessory_upgrades;
use crate::helpers::museum_helper::get_museum_donations;
use crate::item_utils::get_pretty_name;
use crate::prices::auctions::{get_lowest_bin, get_player_auctions, get_auction_deals};
use crate::prices::bazaar::get_price;
use crate::repos::neu::items::get_id_by_name;
use crate::repos::wiki::wiki_searcher::search_skyblock_wiki;
use crate::structs::auctions_structs::Budget;
use crate::structs::player_data_structs::{PlayerDataResponse, StringBuilder};
use crate::tools::hypixel_tools::get_player_status;
use crate::tools::skyblock_tools::{get_dungeons_info, get_skyblock_events, get_fishing_info, get_foraging_info, get_garden_info, get_inventory_contents, search_storage, get_mining_info, get_misc_info, get_profile_overview, get_profile_networth, get_slayers_info};

#[derive(Deserialize, Debug)]
pub struct ToolRequest {
    tool_name: String,
    #[serde(default)]
    args: serde_json::Value,
}

pub async fn execute_tool(Json(req): Json<ToolRequest>) -> impl IntoResponse {
    let tool_name = req.tool_name.as_str();
    let mut sb = StringBuilder::new();

    match tool_name {
        "search_skyblock_wiki" => {
            let query = req.args.get_str("query").unwrap_or_default();
            search_skyblock_wiki(&mut sb, query).await;
            return sb.get_response();
        }
        "get_item_price" => {
            let item_name = req.args.get_str("item_name").unwrap_or_default();
            get_item_price(&mut sb, item_name).await;
            return sb.get_response();
        }
        "get_auction_deals" => {
            let item_name = req.args.get_str("item_name").unwrap_or_default();
            let budget = match req.args.get_str("budget") {
                None => Budget::Medium,
                Some(str) => serde_json::from_str::<Budget>(str).unwrap_or_else(|_| Budget::Medium)
            };
            get_auction_deals(&mut sb, item_name, budget).await;
            return sb.get_response();
        }
        // "get_bazaar_flips" => {}
        // "get_item_recipe" => {}
        "get_skyblock_events" => {
            get_skyblock_events(&mut sb).await;
            return sb.get_response();
        }
        _ => {}
    }

    let player_name = match req.args.get_str("player_name") {
        None => return "Player name is required for this tool".into(),
        Some(username) => username.to_owned()
    };

    let player_uuid = match get_player_uuid(&player_name).await {
        None => return "Couldn't get find the player".into(),
        Some(player_uuid) => player_uuid
    };

    match tool_name {
        "get_player_auctions" => {
            get_player_auctions(&mut sb, &player_uuid).await;
            return sb.get_response();
        }
        _ => {}
    }

    let profile_name = req.args.get_str("profile_name")
        .filter(|n| *n != "null")
        .map(|s| s.to_owned());

    let pdr = match PlayerDataResponse::new(player_name, player_uuid, profile_name).await {
        Err(err) => return err.into(),
        Ok(pdr) => pdr
    };

    match tool_name {
        "get_player_status" => get_player_status(&pdr, &mut sb).await,
        "get_profile_overview" => get_profile_overview(&pdr, &mut sb).await,
        "get_profile_networth" => get_profile_networth(&pdr, &mut sb, false).await,
        "get_profile_section" => match req.args.get_str("category").unwrap_or_default() {
            "mining" => get_mining_info(&pdr, &mut sb).await,
            "garden" => get_garden_info(&pdr, &mut sb).await,
            "foraging" => get_foraging_info(&pdr, &mut sb).await,
            "fishing" => get_fishing_info(&pdr, &mut sb).await,
            "slayers" => get_slayers_info(&pdr, &mut sb).await,
            "dungeons" => get_dungeons_info(&pdr, &mut sb).await,
            "misc" => get_misc_info(&pdr, &mut sb).await,
            _ => sb.push("Category not found!".to_owned())
        },
        "get_inventory_contents" => get_inventory_contents(&pdr, &mut sb).await,
        "search_storage" => {
            let name = req.args.get_str("name").unwrap_or_default();
            let is_pet = req.args.get_bool("is_pet").unwrap_or(false);
            let include_prices = req.args.get_bool("include_prices").unwrap_or(false);
            search_storage(&pdr, &mut sb, name, is_pet, include_prices).await
        },
        "get_accessory_upgrades" => {
            let page = req.args.get_u64("page").unwrap_or(0);
            let soulbound = req.args.get_bool("soulbound").unwrap_or(false);
            get_accessory_upgrades(&pdr, &mut sb, page, soulbound).await
        },
        "get_museum_donations" => {
            let page = req.args.get_u64("page").unwrap_or(0);
            let soulbound = req.args.get_bool("soulbound").unwrap_or(false);
            get_museum_donations(&pdr, &mut sb, page, soulbound).await
        },
        _ => sb.push("Tool not found!".to_owned())
    }

    sb.get_response()
}

async fn get_item_price(sb: &mut StringBuilder, item_name: &str) {
    let item_ids = get_id_by_name(item_name).await;
    let item_id = match item_ids.first() {
        Some(id) => id,
        None => {
            sb.push("Couldn't find the item".to_owned());
            return;
        }
    };

    sb.push(format!("Item: {}", get_pretty_name(item_id)));

    if let Some((buy_price, sell_price)) = get_price(item_id).await {
        sb.push("Source: Bazaar".to_owned());
        sb.push(format!("Buy Price: {} coins", buy_price));
        sb.push(format!("Sell Price: {} coins", sell_price));
        return
    }

    if let Some(lowest_bin) = get_lowest_bin(item_id).await {
        sb.push("Source: Auction House".to_owned());
        sb.push(format!("LowestBIN: {} coins", lowest_bin));
        return
    }

    sb.push("Couldn't find item price".to_owned())
}