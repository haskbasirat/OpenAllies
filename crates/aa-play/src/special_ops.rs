//! Special operations: every kingdom's general (an original `Faction`,
//! Data/Generals) and its four abilities, bought with the kingdom's
//! experience. Owner: specials.
//!
//! Rendition of the exact ports (aa-combat `special_ops`, `admission`,
//! `execution`, `summon`, `periodic`; aa-economy `tick` / `transfers`): a
//! binary32 operation stored at once is the PC53 x87 result, a chain the
//! original keeps in an x87 register is evaluated in `f64` (PC53).
//! - experience (Commander `+0xC`): StartingExperience at the start
//!   (kingdom_start `4CDF10` clamp), ExperienceFactionGainPeriodic per second
//!   (`4CDFD0` from the economy tick) and, per gain of a kingdom's
//!   organization, ExperienceFactionGainMultiplier of it (`571A60` ->
//!   `4CEB70` -> `4E0FD0` -> `4CDFD0`), clamped to [0, experience_max];
//! - use: `KKC_FactionUseAbilityOrder` `61E190` -> `KKC_Faction::UseAbility`
//!   `4CDE00`: admission `4C8F20` (target flags `4C5840` / `4C5A20`,
//!   eligibility `4C8740`, cost `4E24B0`, summon and recharge `4C8A40`; the
//!   faction range is FLT_MAX, FactionData `48BD05`), then `4C9EF0` ->
//!   `4C9550` at GWorld+0xE0: `primary_island_only`, the map clamp `552640`,
//!   the self effects and the payment (`4E2590` / `4CDD90`), a projectile's
//!   impact after its life (`580BC0` / `580920`, admitted again at phase 2),
//!   the area (`4C9C90`: `max_targets` nearest, phase-3 admission, the
//!   attenuation), the per-effect target filters;
//! - effects (`+8` Apply of each effect class): MAIN_DAMAGE and ADD_DAMAGE
//!   through combat (`4CA400`), MORALE_GAIN / MORALE_LOSS (`4CA050`), HEAL
//!   (`4CA0B0`), ENCHANTMENT (`4CA130` / `581700`: not twice, enemy immunity,
//!   expiry `5031B0`, the immunity purge `4FE6F0`), SUMMON (`4CCA70`: at the
//!   target point, the open position `554B30`, the summoned list, not
//!   controllable, the attack order of `4CC470`, the transport's flight
//!   `4CB110`), STEAL_RESOURCES (`4CA680`), DESCRIBE_ECONOMY (`4CC050`);
//! - the Properties component Tick `5818D0` (queue 1, once a second): the
//!   expiry, then periodic heal and periodic damage of the enchantments and
//!   of the actors' own properties (supply drop decay...).
//!
//! The enchantment modifiers reach combat (AV, DV, HEALTH, RECHARGE_TIME,
//! RESIST_DAMAGE, ABILITY_EFFECT), movement ([`speed_multiplier`]) and
//! supply ([`has_flag`] SUPPLIED) through the queries below.
use crate::combat;
use crate::data::{DefClass, DefId, GameData};
use crate::world::{AbilityTarget, ActorId, Command, Event, KingdomId, OrgId, World};
use crate::DT;
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[path = "special_ops/load.rs"]
mod load;
#[path = "special_ops/sai.rs"]
pub mod sai;
#[cfg(test)]
#[path = "special_ops/tests.rs"]
mod tests;

/// AbilityDefinition `+54` bits (aa_rules::ability_load::DEFINITION_FLAGS).
pub const FLAG_ENCHANTMENT: u32 = 1 << 2;
pub const FLAG_BOMBARD: u32 = 1 << 3;
pub const FLAG_CENTER_ON_SOURCE: u32 = 1 << 4;
pub const FLAG_TARGET_LOCATION: u32 = 1 << 5;
pub const FLAG_HEAL: u32 = 1 << 6;
pub const FLAG_SUMMON_MULTIPLE: u32 = 1 << 8;
pub const FLAG_SUMMON_UNIQUE: u32 = 1 << 9;
/// `primary_island_only` (`4C9550` +54 bit 10).
pub const FLAG_PRIMARY_ISLAND_ONLY: u32 = 1 << 10;
/// Loader bit 17: the definition has an enchantment effect.
pub const FLAG_HAS_ENCHANTMENT: u32 = 1 << 17;
/// Effect kinds (aa_rules::commander::EFFECT_KINDS).
pub const SUMMON: u32 = 0;
pub const MORALE_LOSS: u32 = 1;
pub const MORALE_GAIN: u32 = 2;
pub const HEAL: u32 = 3;
pub const MAIN_DAMAGE: u32 = 4;
pub const ADD_DAMAGE: u32 = 5;
pub const ENCHANTMENT: u32 = 6;
pub const CAPTURE: u32 = 7;
pub const STEAL_RESOURCES: u32 = 8;
pub const DESCRIBE_ECONOMY: u32 = 9;
/// Modifier attributes (aa_rules::terrain_speed::ATTRIBUTES).
pub const AV: u32 = 0;
pub const AV_MELEE: u32 = 1;
pub const AV_RANGED: u32 = 2;
pub const DV: u32 = 3;
pub const SPEED_MAX: u32 = 13;
pub const RESUPPLY_RATE: u32 = 14;
pub const HEALTH: u32 = 19;
pub const RECHARGE_TIME: u32 = 24;
/// Target type bits of the predefined registry (targets.rs PREDEFINED_TARGETS).
const BIT_FRIEND: u32 = 1 << 0;
const BIT_ENEMY: u32 = 1 << 1;
const BIT_NEUTRAL: u32 = 1 << 2;
const BIT_INDEPENDENT: u32 = 1 << 3;
const BIT_OWN: u32 = 1 << 4;
const BIT_SELF: u32 = 1 << 5;
const BIT_ORGANIZATION: u32 = 1 << 9;
const BIT_UNDER_CONSTRUCTION: u32 = 1 << 10;
const BIT_CAPTURABLE: u32 = 1 << 11;
const BIT_BODY: u32 = 1 << 12;
const BIT_ACTOR: u32 = 1 << 13;
const BIT_INVULNERABLE: u32 = 1 << 14;
/// The Properties component's COMPONENTS slot (`crate::map::PROPERTIES_SLOT`).
const PROPERTIES_SLOT: u8 = crate::map::PROPERTIES_SLOT;
/// `554B30`'s search distance for a summoned object (4CCA70).
const SUMMON_SEARCH_DISTANCE: f32 = 12.0;
/// `0x85daa0` / `0x85da80` of -pi/4 (aa_spatial::placement): the ring turn.
const ROTATION_SIN_BITS: u32 = 0xbf35_04ef;
const ROTATION_COS_BITS: u32 = 0x3f35_04f7;

// ---------------------------------------------------------------------------
// Data

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeneralData {
    pub key: String,
    pub generals: Vec<General>,
    /// Ego IDS (lower case) -> its GeneralPowers (Data/SAI egos).
    pub ego_powers: Vec<(String, Vec<PowerConfig>)>,
    /// Generals or egos whose reader failed.
    pub notes: Vec<String>,
}

impl GeneralData {
    pub fn general(&self, id: &str) -> Option<usize> {
        self.generals.iter().position(|g| g.id.eq_ignore_ascii_case(id))
    }
    pub fn powers(&self, ego: &str) -> &[PowerConfig] {
        self.ego_powers.iter().find(|(e, _)| e.eq_ignore_ascii_case(ego)).map_or(&[], |(_, p)| p.as_slice())
    }
}

/// One original Faction of template AAGeneral (Templates/template_faction*.tgi).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct General {
    pub id: String,
    /// Localization keys.
    pub name: String,
    pub description: String,
    pub staging_name: String,
    /// icon_texture_ids (Generals/<Nation>/...).
    pub icon: Option<String>,
    pub portrait: Option<String>,
    pub public: bool,
    /// FactionData `+3C` experience_max (AAGeneral 5000; negative: no cap).
    pub experience_max: f32,
    pub abilities: Vec<SpecialOp>,
}

/// One Faction Ability (AASpecialOp* template and its AbilityDefinition).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpecialOp {
    /// AbilityDefinition IDS.
    pub id: String,
    pub name: String,
    pub description: String,
    /// `/UI/%s/Game/SpecialOps/Icon*.png`.
    pub icon: Option<String>,
    pub target_location_vfx: Option<String>,
    /// cost_faction_experience (template_ability_special_op_aa.tgi).
    pub cost: f32,
    pub recharge: f32,
    pub range: f32,
    pub area: f32,
    pub volley_area: f32,
    pub max_targets: u32,
    /// Definition target filter (bits of `GameData::target_types`).
    pub required: u32,
    pub forbidden: u32,
    pub flags: u32,
    /// Main power (`4BE3F0(0)`, the MAIN_DAMAGE amount).
    pub power: f32,
    pub attenuation_min: f32,
    pub attenuation_factor: f32,
    pub projectile: Option<String>,
    /// The projectile's projectile_life: when the effects take place.
    pub projectile_life: f32,
    pub ability_type: Option<String>,
    pub effects: Vec<OpEffect>,
}

impl SpecialOp {
    /// `4C8740` accepts a location: area, volley area or target_location.
    pub fn accepts_location(&self) -> bool {
        self.area > 0.0 || self.volley_area > 0.0 || self.flags & FLAG_TARGET_LOCATION != 0
    }
    /// `4BE8E0`: the object of the first SUMMON effect.
    pub fn summon(&self) -> Option<&Summon> {
        self.effects.iter().find(|e| e.kind == SUMMON).and_then(|e| e.summon.as_ref())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OpEffect {
    pub kind: u32,
    pub required: u32,
    pub forbidden: u32,
    pub hide: bool,
    pub amount: f32,
    pub damage_type: Option<u32>,
    pub heal_type: u32,
    pub enchantment: Option<Enchantment>,
    pub summon: Option<Summon>,
    /// STEAL_RESOURCES amounts in `GameData::resources` order.
    pub steal: Vec<f32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Enchantment {
    /// EnchantmentProperty IDS.
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    /// `+60` duration (s).
    pub duration: f32,
    /// `+64` EnchantmentType IDS (lower case).
    pub kind: String,
    /// `+38` required properties of the affected actor.
    pub required_properties: Vec<String>,
    pub modifiers: Vec<OpModifier>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OpModifier {
    Add { attribute: u32, amount: f32 },
    Multiply { attribute: u32, amount: f32 },
    Resist { damage_type: u32, amount: f32 },
    PeriodicDamage { damage_type: u32, amount: f32 },
    PeriodicHeal { amount: f32 },
    Immunity { kind: Option<String> },
    AbilityEffect { ability_type: String, effect: Box<OpEffect> },
    Flag { flag: String },
    Other,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Summon {
    pub object: String,
    pub controllable: bool,
    pub property: Option<String>,
    /// summon_unique filter (`+1C` unique_target_flags).
    pub unique_required: u32,
    pub unique_forbidden: u32,
    /// The object's transport_actor (an air drop from the map edge).
    pub transport: Option<String>,
    pub ethereal: bool,
}

/// SAI GeneralPowerActorRequirements (`4AFB60`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Requirements {
    pub max_health: f32,
    pub must_be_engaged: bool,
    pub must_not_be_engaged: bool,
    pub friendly: bool,
    pub enemy: bool,
    pub owned: bool,
    /// Any one of them.
    pub properties: Vec<String>,
    pub multiplier: f32,
}
impl Default for Requirements {
    fn default() -> Self {
        Requirements { max_health: 1.0, must_be_engaged: false, must_not_be_engaged: false, friendly: false, enemy: false, owned: false, properties: Vec::new(), multiplier: 1.0 }
    }
}

/// One Priority factor of a GeneralPower (CV, ActorValue, EnemyInfluence...).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Factor {
    pub minimum: f32,
    pub multiplier: f32,
    pub maximum: f32,
    pub consider_friendly: bool,
    pub consider_enemy: bool,
    pub consider_owned: bool,
    pub categories: Vec<Requirements>,
    pub exponent: f32,
    pub exponent_over_one: f32,
    pub exponent_under_one: f32,
    pub count_owned_only: bool,
    pub resources: Vec<(String, f32)>,
}

/// An Ego GeneralPower (Templates/template_sai.tgi GeneralPower, `4AFD50`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PowerConfig {
    pub ability: String,
    /// 0 NONE, 1 NEAR_ACTOR, 2 ON_ACTOR, 3 FOG.
    pub location: u32,
    pub actor_requirements: Requirements,
    pub radius: f32,
    pub candidates: u32,
    pub no_friendly: bool,
    pub no_enemy: bool,
    pub no_owned: bool,
    pub requires_supply: bool,
    pub base: f32,
    pub minimum: f32,
    pub fuzzy: f32,
    pub cv: Factor,
    pub actor_value: Factor,
    pub actor_economy: Factor,
    pub enemy_influence: Factor,
    pub influence_ratio: Factor,
    pub time_since_explored: Factor,
    pub economy_generated: Factor,
    pub flare_priority: f32,
    pub flare_duration: f32,
}

// ---------------------------------------------------------------------------
// State

/// What a special operation lands on: an actor, or an organization (the
/// original's organization actor: resupply targets +ORGANIZATION).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpTarget {
    Actor(ActorId),
    Org(OrgId),
}

/// A GTID of an ability's summoned list (`4C89A0`): the summoned actor, or
/// the organization actor of a summoned regiment (`4FA150`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Summoned {
    Actor(ActorId),
    Org(OrgId),
}

