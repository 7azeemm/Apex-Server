use crate::constants::enchantments::{NPC_ENCHANTS, STACKING_ENCHANTS, TIER_FIVE_ENCHANTS, TIER_ONE_ENCHANTS, TIER_THREE_ENCHANTS, UPGRADABLE_ENCHANTS};
use crate::constants::misc::{GEMSTONES, MASTER_STARS};
use crate::constants::reforges::{EXCLUDE_REFORGES, NPC_REFORGES, REFORGES_APPLY_COST, REFORGE_STONES};
use crate::extensions::fastnbt_ext::ValueExt;
use crate::item_utils::{get_item_name, get_item_rarity, get_pet_info, get_pet_obj, get_pretty_name, get_rarity_index};
use crate::prices::auctions::{get_base_price, get_lowest_bin};
use crate::prices::bazaar::get_buy_price;
use crate::prices::cosmetic_prices::{get_cosmetic_price, get_pet_networth};
use crate::repos::neu::essence_costs::get_essence_costs;
use crate::repos::neu::gemstone_slots_cost::get_item_gemstone_slots;
use crate::structs::item_structs::{ItemNbt, ItemValue, ModifierContext};
use crate::structs::player_data_structs::Pet;
use crate::structs::value_calc_structs::{CountedItemModifier, ModifierHandler, SingleItemModifier};
use crate::utils::format_number;
use async_trait::async_trait;
use fastnbt::Value;
use once_cell::sync::Lazy;
use std::cmp::{max, min};
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
        ("tuned_transmission", Box::new(CountedItemModifier::new("Transmission Tuners", TRANSMISSION_TUNER_ID, Some(4)))),
        ("jalapeno_count", Box::new(SingleItemModifier::new("Jalapeno Book", JALAPENO_BOOK_ID))),
        ("mana_disintegrator_count", Box::new(CountedItemModifier::new("Mana Disintegrators", MANA_DISINTEGRATOR_ID, Some(10)))),
        ("stats_book", Box::new(SingleItemModifier::new("Stats Book", BOOK_OF_STATS_ID))),
        ("wet_book_count", Box::new(CountedItemModifier::new("Wet Book", WET_BOOK_ID, Some(5)))),
        ("farming_for_dummies_count", Box::new(CountedItemModifier::new("Farming For Dummies", FARMING_FOR_DUMMIES_ID, Some(5)))),
        ("wood_singularity_count", Box::new(SingleItemModifier::new("Wood Singularity", WOOD_SINGULARITY_ID))),
        ("polarvoid", Box::new(CountedItemModifier::new("Polarvoid Books", POLARVOID_BOOK_ID, Some(5)))),
        ("divan_powder_coating", Box::new(SingleItemModifier::new("Divan Powder Coating", DIVAN_POWDER_COATING_ID))),
        ("gems", Box::new(GemstonesModifier)),
        ("ability_scroll", Box::new(AbilityScrollModifier)),
        ("ethermerge", Box::new(EtherwarpConduitModifier)),
        ("power_ability_scroll", Box::new(PowerAbilityScrollModifier)),
        ("talisman_enrichment", Box::new(TalismanEnrichmentModifier)),
        ("sinker", Box::new(RodSinkerModifier)),
        ("hook", Box::new(RodHookModifier)),
        ("drill_part_engine", Box::new(DrillPartEngineModifier)),
        ("drill_part_upgrade_module", Box::new(DrillPartUpgradeModuleModifier)),
        ("drill_part_fuel_tank", Box::new(DrillPartFuelTankModifier)),
        ("boosters", Box::new(BoostersModifier)),
        ("skin", Box::new(SkinModifier)),
        ("dye_item", Box::new(DyeModifier)),
        ("petInfo", Box::new(PetModifier)),
    ]
});

