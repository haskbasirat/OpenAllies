//! Special operations: every kingdom's general (an original `Faction`,
//! Data/Generals) and its four abilities, bought with the kingdom's
//! experience. Owner: combat.
//!
//! Behaviour-faithful plain `f32` rendition of the exact ports:
//! - experience (Commander `+0xC`): StartingExperience at the start
//!   (kingdom_start `4CDF10` clamp), ExperienceFactionGainPeriodic per second
//!   (`4CDFD0` from the economy tick) and ExperienceFactionGainMultiplier of
//!   the experience its regiments gain (`4E0FD0`), clamped to [0,
//!   experience_max] (aa_economy::tick / kingdom_start);
//! - use: `KKC_FactionUseAbilityOrder` `61E190` -> `KKC_Faction::UseAbility`
//!   `4CDE00` (aa_combat::special_ops::use_ability): admission `4C8F20`
//!   (target filter `4C5840/4C5A20`, cost `4E24B0`, recharge and summon
//!   limits `4C8A40`, the faction range is FLT_MAX: FactionData `48BD05`),
//!   then `4C9550` (aa_combat::execution::use_ability): self effects and the
//!   payment (`4E2590`/`4CDD90`) at once, a projectile's effects after its
//!   projectile_life, the area with `max_targets` nearest and the
//!   attenuation, the per-effect target filters;
//! - effects (aa_combat::special_ops / summon / morale): MAIN_DAMAGE and
//!   ADD_DAMAGE through combat (`4CA400`), MORALE_GAIN / MORALE_LOSS
//!   (`4CA050`), HEAL (`4CA0B0`), ENCHANTMENT (`4CA130`/`581700`: not twice,
//!   enemy immunity, expiry `5031B0`, the immunity purge `4FE6F0`), SUMMON
//!   (`4CCA70`: at the target point, summoned list, uncontrollable, the
//!   transport's flight time), STEAL_RESOURCES (`4CA680`,
//!   aa_economy::transfers::steal_resources), DESCRIBE_ECONOMY (`4CC050`);
//! - the Properties tick `5818D0`: periodic heal then periodic damage of the
//!   enchantments and of the actors' own properties (supply drop decay...).
//!
//! The enchantment modifiers reach combat (AV, DV, HEALTH, RECHARGE_TIME,
//! RESIST_DAMAGE, ABILITY_EFFECT), movement ([`speed_multiplier`]) and
//! supply ([`has_flag`] SUPPLIED) through the queries below.
use crate::combat;
use crate::data::{DefClass, GameData};
use crate::world::{AbilityTarget, ActorId, Command, Event, KingdomId, OrgId, World};
use crate::DT;
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[path = "special_ops/load.rs"]
mod load;
#[path = "special_ops/sai.rs"]
pub mod sai;

/// AbilityDefinition `+54` bits (aa_rules::abilities::DEFINITION_FLAGS).
pub const FLAG_ENCHANTMENT: u32 = 1 << 2;
pub const FLAG_BOMBARD: u32 = 1 << 3;
pub const FLAG_CENTER_ON_SOURCE: u32 = 1 << 4;
pub const FLAG_TARGET_LOCATION: u32 = 1 << 5;
pub const FLAG_SUMMON_MULTIPLE: u32 = 1 << 8;
pub const FLAG_SUMMON_UNIQUE: u32 = 1 << 9;
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
const BIT_INDEPENDENT: u32 = 1 << 3;
const BIT_OWN: u32 = 1 << 4;
const BIT_SELF: u32 = 1 << 5;
const BIT_ORGANIZATION: u32 = 1 << 9;
const BIT_BODY: u32 = 1 << 12;
const BIT_ACTOR: u32 = 1 << 13;
const BIT_INVULNERABLE: u32 = 1 << 14;

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

/// The kingdom's Commander (kingdom `+0x230`): its general, experience and
/// the per-ability use state.
#[derive(Clone, Debug, Default)]
pub struct Commander {
    pub general: Option<usize>,
    /// `+0xC` balance.
    pub experience: f32,
    /// FactionData `+3C`.
    pub maximum: f32,
    /// Ability `+10` last use (None: never used).
    pub last_use: Vec<Option<f32>>,
    /// Ability `+24` summoned actors.
    pub summoned: Vec<Vec<ActorId>>,
    pub use_count: Vec<u32>,
}

