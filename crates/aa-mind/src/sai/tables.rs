//! SAI enum tables registered by the original constructors (`7BDA60` entries).
//! Values are the native enum integers; names are the TGI identifiers.

/// `7BDDD0` -> `7BDB70`: SAI goal types (`goal_IDS`, goal virtual `+4`).
pub const GOAL_TYPES: [&str; 23] = [
    "ATTACK_REGION",
    "ATTACK_ACTOR",
    "DEFEND_REGION",
    "DEFEND_ACTOR",
    "AIR_ATTACK",
    "RECOVER",
    "REPAIR",
    "EXPLORE",
    "BUILD",
    "CONSTRUCT",
    "PACK",
    "UNPACK",
    "RECRUIT",
    "UPGRADE",
    "GIVE_RESOURCES",
    "GIVE_ACTOR",
    "DESTROY",
    "ATTACH_HERO",
    "MOVE_HERO",
    "RESEARCH",
    "BOMBARD",
    "GENERAL_POWER",
    "ATTACH",
];

/// `7BDCF0` (static initializer `7BDFA0`): SAI `difficulty_level` values.
pub const DIFFICULTY_LEVELS: [&str; 2] = ["sai_difficulty_level_easy", "sai_difficulty_level_hard"];

/// `7BDEA0`: goal actor-proximity enum (map [B40EAC]). Consumer: the general
/// power `Location.type` of the ego loader (`4AFD50`), which accepts only
/// 1..=3 ("Invalid target type in General Power (%s).").
pub const GOAL_ACTOR_PROXIMITY: [&str; 4] = ["NONE", "NEAR_ACTOR", "ON_ACTOR", "FOG"];

/// `499940` -> `497FA0` (static initializer `9CCAB0`, map [B3B258]): component
/// types (`component_IDS` of the ego `ComponentValue` list, `4AAD80`).
pub const COMPONENT_TYPES: [&str; 31] = [
    "BODY",
    "PROJECTILE",
    "CHARACTER",
    "MOVER",
    "MIND",
    "PERFORMER",
    "PROPERTIES",
    "ORGANIZATION",
    "ELEMENT",
    "ECONOMY",
    "DENIZEN",
    "SUPPLY",
    "WALL",
    "STRUCTURE",
    "SETTLEMENT",
    "CONSTRUCTION",
    "RECRUIT",
    "EXPERIENCE",
    "BUILDER",
    "MOBILE_DEPLOYMENT",
    "DEFENDER",
    "RESEARCH",
    "DETECTION",
    "AIRCRAFT",
    "TRANSPORT",
    "LAIR",
    "FLARE",
    "METAGAME",
    "PORTAL",
    "CITY",
    "HQ",
];

/// `497220` on a registered map: `__wcsicmp` name match (ASCII case folding),
/// returning the registered value.
pub fn lookup(table: &[&str], name: &str) -> Option<i32> {
    table
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name))
        .map(|i| i as i32)
}

/// Case-sensitive exact lookup, as registered. The native string-to-enum
/// lookup (and its case handling) belongs to the data loader and is not
/// claimed by this table.
pub fn goal_type(name: &str) -> Option<u32> {
    GOAL_TYPES.iter().position(|n| *n == name).map(|i| i as u32)
}