pub async fn calculate_item_value(item_id: &str, item_nbt: &ItemNbt) -> ItemValue {
    let mut item_value = ItemValue::default();
    let Some(attributes) = item_nbt.get_extra_map() else { return item_value };
    let ctx = ModifierContext::new(item_id, item_nbt);

    if let Some(item_name) = get_item_name(item_nbt) {
        item_value.add(&format!("Item Name: {}", item_name));
    }

    if let Some(rarity) = get_item_rarity(item_nbt) {
        item_value.add(&format!("Rarity: {}", get_pretty_name(&rarity)));
    }

    let price = match get_base_price(item_id).await {
        None => match get_buy_price(item_id).await {
            None => get_cosmetic_price(item_id).await.unwrap_or(0),
            Some(p) => p
        }
        Some(p) => p
    };
    item_value.set_base_value(price * item_nbt.count());

    for (attr, handler) in MODIFIERS.iter() {
        if attributes.contains_key(*attr) {
            if let Some(attr_value) = attributes.get(*attr) {
                handler.calculate_value(&ctx, attr_value, &mut item_value).await;
            }
        }
    }

    item_value.add(&format!("Estimated Item Value: {}", format_number(item_value.value())));
    item_value
}

pub struct AbilityScrollModifier;

#[async_trait]
impl ModifierHandler for AbilityScrollModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(scrolls) = attr.as_list() else { return };

        if !scrolls.is_empty() {
            value.add("Ability Scrolls:");
            for scroll in scrolls {
                if let Some(id) = scroll.as_str() {
                    let price = get_buy_price(&id).await;
                    value.add_v(&format!(" - {}", get_pretty_name(id)), price, 1);
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
        let fuming_books = max(0, count - 10);

        let price = get_buy_price(POTATO_BOOK_ID).await;
        value.add_v(&format!("Hot Potato Books: {}/10", hot_potato_books), price, hot_potato_books);

        if fuming_books > 0 {
            let price = get_buy_price(FUMING_BOOK_ID).await;
            value.add_v(&format!("Fuming Books: {}/5", fuming_books), price, fuming_books);
        }
    }
}

pub struct ReforgeModifier;

#[async_trait]
impl ModifierHandler for ReforgeModifier {
    async fn calculate_value(&self, ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(reforge) = attr.as_str() else { return };
        if reforge == "none" { return; };

        let reforge_price = get_reforge_stone_price(reforge, ctx.item_nbt()).await;
        value.add_v(&format!("Reforge: {}", get_pretty_name(reforge)), reforge_price, 1);
    }
}

async fn get_reforge_stone_price(reforge: &str, item_nbt: &ItemNbt) -> Option<u64> {
    if EXCLUDE_REFORGES.contains(&reforge) { return None; }

    if let Some(stone_id) = REFORGE_STONES.get(reforge) {
        let apply_cost = get_apply_cost(stone_id, item_nbt).unwrap_or(0);
        return get_buy_price(stone_id).await.map(|p| p.add(apply_cost));
    }

    NPC_REFORGES.get(reforge).map(|p| *p)
}

fn get_apply_cost(reforge_id: &str, item_nbt: &ItemNbt) -> Option<u64> {
    let cost_list = REFORGES_APPLY_COST.get(reforge_id)?;
    let item_rarity = get_item_rarity(item_nbt)?;
    let rarity_index = get_rarity_index(&item_rarity)?;
    cost_list.get(rarity_index).map(|v| *v)
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
            let cool_name = &format!("{} {level}", get_pretty_name(name));
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
                let required_item_price = get_buy_price(&*required_item).await.unwrap_or(0);

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

        value.add_v(&format!("Enchantments: [{}]", enchants_list.join(", ")), Some(enchants_value), 1);
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

    let steps = level - base_level;
    let base_id = get_enchantment_id(&*enchant, base_level);
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
        value.add_v("Etherwarp Conduit: Applied", price, 1);
    }
}

pub struct PowerAbilityScrollModifier;

