//! Unit, building and weapon definitions read from the original rules data
//! (`aa_rules::ActorCatalog::load_v14`). Every value here comes from the
//! catalog; nothing is defaulted silently. Owner: core.
//!
//! Behaviour-faithful projection: the typed aa-rules readers that follow the
//! original loader (464BD0 ActorData, 4BF990 abilities, 4C3610 layouts,
//! 4605B0 mobile deployment, 582C40 recruit targets) are used where they
//! exist, plain inherited field reads elsewhere. A Thing whose reader fails
//! is still listed with the fields that did load; every failure is recorded
//! in `GameData::load_notes`.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Index into `GameData::defs`.
pub type DefId = u32;
/// `Def::slots` entry of an empty (null element) organization slot.
pub const NO_DEF: DefId = u32::MAX;

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
    // ---- appended by the core owner ----
    /// Ability IDS (AbilityDefinition IDS) when authored.
    pub id: String,
    /// From the DefenderComponent list (building/HQ turret) rather than the
    /// PerformerComponent list.
    pub defender: bool,
    /// Authored AbilityDefinition recharge_time and Ability reload_time; the
    /// derived `recharge` is max(recharge_time, attack clip length + reload).
    pub recharge_time: f32,
    pub reload_time: f32,
    /// Native ability target filter (bits of `GameData::target_types`):
    /// a target must have every `targets_required` bit and none of `targets_forbidden`.
    pub targets_required: u32,
    pub targets_forbidden: u32,
    pub projectile: Option<String>,
    pub ability_type: Option<String>,
    /// AbilityData+54 turret index, -1 = none.
    pub turret: i32,
    pub volley_area: f32,
    pub attenuation_range_min: f32,
    pub attenuation_factor_max: f32,
    pub impact_impulse: f32,
    pub priority: f32,
    pub fuzzy_range: f32,
    /// Every loaded effect in authored order.
    pub effects: Vec<WeaponEffect>,
    /// Attack clip names of the first animation set (animation1..5).
    pub animations: Vec<String>,
    // ---- appended for combat (52ED30 decision weights) ----
    /// Ability use/wait/move weights and bonuses (template_actorability).
    pub use_weight: f32,
    pub wait_weight: f32,
    pub move_weight: f32,
    pub use_bonus: f32,
    pub wait_bonus: f32,
    pub move_bonus: f32,
    /// ProjectileComponent speed (m/s) and type (ANTIGRAV, LOB, BOMB) of
    /// `projectile`; 0 / None without a projectile.
    pub projectile_speed: f32,
    pub projectile_type: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WeaponEffect {
    /// `aa_rules::commander::EFFECT_KINDS` index (4 main damage, 5 add damage,
    /// 1/2 morale loss/gain, 3 heal...).
    pub kind: u32,
    pub kind_name: String,
    pub amount: f32,
    pub damage_type: Option<u32>,
    pub targets_required: u32,
    pub targets_forbidden: u32,
}

/// One organization formation from the layout (Data/Organizations/Layouts).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Formation {
    /// FormationData IDS, e.g. "regiment_defensive".
    pub id: String,
    /// Offset of every expanded slot (Def::slots order) in metres; [0] is
    /// to the right of the facing, [1] forward (native 4FAD28 rotation).
    pub positions: Vec<[f32; 2]>,
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
    // ---- appended by the core owner ----
    /// Native Thing category (ACTOR, BUILDING, UNIT, PLACEHOLDER, ...).
    pub category: String,
    /// The Thing is native ActorData (category kind 2).
    pub actor_data: bool,
    /// Component presence bits in `aa_rules::native::COMPONENTS` order.
    pub components: u32,
    /// PropertiesComponent property IDS in authored (inherited) order.
    pub properties: Vec<String>,
    /// OR of the properties' enabled_target_flags (bits of `GameData::target_types`).
    pub target_flags: u32,
    /// Damage multiplier per `GameData::damage_types` entry from the
    /// RESIST_DAMAGE modifiers of the properties (502440: max(1 - sum, clamp)).
    pub damage_factor: Vec<f32>,
    pub captureable: bool,
    pub destructible: bool,
    pub rot_time: f32,
    pub sink_time: f32,
    pub unit_shortfall_damage_rate: f32,
    /// Native mover turn type: 0 INSTANT, 1 IN_PLACE, 2 WHILE_MOVING.
    pub turn_type: u32,
    pub detection_range: f32,
    pub remote_detection_range: f32,
    /// Full resource vectors in `GameData::resources` order (per minute for
    /// upkeep/production).
    pub cost_all: Vec<f32>,
    pub upkeep_all: Vec<f32>,
    pub production_all: Vec<f32>,
    pub has_economy: bool,
    pub supply_required: bool,
    /// Native recruitable: 0 NO, 1 YES, 2 WITH_PRIMARY.
    pub recruitable: u32,
    // Organization component (OrganizationComponent + layout)
    pub layout: Option<String>,
    /// Expanded slot index of the primary (captain) slot.
    pub leader_slot: Option<u16>,
    /// Layout entry index of every expanded slot.
    pub slot_entries: Vec<u32>,
    /// Layout entry `required` flag of every expanded slot. A slot whose
    /// authored element is null has `NO_DEF` as its `slots` def id.
    pub slot_required: Vec<bool>,
    pub formations: Vec<Formation>,
    pub default_formation: Option<usize>,
    pub combat_formation: Option<usize>,
    pub retreat_formation: Option<usize>,
    pub supply_used: bool,
    pub in_supply_detached: bool,
    pub shortfall_damage_rate: f32,
    pub recruit_full: bool,
    pub entrenchment_time: f32,
    pub company_group_priority: i32,
    /// HQComponent max_attachments (HQ organizations).
    pub max_attachments: Option<u32>,
    // Element component
    pub morale_value: f32,
    /// ElementComponent resupply_rate (+2CC): fraction of health per second
    /// a fallen member of this kind is rebuilt in supply.
    pub resupply_rate: f32,
    pub group_priority: i32,
    // Supply component
    pub has_supply: bool,
    pub supply_relay: bool,
    pub supply_stockpile_max: f32,
    // Denizen deployment (recruit spawn, native 4FACA0)
    pub creation_offset: Option<[f32; 2]>,
    pub deployment_offset: Option<[f32; 2]>,
    // Mobile deployment
    pub packed_position_offset: [f32; 2],
    pub packed_orientation: [f32; 2],
    pub pack_health_min: f32,
    pub packing_length: f32,
    pub unpacking_length: f32,
    // Footprint
    pub blocking_shape: Option<String>,
    /// SQUARE: [square_size; 2]; RECT: [rect_width, rect_height]; CIRCLE: [blocking_radius; 2].
    pub blocking_size: Option<[f32; 2]>,
    pub placement_category: Option<String>,
    pub minimap_icon: Option<String>,
    pub structure: bool,
    pub city: bool,
    /// Thing `[Flags]`: not drawn (start locations, placeholders, markers).
    pub invisible: bool,
    /// Thing `[Flags]` radar (native Thing flag 0x100): shown on the minimap
    /// (template_thing false, template_actor true, AAVillageCenter true...).
    #[serde(default)]
    pub radar: bool,
    /// Thing `[Flags]` blocking / invulnerable / targetable / flying.
    pub blocking: bool,
    pub invulnerable: bool,
    pub targetable: bool,
    pub flying: bool,
    // ---- appended for combat ----
    /// Thing `[Flags]` tile_locked (placement snapped to the half-cell grid,
    /// 0x6939B0/0x46EE00) and axis_locked (placement facing native -Y).
    pub tile_locked: bool,
    pub axis_locked: bool,
    /// ExperienceComponent fake_cv_value (-1 when absent or not authored).
    pub fake_cv_value: f32,
    /// ExperienceComponent levels, lowest first (the first is assigned at
    /// creation). Empty without an ExperienceComponent.
    pub experience_levels: Vec<ExperienceLevel>,
    /// Character die animations of the base animation set, authored order.
    pub death_clips: Vec<DeathClip>,
    /// Length of the first die clip; None without a Character die clip.
    pub death_time: Option<f32>,
    /// ElementComponent primary_minimum_health (ratio) and
    /// primary_time_to_death (seconds).
    pub primary_minimum_health: f32,
    pub primary_time_to_death: f32,
    /// PerformerComponent move/use/wait bonuses, cv_bias, attack_height,
    /// mana_max, mana_regeneration.
    pub performer_move_bonus: f32,
    pub performer_use_bonus: f32,
    pub performer_wait_bonus: f32,
    pub cv_bias: f32,
    pub attack_height: f32,
    pub mana_max: f32,
    pub mana_regeneration: f32,
    /// ProjectileComponent (projectile Things).
    pub projectile_speed: f32,
    pub projectile_type: Option<String>,
    pub projectile_spread: f32,
    pub projectile_life: f32,
    /// ResearchComponent items (research buildings such as the Corps HQ).
    pub research: Vec<ResearchItem>,
    // ---- appended for movement ----
    /// Thing `[Flags]` water_based (native Thing flag 0x200000, actor flag
    /// bit 21: boats path on water tiles).
    pub water_based: bool,
    /// How the actor moves, from the MoverComponent presence and the Thing
    /// flags the original distinguishes (flying / AircraftComponent,
    /// water_based).
    pub mover_class: MoverClass,
    /// CityComponent zone_radius: units inside it take part in the city's
    /// ownership (City tick 5647A0).
    pub city_zone_radius: f32,
    /// Thing `[Flags]` conform_to_terrain (native Thing flag 0x1000000, set
    /// by the vehicle and trailer templates): the view tilts the model to the
    /// ground under its footprint (0x715E90).
    pub conform_to_terrain: bool,
    /// The Thing's `[Stance]` list in source order (kneel, prone, falling,
    /// packing...); the active one adds its property (45E620).
    #[serde(default)]
    pub stances: Vec<StanceDef>,
    /// PerformerComponent `[Turret]` list (464BD0 / 4C7820), the list
    /// `Weapon::turret` indexes.
    #[serde(default)]
    pub turrets: Vec<TurretDef>,
    /// ActorData +17C recruitment backreferences (460310 / 460000): the
    /// structures that recruit this definition, and for an organization
    /// element also the structures recruiting its organization. 4FBF40
    /// creates a member at the nearest such structure with a DeploymentInfo.
    #[serde(default)]
    pub recruited_at: Vec<DefId>,
}

