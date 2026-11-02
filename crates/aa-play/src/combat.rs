//! Target acquisition, weapon ranges and recharge, the original damage
//! formula, area damage, death, kill credit and combat value. Owner: combat.
//!
//! A rendition of the exact ports on aa-play's world, with their x87 chains
//! at PC53 and binary32 stores (`x87` below; read the ports for the native
//! detail):
//! - hit arithmetic `004CA400` (aa_combat::hit): kind-4 damage is
//!   `max(CombatDamageMinimum, (offense*power*scale - max(defense,0)) * type
//!   factor)`, weapon flag bit1 forces offense 1, bit0 forces defense 0;
//!   offense is the shooter's organization attack efficiency (`57DBF0`, org
//!   `+84`), power is the ability power with the AV modifier (`4BE3F0`,
//!   `501A30/501AA0`), defense the body defense with DV (`501B10`), the type
//!   factor the target's RESIST_DAMAGE aggregate (`501650`/`502440`).
//! - area damage and attenuation `4C9550`/`4C9C90` (aa_combat::execution).
//! - weapon admission `4C8F20` (range, min range, target filter, recharge
//!   `4C83E0`; aa_combat::admission), performer one-ability-at-a-time
//!   (`57F290`, aa_combat::performer).
//! - organization engage list `523DD0/524C10/523EA0/51FE60`
//!   (aa_mind::cai_engage), zone entry `514E40 -> 57AB70 -> 57A500`
//!   ([`zone_entered`], from the ZOC, `crate::zones`), the
//!   damage-driven engage `579490` (aa_combat::engagement), engage Think
//!   `526A70` (1.0 s, aa_mind::cai_engage_think), target priority `527240`
//!   (aa_mind::cai_reactions::target_priority).
//! - member target choice `536E10 -> 535910 -> 534700` over the actors
//!   within the operational range (`558150`, def `+300`/`+304`) that the
//!   regiment's engage list holds (`52DCA0 -> 5210D0`), with the element
//!   priority `537530`, ranked by `52F010` (priority, then score, then
//!   distance), the per-ability decision `5340C0` (aa_mind::uai_kill) with
//!   the `52ED30` score (aa_mind::decision_score), the engage reaction
//!   delay `52E830`; a kill target is kept while valid and engaged.
//! - death: kill credit `4D57C0` (aa_body::credit) with experience
//!   `571820/571960` (aa_body::experience_amounts), slayer morale `578F10`,
//!   member removal `57CD70` (aa_combat::organization::remove_member),
//!   StartDying corpse delay `562380` (rot + sink, capped at 10 s).
//! - combat value `45E090/4BBF90/4618B0` (aa_combat::combat_value), the
//!   actor value used by experience and slayer ratios.
//!
//! Morale, routing, entrenchment, attack efficiency and experience levels are
//! in `morale.rs`; both modules share [`CombatState`] (`World::combat`).
pub mod tasks;
/// The aircraft Mind (AAI.cpp) hosted on aa-mind (core owner).
pub mod air;
/// The DefenderComponent: its ZOC list and its queue-0 Tick.
pub mod defender;

use crate::data::{DefClass, DefId, GameData, Weapon};
use crate::world::{Activity, ActorId, Event, KingdomId, Order, OrgId, World};
use crate::DT;
use glam::Vec2;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::sync::Arc;

/// `526A70` schedules the next regiment engage Think in 1.0 s.
pub const ENGAGE_THINK_SECONDS: f32 = 1.0;
/// `524D70` (sleep) and `521240` (rout retry) Think every 2.0 s.
pub const SLEEP_THINK_SECONDS: f32 = 2.0;
/// `[A34770]` 1e-4: the `52F010` score tie threshold, the `4CE770` minimum
/// award and the minimum reaction delay of `52E830`.
pub const EPSILON: f32 = 1e-4;
/// `562380`: rot time plus sink time is capped at 10 s before removal.
pub const CORPSE_DELAY_CAP: f32 = 10.0;
/// Death clip seconds when the definition has no Character die clip: the
/// original InfantryDie1.kf clip length (Units/*/Infantry rigs).
pub const DEATH_CLIP_FALLBACK: f32 = 1.0;
/// DieAnimation damage_filter 666: the death had no damage type (bleeding,
/// upkeep shortfall); -1 matches any (Templates/template_animation.tgi).
pub const DEATH_FILTER_NONE: i32 = 666;
pub const DEATH_FILTER_ANY: i32 = -1;
/// DamageType damage_filter groups (Data/Game/damage_types.tgi), used to match
/// die clips; `GameData` does not carry them yet.
pub const DAMAGE_FILTERS: [(&str, i32); 4] = [("gunfire", 0), ("flame", 1), ("explosive", 2), ("armor_piercing", 2)];
/// The actor state whose properties entrenchment grants (actor_states_aa.tgi).
pub const ENTRENCHED_STATE: &str = "organization_entrenched";
/// Spatial grid cell (metres) for the target searches.
const CELL: f32 = 16.0;
const MAX_CELLS_PER_AXIS: i32 = 256;

// ---------------------------------------------------------------------------
// Rules (SVars) and per-definition extras

/// The original rules values combat and morale read (Data/Game/SVars.tgi with
/// the SVars_AA.tgi overrides). Read from `GameData::rules`; a missing name
/// falls back to the shipped value and is listed in `missing`.
#[derive(Clone, Debug)]
pub struct Rules {
    pub damage_minimum: f32,
    pub kill_credit_timeout: f32,
    pub attacker_bonus: f32,
    pub engage_reaction_range: f32,
    pub kill_target_highest: bool,
    pub clamp_add: f32,
    pub clamp_multiply: f32,
    pub default_radius: f32,
    pub capture_health_factor: f32,
    /// ROEPrimaryDamageHistoryLength / ROEPrimaryEngageReturnTimeout (`5340C0`).
    pub roe_history_length: f32,
    pub roe_return_timeout: f32,
    // morale (SVars_AA)
    pub morale_engagement_loss: f32,
    pub morale_engagement_loss_factor_max: f32,
    pub morale_death_factor: f32,
    pub morale_retreat_penalty: f32,
    pub morale_slayer_base: f32,
    pub morale_slayer_min: f32,
    pub morale_slayer_max: f32,
    pub morale_breaker_base: f32,
    pub morale_breaker_min: f32,
    pub morale_breaker_max: f32,
    pub morale_replenish: f32,
    pub morale_supply_bonus: f32,
    pub routing_time: f32,
    pub reform_time: f32,
    pub regroup_time: f32,
    pub attack_efficiency_climbing: f32,
    pub attack_efficiency_falling: f32,
    // experience
    pub experience_minimum: f32,
    pub experience_maximum: f32,
    pub experience_kill: f32,
    pub experience_rout: f32,
    pub experience_engage: f32,
    pub experience_default: f32,
    // combat value
    pub cv_range_power: f32,
    pub cv_area_power: f32,
    pub cv_offense_weight: f32,
    pub cv_defense_weight: f32,
    pub cv_defense_weight_ranged: f32,
    pub cv_ranged_minimum: f32,
    pub cv_movement_weight: f32,
    pub cv_movement_default: f32,
    pub cv_discard: f32,
    // rout destination (52B530 / 52A930)
    pub rout_random_candidates: u32,
    pub rout_max_distance_delta: f32,
    pub rout_supply_candidates: u32,
    pub rout_supply_max_angle: f32,
    pub rout_base_value: f32,
    pub rout_factor_enemy_dist: f32,
    pub rout_factor_enemy_dist_cap: f32,
    pub rout_factor_enemy_angle: f32,
    pub rout_factor_moved_dist: f32,
    pub rout_factor_moved_dist_cap: f32,
    pub rout_factor_supply_dist: f32,
    pub rout_factor_supply_dist_cap: f32,
    pub rout_bonus_supply_in_range: f32,
    pub rout_fuzziness: f32,
    /// Names that were not in `GameData::rules` (their shipped value is used).
    pub missing: Vec<&'static str>,
}

struct RuleReader<'a> {
    data: &'a GameData,
    missing: Vec<&'static str>,
}
impl RuleReader<'_> {
    fn get(&mut self, name: &'static str, shipped: f32) -> f32 {
        match self.data.rule_ci(name) {
            Some(v) if v.is_finite() => v,
            _ => {
                self.missing.push(name);
                shipped
            }
        }
    }
}

impl Rules {
    /// Shipped values are the SVars_AA.tgi override when it exists, else SVars.tgi.
    pub fn from_data(data: &GameData) -> Self {
        let mut r = RuleReader { data, missing: Vec::new() };
        let mut rules = Rules {
            damage_minimum: r.get("CombatDamageMinimum", 1.0),
            kill_credit_timeout: r.get("KillCreditTimeout", 2.0),
            attacker_bonus: r.get("AbilityScoreAttackerBonus", 0.5),
            engage_reaction_range: r.get("EngageReactionTimeRange", 0.3),
            kill_target_highest: r.get("KillTargetIsHighestPriority", 1.0) != 0.0,
            clamp_add: r.get("ModifierAttributeClampAdd", 1.0),
            clamp_multiply: r.get("ModifierAttributeClampMultiply", 0.05),
            default_radius: r.get("DefaultRadius", 0.5),
            capture_health_factor: r.get("CaptureHealthFactor", 0.3),
            roe_history_length: r.get("ROEPrimaryDamageHistoryLength", 3.0),
            roe_return_timeout: r.get("ROEPrimaryEngageReturnTimeout", 5.0),
            morale_engagement_loss: r.get("MoraleEngagementLoss", 0.5),
            morale_engagement_loss_factor_max: r.get("MoraleEngagementLossFactorMax", 3.0),
            morale_death_factor: r.get("MoraleDeathFactor", 0.25),
            morale_retreat_penalty: r.get("MoraleRetreatPenalty", 0.0),
            morale_slayer_base: r.get("MoraleSlayerBase", 0.5),
            morale_slayer_min: r.get("MoraleSlayerFactorMin", 0.5),
            morale_slayer_max: r.get("MoraleSlayerFactorMax", 3.0),
            morale_breaker_base: r.get("MoraleBreakerBase", 3.0),
            morale_breaker_min: r.get("MoraleBreakerFactorMin", 0.5),
            morale_breaker_max: r.get("MoraleBreakerFactorMax", 2.0),
            morale_replenish: r.get("MoraleReplenish", 2.0),
            morale_supply_bonus: r.get("MoraleSupplyBonus", 1.0),
            routing_time: r.get("RoutingTime", 10.0),
            reform_time: r.get("ReformTime", 10.0),
            regroup_time: r.get("RegroupTime", 10.0),
            attack_efficiency_climbing: r.get("AttackEfficiencyPointsPerSecondClimbing", 0.01),
            attack_efficiency_falling: r.get("AttackEfficiencyPointsPerSecondFalling", 0.02),
            experience_minimum: r.get("ExperienceGainMinimum", 0.0),
            experience_maximum: r.get("ExperienceGainMaximum", 10.0),
            experience_kill: r.get("ExperienceKillMultiplier", 1.0),
            experience_rout: r.get("ExperienceRoutMultiplier", 1.0),
            experience_engage: r.get("ExperienceEngageMultiplier", 1.0),
            experience_default: r.get("ExperienceDefaultValue", 1.0),
            cv_range_power: r.get("CombatValueRangePower", 0.15),
            cv_area_power: r.get("CombatValueAreaPower", 1.0),
            cv_offense_weight: r.get("CombatValueOffenseWeight", 0.5),
            cv_defense_weight: r.get("CombatValueDefenseWeight", 0.5),
            cv_defense_weight_ranged: r.get("CombatValueDefenseWeightRanged", 0.25),
            cv_ranged_minimum: r.get("CombatValueRangedMinimum", 12.0),
            cv_movement_weight: r.get("CombatValueMovementWeight", 0.2),
            cv_movement_default: r.get("CombatValueMovementDefault", 1.0),
            cv_discard: r.get("CombatValueMeanValuesDiscardPercent", 0.2),
            rout_random_candidates: r.get("RoutRandomCandidates", 20.0).max(0.0) as u32,
            rout_max_distance_delta: r.get("RoutMaxDistanceDelta", 24.0),
            rout_supply_candidates: r.get("RoutSupplyZoneCandidates", 2.0).max(0.0) as u32,
            rout_supply_max_angle: r.get("RoutSupplyZoneMaxAngleDelta", 50.0),
            rout_base_value: r.get("RoutBaseValue", 0.0),
            rout_factor_enemy_dist: r.get("RoutFactorEnemyDist", 10.0),
            rout_factor_enemy_dist_cap: r.get("RoutFactorEnemyDistCap", 2000.0),
            rout_factor_enemy_angle: r.get("RoutFactorEnemyAngle", 0.25),
            rout_factor_moved_dist: r.get("RoutFactorMovedDist", 8.0),
            rout_factor_moved_dist_cap: r.get("RoutFactorMovedDistCap", 2000.0),
            rout_factor_supply_dist: r.get("RoutFactorSupplyDist", 1.0),
            rout_factor_supply_dist_cap: r.get("RoutFactorSupplyDistCap", 2000.0),
            rout_bonus_supply_in_range: r.get("RoutBonusSupplyInRange", 50.0),
            rout_fuzziness: r.get("RoutFuzziness", 20.0),
            missing: Vec::new(),
        };
        rules.missing = r.missing;
        rules
    }
}

/// One ExperienceComponent level (Templates/template_component_aa.tgi): the
/// threshold and the numeric modifiers of its ModifierProperty.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExperienceLevel {
    pub required: f32,
    /// AddModifier AV / AV_RANGED / AV_MELEE (ability power) and DV (defense).
    pub av: f32,
    pub av_ranged: f32,
    pub av_melee: f32,
    pub dv: f32,
    /// MultiplyModifier HEALTH (maximum health x (1 + amount)).
    pub health: f32,
    /// AddModifier MORALE (each member's morale value).
    pub morale: f32,
    /// MultiplyModifier MORALE_RECOVERY_RATE (replenish x (1 + amount)).
    pub morale_recovery: f32,
    /// AddModifier MORALE_BREAKPOINT (the `501CF0` getter subtracts it).
    pub morale_breakpoint: f32,
}

impl ExperienceLevel {
    /// From the typed `data::ExperienceLevel` (its ModifierProperty amounts).
    pub fn from_data(l: &crate::data::ExperienceLevel) -> Self {
        ExperienceLevel {
            required: l.required,
            av: l.amount("AV"),
            av_ranged: l.amount("AV_RANGED"),
            av_melee: l.amount("AV_MELEE"),
            dv: l.amount("DV"),
            health: l.amount("HEALTH"),
            morale: l.amount("MORALE"),
            morale_recovery: l.amount("MORALE_RECOVERY_RATE"),
            morale_breakpoint: l.amount("MORALE_BREAKPOINT"),
        }
    }
}

/// Per-definition combat values derived once from `GameData`.
#[derive(Clone, Debug, Default)]
pub struct DefExtras {
    /// Seconds of the first die clip (`Def::death_time`), used when no clip
    /// is chosen.
    pub death_time: f32,
    /// ExperienceComponent fake_cv_value (template default -1).
    pub fake_cv_value: f32,
    /// ExperienceComponent levels of an organization, lowest first.
    pub levels: Vec<ExperienceLevel>,
    /// Combat value (`4618B0` definition value), computed.
    pub combat_value: f32,
    /// Performer `+60` of a standing actor of this definition (`57E430`, the
    /// Defender abilities too), [`performer_values`].
    pub performer_value: f32,
    /// The organization_entrenched state's properties this definition
    /// qualifies for (their required_properties): ADD DV, RESIST_DAMAGE per
    /// damage type, MULTIPLY MORALE_LOSS_RATE.
    pub entrench_dv: f32,
    pub entrench_resist: Vec<f32>,
    pub entrench_morale_loss: f32,
}