#[async_trait]
impl ModifierHandler for PowerAbilityScrollModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(scroll_id) = attr.as_str() else { return };
        let price = get_lowest_bin(scroll_id).await;
        value.add_v(&format!("{}: Applied", get_pretty_name(scroll_id)), price, 1);
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
                unlocked_slots = v.as_list()
                    .map(|arr| arr.iter().filter_map(|c| c.as_str().map(|s| s.to_owned())).collect())
                    .unwrap_or_default();
                continue;
            }
            if k.ends_with("_gem") {
                let base_key = &k[..k.len() - 4];
                if let Some(base_val) = gems.get(base_key) {
                    let gem = extract_gemstone_field(v, "gem").or_else(|| extract_gemstone_field(v, ""));
                    let quality = extract_gemstone_field(base_val, "quality").or_else(|| extract_gemstone_field(base_val, ""));
                    if let (Some(quality), Some(gem)) = (quality, gem) {
                        let key = format!("{}_{}_GEM", quality, gem);
                        *gemstones.entry(key).or_insert(0) += 1;
                    }
                }
            } else if let Some(pos) = k.find('_') {
                let gem_name = &k[..pos];
                if GEMSTONES.contains(&gem_name) {
                    let quality = extract_gemstone_field(v, "quality").or_else(|| extract_gemstone_field(v, ""));
                    if let Some(quality) = quality {
                        let key = format!("{}_{}_GEM", quality, gem_name);
                        *gemstones.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut gemstones_list = Vec::new();
        let mut gemstones_value = 0;

        for (gem, count) in gemstones {
            let price = get_buy_price(&gem).await.unwrap_or(0);
            gemstones_value += price;

            let gem_name = get_pretty_name(&*gem.replace("_GEM", "_GEMSTONE"));
            gemstones_list.push(format!("{}x {}", count, gem_name));
        }

        if !unlocked_slots.is_empty() {
            if let Some(item_gems) = get_item_gemstone_slots(ctx.item_id()).await {
                for slot in unlocked_slots.iter() {
                    if let Some(unlocked_slot) = item_gems.get(slot) {
                        for cost in unlocked_slot {
                            let mut parts = cost.splitn(2, ':');
                            if let (Some(item), Some(count)) = (parts.next(), parts.next()) {
                                let count: u64 = count.parse().unwrap_or(0);
                                gemstones_value += match item {
                                    "SKYBLOCK_COIN" => count,
                                    _ => get_buy_price(item).await.unwrap_or(get_lowest_bin(item).await.unwrap_or(0))
                                };
                            }
                        }
                    }
                }
            }
        }

        if !gemstones_list.is_empty() {
            value.add(&format!("Gemstones Applied: [{}]", gemstones_list.join(", ")))
        }

        if !unlocked_slots.is_empty() {
            let unlocked_slots: Vec<String> = unlocked_slots.iter().map(|s| get_pretty_name(s)).collect();
            value.add(&format!("Unlocked Gemstones Slots: [{}]", unlocked_slots.join(", ")))
        }
    }
}

fn extract_gemstone_field<'a>(val: &'a Value, field: &str) -> Option<&'a str> {
    match val {
        Value::String(s) if field == "" => Some(s.as_str()),
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
        value.add_v(&format!("{}: Applied", get_pretty_name(&id)), price, 1);
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
            value.add_v(&format!("Rod Sinker: {}", get_pretty_name(id)), price, 1);
        }
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
            value.add_v(&format!("Rod Hook: {}", get_pretty_name(id)), price, 1);
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
        value.add_v(&format!("Drill Engine: {}", get_pretty_name(&id)), price, 1);
    }
}

pub struct DrillPartUpgradeModuleModifier;

#[async_trait]
impl ModifierHandler for DrillPartUpgradeModuleModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(part) = attr.as_str() else { return };

        let id = part.to_uppercase();
        let price = get_lowest_bin(&id).await;
        value.add_v(&format!("Drill Upgrade Module: {}", get_pretty_name(&id)), price, 1);
    }
}

pub struct DrillPartFuelTankModifier;

#[async_trait]
impl ModifierHandler for DrillPartFuelTankModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(part) = attr.as_str() else { return };

        let id = part.to_uppercase();
        let price = get_lowest_bin(&id).await;
        value.add_v(&format!("Drill Fuel Tank: {}", get_pretty_name(&id)), price, 1);
    }
}

