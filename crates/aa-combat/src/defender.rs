//! Defender component (towers, bunkers, anti-air): the update and its target
//! choice.
//!
//! - `0x567960`: the update (Updatable at Defender +8). A pending radius
//!   refresh goes to the zone (`0x515F40`). For each ability (`+10`, `+14`
//!   entries, the count read once): a live target is re-admitted
//!   (`0x4C8F20`, range checked; 7 or 8 skip the ability this update, other
//!   refusals and dead targets clear it, `0x567140`); a target that is gone,
//!   does not qualify (actor virtual `+B4`) or ranks below 2 is chosen again
//!   over the intruder list (`+1C`); then a target is used (`0x57E030` on
//!   the owner's Performer), after its turret is on it (`0x4C81D0`,
//!   `0x57E730`) when the ability has one.
//! - `0x567530`: the best intruder: an organization is searched member by
//!   member (`+7C`, hash order); otherwise the highest `0x566D40` rank wins,
//!   and at an equal rank the strictly nearer (`0x45E790`, x87 distance
//!   against the stored binary32 best).
//! - `0x566D40`: the threat rank 3 / 2 / 1 / 0.
//! - `0x5673B0`: the reach test of an intruder (used when one enters).
//! - `0x566F80`: the radius setter.
//!
//! Proven bit for bit at PC24, PC53 and PC64 by
//! research/phase8-combat/native_defender.py (tests/native_defender.rs) and
//! research/phase8-combat/native_defender_reach.py
//! (tests/native_defender_reach.rs).
use crate::{Result, performer::Context};
use aa_body::F32;
use aa_numeric::{Fast, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// `0x7F7FFFFF`: the best distance before any candidate.
const FAR: F32 = F32(0x7F7F_FFFF);

/// The Defender component words the update reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Defender {
    /// +4
    pub owner: u32,
    /// +10 / +14
    pub abilities: Vec<u32>,
    /// +1C list, word 0 of each node in list order: intruder handles.
    pub intruders: Vec<u32>,
    /// +2C
    pub radius: F32,
    /// +30: the radius is sent to the zone on the next update.
    pub radius_dirty: bool,
}

/// Services of the Defender update.
pub trait DefenderWorld {
    /// The handle table (`[0xB364C0]`): the actor in the slot when the
    /// generation matches and the slot is filled.
    fn resolve(&self, handle: u32) -> Result<Option<u32>>;
    /// Actor `+8` bit 0.
    fn removed(&self, actor: u32) -> Result<bool>;
    /// Actor `+14`.
    fn handle_of(&self, actor: u32) -> Result<u32>;
    /// Actor `+20`, `+24`.
    fn position(&self, actor: u32) -> Result<[F32; 2]>;
    /// Ability `+14`: its target handle.
    fn ability_target(&self, ability: u32) -> Result<u32>;
    /// Ability definition (`[+4]`) `+54` bit 3: the context kind is 3, else 1.
    fn ability_kind_flag(&self, ability: u32) -> Result<bool>;
    /// `0x4C8F20(context, check_range)` on the ability; zero admits.
    fn can_use(&mut self, ability: u32, context: &mut Context, check_range: bool) -> Result<u32>;
    /// `0x567140(ability, target)` on the Defender.
    fn set_target(&mut self, ability: u32, target: Option<u32>) -> Result<()>;
    /// Actor virtual `+B4`.
    fn qualifies(&mut self, actor: u32) -> Result<bool>;
    /// Actor `+7C` organization: its `+28` member entries in `0x4364C0`
    /// order (0 for an empty entry); None without one.
    fn organization_members(&mut self, actor: u32) -> Result<Option<Vec<u32>>>;
    /// Actor `+70` component present and its virtual `+58` true.
    fn aggressive(&mut self, actor: u32) -> Result<bool>;
    /// `0x4D03F0` on the actor.
    fn harmless(&mut self, actor: u32) -> Result<bool>;
    /// Actor `+60` set, ActorData `+291` set and actor `+8` bit 11 clear.
    fn body_takeable(&self, actor: u32) -> Result<bool>;
    /// `0x4D1120` on the actor.
    fn capturable(&mut self, actor: u32) -> Result<bool>;
    /// `0x4C81D0`: the ability has a turret.
    fn ability_has_turret(&self, ability: u32) -> Result<bool>;
    /// `0x4C81E0`: the ability's turret index.
    fn ability_turret(&self, ability: u32) -> Result<i32>;
    /// `0x57E730(index, &point, handle)` on the owner's Performer (`+74`);
    /// bind [`crate::turret::aim`].
    fn aim(&mut self, owner: u32, index: i32, point: [F32; 2], handle: u32) -> Result<bool>;
    /// `0x57E030(ability, &context)` on the owner's Performer (`+74`).
    fn use_ability(&mut self, owner: u32, ability: u32, context: &mut Context) -> Result<()>;
    /// `0x515F40(x, y, radius)` on the owner's zone (owner `+148` `+10`
    /// when owner `+14C` is above 4, else none).
    fn refresh_radius(&mut self, owner: u32, position: [F32; 2], radius: F32) -> Result<()>;
}

