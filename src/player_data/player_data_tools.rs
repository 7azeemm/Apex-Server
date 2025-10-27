use crate::constants::garden::{CANE_CACTUS_MILESTONE_XP, CARROT_POTATO_MILESTONE_XP, COCOA_WART_MILESTONE_XP, CROP_NAMES, GARDEN_LEVELS_XP, MAX_COMPOSTER_UPGRADE_LEVEL, MAX_CROP_MILESTONE, MAX_CROP_UPGRADE_LEVEL, MAX_GARDEN_LEVEL, MAX_PLOTS, MELON_MILESTONE_XP, WHEAT_PUMPKIN_MUSHROOM_MILESTONE_XP};
use crate::constants::misc::{MAX_BESTIARY_LEVEL, MAX_ENIGMA_SOULS, MAX_FAIRY_SOULS, MAX_MINING_COMMISSION_MILESTONE, MAX_TIMECHARMS, TROPHY_FISHING_TIERS};
use crate::constants::setups::SetupType;
use crate::constants::skills::{DUNGEONEERING_SKILL_XP, RUNECRAFTING_SKILL_XP, SKILLS, SKILLS_XP, SKILL_MAX_LEVELS, SOCIAL_SKILL_XP};
use crate::extensions::json_ext::JsonExt;
use crate::item_utils::{get_pet_info, get_pet_obj, get_pretty_name};
use crate::live_data::jacob_contests::get_upcoming_contests;
use crate::live_data::mayor_info::{get_election_over_time_left, get_mayor_info, get_skyblock_date, get_special_mayors_info};
use crate::player_data::profile_fetcher::{get_garden_data, get_museum_items};
use crate::prices::bazaar::get_buy_price;
use crate::prices::cosmetic_prices::get_pet_networth;
use crate::prices::item_value_calculator::{calculate_item_value, get_pet_full_info};
use crate::structs::player_data_structs::{PlayerDataResponse, PlayerProfile, StringBuilder};
use crate::utils::{format_number, format_number_with_commas, get_time_as_secs};
use serde_json::Value;
use std::collections::HashMap;

pub async fn get_player_overview(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

    sb.push(format!("GameMode: {} Profile", get_pretty_name(pdr.profile().game_mode())));
    get_sb_level(profile_data, &mut sb);
    get_purse_and_bank(pdr.profile(), &mut sb);
    get_active_pet(profile_data, &mut sb);
    get_accessories_info(profile_data, &mut sb);
    get_skills(profile_data, &mut sb);

    pdr.set_resp(sb);
}

pub async fn get_mining_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();
    let mining_setup = pdr.profile().setups().get(&SetupType::Mining);

    if let Some(mining_setup) = mining_setup {
        sb.push("Armor:".to_owned());
        for piece in mining_setup.armor() {
            sb.push(format!(" - {piece}"));
        }

        sb.push("Equipment:".to_owned());
        for piece in mining_setup.equipment() {
            sb.push(format!(" - {piece}"));
        }

        for tool in mining_setup.tools() {
            sb.push(format!("Mining Tool: {tool}"))
        }

        sb.push(format!("Pet: {}", mining_setup.pet()));
    }

    sb.pushln();

    let (_, mining_skill) = get_skill_level(profile_data, "SKILL_MINING");
    sb.push(format!("Skill {mining_skill}"));

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
            sb.push(format!(" - {label}: {}", format_number(spent_amount + unspent_amount)));
        }
    }

    if let Some(nucleus_runs) = profile_data.get_u64("leveling/completions/NUCLEUS_RUNS") {
        sb.push(format!("Nucleus Runs: {nucleus_runs} runs"));
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
                sb.push(format!(" - {}: {count}", get_pretty_name(corpse)));
            }
        }
        // if let Some(fossils_donated) = glacite_core.get_array("fossils_donated") {
        //     let fossils: Vec<String> = fossils_donated.iter().filter_map(|v| v.as_str()).map(|s| get_pretty_name(s)).collect();
        //     sb.push(format!("Fossils donated: {}", fossils.join(", ")));
        // }
    }

    pdr.set_resp(sb);
}

