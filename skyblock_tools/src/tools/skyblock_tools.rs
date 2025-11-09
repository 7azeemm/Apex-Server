use crate::constants::garden::{
    CANE_CACTUS_MILESTONE_XP, CARROT_POTATO_MILESTONE_XP, COCOA_WART_MILESTONE_XP, CROP_NAMES,
    GARDEN_LEVELS_XP, MAX_COMPOSTER_UPGRADE_LEVEL, MAX_CROP_MILESTONE, MAX_CROP_UPGRADE_LEVEL,
    MAX_GARDEN_LEVEL, MAX_PLOTS, MELON_MILESTONE_XP, WHEAT_PUMPKIN_MUSHROOM_MILESTONE_XP,
};
use crate::constants::misc::{
    MAX_BESTIARY_LEVEL, MAX_ENIGMA_SOULS, MAX_FAIRY_SOULS, MAX_MINING_COMMISSION_MILESTONE,
    MAX_TIMECHARMS, SLAYER_XP_REQUIRED, TROPHY_FISHING_TIERS,
};
use crate::constants::setups::SetupType;
use crate::constants::skills::{
    DUNGEONEERING_SKILL_XP, RUNECRAFTING_SKILL_XP, SKILLS_XP, SKILL_MAX_LEVELS, SOCIAL_SKILL_XP,
};
use crate::item_utils::{get_pet_info, get_pet_obj, get_pretty_name};
use crate::live_data::jacob_contests::get_upcoming_contests;
use crate::live_data::mayor_info::{
    get_election_over_time_left, get_mayor_info, get_skyblock_date, get_special_mayors_info,
};
use crate::prices::bazaar::get_buy_price;
use crate::prices::item_value_calculator::{calculate_item_value, get_pet_value};
use crate::repos::neu::items::{find_best_matches, get_item_display_name};
use crate::structs::item_structs::ItemValue;
use crate::structs::player_data_structs::{Item, Pet, PlayerDataResponse, StringBuilder};
use crate::tools::profile_fetcher::{get_garden_data, get_museum_items};
use crate::utils::{format_number, format_number_with_commas, get_time_as_secs};
use common::extensions::json_ext::JsonExt;
use serde_json::Value;
use std::cmp::max;
use std::collections::HashMap;

pub async fn get_player_overview(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let data = pdr.profile_data();

    sb.push(format!("GameMode: {} Profile", get_pretty_name(pdr.profile().game_mode())));

    // SkyBlock Level
    if let Some(xp) = data.get_u64("leveling/experience") {
        let level = xp / 100;
        let progress = xp % 100;
        sb.push(format!("SkyBlock Level {} ({}/100 XP)", level, progress));
    }

    // Purse + Bank
    sb.push(format!("Purse: {} coins", format_number(pdr.profile().purse())));
    sb.push(match pdr.profile().bank() {
        None => "Bank: unavailable".to_owned(),
        Some(bank) => format!("Bank: {} coins", format_number(bank)),
    });

    // Active Pet
    if let Some(pets) = data.get_array("pets_data/pets") {
        if let Some(active_pet) = pets.iter().find(|pet| pet.get_bool("active").unwrap_or(false)) {
            if let Some(pet_info) = get_pet_obj(active_pet).and_then(|v| get_pet_info(&v)) {
                sb.push(format!("Active Pet: {pet_info}"))
            }
        }
    }

    // Accessories Info
    if let Some(acc_storage) = data.get("accessory_bag_storage") {
        sb.push("Accessories:".to_owned());

        let selected_power = acc_storage
            .get_str("selected_power")
            .map(get_pretty_name)
            .unwrap_or("N/A".to_owned());
        let mp = acc_storage.get_u64("highest_magical_power").unwrap_or(0);

        sb.push(format!("- Selected Power Stone: {selected_power}"));
        sb.push(format!("- Magical Power: {mp}"));
    }

    // Skills
    if let Some(skills) = data.get_object("tools/experience") && !skills.is_empty() {
        sb.push("Skills:".to_owned());
        let mut total_level = 0;
        let mut count = 0;

        for (skill, xp) in skills {
            if skill == "SKILL_DUNGEONEERING" { continue; }
            let (skill_level, skill_info) = get_skill_level(data, skill, xp.as_f64().map(|xp| xp as u64));

            // Exclude cosmetic skills from average
            if skill != "SKILL_RUNECRAFTING" && skill != "SKILL_SOCIAL" {
                total_level += skill_level;
                count += 1;
            }

            sb.push(format!("- {}: {skill_info}", get_pretty_name(&skill.replace("SKILL_", ""))));
        }

        if count > 0 {
            let avg = total_level as f64 / count as f64;
            sb.push(format!("Average Skill Level: {:.2}", avg));
        }
    } else {
        sb.push("Skills: unavailable".to_owned())
    }

    pdr.set_sb(sb);
}

