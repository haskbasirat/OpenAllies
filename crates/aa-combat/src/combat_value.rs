//! Native combat value: ability evaluation `0x45E090` and the thing-data mean
//! values `0x4BBF90` (KKC_ThingDataManager::ComputeCVMeanValues).
//!
//! All x87 arithmetic runs at the caller's precision control; every
//! `fstp dword` is an explicit binary32 store. The two CRT `pow` calls go
//! through [`aa_numeric::crt_pow::ci_pow_x87`] (the ported `_CIpow`
//! `0x96D890` with its SSE2 body), whose environment input selects the path;
//! the x87 body is reported unsupported rather than approximated.
//!
//! Callers: `0x4618B0` (thing definition value, no modifier context, inputs
//! definition +288/+284/+228, the summed +28 list and +210) and `0x57E430`
//! (performer value with the `0x4CF110` modifier context).
use crate::{Error, Result};
use aa_body::F32;
use aa_numeric::{
    Binary, Precision,
    crt_pow::{self, CrtEnvironment},
};
use std::cmp::Ordering;

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}
fn st(value: &Binary) -> Result<F32> {
    Ok(F32(value.store_f32()?))
}
fn less(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Less
}
fn greater(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Greater
}
fn one() -> Binary {
    Binary::from_i32(1)
}
fn zero() -> Binary {
    Binary::from_i32(0)
}
fn pow(x: &Binary, y: &Binary, env: CrtEnvironment) -> Result<Binary> {
    Ok(crt_pow::ci_pow_x87(x, y, env)?.0)
}
/// Image constants read by `0x45E090` (`[0x9FBDB8]` 0.5f, `[0xA033D0]`
/// 2.24f, `[0xA033C4]` 2.16f, `[0xA033C0]` 29.01f, `[0xA033BC]` 0.25f,
/// `[0xA0145C]` 0.2f, `[0x9F2408]` 100f, binary64 `[0xA00C10]` 1.0 and
/// `[0xA033C8]` 0.10000000149011612).
const HALF: F32 = F32(0x3f00_0000);
const LOG_POWER_OFFSET: F32 = F32(0x400f_5c29);
const LOG_RATE_SCALE: F32 = F32(0x400a_3d71);
const LOG_ATTACK_BASE: F32 = F32(0x41e8_147b);
const QUARTER: F32 = F32(0x3e80_0000);
const LOG_SPEED_WEIGHT: F32 = F32(0x3e4c_cccd);
const HUNDRED: F32 = F32(0x42c8_0000);
const LOG_AREA_POWER: u64 = 0x3ff0_0000_0000_0000;
const LOG_RANGE_POWER: u64 = 0x3fb9_9999_a000_0000;
/// `0x45E6D0` addend `[0xA03448]` (2.0000999f).
const EXTENT_ADDEND: F32 = F32(0x4000_01a3);

/// ThingDataManager (`[0xB39F58]`) values written by [`mean_values`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeanValues {
    /// +74: maximum `0x45E6D0` value of flagged things.
    pub maximum_74: F32,
    /// +78 mean attack value.
    pub attack: F32,
    /// +7C mean attack rate.
    pub attack_rate: F32,
    /// +80 mean defense value.
    pub defense: F32,
    /// +84 mean movement value.
    pub movement: F32,
    /// +88 mean health value.
    pub health: F32,
    /// +8C mean lifetime value (health / offense).
    pub lifetime: F32,
    /// +90 mean offense value (attack - defense).
    pub offense: F32,
}

/// Settings used by `0x45E090` (pointer table `[0xB39AB0]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// 41 CombatDamageMinimum (+A4).
    pub damage_minimum: F32,
    /// 81 CombatValueRangePower (+144).
    pub range_power: F32,
    /// 82 CombatValueAreaPower (+148).
    pub area_power: F32,
    /// 83 CombatValueOffenseWeight (+14C).
    pub offense_weight: F32,
    /// 84 CombatValueDefenseWeight (+150).
    pub defense_weight: F32,
    /// 85 CombatValueMovementWeight (+154).
    pub movement_weight: F32,
    /// 86 CombatValueMovementDefault (+158).
    pub movement_default: F32,
}