fn num(value: F32) -> Result<Fast> {
    Ok(Fast::from_f32_bits(value.0)?)
}

/// `0x45E790`: the x87 distance from `from` to `to` (each difference, the
/// squares, their sum and the root at the running precision).
pub fn distance(from: [F32; 2], to: [F32; 2], precision: Precision) -> Result<Fast> {
    let dy = num(to[1])?.sub(&num(from[1])?, precision)?;
    let dx = num(to[0])?.sub(&num(from[0])?, precision)?;
    Ok(dx
        .mul(&dx, precision)?
        .add(&dy.mul(&dy, precision)?, precision)?
        .sqrt(precision)?)
}

/// Native `0x566D40`: 3 when the actor's `+70` component is aggressive, 2
/// when `0x4D03F0` is false, else 1 when its body can be taken or
/// `0x4D1120` holds, else 0.
pub fn threat_rank(actor: u32, world: &mut impl DefenderWorld) -> Result<i32> {
    if world.aggressive(actor)? {
        return Ok(3);
    }
    if !world.harmless(actor)? {
        return Ok(2);
    }
    if !world.body_takeable(actor)? && !world.capturable(actor)? {
        return Ok(0);
    }
    Ok(1)
}

/// Native `0x567530(candidate, ability, &best_rank, &best_distance)`.
pub fn best_candidate(
    defender: &Defender,
    candidate: Option<u32>,
    ability: u32,
    best_rank: &mut i32,
    best_distance: &mut F32,
    world: &mut impl DefenderWorld,
    precision: Precision,
) -> Result<Option<u32>> {
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    if let Some(members) = world.organization_members(candidate)? {
        let mut chosen = None;
        for member in members {
            let member = (member != 0).then_some(member);
            if let Some(found) =
                best_candidate(defender, member, ability, best_rank, best_distance, world, precision)?
            {
                chosen = Some(found);
            }
        }
        return Ok(chosen);
    }
    let rank = threat_rank(candidate, world)?;
    if rank == 0 || rank < *best_rank {
        return Ok(None);
    }
    let mut context = Context::new(1);
    context.source = Some(defender.owner);
    context.target = Some(candidate);
    if world.can_use(ability, &mut context, true)? != 0 {
        return Ok(None);
    }
    let length = distance(
        world.position(defender.owner)?,
        world.position(candidate)?,
        precision,
    )?;
    if rank > *best_rank || length.compare(&num(*best_distance)?) == Ordering::Less {
        *best_distance = F32(length.store_f32()?);
        *best_rank = rank;
        return Ok(Some(candidate));
    }
    Ok(None)
}

/// Native `0x567960`: one Defender update.
pub fn tick(
    defender: &mut Defender,
    world: &mut impl DefenderWorld,
    precision: Precision,
) -> Result<()> {
    if defender.radius_dirty {
        let position = world.position(defender.owner)?;
        world.refresh_radius(defender.owner, position, defender.radius)?;
        defender.radius_dirty = false;
    }
    // One context (0x4C8300(1)) for every ability of this update.
    let mut context = Context::new(1);
    context.source = Some(defender.owner);
    let count = defender.abilities.len();
    for index in 0..count {
        let ability = defender.abilities[index];
        context.event_kind = if world.ability_kind_flag(ability)? { 3 } else { 1 };
        if let Some(target) = world.resolve(world.ability_target(ability)?)? {
            context.target = Some(target);
            if world.removed(target)? {
                world.set_target(ability, None)?;
            } else {
                match world.can_use(ability, &mut context, true)? {
                    7 | 8 => continue,
                    0 => {}
                    _ => world.set_target(ability, None)?,
                }
            }
        }
        let keep = match world.resolve(world.ability_target(ability)?)? {
            Some(target) => world.qualifies(target)? && threat_rank(target, world)? >= 2,
            None => false,
        };
        if !keep {
            let mut chosen = None;
            let mut rank = -1;
            let mut best = FAR;
            for &handle in &defender.intruders {
                let candidate = world.resolve(handle)?;
                if let Some(found) = best_candidate(
                    defender, candidate, ability, &mut rank, &mut best, world, precision,
                )? {
                    chosen = Some(found);
                }
            }
            world.set_target(ability, chosen)?;
        }
        context.phase = 0;
        let target = world.resolve(world.ability_target(ability)?)?;
        context.target = target;
        let Some(target) = target else {
            continue;
        };
        if world.ability_has_turret(ability)? {
            let handle = world.handle_of(target)?;
            let turret = world.ability_turret(ability)?;
            let point = world.position(target)?;
            if !world.aim(defender.owner, turret, point, handle)? {
                continue;
            }
        }
        world.use_ability(defender.owner, ability, &mut context)?;
    }
    Ok(())
}