pub async fn get_garden_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let garden_setup = pdr.profile().setups().get(&SetupType::Farming);

    if let Some(setup) = garden_setup {
        // let tool_names = vec![
        //     "Wheat Hoe", "Carrot Hoe", "Potato Hoe", "Nether Warts Hoe", "Sugar Cane Hoe", "Melon Axe",
        //     "Pumpkin Axe", "Cocoa Beans Axe", "Cactus Knife", "Mushroom Hoe"
        // ];

        sb.push("Armor:".to_owned());
        for piece in setup.armor() {
            sb.push(format!(" - {piece}"));
        }

        sb.push("Equipment:".to_owned());
        for piece in setup.equipment() {
            sb.push(format!(" - {piece}"));
        }

        // for (i, tool) in setup.tools().iter().enumerate() {
        //     result.push(format!("{}: {}", tool_names.get(i).unwrap_or(&"Tool"), tool))
        // }

        sb.push(format!("Pet: {}", setup.pet()));
    }

    sb.pushln();
    let (_, farming_skill) = get_skill_level(pdr.profile_data(), "SKILL_FARMING");
    sb.push(format!("Skill {farming_skill}"));

    let garden_data = get_garden_data(pdr.profile_mut()).await;
    if let Some(garden_data) = garden_data {
        if let Some(garden) = garden_data.get("garden") {
            if let Some(xp) = garden.get_f64("garden_experience") {
                let level = GARDEN_LEVELS_XP.iter().enumerate()
                    .take_while(|&(_, &threshold)| xp >= threshold as f64)
                    .map(|(i, _)| i as u64)
                    .last()
                    .unwrap_or(1);

                sb.push(format!("Garden Level: {level}/{MAX_GARDEN_LEVEL}"));
            }

            let commission_data = garden.get("commission_data");
            let visitors_served = commission_data.and_then(|v| v.get_u64("total_completed")).unwrap_or(0);
            let unique_visitors = commission_data.and_then(|v| v.get_u64("unique_npcs_served")).unwrap_or(0);
            let plots = garden.get_array("unlocked_plots_ids").and_then(|v| Some(v.len())).unwrap_or(0);

            sb.push(format!("Visitors served: {visitors_served}"));
            sb.push(format!("Unique Visitors served: {unique_visitors}"));
            sb.push(format!("Unlocked Plots: {plots}/{MAX_PLOTS}"));

            if let Some(composter_upgrades) = garden.get_object("composter_data/upgrades") {
                sb.push("Composter upgrades:".to_owned());
                for (upgrade, level) in composter_upgrades {
                    sb.push(format!(" - {}: {level}/{MAX_COMPOSTER_UPGRADE_LEVEL}", get_pretty_name(upgrade)));
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
                        _ => continue
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

                    let mut line = format!(" - {}: {level}/{MAX_CROP_MILESTONE}", CROP_NAMES.get(crop).unwrap_or(&&*"N/A".to_owned()));
                    line.push_str(&match progress {
                        None => " (Max)".to_owned(),
                        Some(progress) => format!(" ({progress}%)")
                    });

                    if let Some(crop_upgrade_level) = crop_upgrades.and_then(|c| c.get(crop)) {
                        let level = crop_upgrade_level.as_u64().unwrap_or(0);
                        line.push_str(&format!(" | Upgrade: {level}/{MAX_CROP_UPGRADE_LEVEL}"))
                    }
                    sb.push(line);
                }
            }
        }
    }

    let profile_data = pdr.profile_data();
    if let Some(medals_inv) = profile_data.get("jacobs_contest/medals_inv") {
        sb.push("Jacob Medals:".to_owned());
        for bracket in vec!["bronze", "silver", "gold", "platinum", "diamond"] {
            let amount = medals_inv.get_u64(bracket).unwrap_or(0);
            sb.push(format!(" - {}: {}", get_pretty_name(bracket), amount))
        }
    }

    pdr.set_resp(sb);
}

