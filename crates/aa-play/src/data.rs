//! Unit, building and weapon definitions read from the original rules data
//! (`aa_rules::ActorCatalog::load_v14`). Every value here comes from the
//! catalog; nothing is defaulted silently. Owner: core.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Index into `GameData::defs`.
pub type DefId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefClass {
    /// A building (Corps HQ, division HQ, depot, bunker, airfield, city...).
    Building,
    /// A truck that unpacks into a building (HQ and depot trucks).
    Truck,
    /// An organization: a regiment or HQ regiment made of members.
    Organization,
    /// One soldier or vehicle inside an organization.
    Member,
    /// Anything else the battle must know about (capturable city, scenery).
    Other,
}

/// Money, ammunition, oil: the order of the original resource registry.
pub type Resources = [f32; 3];
pub const MONEY: usize = 0;
pub const AMMO: usize = 1;
pub const OIL: usize = 2;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Weapon {
    pub name: String,
    pub range: f32,
    pub min_range: f32,
    /// Seconds between shots (the original derived recharge).
    pub recharge: f32,
    pub damage: f32,
    /// Index into `GameData::damage_types`.
    pub damage_type: u32,
    pub area: f32,
    pub max_targets: u32,
    /// The original weapon flag bits (bit0 ignores defense, bit1 fixes offense).
    pub flags: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Def {
    /// Original Thing id, e.g. "rus_corps", "regiment_rus_infantry".
    pub id: String,
    /// Display name key from the rules (localized by the UI).
    pub name: String,
    pub class: Option<DefClass>,
    pub nation: Option<String>,
    // Body
    pub health: f32,
    pub defense: f32,
    /// Index into `GameData::damage_types` columns: what this body is made of.
    pub armor_type: u32,
    // Mover
    pub speed_max: f32,
    pub acceleration: f32,
    pub turn_radius: f32,
    // Performer
    pub weapons: Vec<Weapon>,
    pub sight_range: f32,
    // Economy (per minute for production/upkeep, like the original)
    pub cost: Resources,
    pub upkeep: Resources,
    pub production: Resources,
    pub unit_limit_provided: f32,
    pub unit_limit_consumed: f32,
    // Organization
    /// What this building or HQ can recruit.
    pub recruits: Vec<DefId>,
    /// Seconds to recruit this organization.
    pub recruit_time: f32,
    /// Member slots: (member def, slot offset in metres in the formation frame).
    pub slots: Vec<(DefId, [f32; 2])>,
    pub morale_breakpoint: f32,
    pub control_range: f32,
    pub operational_range: f32,
    // Trucks and buildings
    pub unpacks_to: Option<DefId>,
    pub packs_to: Option<DefId>,
    pub supply_range: f32,
    // Presentation (read by the renderer, never by the rules)
    /// Original model path, e.g. "Units/RUS/RUSInfantry/RUSInfantry.nif".
    pub model: Option<String>,
    /// Original HUD portrait or button icon path, if any.
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GameData {
    pub defs: Vec<Def>,
    pub by_id: BTreeMap<String, DefId>,
    /// Damage type x armor type multipliers from the original tables.
    pub damage_types: Vec<String>,
    pub damage_factors: Vec<Vec<f32>>,
    /// Original game rules (Data/Game/world_rules_aa.tgi, SVars.tgi) by name.
    pub rules: BTreeMap<String, f32>,
}

impl GameData {
    pub fn def(&self, id: DefId) -> &Def {
        &self.defs[id as usize]
    }
    pub fn find(&self, thing: &str) -> Option<DefId> {
        self.by_id.get(&thing.to_ascii_lowercase()).copied()
    }
    pub fn rule(&self, name: &str) -> Option<f32> {
        self.rules.get(name).copied()
    }

    /// Load from the user's installed original data (Data.rwd + the v14
    /// overlay). Implemented by the core owner on top of aa-rules typed
    /// readers (native_ground_definition, native_performer_inputs,
    /// native_economy_inputs, recruit targets, organization component).
    pub fn load(game_root: &std::path::Path) -> anyhow::Result<Self> {
        let _ = game_root;
        anyhow::bail!("GameData::load is not implemented yet")
    }
}