/// Services of the reach test `0x5673B0`.
pub trait ReachWorld {
    /// Actor virtual `+B4`.
    fn qualifies(&mut self, actor: u32) -> Result<bool>;
    /// Actor `+7C` organization: its `+28` member entries in `0x4364C0`
    /// order (0 for an empty entry); None without one.
    fn organization_members(&mut self, actor: u32) -> Result<Option<Vec<u32>>>;
    /// Actor `+8`.
    fn flags(&self, actor: u32) -> Result<u32>;
    /// Actor `+20`, `+24`.
    fn position(&self, actor: u32) -> Result<[F32; 2]>;
    /// `0x4C8740(context, 1)` on the ability.
    fn admits(&mut self, ability: u32, context: &mut Context) -> Result<bool>;
    /// `0x4C84B0(owner, &point)` on the ability.
    fn within(&mut self, ability: u32, owner: u32, point: [F32; 2]) -> Result<bool>;
    /// `0x4C8460(owner, &point)` on the ability.
    fn clear(&mut self, ability: u32, owner: u32, point: [F32; 2]) -> Result<bool>;
    /// Ability definition (`[+4]`) `+3C` is set.
    fn has_projectile(&self, ability: u32) -> Result<bool>;
    /// `0x4CF110` on the owner, then `0x4BE910(modifiers)` on the ability
    /// definition: the x87 range.
    fn range(&mut self, ability: u32, owner: u32) -> Result<Fast>;
}

/// Native `0x5673B0(candidate)`: the Defender can reach the candidate. A
/// qualifying organization (`+B4`, `+7C`) is reached through any member;
/// otherwise the candidate must have `+8` bit 14 and one ability must admit
/// it (`0x4C8740`, `0x4C84B0`, `0x4C8460`) with no projectile or within its
/// range (binary32 squared distance against the x87 squared range).
pub fn reachable(
    defender: &Defender,
    candidate: Option<u32>,
    world: &mut impl ReachWorld,
    precision: Precision,
) -> Result<bool> {
    let Some(candidate) = candidate else {
        return Ok(false);
    };
    if world.qualifies(candidate)?
        && let Some(members) = world.organization_members(candidate)?
    {
        for member in members {
            if reachable(defender, (member != 0).then_some(member), world, precision)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    if world.flags(candidate)? & 0x4000 == 0 {
        return Ok(false);
    }
    let point = world.position(candidate)?;
    let mut context = Context::new(1);
    context.source = Some(defender.owner);
    context.target = Some(candidate);
    for &ability in &defender.abilities {
        if !world.admits(ability, &mut context)?
            || !world.within(ability, defender.owner, point)?
            || !world.clear(ability, defender.owner, point)?
        {
            continue;
        }
        if !world.has_projectile(ability)? {
            return Ok(true);
        }
        let owner = world.position(defender.owner)?;
        let dy = num(point[1])?.sub(&num(owner[1])?, precision)?;
        let dx = num(point[0])?.sub(&num(owner[0])?, precision)?;
        let squared = num(F32(dx
            .mul(&dx, precision)?
            .add(&dy.mul(&dy, precision)?, precision)?
            .store_f32()?))?;
        let range = world.range(ability, defender.owner)?;
        if squared.compare(&range.mul(&range, precision)?) != Ordering::Greater {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Services of the radius setter `0x566F80`.
pub trait RadiusWorld {
    /// Owner ActorData `+174` bit 7.
    fn wide_radius(&self, owner: u32) -> Result<bool>;
    /// `0x45D4C0(argument)` (bit 7 set) or `0x45D6C0(argument)` on the
    /// owner ActorData: the x87 radius.
    fn radius(&mut self, owner: u32, wide: bool, argument: u32) -> Result<Fast>;
}

/// Native `0x566F80(argument)`: the radius (`+2C`, stored to binary32) from
/// the owner's definition, marked for the next update's zone refresh (`+30`).
pub fn set_radius(defender: &mut Defender, argument: u32, world: &mut impl RadiusWorld) -> Result<()> {
    let wide = world.wide_radius(defender.owner)?;
    defender.radius = F32(world.radius(defender.owner, wide, argument)?.store_f32()?);
    defender.radius_dirty = true;
    Ok(())
}