/// The kingdom's Commander (kingdom `+0x230`): its general, experience and
/// the per-ability use state.
#[derive(Clone, Debug, Default)]
pub struct Commander {
    pub general: Option<usize>,
    /// `+0xC` balance.
    pub experience: f32,
    /// FactionData `+3C`.
    pub maximum: f32,
    /// Ability `+10` last use, GWorld+0xE0 of the use (None: never used, the
    /// `4C8650` -FLT_MAX).
    pub last_use: Vec<Option<f32>>,
    /// Ability `+24` summoned GTIDs, head first (`4C89A0`).
    pub summoned: Vec<Vec<Summoned>>,
    pub use_count: Vec<u32>,
}

/// An enchantment instance (`503140`), in its target's `+8` vector order.
#[derive(Clone, Debug)]
pub struct Enchanted {
    pub target: OpTarget,
    pub kingdom: KingdomId,
    pub general: usize,
    pub ability: usize,
    pub effect: usize,
    /// `+4`: GWorld+0xE0 when it was added.
    pub created: f32,
}

/// An enchantment on a kingdom (a +SELF ability such as Radar Scan: the
/// target is the kingdom itself, `4C5840` +SELF).
#[derive(Clone, Debug)]
pub struct KingdomEnchanted {
    pub kingdom: KingdomId,
    pub general: usize,
    pub ability: usize,
    pub effect: usize,
    pub created: f32,
}

/// Effects waiting for a projectile's life or a transport's arrival.
#[derive(Clone, Debug)]
pub enum Pending {
    Impact { kingdom: KingdomId, general: usize, ability: usize, at: f32, center: Vec2, target: Option<OpTarget> },
}

/// Aggregated enchantment modifiers of one actor or organization.
#[derive(Clone, Debug, Default)]
pub struct Mods {
    pub av: f32,
    pub av_melee: f32,
    pub av_ranged: f32,
    pub dv: f32,
    /// Sums of the MULTIPLY amounts (the multiplier is 1 + sum).
    pub health: f32,
    pub speed: f32,
    pub recharge: f32,
    pub resupply: f32,
    /// RESIST_DAMAGE by damage type.
    pub resist: Vec<f32>,
    pub immune: Vec<Option<String>>,
    /// ABILITY_EFFECT: (ability type, the nested effect).
    pub ability_effects: Vec<(String, OpEffect)>,
    /// The ModifierState `+98` periodic damage records (type, amount per
    /// Properties tick), in table order.
    pub periodic_damage: Vec<(u32, f32)>,
    /// Sum of [`Mods::periodic_heals`].
    pub periodic_heal: f32,
    /// The `+A4` periodic heals, in table order (`5818D0` heals one by one).
    pub periodic_heals: Vec<f32>,
    pub flags: Vec<String>,
}

/// Whose Properties component ticks (`5818D0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertiesOwner {
    Actor(ActorId),
    Org(OrgId),
    Kingdom(KingdomId),
}