pub async fn get_foraging_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();

    let (_, foraging_skill) = get_skill_level(pdr.profile_data(), "SKILL_FORAGING");
    sb.push(format!("Skill {foraging_skill}"));

    if let Some(foraging_setup) = pdr.profile().setups().get(&SetupType::Foraging) {
        sb.push("Armor:".to_owned());
        for piece in foraging_setup.armor() {
            sb.push(format!(" - {piece}"));
        }

        sb.push("Equipment:".to_owned());
        for piece in foraging_setup.equipment() {
            sb.push(format!(" - {piece}"));
        }

        for tool in foraging_setup.tools() {
            sb.push(format!("Tool: {tool}"))
        }

        sb.push(format!("Pet: {}", foraging_setup.pet()));
    }

    pdr.set_resp(sb);
}

pub async fn get_fishing_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

    let (_, fishing_skill) = get_skill_level(profile_data, "SKILL_FISHING");
    sb.push(format!("Skill {fishing_skill}"));

    let trophy_fishing_tier = profile_data.get_array("trophy_fish/rewards").map(|a| a.len()).unwrap_or(0);
    sb.push(format!("Trophy Fishing Tier: {}", TROPHY_FISHING_TIERS.get(trophy_fishing_tier).unwrap_or(&"None")));

    if let Some(fishing_setup) = pdr.profile().setups().get(&SetupType::Fishing) {
        sb.push("Armor:".to_owned());
        for piece in fishing_setup.armor() {
            sb.push(format!(" - {piece}"));
        }

        sb.push("Equipment:".to_owned());
        for piece in fishing_setup.equipment() {
            sb.push(format!(" - {piece}"));
        }

        let rods = fishing_setup.tools();
        if !rods.is_empty() {
            if rods.len() == 1 {
                sb.push(format!("Rod: {}", rods.first().unwrap()))
            } else {
                sb.push(format!("Water Rod: {}", rods.first().unwrap()));
                sb.push(format!("Lava Rod: {}", rods.get(1).unwrap()));
            }
        }

        sb.push(format!("Pet: {}", fishing_setup.pet()));
    }

    // if let Some(trophy_fishing) = profile_data.get_array("trophy_fish") {
    //     sb.push("TrophyFishing:".to_owned());
    //     for fish in TROPHY_FISHES {
    //         let count = trophy_fishing.get_u64(fish).unwrap_or(0);
    //         sb.push(format!(" - {}: {}", get_pretty_name(fish), count));
    //     }
    // }

    pdr.set_resp(sb);
}

pub async fn get_slayer_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

    if let Some(slayer_bosses) = profile_data.get_object("slayer/slayer_bosses") {
        sb.push("Slayers:".to_owned());
        for (slayer, data) in slayer_bosses {
            let slayer = get_pretty_name(slayer);
            let level = data.get_object("claimed_levels").map(|v| v.len());
            let xp = data.get_u64("xp");
            if let (Some(level), Some(xp)) = (level, xp) {
                sb.push(format!(" - {slayer} level: {level} (xp: {xp})"));
            }
        }
    }

    pdr.set_resp(sb);
}

