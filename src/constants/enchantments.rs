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

pub static NPC_ENCHANTS: Map<&'static str, f64> = phf_map! {
    "great_spook" => 30000.0,
};

pub static UPGRADABLE_ENCHANTS: Map<&'static str, &'static str> = phf_map! {
    "pesterminator_6" => "PESTHUNTING_GUIDE",
    "charm_6" => "CHAIN_END_TIMES",
    "scavenger_6" => "GOLDEN_BOUNTY",
    "piscary_7" => "TROUBLED_BUBBLE",
    "frail_7" => "SEVERED_PINCER",
    "spiked_hook_7" => "OCTOPUS_TENDRIL",
    "luck_of_the_sea_7" => "GOLD_BOTTLE_CAP"
};