/// An enchantment instance (`503140`).
#[derive(Clone, Debug)]
pub struct Enchanted {
    pub target: OpTarget,
    pub kingdom: KingdomId,
    pub general: usize,
    pub ability: usize,
    pub effect: usize,
    pub created: f32,
}

/// An enchantment on a kingdom (a +SELF ability such as Radar Scan: the
/// target is the kingdom's own commander, `4C9550` phase 0).
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
    Drop { kingdom: KingdomId, general: usize, ability: usize, effect: usize, at: f32, point: Vec2, target: Option<OpTarget> },
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
    pub periodic_damage: Vec<(u32, f32)>,
    pub periodic_heal: f32,
    pub flags: Vec<String>,
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
    /// Periodic damage (type, amount) and heal of each definition's own properties.
    def_periodic: Vec<(Vec<(u32, f32)>, f32)>,
    data_key: usize,
    pub(crate) sai: sai::SaiState,
    /// Enchantments on kingdoms (+SELF abilities).
    pub kingdom_enchantments: Vec<KingdomEnchanted>,
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
    let v = value.max(0.0);
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
                let r = world.random();
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
        summon_name: op
            .effects
            .iter()
            .find_map(|e| e.summon.as_ref().filter(|_| e.kind == SUMMON))
            .and_then(|s| world.data.find(&s.object))
            .map(|d| world.data.def(d).name.clone()),
    })
}

/// `4C83E0` with recharge_bypass (only the CheatNoWait free recharge skips it).
fn recharge_remaining(world: &World, kingdom: KingdomId, index: usize) -> f32 {
    let Some(g) = general(world, kingdom) else { return 0.0 };
    let c = &world.special_ops.commanders[kingdom as usize];
    let free = world.data.rule_ci("CheatNoWait").unwrap_or(0.0) != 0.0;
    match c.last_use.get(index).copied().flatten() {
        Some(t) if !free => (g.abilities[index].recharge - (world.time - t)).max(0.0),
        _ => 0.0,
    }
}

/// `4E24B0`: the experience cost (the special operations cost no resources).
fn can_pay(world: &World, kingdom: KingdomId, op: &SpecialOp) -> bool {
    world.special_ops.commanders.get(kingdom as usize).is_some_and(|c| op.cost <= c.experience)
}