pub async fn get_dungeons_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

    let max_level = *SKILL_MAX_LEVELS.get("SKILL_DUNGEONEERING").unwrap_or(&50);
    if let Some(dungeons) = profile_data.get("dungeons") {
        if let Some(dungeon_types) = dungeons.get("dungeon_types") {
            if let Some(catacombs) = dungeon_types.get("catacombs") {
                let xp = catacombs.get_f64("experience").unwrap_or(0.0) as u64;

                sb.push(format!("Dungeons Level: {}/{max_level}", get_xp_table_level(&DUNGEONEERING_SKILL_XP, xp)));

                if let Some(secrets) = dungeons.get_u64("secrets") {
                    sb.push(format!("Secrets: {secrets}"));
                }

                if let Some(selected_class) = dungeons.get_str("selected_dungeon_class") {
                    let selected_class = get_pretty_name(selected_class);
                    sb.push(format!("Selected Class: {selected_class}"));

                    let dungeon_setups = pdr.profile().setups();
                    if let Some(setup) = SetupType::from_str(&*selected_class).and_then(|setup| dungeon_setups.get(&setup)) {
                        sb.pushln();
                        sb.push(format!("{selected_class} Setup:"));

                        sb.push(" Armor:".to_owned());
                        for piece in setup.armor() {
                            sb.push(format!("  - {piece}"));
                        }

                        sb.push(" Equipment:".to_owned());
                        for piece in setup.equipment() {
                            sb.push(format!("  - {piece}"));
                        }

                        for tool in setup.tools() {
                            sb.push(format!(" Weapon: {tool}"))
                        }

                        sb.push(format!(" Pet: {}", setup.pet()));
                        sb.pushln();
                    }
                }

                if let Some(classes) = dungeons.get_object("player_classes") {
                    sb.push("Dungeon Classes:".to_owned());
                    for (class, xp) in classes {
                        let level = get_xp_table_level(&DUNGEONEERING_SKILL_XP, xp.get_f64("experience").unwrap_or(0.0) as u64);
                        sb.push(format!(" - {}: {level}/{max_level}", get_pretty_name(class)));
                    }
                }

                sb.push("Catacombs runs count:".to_string());
                if let Some(milestone_completions) = catacombs.get_object("milestone_completions") {
                    for (floor, completions) in milestone_completions {
                        let completions = completions.as_f64().unwrap_or(0.0) as u64;
                        if floor == "total" { continue; };
                        let floor_name = if floor == "0" { "Entrance" } else {
                            &*format!("Floor {floor}")
                        };
                        sb.push(format!(" - {floor_name}: {completions}"))
                    }
                }
            }


            if let Some(master_catacombs) = dungeon_types.get("master_catacombs") {
                sb.push("Master Mode runs count:".to_owned());
                if let Some(milestone_completions) = master_catacombs.get_object("milestone_completions") {
                    for (floor, completions) in milestone_completions {
                        let completions = completions.as_f64().unwrap_or(0.0) as u64;
                        if floor == "total" { continue; };
                        sb.push(format!(" - Floor {floor}: {completions}"))
                    }
                }
            }
        }
    }

    pdr.set_resp(sb);
}

pub async fn get_crimson_island_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

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
            for tier in vec!["none", "hot", "burning", "fiery", "infernal"] {
                let comp = kuudra.get_u64(tier).unwrap_or(0);
                let tier_name = if tier == "none" { "Basic".to_string() } else { get_pretty_name(tier) };
                sb.push(format!(" - {tier_name}: {comp} runs"))
            }
        }
    }

    pdr.set_resp(sb);
}

//TODO: don't require profile and should be in different space with price tools
pub async fn get_events_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();

    sb.push(format!("SkyBlock Date: {}", get_skyblock_date()));
    sb.pushln();

    let mayor_info = get_mayor_info().await;
    let mayor = mayor_info.mayor();
    let mayor_perks = mayor.perks().keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
    sb.push(format!("Current Mayor: {} (perks: [{}])", mayor.name(), mayor_perks));

    if let Some(minister) = mayor_info.minister() {
        let minister_perk = minister.perks().keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        sb.push(format!("Current Minister: {} (perks: [{}])", minister.name(), minister_perk));
    }

    sb.pushln();
    sb.push("Special Mayors:".to_owned());
    sb.push(get_special_mayors_info());
    sb.pushln();

    if let Some(election) = mayor_info.election() {
        sb.push("Current Election:".to_owned());
        if election.iter().all(|(_, votes)| votes.is_none()) {
            sb.push(" (Votes are hidden)".to_owned());
        }
        for (mayor, votes) in election.iter() {
            let mut mayor_str = format!(" - {}", mayor.name());
            if let Some(votes) = votes {
                mayor_str.push_str(&format!(" (votes: {})", votes));
            }
            sb.push(mayor_str)
        }
        sb.push(get_election_over_time_left());
        sb.pushln();
    }

    let upcoming_contests = get_upcoming_contests().await;
    if !upcoming_contests.is_empty() {
        sb.push("Jacob Contests:".to_owned());
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

                sb.push(format!(" - [{}] after {}mins (at {})", crops_str, total_minutes, formatted_time));
            }
        }
    }

    pdr.set_resp(sb);
}