/// One PerformerComponent `[Turret]` (TurretDefinition, aa-rules turrets.rs),
/// angles in radians.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TurretDef {
    /// +4 yaw_start.
    pub yaw_start: f32,
    /// +8 yaw rotation per second.
    pub yaw_rate: f32,
    /// +C half the yaw rotation range and +10 its cosine.
    pub yaw_half_range: f32,
    pub yaw_half_range_cos: f32,
    /// +14 pitch_start, +18 pitch rotation per second, +1C/+20 pitch range.
    pub pitch_start: f32,
    pub pitch_rate: f32,
    pub pitch_min: f32,
    pub pitch_max: f32,
    /// +24/+28 offset (raw point2).
    pub offset: [f32; 2],
    /// +2C reset_time, +30 can_reset, +31 can_preaim.
    pub reset_time: f32,
    pub can_reset: bool,
    pub can_preaim: bool,
}

/// One `[Stance]` entry: active while the actor's health fraction is at most
/// `health` and one of `states` holds (no states: always, unless dead).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StanceDef {
    pub ids: String,
    /// property_ids added while the stance is active (e.g. infantry_kneel).
    pub property: Option<String>,
    pub health: f32,
    /// actor_state ids, lower case (mind_engaged, under_siege, dead...).
    pub states: Vec<String>,
}

/// Movement domain of a definition. The original MoverComponent has no
/// class field; ground/water/air come from the Thing flags (`flying`,
/// `water_based`) and the AircraftComponent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoverClass {
    /// No MoverComponent: never moves (buildings, scenery).
    #[default]
    None,
    /// Ground mover: land-passable tiles.
    Land,
    /// water_based mover (ships, landing craft): water-passable tiles.
    Water,
    /// flying or AircraftComponent (planes): ignores terrain passability.
    Air,
}

/// One ExperienceComponent level: its threshold and the modifiers of its
/// ModifierProperty.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExperienceLevel {
    pub required: f32,
    pub modifiers: Vec<Modifier>,
}
impl ExperienceLevel {
    /// Sum of the amounts of the modifiers on `attribute` (any kind).
    pub fn amount(&self, attribute: &str) -> f32 {
        self.modifiers.iter().filter(|m| m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(attribute))).map(|m| m.amount).sum()
    }
}

/// One Character die animation (Templates/template_animation.tgi DieAnimation).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeathClip {
    pub id: String,
    /// Desired length in seconds.
    pub length: f32,
    pub weight: f32,
    /// Damage filter (the damage type damage_filter group) and threshold.
    pub damage_filter: f32,
    pub damage_threshold: f32,
}

/// One Property modifier (Templates/template_modifier.tgi).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Modifier {
    /// Native modifier type: FLAG, ADD, MULTIPLY, ENCHANTMENT_IMMUNITY,
    /// ABILITY_EFFECT, RESIST_DAMAGE, PERIODIC_DAMAGE, PERIODIC_HEAL,
    /// REVERSE_ABILITY, RESOURCE_PRODUCTION.
    pub kind: String,
    /// ADD/MULTIPLY attribute (AV, DV, HEALTH, SPEED_MAX, ...).
    pub attribute: Option<String>,
    pub amount: f32,
    /// RESIST_DAMAGE / PERIODIC_DAMAGE damage type index.
    pub damage_type: Option<u32>,
    /// FLAG modifier flag.
    pub flag: Option<String>,
    /// RESOURCE_PRODUCTION per-minute production in `GameData::resources` order.
    pub production: Vec<f32>,
}

/// A Property definition (Data/Properties/*.tgi, also the technologies).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PropertyDef {
    pub id: String,
    pub name: String,
    pub is_technology: bool,
    /// Actors must have every one of these properties to be affected.
    pub required_properties: Vec<String>,
    pub modifiers: Vec<Modifier>,
}

/// An ActorState definition (Data/Game/actor_states*.tgi).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ActorStateDef {
    pub id: String,
    /// NONE, BUSY, QUEUE.
    pub kind: String,
    pub priority: i32,
    /// Properties gained while in the state (e.g. organization_entrenched ->
    /// infantry_entrench, vehicle_entrench).
    pub properties: Vec<String>,
    /// ActorStateMind flags (false for plain states).
    pub resupply: bool,
    pub morale_replenish: bool,
    pub entrenchable: bool,
}

/// One ResearchComponent ResearchItem.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResearchItem {
    /// Technology property granted.
    pub property: String,
    /// GLOBAL (kingdom-wide) or LOCAL.
    pub kind: String,
    pub research_time: f32,
    /// ResearchCost in `GameData::resources` order.
    pub cost: Vec<f32>,
}