/// Ability definition fields read through `*local`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbilityDefinition {
    /// +54 flags (bit 3 bombard range modifier, bit 7 excluded).
    pub flags_54: u32,
    /// +24 base range (`0x4BE910`).
    pub range_24: F32,
    /// +38 area.
    pub area_38: F32,
    /// +3C nonzero: projectile range modifier and ranged power getter.
    pub projectile_3c: bool,
    /// +64 base power (`0x4BE3F0`).
    pub power_64: F32,
}

/// Local ability definition (the `0x45E090` first argument).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbilityLocal {
    pub definition: AbilityDefinition,
    /// +4.
    pub value_04: F32,
    /// +18 recharge time (`0x4BE4B0`).
    pub recharge_18: F32,
    /// `0x4BE680`: with +40 nonzero and the +3C vector nonempty, its first
    /// entry's +10; otherwise the getter returns 0.
    pub first_entry_10: Option<F32>,
}

/// `0x45E090` arguments 2..6 (see the module callers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitInputs {
    pub defense: F32,
    pub health: F32,
    pub speed: F32,
    pub additive_a: F32,
    pub additive_b: F32,
}

/// Modifier-context getters and runtime state used by `0x45E090`.
pub trait World {
    /// SVar "CombatValueRangedMinimum" (`0x4B2E60`, cached in `[0xB3A668]`).
    fn ranged_minimum(&mut self) -> Result<F32>;
    /// Native `0x45E090` unconditionally binds the cached SVar pointer
    /// `[0xB3A664]` before choosing a defense weight. This is separate from
    /// reading its scalar: the pointer is initialized even when the ordinary
    /// `[Settings+150]` weight is selected. Preserve the native failure prefix.
    fn bind_defense_weight_ranged(&mut self) -> Result<()>;
    /// SVar "CombatValueDefenseWeightRanged" (`[0xB3A664]`).
    fn defense_weight_ranged(&mut self) -> Result<F32>;
    /// `0x501D50`.
    fn modify_projectile_range(&mut self, modifier: u32, base: F32) -> Result<Binary>;
    /// `0x501DB0`.
    fn modify_bombard_range(&mut self, modifier: u32, base: F32) -> Result<Binary>;
    /// `0x501AA0` (ranged) / `0x501A30` (melee); aa-body `ModifierState::power`.
    fn modify_power(&mut self, modifier: u32, base: F32, ranged: bool) -> Result<Binary>;
    /// `0x501560`; aa-body `ModifierState::recharge`.
    fn modify_recharge(&mut self, modifier: u32, base: F32) -> Result<Binary>;
    /// Manager +4C stream virtual +14 (diagnostic CSV enabled).
    fn log_enabled(&mut self) -> Result<bool>;
}

/// `0x4BE910`.
fn range(def: &AbilityDefinition, modifier: Option<u32>, world: &mut impl World) -> Result<Binary> {
    let mut result = n(def.range_24)?;
    if let Some(m) = modifier {
        let mut base = def.range_24;
        if def.projectile_3c {
            result = world.modify_projectile_range(m, base)?;
            base = st(&result)?;
        }
        if def.flags_54 & 8 != 0 {
            result = world.modify_bombard_range(m, base)?;
        }
    }
    Ok(result)
}
/// `0x4BE3F0`.
fn power(def: &AbilityDefinition, modifier: Option<u32>, world: &mut impl World) -> Result<Binary> {
    match modifier {
        None => n(def.power_64),
        Some(m) => world.modify_power(m, def.power_64, def.projectile_3c),
    }
}
/// `0x4BE4B0`.
fn recharge(local: &AbilityLocal, modifier: Option<u32>, world: &mut impl World) -> Result<Binary> {
    match modifier {
        None => n(local.recharge_18),
        Some(m) => world.modify_recharge(m, local.recharge_18),
    }
}
/// `0x4BE680`.
fn first_entry(local: &AbilityLocal) -> Result<Binary> {
    n(local.first_entry_10.unwrap_or(F32::ZERO))
}
/// `f32(0x4BE680 + local+4)` raised to the recharge getter when that is not
/// below it. `reload_stored` selects the diagnostic variant, which stores
/// the second getter result to binary32 (the main path keeps it extended).
fn cycle(
    local: &AbilityLocal,
    modifier: Option<u32>,
    world: &mut impl World,
    p: Precision,
    reload_stored: bool,
) -> Result<Binary> {
    let floor = st(&first_entry(local)?.add(&n(local.value_04)?, p)?)?;
    if less(&recharge(local, modifier, world)?, &n(floor)?) {
        return n(floor);
    }
    let value = recharge(local, modifier, world)?;
    if reload_stored {
        n(st(&value)?)
    } else {
        Ok(value)
    }
}