pub async fn get_misc_info(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let profile_data = pdr.profile_data();

    if let Some(bestiary_level) = profile_data.get_u64("bestiary/milestone/last_claimed_milestone") {
        sb.push(format!("Bestiary Level: {bestiary_level}/{MAX_BESTIARY_LEVEL}"));
    }

    if let Some(pet_score) = profile_data.get_u64("leveling/highest_pet_score") {
        sb.push(format!("Pet Score: {pet_score}"));
    }

    if let Some(fairy_souls) = profile_data.get_u64("fairy_soul/total_collected") {
        sb.push(format!("Fairy Souls: {fairy_souls}/{MAX_FAIRY_SOULS}"))
    }

    if let Some(powers_list) = profile_data.get_array("accessory_bag_storage/unlocked_powers") {
        let values = powers_list.iter().filter_map(|v| v.as_str()).map(|s| get_pretty_name(s)).collect::<Vec<_>>().join(", ");
        sb.push(format!("Unlocked Powers: [{values}]"));
    }

    sb.pushln();

    if let Some(rift) = profile_data.get("rift") {
        sb.push("Rift:".to_owned());
        if let Some(timecharms) = rift.get_array("gallery/secured_trophies") {
            sb.push(format!(" - Timecharms: {}/{MAX_TIMECHARMS}", timecharms.len()));
        }

        if let Some(enigma_souls) = rift.get_array("enigma/found_souls") {
            sb.push(format!(" - Enigma souls: {}/{MAX_ENIGMA_SOULS}", enigma_souls.len()));
        }

        if let Some(motes) = profile_data.get_f64("currencies/motes_purse") {
            sb.push(format!(" - Motes: {}", motes as u64));
        }
        sb.pushln();
    }

    if let Some(essence) = profile_data.get_object("currencies/essence") {
        sb.push("Essences:".to_owned());
        for (name, amount) in essence {
            let amount = amount.get_u64("current").unwrap_or(0);
            sb.push(format!(" - {}: {amount}", get_pretty_name(name)));
        }
    }

    pdr.set_resp(sb);
}

pub async fn get_inventory(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let storage = pdr.profile().storage();

    let armor = storage.armor();
    if !armor.is_empty() {
        sb.push("Armor:".to_owned());
        for piece in armor {
            sb.push(format!(" - {}", piece.name()));
        }
        sb.pushln();
    }

    let equipment = storage.equipment();
    if !equipment.is_empty() {
        sb.push("Equipment:".to_owned());
        for piece in equipment {
            sb.push(format!(" - {}", piece.name()));
        }
        sb.pushln();
    }

    let inventory = storage.inventory();
    if !inventory.is_empty() {
        sb.push("Inventory:".to_owned());
        let mut items: Vec<(&str, u64)> = Vec::new();

        for item in inventory {
            let &count = item.count();
            let item_name = item.name();
            match items.iter_mut().find(|(name, _)| *name == item_name) {
                Some((_, c)) => *c += count,
                None => items.push((item_name, count))
            }
        }

        for (item, count) in items {
            sb.push(match count > 1 {
                true => format!(" - {count}x {item}"),
                false => format!(" - {item}")
            })
        }
    }

    pdr.set_resp(sb);
}