impl Def {
    pub fn has_component(&self, name: &str) -> bool {
        aa_rules::native::COMPONENTS
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .is_some_and(|(_, bit)| self.components & (1 << bit) != 0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceKind {
    Stockpiled,
    Upkeep,
    LimitedConsumed,
}

/// One entry of the original resource registry (Game/resources.tgi).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceDef {
    pub id: String,
    pub kind: ResourceKind,
    /// PurchaseCost per registry entry: money paid per unit of an upkeep shortfall.
    pub purchase_cost: Vec<f32>,
    /// LIMITED_CONSUMED: the provider resource that limits it.
    pub limited_by: Option<usize>,
    /// Some LIMITED_CONSUMED resource is limited by this one (EconomyLimitedResourceMax cap).
    pub limited_provider: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NationDef {
    /// "nation_rus".
    pub ids: String,
    /// "rus".
    pub id: String,
    pub name: String,
    /// placeholder Thing IDS (lower case) -> nation Thing.
    pub placeholders: BTreeMap<String, DefId>,
    pub factions: Vec<String>,
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
    // ---- appended by the core owner ----
    pub resources: Vec<ResourceDef>,
    pub nations: Vec<NationDef>,
    /// Native TargetType ids by bit index.
    pub target_types: Vec<String>,
    /// Armor type (distinct property resistance profile) names, one per
    /// `damage_factors` column.
    pub armor_types: Vec<String>,
    /// Things whose reader failed, with the reason.
    pub load_notes: Vec<String>,
    /// SHA256 of the source ledger the data was read from.
    pub source_key: String,
    /// Terrain/Variation/Biome definitions for binding map tiles.
    pub terrain: crate::map::TerrainTables,
    /// Game/kingdom_colors.tgi in registry order: (IDS, RGB 0..1).
    pub kingdom_colors: Vec<(String, [f32; 3])>,
    /// Every Property definition by lower-case IDS.
    pub properties: BTreeMap<String, PropertyDef>,
    /// Every ActorState definition by lower-case IDS.
    pub actor_states: BTreeMap<String, ActorStateDef>,
    /// Game/handicaps.tgi in order (easy, medium, hard): (IDS, property).
    pub handicaps: Vec<(String, Option<String>)>,
    /// Lower-case index of `rules` (built on the first `rule_ci` miss).
    #[serde(skip)]
    pub rules_lower: std::sync::OnceLock<BTreeMap<String, f32>>,
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
    /// Rule lookup ignoring case (SVars names are case-insensitive natively).
    pub fn rule_ci(&self, name: &str) -> Option<f32> {
        self.rule(name).or_else(|| {
            self.rules_lower
                .get_or_init(|| self.rules.iter().map(|(k, v)| (k.to_ascii_lowercase(), *v)).collect())
                .get(&name.to_ascii_lowercase())
                .copied()
        })
    }
    /// Property definition by IDS (any case).
    pub fn property(&self, id: &str) -> Option<&PropertyDef> {
        self.properties.get(&id.to_ascii_lowercase())
    }
    pub fn resource_index(&self, id: &str) -> Option<usize> {
        self.resources.iter().position(|r| r.id.eq_ignore_ascii_case(id))
    }
    pub fn nation(&self, id: &str) -> Option<&NationDef> {
        let id = id.to_ascii_lowercase();
        self.nations
            .iter()
            .find(|n| n.id == id || n.ids.eq_ignore_ascii_case(&id))
    }
    /// Damage multiplier of `damage_type` against a body of `def`.
    pub fn damage_factor(&self, damage_type: u32, def: DefId) -> f32 {
        let armor = self.def(def).armor_type as usize;
        self.damage_factors
            .get(damage_type as usize)
            .and_then(|row| row.get(armor))
            .copied()
            .unwrap_or(1.0)
    }

    /// Load from the user's installed original data (Data.rwd + the v14
    /// overlay). Implemented by the core owner on top of aa-rules typed
    /// readers (native_ground_definition, native_performer_inputs,
    /// native_economy_inputs, recruit targets, organization component).
    ///
    /// The result is cached as JSON next to the build (see `cache_path`);
    /// set `AA_PLAY_NO_CACHE=1` to force a fresh read.
    pub fn load(game_root: &std::path::Path) -> anyhow::Result<Self> {
        loader::load(game_root)
    }
}

/// The open original sources, shared by the data and map loaders.
pub struct Sources {
    pub files: aa_rules::sources::V14Sources,
}
impl Sources {
    pub fn open(game_root: &std::path::Path) -> anyhow::Result<Self> {
        let files = aa_rules::sources::V14Sources::open(
            game_root.join("Data.rwd"),
            game_root.join("reconstruction").join("native-game").join("runtime-expanded-v14"),
            aa_rules::sources::SourceLimits { max_member_bytes: 64 << 20 },
        )
        .map_err(|e| anyhow::anyhow!("opening the original data in {}: {e}", game_root.display()))?;
        Ok(Self { files })
    }
}

pub(crate) mod loader {
    use super::*;
    use aa_numeric::Precision;
    use aa_rules::{
        abilities::{AnimationResolver, LoadedAbility},
        actors::{ActorCatalog, ResolvedNode, ThingRef},
        native::TypedValue,
        tgi::{Document, Item, Limits},
    };

    pub(crate) const PC: Precision = Precision::Double53;
    const CACHE_VERSION: &str = "aa-play-gamedata-v15";

    fn f(bits: u32) -> f32 {
        f32::from_bits(bits)
    }
    pub(crate) fn num(v: &TypedValue) -> Option<f32> {
        match v {
            TypedValue::Scalar(s) => Some(f32::from_bits(s.bits)),
            TypedValue::Integer { value, .. } => Some(*value as f32),
            TypedValue::Boolean { value, .. } => Some(if *value { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
    pub(crate) fn fx(node: &ResolvedNode<'_>, name: &str) -> Option<f32> {
        node.field(name, PC).ok().flatten().as_ref().and_then(num)
    }
    pub(crate) fn st(node: &ResolvedNode<'_>, name: &str) -> Option<String> {
        match node.field(name, PC).ok().flatten() {
            Some(TypedValue::String { value, .. }) => Some(value),
            _ => match node.plain_string_field(name).ok().flatten() {
                Some(TypedValue::String { value, .. }) => Some(value),
                _ => None,
            },
        }
    }
    pub(crate) fn bl(node: &ResolvedNode<'_>, name: &str) -> Option<bool> {
        match node.field(name, PC).ok().flatten() {
            Some(TypedValue::Boolean { value, .. }) => Some(value),
            _ => None,
        }
    }
    pub(crate) fn pt(node: &ResolvedNode<'_>, name: &str) -> Option<[f32; 2]> {
        match node.field(name, PC).ok().flatten() {
            Some(TypedValue::Point { bits, .. }) if bits.len() >= 2 => Some([f(bits[0]), f(bits[1])]),
            _ => None,
        }
    }
    fn text(v: &TypedValue) -> Option<&str> {
        match v {
            TypedValue::String { value, .. } => Some(value),
            _ => None,
        }
    }

    /// Native 4763C0 source-file-relative path: a leading '/' is from the
    /// Data root, otherwise relative to the defining file's folder.
    pub(crate) fn relative(folder: &str, value: &str) -> Option<String> {
        if value.is_empty() || value.eq_ignore_ascii_case("null") {
            return None;
        }
        let joined = if let Some(v) = value.strip_prefix('/') {
            v.trim_start_matches('/').to_owned()
        } else if folder.is_empty() {
            value.to_owned()
        } else {
            format!("{folder}/{value}")
        };
        normalize(&joined)
    }
    fn normalize(path: &str) -> Option<String> {
        let mut parts = Vec::new();
        for part in path.replace('\\', "/").split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop()?;
                }
                p => parts.push(p.to_owned()),
            }
        }
        (!parts.is_empty()).then(|| parts.join("/"))
    }
    /// aa-runtime data_pre_request::path::model_path (405D60/4763C0):
    /// resource_prefix + aspect_prefix into the model's '%s'.
    pub(crate) fn model_path(folder: &str, resource: &str, aspect: &str, format: &str) -> Option<String> {
        let prefix = if aspect.starts_with('/') { aspect.to_owned() } else { format!("{resource}{aspect}") };
        let expanded = if format.contains("%s") { format.replacen("%s", &prefix, 1) } else { format.to_owned() };
        if expanded.is_empty() {
            return None;
        }
        let path = if let Some(p) = expanded.strip_prefix('/') {
            p.to_owned()
        } else if folder.is_empty() {
            expanded
        } else {
            format!("{folder}/{expanded}")
        };
        normalize(&path)
    }

    fn cache_path(game_root: &std::path::Path) -> std::path::PathBuf {
        if let Ok(p) = std::env::var("AA_PLAY_DATA_CACHE") {
            return p.into();
        }
        game_root.join("rust-rebuild").join("assets-cache").join("aa-play").join("gamedata.json")
    }

    pub(crate) fn load(game_root: &std::path::Path) -> anyhow::Result<GameData> {
        let key = format!("{CACHE_VERSION}:{}", aa_rules::sources::BASELINE_LEDGER_SHA256);
        let path = cache_path(game_root);
        let use_cache = std::env::var("AA_PLAY_NO_CACHE").map_or(true, |v| v.is_empty() || v == "0");
        if use_cache
            && let Ok(bytes) = std::fs::read(&path)
            && let Ok(data) = serde_json::from_slice::<GameData>(&bytes)
            && data.source_key == key
        {
            return Ok(data);
        }
        let mut sources = Sources::open(game_root)?;
        let catalog = ActorCatalog::load_v14(&mut sources.files)
            .map_err(|e| anyhow::anyhow!("loading the original rules catalog: {e}"))?;
        let mut data = build(&mut sources, &catalog)?;
        data.source_key = key;
        if use_cache {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            // Write then rename, so a concurrent reader never sees half a file.
            if let Ok(bytes) = serde_json::to_vec(&data) {
                let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
                if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &path).is_err() {
                    let _ = std::fs::remove_file(&tmp);
                }
            }
        }
        Ok(data)
    }

    /// SVars.tgi then SVars_AA.tgi (the original wildcard loader order; a
    /// later assignment replaces an earlier one), then the WorldRule defaults
    /// (world_rules*.tgi override the SVars defaults, as SVars.tgi notes).
    fn rules(sources: &mut Sources, catalog: &ActorCatalog, notes: &mut Vec<String>) -> BTreeMap<String, f32> {
        let mut rules = BTreeMap::new();
        for file in ["Game/SVars.tgi", "Game/SVars_AA.tgi"] {
            let Ok(file) = sources.files.read(file) else {
                notes.push(format!("rules file {file} missing"));
                continue;
            };
            let Ok(doc) = Document::parse_native(file.bytes, Limits::default()) else {
                notes.push(format!("rules file {} does not parse", file.logical_path));
                continue;
            };
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc.header_name(&node.header).is_some_and(|s| s.eq_ignore_ascii_case("SVars")) {
                    continue;
                }
                for item in &node.items {
                    let Item::Property(p) = item else { continue };
                    let (Some(name), Some(raw)) = (doc.property_name(p), doc.property_value(p)) else { continue };
                    if let Some(v) = parse_scalar(raw) {
                        rules.insert(name.to_owned(), v);
                    }
                }
            }
        }
        if let Ok(nodes) = catalog.resolved_definitions("WorldRule") {
            for node in nodes {
                let (Some(id), Some(default)) = (st(&node, "IDS").or_else(|| st(&node, "ids")), st(&node, "default")) else {
                    continue;
                };
                if let Some(v) = parse_scalar(&default) {
                    rules.insert(id, v);
                }
            }
        }
        rules
    }
    fn parse_scalar(raw: &str) -> Option<f32> {
        let t = raw.trim().trim_matches('"');
        if t.eq_ignore_ascii_case("true") {
            return Some(1.0);
        }
        if t.eq_ignore_ascii_case("false") {
            return Some(0.0);
        }
        if let Some(p) = t.strip_suffix('%') {
            return p.trim().parse::<f32>().ok().map(|v| v / 100.0);
        }
        let t = t.strip_suffix('f').unwrap_or(t);
        t.parse::<f32>().ok()
    }

    fn resources(catalog: &ActorCatalog, notes: &mut Vec<String>) -> Vec<ResourceDef> {
        let mut out = Vec::new();
        let Ok(domain) = catalog.native_resource_registry(PC) else {
            notes.push("native resource registry unavailable".into());
            return out;
        };
        let mut limited = Vec::new();
        for id in &domain.ids {
            let Ok(node) = catalog.resolved_definition("Resource", id, PC) else {
                notes.push(format!("resource {id} unresolved"));
                continue;
            };
            let kind = match st(&node, "type").unwrap_or_default().to_ascii_uppercase().as_str() {
                "STOCKPILED" => ResourceKind::Stockpiled,
                "UPKEEP" => ResourceKind::Upkeep,
                _ => ResourceKind::LimitedConsumed,
            };
            let mut purchase_cost = vec![0.0; domain.ids.len()];
            if let Ok(Some(cost)) = node.child("PurchaseCost") {
                for (i, rid) in domain.ids.iter().enumerate() {
                    purchase_cost[i] = fx(&cost, rid).unwrap_or(0.0);
                }
            }
            limited.push(st(&node, "limited_by").filter(|s| !s.is_empty()));
            out.push(ResourceDef { id: id.clone(), kind, purchase_cost, limited_by: None, limited_provider: false });
        }
        for (i, by) in limited.into_iter().enumerate() {
            if let Some(by) = by
                && let Some(j) = out.iter().position(|r| r.id.eq_ignore_ascii_case(&by))
            {
                out[i].limited_by = Some(j);
                out[j].limited_provider = true;
            }
        }
        out
    }

    /// RESIST_DAMAGE modifiers of one property: (damage type index, amount).
    fn property_resists(catalog: &ActorCatalog, id: &str, damage_types: &[String]) -> Vec<(usize, f32)> {
        let mut out = Vec::new();
        let Ok(node) = catalog.resolved_definition("Property", id, PC) else { return out };
        for m in node.children("Modifier").unwrap_or_default() {
            if !st(&m, "type").is_some_and(|t| t.eq_ignore_ascii_case("RESIST_DAMAGE")) {
                continue;
            }
            let (Some(dt), Some(amount)) = (st(&m, "damage_type"), fx(&m, "amount")) else { continue };
            if let Some(i) = damage_types.iter().position(|d| d.eq_ignore_ascii_case(&dt)) {
                out.push((i, amount));
            }
        }
        out
    }
    fn property_targets(catalog: &ActorCatalog, id: &str, domain: Option<&aa_rules::targets::TargetDomain>) -> u32 {
        let (Some(domain), Ok(node)) = (domain, catalog.resolved_definition("Property", id, PC)) else { return 0 };
        node.field("enabled_target_flags", PC)
            .ok()
            .flatten()
            .and_then(|v| domain.flag_word(&v).ok())
            .unwrap_or(0)
    }

    /// The `[Modifier]` children of a Property node (template_modifier.tgi).
    pub(crate) fn modifiers(node: &ResolvedNode<'_>, damage_types: &[String], resource_ids: &[String]) -> Vec<Modifier> {
        let mut out = Vec::new();
        for m in node.children("Modifier").unwrap_or_default() {
            let kind = st(&m, "type").unwrap_or_default().to_ascii_uppercase();
            let production = m
                .child("Production")
                .ok()
                .flatten()
                .map(|p| resource_ids.iter().map(|r| fx(&p, r).unwrap_or(0.0)).collect())
                .unwrap_or_default();
            out.push(Modifier {
                attribute: st(&m, "attribute").filter(|a| !a.is_empty()),
                amount: fx(&m, "amount").unwrap_or(0.0),
                damage_type: st(&m, "damage_type").and_then(|d| damage_types.iter().position(|x| x.eq_ignore_ascii_case(&d))).map(|i| i as u32),
                flag: st(&m, "modifier_flag").filter(|a| !a.is_empty()),
                production,
                kind,
            });
        }
        out
    }

    struct Pending {
        recruits: Vec<String>,
        slots: Vec<Option<String>>,
        unpacks_to: Option<String>,
        packs_to: Option<String>,
        resists: Vec<(usize, f32)>,
    }

    pub(crate) fn build(sources: &mut Sources, catalog: &ActorCatalog) -> anyhow::Result<GameData> {
        let mut notes = Vec::new();
        let mut data = GameData { rules: rules(sources, catalog, &mut notes), ..Default::default() };
        data.resources = resources(catalog, &mut notes);
        data.damage_types = catalog
            .native_damage_type_registry(PC)
            .map(|d| d.ids)
            .unwrap_or_else(|e| {
                notes.push(format!("damage types: {e}"));
                Vec::new()
            });
        let target_domain = catalog.native_target_domain(PC).ok();
        if let Some(domain) = &target_domain {
            let mut names = vec![String::new(); 32];
            for (name, &i) in &domain.indices {
                if (i as usize) < names.len() {
                    names[i as usize] = name.clone();
                }
            }
            data.target_types = names;
        }
        let layouts = catalog.native_layout_data(PC).map_err(|e| notes.push(format!("layout data: {e}"))).ok();
        let mobile = catalog.native_mobile_deployment_data(PC).map_err(|e| notes.push(format!("mobile deployment: {e}"))).ok();
        let clamp = data.rule_ci("ModifierAttributeClampMultiply").unwrap_or(0.05);
        let mut property_cache: BTreeMap<String, (Vec<(usize, f32)>, u32)> = BTreeMap::new();
        let mut pending = Vec::new();
        let resource_ids: Vec<String> = data.resources.iter().map(|r| r.id.clone()).collect();
        for actor in &catalog.actors {
            let key = actor.id.to_ascii_lowercase();
            if data.by_id.contains_key(&key) {
                continue;
            }
            let (def, p) = read_def(catalog, &data.damage_types, &resource_ids, &actor.id, &actor.source.path, layouts.as_ref(), mobile.as_ref(), &mut notes, &mut |id: &str| {
                property_cache
                    .entry(id.to_ascii_lowercase())
                    .or_insert_with(|| (property_resists(catalog, id, &data.damage_types), property_targets(catalog, id, target_domain.as_ref())))
                    .clone()
            });
            data.by_id.insert(key, data.defs.len() as DefId);
            data.defs.push(def);
            pending.push(p);
        }
        // Resolve references through the Thing translation (427BD0 Find).
        let resolve = |data: &GameData, id: &str| -> Option<DefId> {
            data.find(id).or_else(|| catalog.find(id).and_then(|d| data.find(&d.id)))
        };
        let mut profiles: Vec<(String, Vec<f32>)> = Vec::new();
        for (index, p) in pending.into_iter().enumerate() {
            let recruits = p.recruits.iter().filter_map(|r| resolve(&data, r)).collect();
            let unpacks_to = p.unpacks_to.as_deref().and_then(|r| resolve(&data, r));
            let packs_to = p.packs_to.as_deref().and_then(|r| resolve(&data, r));
            let mut factor = vec![1.0f32; data.damage_types.len()];
            for (i, sum) in data.damage_types.iter().enumerate().map(|(i, _)| (i, p.resists.iter().filter(|r| r.0 == i).map(|r| r.1).sum::<f32>())) {
                factor[i] = (1.0 - sum).max(clamp);
            }
            let armor = match profiles.iter().position(|(_, v)| *v == factor) {
                Some(i) => i,
                None => {
                    let name = data.damage_types.iter().zip(&factor).map(|(d, v)| format!("{d}={v}")).collect::<Vec<_>>().join(",");
                    profiles.push((name, factor.clone()));
                    profiles.len() - 1
                }
            };
            let slot_ids: Vec<DefId> = p.slots.iter().map(|s| s.as_deref().and_then(|r| resolve(&data, r)).unwrap_or(u32::MAX)).collect();
            let def = &mut data.defs[index];
            def.recruits = recruits;
            def.unpacks_to = unpacks_to;
            def.packs_to = packs_to;
            def.damage_factor = factor;
            def.armor_type = armor as u32;
            let positions = def
                .default_formation
                .and_then(|f| def.formations.get(f))
                .map(|f| f.positions.clone())
                .unwrap_or_default();
            def.slots = slot_ids
                .iter()
                .enumerate()
                .map(|(i, &d)| (d, positions.get(i).copied().unwrap_or([0.0, 0.0])))
                .collect();
        }
        // ActorData +17C (460310): every structure listing a definition in
        // its RecruitComponent; then 460000 gives each organization element
        // the organization's entries it lacks.
        for b in 0..data.defs.len() {
            for t in data.defs[b].recruits.clone() {
                if let Some(d) = data.defs.get_mut(t as usize)
                    && !d.recruited_at.contains(&(b as DefId))
                {
                    d.recruited_at.push(b as DefId);
                }
            }
        }
        for o in 0..data.defs.len() {
            if data.defs[o].recruited_at.is_empty() {
                continue;
            }
            let owners = data.defs[o].recruited_at.clone();
            for (m, _) in data.defs[o].slots.clone() {
                if let Some(d) = data.defs.get_mut(m as usize) {
                    for &b in &owners {
                        if !d.recruited_at.contains(&b) {
                            d.recruited_at.push(b);
                        }
                    }
                }
            }
        }
        data.armor_types = profiles.iter().map(|(n, _)| n.clone()).collect();
        data.damage_factors = (0..data.damage_types.len())
            .map(|dt| profiles.iter().map(|(_, v)| v[dt]).collect())
            .collect();
        // Nations and their placeholder mapping (Data/Nations/<nation>.tgi).
        if let Ok(nodes) = catalog.resolved_definitions("Nation") {
            for node in nodes {
                let ids = st(&node, "ids").or_else(|| st(&node, "IDS")).unwrap_or_default();
                let mut nation = NationDef {
                    id: ids.strip_prefix("nation_").unwrap_or(&ids).to_ascii_lowercase(),
                    ids: ids.clone(),
                    name: node.localized("name").ok().flatten().map(|l| l.text).unwrap_or_default(),
                    factions: st(&node, "factions").map(|s| s.split(',').map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()).collect()).unwrap_or_default(),
                    ..Default::default()
                };
                for ph in node.children("Placeholder").unwrap_or_default() {
                    let (Some(from), Some(to)) = (st(&ph, "placeholder_IDS"), st(&ph, "thing_IDS")) else { continue };
                    match resolve(&data, &to) {
                        Some(d) => {
                            nation.placeholders.insert(from.to_ascii_lowercase(), d);
                        }
                        None => notes.push(format!("nation {ids}: placeholder target {to} unknown")),
                    }
                }
                data.nations.push(nation);
            }
        }
        // Property definitions (resistances, states, technologies, levels).
        if let Ok(nodes) = catalog.resolved_definitions("Property") {
            for node in nodes {
                let Some(id) = st(&node, "IDS").or_else(|| st(&node, "ids")) else { continue };
                let required = st(&node, "required_properties")
                    .map(|v| v.split([',', ' ']).map(str::trim).filter(|x| !x.is_empty() && !x.eq_ignore_ascii_case("null")).map(str::to_owned).collect())
                    .unwrap_or_default();
                let def = PropertyDef {
                    name: node.localized("name").ok().flatten().map(|l| l.text).unwrap_or_default(),
                    is_technology: bl(&node, "is_technology").unwrap_or(false),
                    required_properties: required,
                    modifiers: modifiers(&node, &data.damage_types, &resource_ids),
                    id: id.clone(),
                };
                data.properties.entry(id.to_ascii_lowercase()).or_insert(def);
            }
        }
        if let Ok(nodes) = catalog.resolved_definitions("ActorState") {
            for node in nodes {
                let Some(id) = st(&node, "IDS") else { continue };
                let properties = node
                    .children("Property")
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|p| st(p, "property_ids").or_else(|| st(p, "property_IDS")))
                    .collect();
                let def = ActorStateDef {
                    kind: st(&node, "type").unwrap_or_default().to_ascii_uppercase(),
                    priority: fx(&node, "priority").unwrap_or(0.0) as i32,
                    properties,
                    resupply: bl(&node, "resupply").unwrap_or(false),
                    morale_replenish: bl(&node, "morale_replenish").unwrap_or(false),
                    entrenchable: bl(&node, "entrenchable").unwrap_or(false),
                    id: id.clone(),
                };
                data.actor_states.entry(id.to_ascii_lowercase()).or_insert(def);
            }
        }
        if let Ok(nodes) = catalog.resolved_definitions("Handicap") {
            for node in nodes {
                let Some(id) = st(&node, "IDS") else { continue };
                data.handicaps.push((id, st(&node, "property_ids").filter(|p| !p.is_empty() && !p.eq_ignore_ascii_case("null"))));
            }
        }
        // Projectile speeds onto the weapons that fire them.
        let projectile = |data: &GameData, id: &str| resolve(data, id).map(|d| (data.def(d).projectile_speed, data.def(d).projectile_type.clone()));
        for i in 0..data.defs.len() {
            for w in 0..data.defs[i].weapons.len() {
                if let Some(pid) = data.defs[i].weapons[w].projectile.clone()
                    && let Some((speed, kind)) = projectile(&data, &pid)
                {
                    data.defs[i].weapons[w].projectile_speed = speed;
                    data.defs[i].weapons[w].projectile_type = kind;
                }
            }
        }
        // The per-definition rule keys combat reads (`<id>.death_time`,
        // `<id>.fake_cv_value`, `<id>.experience<N>.<ATTRIBUTE>`).
        let mut keys = Vec::new();
        for d in &data.defs {
            if let Some(t) = d.death_time {
                keys.push((format!("{}.death_time", d.id), t));
            }
            if d.has_component("ExperienceComponent") {
                keys.push((format!("{}.fake_cv_value", d.id), d.fake_cv_value));
            }
            for (n, level) in d.experience_levels.iter().enumerate() {
                keys.push((format!("{}.experience{n}.required", d.id), level.required));
                let mut attrs: Vec<String> = level.modifiers.iter().filter_map(|m| m.attribute.clone()).map(|a| a.to_ascii_uppercase()).collect();
                attrs.sort();
                attrs.dedup();
                for a in attrs {
                    keys.push((format!("{}.experience{n}.{a}", d.id), level.amount(&a)));
                }
            }
        }
        data.rules.extend(keys);
        data.terrain = crate::map::terrain_tables(catalog, &mut notes);
        if let Ok(nodes) = catalog.resolved_definitions("KingdomColor") {
            for node in nodes {
                let Some(id) = st(&node, "IDS") else { continue };
                let rgb = match node.field("color", PC).ok().flatten() {
                    Some(TypedValue::Point { bits, .. }) if bits.len() >= 3 => [f(bits[0]), f(bits[1]), f(bits[2])],
                    _ => [1.0, 1.0, 1.0],
                };
                data.kingdom_colors.push((id, rgb));
            }
        }
        data.load_notes = notes;
        Ok(data)
    }

