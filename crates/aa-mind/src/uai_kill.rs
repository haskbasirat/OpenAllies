//! Original unit (Element) kill decision `5340C0` and `KKC_UAIStateKill::Think`
//! `535240` (Source/Sim/Tai/UAI.cpp, state `uai_kill`).
//!
//! `5340C0` picks the best enabled ability of the unit against a target (or a
//! point): rules-of-engagement back-off for a primary element whose company
//! takes too much damage (aa_combat::roe::rate `570E30`), per-ability decision
//! kind/status, utility score (`52ED30`, crate::decision_score) plus the
//! attacker bonus, then the original tie-breaks (ability values, ranges and a
//! world-RNG coin flip). `535240` runs it and hands the result to
//! `ResultAbilityDecision` (`5307A0`, crate::ability_decision::apply).
use crate::{
    Arena, Error, Result, StateId, think_base, within_arrival,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// The native 7-word decision record (`+0` ability, `+4` kind, `+8` status,
/// `+C` score, `+10` target, `+14` squared distance, `+18` target priority).
/// Handles are opaque; 0 means null.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillDecision {
    pub ability: u32,
    pub kind: u32,
    pub status: u32,
    pub score: u32,
    pub target: u32,
    pub distance2: u32,
    pub priority: i32,
}
impl KillDecision {
    /// Initial record written by both callers.
    pub fn initial() -> Self {
        Self {
            ability: 0,
            kind: 1,
            status: 3,
            score: 0xbf80_0000,
            target: 0,
            distance2: 0x7f7f_ffff,
            priority: 0,
        }
    }
}

/// One entry of the performer ability vector (performer +14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ability {
    pub handle: u32,
    /// Ability +18 byte: disabled abilities are skipped by the iterator.
    pub enabled: bool,
    /// Definition +54 flags.
    pub flags54: u32,
    /// Definition +28 byte.
    pub byte28: u8,
}

/// Targeting context built by `4C8300` (kind 1, or 3 for point decisions).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetContext {
    pub kind: u32,
    pub target: u32,
    pub point: Option<[u32; 2]>,
}