/// Values read once per `GameData`.
#[derive(Clone, Debug, Default)]
pub struct Extras {
    pub defs: Vec<DefExtras>,
    /// `4BBF90`, for the per-actor `57E430`.
    means: Means,
    /// `GameData::target_types` bit indices used by the admission filter.
    pub bit_friend: Option<u32>,
    pub bit_enemy: Option<u32>,
    pub bit_independent: Option<u32>,
    pub bit_own: Option<u32>,
    pub bit_self: Option<u32>,
    pub bit_same_organization: Option<u32>,
    pub bit_body: Option<u32>,
    pub bit_actor: Option<u32>,
    pub bit_routing: Option<u32>,
    /// (core) The `neutral` TargetType bit (4C5840 relation 2).
    pub bit_neutral: Option<u32>,
    /// (core) `under_construction`: actor state 21 (actor +0x100 bit 21).
    pub bit_under_construction: Option<u32>,
    /// damage_filter group of every `GameData::damage_types` entry.
    pub damage_filters: Vec<i32>,
    /// Definitions using a fallback (count per field).
    pub fallbacks: Vec<(&'static str, usize)>,
    /// The largest `Size` soft radius, for the area query's cell search.
    pub max_soft_radius: f32,
}

/// Legacy per-definition rules keys, read when the typed field is empty:
/// `<thing id>.death_time`, `<thing id>.fake_cv_value` and
/// `<thing id>.experience<N>.<ATTRIBUTE>` (`required`, `AV`, `DV`, ...).
pub const EXTRA_KEYS: &[&str] = &["death_time", "fake_cv_value", "experience<N>.required"];

impl Extras {
    pub fn from_data(data: &GameData, rules: &Rules) -> Self {
        let bit = |name: &str| {
            data.target_types
                .iter()
                .position(|t| t.eq_ignore_ascii_case(name))
                .map(|i| i as u32)
                .filter(|&i| i < 32)
        };
        let entrench_props: Vec<&crate::data::PropertyDef> = data
            .actor_states
            .get(ENTRENCHED_STATE)
            .map(|s| s.properties.iter().filter_map(|p| data.properties.get(&p.to_ascii_lowercase())).collect())
            .unwrap_or_default();
        let mut death_missing = 0;
        let mut defs = Vec::with_capacity(data.defs.len());
        for def in &data.defs {
            let key = |field: &str| data.rule_ci(&format!("{}.{}", def.id, field));
            let death_time = def.death_time.or_else(|| key("death_time")).unwrap_or_else(|| {
                if matches!(def.class, Some(DefClass::Member)) {
                    death_missing += 1;
                }
                DEATH_CLIP_FALLBACK
            });
            let fake_cv_value = if def.has_component("ExperienceComponent") {
                def.fake_cv_value
            } else {
                key("fake_cv_value").unwrap_or(-1.0)
            };
            let mut levels: Vec<ExperienceLevel> = def.experience_levels.iter().map(ExperienceLevel::from_data).collect();
            if levels.is_empty() && matches!(def.class, Some(DefClass::Organization)) {
                for n in 0.. {
                    let level = |field: &str| key(&format!("experience{n}.{field}"));
                    let Some(required) = level("required") else { break };
                    levels.push(ExperienceLevel {
                        required,
                        av: level("AV").unwrap_or(0.0),
                        av_ranged: level("AV_RANGED").unwrap_or(0.0),
                        av_melee: level("AV_MELEE").unwrap_or(0.0),
                        dv: level("DV").unwrap_or(0.0),
                        health: level("HEALTH").unwrap_or(0.0),
                        morale: level("MORALE").unwrap_or(0.0),
                        morale_recovery: level("MORALE_RECOVERY_RATE").unwrap_or(0.0),
                        morale_breakpoint: level("MORALE_BREAKPOINT").unwrap_or(0.0),
                    });
                }
            }
            let has = |p: &str| def.properties.iter().any(|q| q.eq_ignore_ascii_case(p));
            let mut ex = DefExtras {
                death_time,
                fake_cv_value,
                levels,
                combat_value: 0.0,
                performer_value: 0.0,
                entrench_dv: 0.0,
                entrench_resist: vec![0.0; data.damage_types.len()],
                entrench_morale_loss: 0.0,
            };
            for prop in entrench_props.iter().filter(|p| p.required_properties.iter().all(|r| has(r))) {
                for m in &prop.modifiers {
                    let attribute = m.attribute.as_deref().unwrap_or("");
                    if m.kind.eq_ignore_ascii_case("ADD") && attribute.eq_ignore_ascii_case("DV") {
                        ex.entrench_dv += m.amount;
                    } else if m.kind.eq_ignore_ascii_case("RESIST_DAMAGE")
                        && let Some(r) = m.damage_type.and_then(|t| ex.entrench_resist.get_mut(t as usize))
                    {
                        *r += m.amount;
                    } else if m.kind.eq_ignore_ascii_case("MULTIPLY") && attribute.eq_ignore_ascii_case("MORALE_LOSS_RATE") {
                        ex.entrench_morale_loss += m.amount;
                    }
                }
            }
            defs.push(ex);
        }
        let values = combat_values(data, rules);
        for (d, v) in defs.iter_mut().zip(values) {
            d.combat_value = v;
        }
        let values = performer_values(data, rules);
        for (d, v) in defs.iter_mut().zip(values) {
            d.performer_value = v;
        }
        let damage_filters = data
            .damage_types
            .iter()
            .map(|t| {
                data.rule_ci(&format!("{t}.damage_filter")).map(|v| v as i32).unwrap_or_else(|| {
                    DAMAGE_FILTERS.iter().find(|(n, _)| n.eq_ignore_ascii_case(t)).map_or(DEATH_FILTER_ANY, |(_, f)| *f)
                })
            })
            .collect();
        Extras {
            defs,
            means: means(data, rules),
            bit_friend: bit("friend"),
            bit_enemy: bit("enemy"),
            bit_independent: bit("independent"),
            bit_neutral: bit("neutral"),
            bit_under_construction: bit("under_construction"),
            bit_own: bit("own"),
            bit_self: bit("self"),
            bit_same_organization: bit("same_organization"),
            bit_body: bit("body"),
            bit_actor: bit("actor"),
            bit_routing: bit("routing"),
            damage_filters,
            fallbacks: vec![("death_time", death_missing)],
            max_soft_radius: data.defs.iter().map(|d| d.soft_radius).fold(0.0, f32::max),
        }
    }
    pub fn def(&self, id: DefId) -> &DefExtras {
        static EMPTY: DefExtras = DefExtras {
            death_time: DEATH_CLIP_FALLBACK,
            fake_cv_value: -1.0,
            levels: Vec::new(),
            combat_value: 0.0,
            performer_value: 0.0,
            entrench_dv: 0.0,
            entrench_resist: Vec::new(),
            entrench_morale_loss: 0.0,
        };
        self.defs.get(id as usize).unwrap_or(&EMPTY)
    }
}

/// The die clip played for a death (DieAnimation, template_animation.tgi):
/// among the clips whose damage_threshold is at most the last damage and
/// whose damage_filter is -1, the last damage type's group, or 666 for a
/// death without a damage type, a weighted random pick (`u` in [0, 1)).
pub fn choose_death_clip(clips: &[crate::data::DeathClip], amount: f32, filter: Option<i32>, u: f32) -> Option<usize> {
    let eligible = |c: &crate::data::DeathClip| {
        let f = c.damage_filter as i32;
        c.damage_threshold <= amount && (f == DEATH_FILTER_ANY || Some(f) == filter || (filter.is_none() && f == DEATH_FILTER_NONE))
    };
    let total: f32 = clips.iter().filter(|c| eligible(c)).map(|c| c.weight.max(0.0)).sum();
    if total <= 0.0 {
        return clips.iter().position(eligible);
    }
    let mut r = u * total;
    let mut last = None;
    for (i, c) in clips.iter().enumerate().filter(|(_, c)| eligible(c)) {
        last = Some(i);
        r -= c.weight.max(0.0);
        if r < 0.0 {
            return Some(i);
        }
    }
    last
}

// ---------------------------------------------------------------------------
// Combat value (4BBF90 means, 45E090 ability value, 4618B0 definition value)

#[derive(Clone, Copy, Debug, Default)]
struct Means {
    attack: f32,
    defense: f32,
    movement: f32,
    health: f32,
    lifetime: f32,
    offense: f32,
}

/// The ActorData `+174` component bits the combat value reads (Body 0,
/// Mover 3, Performer 5, Element 8, Experience 17). The component's own
/// fields count too (health, speed, performer abilities, an element
/// class), which only that component loads: definitions built without the
/// bits (synthetic data) answer the same.
struct Components {
    body: bool,
    mover: bool,
    performer: bool,
    element: bool,
    experience: bool,
}
impl Components {
    fn of(def: &crate::data::Def) -> Self {
        Components {
            body: def.has_component("BodyComponent") || def.health > 0.0,
            mover: def.has_component("MoverComponent") || def.speed_max > 0.0,
            performer: def.has_component("PerformerComponent") || def.weapons.iter().any(|w| !w.defender),
            element: def.has_component("ElementComponent") || matches!(def.class, Some(DefClass::Member)),
            experience: def.has_component("ExperienceComponent"),
        }
    }
}

/// The CRT `pow` (`_CIpow` 0x96D890, SSE2 body at PC53; aa_numeric::crt_pow),
/// the host `powf` only where the port declines.
fn crt_pow(x: f64, y: f64) -> f64 {
    aa_numeric::crt_pow::ci_pow(x, y, aa_numeric::crt_pow::CrtEnvironment::SSE2_DOUBLE53).map_or_else(|_| x.powf(y), |o| o.value())
}

/// `4BBF90` (aa_combat::combat_value::mean_values) over the combat things
/// in definition order (Performer, Body and Mover, not an Element with an
/// ExperienceComponent): every positive PerformerComponent ability power,
/// the body defense, the mover speed and the body health. Each column is
/// sorted descending (`869D30`/`4BB850`: new equal keys after the existing
/// ones), `ftol(count * discard)` dropped at each end, summed in order with
/// a store per add, times the unstored `1/n`, stored. Offense is
/// `attack - defense` and lifetime `fstp(health / offense)`.
fn means(data: &GameData, rules: &Rules) -> Means {
    let mut attack = Vec::new();
    let (mut defense, mut movement, mut health) = (Vec::new(), Vec::new(), Vec::new());
    for def in &data.defs {
        let c = Components::of(def);
        if !(c.performer && c.body && c.mover && (!c.experience || !c.element)) {
            continue;
        }
        for w in def.weapons.iter().filter(|w| !w.defender) {
            if w.damage > 0.0 {
                attack.push(w.damage);
            }
        }
        defense.push(def.defense);
        movement.push(def.speed_max);
        health.push(def.health);
    }
    let trimmed = |v: &mut Vec<f32>| -> f32 {
        if v.is_empty() {
            return 0.0;
        }
        v.sort_by(|a, b| b.total_cmp(a));
        let mut k = x87::mul(v.len() as f64, ld(rules.cv_discard)).trunc() as usize;
        if 2 * k >= v.len() {
            // Outside the native domain (an empty trimmed range divides by zero).
            k = 0;
        }
        let slice = &v[k..v.len() - k];
        let sum = slice.iter().fold(0.0f32, |s, &x| fstp(x87::add(ld(x), ld(s))));
        fstp(x87::mul(x87::div(1.0, slice.len() as f64), ld(sum)))
    };
    let mut m = Means {
        attack: trimmed(&mut attack),
        defense: trimmed(&mut defense),
        movement: trimmed(&mut movement),
        health: trimmed(&mut health),
        ..Means::default()
    };
    let offense = x87::sub(ld(m.attack), ld(m.defense));
    // A zero offense divides by zero natively (degenerate data only).
    m.lifetime = if offense != 0.0 { fstp(x87::div(ld(m.health), offense)) } else { 0.0 };
    m.offense = fstp(offense);
    m
}

/// `45E090` (aa_combat::combat_value::ability_value) for one ability of a
/// definition without modifiers: 0 for an ignore_for_cv ability (`+54` bit
/// 7); the offense `fstp(area_factor * fstp(range^RangePower) * rate)`
/// with `rate = fstp(max(power - mean defense, CombatDamageMinimum) /
/// cycle)`; lifetime, movement, weighted offense, life ratio and defense
/// factor each stored; the value `fstp(defense_factor * weighted_offense *
/// movement)` (the property cv sum and the `+210` bias add 0 in the
/// shipped data). Where the original would divide by zero (a zero cycle,
/// zero mean movement or lifetime: degenerate data, never the shipped
/// one) the value stays finite.
fn ability_value(w: &Weapon, defense: f32, health: f32, speed: f32, m: &Means, r: &Rules) -> f32 {
    if w.flags & 0x80 != 0 {
        return 0.0;
    }
    let ranged = !(w.range < r.cv_ranged_minimum);
    let mut offense = 0.0f32;
    if w.damage > 0.0 {
        let mut damage = fstp(x87::sub(ld(w.damage), ld(m.defense)));
        if damage < r.damage_minimum {
            damage = r.damage_minimum;
        }
        let rate = fstp(x87::div(ld(damage), ld(w.recharge.max(EPSILON))));
        let range_factor = fstp(crt_pow(ld(w.range), ld(r.cv_range_power)));
        let area = x87::add(x87::mul(ld(w.area), 0.5), 1.0);
        let area_factor = crt_pow(area, ld(r.cv_area_power));
        offense = fstp(x87::mul(x87::mul(area_factor, ld(range_factor)), ld(rate)));
    }
    let mut lifetime = 0.0f32;
    if health > 0.0 {
        lifetime = fstp(x87::div(ld(health), x87::sub(ld(m.attack), ld(defense))));
    }
    let mut movement = if m.movement != 0.0 {
        fstp(x87::add(x87::mul(x87::sub(x87::div(ld(speed), ld(m.movement)), 1.0), ld(r.cv_movement_weight)), 1.0))
    } else {
        0.0
    };
    if !(movement > 0.0) {
        movement = r.cv_movement_default;
    }
    let weighted_offense = fstp(x87::add(x87::mul(x87::sub(ld(offense), ld(m.offense)), ld(r.cv_offense_weight)), ld(m.offense)));
    let life_ratio = if m.lifetime != 0.0 { fstp(x87::div(ld(lifetime), ld(m.lifetime))) } else { 0.0 };
    let weight = if life_ratio < 1.0 && ranged { r.cv_defense_weight_ranged } else { r.cv_defense_weight };
    let defense_factor = fstp(x87::add(x87::mul(x87::sub(ld(life_ratio), 1.0), ld(weight)), 1.0));
    let value = fstp(x87::mul(x87::mul(ld(defense_factor), ld(weighted_offense)), ld(movement)));
    if value.is_finite() { value } else { 0.0 }
}

/// `4618B0` definition value (`+1E4`): with a PerformerComponent the best
/// `45E090` value over its abilities (a later equal value replaces), with
/// the body's defense and health and the mover's speed; 0 otherwise.
pub fn combat_values(data: &GameData, rules: &Rules) -> Vec<f32> {
    let m = means(data, rules);
    data.defs
        .iter()
        .map(|def| {
            let c = Components::of(def);
            if !c.performer {
                return 0.0;
            }
            let (defense, health) = if c.body { (def.defense, def.health) } else { (0.0, 0.0) };
            let speed = if c.mover { def.speed_max } else { 0.0 };
            def.weapons
                .iter()
                .filter(|w| !w.defender)
                .map(|w| ability_value(w, defense, health, speed, &m, rules))
                .fold(0.0f32, |best, v| if !(best > v) { v } else { best })
        })
        .collect()
}

/// `57E430` Performer `+60` from Body `+18` / `+14` and Mover `+28`: with a
/// PerformerComponent the best `45E090` value (a later equal value replaces,
/// a smaller one keeps) over the Performer's abilities and then the
/// DefenderComponent's (`+B0 +10`), so a corps or a bunker that shoots only
/// through its Defender has a value. [`combat_values`] is the definition's
/// `+1E4`, which leaves the Defender out; experience, the AI's definition
/// values and the other `+1E4` readers stay on it.
fn performer_value_of(def: &crate::data::Def, health: f32, speed: f32, m: &Means, rules: &Rules) -> f32 {
    let c = Components::of(def);
    if !c.performer {
        return 0.0;
    }
    let (defense, health) = if c.body { (def.defense, health) } else { (0.0, 0.0) };
    let speed = if c.mover { speed } else { 0.0 };
    def.weapons
        .iter()
        .filter(|w| !w.defender)
        .chain(def.weapons.iter().filter(|w| w.defender))
        .map(|w| ability_value(w, defense, health, speed, m, rules))
        .fold(0.0f32, |best, v| if !(best > v) { v } else { best })
}

/// [`performer_value_of`] of a thing of each definition with the definition's own
/// values: what a slot not yet filled (`0x564C10`'s organization sum) and the AI's
/// estimates read. A thing has its own, from its creation (`57E430` runs in
/// `4D7230` and in the Defender's InitExisting `566DB0`, and not again):
/// `ActorCombat::performer_value`.
pub fn performer_values(data: &GameData, rules: &Rules) -> Vec<f32> {
    let m = means(data, rules);
    data.defs.iter().map(|def| performer_value_of(def, def.health, def.speed_max, &m, rules)).collect()
}

/// `571820` (aa_body::experience_amounts::ratio): `victim/source*0.5 + 0.5`
/// left unstored (x87); a zero victim value becomes -1 when the victim's
/// fake_cv_value (+4F4) is -1, else ExperienceDefaultValue; a nonpositive
/// source value gives 0.
pub fn experience_ratio(victim_value: f32, victim_fake_cv: f32, source_value: f32, rules: &Rules) -> f64 {
    let mut victim = victim_value;
    if victim == 0.0 {
        victim = if victim_fake_cv.to_bits() == 0xbf80_0000 { -1.0 } else { rules.experience_default };
    }
    if source_value <= 0.0 {
        return 0.0;
    }
    x87::add(x87::mul(x87::div(ld(victim), ld(source_value)), 0.5), 0.5)
}

/// `571960`/`5718E0`: the ratio times the multiplier, stored, then clamped to
/// [ExperienceGainMinimum, ExperienceGainMaximum] (minimum checked first).
pub fn experience_clamped(ratio: f64, multiplier: f32, rules: &Rules) -> f32 {
    let product = fstp(x87::mul(ratio, ld(multiplier)));
    if product < rules.experience_minimum {
        rules.experience_minimum
    } else if product > rules.experience_maximum {
        rules.experience_maximum
    } else {
        product
    }
}

// ---------------------------------------------------------------------------
// x87 arithmetic

/// The simulation thread's x87 at PC53 for binary32 operands: one rounding
/// to 53 bits per operation, binary32 only where the original stores
/// (`fstp dword`). `aa_numeric::pc53` is exact for every normal-range
/// result; where it declines (subnormal or overflowing results, which the
/// combat values never reach) the plain binary64 operation is used.
pub(crate) mod x87 {
    use aa_numeric::pc53;
    #[inline]
    pub fn add(a: f64, b: f64) -> f64 {
        pc53::add(a, b).unwrap_or(a + b)
    }
    #[inline]
    pub fn sub(a: f64, b: f64) -> f64 {
        pc53::sub(a, b).unwrap_or(a - b)
    }
    #[inline]
    pub fn mul(a: f64, b: f64) -> f64 {
        pc53::mul(a, b).unwrap_or(a * b)
    }
    #[inline]
    pub fn div(a: f64, b: f64) -> f64 {
        pc53::div(a, b).unwrap_or(a / b)
    }
    /// `fstp dword`.
    #[inline]
    pub fn store(v: f64) -> f32 {
        v as f32
    }
    /// `fld dword`.
    #[inline]
    pub fn load(v: f32) -> f64 {
        f64::from(v)
    }
}
use x87::{load as ld, store as fstp};

// ---------------------------------------------------------------------------
// The damage formula

/// `004CA400` kind-4 arithmetic (aa_combat::hit). `power` is the ability
/// power (already with AV), `defense` the body defense (with DV), `factor`
/// the damage-type factor: `offense*power*scale - defense`, times the
/// factor, in one x87 chain with a single binary32 store, then
/// CombatDamageMinimum.
pub fn damage_amount(power: f32, offense: f32, scale: f32, defense: f32, factor: f32, flags: u32, minimum: f32) -> f32 {
    let base = if power < 0.0 { 0.0 } else { power };
    let offense = if flags & 2 != 0 { 1.0 } else { offense };
    let defense = if flags & 1 != 0 || defense < 0.0 { 0.0 } else { defense };
    let amount = fstp(x87::mul(x87::sub(x87::mul(x87::mul(ld(offense), ld(base)), ld(scale)), ld(defense)), ld(factor)));
    if amount < minimum { minimum } else { amount }
}

/// `501A30/501AA0` ModifierState::power: base plus the kind add (AV_MELEE or
/// AV_RANGED) plus AV, raised to `min(base + kind, ModifierAttributeClampAdd)`
/// and to 0.
pub fn modified_power(base: f32, kind_add: f32, av: f32, clamp_add: f32) -> f32 {
    let x = base + kind_add;
    let floor = x.min(clamp_add);
    (x + av).max(floor).max(0.0)
}

/// `501B10` ModifierState::defense: base + DV floored at `min(base, clamp)` and 0.
pub fn modified_defense(base: f32, dv: f32, clamp_add: f32) -> f32 {
    let floor = base.min(clamp_add);
    (base + dv).max(floor).max(0.0)
}

// ---------------------------------------------------------------------------
// State

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetRef {
    Org(OrgId),
    Actor(ActorId),
}

/// One `+130` node (`4D6190`, a bombardment's hit): who shells this thing, when
/// it last did and how long that counts (`4D37E0` drops it after).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackedBy {
    pub attacker: TargetRef,
    pub at: f32,
    pub timeout: f32,
}

