use crate::constants::enchantments::{NPC_ENCHANTS, STACKING_ENCHANTS, TIER_FIVE_ENCHANTS, TIER_ONE_ENCHANTS, TIER_THREE_ENCHANTS, UPGRADABLE_ENCHANTS};
use crate::constants::misc::{GEMSTONES, MASTER_STARS, STARRED_ITEMS_INGREDIENT};
use crate::item_utils::{get_item_name, get_item_rarity, get_pet_info, get_pet_level, get_pet_obj, get_pretty_name};
use crate::prices::auctions::{get_base_price, get_lowest_bin};
use crate::prices::bazaar::get_buy_price;
use crate::prices::cosmetic_prices::get_cosmetic_price;
use crate::repos::neu::essence_costs::get_essence_costs;
use crate::repos::neu::gemstone_slots_cost::get_item_gemstone_slots;
use crate::repos::neu::reforge_stones::get_reforge_stone;
use crate::structs::item_structs::{ItemNbt, ItemValue, ModifierContext};
use crate::structs::player_data_structs::Pet;
use crate::structs::value_calc_structs::{CountedItemModifier, ModifierHandler, SingleItemModifier};
use crate::utils::format_number;
use async_trait::async_trait;
use common::extensions::fastnbt_ext::ValueExt;
use fastnbt::Value;
use once_cell::sync::Lazy;
use std::cmp::min;
use std::collections::HashMap;
use std::ops::Add;

const POTATO_BOOK_ID: &str = "HOT_POTATO_BOOK";
const FUMING_BOOK_ID: &str = "FUMING_POTATO_BOOK";
const ART_OF_PEACE_ID: &str = "THE_ART_OF_PEACE";
const ART_OF_WAR_ID: &str = "THE_ART_OF_WAR";
const TRANSMISSION_TUNER_ID: &str = "TRANSMISSION_TUNER";
const SILEX_ID: &str = "SIL_EX";
const STONK_PICKAXE_ID: &str = "STONK_PICKAXE";
const PROMISING_SPADE_ID: &str = "PROMISING_SPADE";
const RECOMBOBULATOR_ID: &str = "RECOMBOBULATOR_3000";
const ETHERWARP_CONDUIT_ID: &str = "ETHERWARP_CONDUIT";
const JALAPENO_BOOK_ID: &str = "JALAPENO_BOOK";
const MANA_DISINTEGRATOR_ID: &str = "MANA_DISINTEGRATOR";
const BOOK_OF_STATS_ID: &str = "BOOK_OF_STATS";
const WET_BOOK_ID: &str = "WET_BOOK";
const FARMING_FOR_DUMMIES_ID: &str = "FARMING_FOR_DUMMIES";
const POLARVOID_BOOK_ID: &str = "POLARVOID_BOOK";
const DIVAN_POWDER_COATING_ID: &str = "DIVAN_POWDER_COATING";
const WOOD_SINGULARITY_ID: &str = "WOOD_SINGULARITY";