/// `0x856F00` CSV row (format [`LOG_FORMAT`]) after the definition name:
/// binary64 bits in argument order (alternate value, value, percent, weighted
/// offense, defense factor, movement, additive a, additive b, power, cycle,
/// range, area, health, defense, speed, offense, lifetime).
pub const LOG_FORMAT: &str =
    "%s,%.1f,%.1f,%.0f%%,%.2f,%.2f,%.2f,%g,%g,%g,%g,%g,%g,%g,%g,%g,%.1f,%.1f\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evaluation {
    pub value: F32,
    pub log: Option<[u64; 17]>,
}

fn d(value: F32) -> Result<u64> {
    Ok(crt_pow::store_binary64(&n(value)?)?)
}
fn dx(value: &Binary) -> Result<u64> {
    Ok(crt_pow::store_binary64(value)?)
}

/// Native `0x45E090(local, defense, health, speed, additive_a, additive_b,
/// modifier)`; `value` is the returned binary32.
#[allow(clippy::too_many_arguments)]
pub fn ability_value(
    local: &AbilityLocal,
    unit: &UnitInputs,
    modifier: Option<u32>,
    means: &MeanValues,
    settings: &Settings,
    env: CrtEnvironment,
    world: &mut impl World,
    p: Precision,
) -> Result<Evaluation> {
    ability_value_with_environment(local, unit, modifier, means, settings, || Ok(env), world, p)
}