pub async fn get_mining_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

    let (_, mining_skill) = get_skill_level(profile_data, "SKILL_MINING", None);
    sb.push(format!("Mining Skill: {mining_skill}"));

    pdr.profile().add_setup_info(SetupType::Mining, &mut sb);
    sb.pushln();

    if let Some(mining_core) = profile_data.get("mining_core") {
        let powders = [
            ("mithril", "Mithril Powder"),
            ("gemstone", "Gemstone Powder"),
            ("glacite", "Glacite Powder"),
        ];

        sb.push("Powders:".to_owned());
        for (key, label) in powders {
            let spent_amount = mining_core.get_u64(&format!("powder_spent_{key}")).unwrap_or(0);
            let unspent_amount = mining_core.get_u64(&format!("powder_{key}")).unwrap_or(0);
            sb.push(format!("- {label}: {}", format_number(spent_amount + unspent_amount)));
        }

        if let Some(nucleus_runs) = mining_core.get_u64("crystals/jade_crystal/total_placed") {
            sb.push(format!("Crystal Nucleus Runs: {nucleus_runs}"));
        }
    }

    if let Some(tutorial) = profile_data.get_array("objectives/tutorial") {
        for lvl in (1..=6).rev() {
            let quest_id = format!("commission_milestone_reward_skyblock_xp_tier_{lvl}");
            if tutorial.iter().any(|v| v.as_str() == Some(&quest_id)) {
                sb.push(format!("Commission Milestone: {lvl}/{MAX_MINING_COMMISSION_MILESTONE}"));
                break;
            }
        }
    }

    if let Some(glacite_core) = profile_data.get("glacite_player_data") {
        if let Some(mineshafts_entered) = glacite_core.get_u64("mineshafts_entered") {
            sb.push(format!("Mineshafts entered: {mineshafts_entered}"));
        }
        if let Some(corpses_looted) = glacite_core.get_object("corpses_looted") {
            sb.push("Corpses looted:".to_owned());
            for (corpse, count) in corpses_looted {
                sb.push(format!("- {}: {count}", get_pretty_name(corpse)));
            }
        }
    }

    pdr.set_sb(sb);
}