static MODIFIERS: Lazy<Vec<(&'static str, Box<dyn ModifierHandler>)>> = Lazy::new(|| {
    vec![
        ("modifier", Box::new(ReforgeModifier)),
        ("enchantments", Box::new(EnchantmentsModifier)),
        ("hot_potato_count", Box::new(PotatoBooksModifier)),
        ("upgrade_level", Box::new(UpgradeLevelModifier)),
        ("rarity_upgrades", Box::new(SingleItemModifier::new("Recombobulator", RECOMBOBULATOR_ID))),
        ("artOfPeaceApplied", Box::new(SingleItemModifier::new("Art Of Peace", ART_OF_PEACE_ID))),
        ("art_of_war_count", Box::new(SingleItemModifier::new("Art Of War", ART_OF_WAR_ID))),
        ("tuned_transmission", Box::new(CountedItemModifier::new("Transmission Tuners", TRANSMISSION_TUNER_ID, 4))),
        ("jalapeno_count", Box::new(SingleItemModifier::new("Jalapeno Book", JALAPENO_BOOK_ID))),
        ("mana_disintegrator_count", Box::new(CountedItemModifier::new("Mana Disintegrators", MANA_DISINTEGRATOR_ID, 10))),
        ("stats_book", Box::new(SingleItemModifier::new("Stats Book", BOOK_OF_STATS_ID))),
        ("wet_book_count", Box::new(CountedItemModifier::new("Wet Book", WET_BOOK_ID, 5))),
        ("farming_for_dummies_count", Box::new(CountedItemModifier::new("Farming For Dummies", FARMING_FOR_DUMMIES_ID, 5))),
        ("wood_singularity_count", Box::new(SingleItemModifier::new("Wood Singularity", WOOD_SINGULARITY_ID))),
        ("polarvoid", Box::new(CountedItemModifier::new("Polarvoid Books", POLARVOID_BOOK_ID, 5))),
        ("divan_powder_coating", Box::new(SingleItemModifier::new("Divan Powder Coating", DIVAN_POWDER_COATING_ID))),
        ("gems", Box::new(GemstonesModifier)),
        ("ability_scroll", Box::new(AbilityScrollModifier)),
        ("ethermerge", Box::new(EtherwarpConduitModifier)),
        ("power_ability_scroll", Box::new(PowerAbilityScrollModifier)),
        ("talisman_enrichment", Box::new(TalismanEnrichmentModifier)),
        ("hook", Box::new(RodHookModifier)),
        ("line", Box::new(RodLineModifier)),
        ("sinker", Box::new(RodSinkerModifier)),
        ("drill_part_engine", Box::new(DrillPartEngineModifier)),
        ("drill_part_upgrade_module", Box::new(DrillPartUpgradeModuleModifier)),
        ("drill_part_fuel_tank", Box::new(DrillPartFuelTankModifier)),
        ("boosters", Box::new(BoostersModifier)),
        ("skin", Box::new(SkinModifier)),
        ("dye_item", Box::new(DyeModifier)),
        ("petInfo", Box::new(PetModifier)),
    ]
});

pub async fn calculate_item_value(item_id: &str, item_nbt: &ItemNbt, include_prices: bool, include_cosmetics: bool) -> ItemValue {
    let mut item_value = ItemValue::new(include_prices, include_cosmetics);
    let Some(attributes) = item_nbt.get_extra_map() else { return item_value };
    let ctx = ModifierContext::new(item_id, item_nbt);

    if let Some(item_name) = get_item_name(item_nbt) {
        item_value.add_line(&format!("Item Name: {}", item_name));
    }

    if let Some(rarity) = get_item_rarity(item_nbt) {
        item_value.add_line(&format!("Rarity: {}", get_pretty_name(&rarity)));
    }

    let price = match get_base_price(item_id).await {
        None => match get_buy_price(item_id).await {
            None => get_cosmetic_price(item_id).await.unwrap_or_default(),
            Some(v) => v,
        },
        Some(v) => v,
    };

    item_value.set_base_value(price * item_nbt.count());

    if let Some(Value::String(raw_id)) = attributes.get("id") {
        if raw_id.contains("STARRED_") {
            if let Some(ingredient) = STARRED_ITEMS_INGREDIENT.get(raw_id) {
                let price = get_buy_price(ingredient).await;
                item_value.add("Starred: Yes", price, 8);
            }
        }
    }

    for (attr, handler) in MODIFIERS.iter() {
        if attributes.contains_key(*attr) {
            if let Some(attr_value) = attributes.get(*attr) {
                handler.calculate_value(&ctx, attr_value, &mut item_value).await;
            }
        }
    }

    item_value.add_line(&format!("Estimated Item Value: {}", format_number(item_value.value())));
    item_value
}

pub struct AbilityScrollModifier;

#[async_trait]
impl ModifierHandler for AbilityScrollModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(scrolls) = attr.as_list() else { return };

        if !scrolls.is_empty() {
            value.add_line("Ability Scrolls:");
            for scroll in scrolls {
                if let Some(id) = scroll.as_str() {
                    let price = get_buy_price(id).await;
                    value.add(&format!("- {}", get_pretty_name(id)), price, 1);
                }
            }
        }
    }
}