/// `4C8A40` summon and recharge checks (factions have no mana).
fn available(world: &World, kingdom: KingdomId, index: usize) -> u32 {
    let Some(g) = general(world, kingdom) else { return 1 };
    let op = &g.abilities[index];
    let c = &world.special_ops.commanders[kingdom as usize];
    let live = c.summoned.get(index).is_some_and(|s| s.iter().any(|&a| world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed)));
    if op.flags & FLAG_SUMMON_MULTIPLE == 0 && live {
        return 9;
    }
    if op.flags & FLAG_SUMMON_UNIQUE != 0
        && let Some(s) = op.effects.iter().find_map(|e| e.summon.as_ref().filter(|_| e.kind == SUMMON))
    {
        let unique = world.actors.iter().any(|a| {
            a.alive && !a.removed && a.kingdom == Some(kingdom) && {
                let flags = world.data.def(a.def).target_flags;
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
    world
        .special_ops
        .enchantments
        .iter()
        .filter(|e| e.target == target)
        .filter_map(|e| {
            let en = data.generals[e.general].abilities[e.ability].effects[e.effect].enchantment.as_ref()?;
            Some((en.name.clone(), en.icon.clone(), (en.duration - (world.time - e.created)).max(0.0)))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Targets

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

fn relation(world: &World, a: Option<KingdomId>, b: Option<KingdomId>) -> u8 {
    world.books.relation(&world.kingdoms, a, b)
}

fn allied(world: &World, a: Option<KingdomId>, b: Option<KingdomId>) -> bool {
    matches!(relation(world, a, b), crate::books::SELF | crate::books::FRIEND)
}

/// `4C5840` flags of a target seen from the kingdom `source`.
fn target_flags(world: &World, source: KingdomId, t: OpTarget) -> u32 {
    let (def, org) = match t {
        OpTarget::Actor(a) => (world.data.def(world.actors[a as usize].def), false),
        OpTarget::Org(o) => (world.data.def(world.orgs[o as usize].def), true),
    };
    let mut flags = def.target_flags | BIT_ACTOR;
    let k = owner(world, t);
    flags |= match relation(world, Some(source), k) {
        crate::books::SELF | crate::books::FRIEND => BIT_FRIEND,
        crate::books::ENEMY => BIT_ENEMY,
        _ => BIT_INDEPENDENT,
    };
    if k == Some(source) {
        flags |= BIT_OWN;
    }
    if org {
        flags |= BIT_ORGANIZATION;
    } else if def.health > 0.0 {
        flags |= BIT_BODY;
    }
    if def.invulnerable {
        flags |= BIT_INVULNERABLE;
    }
    flags
}

fn matches(required: u32, forbidden: u32, flags: u32) -> bool {
    flags & required == required && flags & forbidden == 0
}

/// `4C8740`: an actor (targetable) or organization the ability can target.
fn eligible(world: &World, kingdom: KingdomId, op: &SpecialOp, t: OpTarget) -> bool {
    if !alive(world, t) {
        return false;
    }
    if let OpTarget::Actor(a) = t
        && !world.data.def(world.actors[a as usize].def).targetable
    {
        return false;
    }
    matches(op.required, op.forbidden, target_flags(world, kingdom, t))
}

fn has_enchantment(world: &World, t: OpTarget, property: &str) -> bool {
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

fn immune(world: &World, t: OpTarget, kind: &str) -> bool {
    target_mods(world, t).is_some_and(|m| m.immune.iter().any(|k| k.as_deref().is_none_or(|k| k.eq_ignore_ascii_case(kind))))
}

// ---------------------------------------------------------------------------
// Command

/// Command::UseAbility (`61E190` -> `4CDE00`); Err is the refusal.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    let Command::UseAbility { kingdom, ability, target } = command else {
        return Err(format!("{command:?} is not a special operation"));
    };
    use_ability(world, *kingdom, *ability as usize, *target)
}

/// `4C8F20` for a faction ability (check_range true, phase 0). The target is
/// kept, moved to the clicked actor's organization, or dropped to its point.
pub fn admit(world: &World, kingdom: KingdomId, index: usize, target: &mut Option<OpTarget>) -> Result<(), String> {
    let g = general(world, kingdom).ok_or("special_operation_no_general")?;
    let op = g.abilities.get(index).ok_or("special_operation_unknown")?;
    if let Some(t) = *target
        && !eligible(world, kingdom, op, t)
    {
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
    if let Some(t) = *target {
        // 0x20000: a hostile target immune to every enchantment of the ability.
        if op.flags & FLAG_HAS_ENCHANTMENT != 0 && !allied(world, owner(world, t), Some(kingdom)) {
            let permitted = op.effects.iter().any(|e| e.enchantment.as_ref().is_none_or(|en| !immune(world, t, &en.kind)));
            if !permitted {
                return Err("special_operation_target_immune".into());
            }
        }
        // 4 + 0x20000: the single target already holds the property.
        if op.flags & FLAG_ENCHANTMENT != 0 && op.flags & FLAG_HAS_ENCHANTMENT != 0 && op.area <= 0.0 {
            if op.effects.iter().filter_map(|e| e.enchantment.as_ref()).any(|en| has_enchantment(world, t, &en.id)) {
                return Err("special_operation_already_enchanted".into());
            }
        }
    }
    if !can_pay(world, kingdom, op) {
        return Err("special_operation_not_enough_experience".into());
    }
    match available(world, kingdom, index) {
        0 => Ok(()),
        8 => Err("special_operation_recharging".into()),
        9 => Err("special_operation_summon_alive".into()),
        10 => Err("special_operation_unique".into()),
        _ => Err("special_operation_unavailable".into()),
    }
}

fn use_ability(world: &mut World, kingdom: KingdomId, index: usize, target: Option<AbilityTarget>) -> Result<(), String> {
    let data = world.special_ops.data.clone().ok_or("special operations are not loaded")?;
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
    admit(world, kingdom, index, &mut actor)?;
    let gi = world.special_ops.commanders[kingdom as usize].general.ok_or("special_operation_no_general")?;
    let op = &data.generals[gi].abilities[index];
    // A kingdom source: the target point is clamped to the map and is also
    // the source position (4C9550 phase 0).
    let point = clamp_to_map(world, actor.map_or(point, |t| position(world, t)));
    let now = world.time;
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
        c.use_count[index] += 1;
    }
    // Self effects (+SELF) first, e.g. the visibility actor over a bombed area.
    for (ei, e) in op.effects.iter().enumerate() {
        if e.required & BIT_SELF != 0 {
            apply_effect(world, kingdom, gi, index, ei, None, point, 1.0);
        }
    }
    // 4E2590 / 4CDD90: the experience cost (negative balances allowed).
    commander_mut(world, kingdom).experience -= op.cost;
    if op.projectile.is_some() {
        world.special_ops.pending.push(Pending::Impact { kingdom, general: gi, ability: index, at: now + op.projectile_life, center: point, target: actor });
    } else {
        impact(world, kingdom, gi, index, point, actor);
    }
    Ok(())
}

fn clamp_to_map(world: &World, p: Vec2) -> Vec2 {
    let m = &world.map;
    if m.width == 0 || m.height == 0 {
        return p;
    }
    p.clamp(Vec2::ZERO, Vec2::new(m.width as f32 * m.cell_size, m.height as f32 * m.cell_size))
}

/// Every actor and organization in a circle (the 4C9C90 area query).
fn in_area(world: &World, center: Vec2, radius: f32) -> Vec<OpTarget> {
    let r2 = radius * radius;
    let mut out: Vec<OpTarget> = world
        .actors
        .iter()
        .filter(|a| a.alive && !a.removed && a.pos.truncate().distance_squared(center) <= r2)
        .map(|a| OpTarget::Actor(a.id))
        .collect();
    out.extend(world.orgs.iter().filter(|o| o.alive).map(|o| OpTarget::Org(o.id)).filter(|&t| alive(world, t) && position(world, t).distance_squared(center) <= r2));
    out
}

/// `4C9550` phases 2..3: the area or the single target, then the effects.
fn impact(world: &mut World, kingdom: KingdomId, gi: usize, index: usize, center: Vec2, target: Option<OpTarget>) {
    let data = world.special_ops.data.clone().expect("attached");
    let op = &data.generals[gi].abilities[index];
    if op.area > 0.0 {
        let mut found: Vec<(f32, OpTarget)> = in_area(world, center, op.area)
            .into_iter()
            .filter(|&t| eligible(world, kingdom, op, t))
            .map(|t| (position(world, t).distance_squared(center), t))
            .collect();
        if op.max_targets > 0 {
            found.sort_by(|a, b| a.0.total_cmp(&b.0));
            found.truncate(op.max_targets as usize);
        }
        let radius = world.data.rule_ci("DefaultRadius").unwrap_or(0.5);
        let attenuated = op.attenuation_min < op.area && op.attenuation_factor < 0.99999;
        for (_, t) in found {
            let mut scale = 1.0;
            if attenuated {
                let distance = (position(world, t).distance(center) - radius).max(0.0);
                let width = op.area - op.attenuation_min;
                if distance > op.attenuation_min && width > 0.0 {
                    let ratio = (distance - op.attenuation_min) / width;
                    scale = (1.0 - (1.0 - op.attenuation_factor) * ratio).clamp(op.attenuation_factor, 1.0);
                }
            }
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

/// The non-self effects whose own target filter accepts the target.
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
        HEAL => {
            if let Some(t) = actor {
                let max = world.combat.actor(t).map(|c| c.max_health).filter(|&m| m > 0.0).unwrap_or(world.data.def(world.actors[t as usize].def).health);
                let a = &mut world.actors[t as usize];
                if a.alive {
                    a.health = (a.health + e.amount * scale).min(max);
                }
            }
        }
        ENCHANTMENT => {
            if let Some(t) = target {
                enchant(world, kingdom, gi, index, ei, t);
            } else if op.required & BIT_SELF != 0
                && !world.special_ops.kingdom_enchantments.iter().any(|k| k.kingdom == kingdom && k.general == gi && k.ability == index && k.effect == ei)
            {
                world.special_ops.kingdom_enchantments.push(KingdomEnchanted { kingdom, general: gi, ability: index, effect: ei, created: world.time });
            }
        }
        SUMMON => {
            if let Some(s) = &e.summon {
                summon(world, kingdom, gi, index, ei, s, point, target);
            }
        }
        STEAL_RESOURCES => {
            if let Some(victim) = target.and_then(|t| owner(world, t)).filter(|&k| k != kingdom) {
                steal(world, kingdom, victim, &e.steal);
            }
        }
        DESCRIBE_ECONOMY => {
            if let Some(of) = target.and_then(|t| owner(world, t)) {
                world.events.push(Event::EconomyDescribed { kingdom, of });
            }
        }
        _ => {}
    }
}

/// `4CA680` -> aa_economy::transfers::steal_resources: min(stock, amount)
/// of each stockpiled resource moves to the thief.
fn steal(world: &mut World, thief: KingdomId, victim: KingdomId, amounts: &[f32]) {
    let n = world.data.resources.len();
    let mut moved = vec![0.0; n];
    {
        let b = world.books.kingdom_mut(victim, n);
        for (i, &want) in amounts.iter().enumerate().take(n) {
            if want > 0.0 {
                let v = b.stockpiled[i].min(want).max(0.0);
                b.stockpiled[i] -= v;
                moved[i] = v;
            }
        }
    }
    let b = world.books.kingdom_mut(thief, n);
    for (i, v) in moved.into_iter().enumerate() {
        b.stockpiled[i] += v;
    }
}

/// `4CA130` -> `581700`: a target holding the property already is skipped,
/// so is a target without the required properties and an enemy immune to
/// the enchantment type; then `4FE6F0` purges the hostile enchantments the
/// new immunities cover.
fn enchant(world: &mut World, kingdom: KingdomId, gi: usize, index: usize, ei: usize, target: OpTarget) {
    let data = world.special_ops.data.clone().expect("attached");
    let Some(en) = data.generals[gi].abilities[index].effects[ei].enchantment.as_ref() else { return };
    if has_enchantment(world, target, &en.id) {
        return;
    }
    let def = match target {
        OpTarget::Actor(a) => world.data.def(world.actors[a as usize].def),
        OpTarget::Org(o) => world.data.def(world.orgs[o as usize].def),
    };
    if !en.required_properties.iter().all(|r| def.properties.iter().any(|p| p.eq_ignore_ascii_case(r))) {
        return;
    }
    let target_owner = owner(world, target);
    if !allied(world, target_owner, Some(kingdom)) && immune(world, target, &en.kind) {
        return;
    }
    world.special_ops.enchantments.push(Enchanted { target, kingdom, general: gi, ability: index, effect: ei, created: world.time });
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

/// `4CCA70`: at the target point (target_location); a transported object
/// arrives after its transport's flight from the farther map edge (`4CB110`).
#[allow(clippy::too_many_arguments)]
fn summon(world: &mut World, kingdom: KingdomId, gi: usize, index: usize, ei: usize, s: &Summon, point: Vec2, target: Option<OpTarget>) {
    if let Some(t) = s.transport.as_ref().and_then(|t| world.data.find(t)) {
        let speed = world.data.def(t).speed_max;
        let m = &world.map;
        if speed > 0.0 && m.width > 0 {
            let size = Vec2::new(m.width as f32 * m.cell_size, m.height as f32 * m.cell_size);
            let ex = point.x.max(size.x - point.x);
            let ey = point.y.max(size.y - point.y);
            let flight = ex.max(ey) / speed;
            world.special_ops.pending.push(Pending::Drop { kingdom, general: gi, ability: index, effect: ei, at: world.time + flight, point, target });
            return;
        }
    }
    create_summon(world, kingdom, index, s, point, target);
}

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
    if let Some(list) = commander_mut(world, kingdom).summoned.get_mut(index) {
        list.splice(0..0, created.iter().copied());
    }
    // 4CC470: the summoned organization attacks the targeted enemy.
    if let (Some(o), Some(t)) = (org, target)
        && relation(world, owner(world, t), Some(kingdom)) == crate::books::ENEMY
    {
        let victim = match t {
            OpTarget::Actor(a) => Some(a),
            OpTarget::Org(x) => world.orgs[x as usize].members.first().copied(),
        };
        if let Some(v) = victim {
            world.orgs[o as usize].order = crate::world::Order::Attack { target: v };
        }
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
                    OpModifier::PeriodicHeal { amount } => m.periodic_heal += amount,
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
        world.special_ops.def_periodic = data
            .defs
            .iter()
            .map(|d| {
                let mut damage = Vec::new();
                let mut heal = 0.0;
                for p in &d.properties {
                    let Some(prop) = data.properties.get(&p.to_ascii_lowercase()) else { continue };
                    for m in &prop.modifiers {
                        if m.kind.eq_ignore_ascii_case("PERIODIC_DAMAGE") {
                            damage.push((m.damage_type.unwrap_or(0), m.amount));
                        } else if m.kind.eq_ignore_ascii_case("PERIODIC_HEAL") {
                            heal += m.amount;
                        }
                    }
                }
                (damage, heal)
            })
            .collect();
    }
}

pub fn tick(world: &mut World) {
    prepare(world);
    let now = world.time;
    experience_income(world);
    let mut due = Vec::new();
    world.special_ops.pending.retain(|p| {
        let at = match p {
            Pending::Impact { at, .. } | Pending::Drop { at, .. } => *at,
        };
        if at <= now {
            due.push(p.clone());
        }
        at > now
    });
    for p in due {
        match p {
            Pending::Impact { kingdom, general, ability, center, target, .. } => impact(world, kingdom, general, ability, center, target),
            Pending::Drop { kingdom, general, ability, effect, point, target, .. } => {
                let data = world.special_ops.data.clone().expect("attached");
                if let Some(s) = data.generals[general].abilities[ability].effects[effect].summon.as_ref() {
                    create_summon(world, kingdom, ability, s, point, target);
                }
            }
        }
    }
    expire(world);
    periodic(world);
    sai::think(world);
}

/// `4E0FD0` for what the regiments gained. (core) The per-second income
/// `4CDFD0` runs in the kingdom economy tick (`crate::economy`, aa-economy
/// `Economy::tick`) on this same `Commander::experience`.
fn experience_income(world: &mut World) {
    let periodic = 0.0;
    let multiplier = world.data.rule_ci("ExperienceFactionGainMultiplier").unwrap_or(1.0);
    let ops = &mut world.special_ops;
    if ops.org_experience.len() < world.orgs.len() {
        let from = ops.org_experience.len();
        ops.org_experience.extend(world.orgs[from..].iter().map(|o| o.experience));
    }
    let mut gained = vec![0.0f32; ops.commanders.len()];
    for o in &world.orgs {
        let d = o.experience - ops.org_experience[o.id as usize];
        ops.org_experience[o.id as usize] = o.experience;
        if d > 0.0
            && let Some(g) = gained.get_mut(o.kingdom as usize)
        {
            *g += d;
        }
    }
    for (k, c) in ops.commanders.iter_mut().enumerate() {
        if c.general.is_some() {
            c.experience = clamp_experience(c.experience + periodic * DT + gained[k] * multiplier, c.maximum);
        }
    }
}

/// `5031B0`: an enchantment expires once its duration has passed.
fn expire(world: &mut World) {
    let Some(data) = world.special_ops.data.clone() else { return };
    let now = world.time;
    world.special_ops.kingdom_enchantments.retain(|e| {
        data.generals[e.general].abilities[e.ability].effects[e.effect].enchantment.as_ref().is_some_and(|en| now - e.created < en.duration)
    });
    let mut changed = Vec::new();
    let keep: Vec<bool> = world
        .special_ops
        .enchantments
        .iter()
        .map(|e| {
            let en = data.generals[e.general].abilities[e.ability].effects[e.effect].enchantment.as_ref();
            alive(world, e.target) && en.is_some_and(|en| now - e.created < en.duration)
        })
        .collect();
    let mut i = 0;
    world.special_ops.enchantments.retain(|e| {
        let k = keep[i];
        i += 1;
        if !k {
            changed.push(e.target);
        }
        k
    });
    changed.sort_by_key(|t| match t {
        OpTarget::Actor(a) => (0, *a),
        OpTarget::Org(o) => (1, *o),
    });
    changed.dedup();
    for t in changed {
        refresh_mods(world, t);
    }
}

/// `5818D0`: periodic heal, then periodic damage (with the damage type
/// factor), from the enchantments and the actor's own properties.
fn periodic(world: &mut World) {
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    for id in 0..world.actors.len() {
        let a = &world.actors[id];
        if !a.alive || a.removed {
            continue;
        }
        let own = world.special_ops.def_periodic.get(a.def as usize);
        let enchanted = world.special_ops.mods.get(id);
        let heal = own.map_or(0.0, |o| o.1) + enchanted.map_or(0.0, |m| m.periodic_heal);
        let damage: Vec<(u32, f32)> = own.map(|o| o.0.clone()).unwrap_or_default().into_iter().chain(enchanted.map(|m| m.periodic_damage.clone()).unwrap_or_default()).collect();
        if heal == 0.0 && damage.is_empty() {
            continue;
        }
        if heal > 0.0 {
            let max = st.actor(id as ActorId).map(|c| c.max_health).filter(|&m| m > 0.0).unwrap_or(world.data.def(a.def).health);
            let a = &mut world.actors[id];
            a.health = (a.health + heal * DT).min(max);
        }
        for (dt, amount) in damage {
            let amount = combat::add_damage_amount(world, &st, id as ActorId, amount * DT, dt);
            combat::periodic_damage(world, &mut st, id as ActorId, amount, dt);
        }
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