/// The same45E090 algorithm with actual current-thread control sampling at
/// each reached CRT pow. Excluded paths do not request an environment;
/// nonpositive-power paths request one only for the diagnostic CSV branch.
/// Failure preserves the preceding World callback prefix.
#[allow(clippy::too_many_arguments)]
pub fn ability_value_with_environment(
    local: &AbilityLocal,
    unit: &UnitInputs,
    modifier: Option<u32>,
    means: &MeanValues,
    settings: &Settings,
    mut environment: impl FnMut() -> Result<CrtEnvironment>,
    world: &mut impl World,
    p: Precision,
) -> Result<Evaluation> {
    let def = &local.definition;
    if def.flags_54 & 0x80 != 0 {
        return Ok(Evaluation {
            value: F32::ZERO,
            log: None,
        });
    }
    let ranged_minimum = world.ranged_minimum()?;
    let ranged = !less(&range(def, modifier, world)?, &n(ranged_minimum)?);
    let mut offense = F32::ZERO;
    if greater(&n(def.power_64)?, &zero()) {
        let mut damage = st(&power(def, modifier, world)?.sub(&n(means.defense)?, p)?)?;
        if less(&n(damage)?, &n(settings.damage_minimum)?) {
            damage = settings.damage_minimum;
        }
        let cycle = cycle(local, modifier, world, p, false)?;
        let rate = st(&n(damage)?.div(&cycle, p)?)?;
        let reach = range(def, modifier, world)?;
        let range_factor = st(&pow(&reach, &n(settings.range_power)?, environment()?)?)?;
        let area = n(def.area_38)?.mul(&n(HALF)?, p)?.add(&one(), p)?;
        let area_factor = pow(&area, &n(settings.area_power)?, environment()?)?;
        offense = st(&area_factor.mul(&n(range_factor)?, p)?.mul(&n(rate)?, p)?)?;
    }
    let mut lifetime = F32::ZERO;
    if greater(&n(unit.health)?, &zero()) {
        let divisor = n(means.attack)?.sub(&n(unit.defense)?, p)?;
        lifetime = st(&n(unit.health)?.div(&divisor, p)?)?;
    }
    let mut movement = st(&n(unit.speed)?
        .div(&n(means.movement)?, p)?
        .sub(&one(), p)?
        .mul(&n(settings.movement_weight)?, p)?
        .add(&one(), p)?)?;
    if !greater(&n(movement)?, &zero()) {
        movement = settings.movement_default;
    }
    let weighted_offense = st(&n(offense)?
        .sub(&n(means.offense)?, p)?
        .mul(&n(settings.offense_weight)?, p)?
        .add(&n(means.offense)?, p)?)?;
    let life_ratio = st(&n(lifetime)?.div(&n(means.lifetime)?, p)?)?;
    world.bind_defense_weight_ranged()?;
    let weight = if less(&n(life_ratio)?, &one()) && ranged {
        world.defense_weight_ranged()?
    } else {
        settings.defense_weight
    };
    let defense_factor = st(&n(life_ratio)?
        .sub(&one(), p)?
        .mul(&n(weight)?, p)?
        .add(&one(), p)?)?;
    let value = st(&n(defense_factor)?
        .mul(&n(weighted_offense)?, p)?
        .mul(&n(movement)?, p)?
        .add(&n(unit.additive_a)?, p)?
        .add(&n(unit.additive_b)?, p)?)?;
    let log = if world.log_enabled()? {
        let shifted = power(def, modifier, world)?.sub(&n(LOG_POWER_OFFSET)?, p)?;
        let scale = if less(&shifted, &one()) {
            F32::ONE
        } else {
            st(&shifted)?
        };
        let cycle_value = st(&cycle(local, modifier, world, p, true)?)?;
        let reach = st(&range(def, modifier, world)?)?;
        let area = n(def.area_38)?.mul(&n(HALF)?, p)?.add(&one(), p)?;
        let area_factor = pow(&area, &Binary::from_f64_bits(LOG_AREA_POWER)?, environment()?)?;
        let range_factor = pow(&n(reach)?, &Binary::from_f64_bits(LOG_RANGE_POWER)?, environment()?)?;
        let factors = area_factor.mul(&range_factor, p)?;
        let rate = n(LOG_RATE_SCALE)?
            .div(&n(cycle_value)?, p)?
            .mul(&n(scale)?, p)?;
        let survival = n(unit.health)?.div(&n(LOG_ATTACK_BASE)?.sub(&n(unit.defense)?, p)?, p)?;
        let speed = n(unit.speed)?
            .mul(&n(QUARTER)?, p)?
            .sub(&one(), p)?
            .mul(&n(LOG_SPEED_WEIGHT)?, p)?
            .add(&one(), p)?;
        let alternate = st(&factors
            .mul(&rate, p)?
            .add(&survival, p)?
            .mul(&speed, p)?
            .add(&n(unit.additive_a)?, p)?
            .add(&n(unit.additive_b)?, p)?)?;
        let range_again = range(def, modifier, world)?;
        let power_again = power(def, modifier, world)?;
        let percent = n(value)?.div(&n(alternate)?, p)?.mul(&n(HUNDRED)?, p)?;
        Some([
            d(alternate)?,
            d(value)?,
            dx(&percent)?,
            d(weighted_offense)?,
            d(defense_factor)?,
            d(movement)?,
            d(unit.additive_a)?,
            d(unit.additive_b)?,
            dx(&power_again)?,
            d(cycle_value)?,
            dx(&range_again)?,
            d(def.area_38)?,
            d(unit.health)?,
            d(unit.defense)?,
            d(unit.speed)?,
            d(offense)?,
            d(lifetime)?,
        ])
    } else {
        None
    };
    Ok(Evaluation { value, log })
}