/// One `{gtid, priority}` node of an organization's engage list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngageTarget {
    pub target: TargetRef,
    pub priority: i32,
}

/// `KKC_CAIStateRout` (`51F190` / `521240`).
#[derive(Clone, Debug, PartialEq)]
pub struct Rout {
    pub since: f32,
    pub destination: Option<Vec2>,
    pub next_think: f32,
}

/// `KKC_CAIStateReform` (Mind `+1B8`, Think `51DE70`): entered when a rout
/// reaches its destination with the regiment no longer exhausted.
#[derive(Clone, Debug, PartialEq)]
pub struct Reform {
    /// State `+10` start time.
    pub since: f32,
    pub next_think: f32,
}

/// Combat and morale state of one organization (OrganizationComponent and
/// its regiment Mind). Readable by the UI and the opponent.
#[derive(Clone, Debug, Default)]
pub struct OrgCombat {
    pub initialized: bool,
    /// A saved game's organization: the living count its +60 was read at. The Load
    /// reads +60 before the kingdom's properties reach the members, and it stands
    /// until a change of modifiers (`saved_world`); the periodic refresh leaves it.
    pub saved_morale_living: Option<i32>,
    pub destroyed: bool,
    /// Engage list, ascending priority (`51FE60`); the last entry is engaged.
    pub engage: Vec<EngageTarget>,
    /// Target of an explicit (player) attack order.
    pub explicit: Option<TargetRef>,
    /// The Kill state came from a `move` command on the target (`0x527E30` ->
    /// `0x51D920`): Mind +0x20's kind 5, so `0x51D8F0` reports mind_moving and the
    /// regiment walks in its desired formation and behaviour.
    pub kill_is_move: bool,
    /// The city a Kill state walks to, and the point of the walk
    /// (`movement::command`): the state ends when `0x57A500` no longer accepts the
    /// city (`0x520730` -> Mind +0x140), its capture making it no enemy.
    pub city_target: Option<(ActorId, Vec2)>,
    /// Order replaced while engaging on the organization's own initiative,
    /// restored when the engagement ends.
    pub saved_order: Option<Order>,
    /// The order combat wrote (to tell it apart from a new player order).
    pub auto_order: Option<Order>,
    pub previous_order: Order,
    pub next_think: f32,
    pub next_sleep_think: f32,
    /// Organization position (`+20/+24`, follows its leader).
    pub position: Vec2,
    /// `+64` living members on morale lines and the definition slot count.
    pub living: i32,
    pub slots: i32,
    /// Enemy organizations whose engage list holds this one (actor `+118`).
    pub engaged_by: Vec<OrgId>,
    /// Defenders holding a member as an ability's target (`567140`, the same
    /// `+118` list).
    pub defended_by: Vec<ActorId>,
    /// `+8D`: the current Mind state's behavior_required (`577F30`), kept to see
    /// it turn aggressive.
    pub behavior_8d: bool,
    /// `+8E`: `+8D` has just turned on; the maintenance Tick `57C8E0` then walks
    /// the ZOC's effects (`5147C0`).
    pub rescan_8e: bool,
    /// The order last seen, to tell a new command (`527E30` leaves the engage
    /// state, `51F040`) from what combat itself restored.
    pub seen_order: Option<Order>,
    /// Who shells this organization (`+130`, its members' hits come here).
    pub attacked_by: Vec<AttackedBy>,
    /// What this organization shells (`+128`, `4D3A70`), head first.
    pub bombarding: Vec<TargetRef>,
    /// `+60` maximum morale and the breakpoint `+B8` (with level modifiers).
    pub morale_max: f32,
    pub breakpoint: f32,
    /// `+4C` / `+4D` latches of `57A0A0` / `579600`.
    pub engaged_latch: bool,
    pub engage_latch: bool,
    /// `+84` actual and `+88` desired attack efficiency.
    pub attack_efficiency: f32,
    pub attack_efficiency_desired: f32,
    /// Actor state 5 `mind_exhausted`.
    pub exhausted: bool,
    pub routing: Option<Rout>,
    /// The reform state after a rout's arrival (`5222E0`).
    pub reform: Option<Reform>,
    /// The exhausted Mind state (Mind `+270`, Think `51E260`) that follows a
    /// rout ending at zero morale, until the morale is steady.
    pub exhausted_state: bool,
    /// Current experience level index (into `DefExtras::levels`).
    pub level: usize,
    /// Since when the organization has been in an entrenchable state.
    pub rest_since: Option<f32>,
    /// Actor state 22 `organization_entrenched`.
    pub entrenched: bool,
    /// Last hostile damage on a member (`+90` record): shooter and time.
    pub last_hit: Option<(ActorId, f32)>,
    // ---- appended by the core owner (HUD orders, `tasks`) ----
    /// The bombard / repair / construct Mind state replacing the engage one.
    pub task: Option<tasks::Task>,
    /// The order the task wrote last (a different one is a new command).
    pub task_order: Option<Order>,
    /// Next task Think.
    pub task_think: f32,
}

impl OrgCombat {
    /// Actor `+118` is not empty: an organization's engage list or a Defender's
    /// ability holds this one.
    pub fn is_engaged(&self) -> bool {
        !self.engaged_by.is_empty() || !self.defended_by.is_empty()
    }
    pub fn current_target(&self) -> Option<TargetRef> {
        self.engage.last().map(|t| t.target)
    }
    pub fn engaging(&self) -> bool {
        !self.engage.is_empty() && self.routing.is_none()
    }
}

/// Combat state of one actor.
#[derive(Clone, Debug, Default)]
pub struct ActorCombat {
    pub initialized: bool,
    /// Maximum health with the HEALTH modifier of the organization level.
    pub max_health: f32,
    /// Last hostile damage (Body `+18` record): shooter and time.
    pub last_hit: Option<(ActorId, f32)>,
    /// Engage reaction delay end (`52E830`).
    pub ready_at: f32,
    /// Lone actor (no organization) Think time.
    pub next_think: f32,
    pub queued_death: bool,
    pub died_at: Option<f32>,
    pub dead_at: f32,
    pub remove_at: f32,
    /// Last damage amount and damage type (None without a type), for the die clip.
    pub last_damage: Option<(f32, Option<u32>)>,
    /// Index into `Def::death_clips` of the clip chosen at death.
    pub death_clip: Option<usize>,
    /// Some enemy has this actor as its combat target (actor `+118`).
    pub targeted: bool,
    /// Defenders holding it as an ability's target (`567140`, the same `+118`).
    pub defended_by: Vec<ActorId>,
    /// The thing's own DefenderComponent (`+B0`).
    pub defender: defender::DefenderState,
    /// Performer `+60` (`57E430` at its creation): Body `+14` and Mover `+28` with
    /// the technologies its owner had then.
    pub performer_value: f32,
    /// Who shells this actor (`+130`).
    pub attacked_by: Vec<AttackedBy>,
    /// What this actor shells (`+128`, `4D3A70`), head first.
    pub bombarding: Vec<TargetRef>,
    /// ROE primary damage history (`570F70`): (time, amount).
    pub history: Vec<(f32, f32)>,
    /// ROE back-off: no attack before this time (`5340C0` retry time).
    pub roe_retry_at: f32,
    /// Performer turret states (`+4C`, one per `Def::turrets`).
    pub turrets: Vec<TurretState>,
    /// The regiment target last handed to this member (UAI engage state
    /// `+10`, written by `532070`).
    pub engage_target: Option<TargetRef>,
    // ---- appended by the core owner (HUD orders, `tasks`) ----
    /// The task of an actor outside an organization.
    pub task: Option<tasks::Task>,
    pub task_order: Option<Order>,
    pub task_think: f32,
    /// What this actor uses its task ability on (bombard point or actor,
    /// repair target), set by its own or its organization's task Think.
    pub aim: Option<tasks::Aim>,
}

/// One turret of a Performer (`57F4D0` allocates 0x1C bytes per turret).
#[derive(Clone, Debug, Default)]
pub struct TurretState {
    /// `+0` yaw in the actor's frame (radians; at rest `yaw_start - pi/2`,
    /// so `yaw + pi/2` is the turn from the hull's forward, counter-clockwise).
    pub yaw: f32,
    /// `+4` the yaw before this tick's step (the view interpolates between).
    pub prev: f32,
    /// `+8` when the turret last aimed (the reset timer).
    pub last_used: f32,
    /// The Performer's per-tick turret update is on (`+5C`, `57EF00`).
    pub active: bool,
}

/// A projectile in flight (`Weapon::projectile_speed`).
#[derive(Clone, Debug)]
pub struct Projectile {
    pub shooter: ActorId,
    pub weapon: usize,
    /// None: fired at a ground point (a bombard).
    pub target: Option<ActorId>,
    /// Target position at launch (impact point when the target is gone).
    pub point: Vec2,
    pub impact_at: f32,
    /// Its native id (`crate::creation::projectile`), back at impact.
    pub thing: Option<crate::ids::Gtid>,
}

#[derive(Clone, Debug, Default)]
struct Grid {
    origin: Vec2,
    w: i32,
    h: i32,
    cells: Vec<Vec<ActorId>>,
}
impl Grid {
    fn build(&mut self, world: &World) {
        for c in &mut self.cells {
            c.clear();
        }
        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        for a in world.actors.iter().filter(|a| indexed(a)) {
            let p = a.pos.truncate();
            min = min.min(p);
            max = max.max(p);
        }
        if min.x > max.x {
            self.w = 0;
            self.h = 0;
            return;
        }
        self.origin = min;
        self.w = (((max.x - min.x) / CELL) as i32 + 1).clamp(1, MAX_CELLS_PER_AXIS);
        self.h = (((max.y - min.y) / CELL) as i32 + 1).clamp(1, MAX_CELLS_PER_AXIS);
        let n = (self.w * self.h) as usize;
        if self.cells.len() < n {
            self.cells.resize_with(n, Vec::new);
        }
        for a in world.actors.iter().filter(|a| indexed(a)) {
            let i = self.index(a.pos.truncate());
            self.cells[i].push(a.id);
        }
    }
    fn coord(&self, p: Vec2) -> (i32, i32) {
        (
            (((p.x - self.origin.x) / CELL).floor() as i32).clamp(0, self.w - 1),
            (((p.y - self.origin.y) / CELL).floor() as i32).clamp(0, self.h - 1),
        )
    }
    fn index(&self, p: Vec2) -> usize {
        let (x, y) = self.coord(p);
        (y * self.w + x) as usize
    }
    /// Every indexed actor in the cells overlapping the circle (unfiltered).
    fn query(&self, center: Vec2, radius: f32, out: &mut Vec<ActorId>) {
        out.clear();
        if self.w == 0 || radius.is_nan() || radius < 0.0 {
            return;
        }
        let (x0, y0) = self.coord(center - Vec2::splat(radius));
        let (x1, y1) = self.coord(center + Vec2::splat(radius));
        for y in y0..=y1 {
            for x in x0..=x1 {
                out.extend_from_slice(&self.cells[(y * self.w + x) as usize]);
            }
        }
    }
}

fn indexed(a: &crate::world::Actor) -> bool {
    a.alive && !a.removed && a.kingdom.is_some()
}

/// Combat and morale state of the battle (`World::combat`).
#[derive(Clone, Debug, Default)]
pub struct CombatState {
    pub rules: Option<Arc<Rules>>,
    pub extras: Option<Arc<Extras>>,
    data_key: usize,
    pub orgs: Vec<OrgCombat>,
    pub actors: Vec<ActorCombat>,
    grid: Grid,
    /// Team of each kingdom id.
    teams: Vec<Option<u32>>,
    scratch: Vec<ActorId>,
    deaths: Vec<ActorId>,
    pub projectiles: Vec<Projectile>,
    /// (core) Kingdom relations (`World::books.relations`, native values
    /// 0 self, 1 friend, 2 neutral, 3 enemy) and independent kingdoms.
    relations: Vec<Vec<u8>>,
    independent: Vec<bool>,
}

impl CombatState {
    pub fn rules(&self) -> &Rules {
        self.rules.as_deref().expect("combat state prepared")
    }
    pub fn extras(&self) -> &Extras {
        self.extras.as_deref().expect("combat state prepared")
    }
    pub fn org(&self, id: OrgId) -> Option<&OrgCombat> {
        self.orgs.get(id as usize)
    }
    pub fn actor(&self, id: ActorId) -> Option<&ActorCombat> {
        self.actors.get(id as usize)
    }
    fn team(&self, kingdom: Option<KingdomId>) -> Option<u32> {
        kingdom.and_then(|k| self.teams.get(k as usize).copied().flatten())
    }
    /// (core) 4E52D0 relation of kingdom `a` to kingdom `b`; without a
    /// relation table the team rule (same team friend, else enemy).
    fn relation(&self, a: Option<KingdomId>, b: Option<KingdomId>) -> Option<u8> {
        let (ka, kb) = (a?, b?);
        if ka == kb {
            return Some(crate::books::SELF);
        }
        if let Some(v) = self.relations.get(ka as usize).and_then(|r| r.get(kb as usize)) {
            return Some(*v);
        }
        let (x, y) = (self.team(a)?, self.team(b)?);
        Some(if x == y { crate::books::FRIEND } else { crate::books::ENEMY })
    }
    /// Hostile = the relation is enemy (4E52A0); independents count as the
    /// map's saved relation makes them (cities are enemies, some are neutral).
    fn hostile(&self, a: Option<KingdomId>, b: Option<KingdomId>) -> bool {
        self.relation(a, b) == Some(crate::books::ENEMY)
    }

    /// The per-definition tables, once per `GameData`.
    pub(crate) fn definition_tables(&mut self, world: &World) {
        let key = Arc::as_ptr(&world.data) as usize;
        if self.rules.is_none() || self.data_key != key {
            let rules = Rules::from_data(&world.data);
            let extras = Extras::from_data(&world.data, &rules);
            self.rules = Some(Arc::new(rules));
            self.extras = Some(Arc::new(extras));
            self.data_key = key;
        }
    }