    #[allow(clippy::too_many_arguments)]
    fn read_def(
        catalog: &ActorCatalog,
        damage_types: &[String],
        resource_ids: &[String],
        id: &str,
        source_path: &str,
        layouts: Option<&aa_rules::native::layout_data::NativeLayoutData>,
        mobile: Option<&aa_rules::native_mobile_deployment::NativeMobileDeploymentData>,
        notes: &mut Vec<String>,
        property: &mut dyn FnMut(&str) -> (Vec<(usize, f32)>, u32),
    ) -> (Def, Pending) {
        let mut def = Def { id: id.to_owned(), health: 0.0, ..Default::default() };
        let mut p = Pending { recruits: vec![], slots: vec![], unpacks_to: None, packs_to: None, resists: vec![] };
        let mut note = |what: &str, e: &dyn std::fmt::Display| notes.push(format!("{id}: {what}: {e}"));
        let category = match catalog.native_category(ThingRef::registered(id)) {
            Ok(c) => c,
            Err(e) => {
                note("category", &e);
                def.class = Some(DefClass::Other);
                return (def, p);
            }
        };
        def.category = category.name.to_owned();
        def.actor_data = category.native_kind == 2;
        let node = match catalog.resolved_actor(ThingRef::registered(id)) {
            Ok(n) => n,
            Err(e) => {
                note("resolve", &e);
                def.class = Some(DefClass::Other);
                return (def, p);
            }
        };
        let folder = source_path.replace('\\', "/").rsplit_once('/').map_or(String::new(), |(f, _)| f.to_owned());
        def.name = node.localized("name").ok().flatten().map(|l| l.text).unwrap_or_default();
        for (name, bit) in aa_rules::native::COMPONENTS {
            if node.child(name).ok().flatten().is_some() {
                def.components |= 1 << bit;
            }
        }
        // View: model and icons.
        if let Ok(Some(view)) = node.child("View") {
            let prefix = st(&view, "resource_prefix").unwrap_or_default();
            let variations = bl(&view, "aspects_as_variations").unwrap_or(false);
            let aspect = if !variations { view.child("AspectNormal").ok().flatten() } else { None }
                .or_else(|| view.children("Aspect").ok().and_then(|v| v.into_iter().next()));
            if let Some(aspect) = aspect {
                let aspect_prefix = st(&aspect, "aspect_prefix").unwrap_or_default();
                let format = st(&aspect, "model").unwrap_or_else(|| "%s.nif".into());
                if !prefix.is_empty() || !format.contains("%s") {
                    def.model = model_path(&folder, &prefix, &aspect_prefix, &format);
                }
            }
            // An icon may carry the model's '%s' (e.g. "%sIcon.tga").
            def.icon = st(&view, "icon").and_then(|i| if i.contains("%s") { model_path(&folder, &prefix, "", &i) } else { relative(&folder, &i) });
            def.minimap_icon = st(&view, "minimap_icon").and_then(|i| relative(&folder, &i));
        }
        if let Ok(Some(flags)) = node.child("Flags") {
            def.invisible = bl(&flags, "invisible").unwrap_or(false);
            def.radar = bl(&flags, "radar").unwrap_or(false);
            def.blocking = bl(&flags, "blocking").unwrap_or(false);
            def.invulnerable = bl(&flags, "invulnerable").unwrap_or(false);
            def.targetable = bl(&flags, "targetable").unwrap_or(false);
            def.flying = bl(&flags, "flying").unwrap_or(false);
            def.tile_locked = bl(&flags, "tile_locked").unwrap_or(false);
            def.water_based = bl(&flags, "water_based").unwrap_or(false);
            def.axis_locked = bl(&flags, "axis_locked").unwrap_or(false);
            def.conform_to_terrain = bl(&flags, "conform_to_terrain").unwrap_or(false);
        }
        let null = |v: &String| v.is_empty() || v.eq_ignore_ascii_case("null");
        for s in node.children("Stance").unwrap_or_default() {
            def.stances.push(StanceDef {
                ids: st(&s, "ids").or_else(|| st(&s, "IDS")).unwrap_or_default().to_ascii_lowercase(),
                property: st(&s, "property_ids").filter(|v| !null(v)),
                health: fx(&s, "health").unwrap_or(1.0),
                states: st(&s, "actor_state")
                    .map(|v| v.split([',', ' ', '\t']).filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("null")).map(str::to_ascii_lowercase).collect())
                    .unwrap_or_default(),
            });
        }
        def.fake_cv_value = -1.0;
        if let Ok(Some(proj)) = node.child("ProjectileComponent") {
            def.projectile_speed = fx(&proj, "projectile_speed").unwrap_or(0.0);
            def.projectile_type = st(&proj, "projectile_type").filter(|v| !v.is_empty());
            def.projectile_spread = fx(&proj, "projectile_spread").unwrap_or(0.0);
            def.projectile_life = fx(&proj, "projectile_life").unwrap_or(0.0);
        }
        if let Ok(Some(placement)) = node.child("Placement") {
            def.placement_category = st(&placement, "placement_category");
        }
        if let Ok(Some(size)) = node.child("Size") {
            let shape = st(&size, "blocking_shape");
            def.blocking_size = match shape.as_deref().map(|s| s.to_ascii_uppercase()) {
                Some(s) if s == "SQUARE" => fx(&size, "square_size").map(|v| [v, v]),
                // 0049DC40: shape names of 0046F550 (POINT, CIRCLE, SQUARE, RECT)
                // and their size keys.
                Some(s) if s == "RECT" => Some([fx(&size, "rect_width").unwrap_or(0.0), fx(&size, "rect_height").unwrap_or(0.0)]),
                Some(s) if s == "CIRCLE" => fx(&size, "blocking_radius").map(|v| [v, v]),
                _ => None,
            };
            def.blocking_shape = shape;
        }
        if !def.actor_data {
            def.class = Some(DefClass::Other);
            return (def, p);
        }
        // Properties (nation, intrinsic resistances, target flags).
        if let Ok(Some(props)) = node.child("PropertiesComponent") {
            for child in props.children("Property").unwrap_or_default() {
                let Some(pid) = st(&child, "property_IDS").or_else(|| st(&child, "property_ids")) else { continue };
                if def.properties.iter().any(|q| q.eq_ignore_ascii_case(&pid)) {
                    continue;
                }
                if let Some(n) = pid.to_ascii_lowercase().strip_prefix("nation_") {
                    def.nation = Some(n.to_owned());
                }
                let (resists, targets) = property(&pid);
                p.resists.extend(resists);
                def.target_flags |= targets;
                def.properties.push(pid);
            }
        }
        match catalog.native_attributes(id, PC) {
            Ok(a) => {
                if let Some(b) = a.body {
                    def.health = f(b.health.bits);
                    def.defense = f(b.defense.bits);
                    def.rot_time = f(b.rot_time.bits);
                    def.sink_time = f(b.sink_time.bits);
                    def.captureable = b.captureable.boolean().unwrap_or(false);
                    def.destructible = b.destructible.boolean().unwrap_or(true);
                }
                if let Some(m) = a.mover {
                    def.speed_max = f(m.speed_max.bits);
                    def.acceleration = f(m.acceleration.bits);
                    def.turn_radius = f(m.turn_radius.bits);
                    def.turn_type = m.native_turn_type().unwrap_or(0);
                }
            }
            Err(e) => note("attributes", &e),
        }
        if let Ok(Some(body)) = node.child("BodyComponent") {
            def.unit_shortfall_damage_rate = fx(&body, "unit_shortfall_damage_rate").unwrap_or(0.0);
        }
        if let Ok(Some(det)) = node.child("DetectionComponent") {
            def.detection_range = fx(&det, "detection_range").unwrap_or(0.0);
            def.remote_detection_range = fx(&det, "remote_detection_range").unwrap_or(0.0);
            def.sight_range = def.detection_range;
        }
        if let Ok(Some(defender)) = node.child("DefenderComponent") {
            def.control_range = fx(&defender, "control_range").unwrap_or(0.0);
        }
        // Weapons: the completed native ability load (4BF990), which needs the
        // original Character animation set for the derived recharge.
        let character = aa_animation::definitions::character(catalog, id, PC).unwrap_or_else(|e| {
            note("character animations", &e);
            None
        });
        if let Some(c) = &character {
            for a in c.for_aspect(None).iter().filter(|a| a.timing.kind == 3) {
                let g = |k: &str| a.fields.get(k).and_then(num).unwrap_or(0.0);
                def.death_clips.push(DeathClip { id: a.id.clone(), length: f(a.timing.length.0), weight: g("weight"), damage_filter: g("damage_filter"), damage_threshold: g("damage_threshold") });
            }
            def.death_time = def.death_clips.first().map(|c| c.length);
        }
        let resolver = character.as_ref().map(|c| c as &dyn AnimationResolver);
        match catalog.native_loaded_performer(id, PC, resolver) {
            Ok(Some(perf)) => def.weapons.extend(perf.abilities().iter().map(|a| weapon(a, false, damage_types))),
            Ok(None) => {}
            Err(e) => note("performer abilities", &e),
        }
        match catalog.native_loaded_defender(id, PC, resolver) {
            Ok(Some(d)) => def.weapons.extend(d.abilities().iter().map(|a| weapon(a, true, damage_types))),
            Ok(None) => {}
            Err(e) => note("defender abilities", &e),
        }
        match catalog.native_turrets(id, PC) {
            Ok(list) => {
                def.turrets = list
                    .iter()
                    .map(|t| TurretDef {
                        yaw_start: f32::from_bits(t.yaw_start_bits),
                        yaw_rate: f32::from_bits(t.yaw_rate_bits),
                        yaw_half_range: f32::from_bits(t.yaw_half_range_bits),
                        yaw_half_range_cos: f32::from_bits(t.yaw_half_range_cos_bits),
                        pitch_start: f32::from_bits(t.pitch_start_bits),
                        pitch_rate: f32::from_bits(t.pitch_rate_bits),
                        pitch_min: f32::from_bits(t.pitch_range_start_bits),
                        pitch_max: f32::from_bits(t.pitch_range_end_bits),
                        offset: [f32::from_bits(t.offset_bits[0]), f32::from_bits(t.offset_bits[1])],
                        reset_time: f32::from_bits(t.reset_time_bits),
                        can_reset: t.can_reset,
                        can_preaim: t.can_preaim,
                    })
                    .collect()
            }
            Err(e) => note("turrets", &e),
        }
        // Economy (464BD0 EconomyComponent + 4605B0 forward unpack target).
        match catalog.native_economy_inputs(id, PC) {
            Ok(e) => {
                def.recruitable = e.recruitable_word;
                p.unpacks_to = e.unpack_to_actor.map(|r| r.id);
                if let Some(r) = e.resources {
                    def.has_economy = true;
                    def.supply_required = r.supply_required.boolean().unwrap_or(false);
                    def.cost_all = r.cost_bits.iter().map(|&b| f(b)).collect();
                    def.upkeep_all = r.upkeep_bits.iter().map(|&b| f(b)).collect();
                    def.production_all = r.production_bits.iter().map(|&b| f(b)).collect();
                    let pick = |v: &[f32], name: &str| r.resource_ids.iter().position(|i| i.eq_ignore_ascii_case(name)).and_then(|i| v.get(i).copied()).unwrap_or(0.0);
                    for (slot, name) in [(MONEY, "money"), (AMMO, "ammo"), (OIL, "oil")] {
                        def.cost[slot] = pick(&def.cost_all, name);
                        def.upkeep[slot] = pick(&def.upkeep_all, name);
                        def.production[slot] = pick(&def.production_all, name);
                    }
                    def.unit_limit_provided = pick(&def.production_all, "unit_limit_provided");
                    def.unit_limit_consumed = pick(&def.upkeep_all, "unit_limit_consumed");
                }
            }
            Err(e) => note("economy", &e),
        }
        if let Some(m) = mobile
            && let Ok(d) = m.definition(id)
        {
            p.packs_to = d.pack_to.clone();
            if p.unpacks_to.is_none() {
                p.unpacks_to = d.unpack_to.clone();
            }
        }
        if let Ok(Some(md)) = node.child("MobileDeploymentComponent") {
            def.packed_position_offset = pt(&md, "packed_position_offset").unwrap_or([0.0, 0.0]);
            def.packed_orientation = pt(&md, "packed_orientation").unwrap_or([0.0, -1.0]);
            def.pack_health_min = fx(&md, "pack_health_min").unwrap_or(0.0);
            def.packing_length = fx(&md, "packing_length").unwrap_or(0.0);
            def.unpacking_length = fx(&md, "unpacking_length").unwrap_or(0.0);
        }
        if let Ok(Some(supply)) = node.child("SupplyComponent") {
            def.has_supply = true;
            def.supply_range = fx(&supply, "supply_range").unwrap_or(0.0);
            def.supply_relay = bl(&supply, "is_relay").unwrap_or(false);
            def.supply_stockpile_max = fx(&supply, "stockpile_max").unwrap_or(0.0);
        }
        if let Ok(Some(hq)) = node.child("HQComponent") {
            def.max_attachments = fx(&hq, "max_attachments").map(|v| v.max(0.0) as u32);
        }
        if let Ok(Some(element)) = node.child("ElementComponent") {
            def.morale_value = fx(&element, "morale_value").unwrap_or(0.0);
            def.resupply_rate = fx(&element, "resupply_rate").unwrap_or(0.0);
            def.group_priority = fx(&element, "group_priority").unwrap_or(0.0) as i32;
            def.primary_minimum_health = fx(&element, "primary_minimum_health").unwrap_or(0.0);
            def.primary_time_to_death = fx(&element, "primary_time_to_death").unwrap_or(0.0);
        }
        if let Ok(Some(perf)) = node.child("PerformerComponent") {
            def.performer_move_bonus = fx(&perf, "move_bonus").unwrap_or(0.0);
            def.performer_use_bonus = fx(&perf, "use_bonus").unwrap_or(0.0);
            def.performer_wait_bonus = fx(&perf, "wait_bonus").unwrap_or(0.0);
            def.cv_bias = fx(&perf, "cv_bias").unwrap_or(0.0);
            def.attack_height = fx(&perf, "attack_height").unwrap_or(0.0);
            def.mana_max = fx(&perf, "mana_max").unwrap_or(0.0);
            def.mana_regeneration = fx(&perf, "mana_regeneration").unwrap_or(0.0);
        }
        if let Ok(Some(exp)) = node.child("ExperienceComponent") {
            def.fake_cv_value = fx(&exp, "fake_cv_value").unwrap_or(-1.0);
            for level in exp.children("ExperienceLevel").unwrap_or_default() {
                let required = fx(&level, "experience_required").unwrap_or(0.0);
                let modifiers = level.child("Property").ok().flatten().map(|p| modifiers(&p, damage_types, resource_ids)).unwrap_or_default();
                def.experience_levels.push(ExperienceLevel { required, modifiers });
            }
        }
        if let Ok(Some(research)) = node.child("ResearchComponent") {
            for item in research.children("ResearchItem").unwrap_or_default() {
                let Some(property) = st(&item, "property_IDS").or_else(|| st(&item, "property_ids")) else { continue };
                let cost = item
                    .child("ResearchCost")
                    .ok()
                    .flatten()
                    .map(|c| resource_ids.iter().map(|r| fx(&c, r).unwrap_or(0.0)).collect())
                    .unwrap_or_else(|| vec![0.0; resource_ids.len()]);
                def.research.push(ResearchItem { property, kind: st(&item, "type").unwrap_or_default(), research_time: fx(&item, "research_time").unwrap_or(0.0), cost });
            }
        }
        if let Ok(Some(denizen)) = node.child("DenizenComponent")
            && let Ok(Some(info)) = denizen.child("DeploymentInfo")
        {
            def.creation_offset = pt(&info, "creation_offset");
            def.deployment_offset = pt(&info, "deployment_offset");
        }
        def.structure = node.child("StructureComponent").ok().flatten().is_some();
        if let Ok(Some(city)) = node.child("CityComponent") {
            def.city_zone_radius = fx(&city, "zone_radius").unwrap_or(0.0);
        }
        def.city = node.child("CityComponent").ok().flatten().is_some();
        // Recruit list (582C40 targets, authored order).
        match aa_rules::native::recruit_data::actor_recruit_targets(catalog, id) {
            Ok(t) => p.recruits = t.ids,
            Err(e) => {
                if node.child("RecruitComponent").ok().flatten().is_some() {
                    note("recruit targets", &e);
                }
            }
        }
        // Organization component and its layout (466C4F, 4C2110, 4C3610).
        if let Ok(Some(org)) = node.child("OrganizationComponent") {
            match layouts.map(|l| (l, l.organization_component_fields(&org, id, None, PC))) {
                Some((layouts, Ok(o))) => {
                    def.supply_used = o.supply_used;
                    def.shortfall_damage_rate = f(o.shortfall_damage_rate);
                    def.morale_breakpoint = f(o.morale_breakpoint);
                    def.control_range = f(o.control_range);
                    def.operational_range = f(o.operational_range);
                    def.recruit_time = f(o.recruit_time);
                    def.recruit_full = o.recruit_full;
                    def.in_supply_detached = o.in_supply_detached;
                    def.entrenchment_time = f(o.entrenchment_time);
                    def.company_group_priority = o.company_group_priority;
                    def.layout = Some(o.layout_ids.clone());
                    if let Some(layout) = layouts.find_layout(&o.layout_ids) {
                        let mut parameters: Vec<Option<String>> = vec![None; layout.entries.len()];
                        for (entry, element) in &o.elements {
                            if let Some(slot) = parameters.get_mut(*entry as usize) {
                                *slot = Some(element.clone());
                            }
                        }
                        def.slot_entries = layout.slot_groups.clone();
                        def.leader_slot = (layout.primary_slot >= 0).then_some(layout.primary_slot as u16);
                        p.slots = layout.slot_groups.iter().map(|&e| parameters.get(e as usize).cloned().flatten()).collect();
                        def.slot_required = layout.slot_groups.iter().map(|&e| layout.entries.get(e as usize).is_some_and(|x| x.required)).collect();
                        for lf in &layout.formations {
                            let fid = lf
                                .formation
                                .and_then(|i| layouts.formations.iter().find(|f| f.ordinal == i))
                                .map(|f| f.ids.clone())
                                .unwrap_or_default();
                            let mut positions = Vec::with_capacity(layout.slot_groups.len());
                            for &entry in &layout.slot_groups {
                                let property = layout.entries.get(entry as usize).and_then(|e| e.property);
                                // k-th slot of a property takes its k-th position.
                                let k = layout.slot_groups.iter().take(positions.len()).filter(|&&e| layout.entries.get(e as usize).and_then(|x| x.property) == property).count();
                                let pos = lf.positions.iter().filter(|p| p.property == property).nth(k).map(|p| [f(p.offset_bits[0]), f(p.offset_bits[1])]).unwrap_or([0.0, 0.0]);
                                positions.push(pos);
                            }
                            def.formations.push(Formation { id: fid, positions });
                        }
                        let idx = |i: i32| (i >= 0 && (i as usize) < def.formations.len()).then_some(i as usize);
                        def.default_formation = idx(layout.default_formation).or(if def.formations.is_empty() { None } else { Some(0) });
                        def.combat_formation = idx(layout.combat_formation);
                        def.retreat_formation = idx(layout.retreat_formation);
                    } else {
                        note("layout", &format!("{} not found", o.layout_ids));
                    }
                }
                Some((_, Err(e))) => note("organization", &e),
                None => note("organization", &"layout data unavailable"),
            }
        }
        def.mover_class = if !def.has_component("MoverComponent") {
            MoverClass::None
        } else if def.flying || def.has_component("AircraftComponent") {
            MoverClass::Air
        } else if def.water_based {
            MoverClass::Water
        } else {
            MoverClass::Land
        };
        def.class = Some(if def.has_component("OrganizationComponent") {
            DefClass::Organization
        } else if p.unpacks_to.is_some() {
            DefClass::Truck
        } else if def.structure || def.has_component("CityComponent") || p.packs_to.is_some() {
            DefClass::Building
        } else if def.has_component("ElementComponent") {
            DefClass::Member
        } else {
            DefClass::Other
        });
        (def, p)
    }

    fn weapon(a: &LoadedAbility, defender: bool, damage_types: &[String]) -> Weapon {
        let d = &a.authored.definition.fields;
        let af = &a.authored.fields;
        let g = |m: &BTreeMap<String, TypedValue>, k: &str| m.get(k).and_then(num).unwrap_or(0.0);
        let s = |m: &BTreeMap<String, TypedValue>, k: &str| m.get(k).and_then(text).filter(|v| !v.is_empty()).map(str::to_owned);
        let mut main_type = None;
        let effects = a
            .effects
            .iter()
            .map(|e| {
                let (amount, damage_type) = match &e.data {
                    aa_rules::abilities::EffectData::Damage { damage_type, amount_bits } => (f(*amount_bits), Some(damage_type.id.clone())),
                    aa_rules::abilities::EffectData::Morale { amount_bits } => (f(*amount_bits), None),
                    aa_rules::abilities::EffectData::Heal { amount_bits, .. } => (f(*amount_bits), None),
                    _ => (0.0, None),
                };
                (e, amount, damage_type)
            })
            .collect::<Vec<_>>();
        let mut out_effects = Vec::new();
        for (e, amount, damage_type) in effects {
            if e.kind == 4 && main_type.is_none() {
                main_type = damage_type.clone();
            }
            out_effects.push(WeaponEffect {
                kind: e.kind,
                kind_name: aa_rules::commander::EFFECT_KINDS.get(e.kind as usize).map(|s| s.to_string()).unwrap_or_default(),
                amount,
                damage_type: damage_type.and_then(|dt| damage_types.iter().position(|d| d.eq_ignore_ascii_case(&dt))).map(|i| i as u32),
                targets_required: e.targets.required,
                targets_forbidden: e.targets.forbidden,
            });
        }
        Weapon {
            name: s(d, "IDS").or_else(|| a.ability_type.as_ref().map(|t| t.id.clone())).unwrap_or_else(|| if defender { "defender".into() } else { "ability".into() }),
            id: s(d, "IDS").unwrap_or_default(),
            range: g(d, "range"),
            min_range: g(d, "min_range"),
            recharge: f(a.recharge_bits),
            damage: f(a.power_bits),
            damage_type: main_type.and_then(|dt| damage_types.iter().position(|d| d.eq_ignore_ascii_case(&dt))).unwrap_or(0) as u32,
            area: g(d, "area"),
            max_targets: g(d, "max_targets").max(0.0) as u32,
            flags: a.flags,
            defender,
            recharge_time: g(d, "recharge_time"),
            reload_time: g(af, "reload_time"),
            targets_required: a.targets.required,
            targets_forbidden: a.targets.forbidden,
            projectile: a.projectile.as_ref().map(|p| p.id.clone()),
            ability_type: a.ability_type.as_ref().map(|p| p.id.clone()),
            turret: a.turret_index,
            volley_area: g(d, "volley_area"),
            attenuation_range_min: f(a.geometry.attenuation_min_bits),
            attenuation_factor_max: f(a.geometry.attenuation_factor_bits),
            impact_impulse: g(d, "impact_impulse"),
            priority: g(af, "priority"),
            fuzzy_range: g(af, "fuzzy_range"),
            effects: out_effects,
            animations: a.animations.first().map(|s| s.attacks.iter().map(|b| b.name.clone()).collect()).unwrap_or_default(),
            use_weight: g(af, "use_weight"),
            wait_weight: g(af, "wait_weight"),
            move_weight: g(af, "move_weight"),
            use_bonus: g(af, "use_bonus"),
            wait_bonus: g(af, "wait_bonus"),
            move_bonus: g(af, "move_bonus"),
            projectile_speed: 0.0,
            projectile_type: None,
        }
    }
}