pub async fn get_profile_networth(pdr: &mut PlayerDataResponse) {
    let mut sb = StringBuilder::new();
    let mut total_value = 0;

    {
        let mut profile = pdr.profile();
        let storage = profile.storage();
        let containers = vec![
            ("Inventory", storage.inventory().clone()),
            ("Enderchest", storage.ender_chest().clone()),
            ("Backpacks", storage.backpacks().clone()),
            ("Armor", storage.armor().clone()),
            ("Equipment", storage.equipment().clone()),
            ("Wardrobe", storage.get_wardrobe_items().into_iter().cloned().collect()),
            ("Accessories", storage.accessories().clone()),
            ("Personal Vault", storage.vault().clone())
        ];

        for (name, items) in containers {
            let mut value = 0;
            for item in items {
                value += calculate_item_value(item.item_id(), item.nbt()).await.value();
            }
            sb.push(format!("{}: {} Coins", name, format_number(value)));
            total_value += value;
        }

        let mut sacks_value = 0;
        for (item, amount) in storage.sacks() {
            sacks_value += get_buy_price(item).await.unwrap_or(0) * amount;
        }
        sb.push(format!("Sacks: {} Coins", format_number(sacks_value)));
        total_value += sacks_value;

        let mut pets_value = 0;
        for pet in storage.pets() {
            pets_value += get_pet_networth(pet).await;
        }
        sb.push(format!("Pets: {} Coins", format_number(pets_value)));
        total_value += pets_value;
    }

    let mut museum_value = 0;
    let player_uuid = pdr.player_uuid().to_string();
    if let Some(museum_donations) = get_museum_items(&player_uuid, pdr.profile_mut()).await {
        for donation in museum_donations.iter() {
            if *donation.borrowing() { continue; };
            for item in donation.items() {
                museum_value += calculate_item_value(item.item_id(), item.nbt()).await.value();
            }
        }
    }
    sb.push(format!("Museum: {} Coins", format_number(museum_value)));
    total_value += museum_value;

    let profile = pdr.profile();
    let purse = profile.purse();
    let bank = profile.bank();
    total_value += purse + bank;
    sb.push(format!("Purse: {} Coins", format_number(purse)));
    sb.push(format!("Bank: {} Coins", format_number(bank)));

    let mut essence_value = 0;
    if let Some(essence) = pdr.profile_data().get_object("currencies/essence") {
        for (name, amount) in essence {
            let amount = amount.get_u64("current").unwrap_or(0);
            let price = get_buy_price(&format!("ESSENCE_{}", name)).await.unwrap_or(0);
            essence_value += price * amount;
        }
    }
    total_value += essence_value;
    sb.push(format!("Essence: {} Coins", format_number(essence_value)));

    sb.pushln();
    sb.push(format!("Profile Networth: {} Coins", format_number_with_commas(total_value)));

    pdr.set_resp(sb);
}

fn get_sb_level(data: &Value, sb: &mut StringBuilder) {
    if let Some(xp) = data.get_u64("leveling/experience") {
        let level = xp / 100;
        let progress = xp % 100;
        sb.push(format!("SkyBlock Level {} ({}/100)", level, progress));
    }
}

fn get_skills(data: &Value, sb: &mut StringBuilder) {
    sb.push("Skills:".to_owned());
    let mut total_level = 0;
    let mut count = 0;

    for &skill in SKILLS {
        let (skill_level, skill_line) = get_skill_level(data, skill);

        // Exclude cosmetic skills from average
        if skill != "SKILL_RUNECRAFTING" && skill != "SKILL_SOCIAL" {
            total_level += skill_level;
            count += 1;
        }

        sb.push(format!(" - {skill_line}"));
    }

    if count > 0 {
        let avg = total_level as f64 / count as f64;
        sb.push(format!("Average Skill Level: {:.2}", avg));
    }
}

