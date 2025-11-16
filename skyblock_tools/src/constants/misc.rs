use phf::{Map, phf_map};

pub const RARITIES: &[&str] = &["COMMON", "UNCOMMON", "RARE", "EPIC", "LEGENDARY", "MYTHIC", "DIVINE", "SPECIAL", "VERY SPECIAL", "ULTIMATE COSMETIC", "LEGENJERRY COSMETIC"];
pub const GEMSTONES: &[&str] = &["JADE", "ONYX", "AMBER", "RUBY", "SAPPHIRE", "AMETHYST", "JASPER", "TOPAZ", "PERIDOT", "AQUAMARINE", "CITRINE", "OPAL"];

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
    "FIFTH_MASTER_STAR",
];

pub const TROPHY_FISHING_TIERS: &[&str] = &[
    "None",
    "Novice Trophy Fisher (Caught all Bronze Trophy Fishes)",
    "Adept Trophy Fisher (Caught all Silver Trophy Fishes)",
    "Expert Trophy Fisher (Caught all Gold Trophy Fishes)",
    "Master Trophy Fisher (Caught all Diamond Trophy Fishes)",
];

pub const SLAYER_XP_REQUIRED: Map<&'static str, &[u64]> = phf_map! {
    "zombie" => &[0, 5, 15, 200, 1000, 5000, 20000, 100000, 400000, 1000000],
    "spider" => &[0, 10, 25, 200, 1000, 5000, 20000, 100000, 400000, 1000000],
    "wolf" | "enderman" | "blaze" => &[0, 10, 30, 250, 1500, 5000, 20000, 100000, 400000, 1000000],
    "vampire" => &[0, 20, 75, 240, 840, 2400],
};

pub const ISLAND_NAMES: Map<&'static str, &'static str> = phf_map! {
    "dynamic" => "Private Island",
    "farming_1" => "Barn",
    "foraging_1" => "Park",
    "foraging_2" => "Galatea",
    "mining_1" => "Gold Mine",
    "mining_2" => "Deep Caverns",
    "mining_3" => "Dwarven Mines",
    "combat_1" => "Spider's Den",
    "combat_3" => "The End",
    "fishing_1" => "Backwater Bayou",
    "winter" => "Jerry's Workshop",
    "mineshaft" => "Glacite Mineshafts",
};

pub const ACCESSORY_RARITIES: [&str; 3] = ["ACCESSORY", "HATCESSORY", "DUNGEON ACCESSORY"];

/// Talismans that do not change their ids when they get upgraded
pub const SPECIAL_TALISMANS: &[(&str, &[&str])] = &[
    ("PULSE_RING", &["UNCOMMON", "RARE", "EPIC", "LEGENDARY"]),
    ("BOOK_OF_PROGRESSION", &["COMMON", "UNCOMMON", "RARE", "EPIC", "LEGENDARY", "MYTHIC"]),
    ("RUNEBOOK", &["COMMON", "UNCOMMON", "RARE", "EPIC", "LEGENDARY"]),
    ("PANDORAS_BOX", &["COMMON", "UNCOMMON", "RARE", "EPIC", "LEGENDARY", "MYTHIC"]),
    ("TRAPPER_CREST", &["COMMON", "UNCOMMON"]),
];

pub const MAGICAL_POWER: Map<&'static str, u64> = phf_map! {
    "COMMON" => 3,
    "UNCOMMON" => 5,
    "RARE" => 8,
    "EPIC" => 12,
    "LEGENDARY" => 16,
    "MYTHIC" => 22,
    "SPECIAL" => 3,
    "VERY_SPECIAL" => 5
};

pub const STARRED_ITEMS_INGREDIENT: Map<&'static str, &'static str> = phf_map! {
    "STARRED_ADAPTIVE_BELT" | "STARRED_ADAPTIVE_BOOTS" | "STARRED_ADAPTIVE_CHESTPLATE" | "STARRED_ADAPTIVE_HELMET" | "STARRED_ADAPTIVE_LEGGINGS" | "STARRED_STONE_BLADE" => "SCARF_FRAGMENT",
    "STARRED_BAT_WAND" | "STARRED_BONE_BOOMERANG" | "STARRED_BONE_NECKLACE" | "STARRED_ITEM_SPIRIT_BOW" | "STARRED_SPIRIT_MASK" | "STARRED_THORNS_BOOTS" => "THORN_FRAGMENT",
    "STARRED_BONZO_MASK" | "STARRED_BONZO_STAFF" => "BONZO_FRAGMENT",
    "STARRED_DAEDALUS_AXE" | "STARRED_MIDAS_STAFF" | "STARRED_MIDAS_SWORD" => "GOLDEN_FRAGMENT",
    "STARRED_FELTHORN_REAPER" => "GIANT_FRAGMENT_BIGFOOT",
    "STARRED_GLACIAL_SCYTHE" | "STARRED_ICE_SPRAY_WAND" | "STARRED_YETI_SWORD" => "WINTER_FRAGMENT",
    "STARRED_LAST_BREATH" | "STARRED_SHADOW_ASSASSIN_BOOTS" | "STARRED_SHADOW_ASSASSIN_CHESTPLATE" | "STARRED_SHADOW_ASSASSIN_CLOAK" | "STARRED_SHADOW_ASSASSIN_HELMET" | "STARRED_SHADOW_ASSASSIN_LEGGINGS" | "STARRED_SHADOW_FURY" => "LIVID_FRAGMENT",
};