pub struct PotatoBooksModifier;

#[async_trait]
impl ModifierHandler for PotatoBooksModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(count) = attr.as_u64() else { return };

        let hot_potato_books = min(10, count);
        let fuming_books = count.saturating_sub(10);

        let price = get_buy_price(POTATO_BOOK_ID).await;
        value.add(&format!("Hot Potato Books: {}/10", hot_potato_books), price, hot_potato_books);

        if fuming_books > 0 {
            let price = get_buy_price(FUMING_BOOK_ID).await;
            value.add(&format!("Fuming Books: {}/5", fuming_books), price, fuming_books);
        }
    }
}

pub struct ReforgeModifier;

#[async_trait]
impl ModifierHandler for ReforgeModifier {
    async fn calculate_value(&self, ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(reforge) = attr.as_str() else { return };
        if reforge == "none" { return; };

        let price = match get_reforge_stone(reforge).await {
            None => None,
            Some(stone) => {
                let apply_cost = match get_item_rarity(ctx.item_nbt()) {
                    Some(rarity) => stone.apply_cost.get(&rarity).cloned().unwrap_or_default(),
                    None => 0,
                };
                get_buy_price(stone.id.as_str()).await.map(|p| p.add(apply_cost))
            }
        };

        value.add(&format!("Reforge: {}", get_pretty_name(reforge)), price, 1);
    }
}

pub struct EnchantmentsModifier;

#[async_trait]
impl ModifierHandler for EnchantmentsModifier {
    async fn calculate_value(&self, ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(enchantments) = attr.as_compound() else { return };

        let mut enchants_list = Vec::new();
        let mut enchants_value = 0;

        for (name, level) in enchantments {
            let Some(level) = level.as_u64() else { continue };
            let cool_name = &format!("{} {level}", get_pretty_name(&name.to_lowercase().replace("enchantment", "")));
            enchants_list.push(cool_name.to_string());

            if STACKING_ENCHANTS.contains(&name.as_str()) {
                let enchant_id = get_enchantment_id(name, 1);
                let price = get_buy_price(&enchant_id).await.unwrap_or(0);
                enchants_value += price;
                continue;
            }

            if let Some(price) = NPC_ENCHANTS.get(name) {
                enchants_value += *price;
                continue;
            }

            if let Some(required_item) = UPGRADABLE_ENCHANTS.get(&format!("{}_{}", name, level)) {
                let downgrade_id = get_enchantment_id(name, level - 1);
                let downgrade_price = get_buy_price(&downgrade_id).await.unwrap_or(0);
                let required_item_price = get_buy_price(*required_item).await.unwrap_or(0);

                enchants_value += downgrade_price + required_item_price;
                continue;
            }

            if name == "efficiency" && level > 5 {
                let &item_id = ctx.item_id();
                if item_id != STONK_PICKAXE_ID && item_id != PROMISING_SPADE_ID {
                    let price = get_buy_price(SILEX_ID).await.unwrap_or(0);
                    enchants_value += price * (level - 5);
                    continue;
                }
            }

            enchants_value += get_enchantment_price(name, level).await.unwrap_or(0);
        }

        if !enchants_list.is_empty() {
            value.add(&format!("Enchantments: [{}]", enchants_list.join(", ")), Some(enchants_value), 1);
        }
    }
}

pub async fn get_enchantment_price(enchant: &str, level: u64) -> Option<u64> {
    let enchant = enchant.to_uppercase();
    let id = get_enchantment_id(&enchant, level);
    let base_level = match () {
        _ if TIER_ONE_ENCHANTS.contains(&&*enchant) => 1,
        _ if TIER_THREE_ENCHANTS.contains(&&*enchant) => 3,
        _ if TIER_FIVE_ENCHANTS.contains(&&*enchant) => 5,
        _ => return get_buy_price(&id).await,
    };


    if level < base_level {
        return get_buy_price(&id).await;
    }

    let steps = level - base_level;
    let base_id = get_enchantment_id(&enchant, base_level);
    let base_price = get_buy_price(&base_id).await?;

    Some(base_price.saturating_mul(u64::pow(2, steps as u32)))
}