    /// Bring the per-battle tables up to date (data change, new actors and
    /// organizations, kingdom teams, organization positions and engaged-by).
    pub(crate) fn prepare(&mut self, world: &mut World) {
        self.definition_tables(world);
        self.teams.clear();
        for k in &world.kingdoms {
            let i = k.id as usize;
            if self.teams.len() <= i {
                self.teams.resize(i + 1, None);
            }
            self.teams[i] = Some(k.team);
        }
        self.relations.clone_from(&world.books.relations);
        self.independent = world.kingdoms.iter().map(|k| k.independent).collect();
        let now = world.time;
        if self.actors.len() < world.actors.len() {
            self.actors.resize_with(world.actors.len(), ActorCombat::default);
        }
        for a in &world.actors {
            let s = &mut self.actors[a.id as usize];
            if !s.initialized {
                s.initialized = true;
                s.max_health = crate::production::max_health(world, a.kingdom, a.def);
                s.next_think = now + (a.id % 16) as f32 * DT;
                let def = world.data.def(a.def);
                // Mover +0x28 as 576C70 sets it at creation: the definition's speed
                // through the technologies (stance and formation come later).
                let factor = a.kingdom.map_or(1.0, |k| crate::production::multiply_factor(world, k, &def.properties, "SPEED_MAX"));
                let extras = self.extras.as_deref().expect("definition tables");
                s.performer_value = performer_value_of(def, s.max_health, def.speed_max * factor, &extras.means, self.rules.as_deref().expect("definition tables"));
                // 57F4D0: every turret starts at yaw_start - pi/2 (wrapped once).
                s.turrets = world.data.def(a.def).turrets.iter().map(|t| {
                    let yaw = wrap_once(t.yaw_start - FRAC_PI_2);
                    TurretState { yaw, prev: yaw, last_used: 0.0, active: false }
                }).collect();
            }
            s.targeted = false;
            s.defended_by.clear();
        }
        for a in &world.actors {
            if a.alive
                && !a.removed
                && let Some(t) = a.combat.target
                && let Some(s) = self.actors.get_mut(t as usize)
            {
                s.targeted = true;
            }
        }
        if self.orgs.len() < world.orgs.len() {
            self.orgs.resize_with(world.orgs.len(), OrgCombat::default);
        }
        let mut fresh = Vec::new();
        for o in &world.orgs {
            let s = &mut self.orgs[o.id as usize];
            s.engaged_by.clear();
            s.defended_by.clear();
            if !s.initialized {
                s.initialized = true;
                s.attack_efficiency = 1.0;
                s.attack_efficiency_desired = 1.0;
                s.next_think = now + (o.id % 16) as f32 * DT;
                s.next_sleep_think = s.next_think;
                s.previous_order = o.order.clone();
                s.slots = world.data.def(o.def).slots.len() as i32;
                if s.slots == 0 {
                    s.slots = o.members.len() as i32;
                }
                s.living = o.members.iter().filter(|&&m| world.actors.get(m as usize).is_some_and(|a| a.alive)).count() as i32;
                fresh.push(o.id);
            }
        }
        for id in fresh {
            crate::morale::init_org(world, self, id);
        }
        for o in &world.orgs {
            let leader = o.members.iter().find_map(|&m| world.actors.get(m as usize).filter(|a| a.alive && !a.removed));
            let s = &mut self.orgs[o.id as usize];
            s.position = leader.map_or(o.anchor, |a| a.pos.truncate());
            // +64: 57CBA0 and 57CD70 count the members of the layout's morale_line
            // slots (a definition without a layout: every member).
            let line = &world.data.def(o.def).slot_morale_line;
            let counts = |m: ActorId| {
                let a = world.actors.get(m as usize).filter(|a| a.alive && !a.removed);
                a.is_some_and(|a| line.is_empty() || a.slot.is_some_and(|i| line.get(i as usize).copied().unwrap_or(false)))
            };
            s.living = o.members.iter().filter(|&&m| counts(m)).count() as i32;
        }
        for i in 0..self.orgs.len() {
            if !world.orgs.get(i).is_some_and(|o| o.alive) {
                continue;
            }
            for t in 0..self.orgs[i].engage.len() {
                if let TargetRef::Org(b) = self.orgs[i].engage[t].target
                    && let Some(s) = self.orgs.get_mut(b as usize)
                    && !s.engaged_by.contains(&(i as OrgId))
                {
                    s.engaged_by.push(i as OrgId);
                }
            }
        }
        // 567140 -> 4D3990: a Defender's ability on the target's +118.
        for a in 0..self.actors.len() {
            if self.actors[a].defender.targets.iter().all(|t| t.1.is_none()) {
                continue;
            }
            for t in self.actors[a].defender.engaged_with(world) {
                match t {
                    TargetRef::Org(o) => {
                        if let Some(s) = self.orgs.get_mut(o as usize) {
                            s.defended_by.push(a as ActorId);
                        }
                    }
                    TargetRef::Actor(x) => {
                        if let Some(s) = self.actors.get_mut(x as usize) {
                            s.defended_by.push(a as ActorId);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Queries shared with morale.rs

pub(crate) fn alive(world: &World, id: ActorId) -> bool {
    world.actors.get(id as usize).is_some_and(|a| a.alive && !a.removed)
}

/// The organization (if attached) or the actor itself (`524C10` element ->
/// organization).
pub(crate) fn target_ref(world: &World, id: ActorId) -> TargetRef {
    match world.actors.get(id as usize).and_then(|a| a.org) {
        Some(o) if world.orgs.get(o as usize).is_some_and(|x| x.alive) => TargetRef::Org(o),
        _ => TargetRef::Actor(id),
    }
}

pub(crate) fn target_alive(world: &World, t: TargetRef) -> bool {
    match t {
        TargetRef::Org(o) => world.orgs.get(o as usize).is_some_and(|x| x.alive && x.members.iter().any(|&m| alive(world, m))),
        TargetRef::Actor(a) => alive(world, a),
    }
}

/// A live member (the leader first) standing for a target organization.
/// (orders) `representative` for reports outside the crate.
pub fn representative_public(world: &World, t: TargetRef) -> Option<ActorId> {
    representative(world, t)
}

pub(crate) fn representative(world: &World, t: TargetRef) -> Option<ActorId> {
    match t {
        TargetRef::Org(o) => world.orgs.get(o as usize)?.members.iter().copied().find(|&m| alive(world, m)),
        TargetRef::Actor(a) => alive(world, a).then_some(a),
    }
}

fn target_kingdom(world: &World, t: TargetRef) -> Option<KingdomId> {
    match t {
        TargetRef::Org(o) => world.orgs.get(o as usize).map(|x| x.kingdom),
        TargetRef::Actor(a) => world.actors.get(a as usize).and_then(|x| x.kingdom),
    }
}

/// Can be targeted at all: alive, owned, with a body, not scenery/city.
fn targetable(world: &World, id: ActorId) -> bool {
    let Some(a) = world.actors.get(id as usize) else { return false };
    if !a.alive || a.removed || a.kingdom.is_none() || a.health <= 0.0 {
        return false;
    }
    let def = world.data.def(a.def);
    if def.health <= 0.0 || def.invulnerable {
        return false;
    }
    match def.class {
        Some(DefClass::Organization) => false,
        // scenery stays out; a targetable Thing (a supply drop) does not
        Some(DefClass::Other) => def.targetable,
        _ => true,
    }
}

/// A shooter's current target is still valid: targetable, hostile, not dying.
fn valid_target(world: &World, st: &CombatState, shooter: ActorId, target: ActorId) -> bool {
    targetable(world, target)
        && !st.actors.get(target as usize).is_some_and(|a| a.queued_death)
        && st.hostile(world.actors[shooter as usize].kingdom, world.actors[target as usize].kingdom)
}

/// `4D03F0`: no organization component and no usable ability.
fn harmless(world: &World, id: ActorId) -> bool {
    world.actors.get(id as usize).is_none_or(|a| world.data.def(a.def).weapons.is_empty())
}

/// `4C5840` target flags of `target` seen from `source` and the `4C5A20` filter.
fn admits(world: &World, st: &CombatState, source: ActorId, w: &Weapon, target: ActorId) -> bool {
    if w.targets_required == 0 && w.targets_forbidden == 0 {
        return true;
    }
    let ex = st.extras();
    let (Some(s), Some(t)) = (world.actors.get(source as usize), world.actors.get(target as usize)) else {
        return false;
    };
    let mut flags = world.data.def(t.def).target_flags;
    let set = |flags: &mut u32, bit: Option<u32>| {
        if let Some(b) = bit {
            *flags |= 1 << b;
        }
    };
    // (core) 4C5840: friend / enemy / neutral from the kingdom relation,
    // independent when the target has no owner or an independent owner.
    match st.relation(s.kingdom, t.kingdom) {
        Some(crate::books::SELF | crate::books::FRIEND) => set(&mut flags, ex.bit_friend),
        Some(crate::books::ENEMY) => set(&mut flags, ex.bit_enemy),
        Some(_) => set(&mut flags, ex.bit_neutral),
        None => {}
    }
    if t.kingdom.is_none_or(|k| st.independent.get(k as usize).copied().unwrap_or(false)) {
        set(&mut flags, ex.bit_independent);
    }
    if s.kingdom.is_some() && s.kingdom == t.kingdom {
        set(&mut flags, ex.bit_own);
    }
    if source == target {
        set(&mut flags, ex.bit_self);
    }
    if s.org.is_some() && s.org == t.org {
        set(&mut flags, ex.bit_same_organization);
    }
    set(&mut flags, ex.bit_body);
    set(&mut flags, ex.bit_actor);
    if world.books.actor(target).is_some_and(|b| b.constructing) {
        set(&mut flags, ex.bit_under_construction);
    }
    if let Some(o) = t.org
        && st.orgs.get(o as usize).is_some_and(|x| x.routing.is_some())
    {
        set(&mut flags, ex.bit_routing);
    }
    flags & w.targets_required == w.targets_required && flags & w.targets_forbidden == 0
}

/// Level of the organization an actor belongs to.
fn level_of<'a>(world: &World, st: &'a CombatState, actor: &crate::world::Actor) -> Option<&'a ExperienceLevel> {
    let o = actor.org?;
    let org = world.orgs.get(o as usize)?;
    let s = st.orgs.get(o as usize)?;
    if s.level == 0 {
        return None;
    }
    st.extras().def(org.def).levels.get(s.level)
}

/// RESIST_DAMAGE factor of a damage type against a target, with entrenchment.
pub(crate) fn type_factor(world: &World, st: &CombatState, damage_type: u32, target: &crate::world::Actor) -> f32 {
    let def = world.data.def(target.def);
    let mut factor = def
        .damage_factor
        .get(damage_type as usize)
        .copied()
        .unwrap_or_else(|| world.data.damage_factor(damage_type, target.def));
    if let Some(o) = target.org
        && st.orgs.get(o as usize).is_some_and(|x| x.entrenched)
    {
        factor -= st.extras().def(target.def).entrench_resist.get(damage_type as usize).copied().unwrap_or(0.0);
    }
    if let Some(m) = crate::special_ops::mods(world, target.id) {
        factor -= m.resist.get(damage_type as usize).copied().unwrap_or(0.0);
    }
    // 501650 / 502440: the terrain's RESIST_DAMAGE and the technologies' too.
    let resist = |m: &crate::data::Modifier| m.kind.eq_ignore_ascii_case("RESIST_DAMAGE") && m.damage_type == Some(damage_type);
    for p in terrain_properties(world, target) {
        factor -= p.modifiers.iter().filter(|m| resist(m)).map(|m| m.amount).sum::<f32>();
    }
    if let Some(k) = target.kingdom {
        factor -= crate::production::technology_modifiers(world, k, &def.properties).filter(|m| resist(m)).map(|m| m.amount).sum::<f32>();
    }
    factor.max(st.rules().clamp_multiply)
}

fn target_defense(world: &World, st: &CombatState, target: &crate::world::Actor) -> f32 {
    let def = world.data.def(target.def);
    let mut dv = level_of(world, st, target).map_or(0.0, |l| l.dv);
    if let Some(o) = target.org
        && st.orgs.get(o as usize).is_some_and(|x| x.entrenched)
    {
        dv += st.extras().def(target.def).entrench_dv;
    }
    if let Some(m) = crate::special_ops::mods(world, target.id) {
        dv += m.dv;
    }
    // 501B10 is the thing's whole modifier state: its properties, its owner's
    // technologies (4E3AD0 -> 4CEF10) and the terrain under it (4D4130).
    dv += crate::zones::modifier_add(world, crate::zones::Key::Actor(target.id), "DV").unwrap_or(0.0);
    for p in terrain_properties(world, target) {
        dv += p.modifiers.iter().filter(|m| add_of(m, "DV")).map(|m| m.amount).sum::<f32>();
    }
    modified_defense(def.defense, dv, st.rules().clamp_add)
}

fn add_of(m: &crate::data::Modifier, attribute: &str) -> bool {
    m.kind.eq_ignore_ascii_case("ADD") && m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(attribute))
}

/// `4D4130`: the properties of the terrain under a thing that may take them (gates:
/// a Mover, not attached to a carrier), each applying when the thing has its
/// required_properties (`4CEF10`). The original keeps them from tile change to tile
/// change on the thing; this reads the tile at the time of asking.
fn terrain_properties<'a>(world: &'a World, a: &'a crate::world::Actor) -> impl Iterator<Item = &'a crate::data::PropertyDef> {
    let def = world.data.def(a.def);
    let terrain = (def.speed_max > 0.0)
        .then(|| world.map.cell_of(a.pos.truncate()))
        .flatten()
        .and_then(|(x, y)| world.map.cell_variation.get((y * world.map.width + x) as usize))
        .and_then(|&v| world.map.variations.get(v as usize))
        .and_then(|id| world.data.terrain.find(id));
    terrain
        .into_iter()
        .flat_map(|t| t.properties.iter())
        .filter_map(|p| world.data.property(p))
        .filter(move |p| p.required_properties.iter().all(|r| def.properties.iter().any(|q| q.eq_ignore_ascii_case(r))))
}

fn shooter_power(world: &World, st: &CombatState, shooter: &crate::world::Actor, w: &Weapon) -> f32 {
    let (mut av, mut kind) = level_of(world, st, shooter).map_or((0.0, 0.0), |l| {
        (l.av, if w.projectile.is_some() { l.av_ranged } else { l.av_melee })
    });
    if let Some(m) = crate::special_ops::mods(world, shooter.id) {
        av += m.av;
        kind += if w.projectile.is_some() { m.av_ranged } else { m.av_melee };
    }
    // 501A30 / 501AA0 read the thing's whole modifier state (see `target_defense`).
    let key = crate::zones::Key::Actor(shooter.id);
    av += crate::zones::modifier_add(world, key, "AV").unwrap_or(0.0);
    kind += crate::zones::modifier_add(world, key, if w.projectile.is_some() { "AV_RANGED" } else { "AV_MELEE" }).unwrap_or(0.0);
    modified_power(w.damage, kind, av, st.rules().clamp_add)
}

/// `57DBF0`: the organization's actual attack efficiency, else 1.
fn shooter_offense(st: &CombatState, shooter: &crate::world::Actor) -> f32 {
    shooter.org.and_then(|o| st.orgs.get(o as usize)).map_or(1.0, |o| o.attack_efficiency)
}

// ---------------------------------------------------------------------------
// Decisions (5340C0 / 534700 / 52F010)

/// Decision status per weapon: 0 ready, 1 waiting for recharge, 2 pursue
/// (out of range), 3 unusable (target filter or minimum range: "actors will
/// not back up to get into range").
/// The shape of a thing for the range tests (actor virtual `+14`: ObjectData
/// `+50`) with its transform.
fn range_placement(world: &World, a: ActorId) -> aa_spatial::geometry::Placement {
    let x = &world.actors[a as usize];
    let w = world.data.def(x.def).soft_shape;
    let dir = Vec2::from_angle(x.facing);
    aa_spatial::geometry::Placement {
        shape: aa_spatial::collision::Shape { kind: w[0], radius_bits: w[1], bounds: [w[2], w[3], w[4], w[5]] },
        facing: [dir.x.to_bits(), dir.y.to_bits()],
        position: [x.pos.x.to_bits(), x.pos.y.to_bits()],
    }
}

/// `4EE800` (aa_spatial::range::actors_in_range) at PC53: the shapes of `source`
/// and `target` within `range`; `inverted` is the minimum-range mode. Two points
/// or circles take `46EB70` in the x87 helpers, for the cost of a shot's every
/// update; a rectangle goes through the port (`46E2D0`).
pub(crate) fn in_range(world: &World, source: ActorId, target: ActorId, range: f32, inverted: bool) -> bool {
    let (a, b) = (range_placement(world, target), range_placement(world, source));
    if a.shape.kind > 1 || b.shape.kind > 1 {
        return aa_spatial::range::shapes_in_range(&b, &a, range.to_bits(), inverted, aa_numeric::Precision::Double53).unwrap_or(false);
    }
    // 46F100: the higher kind, then the larger radius, is the primary.
    let (ra, rb) = (f32::from_bits(a.shape.radius_bits), f32::from_bits(b.shape.radius_bits));
    let swap = b.shape.kind < a.shape.kind || (a.shape.kind == b.shape.kind && rb < ra);
    let (primary, other, rp, ro) = if swap { (&a, &b, ra, rb) } else { (&b, &a, rb, ra) };
    let v = |bits: u32| ld(f32::from_bits(bits));
    let dy = fstp(x87::sub(v(other.position[1]), v(primary.position[1])));
    let dx = fstp(x87::sub(v(other.position[0]), v(primary.position[0])));
    let distance = x87::add(x87::mul(ld(dy), ld(dy)), x87::mul(ld(dx), ld(dx)));
    let radius = match primary.shape.kind {
        0 => ld(range),
        _ if other.shape.kind == 0 => x87::add(ld(range), ld(rp)),
        _ => x87::add(x87::add(ld(ro), ld(rp)), ld(range)),
    };
    let threshold = x87::mul(radius, radius);
    if inverted { threshold <= distance } else { threshold >= distance }
}

/// [`status_on`] at a distance already known (a repair's reach, `tasks`).
fn weapon_status(w: &Weapon, recharge_left: f32, distance: f32) -> u8 {
    if w.min_range > 0.0 && distance < w.min_range {
        3
    } else if distance > w.range {
        2
    } else if recharge_left > 0.0 {
        1
    } else {
        0
    }
}

/// `530E50`'s value from the draw (aa_mind::construct): `u * 2 - 1`, written to
/// every Performer ability's `+1C` (the element Mind's `+1B8`).
pub(crate) fn mind_jitter(word: u32) -> f32 {
    let mut number = f64::from(word as i32);
    if word & 0x8000_0000 != 0 {
        number = x87::add(number, f64::from_bits(0x41f0_0000_0000_0000));
    }
    let unit = x87::mul(number, f64::from_bits(0x3df0_0000_0010_0000));
    fstp(x87::sub(x87::add(unit, unit), 1.0))
}

/// `4C8270` stored: the ability's range through its modifiers (`4BE910`: a
/// projectile's PROJECTILE_ABILITY_RANGE `501D50`, flag bit 3 BOMBARD_ABILITY_RANGE
/// `501DB0`, each floored as `501C30`) plus its local `+1C` fuzzy_range times the
/// Mind's jitter (ability `+1C`).
pub(crate) fn effective_range(world: &World, a: ActorId, w: &Weapon) -> f32 {
    let clamp = world.data.rule_ci("ModifierAttributeClampAdd").unwrap_or(1.0);
    let mut range = w.range;
    if w.projectile.is_some()
        && let Some(add) = crate::zones::modifier_add(world, crate::zones::Key::Actor(a), "PROJECTILE_ABILITY_RANGE")
    {
        range = crate::zones::clamp_add(range, add, clamp);
    }
    if w.flags & 8 != 0
        && let Some(add) = crate::zones::modifier_add(world, crate::zones::Key::Actor(a), "BOMBARD_ABILITY_RANGE")
    {
        range = crate::zones::clamp_add(range, add, clamp);
    }
    let jitter = world.books.actor(a).map_or(0.0, |b| b.ability_jitter);
    fstp(x87::add(x87::mul(ld(w.fuzzy_range), ld(jitter)), ld(range)))
}

/// What `4C8F20` answers for the distance and the recharge, in `5340C0`'s terms:
/// 3 inside the minimum range, 2 out of range, 1 recharging, 0 ready.
fn status_on(world: &World, shooter: ActorId, target: ActorId, w: &Weapon, recharge_left: f32) -> u8 {
    if w.min_range > 0.0 && !in_range(world, shooter, target, w.min_range, true) {
        3
    } else if !in_range(world, shooter, target, effective_range(world, shooter, w), false) {
        2
    } else if recharge_left > 0.0 {
        1
    } else {
        0
    }
}

/// `4C8F20(context, 1)` for ability `weapon` of `source` on `target`, phase 0:
/// 1 not eligible (`4C8740`), 2 inside the minimum range, 3 outside the turret's
/// arc, 5 out of range, 7 no mana, 8 recharging, 0 usable. The payment (4) and
/// the summon limits (9, 10) have no source on a weapon.
pub(crate) fn admission(world: &World, st: &CombatState, source: ActorId, weapon: usize, target: ActorId) -> u32 {
    let def = world.data.def(world.actors[source as usize].def);
    let Some(w) = def.weapons.get(weapon) else { return 1 };
    let Some(t) = world.actors.get(target as usize).filter(|t| t.alive && !t.removed) else { return 1 };
    if !world.data.def(t.def).targetable || !admits(world, st, source, w, target) {
        return 1;
    }
    if w.min_range > 0.0 && !in_range(world, source, target, w.min_range, true) {
        return 2;
    }
    if !in_turret_arc(world, source, w, t.pos.truncate()) {
        return 3;
    }
    if !in_range(world, source, target, effective_range(world, source, w), false) {
        return 5;
    }
    if world.actors[source as usize].combat.recharge.get(weapon).is_some_and(|&r| r > 0.0) {
        return 8;
    }
    0
}

/// `4C84B0`: the ability's turret, if it turns, can bear on `point`.
pub(crate) fn in_turret_arc(world: &World, source: ActorId, w: &Weapon, point: Vec2) -> bool {
    let x = &world.actors[source as usize];
    let Some(t) = usize::try_from(w.turret).ok().and_then(|i| world.data.def(x.def).turrets.get(i)) else { return true };
    !turret_can_turn(t) || turret_in_arc(t, x.pos.truncate(), x.facing, point)
}

/// `4C81D0` then `57E730`: a turret that turns is on `target`, one step a call.
pub(crate) fn turret_ready(world: &World, st: &mut CombatState, id: usize, weapon: usize, target: ActorId) -> bool {
    let data = world.data.clone();
    let def = data.def(world.actors[id].def);
    let Some(w) = def.weapons.get(weapon) else { return false };
    let Some((i, td)) = usize::try_from(w.turret).ok().and_then(|i| def.turrets.get(i).map(|t| (i, t))) else { return true };
    if !turret_can_turn(td) {
        return true;
    }
    let point = world.actors[target as usize].pos.truncate();
    turret_in_arc(td, world.actors[id].pos.truncate(), world.actors[id].facing, point) && turret_aim(world, st, id, i, td, point)
}

/// `4D1120`: can be taken by damage: not under construction, not a block that
/// goes with a city (Structure `+34`), not a city, and the Body says captureable.
pub(crate) fn capturable(world: &World, a: ActorId) -> bool {
    let Some(x) = world.actors.get(a as usize) else { return false };
    let d = world.data.def(x.def);
    let b = world.books.actor(a);
    d.captureable && !d.city && !b.is_some_and(|b| b.constructing || b.block_of.is_some())
}

/// The ability's use (`57E030` -> `4C9E90` -> `4C9550`): the recharge, the event,
/// and a projectile or the hit.
pub(crate) fn use_weapon(world: &mut World, st: &mut CombatState, id: ActorId, weapon: usize, target: ActorId) {
    let now = world.time;
    let data = world.data.clone();
    let i = id as usize;
    let w = &data.def(world.actors[i].def).weapons[weapon];
    // 501560: the recharge divided by the RECHARGE_TIME multiplier.
    let multiplier = crate::special_ops::mods(world, id).map_or(1.0, |m| (1.0 + m.recharge).max(st.rules().clamp_multiply));
    if let Some(r) = world.actors[i].combat.recharge.get_mut(weapon) {
        *r = w.recharge / multiplier;
    }
    world.events.push(Event::Fired { shooter: id, target, weapon: weapon as u32 });
    let point = world.actors[target as usize].pos.truncate();
    // 0x4C9550 -> 0x4C8CB0: the projectile is a thing, with its id and its draws.
    let thing = w.projectile.as_deref().and_then(|p| data.find(p)).and_then(|p| crate::creation::projectile(world, p));
    if w.projectile_speed > 0.0 {
        let flight = (point - world.actors[i].pos.truncate()).length() / w.projectile_speed;
        st.projectiles.push(Projectile { shooter: id, weapon, target: Some(target), point, impact_at: now + flight, thing });
    } else {
        if let Some(t) = thing {
            world.ids.removed_transient(t);
        }
        deliver(world, st, id, weapon, Some(target), point);
    }
}

#[derive(Clone, Copy, Debug)]
struct Decision {
    weapon: usize,
    status: u8,
    score: f32,
}

/// `5340C0`: the best ability of `shooter` against `target` (the ROE
/// back-off is checked by the callers through [`roe_backoff`]). Each score
/// (`52ED30`, stored) gets the attacker bonus (`4D4420`,
/// AbilityScoreAttackerBonus) and is stored again; the higher score wins.
/// Equal scores: in pursuit (status 2) the longer effective range
/// (`4C8270`) wins, otherwise the higher ability priority (`4C81C0`, local
/// `+20`), then the shorter effective range, then one world-RNG draw
/// (`85D840`, bit 0 takes the newer ability).
fn decide(world: &mut World, st: &CombatState, shooter: ActorId, target: ActorId) -> Option<Decision> {
    let data = world.data.clone();
    let def = data.def(world.actors[shooter as usize].def);
    let bonus = if attacks(world, target, shooter) { st.rules().attacker_bonus } else { 0.0 };
    let mut best: Option<Decision> = None;
    for (i, w) in def.weapons.iter().enumerate() {
        // The Performer's own list: the Defender's abilities act through its Tick.
        if w.defender || !admits(world, st, shooter, w, target) {
            continue;
        }
        let left = world.actors[shooter as usize].combat.recharge.get(i).copied().unwrap_or(0.0);
        let status = status_on(world, shooter, target, w, left);
        if status == 3 {
            continue;
        }
        // 52ED30: factor*priority + bonus + the performer bonus, stored; a
        // pursuit takes the wait weights while the ability is recharging
        // (4C8A40), the move weights otherwise, stored, then always adds the
        // performer move bonus (stored again).
        let weighted = |factor: f32, bonus: f32| x87::add(x87::mul(ld(factor), ld(w.priority)), ld(bonus));
        let mut score = match status {
            0 => fstp(x87::add(weighted(w.use_weight, w.use_bonus), ld(def.performer_use_bonus))),
            1 => fstp(x87::add(weighted(w.wait_weight, w.wait_bonus), ld(def.performer_wait_bonus))),
            _ => {
                let s = if left > 0.0 { fstp(weighted(w.wait_weight, w.wait_bonus)) } else { fstp(weighted(w.move_weight, w.move_bonus)) };
                fstp(x87::add(ld(def.performer_move_bonus), ld(s)))
            }
        };
        if w.damage > 0.0 {
            // 52ED30 player weight: the 501650 damage-type factor of the
            // target times the stored utility, stored by 5340C0.
            score = fstp(x87::mul(ld(type_factor(world, st, w.damage_type, &world.actors[target as usize])), ld(score)));
        }
        score = fstp(x87::add(ld(score), ld(bonus)));
        let take = match best {
            None => true,
            Some(b) if score > b.score => true,
            Some(b) if score < b.score => false,
            Some(b) => {
                let held = &def.weapons[b.weapon];
                if status == 2 {
                    held.range < w.range
                } else if held.priority != w.priority {
                    held.priority < w.priority
                } else if held.range != w.range {
                    held.range > w.range
                } else {
                    world.random_u32_at(crate::trace::sites::COMBAT_COIN) & 1 != 0
                }
            }
        };
        if take {
            best = Some(Decision { weapon: i, status, score });
        }
    }
    best
}

/// The organization's primary element (the layout's primary slot).
fn is_primary(world: &World, actor: ActorId) -> bool {
    let a = &world.actors[actor as usize];
    a.org.is_some_and(|o| world.orgs.get(o as usize).is_some_and(|org| world.data.def(org.def).leader_slot.is_some() && world.data.def(org.def).leader_slot == a.slot))
}

/// `5340C0` rules of engagement for the primary of a regiment of several
/// members, away from the anchor and targeted: health ratio below
/// primary_minimum_health, or time to death (health over the damage rate of
/// the last ROEPrimaryDamageHistoryLength seconds) below
/// primary_time_to_death, backs it off for ROEPrimaryEngageReturnTimeout.
fn roe_backoff(world: &World, st: &mut CombatState, actor: ActorId) -> bool {
    let now = world.time;
    let i = actor as usize;
    if now < st.actors[i].roe_retry_at {
        return true;
    }
    if !is_primary(world, actor) {
        return false;
    }
    let a = &world.actors[i];
    let Some(o) = a.org else { return false };
    let org = &world.orgs[o as usize];
    let def = world.data.def(a.def);
    if (def.primary_minimum_health <= 0.0 && def.primary_time_to_death <= 0.0)
        || org.members.iter().filter(|&&m| alive(world, m)).count() < 2
        || a.pos.truncate().distance_squared(org.anchor) < 0.25
        || !st.actors[i].targeted
    {
        return false;
    }
    let r = st.rules();
    let max = st.actors[i].max_health.max(EPSILON);
    let mut back = x87::div(ld(a.health), ld(max)) < ld(def.primary_minimum_health);
    if !back && def.primary_time_to_death > 0.0 {
        // 570E30: records not older than fstp(now - length) (570A70), the
        // in-order unstored sum over the length, stored by 5340C0; 5617B0
        // is health / rate.
        let length = r.roe_history_length.max(EPSILON);
        let cutoff = fstp(x87::sub(ld(now), ld(length)));
        let sum = st.actors[i].history.iter().filter(|(t, _)| *t >= cutoff).fold(0.0, |s, (_, d)| x87::add(s, ld(*d)));
        let rate = fstp(x87::div(sum, ld(length)));
        back = rate > 0.0 && x87::div(ld(a.health), ld(rate)) < ld(def.primary_time_to_death);
    }
    if back {
        let timeout = r.roe_return_timeout;
        st.actors[i].roe_retry_at = now + timeout;
    }
    back
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    actor: ActorId,
    priority: i32,
    score: f32,
    distance2: f32,
}

/// `52F010`: true when `c` beats `best` (priority, then score beyond 1e-4,
/// then the nearer target; squared distances within 1e-4 are equal,
/// `455000` with `[A02790]`).
fn better(best: &Option<Candidate>, c: &Candidate) -> bool {
    let Some(b) = best else { return true };
    if b.priority != c.priority {
        return c.priority > b.priority;
    }
    if !((b.score - c.score).abs() < EPSILON) {
        return b.score <= c.score;
    }
    if (b.distance2 - c.distance2).abs() < EPSILON {
        return false;
    }
    !(b.distance2 < c.distance2)
}

/// `4D4420`: `actor` (or its organization) is attacking `us` (or our organization).
fn attacks(world: &World, actor: ActorId, us: ActorId) -> bool {
    let Some(a) = world.actors.get(actor as usize) else { return false };
    let Some(t) = a.combat.target else { return false };
    let our_org = world.actors[us as usize].org;
    t == us || (our_org.is_some() && world.actors.get(t as usize).and_then(|x| x.org) == our_org)
}

/// The organization an actor is attached to (membership `+80` with its
/// `+10` organization and the attached byte `+1C`).
fn attached_org(world: &World, actor: &crate::world::Actor) -> Option<OrgId> {
    actor.org.filter(|&o| world.orgs.get(o as usize).is_some_and(|x| x.alive))
}

/// The target's Mind virtual `+58`: an attached element answers with its
/// organization's behaviour (`52DD00` -> CAI `51D410`: the Mind state's
/// behavior_required, DESIRED reading the organization `+8D` byte), an
/// unattached element with whether it has a performer (`+74`); an actor
/// without a Mind (a building) answers false.
fn mind_aggressive(world: &World, st: &CombatState, t: TargetRef) -> bool {
    match t {
        TargetRef::Org(o) => behavior(world, st, o).0,
        TargetRef::Actor(a) => {
            let Some(actor) = world.actors.get(a as usize) else { return false };
            if let Some(o) = attached_org(world, actor) {
                return behavior(world, st, o).0;
            }
            let def = world.data.def(actor.def);
            matches!(def.class, Some(DefClass::Member)) && !def.weapons.is_empty()
        }
    }
}

/// The harmless classes of `527240` / `537530`: 4 for the current target,
/// 1 for a plain body (Body `+60`, neither captureable `+290` nor
/// destructible `+291`), else 2.
fn harmless_priority(world: &World, target: ActorId, current: bool) -> i32 {
    if current {
        return 4;
    }
    let def = world.data.def(world.actors[target as usize].def);
    if def.health > 0.0 && !def.captureable && !def.destructible { 1 } else { 2 }
}

/// `537530` (element Mind virtual `+1AC`): 0 for a target `4D4FF0(target,
/// 1, 0)` refuses (not an attackable enemy). The current target is the
/// regiment's (CAI `+4C`, `522700`: the target of its attack command, none
/// for a wall `+90`) for an attached element, the element's own kill target
/// (`530DC0`) otherwise; a member of the current target organization counts
/// as it.
/// Then 6 with KillTargetIsHighestPriority, the harmless classes, 5 when
/// current or the target's Mind is aggressive, else 3.
fn element_priority(world: &World, st: &CombatState, shooter: ActorId, target: ActorId) -> i32 {
    let s = &world.actors[shooter as usize];
    let t = &world.actors[target as usize];
    if !targetable(world, target) || !st.hostile(s.kingdom, t.kingdom) {
        return 0;
    }
    let current = match attached_org(world, s) {
        Some(o) => st.orgs[o as usize].explicit.filter(|c| match c {
            TargetRef::Actor(a) => world.actors.get(*a as usize).is_none_or(|x| !world.data.def(x.def).has_component("WallComponent")),
            TargetRef::Org(_) => true,
        }),
        None => s.combat.target.map(TargetRef::Actor),
    };
    let target_org = attached_org(world, t);
    let is_current = current.is_some_and(|c| c == TargetRef::Actor(target) || target_org.is_some_and(|o| c == TargetRef::Org(o)));
    if is_current && st.rules().kill_target_highest {
        return 6;
    }
    if harmless(world, target) {
        return harmless_priority(world, target, is_current);
    }
    if is_current || mind_aggressive(world, st, TargetRef::Actor(target)) { 5 } else { 3 }
}

/// `534700` over the given candidates for one shooter: a candidate needs a
/// nonzero priority (`+1AC`) not below the best one's so far, and at least
/// `min_priority` unless it is `engage_object` (`535910` passes 2 and the
/// engage target object); then its `5340C0` decision ranked by `52F010`.
/// The distance is `5340C0`'s stored `dy*dy + dx*dx`.
fn choose(world: &mut World, st: &CombatState, shooter: ActorId, candidates: &[ActorId], engage_object: Option<ActorId>, min_priority: i32) -> Option<ActorId> {
    let mut best: Option<Candidate> = None;
    for &c in candidates {
        if c == shooter {
            continue;
        }
        let priority = element_priority(world, st, shooter, c);
        if priority == 0
            || best.is_some_and(|b| priority < b.priority)
            || (Some(c) != engage_object && priority < min_priority)
        {
            continue;
        }
        let Some(d) = decide(world, st, shooter, c) else { continue };
        let (s, t) = (world.actors[shooter as usize].pos, world.actors[c as usize].pos);
        let (dx, dy) = (x87::sub(ld(s.x), ld(t.x)), x87::sub(ld(s.y), ld(t.y)));
        let cand = Candidate { actor: c, priority, score: d.score, distance2: fstp(x87::add(x87::mul(dy, dy), x87::mul(dx, dx))) };
        if better(&best, &cand) {
            best = Some(cand);
        }
    }
    best.map(|c| c.actor)
}

/// `527240` for the regiment Mind: its current target is the explicit
/// attack (CAI `+4C`); an organization target is never harmless (`4D03F0`
/// is false with an OrganizationComponent).
fn regiment_priority(world: &World, st: &CombatState, org: OrgId, t: TargetRef) -> i32 {
    let current = st.orgs[org as usize].explicit == Some(t);
    if current && st.rules().kill_target_highest {
        return 6;
    }
    if let TargetRef::Actor(a) = t
        && harmless(world, a)
    {
        return harmless_priority(world, a, current);
    }
    if current || mind_aggressive(world, st, t) { 5 } else { 3 }
}

/// `5210D0` on the regiment engage state: the actor itself or its
/// organization is on the engage list (any priority).
fn engage_keeps(world: &World, st: &CombatState, org: OrgId, actor: ActorId) -> bool {
    let own = world.actors.get(actor as usize).and_then(|a| attached_org(world, a));
    st.orgs[org as usize]
        .engage
        .iter()
        .any(|n| n.target == TargetRef::Actor(actor) || own.is_some_and(|o| n.target == TargetRef::Org(o)))
}

/// `51FE60` (from the head): before the first node whose priority is >= it.
fn insert_sorted(list: &mut Vec<EngageTarget>, target: TargetRef, priority: i32) {
    let i = list.iter().position(|n| priority <= n.priority).unwrap_or(list.len());
    list.insert(i, EngageTarget { target, priority });
}

/// `57A500(target, 1, 1)`: `4D0380` (the target has a Body or an
/// OrganizationComponent, is not invulnerable and is targetable for this kingdom,
/// `+84`), its enemy relation to this organization's owner (`+16C` -> `+140`), an
/// attached member within the operating radius (`578470`), and a member's ability
/// that could be used on it (`4D4FF0(target, 1, 0)`).
fn zone_accepts(world: &World, st: &CombatState, org: OrgId, t: TargetRef) -> bool {
    let kingdom = world.orgs[org as usize].kingdom;
    let (def, owner) = match t {
        TargetRef::Org(x) => (world.orgs[x as usize].def, Some(world.orgs[x as usize].kingdom)),
        TargetRef::Actor(a) => (world.actors[a as usize].def, world.actors[a as usize].kingdom),
    };
    let d = world.data.def(def);
    let org_target = matches!(t, TargetRef::Org(_));
    if !org_target && !(d.health > 0.0 && !d.invulnerable && d.targetable) {
        return false;
    }
    st.hostile(owner, Some(kingdom)) && within_operating(world, st, org, t) && any_member_ability(world, st, org, t)
}

/// `57A500(target, 0, 0)`, the kill state's test (0x51D920): as [`zone_accepts`]
/// without the operating radius. When it refuses, the kill and move handlers
/// (0x527FB0, 0x527E30) fall back to a Move at the thing (0x520320). (orders)
pub(crate) fn kill_accepts(world: &mut World, org: OrgId, target: ActorId) -> bool {
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    let kingdom = world.orgs[org as usize].kingdom;
    let t = &world.actors[target as usize];
    let d = world.data.def(t.def);
    let accepted = d.health > 0.0
        && !d.invulnerable
        && d.targetable
        && st.hostile(t.kingdom, Some(kingdom))
        && any_member_ability(world, &st, org, TargetRef::Actor(target));
    world.combat = st;
    accepted
}

/// `4D4FF0(target, 1, 0)` on the organization: some member's Performer ability is
/// eligible (`4C8740`), bears (`4C84B0`) and is past its minimum range (`4C8460`)
/// on the target or, for an organization, on a member of it. `57E920` has no
/// maximum range here (its last argument is 0).
fn any_member_ability(world: &World, st: &CombatState, org: OrgId, t: TargetRef) -> bool {
    let on = |m: ActorId, x: ActorId| {
        let def = world.data.def(world.actors[m as usize].def);
        let point = world.actors[x as usize].pos.truncate();
        def.weapons.iter().any(|w| !w.defender && admits(world, st, m, w, x) && in_turret_arc(world, m, w, point) && (w.min_range <= 0.0 || in_range(world, m, x, w.min_range, true)))
    };
    world.orgs[org as usize].members.iter().any(|&m| {
        alive(world, m)
            && match t {
                TargetRef::Org(x) => world.orgs[x as usize].members.iter().any(|&y| alive(world, y) && on(m, y)),
                TargetRef::Actor(x) => alive(world, x) && on(m, x),
            }
    })
}

/// `57AB70` (from `514E40`): an enemy, `who`, its organization when attached
/// (`514DA0`), entered this organization's ZOC, or the ZOC moved over it.
/// Accepted by `57A500`, it goes to the Mind (`524C10`) when the behaviour is
/// aggressive (`+8D`) or when it is the Mind's current target (virtual `+4C`).
pub(crate) fn zone_entered(world: &mut World, st: &mut CombatState, org: OrgId, who: crate::zones::Key) {
    let i = org as usize;
    if !world.orgs.get(i).is_some_and(|o| o.alive) || st.orgs.get(i).is_none_or(|s| s.destroyed || s.routing.is_some()) {
        return;
    }
    let t = match who {
        crate::zones::Key::Actor(a) => TargetRef::Actor(a),
        crate::zones::Key::Org(o) => TargetRef::Org(o),
    };
    if !target_alive(world, t) || !zone_accepts(world, st, org, t) {
        return;
    }
    if !behavior(world, st, org).0 && st.orgs[i].explicit != Some(t) {
        return;
    }
    add_engage_target(world, st, org, t);
}

/// `57A500` with members (`578470`): some attached member of the target is
/// within the operating radius of the organization position.
fn within_operating(world: &World, st: &CombatState, org: OrgId, t: TargetRef) -> bool {
    let o = &world.orgs[org as usize];
    let radius = world.data.def(o.def).operational_range;
    // 466D3D: +304 = fstp(operational_range^2); 578470 (aa_combat::
    // engagement::within_operating_radius): unstored dx*dx + dy*dy <= +304.
    let r2 = ld(fstp(x87::mul(ld(radius), ld(radius))));
    let pos = st.orgs[org as usize].position;
    let near = |a: ActorId| {
        let p = world.actors[a as usize].pos;
        let (dy, dx) = (x87::sub(ld(pos.y), ld(p.y)), x87::sub(ld(pos.x), ld(p.x)));
        alive(world, a) && x87::add(x87::mul(dx, dx), x87::mul(dy, dy)) <= r2
    };
    let attached = |m: ActorId| world.fog.engine.as_ref().and_then(|e| e.attached(m)) != Some(false);
    match t {
        TargetRef::Org(x) => world.orgs[x as usize].members.iter().any(|&m| attached(m) && near(m)),
        TargetRef::Actor(a) => near(a),
    }
}

/// `521060`'s `522CC0` (Mind virtual `+A0`): not a city (`+D4`) nor a block that
/// goes with one (`+94 +34`, `51D560`); the Mind's kill target is allowed (`+D0`);
/// a harmless Body neither captureable nor destructible is refused.
pub(crate) fn mind_can_target(world: &World, st: &CombatState, org: OrgId, t: TargetRef) -> bool {
    let TargetRef::Actor(a) = t else { return true };
    let Some(x) = world.actors.get(a as usize) else { return false };
    let d = world.data.def(x.def);
    if d.city || world.books.actor(a).is_some_and(|b| b.block_of.is_some_and(|c| alive(world, c))) {
        return false;
    }
    if st.orgs[org as usize].explicit == Some(t) {
        return true;
    }
    !(harmless(world, a) && d.health > 0.0 && !d.destructible && !d.captureable)
}

/// `521060` after the list test: `522CC0` and `4D4FF0(target, 1, 0)`.
fn engageable(world: &World, st: &CombatState, org: OrgId, t: TargetRef) -> bool {
    target_alive(world, t) && mind_can_target(world, st, org, t) && any_member_ability(world, st, org, t)
}

/// `524C10`/`523DD0`: add a target (element resolved to its organization).
pub(crate) fn add_engage_target(world: &World, st: &mut CombatState, org: OrgId, t: TargetRef) -> bool {
    if st.orgs[org as usize].engage.iter().any(|n| n.target == t) || !engageable(world, st, org, t) {
        return false;
    }
    let priority = regiment_priority(world, st, org, t);
    insert_sorted(&mut st.orgs[org as usize].engage, t, priority);
    true
}

/// `+8C`: the desired formation's `aggressive` flag (`577F30`, FormationData
/// `+1A`), set with the formation `57BFF0` and at `57C870`; no formation is 0.
fn desired_aggressive(world: &World, org: OrgId) -> bool {
    let def = world.data.def(world.orgs[org as usize].def);
    match crate::player_orders::desired_formation(world, org).and_then(|f| def.formations.get(f)) {
        Some(f) => f.aggressive,
        // a definition with no layout formations: no +70
        None => def.formations.is_empty(),
    }
}

/// Whether the organization's current Mind behavior is aggressive (`+8D`,
/// `577F30`: the state's behavior_required, DESIRED reading `+8C`) and whether
/// its Mind is free (`578E90`). actor_states.tgi: mind_moving, mind_resting,
/// mind_exploring DESIRED; mind_attacking, mind_engaged, mind_guarding,
/// mind_bombard, mind_reforming, mind_exhausted AGGRESSIVE; mind_routing,
/// mind_retreating, mind_repairing, mind_building, mind_unpacking PASSIVE.
pub(crate) fn behavior(world: &World, st: &CombatState, org: OrgId) -> (bool, bool) {
    let s = &st.orgs[org as usize];
    if s.routing.is_some() {
        return (false, false);
    }
    if s.auto_order.is_some() || s.reform.is_some() || s.exhausted_state {
        return (true, true);
    }
    match s.task {
        Some(tasks::Task::Bombard(_)) => return (true, false),
        Some(_) => return (false, false),
        None => {}
    }
    if crate::orders::retreating(world, org) {
        return (false, false);
    }
    if crate::orders::guarding(world, org) {
        return (true, false);
    }
    let desired = desired_aggressive(world, org);
    match &world.orgs[org as usize].order {
        Order::Idle => (desired, true),
        // 51D8F0: a move command's Kill state is mind_moving, no behavior_required.
        Order::Attack { .. } | Order::Move { .. } if s.kill_is_move => (desired, false),
        Order::Move { attack_move, .. } => (*attack_move || desired, false),
        Order::Attack { .. } => (true, false),
        Order::Deploy { .. } => (false, false),
    }
}

/// `577FD0` Organization `+AC`: the ZOC's radius, ActorData `+2FC` through the
/// CONTROL_RANGE multiply of its modifiers (`45D4C0` -> `501520`). In the shipped
/// data those are the states': mind_guarding +25%, mind_exhausted -25%.
pub(crate) fn control_radius(world: &World, st: &CombatState, org: OrgId) -> f32 {
    let s = &st.orgs[org as usize];
    let mut sum = 0.0f32;
    let mut state = |id: &str| {
        let Some(def) = world.data.actor_states.get(id) else { return };
        for p in def.properties.iter().filter_map(|p| world.data.property(p)) {
            for m in p.modifiers.iter().filter(|m| m.kind.eq_ignore_ascii_case("MULTIPLY") && m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("CONTROL_RANGE"))) {
                sum += m.amount;
            }
        }
    };
    if crate::orders::guarding(world, org) && s.auto_order.is_none() {
        state("mind_guarding");
    }
    if s.exhausted {
        state("mind_exhausted");
    }
    let clamp = world.data.rule_ci("ModifierAttributeClampMultiply").unwrap_or(0.05);
    world.data.def(world.orgs[org as usize].def).control_range * (1.0 + sum).max(clamp)
}

/// `57CD70` for the member an unpack replaces (`4D5EA0` -> `4F6760` destroys the
/// old thing): the MoraleDeathFactor loss of its `+2C0`, as for a death.
pub(crate) fn member_replaced(world: &mut World, org: OrgId, old_def: DefId) {
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    let factor = st.rules().morale_death_factor;
    crate::morale::lose(world, &mut st, org, world.data.def(old_def).morale_value * factor);
    world.combat = st;
}

/// `577F30`'s tail: `+8E` is set when `+8D` turns on, cleared when it is off.
fn track_behavior(world: &World, st: &mut CombatState, org: OrgId) {
    let now = behavior(world, st, org).0;
    let s = &mut st.orgs[org as usize];
    if !now {
        s.rescan_8e = false;
    } else if !s.behavior_8d {
        s.rescan_8e = true;
    }
    s.behavior_8d = now;
}

// ---------------------------------------------------------------------------
// The tick

pub fn tick(world: &mut World) {
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    expire_attacked_by(world, &mut st);
    queue_other_deaths(world, &mut st);
    st.grid.build(world);
    for id in 0..world.orgs.len() as OrgId {
        org_think(world, &mut st, id);
    }
    world.orders.move_onto.clear();
    lone_think(world, &mut st);
    fire(world, &mut st);
    process_deaths(world, &mut st);
    refresh_orders(world, &mut st);
    corpses(world, &mut st);
    world.combat = st;
}

/// Bodies brought to zero health by something other than a weapon (the
/// upkeep shortfall `578660`, periodic damage) take the same `5625C0` zero
/// health path: a capturable body is restored to 1, another non-destructible
/// one stays at zero, anything else joins the death queue. That damage record
/// has no source, so it replaces the last shooter and no kill credit follows.
fn queue_other_deaths(world: &mut World, st: &mut CombatState) {
    for id in 0..world.actors.len() {
        let a = &world.actors[id];
        if !a.alive || a.removed || a.health > 0.0 || st.actors[id].queued_death {
            continue;
        }
        let def = world.data.def(a.def);
        if def.health <= 0.0 {
            continue;
        }
        if !def.destructible && !matches!(def.class, Some(DefClass::Member)) {
            if capturable(world, id as ActorId) {
                world.actors[id].health = 1.0;
            }
            continue;
        }
        st.actors[id].last_hit = None;
        st.actors[id].last_damage = Some((0.0, None));
        if let Some(o) = world.actors[id].org
            && let Some(org) = st.orgs.get_mut(o as usize)
        {
            org.last_hit = None;
        }
        st.actors[id].queued_death = true;
        st.deaths.push(id as ActorId);
    }
}

/// After deaths: an attack order whose target actor died follows the
/// target's organization (`524C10` resolves an element to its organization),
/// before the movement system would drop the order as "target lost".
fn refresh_orders(world: &mut World, st: &mut CombatState) {
    for i in 0..world.orgs.len() {
        if !world.orgs[i].alive || st.orgs[i].routing.is_some() {
            continue;
        }
        let Order::Attack { target } = world.orgs[i].order else { continue };
        if alive(world, target) {
            continue;
        }
        let auto = st.orgs[i].auto_order.is_some();
        let next = if auto {
            st.orgs[i].current_target().and_then(|t| representative(world, t))
        } else {
            st.orgs[i].explicit.and_then(|t| representative(world, t))
        };
        match next {
            Some(rep) => {
                world.orgs[i].order = Order::Attack { target: rep };
                if auto {
                    st.orgs[i].auto_order = Some(Order::Attack { target: rep });
                }
            }
            None if auto => {
                st.orgs[i].auto_order = None;
                world.orgs[i].order = st.orgs[i].saved_order.take().unwrap_or(Order::Idle);
                st.orgs[i].seen_order = Some(world.orgs[i].order.clone());
            }
            None => {}
        }
    }
}

/// Player override detection, explicit attack, and the `526A70` engage Think.
fn org_think(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    if !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    let now = world.time;
    // A new player/opponent order replaces an engagement begun by the
    // organization itself; a plain move empties the engage list.
    let command = st.orgs[i].seen_order.as_ref() != Some(&world.orgs[i].order) && st.orgs[i].auto_order.as_ref() != Some(&world.orgs[i].order);
    if let Some(auto) = st.orgs[i].auto_order.clone()
        && world.orgs[i].order != auto
    {
        st.orgs[i].auto_order = None;
        st.orgs[i].saved_order = None;
    }
    if command {
        st.orgs[i].kill_is_move = crate::orders::take_move_onto(world, id);
    }
    if let Some((city, to)) = st.orgs[i].city_target {
        let walking = matches!(world.orgs[i].order, Order::Move { to: t, attack_move: true } if t.distance_squared(to) < 1.0);
        if !walking {
            st.orgs[i].city_target = None;
        } else if !alive(world, city) || !st.hostile(Some(world.orgs[i].kingdom), world.actors[city as usize].kingdom) {
            st.orgs[i].city_target = None;
            world.orgs[i].order = Order::Idle;
        }
    }
    st.orgs[i].seen_order = Some(world.orgs[i].order.clone());
    track_behavior(world, st, id);
    if st.orgs[i].routing.is_some() {
        return;
    }
    // (core) a HUD order's Mind state (bombard, repair, construct) replaces
    // the engage Think while it lasts.
    if tasks::org_think(world, st, id) {
        return;
    }
    if command && matches!(world.orgs[i].order, Order::Move { .. } | Order::Deploy { .. }) {
        // 0x52A420 replaces the engage state, whose exit 0x520FE0 empties the list
        // (0x51F040): a move and a kill at a location alike (0x520440).
        // (orders) only the retreat command pays the penalty (0x51D1E0, in
        // `crate::orders`): the move handler 0x527E30 does not.
        st.orgs[i].engage.clear();
        st.orgs[i].explicit = None;
    }
    // Explicit attack (cai_kill): the target, resolved to its organization,
    // is the current kill target (priority 6 with KillTargetIsHighestPriority).
    let explicit = match (&world.orgs[i].order, &st.orgs[i].auto_order) {
        (Order::Attack { target }, None) => Some(*target),
        _ => None,
    };
    match explicit {
        Some(target) => {
            let t = target_ref(world, target);
            if !target_alive(world, t) || !st.hostile(Some(world.orgs[i].kingdom), target_kingdom(world, t)) {
                // Target lost (Mind +140): back to rest.
                st.orgs[i].explicit = None;
                st.orgs[i].engage.retain(|n| n.target != t);
                world.orgs[i].order = Order::Idle;
            } else {
                if !alive(world, target)
                    && let Some(rep) = representative(world, t)
                {
                    world.orgs[i].order = Order::Attack { target: rep };
                }
                // 51D920: the kill state. It walks (520730 -> the Move state); the
                // target joins the engage list when it enters the ZOC (57AB70).
                st.orgs[i].explicit = Some(t);
            }
        }
        None => st.orgs[i].explicit = None,
    }
    let think = now >= st.orgs[i].next_think;
    if think {
        st.orgs[i].next_think = now + ENGAGE_THINK_SECONDS;
        // 523EA0 refresh: drop lost targets and those outside the operating radius.
        let mut list = std::mem::take(&mut st.orgs[i].engage);
        list.retain(|n| {
            target_alive(world, n.target)
                && st.hostile(Some(world.orgs[i].kingdom), target_kingdom(world, n.target))
                && within_operating(world, st, id, n.target)
        });
        st.orgs[i].engage = list;
    }
    // Begin / end the engagement on the organization's own initiative.
    let tail = st.orgs[i].current_target();
    match tail {
        Some(t) if st.orgs[i].explicit.is_none() => {
            if let Some(rep) = representative(world, t) {
                let order = Order::Attack { target: rep };
                // 524A00: the engage state takes over from sleep and from a move
                // (52A810(3): its priority), and the move goes on afterwards.
                let resting_or_attack_move = matches!(world.orgs[i].order, Order::Idle | Order::Move { .. });
                if st.orgs[i].auto_order.is_none() && resting_or_attack_move {
                    st.orgs[i].saved_order = Some(world.orgs[i].order.clone());
                }
                if st.orgs[i].auto_order.is_some() || resting_or_attack_move {
                    world.orgs[i].order = order.clone();
                    st.orgs[i].auto_order = Some(order);
                }
            }
        }
        None if st.orgs[i].auto_order.is_some() => {
            st.orgs[i].auto_order = None;
            world.orgs[i].order = st.orgs[i].saved_order.take().unwrap_or(Order::Idle);
            // the move goes on: no new command
            st.orgs[i].seen_order = Some(world.orgs[i].order.clone());
        }
        _ => {}
    }
    assign_members(world, st, id);
}

/// The regiment's members against its engaged target. `526A70` step 5 hands
/// every member the tail target (UAI `+F0`, `532070`); `536E10` keeps a
/// member's kill target while its kill state is active (the target valid)
/// and the regiment's engage list still holds the handed target (`5210D0`;
/// the tail always is on it here); otherwise `535910` picks one: `534700`
/// over the eligible actors within the operational range of the organization
/// position (`558150` -> `578470`: `dx*dx + dy*dy <= +304`, the squared
/// range) that the regiment's engage list holds, itself or through its
/// organization (UAI `+A0`, `52DCA0` -> `5210D0`), with minimum priority 2.
/// `52DCA0` also admits allies (actor virtual `+164`: `+138` on the
/// candidate's owner), which `537530` then answers with priority 0.
fn assign_members(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let now = world.time;
    let tail = st.orgs[i].current_target();
    let members = world.orgs[i].members.clone();
    let Some(t) = tail.filter(|_| st.orgs[i].routing.is_none()) else {
        for m in members {
            if let Some(a) = world.actors.get_mut(m as usize) {
                a.combat.target = None;
            }
            if let Some(s) = st.actors.get_mut(m as usize) {
                s.engage_target = None;
            }
        }
        return;
    };
    let pos = st.orgs[i].position;
    let range = world.data.def(world.orgs[i].def).operational_range;
    // 466D3D: +304 = fstp(operational_range * operational_range).
    let r2 = fstp(x87::mul(ld(range), ld(range)));
    let within = |world: &World, a: ActorId| {
        let p = world.actors[a as usize].pos;
        let (dy, dx) = (x87::sub(ld(pos.y), ld(p.y)), x87::sub(ld(pos.x), ld(p.x)));
        x87::add(x87::mul(dx, dx), x87::mul(dy, dy)) <= ld(r2)
    };
    let mut candidates = std::mem::take(&mut st.scratch);
    candidates.clear();
    for n in &st.orgs[i].engage {
        let reach = |world: &World, a: ActorId| within(world, a);
        match n.target {
            TargetRef::Org(o) => {
                for &m in &world.orgs[o as usize].members {
                    if alive(world, m) && reach(world, m) && !candidates.contains(&m) {
                        candidates.push(m);
                    }
                }
            }
            TargetRef::Actor(a) => {
                if alive(world, a) && reach(world, a) && !candidates.contains(&a) {
                    candidates.push(a);
                }
            }
        }
    }
    // 535910's object: the engage target, an organization actor (never a
    // member candidate) or the actor itself.
    let engage_object = match t {
        TargetRef::Actor(a) => Some(a),
        TargetRef::Org(_) => None,
    };
    for m in members {
        if !alive(world, m) || st.actors[m as usize].queued_death {
            continue;
        }
        let current = world.actors[m as usize].combat.target;
        let kill_active = current.is_some_and(|c| valid_target(world, st, m, c));
        // 532070: a new engage target re-enters the engage state (reselect).
        let handed = st.actors[m as usize].engage_target.replace(t);
        // Playability bridge: `536E10` asks `5210D0` about the handed target
        // only, and the kill state walks after its target. Members here do
        // not chase, so a kill target that left the engage list is dropped
        // rather than kept out of reach.
        if handed == Some(t) && kill_active && current.is_some_and(|c| engage_keeps(world, st, id, c)) {
            continue;
        }
        let chosen = if now < st.actors[m as usize].roe_retry_at { None } else { choose(world, st, m, &candidates, engage_object, 2) };
        if chosen.is_some() && !kill_active {
            // 536E10 -> 52E830 engage reaction delay (the kill state was not
            // active): u * EngageReactionTimeRange.
            let delay = (world.random_at(crate::trace::sites::ENGAGE_REACTION) * st.rules().engage_reaction_range).max(EPSILON);
            st.actors[m as usize].ready_at = now + delay;
        }
        world.actors[m as usize].combat.target = chosen;
    }
    st.scratch = candidates;
}

/// Actors with weapons outside an organization (bunkers, defended
/// buildings): their own target within their weapon range, or the target of
/// an explicit actor attack order.
fn lone_think(world: &mut World, st: &mut CombatState) {
    let now = world.time;
    for id in 0..world.actors.len() as ActorId {
        let a = &world.actors[id as usize];
        if !a.alive || a.removed || a.org.is_some() || a.kingdom.is_none() || st.actors[id as usize].queued_death {
            continue;
        }
        if tasks::lone_think(world, st, id) {
            continue;
        }
        let data = world.data.clone();
        let a = &world.actors[id as usize];
        let def = data.def(a.def);
        // The Defender's abilities have their own list and Tick (`defender`).
        if !def.weapons.iter().any(|w| !w.defender) || matches!(def.class, Some(DefClass::Truck)) {
            continue;
        }
        let current_ok = a.combat.target.is_some_and(|t| valid_target(world, st, id, t));
        if now < st.actors[id as usize].next_think && current_ok {
            continue;
        }
        st.actors[id as usize].next_think = now + ENGAGE_THINK_SECONDS;
        let mut candidates = std::mem::take(&mut st.scratch);
        candidates.clear();
        let mut ordered = None;
        match a.order {
            Order::Attack { target } => {
                candidates.push(target);
                ordered = Some(target);
            }
            _ => {
                let range = def.weapons.iter().filter(|w| !w.defender).map(|w| w.range).fold(0.0f32, f32::max);
                let pos = a.pos.truncate();
                st.grid.query(pos, range, &mut candidates);
                candidates.retain(|&c| world.actors[c as usize].pos.truncate().distance_squared(pos) <= range * range);
            }
        }
        // Outside an organization there is no regiment engage list: every
        // nonzero priority competes (the ordered target is the object).
        let chosen = choose(world, st, id, &candidates, ordered, 0);
        st.scratch = candidates;
        let prev = world.actors[id as usize].combat.target;
        if chosen.is_some() && prev.is_none() {
            let delay = (world.random_at(crate::trace::sites::ENGAGE_REACTION) * st.rules().engage_reaction_range).max(EPSILON);
            st.actors[id as usize].ready_at = now + delay;
        }
        world.actors[id as usize].combat.target = chosen;
    }
}

/// Weapon recharge, the performer start (one ability at a time) and hits.
fn fire(world: &mut World, st: &mut CombatState) {
    let now = world.time;
    let data = world.data.clone();
    for id in 0..world.actors.len() {
        {
            let a = &mut world.actors[id];
            for r in &mut a.combat.recharge {
                *r = (*r - DT).max(0.0);
            }
        }
        let a = &world.actors[id];
        if !a.alive || a.removed || st.actors[id].queued_death {
            continue;
        }
        // 57EF00: idle turrets turn back to rest after reset_time.
        turret_reset_tick(world, st, id);
        let a = &world.actors[id];
        let Some(target) = a.combat.target else {
            // (core) a task's ability use (tasks::fire_aim).
            if let Some(aim) = st.actors[id].aim
                && !a.org.is_some_and(|o| st.orgs[o as usize].routing.is_some())
            {
                tasks::fire_aim(world, st, id, aim);
                continue;
            }
            if a.activity == Activity::Attacking {
                world.set_activity(id as ActorId, Activity::Idle);
            }
            continue;
        };
        if a.org.is_some_and(|o| st.orgs[o as usize].routing.is_some()) {
            continue;
        }
        if !valid_target(world, st, id as ActorId, target) {
            world.actors[id].combat.target = None;
            continue;
        }
        if now < st.actors[id].ready_at {
            continue;
        }
        if roe_backoff(world, st, id as ActorId) {
            // Back to its position (movement keeps the slot without a target).
            world.actors[id].combat.target = None;
            continue;
        }
        let Some(d) = decide(world, st, id as ActorId, target) else { continue };
        if d.status == 2 {
            if world.actors[id].activity == Activity::Attacking {
                world.set_activity(id as ActorId, Activity::Idle);
            }
            continue;
        }
        if d.status != 0 {
            continue;
        }
        let w = &data.def(world.actors[id].def).weapons[d.weapon];
        // 57F290 -> 57EE40: an ability with a turret that can turn waits in
        // the aiming state until 57E730 has the turret on the target (one
        // yaw_rotation_per_sec step a tick); the use (57ED20) follows.
        if !turret_ready(world, st, id, d.weapon, target) {
            continue;
        }
        world.set_activity(id as ActorId, Activity::Attacking);
        // 57F290: face the target when not moving (turrets turn on their own).
        let delta = world.actors[target as usize].pos.truncate() - world.actors[id].pos.truncate();
        if w.turret < 0 && world.actors[id].motion.velocity.length_squared() < 1e-4 && delta.length_squared() > 0.01 {
            world.actors[id].facing = delta.y.atan2(delta.x);
        }
        use_weapon(world, st, id as ActorId, d.weapon, target);
    }
    impacts(world, st);
}

// ---------------------------------------------------------------------------
// Turrets (Performer turret list; TurretDefinition from 4C7820)

/// `+2*pi` once when negative (the original's single correction).
fn wrap_once(a: f32) -> f32 {
    if a < 0.0 { a + TAU } else { a }
}

/// `4C7620`: the turret turns (a yaw rate and a yaw range).
fn turret_can_turn(t: &crate::data::TurretDef) -> bool {
    t.yaw_rate - EPSILON > 0.0 && t.yaw_half_range - EPSILON > 0.0
}

/// `4C76A0`: the turret offset turned by the actor's facing.
fn turret_offset(t: &crate::data::TurretDef, facing: Vec2) -> Vec2 {
    let o = Vec2::from(t.offset);
    Vec2::new(facing.x * o.x - facing.y * o.y, facing.x * o.y + facing.y * o.x)
}

/// `4C76E0`: a turret limited to less than the full turn reaches `point`
/// only inside its yaw range around its start direction.
fn turret_in_arc(t: &crate::data::TurretDef, pos: Vec2, facing: f32, point: Vec2) -> bool {
    if !turret_can_turn(t) || t.yaw_half_range + EPSILON >= PI {
        return true;
    }
    let f = Vec2::new(facing.cos(), facing.sin());
    let start = facing + t.yaw_start;
    let dir = Vec2::new(start.cos(), start.sin());
    let d = (point - (pos + turret_offset(t, f))).normalize_or_zero();
    d.dot(dir) >= t.yaw_half_range_cos
}

/// `57DEE0`: one tick of turning toward `target` (yaw_rotation_per_sec x
/// the tick), the short way round; true when the turret reached it.
fn turret_step(s: &mut TurretState, rate: f32, target: f32) -> bool {
    let mut cur = s.yaw;
    let mut target = target;
    let step = rate * DT;
    let d = (target - cur).abs();
    if TAU - d < d {
        if cur >= target {
            target += TAU;
        } else {
            cur += TAU;
        }
    }
    s.prev = cur;
    let (yaw, reached) = if cur >= target {
        let v = cur - step;
        if v > target { (v, false) } else { (target, true) }
    } else {
        let v = cur + step;
        if v < target { (v, false) } else { (target, true) }
    };
    s.yaw = yaw;
    reached
}

/// `57E730`: turret `i` of `id` turns toward `point` (its yaw in the
/// actor's frame, `atan(dir) - atan(facing) - pi/2`); the reset timer
/// restarts. True when it is on the point.
fn turret_aim(world: &World, st: &mut CombatState, id: usize, i: usize, t: &crate::data::TurretDef, point: Vec2) -> bool {
    let a = &world.actors[id];
    let f = Vec2::new(a.facing.cos(), a.facing.sin());
    let d = point - (a.pos.truncate() + turret_offset(t, f));
    let len = d.length();
    let dir = if len <= EPSILON { Vec2::ZERO } else { d / len };
    // 85D8D0: the table atan of (x, y), counter-clockwise in [0, 2 pi).
    let angle = |v: Vec2| wrap_once(v.y.atan2(v.x));
    let yaw = wrap_once(angle(dir) - angle(f) - FRAC_PI_2);
    let now = world.time;
    let Some(s) = st.actors.get_mut(id).and_then(|c| c.turrets.get_mut(i)) else { return true };
    s.last_used = now;
    if t.can_reset {
        s.active = true;
    }
    turret_step(s, t.yaw_rate, yaw)
}

/// `57EF00`: a turret that may reset turns back to `yaw_start - pi/2` once
/// it has not aimed for `reset_time`; the update stops when all are home.
fn turret_reset_tick(world: &World, st: &mut CombatState, id: usize) {
    let now = world.time;
    let def = world.data.def(world.actors[id].def);
    let Some(c) = st.actors.get_mut(id) else { return };
    for (s, t) in c.turrets.iter_mut().zip(&def.turrets) {
        if !s.active || !t.can_reset {
            s.prev = s.yaw;
            continue;
        }
        if t.reset_time <= now - s.last_used {
            let rest = wrap_once(t.yaw_start - FRAC_PI_2);
            if turret_step(s, t.yaw_rate, rest) {
                s.active = false;
            }
        } else {
            s.prev = s.yaw;
        }
    }
}

/// Projectiles reaching their target: the target if it is still there,
/// otherwise the launch point (area weapons only).
fn impacts(world: &mut World, st: &mut CombatState) {
    let now = world.time;
    let mut i = 0;
    while i < st.projectiles.len() {
        if st.projectiles[i].impact_at > now {
            i += 1;
            continue;
        }
        let p = st.projectiles.swap_remove(i);
        if let Some(t) = p.thing {
            world.ids.removed_transient(t);
        }
        deliver(world, st, p.shooter, p.weapon, p.target, p.point);
    }
}

/// `4C9550` phases: single target, or the area around the target (or the
/// ground point of a bombard) with the `max_targets` nearest and the
/// attenuation of phase 3.
fn deliver(world: &mut World, st: &mut CombatState, shooter: ActorId, weapon: usize, target: Option<ActorId>, point: Vec2) {
    let data = world.data.clone();
    let w = &data.def(world.actors[shooter as usize].def).weapons[weapon];
    let present = target.filter(|&t| targetable(world, t) && !st.actors[t as usize].queued_death);
    if w.area <= 0.0 {
        if let Some(t) = present {
            apply_hit(world, st, shooter, w, t, 1.0);
        }
        return;
    }
    let center = present.map_or(point, |t| world.actors[t as usize].pos.truncate());
    let mut found = std::mem::take(&mut st.scratch);
    st.grid.query(center, w.area + st.extras().max_soft_radius, &mut found);
    // 4C9C90: every actor of the area query the ability admits (4C8740 with
    // the ability's target filter, which decides friend or enemy). The query is
    // 558040 mode 1 (aa_spatial::circle_actors): (soft radius + area)^2 >= dx^2 +
    // dy^2 at PC53 with dx stored, so a building counts from its own radius.
    found.retain(|&c| {
        let x = &world.actors[c as usize];
        let reach = x87::add(ld(world.data.def(x.def).soft_radius), ld(w.area));
        let dx = ld(fstp(x87::sub(ld(x.pos.x), ld(center.x))));
        let dy = x87::sub(ld(x.pos.y), ld(center.y));
        targetable(world, c) && x87::mul(reach, reach) >= x87::add(x87::mul(dy, dy), x87::mul(dx, dx)) && admits(world, st, shooter, w, c)
    });
    if w.max_targets > 0 {
        found.sort_by(|&a, &b| {
            let da = world.actors[a as usize].pos.truncate().distance_squared(center);
            let db = world.actors[b as usize].pos.truncate().distance_squared(center);
            da.total_cmp(&db)
        });
        found.truncate(w.max_targets as usize);
    }
    // 4C9550 phase 3 (aa_combat::execution::use_ability): attenuated when
    // attenuation_range_min < area and attenuation_factor_max < 0x3F7FF972
    // (0.9999). The distance is fstp(sqrt(dx*dx + dy*dy)) less the target
    // radius (its `Size` soft radius), at least 0; past attenuation_range_min
    // the scale is fstp(1 - (1 - factor) * (distance - min) / (area - min)),
    // clamped to [factor, 1].
    let attenuated = w.attenuation_range_min < w.area && w.attenuation_factor_max < f32::from_bits(0x3f7f_f972);
    for &c in &found {
        let mut scale = 1.0;
        if attenuated {
            let p = world.actors[c as usize].pos;
            let (dx, dy) = (x87::sub(ld(center.x), ld(p.x)), x87::sub(ld(center.y), ld(p.y)));
            let length = fstp(x87::add(x87::mul(dx, dx), x87::mul(dy, dy)).sqrt());
            let mut distance = x87::sub(ld(length), ld(world.data.def(world.actors[c as usize].def).soft_radius));
            if distance < 0.0 {
                distance = 0.0;
            }
            let (min, factor) = (ld(w.attenuation_range_min), ld(w.attenuation_factor_max));
            let width = x87::sub(ld(w.area), min);
            if distance > min && width > 0.0 {
                let ratio = x87::div(x87::sub(distance, min), width);
                scale = fstp(x87::sub(1.0, x87::mul(x87::sub(1.0, factor), ratio)));
                if scale < w.attenuation_factor_max {
                    scale = w.attenuation_factor_max;
                }
                if scale > 1.0 {
                    scale = 1.0;
                }
            }
        }
        apply_hit(world, st, shooter, w, c, scale);
    }
    st.scratch = found;
}

/// The shooter fires under a Bombard Mind state: its hit is the event kind 3
/// of `5625C0` (context mode 3, `4C8300`).
fn bombarding(world: &World, st: &CombatState, shooter: ActorId) -> bool {
    let of_org = world.actors[shooter as usize].org.and_then(|o| st.orgs.get(o as usize)).is_some_and(|o| matches!(o.task, Some(tasks::Task::Bombard(_))));
    of_org || st.actors.get(shooter as usize).is_some_and(|a| matches!(a.task, Some(tasks::Task::Bombard(_))))
}

/// `4D6190(source, weapon)` on `target` (aa_combat::reactions::attacked_by):
/// the victim is its organization when a member; the attacker its organization
/// when attached (byte `+1C`), else itself. A new `+130` node lasts
/// BombardTargetTimeout, plus the weapon's definition `+20` when the victim's
/// Mind itself bombards (`+6C`), and `4D3A70` lists the victim in the
/// attacker's `+128`; a known attacker has its time refreshed.
fn attacked_by(world: &World, st: &mut CombatState, target: ActorId, shooter: ActorId, w: &Weapon) {
    let victim = target_ref(world, target);
    let attached = world.fog.engine.as_ref().and_then(|e| e.attached(shooter)).unwrap_or(false);
    let attacker = match world.actors[shooter as usize].org {
        Some(o) if attached => TargetRef::Org(o),
        _ => TargetRef::Actor(shooter),
    };
    let now = world.time;
    let bombards = match victim {
        TargetRef::Org(o) => st.orgs.get(o as usize).is_some_and(|x| matches!(x.task, Some(tasks::Task::Bombard(_)))),
        TargetRef::Actor(a) => st.actors.get(a as usize).is_some_and(|x| matches!(x.task, Some(tasks::Task::Bombard(_)))),
    };
    let list = match victim {
        TargetRef::Org(o) => &mut st.orgs[o as usize].attacked_by,
        TargetRef::Actor(a) => &mut st.actors[a as usize].attacked_by,
    };
    if let Some(e) = list.iter_mut().find(|e| e.attacker == attacker) {
        e.at = now;
        return;
    }
    let mut timeout = world.data.rule_ci("BombardTargetTimeout").unwrap_or(1.0);
    if bombards {
        timeout += w.recharge;
    }
    list.insert(0, AttackedBy { attacker, at: now, timeout });
    match attacker {
        TargetRef::Org(o) => st.orgs[o as usize].bombarding.insert(0, victim),
        TargetRef::Actor(a) => st.actors[a as usize].bombarding.insert(0, victim),
    }
}

/// `4D37E0`: a `+130` node goes when its owner is dead, its attacker gone or
/// its time over (`at + timeout <= now`), and its attacker's `+128` forgets the
/// victim (`4F1430`). The original runs it at each Mind update and at a death;
/// here once per combat tick.
fn expire_attacked_by(world: &World, st: &mut CombatState) {
    let now = world.time;
    let mut over: Vec<(TargetRef, TargetRef)> = Vec::new();
    let mut sweep = |list: &mut Vec<AttackedBy>, owner: TargetRef, alive_owner: bool| {
        list.retain(|e| {
            let keep = alive_owner && target_alive(world, e.attacker) && e.at + e.timeout > now;
            if !keep {
                over.push((e.attacker, owner));
            }
            keep
        });
    };
    for i in 0..st.actors.len() {
        if !st.actors[i].attacked_by.is_empty() {
            let owner = i as ActorId;
            sweep(&mut st.actors[i].attacked_by, TargetRef::Actor(owner), alive(world, owner));
        }
    }
    for i in 0..st.orgs.len() {
        if !st.orgs[i].attacked_by.is_empty() {
            let owner = TargetRef::Org(i as OrgId);
            sweep(&mut st.orgs[i].attacked_by, owner, target_alive(world, owner));
        }
    }
    for (attacker, victim) in over {
        let list = match attacker {
            TargetRef::Org(o) => st.orgs.get_mut(o as usize).map(|x| &mut x.bombarding),
            TargetRef::Actor(a) => st.actors.get_mut(a as usize).map(|x| &mut x.bombarding),
        };
        if let Some(list) = list
            && let Some(i) = list.iter().position(|v| *v == victim)
        {
            list.remove(i);
        }
    }
}

/// The hit (`004CA400` -> Body `5625C0`) and the damage-driven engage `579490`.
fn apply_hit(world: &mut World, st: &mut CombatState, shooter: ActorId, w: &Weapon, target: ActorId, scale: f32) {
    let now = world.time;
    let amount = {
        let s = &world.actors[shooter as usize];
        let t = &world.actors[target as usize];
        damage_amount(
            shooter_power(world, st, s, w),
            shooter_offense(st, s),
            scale,
            target_defense(world, st, t),
            type_factor(world, st, w.damage_type, t),
            w.flags,
            st.rules().damage_minimum,
        )
    };
    if amount <= 0.0 {
        return;
    }
    let t = &mut world.actors[target as usize];
    t.health = (t.health - amount).max(0.0);
    let dead = t.health <= 0.0;
    let victim_org = t.org;
    world.events.push(Event::Hit { target, damage: amount });
    st.actors[target as usize].last_hit = Some((shooter, now));
    st.actors[target as usize].last_damage = Some((amount, Some(w.damage_type)));
    if bombarding(world, st, shooter) {
        attacked_by(world, st, target, shooter, w);
    }
    if is_primary(world, target) {
        // 570F70 / 570A70: the primary's damage history, pruned of records
        // older than fstp(now - ROEPrimaryDamageHistoryLength).
        let cutoff = fstp(x87::sub(ld(now), ld(st.rules().roe_history_length)));
        let h = &mut st.actors[target as usize].history;
        h.retain(|(t, _)| *t >= cutoff);
        h.push((now, amount));
    }
    let kingdom = world.actors[shooter as usize].kingdom;
    if dead && !st.actors[target as usize].queued_death && !zero_health(world, st, kingdom, target) {
        st.actors[target as usize].queued_death = true;
        st.deaths.push(target);
    }
    // ABILITY_EFFECT modifiers of the shooter (special operations): their
    // ADD_DAMAGE (type factor x amount) joins every attack of the type.
    if !dead {
        for (_, extra, dt) in crate::special_ops::extra_hits(world, shooter, w.ability_type.as_deref()) {
            let add = add_damage_amount(world, st, target, extra, dt);
            if add > 0.0 && world.actors[target as usize].alive {
                let t = &mut world.actors[target as usize];
                t.health = (t.health - add).max(0.0);
                let now_dead = t.health <= 0.0;
                world.events.push(Event::Hit { target, damage: add });
                if now_dead && !st.actors[target as usize].queued_death && !zero_health(world, st, kingdom, target) {
                    st.actors[target as usize].queued_death = true;
                    st.deaths.push(target);
                }
            }
        }
    }
    if let Some(o) = victim_org {
        st.orgs[o as usize].last_hit = Some((shooter, now));
        // 579490 OnElementDamaged: an attacker itself inside the operating
        // radius (578470 on the attacker) is engaged (524C10, resolved to
        // its organization) when the behavior is aggressive or the Mind is
        // free.
        let (aggressive, free) = behavior(world, st, o);
        if (aggressive || free) && st.orgs[o as usize].routing.is_none() && within_operating(world, st, o, TargetRef::Actor(shooter)) {
            let t = target_ref(world, shooter);
            add_engage_target(world, st, o, t);
        }
    }
}

/// `5625C0` at zero health before the death queue: a capturable actor
/// (`4D1120`: captureable, hostile source) changes owner with
/// CaptureHealthFactor of its maximum health (`4D5230`), or without a capture
/// is restored to 1; a non-destructible body stays at zero. Returns true when
/// the actor does not die.
fn zero_health(world: &mut World, st: &mut CombatState, kingdom: Option<KingdomId>, target: ActorId) -> bool {
    let def = world.data.def(world.actors[target as usize].def);
    // (core) 4D1120: an actor with a CityComponent is never captured by
    // damage; cities change hands by presence (City tick 5647A0, supply.rs).
    let (captureable, destructible) = (capturable(world, target), def.destructible);
    if captureable && st.hostile(kingdom, world.actors[target as usize].kingdom)
        && let Some(k) = kingdom
    {
        let max = st.actors[target as usize].max_health;
        let t = &mut world.actors[target as usize];
        t.kingdom = Some(k);
        t.health = max * st.rules().capture_health_factor;
        t.combat.target = None;
        world.events.push(Event::Captured { actor: target, kingdom: k });
        return true;
    }
    // 4D1120 and no capture: 561E50(1.0) (+100 bit 30 is never set here).
    if captureable {
        world.actors[target as usize].health = 1.0;
        return true;
    }
    // Otherwise the body stays at zero until +291 says it dies: a city block in the
    // Comrades film holds hp 0 for 168 ticks, then heals.
    !destructible && def.health > 0.0 && !matches!(def.class, Some(DefClass::Member))
}

/// MAIN_DAMAGE of a faction ability (`4CA400` with a kingdom source: no
/// offensive component, so offense 1; the ability's main power).
pub(crate) fn kingdom_hit_amount(world: &World, st: &CombatState, target: ActorId, power: f32, scale: f32, damage_type: u32, flags: u32) -> f32 {
    let t = &world.actors[target as usize];
    damage_amount(power, 1.0, scale, target_defense(world, st, t), type_factor(world, st, damage_type, t), flags, st.rules().damage_minimum)
}

/// ADD_DAMAGE and periodic damage (`4CA400` other kinds): type factor x
/// amount (stored once), no defense, no minimum.
pub(crate) fn add_damage_amount(world: &World, st: &CombatState, target: ActorId, amount: f32, damage_type: u32) -> f32 {
    let base = if amount < 0.0 { 0.0 } else { amount };
    fstp(x87::mul(ld(type_factor(world, st, damage_type, &world.actors[target as usize])), ld(base)))
}

/// Damage from a kingdom (a special operation): Body `5625C0` without an
/// actor source, so no kill credit and no reactive engagement.
pub(crate) fn damage_from_kingdom(world: &mut World, st: &mut CombatState, kingdom: KingdomId, target: ActorId, amount: f32, damage_type: Option<u32>) {
    let i = target as usize;
    if amount <= 0.0 || !world.actors[i].alive {
        return;
    }
    let t = &mut world.actors[i];
    t.health = (t.health - amount).max(0.0);
    let dead = t.health <= 0.0;
    world.events.push(Event::Hit { target, damage: amount });
    if st.actors.len() <= i {
        st.actors.resize_with(i + 1, ActorCombat::default);
    }
    st.actors[i].last_hit = None;
    st.actors[i].last_damage = Some((amount, damage_type));
    if dead && !st.actors[i].queued_death && !zero_health(world, st, Some(kingdom), target) {
        st.actors[i].queued_death = true;
        st.deaths.push(target);
    }
}

/// Periodic damage of the Properties tick (`5818D0`: event kind 2, reason
/// 10, the Body type modifier): no event per tick, no credit.
pub(crate) fn periodic_damage(world: &mut World, st: &mut CombatState, target: ActorId, amount: f32, damage_type: u32) {
    let i = target as usize;
    if amount <= 0.0 || !world.actors[i].alive {
        return;
    }
    let t = &mut world.actors[i];
    t.health = (t.health - amount).max(0.0);
    let dead = t.health <= 0.0;
    if st.actors.len() <= i {
        st.actors.resize_with(i + 1, ActorCombat::default);
    }
    st.actors[i].last_hit = None;
    st.actors[i].last_damage = Some((amount, Some(damage_type)));
    if dead && !st.actors[i].queued_death && !zero_health(world, st, None, target) {
        st.actors[i].queued_death = true;
        st.deaths.push(target);
    }
}

/// Maximum health from the definition, the regiment's level and the
/// enchantments (HEALTH multipliers add; floored at
/// ModifierAttributeClampMultiply). Current health is kept, at most the new
/// maximum.
pub(crate) fn refresh_max_health(world: &mut World, st: &mut CombatState, actor: ActorId) {
    let i = actor as usize;
    let Some(a) = world.actors.get(i) else { return };
    // (core) 502A70: level, handicap/technology and enchantment HEALTH
    // amounts in one multiplier.
    let mut sum = level_of(world, st, a).map_or(0.0, |l| l.health) + crate::production::health_amount(world, a.kingdom, a.def);
    if let Some(m) = crate::special_ops::mods(world, actor) {
        sum += m.health;
    }
    let max = world.data.def(a.def).health * (1.0 + sum).max(st.rules().clamp_multiply);
    if st.actors.len() <= i {
        st.actors.resize_with(i + 1, ActorCombat::default);
    }
    st.actors[i].max_health = max;
    let a = &mut world.actors[i];
    if a.health > max {
        a.health = max;
    }
}

/// Death (`5625C0` zero health -> death queue -> BeginDeath), kill credit,
/// slayer morale, member removal and organization destruction.
fn process_deaths(world: &mut World, st: &mut CombatState) {
    let now = world.time;
    let data = world.data.clone();
    let rules = st.rules.clone().expect("prepared");
    let extras = st.extras.clone().expect("prepared");
    let mut deaths = std::mem::take(&mut st.deaths);
    for &v in &deaths {
        let victim_def = world.actors[v as usize].def;
        {
            let a = &mut world.actors[v as usize];
            a.alive = false;
            a.health = 0.0;
            a.combat.target = None;
        }
        world.set_activity(v, Activity::Dying);
        world.events.push(Event::Died { actor: v });
        // (core) 0x4D7960 -> 0x568B10: a deployed denizen frees its slot.
        crate::denizen::died(world, v);
        let def = data.def(victim_def);
        let ex = extras.def(victim_def);
        let clip = if def.death_clips.is_empty() {
            None
        } else {
            let (amount, kind) = st.actors[v as usize].last_damage.unwrap_or((0.0, None));
            let filter = kind.map(|t| extras.damage_filters.get(t as usize).copied().unwrap_or(DEATH_FILTER_ANY));
            choose_death_clip(&def.death_clips, amount, filter, world.random_at(crate::trace::sites::DEATH_CLIP))
        };
        let clip_time = clip.map_or(ex.death_time, |c| def.death_clips[c].length);
        {
            let s = &mut st.actors[v as usize];
            s.death_clip = clip;
            s.died_at = Some(now);
            s.dead_at = now + clip_time;
            s.remove_at = s.dead_at + (def.rot_time + def.sink_time).min(CORPSE_DELAY_CAP);
        }
        // 4D57C0 kill credit: last hostile damage within KillCreditTimeout.
        let killer = st.actors[v as usize]
            .last_hit
            .filter(|(_, t)| now - *t <= rules.kill_credit_timeout)
            .map(|(k, _)| k)
            .filter(|&k| st.hostile(world.actors[k as usize].kingdom, world.actors[v as usize].kingdom));
        let victim_org = world.actors[v as usize].org;
        if let Some(k) = killer {
            let killer_def = world.actors[k as usize].def;
            let killer_org = world.actors[k as usize].org;
            let killer_value = extras.def(killer_def).combat_value;
            let amount = experience_clamped(
                experience_ratio(ex.combat_value, ex.fake_cv_value, killer_value, &rules),
                rules.experience_kill,
                &rules,
            );
            if let Some(ko) = killer_org {
                crate::morale::award(world, st, ko, amount);
                if victim_org.is_some() {
                    crate::morale::slayer(world, st, ko, killer_value, ex.combat_value);
                }
            }
        }
        // 57CD70 member removal.
        if let Some(o) = victim_org {
            let oi = o as usize;
            world.orgs[oi].members.retain(|&m| m != v);
            world.actors[v as usize].org = None;
            world.actors[v as usize].slot = None;
            st.orgs[oi].living = {
                let line = &world.data.def(world.orgs[oi].def).slot_morale_line;
                world.orgs[oi].members.iter().filter(|&&m| world.actors.get(m as usize).is_some_and(|a| a.alive && !a.removed && (line.is_empty() || a.slot.is_some_and(|i| line.get(i as usize).copied().unwrap_or(false))))).count() as i32
            };
            let loss = def.morale_value * rules.morale_death_factor;
            if world.orgs[oi].members.iter().any(|&m| alive(world, m)) {
                crate::morale::lose(world, st, o, loss);
            } else {
                destroy_org(world, st, o, killer);
            }
        }
    }
    deaths.clear();
    st.deaths = deaths;
}

/// The last member is gone: the organization is destroyed (its own kill
/// credit with the organization value ratio).
fn destroy_org(world: &mut World, st: &mut CombatState, o: OrgId, killer: Option<ActorId>) {
    let oi = o as usize;
    if st.orgs[oi].destroyed {
        return;
    }
    let rules = st.rules.clone().expect("prepared");
    st.orgs[oi].destroyed = true;
    st.orgs[oi].engage.clear();
    st.orgs[oi].routing = None;
    world.orgs[oi].alive = false;
    world.orgs[oi].order = Order::Idle;
    world.events.push(Event::OrgDestroyed { org: o });
    let killer = killer.or_else(|| {
        st.orgs[oi].last_hit.filter(|(_, t)| world.time - *t <= rules.kill_credit_timeout).map(|(k, _)| k)
    });
    if let Some(k) = killer
        && let Some(ko) = world.actors[k as usize].org
        && world.orgs[ko as usize].alive
    {
        let victim_value = org_value(world, st, o);
        let source_value = org_value(world, st, ko);
        let fake = st.extras().def(world.orgs[oi].def).fake_cv_value;
        let amount = experience_clamped(experience_ratio(victim_value, fake, source_value, &rules), rules.experience_kill, &rules);
        crate::morale::award(world, st, ko, amount);
    }
}

/// `4D01E0` of an organization: its `+6C` maximum combat value (the slot
/// definitions' values, members and vacant slots alike).
pub fn org_value(world: &World, st: &CombatState, o: OrgId) -> f32 {
    let org = &world.orgs[o as usize];
    let def = world.data.def(org.def);
    let ex = st.extras();
    if def.slots.is_empty() {
        return org.members.iter().map(|&m| ex.def(world.actors[m as usize].def).combat_value).sum();
    }
    def.slots.iter().filter(|(d, _)| *d != u32::MAX).map(|(d, _)| ex.def(*d).combat_value).sum()
}

/// Dying -> Dead after the death clip; corpse removed after rot + sink.
fn corpses(world: &mut World, st: &mut CombatState) {
    let now = world.time;
    for id in 0..world.actors.len() {
        let Some(_) = st.actors[id].died_at else { continue };
        let a = &world.actors[id];
        if a.removed {
            continue;
        }
        if a.activity == Activity::Dying && now >= st.actors[id].dead_at {
            world.set_activity(id as ActorId, Activity::Dead);
        }
        if now >= st.actors[id].remove_at && world.actors[id].activity == Activity::Dead {
            world.actors[id].removed = true;
        }
    }
}

/// Combat state of an organization, for the UI and the opponent.
pub fn org_state(world: &World, org: OrgId) -> Option<&OrgCombat> {
    world.combat.org(org)
}

#[cfg(test)]
mod native_tests {
    use super::*;

    /// 4CA400 keeps the chain on the x87 stack and stores once:
    /// (0.7f * 5 - 2) * 0.8f is stored as 0x3F999999, where rounding every
    /// operation to binary32 would give 0x3F99999A.
    #[test]
    fn hit_arithmetic_stores_once() {
        let amount = damage_amount(5.0, 0.7, 1.0, 2.0, 0.8, 0, 1.0);
        assert_eq!(amount.to_bits(), 0x3f99_9999);
        assert_eq!(((0.7f32 * 5.0 - 2.0) * 0.8).to_bits(), 0x3f99_999a);
    }

    /// 52F010: priority first, scores within 1e-4 tie, squared distances
    /// within 1e-4 are equal (455000), otherwise the nearer wins.
    #[test]
    fn candidate_ranking() {
        let c = |priority, score, distance2| Candidate { actor: 0, priority, score, distance2 };
        assert!(better(&None, &c(1, 0.0, 0.0)));
        assert!(better(&Some(c(3, 9.0, 1.0)), &c(5, 1.0, 9.0)));
        assert!(!better(&Some(c(5, 1.0, 1.0)), &c(3, 9.0, 0.0)));
        assert!(better(&Some(c(5, 1.0, 9.0)), &c(5, 1.5, 9.0)));
        assert!(better(&Some(c(5, 1.0, 9.0)), &c(5, 1.00005, 4.0)), "scores tie, nearer wins");
        assert!(!better(&Some(c(5, 1.0, 100.0)), &c(5, 1.0, 100.00005)), "equal distances keep the best");
        assert!(!better(&Some(c(5, 1.0, 4.0)), &c(5, 1.0, 9.0)));
    }

    /// 571820 / 571960: the ratio stays unstored until the multiplier
    /// product is stored and clamped.
    #[test]
    fn experience_amounts() {
        let rules = Rules::from_data(&GameData::default());
        let ratio = experience_ratio(30.0, -1.0, 20.0, &rules);
        assert_eq!(ratio, 1.25);
        assert_eq!(experience_clamped(ratio, 1.0, &rules), 1.25);
        assert_eq!(experience_ratio(0.0, -1.0, 20.0, &rules), x87::add(x87::mul(-1.0 / 20.0, 0.5), 0.5));
        assert_eq!(experience_ratio(5.0, -1.0, 0.0, &rules), 0.0);
        assert_eq!(experience_clamped(100.0, 1.0, &rules), rules.experience_maximum);
    }
}

#[cfg(test)]
mod turret_tests {
    use super::*;

    /// 57DEE0: one rate x tick step, the short way round, and "aimed" only
    /// once the step reaches the target.
    #[test]
    fn turret_turns_the_short_way_at_its_rate() {
        let rate = 37f32.to_radians();
        let rest = wrap_once(-FRAC_PI_2);
        let mut s = TurretState { yaw: rest, prev: rest, last_used: 0.0, active: false };
        // A target 30 degrees to the left of forward: rest + 30 deg.
        let target = rest + 30f32.to_radians();
        let mut ticks = 0;
        while !turret_step(&mut s, rate, target) {
            ticks += 1;
            assert!(ticks < 1000);
        }
        let expected = (30.0 / 37.0 / DT).ceil() as i32;
        assert!((ticks + 1 - expected).abs() <= 1, "{ticks} ticks for 30 deg at 37 deg/s (expected about {expected})");
        assert!((s.yaw - target).abs() < 1e-5);
        // Across 0 / 2 pi: from 350 deg toward 10 deg turns +20 deg, not -340.
        let mut s = TurretState { yaw: 350f32.to_radians(), ..Default::default() };
        turret_step(&mut s, rate, 10f32.to_radians());
        assert!(s.yaw > 350f32.to_radians(), "turned the long way: {}", s.yaw.to_degrees());
    }
}