/// Services of `5340C0`, called in native order.
pub trait ChooseWorld {
    /// Mind +1AC retry time and world +E0 time.
    fn retry_time(&mut self) -> Result<u32>;
    fn set_retry_time(&mut self, bits: u32) -> Result<()>;
    fn world_time(&mut self) -> Result<u32>;
    /// Actor +60 body and +80 membership present and `570650(membership)`.
    fn primary_member(&mut self) -> Result<bool>;
    /// `57A730` on the organization component.
    fn member_count(&mut self) -> Result<i32>;
    /// `4CF290` organization anchor position.
    fn organization_anchor(&mut self) -> Result<[u32; 2]>;
    /// Mind actor +20/+24.
    fn actor_position(&mut self) -> Result<[u32; 2]>;
    /// Actor +118 engaged list present.
    fn engaged(&mut self) -> Result<bool>;
    /// `570E30(membership)` (aa_combat::roe::rate), unstored.
    fn roe_rate(&mut self) -> Result<Binary>;
    /// Body +10 / +14.
    fn health(&mut self) -> Result<[u32; 2]>;
    /// Actor definition +2D0 and +2D4.
    fn roe_ratio_threshold(&mut self) -> Result<u32>;
    fn roe_value_threshold(&mut self) -> Result<u32>;
    /// `5617B0(rate)` on the body, unstored.
    fn roe_value(&mut self, rate_bits: u32) -> Result<Binary>;
    /// SVar at `[B39AB0]+19C` (ROE retry delay).
    fn roe_retry_delay(&mut self) -> Result<u32>;
    /// `4D4420(actor)` on the target.
    fn target_attacks_us(&mut self, target: u32) -> Result<bool>;
    /// SVar at `[B39AB0]+1C4` (attacker bonus).
    fn attacker_bonus(&mut self) -> Result<u32>;
    /// `4C8F20(&context, 1)` on the ability: decision kind.
    fn ability_kind(&mut self, ability: u32, context: &TargetContext) -> Result<u32>;
    /// `52ECC0(kind)` on the Mind: decision status.
    fn decision_status(&mut self, kind: u32) -> Result<u32>;
    /// Membership organization present and `578E50()` on its component.
    fn organization_busy(&mut self) -> Result<bool>;
    /// `531FC0(definition, target)` on the Mind.
    fn can_attack(&mut self, ability: u32, target: u32) -> Result<bool>;
    /// `52ED30(&record)` on the Mind (crate::decision_score), unstored.
    fn score(&mut self, record: &KillDecision) -> Result<Binary>;
    /// `4C83E0(actor)` on the ability, unstored.
    fn point_score(&mut self, ability: u32) -> Result<Binary>;
    /// `4CF110` actor player.
    fn player(&mut self) -> Result<u32>;
    /// `4C81C0()` and `4C8270(player)` on an ability, unstored.
    fn ability_range(&mut self, ability: u32) -> Result<Binary>;
    fn ability_value(&mut self, ability: u32, player: u32) -> Result<Binary>;
    /// One `85D840` draw on the world RNG (`4F02B0`).
    fn world_random(&mut self) -> Result<u32>;
    /// Mind virtual +1AC(target).
    fn priority(&mut self, target: u32) -> Result<i32>;
    /// Target +20/+24.
    fn target_position(&mut self, target: u32) -> Result<[u32; 2]>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn bit(flags: u32, index: u32) -> bool {
    (flags >> index) & 1 != 0
}

/// Complete `5340C0(target, point, out, by_point)`; returns the status.
pub fn choose(
    target: Option<u32>,
    point: Option<[u32; 2]>,
    mut out: Option<&mut KillDecision>,
    by_point: bool,
    abilities: &[Ability],
    world: &mut impl ChooseWorld,
    p: Precision,
) -> Result<u32> {
    let target_word = target.unwrap_or(0);
    let mut best = KillDecision::initial();
    if let Some(o) = out.as_deref_mut() {
        o.target = target_word;
    }
    let fail = |out: Option<&mut KillDecision>| {
        if let Some(o) = out {
            o.status = 3;
        }
        Ok(3)
    };
    if target.is_none() && point.is_none() {
        return fail(out);
    }
    let retry = world.retry_time()?;
    let now = world.world_time()?;
    if host(retry) > host(now) {
        return fail(out);
    }
    if world.primary_member()? {
        let count = world.member_count()?;
        let anchor = world.organization_anchor()?;
        if count > 1 && !within_arrival(world.actor_position()?, anchor, p)? && world.engaged()? {
            let rate = store(world.roe_rate()?)?;
            let [current, maximum] = world.health()?;
            let ratio = num(n(current)?.div(&n(maximum)?, p))?;
            let backoff = if ratio.compare(&n(world.roe_ratio_threshold()?)?) == Ordering::Less {
                true
            } else {
                let threshold = world.roe_value_threshold()?;
                world.roe_value(rate)?.compare(&n(threshold)?) == Ordering::Less
            };
            if backoff {
                if let Some(o) = out {
                    o.status = 3;
                }
                let delay = world.roe_retry_delay()?;
                let now = world.world_time()?;
                world.set_retry_time(store(num(n(delay)?.add(&n(now)?, p))?)?)?;
                return Ok(3);
            }
        }
    }
    let bonus = match target {
        Some(t) if world.target_attacks_us(t)? => world.attacker_bonus()?,
        _ => 0,
    };
    let context = TargetContext {
        kind: if by_point { 3 } else { 1 },
        target: target_word,
        point,
    };
    let mut index = abilities
        .iter()
        .position(|a| a.enabled)
        .unwrap_or(abilities.len());
    while index < abilities.len() {
        let ability = abilities[index];
        'candidate: {
            let mut cand = KillDecision {
                ability: ability.handle,
                target: target_word,
                ..KillDecision::initial()
            };
            if by_point && !bit(ability.flags54, 3) {
                break 'candidate;
            }
            cand.kind = world.ability_kind(ability.handle, &context)?;
            let mut status = world.decision_status(cand.kind)?;
            cand.status = status;
            if status == 3 {
                break 'candidate;
            }
            if world.organization_busy()? && ability.byte28 & 1 != 0 && status == 1 {
                break 'candidate;
            }
            if by_point && point.is_some() {
                if status != 1 {
                    let o = out.ok_or_else(|| {
                        Error::World("5340C0 point decision without an output record".into())
                    })?;
                    *o = cand;
                    return Ok(status);
                }
                cand.score = store(world.point_score(ability.handle)?)?;
                if best.ability == 0 || host(cand.score) < host(best.score) {
                    best = cand;
                }
                break 'candidate;
            }
            if bit(ability.flags54, 2)
                && bit(ability.flags54, 17)
                && status == 0
                && world.can_attack(ability.handle, target_word)?
            {
                status = 1;
                cand.status = 1;
            }
            cand.score = store(world.score(&cand)?)?;
            if bit(ability.flags54, 6) && status == 0 {
                cand.status = 1;
                let alternative = world.score(&cand)?;
                cand.status = status;
                if alternative.compare(&n(cand.score)?) == Ordering::Greater {
                    status = 1;
                    cand.score = store(alternative)?;
                    cand.status = 1;
                }
            }
            cand.score = store(num(n(cand.score)?.add(&n(bonus)?, p))?)?;
            if best.ability == 0 {
                best = cand;
                break 'candidate;
            }
            if host(cand.score) < host(best.score) {
                break 'candidate;
            }
            if host(cand.score) > host(best.score) {
                best = cand;
                break 'candidate;
            }
            let player = world.player()?;
            let take = if status == 2 {
                let mine = store(world.ability_value(best.ability, player)?)?;
                let theirs = world.ability_value(ability.handle, player)?;
                num(n(mine)?.sub(&theirs, p))?.compare(&Binary::from_i32(0)) == Ordering::Less
            } else {
                let mine = store(world.ability_range(best.ability)?)?;
                let theirs = world.ability_range(ability.handle)?;
                let difference = num(n(mine)?.sub(&theirs, p))?;
                match difference.compare(&Binary::from_i32(0)) {
                    Ordering::Greater => false,
                    Ordering::Less => true,
                    Ordering::Equal => {
                        let mine = n(store(world.ability_value(best.ability, player)?)?)?;
                        let theirs = world.ability_value(ability.handle, player)?;
                        match mine.compare(&theirs) {
                            Ordering::Less => false,
                            Ordering::Greater => true,
                            Ordering::Equal => world.world_random()? & 1 != 0,
                        }
                    }
                }
            };
            if take {
                best = cand;
            }
        }
        index += 1;
        while index < abilities.len() && !abilities[index].enabled {
            index += 1;
        }
    }
    let [ax, ay] = world.actor_position()?;
    let (dx, dy) = match target {
        Some(t) => {
            best.priority = world.priority(t)?;
            if best.target == 0 {
                best.target = t;
            }
            let [tx, ty] = world.target_position(t)?;
            (num(n(ax)?.sub(&n(tx)?, p))?, num(n(ay)?.sub(&n(ty)?, p))?)
        }
        None => {
            let [px, py] =
                point.ok_or_else(|| Error::World("5340C0 without target or point".into()))?;
            (num(n(ax)?.sub(&n(px)?, p))?, num(n(ay)?.sub(&n(py)?, p))?)
        }
    };
    best.distance2 = store(num(num(dy.mul(&dy, p))?.add(&num(dx.mul(&dx, p))?, p))?)?;
    if let Some(o) = out {
        *o = best;
    }
    Ok(best.status)
}