pub fn get_enchantment_id(enchant_name: &str, level: u64) -> String {
    format!("ENCHANTMENT_{}_{}", enchant_name.to_uppercase(), level)
}

pub struct EtherwarpConduitModifier;

#[async_trait]
impl ModifierHandler for EtherwarpConduitModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, _attr: &Value, value: &mut ItemValue) {
        let price = get_lowest_bin(ETHERWARP_CONDUIT_ID).await;
        value.add("Etherwarp Conduit: Applied", price, 1);
    }
}

pub struct PowerAbilityScrollModifier;

#[async_trait]
impl ModifierHandler for PowerAbilityScrollModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(scroll_id) = attr.as_str() else { return };
        let price = get_lowest_bin(scroll_id).await;
        value.add(&format!("{}: Applied", get_pretty_name(scroll_id)), price, 1);
    }
}

pub struct GemstonesModifier;

#[async_trait]
impl ModifierHandler for GemstonesModifier {
    async fn calculate_value(&self, ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(gems) = attr.as_compound() else { return };
        let mut gemstones: HashMap<String, u64> = HashMap::new();
        let mut unlocked_slots = Vec::new();

        for (k, v) in gems {
            if k == "unlocked_slots" {
                unlocked_slots = v
                    .as_list()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|c| c.as_str().map(|s| s.to_owned()))
                            .collect()
                    }).unwrap_or_default();
                continue;
            }
            if k.ends_with("_gem") {
                let base_key = &k[..k.len() - 4];
                if let Some(base_val) = gems.get(base_key) {
                    let gem = extract_gemstone_field(v, "gem").or_else(|| extract_gemstone_field(v, ""));
                    let quality = extract_gemstone_field(base_val, "quality")
                        .or_else(|| extract_gemstone_field(base_val, ""));
                    if let (Some(quality), Some(gem)) = (quality, gem) {
                        let key = format!("{}_{}_GEM", quality, gem);
                        *gemstones.entry(key).or_insert(0) += 1;
                    }
                }
            } else if let Some(pos) = k.find('_') {
                let gem_name = &k[..pos];
                if GEMSTONES.contains(&gem_name) {
                    let quality = extract_gemstone_field(v, "quality")
                        .or_else(|| extract_gemstone_field(v, ""));
                    if let Some(quality) = quality {
                        let key = format!("{}_{}_GEM", quality, gem_name);
                        *gemstones.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }

        if !gemstones.is_empty() {
            value.add_line("Gemstones Applied:");
            for (gem, count) in gemstones {
                let gem_name = get_pretty_name(&gem.replace("_GEM", "_GEMSTONE"));
                let price = get_buy_price(&gem).await;
                value.add(&format!("- {}x {}", count, gem_name), price, count);
            }
        }

        if !unlocked_slots.is_empty() && let Some(item_gems) = get_item_gemstone_slots(ctx.item_id()).await {
            value.add_line("Unlocked Gemstones Slots:");
            for slot in unlocked_slots.iter() {
                if let Some(slot_cost) = item_gems.get(slot) {
                    let mut items_cost = 0;
                    for item in slot_cost {
                        let mut parts = item.splitn(2, ':');
                        if let (Some(item), Some(count)) = (parts.next(), parts.next()) {
                            let count: u64 = count.parse().unwrap_or(1);
                            let price = match item == "SKYBLOCK_COIN" {
                                true => 1,
                                false => match get_buy_price(item).await {
                                    Some(p) => p,
                                    None => get_lowest_bin(item).await.unwrap_or(0),
                                },
                            };
                            items_cost += price * count;
                        }
                    }
                    value.add(&format!("- {}", get_pretty_name(slot)), Some(items_cost), 1);
                }
            }
        }
    }
}

