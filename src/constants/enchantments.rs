use phf::{phf_map, phf_set, Map, Set};

pub static STACKING_ENCHANTS: Set<&'static str> = phf_set! {
    "champion",
    "compact",
    "cultivating",
    "expertise",
    "hecatomb",
    "toxophilite",
    "absorb"
};

pub static NPC_ENCHANTS: Map<&'static str, u64> = phf_map! {
    "great_spook" => 30000,
};

pub static UPGRADABLE_ENCHANTS: Map<&'static str, &'static str> = phf_map! {
    "pesterminator_6" => "PESTHUNTING_GUIDE",
    "charm_6" => "CHAIN_END_TIMES",
    "scavenger_6" => "GOLDEN_BOUNTY",
    "piscary_7" => "TROUBLED_BUBBLE",
    "frail_7" => "SEVERED_PINCER",
    "spiked_hook_7" => "OCTOPUS_TENDRIL",
    "luck_of_the_sea_7" => "GOLD_BOTTLE_CAP",
    "ender_slayer_7" => "ENDSTONE_IDOL",
    "smite_7" => "SEVERED_HAND",
    "bane_of_arthropods_7" => "ENSNARED"
};

pub static TIER_ONE_ENCHANTS: &[&str] = &["CHARM", "DIVINE_GIFT", "CORRUPTION", "GREEN_THUMB", "ICE_COLD", "LAPIDARY",
    "OVERLOAD", "PALEONTOLOGIST", "PRISTINE", "SCUBA", "SMARTY_PANTS", "SMOLDERING", "TIDAL", "ULTIMATE_CHIMERA",
    "ULTIMATE_REITERATE", "ULTIMATE_FATAL_TEMPO", "ULTIMATE_FLASH", "ULTIMATE_INFERNO", "ULTIMATE_REFRIGERATE",
    "ULTIMATE_REND", "ULTIMATE_LEGION", "ULTIMATE_SOUL_EATER", "ULTIMATE_SWARM", "ULTIMATE_WISE", "ULTIMATE_WISDOM"
];

pub static TIER_THREE_ENCHANTS: &[&str] = &["ULTIMATE_BOBBIN_TIME", "BIG_BRAIN", "COUNTER_STRIKE", "FOREST_PLEDGE"];

pub static TIER_FIVE_ENCHANTS: &[&str] = &["FEROCIOUS_MANA", "HARDENED_MANA", "MANA_VAMPIRE", "STRONG_MANA"];