pub async fn get_garden_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();

    let (_, farming_skill) = get_skill_level(pdr.profile_data(), "SKILL_FARMING", None);
    sb.push(format!("Farming Skill: {farming_skill}"));

    pdr.profile().add_setup_info(SetupType::Farming, &mut sb);
    sb.pushln();

    if let Some(garden) = get_garden_data(pdr.profile_mut()).await {
        if let Some(xp) = garden.get_f64("garden_experience") {
            let (_, level_info) = get_level_and_progress(GARDEN_LEVELS_XP, xp as u64, 1, MAX_GARDEN_LEVEL, None);
            sb.push(format!("Garden Level: {level_info}"));
        }

        let commission_data = garden.get("commission_data");
        let visitors_served = commission_data.get_u64("total_completed").unwrap_or(0);
        let unique_visitors = commission_data.get_u64("unique_npcs_served").unwrap_or(0);
        let plots = garden.get_array("unlocked_plots_ids").map(|v| v.len()).unwrap_or(0);

        sb.push(format!("Visitors served: {visitors_served}"));
        sb.push(format!("Unique Visitors served: {unique_visitors}"));
        sb.push(format!("Unlocked Plots: {plots}/{MAX_PLOTS}"));

        if let Some(composter_upgrades) = garden.get_object("composter_data/upgrades") {
            sb.push("Composter upgrades:".to_owned());
            for (upgrade, level) in composter_upgrades {
                sb.push(format!("- {}: {level}/{MAX_COMPOSTER_UPGRADE_LEVEL}", get_pretty_name(upgrade)));
            }
        }

        if let Some(crop_milestones) = garden.get_object("resources_collected") {
            let crop_upgrades = garden.get_object("crop_upgrade_levels");

            sb.push("Crop Milestones/Upgrades:".to_owned());
            for (crop, crop_xp) in crop_milestones {
                let crop_xp = crop_xp.as_u64().unwrap_or(0);
                let crop = crop.as_str();

                let xp_table = match crop {
                    "WHEAT" | "PUMPKIN" | "MUSHROOM_COLLECTION" => WHEAT_PUMPKIN_MUSHROOM_MILESTONE_XP,
                    "CARROT_ITEM" | "POTATO_ITEM" => CARROT_POTATO_MILESTONE_XP,
                    "SUGAR_CANE" | "CACTUS" => CANE_CACTUS_MILESTONE_XP,
                    "MELON" => MELON_MILESTONE_XP,
                    "INK_SACK:3" | "NETHER_STALK" => COCOA_WART_MILESTONE_XP,
                    _ => continue,
                };

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

                let mut line = format!("- {}: {level}/{MAX_CROP_MILESTONE}", CROP_NAMES.get(crop).unwrap_or(&&*"N/A".to_owned()));
                if let Some(progress) = progress {
                    line.push_str(&format!(" ({progress}%)"))
                }

                if let Some(crop_upgrade_level) = crop_upgrades.and_then(|c| c.get(crop)) {
                    let level = crop_upgrade_level.as_u64().unwrap_or(0);
                    line.push_str(&format!(" | Upgrade: {level}/{MAX_CROP_UPGRADE_LEVEL}"))
                }
                sb.push(line);
            }
        }
    }

    if let Some(medals_inv) = pdr.profile_data().get("jacobs_contest/medals_inv") {
        sb.push("Jacob Medals:".to_owned());
        for bracket in ["bronze", "silver", "gold", "platinum", "diamond"] {
            let amount = medals_inv.get_u64(bracket).unwrap_or(0);
            sb.push(format!("- {}: {}", get_pretty_name(bracket), amount))
        }
    }

    pdr.set_sb(sb);
}

pub async fn get_foraging_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();

    let (_, foraging_skill) = get_skill_level(pdr.profile_data(), "SKILL_FORAGING", None);
    sb.push(format!("Foraging Skill: {foraging_skill}"));

    pdr.profile().add_setup_info(SetupType::Foraging, &mut sb);

    pdr.set_sb(sb);
}

pub async fn get_fishing_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

    let (_, fishing_skill) = get_skill_level(profile_data, "SKILL_FISHING", None);
    sb.push(format!("Fishing Skill: {fishing_skill}"));

    let trophy_fishing_tier = profile_data
        .get_array("trophy_fish/rewards")
        .map(|a| a.len())
        .unwrap_or(0);
    sb.push(format!("Trophy Fishing Tier: {}", TROPHY_FISHING_TIERS.get(trophy_fishing_tier).unwrap_or(&"None")));

    sb.pushln();
    pdr.profile().add_setup_info(SetupType::Fishing, &mut sb);

    pdr.set_sb(sb);
}

pub async fn get_slayer_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();

    if let Some(slayer_bosses) = pdr.profile_data().get_object("slayer/slayer_bosses") {
        for (slayer, data) in slayer_bosses {
            let xp = data.get_u64("xp").unwrap_or_default();

            if let Some(xp_table) = SLAYER_XP_REQUIRED.get(slayer) {
                let slayer_name = get_pretty_name(slayer);
                let max_level = if slayer == "vampire" { 5 } else { 9 };
                let (_, level_info) = get_level_and_progress(xp_table, xp, 0, max_level, None);
                sb.push(format!("{slayer_name} level: {level_info}"));
            }
        }
    } else {
        sb.push("Slayers: unavailable".to_owned())
    }

    pdr.set_sb(sb);
}

