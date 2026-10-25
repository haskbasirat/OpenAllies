//! Target acquisition, weapon ranges and recharge, the original damage
//! formula, area damage, death, kill credit and combat value. Owner: combat.
//!
//! Behaviour-faithful plain `f32` rendition of the exact ports (read them for
//! the native detail):
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
//!   (aa_mind::cai_engage), zone entry `514E40 -> 57AB70 -> 57A500`, the
//!   damage-driven engage `579490` (aa_combat::engagement), engage Think
//!   `526A70` (1.0 s, aa_mind::cai_engage_think), target priority `527240`
//!   (aa_mind::cai_reactions::target_priority).
//! - member target choice `536E10 -> 535910 -> 534700` over the target
//!   organization's members within the operational range (`558150`, def
//!   `+300`), ranked by `52F010` (priority, then score, then distance), the
//!   per-ability decision `5340C0` (aa_mind::uai_kill) with the `52ED30`
//!   score (aa_mind::decision_score), the engage reaction delay `52E830`.
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
        }
    }
    pub fn def(&self, id: DefId) -> &DefExtras {
        static EMPTY: DefExtras = DefExtras {
            death_time: DEATH_CLIP_FALLBACK,
            fake_cv_value: -1.0,
            levels: Vec::new(),
            combat_value: 0.0,
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

/// `4BBF90`: trimmed means (descending sort, `count*discard` dropped at
/// each end) over the combat members' powers, defenses, speeds and healths.
fn means(data: &GameData, rules: &Rules) -> Means {
    let mut attack = Vec::new();
    let (mut defense, mut movement, mut health) = (Vec::new(), Vec::new(), Vec::new());
    for def in &data.defs {
        if !matches!(def.class, Some(DefClass::Member)) || def.health <= 0.0 {
            continue;
        }
        for w in &def.weapons {
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
        let k = (v.len() as f32 * rules.cv_discard) as usize;
        let slice = if 2 * k < v.len() { &v[k..v.len() - k] } else { &v[..] };
        slice.iter().sum::<f32>() / slice.len() as f32
    };
    let mut m = Means {
        attack: trimmed(&mut attack),
        defense: trimmed(&mut defense),
        movement: trimmed(&mut movement),
        health: trimmed(&mut health),
        ..Means::default()
    };
    m.offense = m.attack - m.defense;
    m.lifetime = if m.offense != 0.0 { m.health / m.offense } else { 0.0 };
    m
}

/// `45E090` for one ability of a definition without modifiers.
fn ability_value(w: &Weapon, defense: f32, health: f32, speed: f32, m: &Means, r: &Rules) -> f32 {
    let ranged = w.range >= r.cv_ranged_minimum;
    let mut offense = 0.0;
    if w.damage > 0.0 {
        let damage = (w.damage - m.defense).max(r.damage_minimum);
        let cycle = w.recharge.max(EPSILON);
        let rate = damage / cycle;
        let range_factor = w.range.max(0.0).powf(r.cv_range_power);
        let area_factor = (w.area * 0.5 + 1.0).powf(r.cv_area_power);
        offense = area_factor * range_factor * rate;
    }
    let lifetime = if health > 0.0 { health / (m.attack - defense) } else { 0.0 };
    let mut movement = if m.movement != 0.0 { (speed / m.movement - 1.0) * r.cv_movement_weight + 1.0 } else { 0.0 };
    if movement.is_nan() || movement <= 0.0 {
        movement = r.cv_movement_default;
    }
    let weighted_offense = (offense - m.offense) * r.cv_offense_weight + m.offense;
    let life_ratio = if m.lifetime != 0.0 { lifetime / m.lifetime } else { 0.0 };
    let weight = if life_ratio < 1.0 && ranged { r.cv_defense_weight_ranged } else { r.cv_defense_weight };
    let defense_factor = (life_ratio - 1.0) * weight + 1.0;
    let value = defense_factor * weighted_offense * movement;
    if value.is_finite() { value } else { 0.0 }
}

/// `4618B0` definition value: the best ability value (0 without abilities).
pub fn combat_values(data: &GameData, rules: &Rules) -> Vec<f32> {
    let m = means(data, rules);
    data.defs
        .iter()
        .map(|def| {
            def.weapons
                .iter()
                .map(|w| ability_value(w, def.defense, def.health, def.speed_max, &m, rules))
                .fold(0.0f32, |best, v| if v >= best { v } else { best })
        })
        .collect()
}

/// `571820`: `victim/source*0.5 + 0.5`; a zero victim value becomes -1 when
/// the victim's fake_cv_value is -1, else ExperienceDefaultValue; a
/// nonpositive source value gives 0.
pub fn experience_ratio(victim_value: f32, victim_fake_cv: f32, source_value: f32, rules: &Rules) -> f32 {
    let mut victim = victim_value;
    if victim == 0.0 {
        victim = if victim_fake_cv == -1.0 { -1.0 } else { rules.experience_default };
    }
    if source_value <= 0.0 {
        return 0.0;
    }
    victim / source_value * 0.5 + 0.5
}

/// `571960`/`5718E0`: the ratio times the multiplier, clamped to
/// [ExperienceGainMinimum, ExperienceGainMaximum] (minimum checked first).
pub fn experience_clamped(ratio: f32, multiplier: f32, rules: &Rules) -> f32 {
    let product = ratio * multiplier;
    if product < rules.experience_minimum {
        rules.experience_minimum
    } else if product > rules.experience_maximum {
        rules.experience_maximum
    } else {
        product
    }
}

// ---------------------------------------------------------------------------
// The damage formula

/// `004CA400` kind-4 arithmetic. `power` is the ability power (already with
/// AV), `defense` the body defense (with DV), `factor` the damage-type factor.
pub fn damage_amount(power: f32, offense: f32, scale: f32, defense: f32, factor: f32, flags: u32, minimum: f32) -> f32 {
    let base = power.max(0.0);
    let offense = if flags & 2 != 0 { 1.0 } else { offense };
    let defense = if flags & 1 != 0 { 0.0 } else { defense.max(0.0) };
    let amount = (offense * base * scale - defense) * factor;
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

/// Combat and morale state of one organization (OrganizationComponent and
/// its regiment Mind). Readable by the UI and the opponent.
#[derive(Clone, Debug, Default)]
pub struct OrgCombat {
    pub initialized: bool,
    pub destroyed: bool,
    /// Engage list, ascending priority (`51FE60`); the last entry is engaged.
    pub engage: Vec<EngageTarget>,
    /// Target of an explicit (player) attack order.
    pub explicit: Option<TargetRef>,
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
    /// ROE primary damage history (`570F70`): (time, amount).
    pub history: Vec<(f32, f32)>,
    /// ROE back-off: no attack before this time (`5340C0` retry time).
    pub roe_retry_at: f32,
    /// Performer turret states (`+4C`, one per `Def::turrets`).
    pub turrets: Vec<TurretState>,
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

    /// Bring the per-battle tables up to date (data change, new actors and
    /// organizations, kingdom teams, organization positions and engaged-by).
    pub(crate) fn prepare(&mut self, world: &mut World) {
        let key = Arc::as_ptr(&world.data) as usize;
        if self.rules.is_none() || self.data_key != key {
            let rules = Rules::from_data(&world.data);
            let extras = Extras::from_data(&world.data, &rules);
            self.rules = Some(Arc::new(rules));
            self.extras = Some(Arc::new(extras));
            self.data_key = key;
        }
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
                // 57F4D0: every turret starts at yaw_start - pi/2 (wrapped once).
                s.turrets = world.data.def(a.def).turrets.iter().map(|t| {
                    let yaw = wrap_once(t.yaw_start - FRAC_PI_2);
                    TurretState { yaw, prev: yaw, last_used: 0.0, active: false }
                }).collect();
            }
            s.targeted = false;
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
            self.orgs[o.id as usize].position = leader.map_or(o.anchor, |a| a.pos.truncate());
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
    modified_defense(def.defense, dv, st.rules().clamp_add)
}

fn shooter_power(world: &World, st: &CombatState, shooter: &crate::world::Actor, w: &Weapon) -> f32 {
    let (mut av, mut kind) = level_of(world, st, shooter).map_or((0.0, 0.0), |l| {
        (l.av, if w.projectile.is_some() { l.av_ranged } else { l.av_melee })
    });
    if let Some(m) = crate::special_ops::mods(world, shooter.id) {
        av += m.av;
        kind += if w.projectile.is_some() { m.av_ranged } else { m.av_melee };
    }
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

#[derive(Clone, Copy, Debug)]
struct Decision {
    weapon: usize,
    status: u8,
    score: f32,
}

/// `5340C0`: the best ability of `shooter` against `target` (the ROE
/// back-off is checked by the callers through [`roe_backoff`]).
fn decide(world: &World, st: &CombatState, shooter: ActorId, target: ActorId) -> Option<Decision> {
    let s = &world.actors[shooter as usize];
    let t = &world.actors[target as usize];
    let def = world.data.def(s.def);
    let distance = s.pos.truncate().distance(t.pos.truncate());
    let mut best: Option<Decision> = None;
    for (i, w) in def.weapons.iter().enumerate() {
        if !admits(world, st, shooter, w, target) {
            continue;
        }
        let left = s.combat.recharge.get(i).copied().unwrap_or(0.0);
        let status = weapon_status(w, left, distance);
        if status == 3 {
            continue;
        }
        // 52ED30: factor*priority + bonus + the performer bonus; pursuit takes
        // the wait weights while the ability is recharging (4C8A40), the move
        // weights otherwise, and always the performer move bonus.
        let mut score = match status {
            0 => w.use_weight * w.priority + w.use_bonus + def.performer_use_bonus,
            1 => w.wait_weight * w.priority + w.wait_bonus + def.performer_wait_bonus,
            _ if left > 0.0 => w.wait_weight * w.priority + w.wait_bonus + def.performer_move_bonus,
            _ => w.move_weight * w.priority + w.move_bonus + def.performer_move_bonus,
        };
        if w.damage > 0.0 {
            // 52ED30 player weight: the 501650 damage-type factor of the target.
            score *= type_factor(world, st, w.damage_type, t);
        }
        let take = match best {
            None => true,
            Some(b) if score > b.score => true,
            Some(b) if score < b.score => false,
            // Equal scores: the shorter range wins ("chooses the shortest ranged ability").
            Some(b) => status != 2 && w.range < def.weapons[b.weapon].range,
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
    let mut back = a.health / max < def.primary_minimum_health;
    if !back && def.primary_time_to_death > 0.0 {
        let window = r.roe_history_length.max(EPSILON);
        let sum: f32 = st.actors[i].history.iter().filter(|(t, _)| now - *t < window).map(|(_, d)| *d).sum();
        let rate = sum / window;
        back = rate > 0.0 && a.health / rate < def.primary_time_to_death;
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
/// then the nearer target).
fn better(best: &Option<Candidate>, c: &Candidate) -> bool {
    let Some(b) = best else { return true };
    if b.priority != c.priority {
        return c.priority > b.priority;
    }
    if (b.score - c.score).abs() >= EPSILON {
        return c.score > b.score;
    }
    c.distance2 < b.distance2
}

/// `4D4420`: `actor` (or its organization) is attacking `us` (or our organization).
fn attacks(world: &World, actor: ActorId, us: ActorId) -> bool {
    let Some(a) = world.actors.get(actor as usize) else { return false };
    let Some(t) = a.combat.target else { return false };
    let our_org = world.actors[us as usize].org;
    t == us || (our_org.is_some() && world.actors.get(t as usize).and_then(|x| x.org) == our_org)
}

/// `527240` for an element Mind choosing among elements.
fn element_priority(world: &World, st: &CombatState, shooter: ActorId, target: ActorId) -> i32 {
    let current = world.actors[shooter as usize].combat.target == Some(target);
    if current && st.rules().kill_target_highest {
        return 6;
    }
    if harmless(world, target) {
        return if current { 4 } else { 1 };
    }
    let fighting = world.actors[target as usize].combat.target.is_some();
    if current || fighting { 5 } else { 3 }
}

/// `534700` over the given candidates for one shooter.
fn choose(world: &World, st: &CombatState, shooter: ActorId, candidates: &[ActorId]) -> Option<ActorId> {
    let s = &world.actors[shooter as usize];
    let mut best: Option<Candidate> = None;
    for &c in candidates {
        if c == shooter || !targetable(world, c) || !st.hostile(s.kingdom, world.actors[c as usize].kingdom) {
            continue;
        }
        let Some(d) = decide(world, st, shooter, c) else { continue };
        let priority = element_priority(world, st, shooter, c);
        if priority == 0 {
            continue;
        }
        let bonus = if attacks(world, c, shooter) { st.rules().attacker_bonus } else { 0.0 };
        let cand = Candidate {
            actor: c,
            priority,
            score: d.score + bonus,
            distance2: s.pos.truncate().distance_squared(world.actors[c as usize].pos.truncate()),
        };
        if better(&best, &cand) {
            best = Some(cand);
        }
    }
    best.map(|c| c.actor)
}

/// `527240` for the regiment Mind (organization targets are never harmless).
fn regiment_priority(world: &World, st: &CombatState, org: OrgId, t: TargetRef) -> i32 {
    let current = st.orgs[org as usize].explicit == Some(t);
    if current && st.rules().kill_target_highest {
        return 6;
    }
    let (harmless_target, fighting) = match t {
        TargetRef::Org(o) => (false, st.orgs.get(o as usize).is_some_and(|x| x.engaging())),
        TargetRef::Actor(a) => (harmless(world, a), world.actors.get(a as usize).is_some_and(|x| x.combat.target.is_some())),
    };
    if harmless_target {
        return if current { 4 } else { 1 };
    }
    if current || fighting { 5 } else { 3 }
}

/// `51FE60` (from the head): before the first node whose priority is >= it.
fn insert_sorted(list: &mut Vec<EngageTarget>, target: TargetRef, priority: i32) {
    let i = list.iter().position(|n| priority <= n.priority).unwrap_or(list.len());
    list.insert(i, EngageTarget { target, priority });
}

/// `57A500` with members (`578470`): some attached member of the target is
/// within the operating radius of the organization position.
fn within_operating(world: &World, st: &CombatState, org: OrgId, t: TargetRef) -> bool {
    let o = &world.orgs[org as usize];
    let radius = world.data.def(o.def).operational_range;
    let r2 = radius * radius;
    let pos = st.orgs[org as usize].position;
    let near = |a: ActorId| alive(world, a) && world.actors[a as usize].pos.truncate().distance_squared(pos) <= r2;
    match t {
        TargetRef::Org(x) => world.orgs[x as usize].members.iter().any(|&m| near(m)),
        TargetRef::Actor(a) => near(a),
    }
}

/// `57A500` relation: an enemy that some member weapon can target (`4D4FF0`).
fn engageable(world: &World, st: &CombatState, org: OrgId, t: TargetRef) -> bool {
    let o = &world.orgs[org as usize];
    if !target_alive(world, t) || !st.hostile(Some(o.kingdom), target_kingdom(world, t)) {
        return false;
    }
    let Some(rep) = representative(world, t) else { return false };
    if !targetable(world, rep) {
        return false;
    }
    o.members.iter().any(|&m| {
        alive(world, m) && world.data.def(world.actors[m as usize].def).weapons.iter().any(|w| admits(world, st, m, w, rep))
    })
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

/// Whether the organization's current Mind behavior is aggressive (`+8D`,
/// from the current Mind state: resting DESIRED, attack-move and attacking
/// AGGRESSIVE, a plain move PASSIVE) and whether its Mind is free (`578E90`).
fn behavior(world: &World, st: &CombatState, org: OrgId) -> (bool, bool) {
    let s = &st.orgs[org as usize];
    if s.routing.is_some() {
        return (false, false);
    }
    if s.auto_order.is_some() {
        return (true, true);
    }
    match &world.orgs[org as usize].order {
        Order::Idle => (true, true),
        Order::Move { attack_move, .. } => (*attack_move, false),
        Order::Attack { .. } => (true, false),
        Order::Deploy { .. } => (false, false),
    }
}

// ---------------------------------------------------------------------------
// The tick

pub fn tick(world: &mut World) {
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    queue_other_deaths(world, &mut st);
    st.grid.build(world);
    for id in 0..world.orgs.len() as OrgId {
        org_think(world, &mut st, id);
    }
    lone_think(world, &mut st);
    fire(world, &mut st);
    process_deaths(world, &mut st);
    refresh_orders(world, &mut st);
    corpses(world, &mut st);
    world.combat = st;
}

/// Bodies brought to zero health by something other than a weapon (the
/// upkeep shortfall `578660`, periodic damage) take the same `5625C0` zero
/// health path: a non-destructible body is restored to 1, anything else
/// joins the death queue. That damage record has no source, so it replaces
/// the last shooter and no kill credit follows.
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
            world.actors[id].health = 1.0;
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
    // organization itself; a plain move is a retreat (579AB0, in morale.rs)
    // and empties the engage list.
    if let Some(auto) = st.orgs[i].auto_order.clone()
        && world.orgs[i].order != auto
    {
        st.orgs[i].auto_order = None;
        st.orgs[i].saved_order = None;
    }
    if st.orgs[i].routing.is_some() {
        return;
    }
    // (core) a HUD order's Mind state (bombard, repair, construct) replaces
    // the engage Think while it lasts.
    if tasks::org_think(world, st, id) {
        return;
    }
    if st.orgs[i].auto_order.is_none()
        && matches!(world.orgs[i].order, Order::Move { attack_move: false, .. } | Order::Deploy { .. })
    {
        let was_engaged = !st.orgs[i].engage.is_empty();
        st.orgs[i].engage.clear();
        st.orgs[i].explicit = None;
        if was_engaged {
            crate::morale::retreat_penalty(world, st, id);
        }
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
                if st.orgs[i].explicit != Some(t) {
                    st.orgs[i].explicit = Some(t);
                    st.orgs[i].engage.retain(|n| n.target != t);
                    let p = regiment_priority(world, st, id, t);
                    insert_sorted(&mut st.orgs[i].engage, t, p);
                }
            }
        }
        None => st.orgs[i].explicit = None,
    }
    let think = now >= st.orgs[i].next_think;
    if think {
        st.orgs[i].next_think = now + ENGAGE_THINK_SECONDS;
        let (aggressive, _) = behavior(world, st, id);
        // Zone entry (514E40 -> 57AB70): enemies inside the control zone.
        if aggressive {
            let def = world.data.def(world.orgs[i].def);
            let mut radius = def.control_range;
            if st.orgs[i].exhausted {
                radius *= 0.75; // actor_state_exhausted: control_range x(1 - .25)
            }
            let pos = st.orgs[i].position;
            let mut found = std::mem::take(&mut st.scratch);
            st.grid.query(pos, radius, &mut found);
            let r2 = radius * radius;
            for &a in &found {
                let actor = &world.actors[a as usize];
                if actor.pos.truncate().distance_squared(pos) > r2
                    || !st.hostile(Some(world.orgs[i].kingdom), actor.kingdom)
                    || !targetable(world, a)
                {
                    continue;
                }
                let t = target_ref(world, a);
                if within_operating(world, st, id, t) {
                    add_engage_target(world, st, id, t);
                }
            }
            st.scratch = found;
        }
        // 523EA0 refresh: drop lost targets and those outside the operating radius.
        let mut list = std::mem::take(&mut st.orgs[i].engage);
        let explicit = st.orgs[i].explicit;
        list.retain(|n| {
            target_alive(world, n.target)
                && st.hostile(Some(world.orgs[i].kingdom), target_kingdom(world, n.target))
                && (Some(n.target) == explicit || within_operating(world, st, id, n.target))
        });
        st.orgs[i].engage = list;
    }
    // Begin / end the engagement on the organization's own initiative.
    let tail = st.orgs[i].current_target();
    match tail {
        Some(t) if st.orgs[i].explicit.is_none() => {
            if let Some(rep) = representative(world, t) {
                let order = Order::Attack { target: rep };
                let resting_or_attack_move = matches!(world.orgs[i].order, Order::Idle | Order::Move { attack_move: true, .. });
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
        }
        _ => {}
    }
    assign_members(world, st, id, think);
}

/// `536E10` per member: keep or reselect a target inside the engaged target.
fn assign_members(world: &mut World, st: &mut CombatState, id: OrgId, think: bool) {
    let i = id as usize;
    let now = world.time;
    let tail = st.orgs[i].current_target();
    let explicit = st.orgs[i].explicit;
    let members = world.orgs[i].members.clone();
    let Some(t) = tail.filter(|_| st.orgs[i].routing.is_none()) else {
        for m in members {
            if let Some(a) = world.actors.get_mut(m as usize) {
                a.combat.target = None;
            }
        }
        return;
    };
    // Candidates (558150): the target's live members within the operational
    // range of the organization (no radius limit for an explicit attack).
    let pos = st.orgs[i].position;
    let radius = world.data.def(world.orgs[i].def).operational_range;
    let mut candidates = std::mem::take(&mut st.scratch);
    candidates.clear();
    match t {
        TargetRef::Org(o) => candidates.extend(world.orgs[o as usize].members.iter().copied().filter(|&m| {
            alive(world, m) && (explicit == Some(t) || world.actors[m as usize].pos.truncate().distance_squared(pos) <= radius * radius)
        })),
        TargetRef::Actor(a) => candidates.push(a),
    }
    for m in members {
        if !alive(world, m) || st.actors[m as usize].queued_death {
            continue;
        }
        let current = world.actors[m as usize].combat.target;
        let valid = current.is_some_and(|c| valid_target(world, st, m, c) && candidates.contains(&c));
        if !think && valid {
            continue;
        }
        let chosen = if now < st.actors[m as usize].roe_retry_at { None } else { choose(world, st, m, &candidates) };
        if chosen.is_some() && current.is_none() {
            // 52E830 engage reaction delay: u * EngageReactionTimeRange.
            let delay = (world.random() * st.rules().engage_reaction_range).max(EPSILON);
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
        let a = &world.actors[id as usize];
        let def = world.data.def(a.def);
        if def.weapons.is_empty() || matches!(def.class, Some(DefClass::Truck)) {
            continue;
        }
        let current_ok = a.combat.target.is_some_and(|t| valid_target(world, st, id, t));
        if now < st.actors[id as usize].next_think && current_ok {
            continue;
        }
        st.actors[id as usize].next_think = now + ENGAGE_THINK_SECONDS;
        let mut candidates = std::mem::take(&mut st.scratch);
        candidates.clear();
        match a.order {
            Order::Attack { target } => candidates.push(target),
            _ => {
                let range = def.weapons.iter().map(|w| w.range).fold(0.0f32, f32::max);
                let pos = a.pos.truncate();
                st.grid.query(pos, range, &mut candidates);
                candidates.retain(|&c| world.actors[c as usize].pos.truncate().distance_squared(pos) <= range * range);
            }
        }
        let chosen = choose(world, st, id, &candidates);
        st.scratch = candidates;
        let prev = world.actors[id as usize].combat.target;
        if chosen.is_some() && prev.is_none() {
            let delay = (world.random() * st.rules().engage_reaction_range).max(EPSILON);
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
        if let Some(t) = usize::try_from(w.turret).ok().and_then(|i| data.def(world.actors[id].def).turrets.get(i).map(|t| (i, t)))
            && turret_can_turn(t.1)
        {
            let (i, td) = t;
            let point = world.actors[target as usize].pos.truncate();
            if !turret_in_arc(td, world.actors[id].pos.truncate(), world.actors[id].facing, point) {
                continue;
            }
            if !turret_aim(world, st, id, i, td, point) {
                continue;
            }
        }
        // 501560: the recharge divided by the RECHARGE_TIME multiplier.
        let multiplier = crate::special_ops::mods(world, id as ActorId).map_or(1.0, |m| (1.0 + m.recharge).max(st.rules().clamp_multiply));
        world.actors[id].combat.recharge[d.weapon] = w.recharge / multiplier;
        world.events.push(Event::Fired { shooter: id as ActorId, target, weapon: d.weapon as u32 });
        world.set_activity(id as ActorId, Activity::Attacking);
        // 57F290: face the target when not moving (turrets turn on their own).
        let delta = world.actors[target as usize].pos.truncate() - world.actors[id].pos.truncate();
        if w.turret < 0 && world.actors[id].motion.velocity.length_squared() < 1e-4 && delta.length_squared() > 0.01 {
            world.actors[id].facing = delta.y.atan2(delta.x);
        }
        let point = world.actors[target as usize].pos.truncate();
        if w.projectile_speed > 0.0 {
            let flight = delta.length() / w.projectile_speed;
            st.projectiles.push(Projectile { shooter: id as ActorId, weapon: d.weapon, target: Some(target), point, impact_at: now + flight });
        } else {
            deliver(world, st, id as ActorId, d.weapon, Some(target), point);
        }
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
    st.grid.query(center, w.area, &mut found);
    let kingdom = world.actors[shooter as usize].kingdom;
    found.retain(|&c| {
        targetable(world, c)
            && st.hostile(kingdom, world.actors[c as usize].kingdom)
            && world.actors[c as usize].pos.truncate().distance(center) <= w.area
            && admits(world, st, shooter, w, c)
    });
    if w.max_targets > 0 {
        found.sort_by(|&a, &b| {
            let da = world.actors[a as usize].pos.truncate().distance_squared(center);
            let db = world.actors[b as usize].pos.truncate().distance_squared(center);
            da.total_cmp(&db)
        });
        found.truncate(w.max_targets as usize);
    }
    let radius = st.rules().default_radius;
    let attenuated = w.attenuation_range_min < w.area && w.attenuation_factor_max < 0.99999;
    for &c in &found {
        let mut scale = 1.0;
        if attenuated {
            let distance = (world.actors[c as usize].pos.truncate().distance(center) - radius).max(0.0);
            let width = w.area - w.attenuation_range_min;
            if distance > w.attenuation_range_min && width > 0.0 {
                let ratio = (distance - w.attenuation_range_min) / width;
                scale = (1.0 - (1.0 - w.attenuation_factor_max) * ratio).clamp(w.attenuation_factor_max, 1.0);
            }
        }
        apply_hit(world, st, shooter, w, c, scale);
    }
    st.scratch = found;
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
    if is_primary(world, target) {
        // 570F70 / 570A70: the primary's damage history.
        let window = st.rules().roe_history_length;
        let h = &mut st.actors[target as usize].history;
        h.retain(|(t, _)| now - *t < window);
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
        // 579490 OnElementDamaged: attacker inside the operating radius is
        // engaged when the behavior is aggressive or the Mind is free.
        let (aggressive, free) = behavior(world, st, o);
        if (aggressive || free) && st.orgs[o as usize].routing.is_none() {
            let t = target_ref(world, shooter);
            if within_operating(world, st, o, t) {
                add_engage_target(world, st, o, t);
            }
        }
    }
}

/// `5625C0` at zero health before the death queue: a capturable actor
/// (`4D1120`: captureable, hostile source) changes owner with
/// CaptureHealthFactor of its maximum health (`4D5230`); a non-destructible
/// body is restored to 1. Returns true when the actor does not die.
fn zero_health(world: &mut World, st: &mut CombatState, kingdom: Option<KingdomId>, target: ActorId) -> bool {
    let def = world.data.def(world.actors[target as usize].def);
    // (core) 4D1120: an actor with a CityComponent is never captured by
    // damage; cities change hands by presence (City tick 5647A0, supply.rs).
    let (captureable, destructible) = (def.captureable && !def.city, def.destructible);
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
    if !destructible && def.health > 0.0 && !matches!(def.class, Some(DefClass::Member)) {
        world.actors[target as usize].health = 1.0;
        return true;
    }
    false
}

/// MAIN_DAMAGE of a faction ability (`4CA400` with a kingdom source: no
/// offensive component, so offense 1; the ability's main power).
pub(crate) fn kingdom_hit_amount(world: &World, st: &CombatState, target: ActorId, power: f32, scale: f32, damage_type: u32, flags: u32) -> f32 {
    let t = &world.actors[target as usize];
    damage_amount(power, 1.0, scale, target_defense(world, st, t), type_factor(world, st, damage_type, t), flags, st.rules().damage_minimum)
}

/// ADD_DAMAGE and periodic damage (`4CA400` other kinds): type factor x
/// amount, no defense, no minimum.
pub(crate) fn add_damage_amount(world: &World, st: &CombatState, target: ActorId, amount: f32, damage_type: u32) -> f32 {
    amount.max(0.0) * type_factor(world, st, damage_type, &world.actors[target as usize])
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
            choose_death_clip(&def.death_clips, amount, filter, world.random())
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
            st.orgs[oi].living -= 1;
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