/// `0x45E6D0` inputs of a thing definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extent {
    /// +468 (signed count).
    pub count_468: i32,
    /// +478.
    pub base_478: F32,
    /// `[[[+464]+8]+1C]+3C`, read only when the count is at least 1.
    pub nested_3c: F32,
}
impl Extent {
    /// Native `0x45E6D0` (extended result).
    pub fn value(&self, p: Precision) -> Result<Binary> {
        if self.count_468 < 1 {
            return n(self.base_478);
        }
        Ok(n(self.nested_3c)?
            .add(&n(self.base_478)?, p)?
            .add(&n(EXTENT_ADDEND)?, p)?)
    }
}

/// One thing definition of the manager +68 list, in list order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThingInput {
    /// +174 flags.
    pub flags_174: u32,
    pub extent: Extent,
    /// +1EC/+1F0 local abilities in order.
    pub abilities: Vec<AbilityLocal>,
    /// +288 (`0x45D760`, no modifier context).
    pub defense_288: F32,
    /// +228 (`0x45D6F0`).
    pub movement_228: F32,
    /// +284 (`0x45D730`).
    pub health_284: F32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeanOutcome {
    pub means: MeanValues,
    /// The `m_mean_attack_value > m_mean_defense_value` check failed while
    /// `[0xB3B4F8]` was clear: the caller runs `0x855D30` (ThingData.cpp line
    /// 0x24B) and stores its result to `[0xB3B4F8]`. The values do not depend
    /// on it. The seven `0x404F60` log lines print the stored means.
    pub assertion: bool,
}