pub async fn get_dungeons_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

    if let Some(dungeons) = profile_data.get("dungeons") {
        if let Some(dungeon_types) = dungeons.get("dungeon_types") {
            if let Some(catacombs) = dungeon_types.get("catacombs") {
                let xp = catacombs.get_f64("experience").unwrap_or_default() as u64;
                let (_, level_str) = get_skill_level(profile_data, "SKILL_DUNGEONEERING", Some(xp));
                sb.push(format!("Dungeons Level: {level_str}"));

                if let Some(classes) = dungeons.get_object("player_classes") {
                    sb.push("Dungeon Classes:".to_owned());
                    for (class, xp) in classes {
                        let xp = xp.get_f64("experience").unwrap_or_default() as u64;
                        let (_, level_str) = get_skill_level(profile_data, "SKILL_DUNGEONEERING", Some(xp));
                        sb.push(format!("- {}: {level_str}", get_pretty_name(class)));
                    }
                }

                if let Some(selected_class) = dungeons.get_str("selected_dungeon_class") {
                    let selected_class = get_pretty_name(selected_class);
                    sb.push(format!("Selected Class: {selected_class}"));
                    sb.pushln();

                    if let Some(setup_type) = SetupType::from_str(&selected_class) {
                        sb.push(format!("{selected_class} Setup:"));
                        pdr.profile().add_setup_info(setup_type, &mut sb);
                        sb.pushln();
                    }
                }

                if let Some(secrets) = dungeons.get_u64("secrets") {
                    sb.push(format!("Secrets: {secrets}"));
                }

                fn extract_floor_completions(milestone_completions: Option<&serde_json::Map<String, Value>>) -> Vec<(u32, u64)> {
                    milestone_completions
                        .map(|mc| {
                            let mut floors: Vec<(u32, u64)> = mc
                                .iter()
                                .filter(|(floor, _)| *floor != "total" && *floor != "0")
                                .filter_map(|(floor, completions)| {
                                    let completions = completions.as_f64().unwrap_or(0.0) as u64;
                                    floor.parse::<u32>().ok().map(|floor_num| (floor_num, completions))
                                })
                                .collect();
                            floors.sort_by_key(|(floor_num, _)| *floor_num);
                            floors
                        }).unwrap_or_default()
                }

                let catacombs_floors = extract_floor_completions(catacombs.get_object("milestone_completions"));
                let master_floors = dungeon_types.get("master_catacombs").and_then(|mc| {
                    Some(extract_floor_completions(mc.get_object("milestone_completions")))
                });

                let mut total = 0;
                sb.push("Runs count:".to_owned());
                if let Some(master_floors) = master_floors {
                    sb.push("- Note: MM1 = Master Mode 1".to_owned());
                    let master_map: HashMap<u32, u64> = master_floors.into_iter().collect();
                    for (floor_num, completions) in catacombs_floors {
                        let mm_completions = master_map.get(&floor_num).copied().unwrap_or(0);
                        sb.push(format!("- F{floor_num}: {completions} | MM{floor_num}: {mm_completions}"));
                        total += completions + mm_completions;
                    }
                } else {
                    for (floor_num, completions) in catacombs_floors {
                        sb.push(format!("- F{floor_num}: {completions}"));
                        total += completions;
                    }
                }

                if total > 0 {
                    sb.push(format!("- Total: {total}"))
                }
            }
        }
    }

    pdr.set_sb(sb);
}