pub struct BoostersModifier;

#[async_trait]
impl ModifierHandler for BoostersModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(boosters) = attr.as_list() else { return };

        if !boosters.is_empty() {
            value.add("Boosters:");
            for booster in boosters {
                if let Some(booster) = booster.as_str() {
                    let id = format!("{}_BOOSTER", booster.to_uppercase());
                    let price = get_buy_price(&id).await;
                    value.add_v(&format!(" - {}", get_pretty_name(&id)), price, 1);
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
        value.add_v(&format!("Skin: {}", &get_pretty_name(skin)), price, 1);
    }
}

pub struct DyeModifier;

#[async_trait]
impl ModifierHandler for DyeModifier {
    async fn calculate_value(&self, _ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(dye) = attr.as_str() else { return };
        let price = get_lowest_bin(dye).await;
        value.add_v(&format!("Dye: {}", &get_pretty_name(&*dye.replace("DYE_", ""))), price, 1);
    }
}

pub struct UpgradeLevelModifier;

#[async_trait]
impl ModifierHandler for UpgradeLevelModifier {
    async fn calculate_value(&self, ctx: &ModifierContext<'_>, attr: &Value, value: &mut ItemValue) {
        let Some(level) = attr.as_u64() else { return };
        let Some(item_upgrade_costs) = get_essence_costs(ctx.item_id()).await else { return };

        let max_stars = item_upgrade_costs.stars.len() as u64;
        let essence_id = format!("ESSENCE_{}", item_upgrade_costs.essence_type.to_uppercase());

        let regular_stars = min(max_stars, level);
        let master_stars = level.saturating_sub(max_stars);

        let mut essence_amount = 0;
        let mut items_cost: HashMap<&str, u64> = HashMap::new();

        for (count, star) in item_upgrade_costs.stars.iter() {
            if *count <= regular_stars {
                essence_amount += star.essence;
                for (item, amount) in star.items.iter() {
                    *items_cost.entry(item).or_insert(0) += amount;
                }
            }
        }

        for (star, id) in MASTER_STARS.iter().enumerate() {
            if star + 1 <= master_stars as usize {
                items_cost.insert(id, 1);
            }
        }

        if is_dungeon_item(ctx.item_nbt()) && let Some(dungeonize_cost) = item_upgrade_costs.dungeonize_cost {
            essence_amount += dungeonize_cost;
        }

        let mut stars_value = 0;
        stars_value += get_buy_price(&essence_id).await.unwrap_or(0);

        for (item, count) in items_cost {
            stars_value += get_buy_price(item).await.unwrap_or(0) * count;
        }

        value.add_v(&format!("Stars: {regular_stars}/{max_stars}"), Some(stars_value), 1);
        if master_stars > 0 {
            value.add(&format!("Master Stars: {master_stars}/5"))
        }
    }
}

fn is_dungeon_item(item_nbt: &ItemNbt) -> bool {
    item_nbt.get_extra_map().and_then(|m| m.get("dungeon_item")).is_some()
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
            let (pet_info, pet_value) = get_pet_full_info(&pet).await;
            value.add_value(pet_value);
            for line in pet_info {
                value.add(&line);
            }
        }
    }
}

pub async fn get_pet_full_info(pet: &Pet) -> (Vec<String>, u64) {
    let mut vec = Vec::new();
    let mut value = 0;

    if let Some(pet_info) = get_pet_info(&pet) {
        let price = get_pet_networth(&pet).await;
        vec.push(format!("Pet: {pet_info}"));
        value += price;

        if let Some(skin) = pet.skin() {
            // Skin price is included in pet networth above
            vec.push(format!("Pet Skin: {}", get_pretty_name(skin)));
        }

        if let Some(held_item) = pet.held_item() {
            let price = get_lowest_bin(held_item).await.unwrap_or(0);
            value += price;
            vec.push(format!("Pet Item: {}", get_pretty_name(held_item)));
        }
    }

    (vec, value)
}