/// Performer fields read by `535240`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformerState {
    /// `4DB0F0`: the performer is its actor's current performer (+E4).
    pub current: bool,
    /// Performer +10 stage (4 = finished).
    pub stage: u32,
}

/// Services of `535240`.
pub trait KillThinkWorld {
    /// Actor +74 performer state (`None` when absent).
    fn performer(&mut self) -> Result<Option<PerformerState>>;
    /// GTID registry lookup of state+10.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// `4D4FF0(target, 1, 0)` on the actor (0 = allowed).
    fn relation(&mut self, target: u32) -> Result<i32>;
    /// `5340C0(target, null, &record, 0)` ([`choose`]).
    fn choose(&mut self, target: u32, record: &mut KillDecision) -> Result<u32>;
    /// `5307A0(&record, 0)` (crate::ability_decision::apply).
    fn apply(&mut self, record: &mut KillDecision) -> Result<()>;
    /// Actor virtual +108(&scratch) then target virtual +88(position).
    fn in_range(&mut self, target: u32) -> Result<bool>;
    /// Performer `57DD90(1, target, &scratch)` / `57DEA0()`.
    fn approach(&mut self, target: u32) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    /// Mind virtual +140(gtid): target lost.
    fn target_lost(&mut self, gtid: u32) -> Result<()>;
}

fn waiting(world: &mut impl KillThinkWorld) -> Result<bool> {
    world
        .performer()?
        .ok_or_else(|| Error::World("4DB0F0 on a null performer".into()))
        .map(|s| s.current && s.stage != 4)
}

/// Complete `535240`. Returns the base Think's wrong-activity flag.
pub fn think(
    state: StateId,
    target_gtid: u32,
    arena: &mut Arena,
    world: &mut impl KillThinkWorld,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    if waiting(world)? {
        // Native diagnostic "unit %d KILL waiting for ability to finish".
        return Ok(wrong);
    }
    let target = match world.resolve(target_gtid)? {
        Some(t) if world.relation(t)? == 0 => t,
        _ => {
            world.target_lost(target_gtid)?;
            return Ok(wrong);
        }
    };
    let mut record = KillDecision::initial();
    world.choose(target, &mut record)?;
    world.apply(&mut record)?;
    let Some(performer) = world.performer()? else {
        return Ok(wrong);
    };
    if performer.current && performer.stage != 4 {
        return Ok(wrong);
    }
    if record.status == 0 {
        return Ok(wrong);
    }
    if world.in_range(target)? {
        world.approach(target)?;
    } else {
        world.stop()?;
    }
    Ok(wrong)
}
