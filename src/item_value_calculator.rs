use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::Deref;
use std::sync::Arc;
use async_trait::async_trait;
use fastnbt::Value;
use once_cell::sync::Lazy;
use sea_orm::Iden;
use tokio::sync::RwLock;
use crate::modifiers::ability_scroll::AbilityScrollModifier;
use crate::modifiers::art_of_peace::ArtOfPieceModifier;
use crate::modifiers::art_of_war::ArtOfWarModifier;
use crate::modifiers::book_of_stats::BookOfStatsModifier;
use crate::modifiers::boosters::BoostersModifier;
use crate::modifiers::divan_powder_coating::DivanPowderCoatingModifier;
use crate::modifiers::drill_part_engine::DrillPartEngineModifier;
use crate::modifiers::drill_part_upgrade_module::DrillPartUpgradeModuleModifier;
use crate::modifiers::dye::DyeModifier;
use crate::modifiers::enchantments::EnchantmentsModifier;
use crate::modifiers::etherwarp_conduit::EtherwarpConduitModifier;
use crate::modifiers::farming_for_dummies::FarmingForDummiesModifier;
use crate::modifiers::gemstones::GemstonesModifier;
use crate::modifiers::jalapeno_book::JalapenoBookModifier;
use crate::modifiers::mana_disintegrator::ManaDisintegratorModifier;
use crate::modifiers::pet::PetModifier;
use crate::modifiers::polarvoid::PolarvoidModifier;
use crate::modifiers::potato_books::PotatoBooksModifier;
use crate::modifiers::power_ability_scroll::PowerAbilityScrollModifier;
use crate::modifiers::recombobulator::RecombobulatorModifier;
use crate::modifiers::reforge::ReforgeModifier;
use crate::modifiers::rod_hook::RodHookModifier;
use crate::modifiers::rod_sinker::RodSinkerModifier;
use crate::modifiers::skin::SkinModifier;
use crate::modifiers::talisman_enrichment::TalismanEnrichmentModifier;
use crate::modifiers::transmission_tuner::TransmissionTunerModifier;
use crate::modifiers::upgrade_level::UpgradeLevelModifier;
use crate::modifiers::wet_book::WetBookModifier;
use crate::modifiers::wood_singularity_count::WoodSingularityModifier;
use crate::structs::{Auction, AuctionItem, ItemNbt, ItemValue, PriceData, PriceDataSource, SharedPriceData};

#[async_trait]
pub trait ModifierHandler: Send + Sync {
    async fn calculate_value(&self, item_id: &str, modifier: &Value, modifiers: &mut ItemValue) -> bool;
}

pub static MODIFIERS: Lazy<HashMap<&'static str, Box<dyn ModifierHandler>>> = Lazy::new(|| {
    let mut map: HashMap<&'static str, Box<dyn ModifierHandler>> = HashMap::new();
    map.insert("modifier", Box::new(ReforgeModifier));
    map.insert("enchantments", Box::new(EnchantmentsModifier));
    map.insert("hot_potato_count", Box::new(PotatoBooksModifier));
    map.insert("rarity_upgrades", Box::new(RecombobulatorModifier));
    map.insert("artOfPeaceApplied", Box::new(ArtOfPieceModifier));
    map.insert("art_of_war_count", Box::new(ArtOfWarModifier));
    map.insert("tuned_transmission", Box::new(TransmissionTunerModifier));
    map.insert("ethermerge", Box::new(EtherwarpConduitModifier));
    map.insert("jalapeno_count", Box::new(JalapenoBookModifier));
    map.insert("mana_disintegrator_count", Box::new(ManaDisintegratorModifier));
    map.insert("stats_book", Box::new(BookOfStatsModifier));
    map.insert("wet_book_count", Box::new(WetBookModifier));
    map.insert("farming_for_dummies_count", Box::new(FarmingForDummiesModifier));
    map.insert("power_ability_scroll", Box::new(PowerAbilityScrollModifier));
    map.insert("gems", Box::new(GemstonesModifier));
    map.insert("talisman_enrichment", Box::new(TalismanEnrichmentModifier));
    map.insert("ability_scroll", Box::new(AbilityScrollModifier));
    map.insert("sinker", Box::new(RodSinkerModifier));
    map.insert("hook", Box::new(RodHookModifier));
    map.insert("polarvoid", Box::new(PolarvoidModifier));
    map.insert("divan_powder_coating", Box::new(DivanPowderCoatingModifier));
    map.insert("drill_part_engine", Box::new(DrillPartEngineModifier));
    map.insert("drill_part_upgrade_module", Box::new(DrillPartUpgradeModuleModifier));
    map.insert("boosters", Box::new(BoostersModifier));
    map.insert("skin", Box::new(SkinModifier));
    map.insert("dye_item", Box::new(DyeModifier));
    map.insert("wood_singularity_count", Box::new(WoodSingularityModifier));
    map.insert("upgrade_level", Box::new(UpgradeLevelModifier));
    map.insert("petInfo", Box::new(PetModifier));
    map
});

pub async fn calculate_auction_value(auction: &mut AuctionItem) {
    let modifiers_to_process = auction.value().modifiers_to_process().clone();
    if modifiers_to_process.is_empty() {
        return;
    }

    let item_id = auction.item_id().to_string();
    let Some(Value::Compound(attributes)) = auction.item_nbt().tag.as_ref().and_then(|tag| tag.extra_attributes.as_ref()).cloned() else { return; };

    for modifier_key in modifiers_to_process {
        if let (Some(handler), Some(attr_value)) = (MODIFIERS.get(modifier_key.as_str()), attributes.get(&modifier_key)) {
            let value = auction.value_mut();
            if handler.calculate_value(item_id.as_str(), attr_value, value).await {
                value.modifiers_to_process_mut().remove(&modifier_key);
            }
        }
    }
}