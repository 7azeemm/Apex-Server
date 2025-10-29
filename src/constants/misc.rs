use phf::{phf_map, Map};

pub const RARITIES: &[&str] = &["COMMON", "UNCOMMON", "RARE", "EPIC", "LEGENDARY", "MYTHIC", "DIVINE", "SPECIAL", "VERY SPECIAL", "ULTIMATE COSMETIC", "LEGENJERRY COSMETIC"];
pub const GEMSTONES: &[&str] = &[
    "JADE", "ONYX", "AMBER", "RUBY", "SAPPHIRE", "AMETHYST", "JASPER", "TOPAZ",
    "PERIDOT", "AQUAMARINE", "CITRINE", "OPAL"
];

pub const MAX_FAIRY_SOULS: i16 = 266;
pub const MAX_BESTIARY_LEVEL: u16 = 376;
pub const MAX_ENIGMA_SOULS: i8 = 52;
pub const MAX_TIMECHARMS: i8 = 8;
pub const MAX_MINING_COMMISSION_MILESTONE: i8 = 6;

pub const SKYBLOCK_YEAR_TO_REAL_HOURS: i8 = 124;

pub const MASTER_STARS: &[&str] = &[
    "FIRST_MASTER_STAR",
    "SECOND_MASTER_STAR",
    "THIRD_MASTER_STAR",
    "FOURTH_MASTER_STAR",
    "FIFTH_MASTER_STAR"
];

pub const TROPHY_FISHING_TIERS: &[&str] = &[
    "None",
    "Novice Trophy Fisher (Caught all Bronze Trophy Fishes)",
    "Adept Trophy Fisher (Caught all Silver Trophy Fishes)",
    "Expert Trophy Fisher (Caught all Gold Trophy Fishes)",
    "Master Trophy Fisher (Caught all Diamond Trophy Fishes)"
];

pub const TROPHY_FISHES: &[&str] = &[
    "sulphur_skitter", "blobfish", "obfuscated_fish_1", "steaming_hot_flounder",
    "gusher", "obfuscated_fish_2", "slugfish", "flyfish", "obfuscated_fish_3", "vanille", "lava_horse", "mana_ray",
    "volcanic_stonefish", "skeleton_fish", "moldfin", "soul_fish", "karate_fish", "golden_fish"
];

pub const SLAYER_XP_REQUIRED: Map<&'static str, &[u64]> = phf_map! {
    "zombie" => &[0, 5, 15, 200, 1000, 5000, 20000, 100000, 400000, 1000000],
    "spider" => &[0, 10, 25, 200, 1000, 5000, 20000, 100000, 400000, 1000000],
    "wolf" | "enderman" | "blaze" => &[0, 10, 30, 250, 1500, 5000, 20000, 100000, 400000, 1000000],
    "vampire" => &[0, 20, 75, 240, 840, 2400],
};

pub const ISLAND_NAMES: Map<&'static str, &'static str> = phf_map! {
    "dynamic" => "Island",
    "farming_1" => "Barn",
    "foraging_1" => "Park",
    "foraging_2" => "Galatea",
    "mining_1" => "Gold Mine",
    "mining_2" => "Deep Caverns",
    "mining_3" => "Dwarven Mines",
    "combat_1" => "Spider's Den",
    "combat_3" => "The End",
    "fishing_1" => "Backwater Bayou"
};