/// `0x869D30` bisection with comparator `0x4BB020` (key > element -> -1,
/// otherwise 1; never 0), then `0x4BB850` insertion at the found index: a
/// descending order with new equal keys placed after existing ones.
fn insert(values: &mut Vec<F32>, key: F32) -> Result<()> {
    let k = n(key)?;
    let (mut lo, mut hi) = (0usize, values.len());
    while lo < hi {
        let mid = (lo + hi) / 2;
        if greater(&k, &n(values[mid])?) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    values.insert(lo, key);
    Ok(())
}

/// `_ftol2` (`0x96CC4C`) of `PC(count * discard)`.
fn discarded(count: usize, discard: F32, p: Precision) -> Result<usize> {
    let count = i32::try_from(count).map_err(|_| Error::Invalid("thing count exceeds int32"))?;
    let k = Binary::from_i32(count)
        .mul(&n(discard)?, p)?
        .truncate_i32()?;
    usize::try_from(k).map_err(|_| Error::Invalid("negative CombatValueMeanValuesDiscardPercent"))
}

/// Trimmed f32 sums over `[k, len-k)` with `1/(len-2k)` scaling.
fn trimmed(columns: &[&Vec<F32>], k: usize, p: Precision) -> Result<Vec<F32>> {
    let len = columns[0].len();
    if k >= len || len - k <= k {
        return Err(Error::Invalid(
            "combat value means over an empty trimmed range",
        ));
    }
    let mut sums = vec![F32::ZERO; columns.len()];
    for i in k..len - k {
        for (sum, column) in sums.iter_mut().zip(columns) {
            *sum = st(&n(column[i])?.add(&n(*sum)?, p)?)?;
        }
    }
    let count =
        i32::try_from(len - 2 * k).map_err(|_| Error::Invalid("thing count exceeds int32"))?;
    let inverse = one().div(&Binary::from_i32(count), p)?;
    sums.iter()
        .map(|sum| st(&inverse.mul(&n(*sum)?, p)?))
        .collect()
}

/// Native `0x4BBF90`. `discard` is setting 87
/// CombatValueMeanValuesDiscardPercent (+15C); `assertion_suppressed` is
/// `[0xB3B4F8]`. Empty trimmed ranges (native divide by zero/negative count)
/// and a negative discard (native out-of-bounds reads) are rejected.
pub fn mean_values(
    things: &[ThingInput],
    discard: F32,
    assertion_suppressed: bool,
    p: Precision,
) -> Result<MeanOutcome> {
    let mut means = MeanValues {
        maximum_74: F32::ZERO,
        attack: F32::ZERO,
        attack_rate: F32::ZERO,
        defense: F32::ZERO,
        movement: F32::ZERO,
        health: F32::ZERO,
        lifetime: F32::ZERO,
        offense: F32::ZERO,
    };
    let (mut attack, mut rate) = (Vec::new(), Vec::new());
    let (mut defense, mut movement, mut health) = (Vec::new(), Vec::new(), Vec::new());
    for thing in things {
        let flags = thing.flags_174;
        if flags >> 14 & 1 != 0 && greater(&thing.extent.value(p)?, &n(means.maximum_74)?) {
            means.maximum_74 = st(&thing.extent.value(p)?)?;
        }
        let combat = flags >> 5 & 1 != 0
            && flags & 1 != 0
            && flags >> 3 & 1 != 0
            && (flags >> 17 & 1 == 0 || flags >> 8 & 1 == 0);
        if !combat {
            continue;
        }
        for local in &thing.abilities {
            if !greater(&n(local.definition.power_64)?, &zero()) {
                continue;
            }
            insert(&mut attack, local.definition.power_64)?;
            let floor = st(&first_entry(local)?.add(&n(local.value_04)?, p)?)?;
            let value = if less(&n(local.recharge_18)?, &n(floor)?) {
                floor
            } else {
                local.recharge_18
            };
            insert(&mut rate, value)?;
        }
        insert(&mut defense, thing.defense_288)?;
        insert(&mut movement, thing.movement_228)?;
        insert(&mut health, thing.health_284)?;
    }
    let k = discarded(attack.len(), discard, p)?;
    let first = trimmed(&[&attack, &rate], k, p)?;
    means.attack = first[0];
    means.attack_rate = first[1];
    let k = discarded(defense.len(), discard, p)?;
    let second = trimmed(&[&defense, &movement, &health], k, p)?;
    means.defense = second[0];
    means.movement = second[1];
    means.health = second[2];
    let assertion = !assertion_suppressed && !greater(&n(means.attack)?, &n(means.defense)?);
    let offense = n(means.attack)?.sub(&n(means.defense)?, p)?;
    means.lifetime = st(&n(means.health)?.div(&offense, p)?)?;
    means.offense = st(&offense)?;
    Ok(MeanOutcome { means, assertion })
}

/// One entry of a performer ability table (`+8` local definition, `+18`
/// enabled byte).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableEntry {
    pub enabled: bool,
    pub local: AbilityLocal,
}

/// Inputs of `0x57E430` read from the performer's actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerformerInputs {
    /// Actor +78 Properties component: its property list (+10/+14) entries'
    /// `[+C]+28` values in order. `None` when the component is absent.
    pub property_values: Option<Vec<F32>>,
    /// `0x4CF110`: the component's modifier (+1C), present with the component.
    pub modifier: Option<u32>,
    /// Actor +60 body: (+18 defense, +14 health).
    pub body: Option<(F32, F32)>,
    /// Actor +6C component +28.
    pub speed: Option<F32>,
    /// Actor definition +210.
    pub bonus_210: F32,
    /// Performer +14 ability table.
    pub abilities: Vec<TableEntry>,
    /// Actor +B0 component's +10 ability table, when the component exists.
    pub secondary: Option<Vec<TableEntry>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerformerValue {
    /// Performer +60.
    pub value: F32,
    /// `0x856F00` rows of the evaluations, in call order.
    pub logs: Vec<[u64; 17]>,
}