/// The SAI map's island of every tile and its primary island (`7ACB00`
/// static analysis, `7AA2A0`), for `primary_island_only`.
#[derive(Clone, Debug, Default)]
struct Islands {
    key: usize,
    shift: u32,
    width: u32,
    height: u32,
    extent: [f32; 2],
    tile: Vec<i32>,
    primary: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct SpecialOpsState {
    pub data: Option<Arc<GeneralData>>,
    /// By kingdom id.
    pub commanders: Vec<Commander>,
    pub enchantments: Vec<Enchanted>,
    pub pending: Vec<Pending>,
    /// By actor id / organization id (default when nothing enchants it).
    pub mods: Vec<Mods>,
    pub org_mods: Vec<Mods>,
    /// Actors summoned without control (`set_uncontrollable`).
    pub uncontrollable: Vec<ActorId>,
    /// StartingExperience of the session (None: the world rule default).
    pub starting_experience: Option<f32>,
    org_experience: Vec<f32>,
    /// Periodic damage (type, amount) and heals of each definition's own properties.
    def_periodic: Vec<(Vec<(u32, f32)>, Vec<f32>)>,
    data_key: usize,
    /// The `routing` target type bit (Properties `+D8`, the organization's
    /// routing property).
    routing_bit: Option<u32>,
    pub(crate) sai: sai::SaiState,
    /// Enchantments on kingdoms (+SELF abilities).
    pub kingdom_enchantments: Vec<KingdomEnchanted>,
    /// The scheduler delivers the Properties Tick itself ([`properties_tick`]).
    properties_leaf: bool,
    /// Combat credits the organizations' gains itself ([`credit_experience`]).
    direct_experience: bool,
    islands: Option<Islands>,
}

/// Install the generals' data (load it with [`GeneralData::load`]).
pub fn attach(world: &mut World, data: Arc<GeneralData>) {
    world.special_ops.data = Some(data);
}

/// The session's StartingExperience world rule (staging screen value).
pub fn set_starting_experience(world: &mut World, value: f32) {
    world.special_ops.starting_experience = Some(value);
    for c in world.special_ops.commanders.iter_mut().filter(|c| c.general.is_some()) {
        c.experience = clamp_experience(value, c.maximum);
    }
}

/// `4CDF10`: below 0 -> 0, then the maximum when it is >= 0 and smaller.
fn clamp_experience(value: f32, max: f32) -> f32 {
    let v = if value < 0.0 { 0.0 } else { value };
    if max >= 0.0 && max < v { max } else { v }
}

fn commander_mut(world: &mut World, kingdom: KingdomId) -> &mut Commander {
    let i = kingdom as usize;
    let c = &mut world.special_ops.commanders;
    if c.len() <= i {
        c.resize_with(i + 1, Commander::default);
    }
    &mut c[i]
}

/// A saved commander's words (0x4C8650): its balance (+0xC) and each ability's last
/// use (+10; -FLT_MAX for never).
pub fn restore_commander(world: &mut World, kingdom: KingdomId, experience: f32, last_use: Vec<f32>) {
    let c = commander_mut(world, kingdom);
    c.experience = experience;
    for (slot, time) in c.last_use.iter_mut().zip(last_use) {
        *slot = (time > -f32::MAX).then_some(time);
    }
}

/// Give `kingdom` its general (Faction IDS); None picks one of the nation's
/// public factions at random, as the setup does for "Random" (0x50E8B0).
pub fn set_general(world: &mut World, kingdom: KingdomId, general: Option<&str>) -> Option<usize> {
    let data = world.special_ops.data.clone()?;
    // (core) A general WorldParameters 0x50E8B0 picked at random during the
    // map restore (`World::new_skirmish_with`) is the one for None.
    let picked = world.books.kingdom(kingdom).and_then(|b| b.general.clone());
    let index = match general.or(picked.as_deref()).and_then(|g| data.general(g)) {
        Some(i) => Some(i),
        None => {
            let nation = world.kingdoms.iter().find(|k| k.id == kingdom).map(|k| k.nation.clone()).unwrap_or_default();
            let factions: Vec<usize> = world
                .data
                .nation(&nation)
                .map(|n| n.factions.iter().filter_map(|f| data.general(f)).filter(|&i| data.generals[i].public).collect())
                .unwrap_or_default();
            if factions.is_empty() {
                None
            } else {
                let r = world.random_at(crate::trace::sites::SPECIAL_OPS_FACTION);
                Some(factions[((r * factions.len() as f32) as usize).min(factions.len() - 1)])
            }
        }
    }?;
    let starting = world.special_ops.starting_experience.or_else(|| world.data.rule_ci("StartingExperience")).unwrap_or(0.0);
    let g = &data.generals[index];
    let n = g.abilities.len();
    let max = g.experience_max;
    let c = commander_mut(world, kingdom);
    c.general = Some(index);
    c.maximum = max;
    c.experience = clamp_experience(starting, max);
    c.last_use = vec![None; n];
    c.summoned = vec![Vec::new(); n];
    c.use_count = vec![0; n];
    Some(index)
}

/// Every kingdom without a general gets one at random (the setup default).
pub fn assign_missing_generals(world: &mut World) {
    let ids: Vec<KingdomId> = world.kingdoms.iter().map(|k| k.id).collect();
    for k in ids {
        let has = world.special_ops.commanders.get(k as usize).is_some_and(|c| c.general.is_some());
        if !has {
            set_general(world, k, None);
        }
    }
}

// ---------------------------------------------------------------------------
// Queries (HUD, opponent, combat, movement, supply)

/// GWorld+0xE0 (`[B36534]+E0`), the world time an ability use (`4C9EF0`),
/// its recharge (`4C83E0`) and an enchantment (`503140` / `5031B0`) read:
/// the begin time of the last completed tick while the decrees run, the
/// current tick's begin time inside the update.
pub(crate) fn native_now(world: &World) -> f32 {
    f32::from_bits(world.clock.begin.0)
}

pub fn general(world: &World, kingdom: KingdomId) -> Option<&General> {
    let c = world.special_ops.commanders.get(kingdom as usize)?;
    Some(&world.special_ops.data.as_ref()?.generals[c.general?])
}

/// The kingdom's experience and its maximum (FactionExperience `0x604130`).
pub fn experience(world: &World, kingdom: KingdomId) -> Option<(f32, f32)> {
    let c = world.special_ops.commanders.get(kingdom as usize)?;
    c.general?;
    Some((c.experience, c.maximum))
}

/// One ability as the HUD shows it (`6073C0`, tooltip `6081B0`).
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityView {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub cost: f32,
    /// Full recharge and the seconds left (`4C8220`, `4C83E0`).
    pub recharge: f32,
    pub remaining: f32,
    /// `4E24B0`.
    pub affordable: bool,
    /// `4C8A40` code (0 usable, 8 recharging, 9 a summon still alive, 10 unique).
    pub available: u32,
    /// Enabled (`6073C0`): affordable, recharged and not 9/10.
    pub usable: bool,
    pub accepts_location: bool,
    pub area: f32,
    pub target_location_vfx: Option<String>,
    /// The definition targets +SELF: used at once, without targeting
    /// (`KKC_GameInterface::CommandUseFactionAbility` 0x612912).
    pub self_target: bool,
    pub volley_area: f32,
    /// The display name key of the summoned object (the
    /// ui_error_faction_ability_already_summoned message, `0x4BE8E0` ->
    /// `0x4CA3E0` +0x10).
    pub summon_name: Option<String>,
}

pub fn abilities(world: &World, kingdom: KingdomId) -> Vec<AbilityView> {
    let Some(g) = general(world, kingdom) else { return Vec::new() };
    (0..g.abilities.len()).filter_map(|i| view(world, kingdom, i)).collect()
}

pub fn view(world: &World, kingdom: KingdomId, index: usize) -> Option<AbilityView> {
    let g = general(world, kingdom)?;
    let op = g.abilities.get(index)?;
    let remaining = recharge_remaining(world, kingdom, index);
    let affordable = can_pay(world, kingdom, op);
    let available = available(world, kingdom, index);
    Some(AbilityView {
        index,
        id: op.id.clone(),
        name: op.name.clone(),
        description: op.description.clone(),
        icon: op.icon.clone(),
        cost: op.cost,
        recharge: op.recharge,
        remaining,
        affordable,
        available,
        usable: affordable && remaining <= 0.0 && available != 9 && available != 10,
        accepts_location: op.accepts_location(),
        area: op.area,
        target_location_vfx: op.target_location_vfx.clone(),
        self_target: op.required & BIT_SELF != 0,
        volley_area: op.volley_area,
        summon_name: op.summon().and_then(|s| world.data.find(&s.object)).map(|d| world.data.def(d).name.clone()),
    })
}

/// `4C83E0`: the recharge left. Every Commander ability has recharge_bypass
/// (`4CE1D0`), so the CheatNoWait rule (`[B39AB0]+1C0`) skips it; otherwise
/// `recharge - (GWorld+0xE0 - last use)` in the x87 register, zero when not
/// above zero. A never used ability's -FLT_MAX last use gives zero.
fn recharge_remaining(world: &World, kingdom: KingdomId, index: usize) -> f32 {
    let Some(g) = general(world, kingdom) else { return 0.0 };
    let c = &world.special_ops.commanders[kingdom as usize];
    if world.data.rule_ci("CheatNoWait").unwrap_or(0.0) != 0.0 {
        return 0.0;
    }
    let Some(t) = c.last_use.get(index).copied().flatten() else { return 0.0 };
    let left = f64::from(g.abilities[index].recharge) - (f64::from(native_now(world)) - f64::from(t));
    if left <= 0.0 { 0.0 } else { left as f32 }
}

/// `4E24B0` -> `4CDD60`: the integer experience cost (`FILD`) is at most
/// the balance (the special operations cost no resources).
fn can_pay(world: &World, kingdom: KingdomId, op: &SpecialOp) -> bool {
    world.special_ops.commanders.get(kingdom as usize).is_some_and(|c| f64::from(op.cost) <= f64::from(c.experience))
}

/// A summoned GTID still resolves (`4C89F0`): the thing is not destroyed.
fn resolves(world: &World, s: Summoned) -> bool {
    match s {
        Summoned::Actor(a) => world.actors.get(a as usize).is_some_and(|x| !x.removed),
        Summoned::Org(o) => world.orgs.get(o as usize).is_some_and(|x| x.alive),
    }
}

/// `4C8A40` summon and recharge checks (a faction has no mana, code 7).
fn available(world: &World, kingdom: KingdomId, index: usize) -> u32 {
    let Some(g) = general(world, kingdom) else { return 1 };
    let op = &g.abilities[index];
    let c = &world.special_ops.commanders[kingdom as usize];
    // 4C89F0 drops the stale GTIDs first (here: `prune_summoned` each tick).
    let live = c.summoned.get(index).is_some_and(|s| s.iter().any(|&x| resolves(world, x)));
    if op.flags & FLAG_SUMMON_MULTIPLE == 0 && live {
        return 9;
    }
    // +54 bit 9: an owned actor (kingdom +284 list, not destroyed) whose
    // 4CF410 flags match the summon's unique filter.
    if op.flags & FLAG_SUMMON_UNIQUE != 0
        && let Some(s) = op.summon()
    {
        let unique = world.actors.iter().any(|a| {
            !a.removed && a.alive && a.kingdom == Some(kingdom) && {
                let flags = class_flags(world, OpTarget::Actor(a.id));
                flags & s.unique_required == s.unique_required && flags & s.unique_forbidden == 0
            }
        });
        if unique {
            return 10;
        }
    }
    if recharge_remaining(world, kingdom, index) > 0.0 {
        return 8;
    }
    0
}

/// The enchantment modifiers of an actor.
pub fn mods(world: &World, actor: ActorId) -> Option<&Mods> {
    world.special_ops.mods.get(actor as usize)
}

/// SPEED_MAX multiplier of the actor's enchantments (for movement).
pub fn speed_multiplier(world: &World, actor: ActorId) -> f32 {
    let clamp = world.data.rule_ci("ModifierAttributeClampMultiply").unwrap_or(0.05);
    mods(world, actor).map_or(1.0, |m| (1.0 + m.speed).max(clamp))
}

/// A FLAG modifier of an actor's enchantments (MINIMAP_NO_FOG...).
pub fn has_flag(world: &World, actor: ActorId, flag: &str) -> bool {
    mods(world, actor).is_some_and(|m| m.flags.iter().any(|f| f.eq_ignore_ascii_case(flag)))
}

/// A FLAG modifier of an organization's enchantments (resupply: SUPPLIED).
pub fn org_has_flag(world: &World, org: OrgId, flag: &str) -> bool {
    world.special_ops.org_mods.get(org as usize).is_some_and(|m| m.flags.iter().any(|f| f.eq_ignore_ascii_case(flag)))
}

/// RESUPPLY_RATE multiplier of an organization's enchantments.
pub fn org_resupply_multiplier(world: &World, org: OrgId) -> f32 {
    world.special_ops.org_mods.get(org as usize).map_or(1.0, |m| (1.0 + m.resupply).max(0.05))
}

/// Actors a special operation summoned without control.
pub fn controllable(world: &World, actor: ActorId) -> bool {
    !world.special_ops.uncontrollable.contains(&actor)
}

/// A kingdom's own enchantments carry the FLAG modifier (e.g. Radar Scan's
/// MINIMAP_NO_FOG: the minimap shows everything while it lasts).
pub fn kingdom_has_flag(world: &World, kingdom: KingdomId, flag: &str) -> bool {
    let Some(data) = world.special_ops.data.as_ref() else { return false };
    world.special_ops.kingdom_enchantments.iter().filter(|e| e.kingdom == kingdom).any(|e| {
        data.generals[e.general].abilities[e.ability].effects[e.effect]
            .enchantment
            .as_ref()
            .is_some_and(|en| en.modifiers.iter().any(|m| matches!(m, OpModifier::Flag { flag: f } if f.eq_ignore_ascii_case(flag))))
    })
}

/// The enchantments on a target: (property name key, icon, seconds left).
pub fn enchantments_on(world: &World, target: OpTarget) -> Vec<(String, Option<String>, f32)> {
    let Some(data) = world.special_ops.data.as_ref() else { return Vec::new() };
    let now = native_now(world);
    world
        .special_ops
        .enchantments
        .iter()
        .filter(|e| e.target == target)
        .filter_map(|e| {
            let en = data.generals[e.general].abilities[e.ability].effects[e.effect].enchantment.as_ref()?;
            Some((en.name.clone(), en.icon.clone(), (en.duration - (now - e.created)).max(0.0)))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Targets

/// A target is there: the actor not dead (actor `+8` bit 0), the
/// organization alive with a living member.
fn alive(world: &World, t: OpTarget) -> bool {
    match t {
        OpTarget::Actor(a) => world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed),
        OpTarget::Org(o) => world.orgs.get(o as usize).is_some_and(|x| x.alive && x.members.iter().any(|&m| world.actors.get(m as usize).is_some_and(|a| a.alive))),
    }
}

pub(crate) fn position(world: &World, t: OpTarget) -> Vec2 {
    match t {
        OpTarget::Actor(a) => world.actors[a as usize].pos.truncate(),
        OpTarget::Org(o) => world.combat.org(o).map(|s| s.position).filter(|p| *p != Vec2::ZERO).unwrap_or(world.orgs[o as usize].anchor),
    }
}

fn owner(world: &World, t: OpTarget) -> Option<KingdomId> {
    match t {
        OpTarget::Actor(a) => world.actors[a as usize].kingdom,
        OpTarget::Org(o) => Some(world.orgs[o as usize].kingdom),
    }
}

fn def_of(world: &World, t: OpTarget) -> DefId {
    match t {
        OpTarget::Actor(a) => world.actors[a as usize].def,
        OpTarget::Org(o) => world.orgs[o as usize].def,
    }
}

fn relation(world: &World, a: Option<KingdomId>, b: Option<KingdomId>) -> u8 {
    world.books.relation(&world.kingdoms, a, b)
}

/// Kingdom virtual `+0x138` (`4E5250`): owner itself or relation friend.
fn allied(world: &World, a: Option<KingdomId>, b: Option<KingdomId>) -> bool {
    matches!(relation(world, a, b), crate::books::SELF | crate::books::FRIEND)
}

/// Kingdom virtual `+0x140` (`4E52A0`): relation enemy.
fn enemy(world: &World, a: Option<KingdomId>, b: Option<KingdomId>) -> bool {
    relation(world, a, b) == crate::books::ENEMY
}

/// Actor `+0x100` bit 21: under construction (placed by a builder, or a
/// building unpacking from its truck).
fn under_construction(world: &World, t: OpTarget) -> bool {
    match t {
        OpTarget::Actor(a) => {
            world.books.actor(a).is_some_and(|b| b.constructing || b.deploy.as_ref().is_some_and(|d| d.kind == crate::books::DeployKind::Unpacking))
        }
        OpTarget::Org(_) => false,
    }
}

/// `4CF410`: ActorData `+1B8` (the properties' enabled target flags, a
/// ConstructionComponent's constructable) OR the Properties `+D8` runtime
/// flags (here: an organization's routing property).
fn class_flags(world: &World, t: OpTarget) -> u32 {
    let mut flags = world.data.def(def_of(world, t)).target_flags;
    if let Some(bit) = world.special_ops.routing_bit {
        let org = match t {
            OpTarget::Org(o) => Some(o),
            OpTarget::Actor(a) => world.actors[a as usize].org,
        };
        if org.and_then(|o| world.combat.org(o)).is_some_and(|s| s.routing.is_some()) {
            flags |= 1 << bit;
        }
    }
    flags
}

/// `4C5840` flags of a target seen from the kingdom `source` (the context
/// source and allegiance of a faction ability).
fn target_flags(world: &World, source: KingdomId, t: OpTarget) -> u32 {
    let def = world.data.def(def_of(world, t));
    let org = matches!(t, OpTarget::Org(_));
    let mut flags = class_flags(world, t) | BIT_ACTOR;
    let k = owner(world, t);
    // allegiance +138 allied -> friend, +140 enemy -> enemy, else neutral
    flags |= if allied(world, Some(source), k) {
        BIT_FRIEND
    } else if enemy(world, Some(source), k) {
        BIT_ENEMY
    } else {
        BIT_NEUTRAL
    };
    if k == Some(source) {
        flags |= BIT_OWN;
    }
    // +7C Organization component
    if org {
        flags |= BIT_ORGANIZATION;
    }
    if under_construction(world, t) {
        flags |= BIT_UNDER_CONSTRUCTION;
    }
    // +60 Body: 4D1120 capturable (not under construction, no CityComponent,
    // the definition's +290 captureable byte)
    let body = !org && def.health > 0.0;
    if body && def.captureable && !def.city && !under_construction(world, t) {
        flags |= BIT_CAPTURABLE;
    }
    // The source is a kingdom: never the target itself (+SELF goes to the
    // kingdom, `KingdomEnchanted`), no organization to share, no summoner.
    // No owner, or an independent owner (kingdom +190 == 1).
    if k.is_none_or(|k| world.kingdoms.get(k as usize).is_some_and(|x| x.independent)) {
        flags |= BIT_INDEPENDENT;
    }
    if body {
        flags |= BIT_BODY;
    }
    // +8 bit 11
    if def.invulnerable {
        flags |= BIT_INVULNERABLE;
    }
    flags
}

fn matches(required: u32, forbidden: u32, flags: u32) -> bool {
    flags & required == required && flags & forbidden == 0
}

/// `4C8740`: an actor (targetable, `+8` bit 14) or organization the ability
/// can target.
fn eligible(world: &World, kingdom: KingdomId, op: &SpecialOp, t: OpTarget) -> bool {
    if !alive(world, t) || !world.data.def(def_of(world, t)).targetable {
        return false;
    }
    matches(op.required, op.forbidden, target_flags(world, kingdom, t))
}

/// `581280(property, 0)`: the target holds the property, from any source
/// (an enchantment, or one of its own properties).
fn has_property(world: &World, t: OpTarget, property: &str) -> bool {
    if world.data.def(def_of(world, t)).properties.iter().any(|p| p.eq_ignore_ascii_case(property)) {
        return true;
    }
    if let OpTarget::Actor(a) = t
        && world.books.actor(a).is_some_and(|b| b.properties.iter().any(|p| p.eq_ignore_ascii_case(property)))
    {
        return true;
    }
    let Some(data) = world.special_ops.data.as_ref() else { return false };
    world.special_ops.enchantments.iter().any(|e| {
        e.target == t && data.generals[e.general].abilities[e.ability].effects[e.effect].enchantment.as_ref().is_some_and(|x| x.id.eq_ignore_ascii_case(property))
    })
}

fn target_mods(world: &World, t: OpTarget) -> Option<&Mods> {
    match t {
        OpTarget::Actor(a) => world.special_ops.mods.get(a as usize),
        OpTarget::Org(o) => world.special_ops.org_mods.get(o as usize),
    }
}

/// `501610`: the target's modifier state is immune to the enchantment type.
fn immune(world: &World, t: OpTarget, kind: &str) -> bool {
    target_mods(world, t).is_some_and(|m| m.immune.iter().any(|k| k.as_deref().is_none_or(|k| k.eq_ignore_ascii_case(kind))))
}

/// `4C8F20` 0x20000: a hostile target immune to every enchantment of the
/// ability (an effect without a property is always permitted).
fn immune_to_all(world: &World, kingdom: KingdomId, op: &SpecialOp, t: OpTarget) -> bool {
    op.flags & FLAG_HAS_ENCHANTMENT != 0
        && !allied(world, owner(world, t), Some(kingdom))
        && !op.effects.iter().any(|e| e.enchantment.as_ref().is_none_or(|en| !immune(world, t, &en.kind)))
}

/// `4C8F20` 4 + 0x20000 (code 6): the target already holds a property of
/// one of the ability's enchantment effects.
fn already_enchanted(world: &World, op: &SpecialOp, t: OpTarget) -> bool {
    op.flags & FLAG_ENCHANTMENT != 0
        && op.flags & FLAG_HAS_ENCHANTMENT != 0
        && op.effects.iter().filter(|e| e.kind == ENCHANTMENT).filter_map(|e| e.enchantment.as_ref()).any(|en| has_property(world, t, &en.id))
}

// ---------------------------------------------------------------------------
// Command

/// Command::UseAbility (`61E190` -> `4CDE00`); Err is the refusal.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    let Command::UseAbility { kingdom, ability, target } = command else {
        return Err(format!("{command:?} is not a special operation"));
    };
    // A computer player's order has no HUD target choice; its film decree stays
    // a computer's in a replay (`aa_parity::replay::plan`).
    let hud = !world.kingdoms.iter().any(|k| k.id == *kingdom && matches!(k.controller, crate::world::Controller::Computer(_)));
    use_ability(world, *kingdom, *ability as usize, *target, hud)
}

/// A computer player's order (the SAI's `61E270` / `61E2B0` emit the request
/// that runs `4CDE00` as a decree in the next admin tick, so a film holds it):
/// `Command::UseAbility` of a computer's kingdom, which has no HUD target choice
/// (`command`).
pub(crate) fn use_from_sai(world: &mut World, kingdom: KingdomId, index: usize, target: AbilityTarget) {
    world.sai_orders.push(Command::UseAbility { kingdom, ability: index as u32, target: Some(target) });
}

/// `4C8F20` for a faction ability (check_range true, phase 0), codes in
/// native order: 1 the target, 4 the cost, 6 already enchanted, 9 / 10 /
/// 8 availability. The target is kept, moved to the clicked actor's
/// organization, or dropped to its point (the HUD's choice).
pub fn admit(world: &World, kingdom: KingdomId, index: usize, target: &mut Option<OpTarget>) -> Result<(), String> {
    admit_with(world, kingdom, index, target, true)
}

fn admit_with(world: &World, kingdom: KingdomId, index: usize, target: &mut Option<OpTarget>, hud: bool) -> Result<(), String> {
    let g = general(world, kingdom).ok_or("special_operation_no_general")?;
    let op = g.abilities.get(index).ok_or("special_operation_unknown")?;
    // A target that is not there (actor +8 bit 0) gives its point to an
    // ability that takes a location; otherwise code 1.
    if let Some(t) = *target
        && !alive(world, t)
    {
        if op.accepts_location() {
            *target = None;
        } else {
            return Err("special_operation_invalid_target".into());
        }
    }
    if let Some(t) = *target
        && !eligible(world, kingdom, op, t)
    {
        if !hud {
            return Err("special_operation_invalid_target".into());
        }
        // (HUD) The clicked member stands for its organization when the
        // ability targets organizations (resupply), and the ground under the
        // cursor for an ability that takes a location.
        let org = match t {
            OpTarget::Actor(a) => world.actors.get(a as usize).and_then(|x| x.org).map(OpTarget::Org),
            OpTarget::Org(_) => None,
        };
        match org.filter(|&o| eligible(world, kingdom, op, o)) {
            Some(o) => *target = Some(o),
            None if op.accepts_location() => *target = None,
            None => return Err("special_operation_invalid_target".into()),
        }
    }
    if target.is_none() && !op.accepts_location() {
        return Err("special_operation_requires_target".into());
    }
    if let Some(t) = *target
        && immune_to_all(world, kingdom, op, t)
    {
        return Err("special_operation_target_immune".into());
    }
    if !can_pay(world, kingdom, op) {
        return Err("special_operation_not_enough_experience".into());
    }
    // (HUD) An area ability aimed at an actor is aimed at its ground point.
    if let Some(t) = *target
        && (!hud || op.area <= 0.0)
        && already_enchanted(world, op, t)
    {
        return Err("special_operation_already_enchanted".into());
    }
    match available(world, kingdom, index) {
        0 => Ok(()),
        8 => Err("special_operation_recharging".into()),
        9 => Err("special_operation_summon_alive".into()),
        10 => Err("special_operation_unique".into()),
        _ => Err("special_operation_unavailable".into()),
    }
}

/// `4C8F20` again at phase 2 (a projectile's impact) or 3 (each area
/// target): no cost, range or availability; at phase 2 the eligibility,
/// at both the hostile immunity (code 1) and the held property (code 6).
fn admit_later(world: &World, kingdom: KingdomId, op: &SpecialOp, target: &mut Option<OpTarget>, phase: u32) -> bool {
    if let Some(t) = *target
        && !alive(world, t)
    {
        if !op.accepts_location() {
            return false;
        }
        *target = None;
    }
    let Some(t) = *target else { return op.accepts_location() };
    if phase == 2 && !eligible(world, kingdom, op, t) {
        return false;
    }
    !immune_to_all(world, kingdom, op, t) && !already_enchanted(world, op, t)
}

fn use_ability(world: &mut World, kingdom: KingdomId, index: usize, target: Option<AbilityTarget>, hud: bool) -> Result<(), String> {
    let data = world.special_ops.data.clone().ok_or("special operations are not loaded")?;
    prepare(world);
    let (point, mut actor) = match target {
        Some(AbilityTarget::Point(p)) => (p, None),
        Some(AbilityTarget::Actor(a)) => {
            let p = world.actors.get(a as usize).map(|x| x.pos.truncate()).ok_or("special_operation_invalid_target")?;
            (p, Some(OpTarget::Actor(a)))
        }
        // a +SELF ability (`0x612912`: sent without a target)
        None if general(world, kingdom).and_then(|g| g.abilities.get(index)).is_some_and(|op| op.required & BIT_SELF != 0) => (Vec2::ZERO, None),
        None => return Err("special_operation_requires_target".into()),
    };
    admit_with(world, kingdom, index, &mut actor, hud)?;
    let gi = world.special_ops.commanders[kingdom as usize].general.ok_or("special_operation_no_general")?;
    let op = &data.generals[gi].abilities[index];
    // 4C9550 +54 bit 10: the target's point inside the map (4282A0) and on
    // the SAI map's primary island, before anything is used or paid; else
    // "ability_effect_invalid_location|s" (message 0x11) to the allegiance.
    let at = actor.map_or(point, |t| position(world, t));
    if op.flags & FLAG_PRIMARY_ISLAND_ONLY != 0 && !on_primary_island(world, at) {
        return Err("ability_effect_invalid_location".into());
    }
    // Phase 0: the target's position; a kingdom source clamps it into the
    // map interior (552640) and takes it as the source position.
    let point = clamp_interior(world, at);
    let now = native_now(world);
    // FX event 1 (4C8500).
    world.events.push(Event::SpecialOperation {
        kingdom,
        ability: op.id.clone(),
        at: point,
        target: match actor {
            Some(OpTarget::Actor(a)) => Some(a),
            _ => None,
        },
    });
    {
        let c = commander_mut(world, kingdom);
        c.last_use[index] = Some(now);
        c.use_count[index] = c.use_count[index].wrapping_add(1);
    }
    // Self effects (+SELF) first, e.g. the visibility actor over a bombed
    // area: the context target is the source.
    for (ei, e) in op.effects.iter().enumerate() {
        if e.required & BIT_SELF != 0 {
            apply_effect(world, kingdom, gi, index, ei, None, point, 1.0);
        }
    }
    // 4E2590 -> 4CDD90: the balance minus the integer cost, stored binary32
    // (negative balances allowed); no resource cost.
    {
        let c = commander_mut(world, kingdom);
        c.experience = (f64::from(c.experience) - f64::from(op.cost)) as f32;
    }
    // (No general's ability has a volley_area: the phase-1 volley is not reached.)
    if op.projectile.is_some() {
        // 580BC0: a projectile with a life > 0 lands after it (580920).
        world.special_ops.pending.push(Pending::Impact { kingdom, general: gi, ability: index, at: now + op.projectile_life, center: point, target: actor });
    } else {
        impact(world, kingdom, gi, index, point, actor, 0);
    }
    Ok(())
}

/// `552640` on the map: [cell, extent - cell - 1e-4] on each axis, the upper
/// limit stored binary32 first; a value below the cell size is replaced.
fn clamp_interior(world: &World, p: Vec2) -> Vec2 {
    let m = &world.map;
    if m.width == 0 || m.height == 0 {
        return p;
    }
    let extent = map_extent(world);
    let cell = m.cell_size;
    let mut xy = [p.x, p.y];
    for (axis, v) in xy.iter_mut().enumerate() {
        let limit = (f64::from(extent[axis]) - f64::from(cell) - f64::from(f32::from_bits(0x38d1_b717))) as f32;
        if cell > *v {
            *v = cell;
        } else if *v > limit {
            *v = limit;
        }
    }
    Vec2::new(xy[0], xy[1])
}

/// Map `+C` / `+10` world extents.
fn map_extent(world: &World) -> [f32; 2] {
    let m = &world.map;
    if m.world_extents[0] > 0.0 && m.world_extents[1] > 0.0 {
        m.world_extents
    } else {
        [m.width as f32 * m.cell_size, m.height as f32 * m.cell_size]
    }
}

/// `4C9550` +54 bit 10: `4282A0` (0 <= x < W, 0 <= y < H), then the tile's
/// region (`42A530` / `7B9300`) on the primary island (`7B8FD0` ==
/// `[B3652C]+2C`).
fn on_primary_island(world: &mut World, p: Vec2) -> bool {
    let key = Arc::as_ptr(&world.map) as usize;
    if world.special_ops.islands.as_ref().is_none_or(|i| i.key != key) {
        world.special_ops.islands = Some(build_islands(world, key));
    }
    let Some(i) = world.special_ops.islands.as_ref() else { return true };
    if !(p.x >= 0.0 && p.x < i.extent[0] && p.y >= 0.0 && p.y < i.extent[1]) {
        return false;
    }
    let Some(primary) = i.primary else { return false };
    let (x, y) = ((p.x as i32) >> i.shift, (p.y as i32) >> i.shift);
    if x < 0 || y < 0 || x as u32 >= i.width || y as u32 >= i.height {
        return false;
    }
    i.tile[(y as u32 * i.width + x as u32) as usize] == primary as i32
}

/// The static SAI map analysis `7ACB00` (aa_navigation::topology_builder)
/// over the map's tile flags, with its SVars, keeping each tile's island
/// and the primary island `7AA2A0`.
fn build_islands(world: &World, key: usize) -> Islands {
    use aa_navigation::topology::{GridLayout, StaticTile};
    use aa_navigation::topology_builder::{StaticGrid, StaticMapParameters, build};
    let m = &world.map;
    let extent = map_extent(world);
    let mut out = Islands { key, width: m.width, height: m.height, extent, ..Default::default() };
    let shift = m.cell_size.log2();
    if m.width == 0 || m.height == 0 || m.flags.len() != (m.width * m.height) as usize || shift.fract() != 0.0 || shift < 0.0 {
        return out;
    }
    out.shift = shift as u32;
    let rule = |name: &str, shipped: f32| world.data.rule_ci(name).filter(|v| v.is_finite()).unwrap_or(shipped);
    // 7AA180: trunc(floor(width * pow((W + H) * 0.5 / base, exp) + 0.5)).
    let mean = (f64::from(m.width) + f64::from(m.height)) * 0.5 / f64::from(rule("StaticMapBaseMapWidth", 64.0));
    let max_width = (mean.powf(f64::from(rule("StaticMapMaxRectangleWidthExp", f32::from_bits(0x3eaa_a64c)))) * f64::from(rule("StaticMapMaxRectangleWidth", 12.0)) + 0.5).floor() as i32;
    let params = StaticMapParameters {
        starting_square_size: rule("StaticMapStartingSquareSize", 10.0) as i32,
        use_fast_square_reduction: rule("StaticMapUseFastSquareReduction", 1.0) != 0.0,
        fast_square_reduction_mult: rule("StaticMapFastSquareReductionMult", 3.0) as i32,
        fast_square_reduction_div: rule("StaticMapFastSquareReductionDiv", 4.0) as i32,
        min_region_size: rule("StaticMapMinRegionSize", 9.0) as i32,
        min_region_width: rule("StaticMapMinRegionWidth", 3.0) as i32,
        max_region_size_for_expansion: rule("StaticMapMaxRegionSizeForExpansion", 1000.0) as i32,
        max_rectangle_width: max_width,
    };
    let tiles: Vec<StaticTile> = m.flags.iter().map(|&f| StaticTile { flags20: f, definition_type: (f & crate::map::TERRAIN_TYPE_MASK) >> crate::map::TERRAIN_TYPE_SHIFT }).collect();
    let layout = GridLayout { width: m.width as i32, height: m.height as i32, coordinate_shift: out.shift, extent_bits: [extent[0].to_bits(), extent[1].to_bits()] };
    let grid = StaticGrid { layout, cell_size_bits: m.cell_size.to_bits(), tiles: &tiles };
    if let Ok(a) = build(&grid, &params, aa_numeric::Precision::Double53) {
        out.tile = a.topology.tile_rectangles.iter().map(|r| r.map_or(-1, |r| a.topology.rectangles[r as usize].island)).collect();
        out.primary = a.post_load.primary_island;
    }
    out
}

/// The 4C9C90 area query: every actor and organization whose position lies
/// in the circle, in id order (actors, then organizations).
fn in_area(world: &World, center: Vec2, radius: f32) -> Vec<OpTarget> {
    let r2 = f64::from(radius) * f64::from(radius);
    let d2 = |p: Vec2| {
        let (dx, dy) = (f64::from(p.x) - f64::from(center.x), f64::from(p.y) - f64::from(center.y));
        dx * dx + dy * dy
    };
    let mut out: Vec<OpTarget> = world.actors.iter().filter(|a| a.alive && !a.removed && d2(a.pos.truncate()) <= r2).map(|a| OpTarget::Actor(a.id)).collect();
    out.extend(world.orgs.iter().filter(|o| o.alive).map(|o| OpTarget::Org(o.id)).filter(|&t| alive(world, t) && d2(position(world, t)) <= r2));
    out
}

/// PC53 squared distance (`dx*dx + dy*dy` in the x87 register).
fn squared(a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (f64::from(a.x) - f64::from(b.x), f64::from(a.y) - f64::from(b.y));
    dx * dx + dy * dy
}

/// `4C8560` linked merge sort of the prepended selection: the left run
/// wins only when strictly nearer (aa_combat::execution::sort_targets).
fn sort_nearest(world: &World, targets: &mut [OpTarget], center: Vec2) {
    if targets.len() < 2 {
        return;
    }
    let mid = targets.len().div_ceil(2);
    let (left, right) = targets.split_at_mut(mid);
    sort_nearest(world, left, center);
    sort_nearest(world, right, center);
    let mut merged = Vec::with_capacity(targets.len());
    let (mut a, mut b) = (0, mid);
    while a < mid && b < targets.len() {
        if squared(center, position(world, targets[a])) < squared(center, position(world, targets[b])) {
            merged.push(targets[a]);
            a += 1;
        } else {
            merged.push(targets[b]);
            b += 1;
        }
    }
    merged.extend_from_slice(&targets[a..mid]);
    merged.extend_from_slice(&targets[b..]);
    targets.copy_from_slice(&merged);
}

/// `4C9550` phases 2..3: a projectile's impact admitted again (phase 2),
/// the area (`4C9C90`) or the single target, then the effects.
fn impact(world: &mut World, kingdom: KingdomId, gi: usize, index: usize, center: Vec2, mut target: Option<OpTarget>, phase: u32) {
    let data = world.special_ops.data.clone().expect("attached");
    let op = &data.generals[gi].abilities[index];
    if phase == 2 && !admit_later(world, kingdom, op, &mut target, 2) {
        return;
    }
    if op.area > 0.0 {
        // 4C9C90: each eligible target (4C8740) at once, or with
        // max_targets the prepended selection sorted nearest first.
        let eligible: Vec<OpTarget> = in_area(world, center, op.area).into_iter().filter(|&t| eligible(world, kingdom, op, t)).collect();
        let order: Vec<OpTarget> = if op.max_targets > 0 {
            let mut selected: Vec<OpTarget> = eligible.into_iter().rev().collect();
            sort_nearest(world, &mut selected, center);
            selected.truncate(op.max_targets as usize);
            selected
        } else {
            eligible
        };
        for t in order {
            // 4C9550 phase 3 on the target.
            let mut on = Some(t);
            if !admit_later(world, kingdom, op, &mut on, 3) {
                continue;
            }
            let Some(t) = on else { continue };
            let scale = attenuation(world, op, center, t);
            effects_on(world, kingdom, gi, index, Some(t), center, scale);
        }
        return;
    }
    let target = target.filter(|&t| alive(world, t));
    if target.is_none() && op.flags & FLAG_TARGET_LOCATION == 0 {
        return;
    }
    effects_on(world, kingdom, gi, index, target, center, 1.0);
}

/// `4C9550` phase 3: with attenuation_range_min < area and
/// attenuation_factor_max < 0.9999, the scale from the target's distance
/// (stored binary32) less its radius, not below zero:
/// `1 - (1 - factor) * (d - min) / (area - min)` in the register, stored,
/// clamped to [factor, 1].
fn attenuation(world: &World, op: &SpecialOp, center: Vec2, t: OpTarget) -> f32 {
    if !(op.attenuation_min < op.area && op.attenuation_factor < f32::from_bits(0x3f7f_f972)) {
        return 1.0;
    }
    // Target virtual +0 (4D1E30): its target / soft shape radius; aa-play
    // keeps no soft shape and uses the DefaultRadius rule.
    let radius = f64::from(world.data.rule_ci("DefaultRadius").unwrap_or(0.5));
    let distance = squared(center, position(world, t)).sqrt() as f32;
    let distance = (f64::from(distance) - radius).max(0.0);
    let (min, area, factor) = (f64::from(op.attenuation_min), f64::from(op.area), f64::from(op.attenuation_factor));
    let width = area - min;
    let mut scale = 1.0f32;
    if distance > min && width > 0.0 {
        scale = (1.0 - (1.0 - factor) * ((distance - min) / width)) as f32;
        if scale < op.attenuation_factor {
            scale = op.attenuation_factor;
        }
        if scale > 1.0 {
            scale = 1.0;
        }
    }
    scale
}

/// The non-self effects whose own target filter accepts the target
/// (`4CA390` not +SELF, `4CA210`: no actor -> required == 0, else 4C5A20).
fn effects_on(world: &mut World, kingdom: KingdomId, gi: usize, index: usize, target: Option<OpTarget>, center: Vec2, scale: f32) {
    let data = world.special_ops.data.clone().expect("attached");
    let op = &data.generals[gi].abilities[index];
    for (ei, e) in op.effects.iter().enumerate() {
        if e.required & BIT_SELF != 0 {
            continue;
        }
        let accepts = match target {
            None => e.required == 0,
            Some(t) => (e.required == 0 && e.forbidden == 0) || matches(e.required, e.forbidden, target_flags(world, kingdom, t)),
        };
        if accepts {
            apply_effect(world, kingdom, gi, index, ei, target, center, scale);
        }
    }
}

// ---------------------------------------------------------------------------
// Effects

fn org_of(world: &World, t: OpTarget) -> Option<OrgId> {
    match t {
        OpTarget::Org(o) => Some(o),
        OpTarget::Actor(a) => world.actors.get(a as usize)?.org,
    }
    .filter(|&o| world.orgs.get(o as usize).is_some_and(|x| x.alive))
}

#[allow(clippy::too_many_arguments)]
fn apply_effect(world: &mut World, kingdom: KingdomId, gi: usize, index: usize, ei: usize, target: Option<OpTarget>, point: Vec2, scale: f32) {
    let data = world.special_ops.data.clone().expect("attached");
    let op = &data.generals[gi].abilities[index];
    let e = &op.effects[ei];
    let actor = match target {
        Some(OpTarget::Actor(a)) => Some(a),
        _ => None,
    };
    match e.kind {
        MAIN_DAMAGE | ADD_DAMAGE => {
            if let Some(t) = actor {
                let mut st = std::mem::take(&mut world.combat);
                st.prepare(world);
                let dt = e.damage_type.unwrap_or(0);
                let amount = if e.kind == MAIN_DAMAGE {
                    combat::kingdom_hit_amount(world, &st, t, op.power, scale, dt, op.flags)
                } else {
                    combat::add_damage_amount(world, &st, t, e.amount, dt)
                };
                combat::damage_from_kingdom(world, &mut st, kingdom, t, amount, Some(dt));
                world.combat = st;
            }
        }
        // 4CA050: the target's organization (4CE6C0 +7C), scale * amount
        // stored binary32; MORALE_GAIN 5786D0, MORALE_LOSS 579660.
        MORALE_GAIN | MORALE_LOSS => {
            if let Some(o) = target.and_then(|t| org_of(world, t)) {
                let mut st = std::mem::take(&mut world.combat);
                st.prepare(world);
                let amount = scale * e.amount;
                if e.kind == MORALE_GAIN {
                    crate::morale::gain(world, &mut st, o, amount);
                } else {
                    crate::morale::lose(world, &mut st, o, amount);
                }
                world.combat = st;
            }
        }
        // 4CA0B0: a target with a Body; amount (501B70 with the kingdom's
        // heal bonus, none kept) times scale, then 562BA0.
        HEAL => {
            if let Some(t) = actor {
                let max = world.combat.actor(t).map(|c| c.max_health).filter(|&m| m > 0.0).unwrap_or(world.data.def(world.actors[t as usize].def).health);
                body_heal(world, t, e.amount * scale, max);
            }
        }
        ENCHANTMENT => {
            if let Some(t) = target {
                enchant(world, kingdom, gi, index, ei, t);
            } else if op.required & BIT_SELF != 0
                && !world.special_ops.kingdom_enchantments.iter().any(|k| k.kingdom == kingdom && k.general == gi && k.ability == index && k.effect == ei)
            {
                let created = native_now(world);
                world.special_ops.kingdom_enchantments.push(KingdomEnchanted { kingdom, general: gi, ability: index, effect: ei, created });
            }
        }
        SUMMON => {
            if let Some(s) = &e.summon {
                summon(world, kingdom, index, s, point, target);
            }
        }
        // 4CA680: the target's kingdom (virtual +108); none logs only.
        STEAL_RESOURCES => {
            if let Some(victim) = target.and_then(|t| owner(world, t)) {
                steal(world, kingdom, victim, &e.steal);
            }
        }
        DESCRIBE_ECONOMY => {
            if let Some(of) = target.and_then(|t| owner(world, t)) {
                world.events.push(Event::EconomyDescribed { kingdom, of });
            }
        }
        // CAPTURE (4CA260) is in no general's ability.
        _ => {}
    }
}

/// `562BA0` plain heal: below the maximum and a positive amount, health +
/// amount (stored binary32), at most the maximum.
fn body_heal(world: &mut World, t: ActorId, amount: f32, max: f32) {
    let a = &mut world.actors[t as usize];
    if a.alive && a.health < max && amount > 0.0 {
        a.health = (a.health + amount).min(max);
    }
}

/// `4CA680` (aa_economy::transfers::steal_resources): every positive
/// amount moves `min(stock, amount)` (the stock only when strictly less),
/// victim `4E16D0` then thief `4E2080`, row by row.
fn steal(world: &mut World, thief: KingdomId, victim: KingdomId, amounts: &[f32]) {
    let n = world.data.resources.len();
    for (i, &want) in amounts.iter().enumerate().take(n) {
        if want.is_nan() || want <= 0.0 {
            continue;
        }
        let stock = world.books.kingdom_mut(victim, n).stockpiled[i];
        let v = if stock < want { stock } else { want };
        world.books.kingdom_mut(victim, n).stockpiled[i] -= v;
        world.books.kingdom_mut(thief, n).stockpiled[i] += v;
    }
}

/// `4CA130` -> `581700`: a target holding the property already (from any
/// source) is skipped, so is a target without the required properties and
/// an enemy immune to the enchantment type; then `4FE6F0` purges the
/// hostile enchantments the new immunities cover.
fn enchant(world: &mut World, kingdom: KingdomId, gi: usize, index: usize, ei: usize, target: OpTarget) {
    let data = world.special_ops.data.clone().expect("attached");
    let Some(en) = data.generals[gi].abilities[index].effects[ei].enchantment.as_ref() else { return };
    if has_property(world, target, &en.id) {
        return;
    }
    let def = world.data.def(def_of(world, target));
    if !en.required_properties.iter().all(|r| def.properties.iter().any(|p| p.eq_ignore_ascii_case(r))) {
        return;
    }
    let target_owner = owner(world, target);
    if !allied(world, target_owner, Some(kingdom)) && immune(world, target, &en.kind) {
        return;
    }
    let created = native_now(world);
    world.special_ops.enchantments.push(Enchanted { target, kingdom, general: gi, ability: index, effect: ei, created });
    let immunities: Vec<Option<String>> = en.modifiers.iter().filter_map(|m| if let OpModifier::Immunity { kind } = m { Some(kind.clone()) } else { None }).collect();
    if !immunities.is_empty() {
        let purge: Vec<bool> = world
            .special_ops
            .enchantments
            .iter()
            .map(|e| {
                e.target == target && !allied(world, Some(e.kingdom), target_owner) && {
                    let kind = data.generals[e.general].abilities[e.ability].effects[e.effect].enchantment.as_ref().map(|x| x.kind.as_str()).unwrap_or("");
                    immunities.iter().any(|k| k.as_deref().is_none_or(|k| k.eq_ignore_ascii_case(kind)))
                }
            })
            .collect();
        let mut i = 0;
        world.special_ops.enchantments.retain(|_| {
            let keep = !purge[i];
            i += 1;
            keep
        });
    }
    refresh_mods(world, target);
}

/// `4CCA70` with target_location (every general's summon): the target
/// point, direction (0, -1); unless ethereal, `554B30` finds an open spot
/// within 12 m or the summon fails ("ability_effect_summon_failed",
/// `4CB5B0`); a transported object (ActorData +184) flies in with its
/// transport (`4CB110`), the others are created at once (`4CC470` /
/// `4CC2C0`).
#[allow(clippy::too_many_arguments)]
fn summon(world: &mut World, kingdom: KingdomId, index: usize, s: &Summon, point: Vec2, target: Option<OpTarget>) {
    let Some(def) = world.data.find(&s.object) else { return };
    let mut at = point;
    if !s.ethereal && !open_position(world, def, &mut at) {
        world.events.push(Event::Refused { kingdom, reason: "ability_effect_summon_failed".into() });
        return;
    }
    // 4CB110: the transport enters from the farther map edge
    // (aa_combat::summon::transport_entry) and flies to the point (`crate::transport`).
    if let Some(t) = s.transport.as_ref().and_then(|t| world.data.find(t))
        && crate::transport::summon(world, kingdom, t, def, at, s.controllable, s.property.clone())
    {
        return;
    }
    create_summon(world, kingdom, index, s, at, target);
}

/// The occupancy test of `554B30`'s query (`553BA0`, mask 0x500, blocking
/// actors): inside the map, the shape's tiles passable for the object and
/// clear of static blockers, no building footprint, no blocking actor.
fn spot_open(world: &World, def: DefId, p: Vec2) -> bool {
    let d = world.data.def(def);
    let r = d.blocking_size.map_or(0.0, |s| if d.blocking_shape.as_deref().is_some_and(|x| x.eq_ignore_ascii_case("CIRCLE")) { s[0] } else { 0.5 * s[0].max(s[1]) }).max(0.0);
    let m = &world.map;
    if m.width > 0 {
        let [w, h] = map_extent(world);
        if p.x - r < 0.0 || p.y - r < 0.0 || p.x + r > w || p.y + r > h {
            return false;
        }
        let c = m.cell_size;
        let cell = |x: f32, y: f32| ((y / c) as u32).min(m.height - 1) * m.width + ((x / c) as u32).min(m.width - 1);
        let mask = crate::map::BLOCKED | 0x400 | if d.water_based { crate::map::NOT_WATER_PASSABLE } else { crate::map::NOT_LAND_PASSABLE };
        for (x, y) in [(p.x, p.y), (p.x - r, p.y - r), (p.x + r, p.y - r), (p.x - r, p.y + r), (p.x + r, p.y + r)] {
            if m.flags.get(cell(x, y) as usize).is_some_and(|f| f & mask != 0) {
                return false;
            }
        }
    }
    for a in world.actors.iter().filter(|a| a.alive && !a.removed) {
        let od = world.data.def(a.def);
        if !od.blocking {
            continue;
        }
        let q = a.pos.truncate();
        let Some(s) = od.blocking_size else { continue };
        if od.blocking_shape.as_deref().is_some_and(|x| x.eq_ignore_ascii_case("CIRCLE")) {
            let rr = r + s[0];
            if (p - q).length_squared() < rr * rr {
                return false;
            }
        } else {
            let half = Vec2::new(s[0], s[1]) * 0.5;
            let gap = ((p - q).abs() - half).max(Vec2::ZERO);
            if gap.length_squared() < r * r || (p - q).abs().cmplt(half).all() {
                return false;
            }
        }
    }
    true
}

/// `554B30` with the 4CCA70 distance of 12 m.
fn open_position(world: &World, def: DefId, point: &mut Vec2) -> bool {
    open_position_within(world, def, point, SUMMON_SEARCH_DISTANCE)
}

/// `554B30` (aa_spatial::placement::find_placement_within / 554850): the
/// point itself, else rings k = 1..=rings at k * step along the direction
/// (0, -1), turned eight times by the native -pi/4 table values; the ring
/// count is trunc(distance / radius) clamped to 1..=20 (the radius the
/// blocking radius, at least a quarter cell), the step distance / rings.
fn open_position_within(world: &World, def: DefId, point: &mut Vec2, distance: f32) -> bool {
    if spot_open(world, def, *point) {
        return true;
    }
    let d = world.data.def(def);
    let shape_radius = d.blocking_size.map_or(0.0, |s| s[0]);
    let floor = world.map.cell_size * 0.25;
    let radius = if shape_radius < floor { floor } else { shape_radius };
    if radius.is_nan() || radius <= 0.0 {
        return false;
    }
    let rings = ((f64::from(distance) / f64::from(radius)) as i32).clamp(1, 20);
    let step = (f64::from(distance) / f64::from(rings)) as f32;
    let (sin, cos) = (f64::from(f32::from_bits(ROTATION_SIN_BITS)), f64::from(f32::from_bits(ROTATION_COS_BITS)));
    let direction = [0.0f32, -1.0];
    for ring in 1..=rings {
        let distance = f64::from(ring) * f64::from(step);
        let mut offset = [(distance * f64::from(direction[0])) as f32, (distance * f64::from(direction[1])) as f32];
        for _ in 0..8 {
            let candidate = Vec2::new((f64::from(offset[0]) + f64::from(point.x)) as f32, (f64::from(offset[1]) + f64::from(point.y)) as f32);
            if spot_open(world, def, candidate) {
                *point = candidate;
                return true;
            }
            let (ox, oy) = (f64::from(offset[0]), f64::from(offset[1]));
            offset = [(ox * cos - oy * sin) as f32, (oy * cos + sin * ox) as f32];
        }
    }
    false
}

/// `4CC470` (an actor; `4FA150` for an organization) / `4CC2C0` (a plain
/// object): created at the point facing (0, -1); not controllable (`+8 |=
/// 4`); the GTID head-inserted into the ability's list (`4C89A0`, not for a
/// transported object, which `4CB110` never lists); the effect's property
/// (`4CEF10` kind 3); FX event 4; then, with a target, the order to attack
/// it when it is an enemy of the summoned (virtual `+16C` = `4D1F40`), else
/// whoever last damaged its body (`432E40`).
fn create_summon(world: &mut World, kingdom: KingdomId, index: usize, s: &Summon, point: Vec2, target: Option<OpTarget>) {
    let Some(def) = world.data.find(&s.object) else { return };
    // target_location: facing (0, -1)
    let facing = -std::f32::consts::FRAC_PI_2;
    let (created, org): (Vec<ActorId>, Option<OrgId>) = if matches!(world.data.def(def).class, Some(DefClass::Organization)) {
        let o = crate::production::spawn_organization(world, def, kingdom, point, facing, None);
        world.events.push(Event::Recruited { kingdom, org: Some(o), actor: None });
        (world.orgs[o as usize].members.clone(), Some(o))
    } else {
        (vec![crate::production::spawn_single(world, def, Some(kingdom), point, facing)], None)
    };
    if !s.controllable {
        world.special_ops.uncontrollable.extend(created.iter().copied());
    }
    let id = match org {
        Some(o) => Summoned::Org(o),
        None => Summoned::Actor(created[0]),
    };
    if let Some(list) = commander_mut(world, kingdom).summoned.get_mut(index) {
        list.insert(0, id);
    }
    if let Some(p) = &s.property {
        for &a in &created {
            let b = world.books.actor_mut(a);
            if !b.properties.iter().any(|q| q.eq_ignore_ascii_case(p)) {
                b.properties.push(p.clone());
            }
        }
    }
    let Some(t) = target.filter(|&t| alive(world, t)) else { return };
    let victim = if enemy(world, owner(world, t), Some(kingdom)) {
        match t {
            OpTarget::Actor(a) => Some(a),
            OpTarget::Org(x) => world.orgs[x as usize].members.iter().copied().find(|&m| world.actors[m as usize].alive),
        }
    } else {
        match t {
            OpTarget::Actor(a) => world.combat.actor(a).and_then(|c| c.last_hit).map(|(s, _)| s).filter(|&s| world.actors.get(s as usize).is_some_and(|x| x.alive && !x.removed)),
            OpTarget::Org(_) => None,
        }
    };
    // (The kill command 4CA330 -> 4D9B60 reaches a summoned regiment's
    // order; a single summoned actor has no order aa-play carries out.)
    if let (Some(o), Some(v)) = (org, victim) {
        world.orgs[o as usize].order = crate::world::Order::Attack { target: v };
    }
}

// ---------------------------------------------------------------------------
// Modifiers

fn refresh_mods(world: &mut World, target: OpTarget) {
    let data = world.special_ops.data.clone();
    let mut m = Mods { resist: vec![0.0; world.data.damage_types.len()], ..Default::default() };
    if let Some(data) = data {
        for e in world.special_ops.enchantments.iter().filter(|e| e.target == target) {
            let Some(en) = data.generals[e.general].abilities[e.ability].effects[e.effect].enchantment.as_ref() else { continue };
            for md in &en.modifiers {
                match md {
                    OpModifier::Add { attribute, amount } => match *attribute {
                        AV => m.av += amount,
                        AV_MELEE => m.av_melee += amount,
                        AV_RANGED => m.av_ranged += amount,
                        DV => m.dv += amount,
                        _ => {}
                    },
                    OpModifier::Multiply { attribute, amount } => match *attribute {
                        SPEED_MAX => m.speed += amount,
                        HEALTH => m.health += amount,
                        RECHARGE_TIME => m.recharge += amount,
                        RESUPPLY_RATE => m.resupply += amount,
                        _ => {}
                    },
                    OpModifier::Resist { damage_type, amount } => {
                        if let Some(r) = m.resist.get_mut(*damage_type as usize) {
                            *r += amount;
                        }
                    }
                    OpModifier::PeriodicDamage { damage_type, amount } => m.periodic_damage.push((*damage_type, *amount)),
                    OpModifier::PeriodicHeal { amount } => {
                        m.periodic_heal += amount;
                        m.periodic_heals.push(*amount);
                    }
                    OpModifier::Immunity { kind } => m.immune.push(kind.clone()),
                    OpModifier::AbilityEffect { ability_type, effect } => m.ability_effects.push((ability_type.clone(), (**effect).clone())),
                    OpModifier::Flag { flag } => m.flags.push(flag.clone()),
                    OpModifier::Other => {}
                }
            }
        }
    }
    let list = match target {
        OpTarget::Actor(_) => &mut world.special_ops.mods,
        OpTarget::Org(_) => &mut world.special_ops.org_mods,
    };
    let i = match target {
        OpTarget::Actor(a) => a as usize,
        OpTarget::Org(o) => o as usize,
    };
    if list.len() <= i {
        list.resize_with(i + 1, Mods::default);
    }
    list[i] = m;
    if let OpTarget::Actor(a) = target {
        let mut st = std::mem::take(&mut world.combat);
        st.prepare(world);
        combat::refresh_max_health(world, &mut st, a);
        world.combat = st;
    }
}

/// The extra damage of an attack (ABILITY_EFFECT on the shooter, e.g.
/// communist zeal's +3 armor_piercing on every `attack` ability): (kind,
/// amount, damage type).
pub(crate) fn extra_hits(world: &World, shooter: ActorId, ability_type: Option<&str>) -> Vec<(u32, f32, u32)> {
    let (Some(t), Some(m)) = (ability_type, mods(world, shooter)) else { return Vec::new() };
    m.ability_effects
        .iter()
        .filter(|(at, e)| at.eq_ignore_ascii_case(t) && (e.kind == ADD_DAMAGE || e.kind == MAIN_DAMAGE))
        .map(|(_, e)| (e.kind, e.amount, e.damage_type.unwrap_or(0)))
        .collect()
}

// ---------------------------------------------------------------------------
// Tick

fn prepare(world: &mut World) {
    let key = Arc::as_ptr(&world.data) as usize;
    if world.special_ops.data_key != key || world.special_ops.def_periodic.len() != world.data.defs.len() {
        world.special_ops.data_key = key;
        let data = world.data.clone();
        world.special_ops.routing_bit = data.target_types.iter().position(|t| t.eq_ignore_ascii_case("routing")).map(|i| i as u32).filter(|&i| i < 32);
        world.special_ops.def_periodic = data
            .defs
            .iter()
            .map(|d| {
                let mut damage = Vec::new();
                let mut heals = Vec::new();
                for p in &d.properties {
                    let Some(prop) = data.properties.get(&p.to_ascii_lowercase()) else { continue };
                    for m in &prop.modifiers {
                        if m.kind.eq_ignore_ascii_case("PERIODIC_DAMAGE") {
                            damage.push((m.damage_type.unwrap_or(0), m.amount));
                        } else if m.kind.eq_ignore_ascii_case("PERIODIC_HEAL") {
                            heals.push(m.amount);
                        }
                    }
                }
                (damage, heals)
            })
            .collect();
    }
}

pub fn tick(world: &mut World) {
    prepare(world);
    experience_income(world);
    prune_summoned(world);
    let now = native_now(world);
    let mut due = Vec::new();
    world.special_ops.pending.retain(|p| {
        // 580920: the projectile lands on the update whose remaining time
        // goes strictly negative (one tick after it reaches zero); the
        // transport drops on its arrival.
        let ready = match p {
            Pending::Impact { at, .. } => *at < now,
        };
        if ready {
            due.push(p.clone());
        }
        !ready
    });
    for p in due {
        match p {
            Pending::Impact { kingdom, general, ability, center, target, .. } => impact(world, kingdom, general, ability, center, target, 2),
        }
    }
    properties_pass(world);
    sai::think(world);
}

/// `4C89F0`: summoned GTIDs that no longer resolve leave the lists.
fn prune_summoned(world: &mut World) {
    let mut lists = std::mem::take(&mut world.special_ops.commanders);
    for c in &mut lists {
        for list in &mut c.summoned {
            list.retain(|&s| resolves(world, s));
        }
    }
    world.special_ops.commanders = lists;
}

/// `4E0FD0` -> `4CDFD0` with the multiply flag (aa_economy::tick::
/// add_commander_experience): a kingdom's organization gained `amount`
/// (`571A60` -> `4CEB70`: the organization actor has a kingdom and no
/// regiment membership): `amount * ExperienceFactionGainMultiplier` stored,
/// added and stored, below 0 -> 0, above a non-negative maximum -> it.
/// Combat calls it from the experience award (`4CE770` -> `571A60`); once
/// it does, special_ops stops reading the organizations' gains itself.
#[allow(dead_code)]
pub(crate) fn credit_experience(world: &mut World, kingdom: KingdomId, amount: f32) {
    world.special_ops.direct_experience = true;
    credit(world, kingdom, amount);
}

fn credit(world: &mut World, kingdom: KingdomId, amount: f32) {
    let multiplier = world.data.rule_ci("ExperienceFactionGainMultiplier").unwrap_or(1.0);
    let Some(c) = world.special_ops.commanders.get_mut(kingdom as usize).filter(|c| c.general.is_some()) else { return };
    let gain = amount * multiplier;
    c.experience = clamp_experience(gain + c.experience, c.maximum);
}

/// The organizations' experience gains since the last tick, each credited
/// to its kingdom in organization order (until combat credits them at the
/// award, [`credit_experience`]). (core) The per-second income `4CDFD0`
/// runs in the kingdom economy tick on this same `Commander::experience`.
fn experience_income(world: &mut World) {
    let direct = world.special_ops.direct_experience;
    if world.special_ops.org_experience.len() < world.orgs.len() {
        let from = world.special_ops.org_experience.len();
        let added: Vec<f32> = world.orgs[from..].iter().map(|o| o.experience).collect();
        world.special_ops.org_experience.extend(added);
    }
    for i in 0..world.orgs.len() {
        let (kingdom, now) = (world.orgs[i].kingdom, world.orgs[i].experience);
        let d = now - world.special_ops.org_experience[i];
        world.special_ops.org_experience[i] = now;
        if !direct && d > 0.0 {
            credit(world, kingdom, d);
        }
    }
}

// ---------------------------------------------------------------------------
// The Properties component Tick (5818D0)

/// The scheduler's queue-1 leaf of a Properties component (Owner::Component
/// / OrgComponent / KingdomComponent with slot 6): one `5818D0` with the
/// per-call amounts. Once the scheduler delivers it, the pass in [`tick`]
/// stops reading the delivered bucket itself.
#[allow(dead_code)]
pub(crate) fn properties_tick(world: &mut World, owner: PropertiesOwner) {
    world.special_ops.properties_leaf = true;
    prepare(world);
    run_properties(world, owner, 1.0);
}

fn scheduler_owner(owner: PropertiesOwner) -> crate::scheduler::Owner {
    match owner {
        PropertiesOwner::Actor(a) => crate::scheduler::Owner::Component(a, PROPERTIES_SLOT),
        PropertiesOwner::Org(o) => crate::scheduler::Owner::OrgComponent(o, PROPERTIES_SLOT),
        PropertiesOwner::Kingdom(k) => crate::scheduler::Owner::KingdomComponent(k, PROPERTIES_SLOT),
    }
}

/// An actor with something for its Properties Tick: enchantments, its own
/// periodic properties.
fn actor_needs(world: &World, a: ActorId) -> bool {
    let x = &world.actors[a as usize];
    if !x.alive || x.removed {
        return false;
    }
    let own = world.special_ops.def_periodic.get(x.def as usize).is_some_and(|(d, h)| !d.is_empty() || !h.is_empty());
    own || world.special_ops.enchantments.iter().any(|e| e.target == OpTarget::Actor(a))
}

/// The Properties Ticks of this tick: the bucket queue 1 delivered (head
/// first; the scheduler does not deliver the leaf yet, so the other queue-1
/// leaves of the bucket are not interleaved), then every owner without a
/// queue-1 Properties registration once per tick with the amounts times DT
/// (test worlds, things created outside `crate::creation`).
fn properties_pass(world: &mut World) {
    // Enchantments of destroyed things go with their Properties component.
    let gone: Vec<bool> = world
        .special_ops
        .enchantments
        .iter()
        .map(|e| match e.target {
            OpTarget::Actor(a) => world.actors.get(a as usize).is_none_or(|x| x.removed),
            OpTarget::Org(o) => world.orgs.get(o as usize).is_none_or(|x| !x.alive),
        })
        .collect();
    if gone.iter().any(|&g| g) {
        let mut i = 0;
        let mut changed = Vec::new();
        world.special_ops.enchantments.retain(|e| {
            let keep = !gone[i];
            i += 1;
            if !keep {
                changed.push(e.target);
            }
            keep
        });
        changed.dedup();
        for t in changed {
            refresh_mods(world, t);
        }
    }
    let any = !world.special_ops.enchantments.is_empty()
        || !world.special_ops.kingdom_enchantments.is_empty()
        || world.actors.iter().any(|a| a.alive && !a.removed && world.special_ops.def_periodic.get(a.def as usize).is_some_and(|(d, h)| !d.is_empty() || !h.is_empty()));
    if !any {
        return;
    }
    if !world.special_ops.properties_leaf {
        for owner in delivered_bucket(world) {
            let needs = match owner {
                PropertiesOwner::Actor(a) => world.actors.get(a as usize).is_some() && actor_needs(world, a),
                PropertiesOwner::Org(o) => world.special_ops.enchantments.iter().any(|e| e.target == OpTarget::Org(o)),
                PropertiesOwner::Kingdom(k) => world.special_ops.kingdom_enchantments.iter().any(|e| e.kingdom == k),
            };
            if needs {
                run_properties(world, owner, 1.0);
            }
        }
    }
    // Owners without a queue-1 registration: every tick.
    let mut unregistered: Vec<PropertiesOwner> = Vec::new();
    for a in 0..world.actors.len() as ActorId {
        if actor_needs(world, a) && !world.is_scheduled(scheduler_owner(PropertiesOwner::Actor(a))) {
            unregistered.push(PropertiesOwner::Actor(a));
        }
    }
    let mut orgs: Vec<OrgId> = world.special_ops.enchantments.iter().filter_map(|e| if let OpTarget::Org(o) = e.target { Some(o) } else { None }).collect();
    orgs.sort_unstable();
    orgs.dedup();
    for o in orgs {
        if !world.is_scheduled(scheduler_owner(PropertiesOwner::Org(o))) {
            unregistered.push(PropertiesOwner::Org(o));
        }
    }
    let mut kingdoms: Vec<KingdomId> = world.special_ops.kingdom_enchantments.iter().map(|e| e.kingdom).collect();
    kingdoms.sort_unstable();
    kingdoms.dedup();
    for k in kingdoms {
        if !world.is_scheduled(scheduler_owner(PropertiesOwner::Kingdom(k))) {
            unregistered.push(PropertiesOwner::Kingdom(k));
        }
    }
    for owner in unregistered {
        run_properties(world, owner, DT);
    }
}

/// The Properties owners of the queue-1 bucket this tick's update delivered
/// (the bucket before the queue's current one), head first.
fn delivered_bucket(world: &World) -> Vec<PropertiesOwner> {
    let Some(q) = world.sched.queues.snapshot().into_iter().find(|q| q.id == crate::scheduler::EVERY_SECOND) else { return Vec::new() };
    let n = q.buckets.len();
    if n == 0 {
        return Vec::new();
    }
    let bucket = (q.current + n - 1) % n;
    q.buckets[bucket]
        .iter()
        .filter_map(|&(key, _)| match crate::scheduler::Owner::from_key(key) {
            Some(crate::scheduler::Owner::Component(a, PROPERTIES_SLOT)) => Some(PropertiesOwner::Actor(a)),
            Some(crate::scheduler::Owner::OrgComponent(o, PROPERTIES_SLOT)) => Some(PropertiesOwner::Org(o)),
            Some(crate::scheduler::Owner::KingdomComponent(k, PROPERTIES_SLOT)) => Some(PropertiesOwner::Kingdom(k)),
            _ => None,
        })
        .collect()
}

/// `5031B0` (aa_combat::special_ops::enchantment_expired): `now - created`
/// in the register is not below the duration.
fn expired(now: f32, created: f32, duration: f32) -> bool {
    f64::from(now) - f64::from(created) >= f64::from(duration)
}

/// One `5818D0`: the instances in vector order, an expired one removed
/// (`5034F0`, the modifiers recomputed) and the entry shifted into its
/// place passed over until the next Tick; then, for a Body, the periodic
/// heals (`562BA0`, not while dying) and the periodic damage (`5625C0`,
/// event 2, reason 10, the damage type modifier), `scale` times each amount.
fn run_properties(world: &mut World, owner: PropertiesOwner, scale: f32) {
    let now = native_now(world);
    if let Some(data) = world.special_ops.data.clone() {
        expire_instances(world, &data, owner, now);
    }
    // Only an actor has a Body (+60).
    let PropertiesOwner::Actor(a) = owner else { return };
    periodic_effects(world, a, scale);
}

/// `5818D0`'s instance loop for one owner (see [`run_properties`]).
fn expire_instances(world: &mut World, data: &GeneralData, owner: PropertiesOwner, now: f32) {
    match owner {
        PropertiesOwner::Kingdom(k) => {
            let list: Vec<usize> = (0..world.special_ops.kingdom_enchantments.len()).filter(|&i| world.special_ops.kingdom_enchantments[i].kingdom == k).collect();
            let mut remove = Vec::new();
            let mut i = 0;
            let mut cur = list;
            while i < cur.len() {
                let e = &world.special_ops.kingdom_enchantments[cur[i]];
                let duration = data.generals[e.general].abilities[e.ability].effects[e.effect].enchantment.as_ref().map_or(0.0, |en| en.duration);
                if expired(now, e.created, duration) {
                    remove.push(cur.remove(i));
                }
                i += 1;
            }
            if !remove.is_empty() {
                let mut idx = 0;
                world.special_ops.kingdom_enchantments.retain(|_| {
                    let keep = !remove.contains(&idx);
                    idx += 1;
                    keep
                });
            }
            return;
        }
        PropertiesOwner::Actor(_) | PropertiesOwner::Org(_) => {}
    }
    let target = match owner {
        PropertiesOwner::Actor(a) => OpTarget::Actor(a),
        PropertiesOwner::Org(o) => OpTarget::Org(o),
        PropertiesOwner::Kingdom(_) => unreachable!(),
    };
    let list: Vec<usize> = (0..world.special_ops.enchantments.len()).filter(|&i| world.special_ops.enchantments[i].target == target).collect();
    let mut remove = Vec::new();
    let mut i = 0;
    let mut cur = list;
    while i < cur.len() {
        let e = &world.special_ops.enchantments[cur[i]];
        let duration = data.generals[e.general].abilities[e.ability].effects[e.effect].enchantment.as_ref().map_or(0.0, |en| en.duration);
        if expired(now, e.created, duration) {
            remove.push(cur.remove(i));
        }
        i += 1;
    }
    if !remove.is_empty() {
        let mut idx = 0;
        world.special_ops.enchantments.retain(|_| {
            let keep = !remove.contains(&idx);
            idx += 1;
            keep
        });
        refresh_mods(world, target);
    }
}

/// `5818D0`'s periodic heals (`562BA0`, not while dying) and periodic damage
/// (`5625C0`, event 2, reason 10, the damage type modifier) of an actor's
/// own properties and enchantments, `scale` times each amount.
fn periodic_effects(world: &mut World, a: ActorId, scale: f32) {
    let id = a as usize;
    let Some(x) = world.actors.get(id) else { return };
    if !x.alive || x.removed {
        return;
    }
    let def = x.def;
    let (own_damage, own_heals) = world.special_ops.def_periodic.get(def as usize).cloned().unwrap_or_default();
    let (ench_damage, ench_heals) = world.special_ops.mods.get(id).map(|m| (m.periodic_damage.clone(), m.periodic_heals.clone())).unwrap_or_default();
    if own_damage.is_empty() && own_heals.is_empty() && ench_damage.is_empty() && ench_heals.is_empty() {
        return;
    }
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    let max = st.actor(a).map(|c| c.max_health).filter(|&m| m > 0.0).unwrap_or(world.data.def(def).health);
    // 5818D0: the heals wait while the thing is under siege (+100 bit 20).
    if !crate::supply::under_siege(world, a) {
        for h in own_heals.iter().chain(&ench_heals) {
            body_heal(world, a, h * scale, max);
        }
    }
    for (dt, amount) in own_damage.iter().chain(&ench_damage) {
        let amount = combat::add_damage_amount(world, &st, a, amount * scale, *dt);
        combat::periodic_damage(world, &mut st, a, amount, *dt);
    }
    world.combat = st;
}

/// The data a world carries (tests, tools).
pub fn data(world: &World) -> Option<&GeneralData> {
    world.special_ops.data.as_deref()
}

/// The public generals of a nation (for the setup screen).
pub fn nation_generals<'a>(data: &'a GeneralData, game: &GameData, nation: &str) -> Vec<&'a General> {
    game.nation(nation).map(|n| n.factions.iter().filter_map(|f| data.general(f)).map(|i| &data.generals[i]).filter(|g| g.public).collect()).unwrap_or_default()
}