fn extract_gemstone_field<'a>(val: &'a Value, field: &str) -> Option<&'a str> {
    match val {
        Value::String(s) if field.is_empty() => Some(s.as_str()),
        Value::Compound(map) => match map.get(field) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

pub struct TalismanEnrichmentModifier;

#[async_trait]
impl ModifierHandler for TalismanEnrichmentModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(enrichment) = attr.as_str() else { return };

        let id = format!("TALISMAN_ENRICHMENT_{}", enrichment.to_uppercase());
        let price = get_lowest_bin(&id).await;
        value.add(&format!("{}: Applied", get_pretty_name(&id)), price, 1);
    }
}

pub struct RodHookModifier;

#[async_trait]
impl ModifierHandler for RodHookModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(hook) = attr.as_compound() else { return };

        if let Some(Value::String(part)) = hook.get("part") {
            let id = &part.to_uppercase();
            let price = get_lowest_bin(id).await;
            value.add(&format!("Rod Hook: {}", get_pretty_name(id)), price, 1);
        }
    }
}

pub struct RodLineModifier;

#[async_trait]
impl ModifierHandler for RodLineModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(line) = attr.as_compound() else { return };

        if let Some(Value::String(part)) = line.get("part") {
            let id = &part.to_uppercase();
            let price = get_lowest_bin(id).await;
            value.add(&format!("Rod Line: {}", get_pretty_name(id)), price, 1);
        }
    }
}

pub struct RodSinkerModifier;

#[async_trait]
impl ModifierHandler for RodSinkerModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(sinker) = attr.as_compound() else { return };

        if let Some(Value::String(part)) = sinker.get("part") {
            let id = &part.to_uppercase();
            let price = get_lowest_bin(id).await;
            value.add(&format!("Rod Sinker: {}", get_pretty_name(id)), price, 1);
        }
    }
}

pub struct DrillPartEngineModifier;

#[async_trait]
impl ModifierHandler for DrillPartEngineModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(part) = attr.as_str() else { return };

        let id = part.to_uppercase();
        let price = get_lowest_bin(&id).await;
        value.add(&format!("Drill Engine: {}", get_pretty_name(&id)), price, 1);
    }
}

pub struct DrillPartUpgradeModuleModifier;

#[async_trait]
impl ModifierHandler for DrillPartUpgradeModuleModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(part) = attr.as_str() else { return };

        let id = part.to_uppercase();
        let price = get_lowest_bin(&id).await;
        value.add(&format!("Drill Upgrade Module: {}", get_pretty_name(&id)), price, 1);
    }
}

pub struct DrillPartFuelTankModifier;

#[async_trait]
impl ModifierHandler for DrillPartFuelTankModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(part) = attr.as_str() else { return };

        let id = part.to_uppercase();
        let price = get_lowest_bin(&id).await;
        value.add(&format!("Drill Fuel Tank: {}", get_pretty_name(&id)), price, 1);
    }
}

pub struct BoostersModifier;

#[async_trait]
impl ModifierHandler for BoostersModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(boosters) = attr.as_list() else { return };

        if !boosters.is_empty() {
            value.add_line("Boosters:");
            for booster in boosters {
                if let Some(booster) = booster.as_str() {
                    let id = format!("{}_BOOSTER", booster.to_uppercase());
                    let price = get_buy_price(&id).await;
                    value.add(&format!("- {}", get_pretty_name(&id)), price, 1);
                }
            }
        }
    }
}

pub struct SkinModifier;

#[async_trait]
impl ModifierHandler for SkinModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(skin) = attr.as_str() else { return };
        let price = get_lowest_bin(skin).await;
        value.add(&format!("Skin: {}", &get_pretty_name(skin)), price, 1);
    }
}

pub struct DyeModifier;

#[async_trait]
impl ModifierHandler for DyeModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(dye) = attr.as_str() else { return };
        let price = get_lowest_bin(dye).await;
        let dye_name = dye.replace("DYE_", "");
        value.add_cosmetic(&format!("Dye: {}", get_pretty_name(&dye_name)), price);
    }
}

pub struct UpgradeLevelModifier;