pub async fn get_events_info(sb: &mut StringBuilder) {
    sb.push(format!("SkyBlock Date: {}", get_skyblock_date()));
    sb.pushln();

    let mayor_info = get_mayor_info().await;
    let mayor = mayor_info.mayor();
    let mayor_perks = mayor.perks().keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
    sb.push(format!("Current Mayor: {} (perks: [{}])", mayor.name(), mayor_perks));

    if let Some(minister) = mayor_info.minister() {
        let minister_perk = minister.perks().keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        sb.push(format!("Current Minister: {} (perk: {})", minister.name(), minister_perk));
    }

    sb.pushln();

    if let Some(election) = mayor_info.election() {
        sb.push("Current Open Election:".to_owned());
        let total_votes: u64 = election.iter().filter_map(|(_, votes)| *votes).sum();

        if total_votes == 0 {
            sb.push(" (Votes are hidden)".to_owned());
        }

        for (mayor, votes) in election.iter() {
            let mut mayor_str = format!("- {}", mayor.name());
            if let Some(votes) = votes {
                let percentage = match total_votes {
                    0 => 0,
                    _ => ((*votes as f64 / total_votes as f64) * 100.0).round() as u64,
                };
                mayor_str.push_str(&format!(" ({}% votes)", percentage));
            }
            sb.push(mayor_str)
        }

        sb.push(get_election_over_time_left());
        sb.pushln();
    }

    sb.push("Special Mayors:".to_owned());
    sb.push(get_special_mayors_info());
    sb.pushln();

    let upcoming_contests = get_upcoming_contests().await;
    if !upcoming_contests.is_empty() {
        sb.push("Jacob's Contests:".to_owned());
        for (time, crops) in upcoming_contests.iter() {
            if let Ok(timestamp) = time.parse::<u64>() {
                let current_time = get_time_as_secs();
                let time_diff = timestamp.saturating_sub(current_time);
                let total_minutes = time_diff / 60;

                // Format crops joined by "/"
                let crops_str = crops.join(", ");

                // Convert timestamp to HH:MM:SS format
                let hours = (timestamp % 86400) / 3600;
                let minutes = (timestamp % 3600) / 60;
                let seconds = timestamp % 60;
                let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

                sb.push(format!("- [{}] after {}mins (at {})", crops_str, total_minutes, formatted_time));
            }
        }
    }
}

pub async fn get_inventory(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let storage = pdr.profile().storage();

    let armor = storage.armor();
    match armor.is_empty() {
        true => sb.push("Armor: unavailable".to_owned()),
        false => {
            sb.push("Armor:".to_owned());
            for piece in armor {
                sb.push(format!("- {}", piece.name()));
            }
        }
    }

    let equipment = storage.equipment();
    match equipment.is_empty() {
        true => sb.push("Equipment: unavailable".to_owned()),
        false => {
            sb.push("Equipment:".to_owned());
            for piece in equipment {
                sb.push(format!("- {}", piece.name()));
            }
        }
    }

    sb.pushln();

    let inventory = storage.inventory();
    match inventory.is_empty() {
        true => sb.push("Inventory: unavailable".to_owned()),
        false => {
            sb.push("Inventory:".to_owned());
            let mut items: Vec<(&str, u64)> = Vec::new();

            for item in inventory {
                let &count = item.count();
                let item_name = item.name();
                match items.iter_mut().find(|(name, _)| *name == item_name) {
                    Some((_, c)) => *c += count,
                    None => items.push((item_name, count)),
                }
            }

            for (item, count) in items {
                sb.push(match count > 1 {
                    true => format!("- {count}x {item}"),
                    false => format!("- {item}"),
                })
            }
        }
    }

    pdr.set_sb(sb);
}