fn get_skill_level(data: &Value, skill: &str) -> (u64, String) {
    let xp = data.get_f64(&format!("player_data/experience/{skill}")).unwrap_or(0.0) as u64;

    let get_skill_cap = |skill: &str| -> Option<u64> {
        match skill {
            "SKILL_TAMING" => data.get_array("pets_data/pet_care/pet_types_sacrificed").map(|arr| arr.len() as u64),
            "SKILL_FARMING" => data.get_u64("jacobs_contest/perks/farming_level_cap"),
            _ => None
        }
    };

    let xp_table: &[u64] = match skill {
        "SKILL_RUNECRAFTING" => &RUNECRAFTING_SKILL_XP,
        "SKILL_SOCIAL" => &SOCIAL_SKILL_XP,
        _ => &SKILLS_XP,
    };

    let level = get_xp_table_level(xp_table, xp);
    let max_level = *SKILL_MAX_LEVELS.get(skill).unwrap_or(&60);
    let mut skill_cap = None;
    let mut cap_limit = max_level;

    if let Some(cap) = get_skill_cap(skill) {
        let cap = (50 + cap).min(max_level);
        if cap > 0 {
            skill_cap = Some(cap);
            cap_limit = cap;
        }
    }

    let skill_level = level.min(cap_limit);

    // Calculate percentage to next level if not maxed/capped
    let mut progress = None;
    if skill_level < cap_limit {
        let curr_lvl_xp = xp_table.get(skill_level as usize).cloned().unwrap_or(0);
        let next_lvl_xp = xp_table.get(skill_level as usize + 1).cloned().unwrap_or(curr_lvl_xp);
        let gained = xp.saturating_sub(curr_lvl_xp);
        let needed = next_lvl_xp.saturating_sub(curr_lvl_xp);
        if needed > 0 {
            let percent = (gained as f64 / needed as f64 * 100.0).floor() as u64;
            progress = Some(format!("{}% ({}/{})", percent, format_number(gained), format_number(needed)));
        }
    }

    let mut skill_line = format!("{}: {skill_level}", get_pretty_name(&skill.replace("SKILL_", "")));

    match skill_level != max_level {
        true => skill_line.push_str(&format!("/{max_level}")),
        false => skill_line.push_str(" (Max)")
    }

    if let Some(cap) = skill_cap && cap != max_level {
        skill_line.push_str(&format!(" (cap: {})", cap));
    }
    if let Some(progress) = progress {
        skill_line.push_str(&format!(" — {}", progress));
    }

    (skill_level, skill_line)
}

fn get_xp_table_level(xp_table: &[u64], xp: u64) -> u64 {
    xp_table.iter().enumerate()
        .take_while(|&(_, &threshold)| xp >= threshold)
        .map(|(i, _)| i as u64)
        .last()
        .unwrap_or(0)
}

fn get_purse_and_bank(profile: &PlayerProfile, sb: &mut StringBuilder) {
    sb.push(format!("Purse: {} Coins", format_number(profile.purse())));
    sb.push(format!("Bank: {} Coins", format_number(profile.bank())));
}

fn get_accessories_info(data: &Value, sb: &mut StringBuilder) {
    let Some(acc_storage) = data.get("accessory_bag_storage") else { return };
    sb.push("Accessories:".to_owned());

    let selected_power = acc_storage.get_str("selected_power").map(|s| get_pretty_name(s)).unwrap_or("N/A".to_owned());
    sb.push(format!(" - Selected Power Stone: {selected_power}"));

    let mp = acc_storage.get_u64("highest_magical_power").unwrap_or(0);
    sb.push(format!(" - Magical Power: {mp}"));
}

fn get_active_pet(data: &Value, sb: &mut StringBuilder) {
    let Some(pets) = data.get_array("pets_data/pets") else { return };
    if let Some(active_pet) = pets.iter().find(|pet| pet.get_bool("active").unwrap_or(false)) {
        if let Some(pet_info) = get_pet_obj(active_pet).and_then(|v| get_pet_info(&v)) {
            sb.push(format!("Active Pet: {pet_info}"))
        }
    }
}