/// Native `0x57E430`: the performer's combat value, the maximum `0x45E090`
/// result over the enabled entries of both ability tables (starting from 0;
/// the property sum accumulates extended and is stored once).
#[allow(clippy::too_many_arguments)]
pub fn performer_value(
    inputs: &PerformerInputs,
    means: &MeanValues,
    settings: &Settings,
    env: CrtEnvironment,
    world: &mut impl World,
    p: Precision,
) -> Result<PerformerValue> {
    let mut sum = zero();
    for value in inputs.property_values.iter().flatten() {
        sum = sum.add(&n(*value)?, p)?;
    }
    let sum = st(&sum)?;
    let (defense, health) = inputs.body.unwrap_or((F32::ZERO, F32::ZERO));
    let unit = UnitInputs {
        defense,
        health,
        speed: inputs.speed.unwrap_or(F32::ZERO),
        additive_a: sum,
        additive_b: inputs.bonus_210,
    };
    let mut best = F32::ZERO;
    let mut logs = Vec::new();
    let tables = [Some(&inputs.abilities), inputs.secondary.as_ref()];
    for entry in tables.into_iter().flatten().flatten() {
        if !entry.enabled {
            continue;
        }
        let out = ability_value(
            &entry.local,
            &unit,
            inputs.modifier,
            means,
            settings,
            env,
            world,
            p,
        )?;
        logs.extend(out.log);
        if !less(&n(out.value)?, &n(best)?) {
            best = out.value;
        }
    }
    Ok(PerformerValue { value: best, logs })
}

/// Thing definition fields read by `0x4618B0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionThing {
    /// +174 flags (bit0 body values, bit3 speed, bit5 combat value, bit6
    /// +1D4 list, bit7 +2F0 children, bit10 +35C weighted list).
    pub flags_174: u32,
    /// +1D4 list items' +28, in list order.
    pub list_1d4: Vec<F32>,
    pub defense_288: F32,
    pub health_284: F32,
    pub speed_228: F32,
    pub bonus_210: F32,
    /// +1EC/+1F0 local abilities.
    pub abilities: Vec<AbilityLocal>,
    /// +2F0 children in index order (`[+C][[+8]+28[i]]`, i < `[+8]+2C`);
    /// `None` for a null slot.
    pub children_2f0: Vec<Option<u32>>,
    /// +35C list: (node+0 definition, node+8 integer count) in list order.
    pub weighted_35c: Vec<(u32, i32)>,
}

/// Definition float fields written by `0x4618B0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefinitionField {
    /// +1E4 combat value (`-1.0` marks "not computed yet" for the recursion).
    Value1E4,
    /// +30C sum of the +2F0 children's values.
    Children30C,
    /// +36C count-weighted sum of the +35C list's values.
    Weighted36C,
}

/// `-1.0f` (`0xBF800000`), the uncomputed +1E4 marker tested by `0x4618B0`.
pub const UNCOMPUTED: F32 = F32(0xbf80_0000);

pub trait DefinitionWorld: World {
    fn definition(&mut self, id: u32) -> Result<DefinitionThing>;
    fn flags_174(&mut self, id: u32) -> Result<u32>;
    fn field(&mut self, id: u32, field: DefinitionField) -> Result<F32>;
    fn set_field(&mut self, id: u32, field: DefinitionField, value: F32) -> Result<()>;
    /// Diagnostic CSV prefix when the stream is enabled: names (+8, +10) and
    /// the `%g%c` resource tables (+324, +330) via `0x405C90`/`0x856F00`.
    fn log_prefix(&mut self, id: u32) -> Result<()>;
    /// A `0x45E090` diagnostic row ([`LOG_FORMAT`]) in call order.
    fn log_row(&mut self, id: u32, row: [u64; 17]) -> Result<()>;
    /// `0x4BB590` prologue: SVar "CombatValueCSV" (`0x4B2E60` kind 4, cached
    /// in `[0xB3B4F0]`); when nonempty, open it on the manager +4C stream
    /// (virtual +20) and write the header lines including the stored means.
    fn csv_begin(&mut self, means: &MeanValues) -> Result<()>;
    /// `0x4BB590` epilogue when the stream reports enabled: virtual +28.
    fn csv_close(&mut self) -> Result<()>;
}