pub async fn get_misc_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

    if let Some(bestiary_level) = profile_data.get_u64("bestiary/milestone/last_claimed_milestone") {
        let max_be_level = max(bestiary_level, MAX_BESTIARY_LEVEL as u64);
        sb.push(format!("Bestiary Level: {bestiary_level}/{max_be_level}"));
    }

    if let Some(pet_score) = profile_data.get_u64("leveling/highest_pet_score") {
        sb.push(format!("Pet Score: {pet_score}"));
    }

    if let Some(fairy_souls) = profile_data.get_u64("fairy_soul/total_collected") {
        let max_fairy_souls = max(fairy_souls, MAX_FAIRY_SOULS as u64);
        sb.push(format!("Fairy Souls: {fairy_souls}/{max_fairy_souls}"))
    }

    if let Some(powers_list) = profile_data.get_array("accessory_bag_storage/unlocked_powers") {
        let values = powers_list.iter().filter_map(|v| v.as_str()).map(get_pretty_name).collect::<Vec<_>>().join(", ");
        sb.push(format!("Unlocked Powers: [{values}]"));
    }

    sb.pushln();

    if let Some(rift) = profile_data.get("rift") {
        sb.push("Rift:".to_owned());
        if let Some(timecharms) = rift.get_array("gallery/secured_trophies") {
            sb.push(format!("- Timecharms: {}/{MAX_TIMECHARMS}", timecharms.len()));
        }

        if let Some(enigma_souls) = rift.get_array("enigma/found_souls") {
            sb.push(format!("- Enigma souls: {}/{MAX_ENIGMA_SOULS}", enigma_souls.len()));
        }

        if let Some(motes) = profile_data.get_f64("currencies/motes_purse") {
            sb.push(format!("- Motes: {}", motes as u64));
        }
        sb.pushln();
    }

    if let Some(crimson_data) = profile_data.get("nether_island_player_data") {
        if let Some(selected_faction) = crimson_data.get_str("selected_faction") {
            sb.push(format!("Selected Faction: {}", get_pretty_name(selected_faction)));
        }

        if let Some(mages_rep) = crimson_data.get_f64("mages_reputation") {
            sb.push(format!("Mages reputation: {}", mages_rep as u64));
        }

        if let Some(barb_rep) = crimson_data.get_f64("barbarians_reputation") {
            sb.push(format!("Barbarians reputation: {}", barb_rep as u64));
        }

        if let Some(kuudra) = crimson_data.get("kuudra_completed_tiers") {
            sb.push("Kuudra:".to_string());
            for tier in ["none", "hot", "burning", "fiery", "infernal"] {
                let comp = kuudra.get_u64(tier).unwrap_or(0);
                let tier_name = if tier == "none" { "Basic".to_string() } else { get_pretty_name(tier) };
                sb.push(format!("- {tier_name}: {comp} runs"))
            }
        }

        sb.pushln();
    }

    match profile_data.get_object("currencies/essence") {
        None => sb.push("Essences: unavailable".to_owned()),
        Some(essences) => {
            sb.push("Essences:".to_owned());
            for (name, amount) in essences {
                let amount = amount.get_u64("current").unwrap_or(0);
                sb.push(format!("- {}: {amount}", get_pretty_name(name)));
            }
        }
    }

    pdr.set_sb(sb);
}

pub async fn get_profile_networth(pdr: &mut PlayerDataResponse, detailed: bool) {
    let mut sb = StringBuilder::new();
    let mut total_value = 0;

    {
        let storage = pdr.profile().storage();
        let containers = vec![
            ("Inventory", storage.inventory().clone()),
            ("Enderchest", storage.ender_chest().clone()),
            ("Storage", storage.backpacks().clone()),
            ("Armor", storage.armor().clone()),
            ("Equipment", storage.equipment().clone()),
            ("Wardrobe", storage.get_wardrobe_items().into_iter().cloned().collect()),
            ("Accessories", storage.accessories().clone()),
            ("Personal Vault", storage.vault().clone()),
        ];

        for (name, items) in containers {
            let mut value = 0;
            let mut item_values = Vec::new();
            for item in items {
                let item_value = calculate_item_value(item.item_id(), item.nbt(), false, true).await;
                item_values.push((item.name().to_owned(), item_value.value()));
                value += item_value.value();
            }

            sb.push(format!("{name}: {} coins", format_number(value)));
            if detailed {
                item_values.sort_by(|a, b| b.1.cmp(&a.1));
                for (item_name, value) in item_values.iter().take(50) {
                    sb.push(format!("- {item_name}: {} coins", format_number(*value)));
                }
                sb.pushln();
            }
            total_value += value;
        }

        let mut item_values = Vec::new();
        let mut sacks_value = 0;
        for (item, amount) in storage.sacks() {
            let price = get_buy_price(item).await.unwrap_or(0) * amount;
            sacks_value += price;
            item_values.push((format!("{}x {}", amount, get_pretty_name(item)), price));
        }

        sb.push(format!("Sacks: {} coins", format_number(sacks_value)));
        if detailed {
            item_values.sort_by(|a, b| b.1.cmp(&a.1));
            for (name, price) in item_values.iter().take(50) {
                sb.push(format!("- {name}: {} coins", format_number(*price)));
            }
        }
        total_value += sacks_value;

        let mut item_values = Vec::new();
        let mut pets_value = 0;
        for pet in storage.pets() {
            let mut value = ItemValue::new(detailed, true);
            get_pet_value(pet, &mut value).await;
            let price = value.value();
            pets_value += price;
            item_values.push((get_pet_info(pet).unwrap_or(pet.name().to_owned()), price));
        }

        sb.push(format!("Pets: {} coins", format_number(pets_value)));
        if detailed {
            item_values.sort_by(|a, b| b.1.cmp(&a.1));
            for (pet, price) in item_values.iter().take(50) {
                sb.push(format!("- {}: {} coins", pet, format_number(*price)));
            }
        }
        total_value += pets_value;
    }

    let api_disabled = total_value == 0;

    let mut museum_value = 0;
    let player_uuid = pdr.player_uuid().to_string();
    if let Some(museum_donations) = get_museum_items(&player_uuid, pdr.profile_mut()).await {
        for donation in museum_donations.iter() {
            if *donation.borrowing() { continue; };
            for item in donation.items() {
                museum_value += calculate_item_value(item.item_id(), item.nbt(), false, true).await.value();
            }
        }
    }
    sb.push(format!("Museum: {} coins", format_number(museum_value)));
    total_value += museum_value;

    let profile = pdr.profile();
    let purse = profile.purse();
    sb.push(format!("Purse: {} coins", format_number(purse)));
    total_value += purse;

    match profile.bank() {
        None => sb.push("Bank: unavailable".to_owned()),
        Some(bank) => {
            sb.push(format!("Bank: {} coins", format_number(bank)));
            total_value += bank;
        }
    }

    let mut essence_value = 0;
    if let Some(essence) = pdr.profile_data().get_object("currencies/essence") {
        for (name, amount) in essence {
            let amount = amount.get_u64("current").unwrap_or(0);
            let price = get_buy_price(&format!("ESSENCE_{}", name)).await.unwrap_or(0);
            essence_value += price * amount;
        }
    }
    total_value += essence_value;
    sb.push(format!("Essence: {} coins", format_number(essence_value)));

    sb.pushln();
    sb.push(format!("Profile Networth: {} coins", format_number_with_commas(total_value)));

    if api_disabled {
        sb.pushln();
        sb.push("NOTE: the player's apis are likely disabled! they should be enabled to provide correct estimation.".to_owned());
    }

    pdr.set_sb(sb);
}