// ---- appended by the scenarios owner (crate::triggers) ----

/// `554B30` along a given direction (554850, the search of
/// [`open_position_within`] with the caller's direction): the free spot, or
/// `None`.
pub(crate) fn open_position_along(world: &World, def: DefId, point: Vec2, direction: Vec2, distance: f32) -> Option<Vec2> {
    if spot_open(world, def, point) {
        return Some(point);
    }
    let d = world.data.def(def);
    let shape_radius = d.blocking_size.map_or(0.0, |s| s[0]);
    let floor = world.map.cell_size * 0.25;
    let radius = if shape_radius < floor { floor } else { shape_radius };
    if radius.is_nan() || radius <= 0.0 {
        return None;
    }
    let rings = ((f64::from(distance) / f64::from(radius)) as i32).clamp(1, 20);
    let step = (f64::from(distance) / f64::from(rings)) as f32;
    let (sin, cos) = (f64::from(f32::from_bits(ROTATION_SIN_BITS)), f64::from(f32::from_bits(ROTATION_COS_BITS)));
    for ring in 1..=rings {
        let distance = f64::from(ring) * f64::from(step);
        let mut offset = [(distance * f64::from(direction.x)) as f32, (distance * f64::from(direction.y)) as f32];
        for _ in 0..8 {
            let candidate = Vec2::new((f64::from(offset[0]) + f64::from(point.x)) as f32, (f64::from(offset[1]) + f64::from(point.y)) as f32);
            if spot_open(world, def, candidate) {
                return Some(candidate);
            }
            let (ox, oy) = (f64::from(offset[0]), f64::from(offset[1]));
            offset = [(ox * cos - oy * sin) as f32, (oy * cos + sin * ox) as f32];
        }
    }
    None
}

/// KKC_Ability::TriggerUse `4C9F10` for an ability a general's data holds:
/// `4C8F20` phase 1 is not a player's use (no cost, range or recharge asked of
/// the kingdom); the area effect lands at the point at once, or after the
/// projectile's life. False when no general has the ability.
pub(crate) fn trigger_use(world: &mut World, kingdom: KingdomId, ability: &str, at: Vec2) -> bool {
    let Some(data) = world.special_ops.data.clone() else { return false };
    let Some((gi, index)) = data.generals.iter().enumerate().find_map(|(g, x)| x.abilities.iter().position(|o| o.id.eq_ignore_ascii_case(ability)).map(|i| (g, i))) else {
        return false;
    };
    prepare(world);
    let op = &data.generals[gi].abilities[index];
    // FX event 1 (4C8500), as a player's use
    world.events.push(Event::SpecialOperation { kingdom, ability: op.id.clone(), at, target: None });
    if op.projectile.is_some() {
        let now = native_now(world);
        world.special_ops.pending.push(Pending::Impact { kingdom, general: gi, ability: index, at: now + op.projectile_life, center: at, target: None });
    } else {
        impact(world, kingdom, gi, index, at, None, 0);
    }
    true
}