pub async fn get_item(pdr: &mut PlayerDataResponse, item_name: &str, is_pet: bool) {
    let mut sb = StringBuilder::new();
    let storage = pdr.profile().storage();

    if is_pet {
        let pet_names: Vec<String> = storage.pets().iter()
            .map(|pet| get_pretty_name(pet.name()))
            .collect();

        let matches = find_best_match(item_name, &pet_names);
        if !matches.is_empty() {
            let best_match = *matches.first().unwrap();
            let pet = storage.pets().iter()
                .find(|pet| get_pretty_name(pet.name()) == best_match)
                .map(|pet| (*pet).clone());

            if let Some(pet) = pet {
                let (lines, pet_value) = get_pet_full_info(&pet).await;
                for line in lines {
                    sb.push(line);
                }
                sb.push(format!("Pet Value: {} Coins", format_number_with_commas(pet_value)));
                if *pet.active() {
                    sb.push("The Pet is Active".to_owned());
                }
            } else {
                sb.push("Couldn't find any pet matches!".to_owned())
            }

            if matches.len() > 1 {
                sb.push("Other Similar Pets:".to_owned());
                for pet in matches.iter().skip(1) {
                    sb.push(format!(" - {pet}"));
                }
            }
        } else {
            sb.push("Couldn't find any pet matches!".to_owned())
        }
    } else {
        let items = storage.get_items_list();
        let mut item_names: Vec<String> = items.iter().map(|item| item.name().to_owned()).collect();
        let sacks = storage.sacks();
        let sack_names: HashMap<String, String> = sacks.iter().map(|(s, _)| (get_pretty_name(s), s.to_owned())).collect();
        item_names.extend(sack_names.keys().cloned());
        item_names.dedup();

        let matches = find_best_match(item_name, &item_names);
        if !matches.is_empty() {
            let item_name = *matches.first().unwrap();
            let item = items.iter()
                .find(|item| item.name() == item_name)
                .map(|item| (*item).clone());

            if let Some(ref item) = item {
                let &count = item.count();
                sb.push(match count > 1 {
                    true => format!("Item: {count}x {item_name}"),
                    false => format!("Item: {item_name}")
                });
                let value = calculate_item_value(item.item_id(), item.nbt()).await;
                for line in value.info().iter().skip(1) {
                    sb.push(line.to_owned());
                }
            } else if let Some(item_id) = sack_names.get(item_name) && let Some(amount) = sacks.get(item_id) {
                sb.push(format!("Item: {amount}x {item_name} (Found in Sacks)"));
            } else {
                sb.push("Couldn't find any item matches!".to_owned())
            }

            if matches.len() > 1 {
                sb.pushln();
                sb.push("Other Similar Items:".to_owned());
                for item in matches.iter().skip(1) {
                    sb.push(format!(" - {item}"));
                }
            }
        } else {
            sb.push("Couldn't find any item matches!".to_owned())
        }
    }

    pdr.set_resp(sb);
}

fn find_best_match<'a>(query: &'a str, list: &'a [String]) -> Vec<&'a str> {
    // 1. Collect scores
    let mut scored: Vec<(&str, usize)> = list
        .iter()
        .map(|item| (item.as_str(), word_overlap_score(item, query)))
        .collect();

    // 2. Sort by overlap
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    // 3. Take top N
    let best_matches: Vec<&str> = scored.iter().take(5).filter(|(_, s)| *s > 0).map(|(s, _)| *s).collect();

    // 4. Exact (case-insensitive) match prioritization
    if let Some((exact, _)) = scored.iter().find(|(cand, _)| cand.eq_ignore_ascii_case(query)) {
        let mut out = Vec::with_capacity(best_matches.len() + 1);
        out.push(*exact); // ensure exact match first
        for m in best_matches {
            if m != *exact {
                out.push(m);
            }
        }
        return out;
    }

    best_matches
}

fn word_overlap_score(candidate: &str, query: &str) -> usize {
    let lowercase = query.to_lowercase();
    let query_words: Vec<_> = lowercase.split_whitespace().collect();
    let candidate = candidate.to_lowercase();

    query_words.iter().filter(|w| candidate.contains(*w)).count()
}