fn get_skill_level(data: &Value, skill: &str, xp: Option<u64>) -> (u64, String) {
    let xp = match xp {
        Some(xp) => xp,
        None => match data.get_f64(&format!("tools/experience/{skill}")) {
            Some(xp) => xp as u64,
            None => return (0, "unavailable".to_owned()),
        },
    };

    let get_skill_cap = match skill {
        "SKILL_TAMING" => data.get_array("pets_data/pet_care/pet_types_sacrificed").map(|arr| arr.len() as u64),
        "SKILL_FARMING" => data.get_u64("jacobs_contest/perks/farming_level_cap"),
        _ => None,
    };

    let xp_table: &[u64] = match skill {
        "SKILL_DUNGEONEERING" => DUNGEONEERING_SKILL_XP,
        "SKILL_RUNECRAFTING" => RUNECRAFTING_SKILL_XP,
        "SKILL_SOCIAL" => SOCIAL_SKILL_XP,
        _ => SKILLS_XP,
    };

    let max_level = *SKILL_MAX_LEVELS.get(skill).unwrap_or(&50);
    let mut cap_limit = max_level;

    if let Some(cap) = get_skill_cap {
        let cap = (50 + cap).min(max_level);
        if cap > 0 {
            cap_limit = cap;
        }
    }

    get_level_and_progress(xp_table, xp, 0, max_level, Some(cap_limit))
}

fn get_level_and_progress(xp_table: &[u64], xp: u64, default: u64, max: u64, cap: Option<u64>) -> (u64, String) {
    let level = xp_table.iter().enumerate()
        .take_while(|&(_, &threshold)| xp >= threshold)
        .map(|(i, _)| i as u64)
        .last()
        .unwrap_or(default);

    let level = level.min(cap.unwrap_or(level));
    let mut str = level.to_string();

    match level != max {
        true => str.push_str(&format!("/{max}")),
        false => str.push_str(" (MAX)"),
    }

    if level != max && let Some(cap) = cap && cap != max {
        str.push_str(&format!(" (Cap: {})", cap));
    }

    if level < max && level < cap.unwrap_or(u64::MAX) {
        let curr_lvl_xp = xp_table.get(level as usize).cloned().unwrap_or(0);
        let next_lvl_xp = xp_table
            .get(level as usize + 1)
            .cloned()
            .unwrap_or(curr_lvl_xp);
        let gained = xp.saturating_sub(curr_lvl_xp);
        let needed = next_lvl_xp.saturating_sub(curr_lvl_xp);

        if needed > 0 {
            let percent = (gained as f64 / needed as f64 * 100.0).floor() as u64;
            str.push_str(&format!(" — {}% ({}/{} XP)", percent, format_number(gained), format_number(needed)));
        }
    }

    (level, str)
}