/// Native `0x4BB590` (KKC_ThingDataManager::ComputeCVActorDataValues):
/// `0x4618B0` for every manager +68 list definition still marked
/// [`UNCOMPUTED`], in list order, inside the optional diagnostic CSV.
pub fn actor_data_values(
    definitions: &[u32],
    means: &MeanValues,
    settings: &Settings,
    env: CrtEnvironment,
    world: &mut impl DefinitionWorld,
    p: Precision,
) -> Result<()> {
    world.csv_begin(means)?;
    for &id in definitions {
        if world.field(id, DefinitionField::Value1E4)? == UNCOMPUTED {
            definition_value(id, means, settings, env, world, p)?;
        }
    }
    if world.log_enabled()? {
        world.csv_close()?;
    }
    Ok(())
}

/// Native `0x4618B0` (thiscall on a thing definition), recursive over
/// uncomputed children through the world.
pub fn definition_value(
    id: u32,
    means: &MeanValues,
    settings: &Settings,
    env: CrtEnvironment,
    world: &mut impl DefinitionWorld,
    p: Precision,
) -> Result<()> {
    let thing = world.definition(id)?;
    let flags = thing.flags_174;
    world.set_field(id, DefinitionField::Value1E4, F32::ZERO)?;
    if flags >> 5 & 1 != 0 {
        let mut sum = F32::ZERO;
        if flags >> 6 & 1 != 0 && !thing.list_1d4.is_empty() {
            let mut acc = zero();
            for value in &thing.list_1d4 {
                acc = acc.add(&n(*value)?, p)?;
            }
            sum = st(&acc)?;
        }
        let (defense, health) = if flags & 1 != 0 {
            (thing.defense_288, thing.health_284)
        } else {
            (F32::ZERO, F32::ZERO)
        };
        let unit = UnitInputs {
            defense,
            health,
            speed: if flags >> 3 & 1 != 0 {
                thing.speed_228
            } else {
                F32::ZERO
            },
            additive_a: sum,
            additive_b: thing.bonus_210,
        };
        let mut best = F32::ZERO;
        for local in &thing.abilities {
            if world.log_enabled()? {
                world.log_prefix(id)?;
            }
            let out = ability_value(local, &unit, None, means, settings, env, world, p)?;
            if let Some(row) = out.log {
                world.log_row(id, row)?;
            }
            if !greater(&n(best)?, &n(out.value)?) {
                best = out.value;
            }
        }
        world.set_field(id, DefinitionField::Value1E4, best)?;
    }
    world.set_field(id, DefinitionField::Children30C, F32::ZERO)?;
    if flags >> 7 & 1 != 0 {
        for child in thing.children_2f0.iter().flatten() {
            if world.flags_174(*child)? >> 5 & 1 == 0 {
                continue;
            }
            if world.field(*child, DefinitionField::Value1E4)? == UNCOMPUTED {
                definition_value(*child, means, settings, env, world, p)?;
            }
            let total = n(world.field(id, DefinitionField::Children30C)?)?
                .add(&n(world.field(*child, DefinitionField::Value1E4)?)?, p)?;
            world.set_field(id, DefinitionField::Children30C, st(&total)?)?;
        }
    }
    world.set_field(id, DefinitionField::Weighted36C, F32::ZERO)?;
    if flags >> 10 & 1 != 0 {
        for (child, count) in &thing.weighted_35c {
            if world.flags_174(*child)? >> 5 & 1 == 0 {
                continue;
            }
            if world.field(*child, DefinitionField::Value1E4)? == UNCOMPUTED {
                definition_value(*child, means, settings, env, world, p)?;
            }
            let total = Binary::from_i32(*count)
                .mul(&n(world.field(*child, DefinitionField::Value1E4)?)?, p)?
                .add(&n(world.field(id, DefinitionField::Weighted36C)?)?, p)?;
            world.set_field(id, DefinitionField::Weighted36C, st(&total)?)?;
        }
    }
    Ok(())
}