#[async_trait]
impl ModifierHandler for UpgradeLevelModifier {
    async fn calculate_value(&self, ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(level) = attr.as_u64() else { return };
        let Some(item_upgrade_costs) = get_essence_costs(ctx.item_id()).await else { return };

        let max_stars = item_upgrade_costs.stars().len() as u64;
        let essence_id = format!("ESSENCE_{}", item_upgrade_costs.essence_type().to_uppercase());

        let regular_stars = min(max_stars, level);
        let master_stars = level.saturating_sub(max_stars);

        let mut essence_amount = 0;
        let mut items_cost: HashMap<&str, u64> = HashMap::new();

        for (&count, star) in item_upgrade_costs.stars() {
            if count <= regular_stars {
                essence_amount += *star.essence();
                for (item, amount) in star.items() {
                    *items_cost.entry(item).or_insert(0) += *amount;
                }
            }
        }

        if let Some(dungeonize_cost) = item_upgrade_costs.dungeonize_cost() {
            if ctx.item_nbt().get_extra_map().and_then(|m| m.get("dungeon_item")).is_some() {
                essence_amount += dungeonize_cost;
            }
        }

        items_cost.insert(&essence_id, essence_amount);

        let mut stars_cost = 0;
        for (item, count) in items_cost {
            stars_cost += get_buy_price(item).await.unwrap_or(0) * count;
        }

        value.add(&format!("Stars: {regular_stars}/{max_stars}"), Some(stars_cost), 1);

        if master_stars > 0 {
            let mut master_stars_cost = 0;
            for (star, id) in MASTER_STARS.iter().enumerate() {
                if star < master_stars as usize {
                    master_stars_cost += get_buy_price(id).await.unwrap_or(0);
                }
            }
            value.add(&format!("Master Stars: {master_stars}/5"), Some(master_stars_cost), 1);
        }
    }
}

pub struct PetModifier;

#[async_trait]
impl ModifierHandler for PetModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(pet_info) = attr.as_str() else { return };

        let pet_data: serde_json::Value = match serde_json::from_str(pet_info) {
            Ok(data) => data,
            Err(_) => return,
        };

        if let Some(pet) = get_pet_obj(&pet_data) {
            get_pet_value(&pet, value).await;
        }
    }
}

pub async fn get_pet_value(pet: &Pet, value: &mut ItemValue) {
    if let Some(pet_info) = get_pet_info(pet) {
        value.set_base_value(0);

        let price = get_pet_networth(pet).await;
        match pet.skin() {
            Some(skin) => value.add(&format!("Pet: {pet_info}, Skin: {}", get_pretty_name(skin)), Some(price), 1),
            None => value.add(&format!("Pet: {pet_info}"), Some(price), 1),
        }

        if let Some(held_item) = pet.held_item() {
            let price = get_lowest_bin(held_item).await;
            value.add(&format!("Pet Item: {}", get_pretty_name(held_item)), price, 1);
        }
    }
}

pub async fn get_pet_networth(pet: &Pet) -> u64 {
    let (level, _) = get_pet_level(pet.name(), pet.tier(), *pet.xp() as u64);
    let level = match level {
        0..100 => 1,
        100..200 => 100,
        _ => level,
    };
    let id = format!("LVL_{level}_{}_{}", pet.tier(), pet.name());
    let base_id = format!("{}_{}", pet.tier(), pet.name());

    if let Some(skin) = pet.skin() {
        let id_with_skin = format!("{id}_SKINNED_{skin}");
        if let Some(price) = get_cosmetic_price(&id_with_skin).await {
            return price;
        }

        let mut pet_value = 0;
        pet_value += match get_cosmetic_price(&id).await {
            None => get_lowest_bin(&base_id).await.unwrap_or(0),
            Some(price) => price,
        };

        let skin_id = format!("PET_SKIN_{skin}");
        pet_value += match get_cosmetic_price(&skin_id).await {
            None => get_lowest_bin(&skin_id).await.unwrap_or(0),
            Some(price) => price,
        };

        return pet_value;
    }

    if let Some(price) = get_cosmetic_price(&id).await {
        return price;
    }

    get_lowest_bin(&base_id).await.unwrap_or(0)
}