pub async fn get_pet(pdr: &mut PlayerDataResponse, item_name: &str, include_prices: bool) {
    let mut sb = StringBuilder::new();
    let storage = pdr.profile().storage();

    // Collect pets
    let pets: HashMap<String, &Pet> = storage.pets().iter().map(|pet| (get_pretty_name(pet.name()), pet)).collect();
    let pet_names: Vec<String> = pets.keys().map(|n| n.to_owned()).collect();

    // Find best matches
    let matches = find_best_matches(item_name, &pet_names);
    let Some(best_pet) = matches.first() else {
        sb.push("Couldn't find any matching pet!".to_owned());
        pdr.set_sb(sb);
        return;
    };

    // Try to find in pets
    match pets.iter().find(|(name, _)| name == best_pet) {
        None => sb.push("Couldn't find any matching pet!".to_owned()),
        Some((_, pet)) => {
            let mut value = ItemValue::new(include_prices, true);
            get_pet_value(pet, &mut value).await;
            for line in value.info() {
                sb.push(line.to_owned());
            }
            sb.push(format!("Estimated Pet Value: {} coins", format_number_with_commas(value.value())));
            if *pet.active() {
                sb.push("The Pet is Active".to_owned());
            }
        }
    }

    // Add similar pets
    if matches.len() > 1 {
        sb.pushln();
        sb.push("Other Similar Pets:".to_owned());
        for pet in matches.iter().skip(1) {
            sb.push(format!("- {pet}"));
        }
    }

    pdr.set_sb(sb);
}

pub async fn get_item(pdr: &mut PlayerDataResponse, item_name: &str, include_prices: bool) {
    let mut sb = StringBuilder::new();
    let storage = pdr.profile().storage();

    // Collect storage items
    let mut items: Vec<(String, &Item)> = Vec::new();
    for item in storage.get_items_list() {
        let display_name = get_item_display_name(item.item_id()).await.unwrap_or(item.name().to_owned());
        items.push((display_name, item));
    }

    // Collect sack items
    let mut sack_items = HashMap::new();
    for (id, amount) in storage.sacks().iter() {
        let name = get_item_display_name(id).await.unwrap_or(get_pretty_name(id));
        sack_items.insert(name, *amount);
    }

    // Build combined searchable list
    let mut item_names: Vec<String> = items.iter().map(|(n, _)| n.clone()).collect();
    item_names.extend(sack_items.keys().cloned());
    item_names.sort();
    item_names.dedup();

    // Find best matches
    let matches = find_best_matches(item_name, &item_names);
    let Some(best_item) = matches.first() else {
        sb.push("Couldn't find any matching item!".to_owned());
        pdr.set_sb(sb);
        return;
    };

    // Try to find in items
    match items.iter().find(|(name, _)| name == best_item) {
        Some((_, item)) => {
            let name = item.name();
            let &count = item.count();
            sb.push(match count > 1 {
                true => format!("Item: {count}x {name}"),
                false => format!("Item: {name}"),
            });

            let value = calculate_item_value(item.item_id(), item.nbt(), include_prices, true).await;
            for line in value.info().iter().skip(1) {
                sb.push(line.to_owned());
            }
        }
        None => match sack_items.get(*best_item) {
            Some(amount) => sb.push(format!("Item: {amount}x {best_item} (in Sacks)")),
            None => sb.push("Couldn't find any matching item!".to_owned()),
        },
    }

    // Add similar items
    if matches.len() > 1 {
        sb.pushln();
        sb.push("Other Similar Items:".to_owned());
        for item in matches.iter().skip(1) {
            sb.push(format!("- {item}"));
        }
    }

    pdr.set_sb(sb);
}
