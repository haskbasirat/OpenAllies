//! Original SAI base goal behaviour (`KKC_SAIGoal`, Source/Sai/SAIGoal.cpp):
//! the base-class virtuals that most goal classes keep (vtable slot in
//! brackets) and the helpers they share.
//!
//! - `7A3B00` priority with a hypothetical actor change, including the
//!   per-type SAIData fuzzy modifiers (`4A7CD0`) and the travel weighting
//!   `4A7130` (CRT `_CIpow` `96D890`, ported in `aa_numeric::crt_pow`);
//! - `7A4FB0` [+24] swap value, `7A5020` [+28] actor value, `7A5080` [+2C]
//!   held-actor value, `59E240` [+30] actor count;
//! - `7A6930` [+64] add actor, `7A6A90` [+68] remove actor, `7A67B0` [+60]
//!   re-evaluate, `7A5600` [+80] deselect with `7A3D30` (release every actor
//!   back to the player's pools through `7AFB00`) and the owner update
//!   `7ACF50`/`7A0F80`.
//!
//! Every goal virtual reached from these bodies is called through the world
//! (classes may override it), as are the actor virtuals and the game-side
//! services: `79F390(7A10F0(actor))`, the actor travel time `7A1C10` and the
//! modifier match `4A7CD0(7A10F0(actor))`. `7A3860`/`7A5200` are the ported
//! `goal_priority` routines, bound by the caller.
use super::assign::{ActorId, CATEGORIES, NO_CATEGORY, Resources, State};
use super::engine::GoalId;
use crate::{
    Error, Result,
    x87::{n, num, store},
};
use aa_numeric::{
    Binary, Precision,
    crt_pow::{CrtEnvironment, ci_pow},
};
use std::cmp::Ordering;

/// `[A9C778]`: -1e6, the swap value of an actor that cannot serve the goal.
const CANNOT_SERVE: u32 = 0xc974_2400;
/// `[9FDC48]`: -1.
const MINUS_ONE: u32 = 0xbf80_0000;
const ONE: u32 = 0x3f80_0000;
/// `[A16F30]` 0.8f and `[A0145C]`/`0x3E4CCCCD` 0.2f of `4A7130`.
const TRAVEL_SCALE: u32 = 0x3f4c_cccd;
const TRAVEL_FLOOR: u32 = 0x3e4c_cccd;

/// SAIData fields read by `4A7130`, and the CRT state that selects the
/// `_CIpow` body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TravelWeighting {
    /// SAIData `+36C` travel-time normaliser (binary32).
    pub normaliser: u32,
    /// SAIData `+370` exponent (binary32).
    pub exponent: u32,
    pub crt: CrtEnvironment,
}

/// Goal virtuals, actor virtuals and game services used by the base bodies.
pub trait GoalBaseWorld {
    /// Goal `+4`: goal type.
    fn goal_type(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<u32>;
    /// Goal `+8`: pool category.
    fn category(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<u32>;
    /// Goal `+20`: target location (an opaque native pointer).
    fn location(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<u32>;
    /// Goal `+28(actor, check)`.
    fn value(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
        check: bool,
    ) -> Result<Binary>;
    /// Goal `+2C(actor)`.
    fn held_value(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
    ) -> Result<Binary>;
    /// Goal `+30` (base `59E240`: the `+14` count).
    fn count(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<i32>;
    /// Goal `+38`: required actor.
    fn required_actor(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<Option<ActorId>>;
    /// Goal `+68(actor, resources, refresh, keep, extra)` (base `7A6A90`).
    #[allow(clippy::too_many_arguments)]
    fn remove_actor(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
        resources: Resources<'_>,
        refresh: bool,
        keep: bool,
        extra: u32,
    ) -> Result<()>;
    /// Goal `+7C(add, remove)`: class priority with a hypothetical change.
    fn priority_hook(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        add: Option<ActorId>,
        remove: Option<ActorId>,
    ) -> Result<Binary>;
    /// Goal `+80` (base `7A5600`).
    fn deselect(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<()>;
    /// Actor `+C`.
    fn actor_dead(&mut self, s: &mut State<'_>, actor: ActorId) -> Result<bool>;
    /// Actor `+4`.
    fn actor_flag4(&mut self, s: &mut State<'_>, actor: ActorId) -> Result<bool>;
    /// Actor `+20(goal, flag)`.
    fn actor_can_serve(
        &mut self,
        s: &mut State<'_>,
        actor: ActorId,
        goal: GoalId,
        flag: bool,
    ) -> Result<bool>;
    /// `79F390(7A10F0(actor))` on the actor's game object.
    fn unit_check(&mut self, s: &mut State<'_>, actor: ActorId) -> Result<bool>;
    /// Actor `7A1C10(location)`: travel time to a goal location (unstored);
    /// ported as `sai::travel::travel_time` over `TravelWorld`.
    fn travel_time(&mut self, s: &mut State<'_>, actor: ActorId, location: u32) -> Result<Binary>;
    /// SAIData type entry `+14`: the goal type's modifier objects in list
    /// order (`None` when the list pointer is null).
    fn modifiers(&mut self, s: &mut State<'_>, goal_type: u32) -> Result<Option<Vec<u32>>>;
    /// `4A7CD0(7A10F0(actor), &sum, &product, &matched)` on a modifier.
    fn apply_modifier(
        &mut self,
        s: &mut State<'_>,
        modifier: u32,
        actor: ActorId,
        sum: &mut u32,
        product: &mut u32,
        matched: &mut bool,
    ) -> Result<()>;
    /// SAIData `+36C/+370` and the CRT path state.
    fn travel_weighting(&mut self, s: &mut State<'_>) -> Result<TravelWeighting>;
    /// `7A3860(resources, relaxed)`.
    fn affordable(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        resources: Resources<'_>,
        relaxed: bool,
    ) -> Result<bool>;
    /// `7A5200(resources, draw)`.
    fn finalize(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        resources: Resources<'_>,
        draw: bool,
    ) -> Result<()>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn null() -> Error {
    Error::World("null SAI actor dereference".into())
}
fn travel(s: &State<'_>, goal: GoalId) -> Result<u32> {
    Ok(s.ctl.goal(goal)?.travel)
}
fn set_travel(s: &mut State<'_>, goal: GoalId, bits: u32) -> Result<()> {
    s.ctl.goal_mut(goal)?.travel = bits;
    Ok(())
}
fn count(s: &State<'_>, goal: GoalId) -> Result<i32> {
    Ok(s.roster.goal_list(goal)?.count)
}
/// `!actor.+4 ? !79F390(7A10F0(actor)) : true`: the actor counts for travel.
fn counts_for_travel(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    actor: ActorId,
) -> Result<bool> {
    Ok(world.actor_flag4(s, actor)? || !world.unit_check(s, actor)?)
}

/// `7ACF50` -> `7A0F80`: set the actor's owning goal, drop its cached route
/// (`+30`, freed) and invalidate its cached travel time (`+14` = -1).
pub fn set_owner(s: &mut State<'_>, actor: Option<ActorId>, goal: Option<GoalId>) -> Result<()> {
    let record = s.roster.actor_mut(actor)?;
    record.route = None;
    record.owner = goal;
    record.travel_cache = MINUS_ONE;
    Ok(())
}

/// Complete `7AFB00` (player): return a live actor to the pool of
/// `category` unless it is in the player's withheld list (`+38`).
pub fn return_to_pool(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    category: u32,
    actor: Option<ActorId>,
) -> Result<()> {
    let id = actor.ok_or_else(null)?;
    if world.actor_dead(s, id)? || s.roster.reserved.contains(actor) {
        return Ok(());
    }
    let pool = s.roster.pool_mut(category)?;
    pool.count += 1;
    pool.push_front(actor);
    Ok(())
}

/// Complete `7A3D30`: remove every actor (goal `+68` with keep set) and
/// return each live one to the pools of all its categories; current = 0.
pub fn release_all(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    extra: u32,
) -> Result<()> {
    while let Some(head) = s.roster.goal_list(goal)?.head() {
        let actor = s.roster.goal_list(goal)?.actor(head);
        world.remove_actor(s, goal, actor, Resources::Null, false, true, extra)?;
        let id = actor.ok_or_else(null)?;
        if !world.actor_dead(s, id)? {
            for i in 0..CATEGORIES as u32 {
                if s.roster.actor(actor)?.categories & (1_u64 << i) != 0 {
                    return_to_pool(world, s, i, actor)?;
                }
            }
        }
    }
    s.ctl.goal_mut(goal)?.current = 0;
    Ok(())
}

/// Complete `7A5600` [+80]: release all actors, then a non-inactive goal
/// becomes a candidate (status 1).
pub fn deselect(world: &mut impl GoalBaseWorld, s: &mut State<'_>, goal: GoalId) -> Result<()> {
    let status = s.ctl.goal(goal)?.status;
    release_all(world, s, goal, 0)?;
    if status != 0 {
        s.ctl.goal_mut(goal)?.status = 1;
    }
    Ok(())
}

/// `59E240` [+30]: the actor count `+14`.
pub fn actor_count(s: &State<'_>, goal: GoalId) -> Result<i32> {
    count(s, goal)
}

/// Complete `7A4FB0` [+24]: value of gaining `gained` for `lost`.
#[allow(clippy::too_many_arguments)]
pub fn swap_value(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    gained: Option<ActorId>,
    lost: Option<ActorId>,
    check: bool,
    p: Precision,
) -> Result<Binary> {
    match gained {
        Some(id) => {
            if check && !world.actor_can_serve(s, id, goal, false)? {
                return n(CANNOT_SERVE);
            }
            if lost.is_none() {
                return world.value(s, goal, gained, false);
            }
        }
        None => {
            if lost.is_some() {
                let held = world.held_value(s, goal, lost)?;
                return num(held.mul(&n(MINUS_ONE)?, p));
            }
        }
    }
    let v = priority_with_actors(world, s, goal, gained, lost, p)?;
    num(v.sub(&n(s.ctl.goal(goal)?.current)?, p))
}

/// Complete `7A5020` [+28]: value of adding `actor`.
pub fn value(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    actor: Option<ActorId>,
    check: bool,
    p: Precision,
) -> Result<Binary> {
    if check && !world.actor_can_serve(s, actor.ok_or_else(null)?, goal, false)? {
        return Ok(Binary::from_i32(0));
    }
    if world.required_actor(s, goal)?.is_some() {
        return priority_with_actors(world, s, goal, actor, None, p);
    }
    if world.category(s, goal)? == NO_CATEGORY {
        return Ok(Binary::from_i32(0));
    }
    let v = priority_with_actors(world, s, goal, actor, None, p)?;
    num(v.sub(&n(s.ctl.goal(goal)?.current)?, p))
}

/// Complete `7A5080` [+2C]: value of keeping `actor`.
pub fn held_value(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    actor: Option<ActorId>,
    p: Precision,
) -> Result<Binary> {
    if world.actor_dead(s, actor.ok_or_else(null)?)? {
        return Ok(Binary::from_i32(0));
    }
    if world.required_actor(s, goal)?.is_some() {
        return n(s.ctl.goal(goal)?.base);
    }
    if world.category(s, goal)? == NO_CATEGORY {
        return Ok(Binary::from_i32(0));
    }
    let current = s.ctl.goal(goal)?.current;
    let v = priority_with_actors(world, s, goal, None, actor, p)?;
    num(n(current)?.sub(&v, p))
}

/// The refresh tail shared by `7A6930`/`7A6A90`: when `+38 > 0`, current =
/// `affordable(resources, 0) ? (float)7A3B00(0,0) : 0`, and a selected goal
/// whose current is not positive deselects unless `keep`.
fn refresh(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    res: Resources<'_>,
    keep: bool,
    p: Precision,
) -> Result<()> {
    if host(s.ctl.goal(goal)?.base) > 0.0 {
        let current = if world.affordable(s, goal, res, false)? {
            store(priority_with_actors(world, s, goal, None, None, p)?)?
        } else {
            0
        };
        s.ctl.goal_mut(goal)?.current = current;
        if host(current) <= 0.0 && s.ctl.goal(goal)?.status == 2 && !keep {
            world.deselect(s, goal)?;
        }
    }
    Ok(())
}

/// Complete `7A6930` [+64]: add `actor` (new head node), folding its travel
/// time into the average `+30` when it counts for travel.
#[allow(clippy::too_many_arguments)]
pub fn add_actor(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    actor: Option<ActorId>,
    res: Resources<'_>,
    do_refresh: bool,
    keep: bool,
    p: Precision,
) -> Result<()> {
    let id = actor.ok_or_else(null)?;
    if world.actor_dead(s, id)? {
        return Ok(());
    }
    if counts_for_travel(world, s, id)? {
        let n_before = count(s, goal)?;
        let location = world.location(s, goal)?;
        let t = world.travel_time(s, id, location)?;
        let total = num(Binary::from_i32(n_before).mul(&n(travel(s, goal)?)?, p))?;
        let total = num(t.add(&total, p))?;
        let average = num(total.div(&Binary::from_i32(n_before.wrapping_add(1)), p))?;
        set_travel(s, goal, store(average)?)?;
    }
    let list = s.roster.goal_list_mut(goal)?;
    list.count += 1;
    list.push_front(actor);
    set_owner(s, actor, Some(goal))?;
    if do_refresh {
        refresh(world, s, goal, res, keep, p)?;
    }
    Ok(())
}

/// Complete `7A6A90` [+68]: remove `actor`, updating the travel average (a
/// dead actor triggers a full recount over the remaining live actors).
#[allow(clippy::too_many_arguments)]
pub fn remove_actor(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    actor: Option<ActorId>,
    res: Resources<'_>,
    do_refresh: bool,
    keep: bool,
    p: Precision,
) -> Result<()> {
    let id = actor.ok_or_else(null)?;
    if world.actor_dead(s, id)? {
        recount_travel(world, s, goal, p)?;
    } else if counts_for_travel(world, s, id)? {
        let n_before = count(s, goal)?;
        if n_before > 1 {
            let total = store(num(
                Binary::from_i32(n_before).mul(&n(travel(s, goal)?)?, p)
            )?)?;
            let location = world.location(s, goal)?;
            let t = world.travel_time(s, id, location)?;
            let rest = num(n(total)?.sub(&t, p))?;
            let average = num(rest.div(&Binary::from_i32(n_before - 1), p))?;
            set_travel(s, goal, store(average)?)?;
        } else {
            set_travel(s, goal, 0)?;
        }
    }
    let list = s.roster.goal_list_mut(goal)?;
    if list.remove_first(actor) {
        list.count -= 1;
    } else {
        // The native code calls the actor's +C here and ignores the result.
        world.actor_dead(s, id)?;
    }
    set_owner(s, actor, None)?;
    if do_refresh {
        refresh(world, s, goal, res, keep, p)?;
    }
    Ok(())
}

/// The travel recount of `7A6A90`/`7A67B0`: `+30` = mean travel time of the
/// live actors that count for travel (0 when none).
fn recount_travel(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    p: Precision,
) -> Result<()> {
    set_travel(s, goal, 0)?;
    let mut node = s.roster.goal_list(goal)?.head();
    let mut counted = 0_i32;
    while let Some(at) = node {
        let actor = s.roster.goal_list(goal)?.actor(at).ok_or_else(null)?;
        if !world.actor_dead(s, actor)? && counts_for_travel(world, s, actor)? {
            let location = world.location(s, goal)?;
            let t = world.travel_time(s, actor, location)?;
            let sum = num(t.add(&n(travel(s, goal)?)?, p))?;
            set_travel(s, goal, store(sum)?)?;
            counted += 1;
        }
        node = s.roster.goal_list(goal)?.next(at);
    }
    if counted > 0 {
        let average = num(n(travel(s, goal)?)?.div(&Binary::from_i32(counted), p))?;
        set_travel(s, goal, store(average)?)?;
    }
    Ok(())
}

/// Complete `7A67B0` [+60]: drop dead actors (`7A6480` with `7A10E0`),
/// recount travel, finalize the base priority (`7A5200(res, 1)`) and refresh
/// the current priority.
pub fn reevaluate(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    res: Resources<'_>,
    p: Precision,
) -> Result<()> {
    remove_dead(world, s, goal)?;
    set_travel(s, goal, 0)?;
    if count(s, goal)? != 0 {
        recount_travel(world, s, goal, p)?;
    }
    world.finalize(s, goal, res, true)?;
    let base = s.ctl.goal(goal)?.base;
    if host(base) <= 0.0 && s.ctl.goal(goal)?.status == 2 {
        world.deselect(s, goal)?;
    }
    let base = s.ctl.goal(goal)?.base;
    s.ctl.goal_mut(goal)?.current = base;
    if host(base) > 0.0 {
        let current = if world.affordable(s, goal, res, false)? {
            store(priority_with_actors(world, s, goal, None, None, p)?)?
        } else {
            0
        };
        s.ctl.goal_mut(goal)?.current = current;
    }
    Ok(())
}

/// `7A6480` over the goal list with predicate `7A10E0` (actor `+C`).
fn remove_dead(world: &mut impl GoalBaseWorld, s: &mut State<'_>, goal: GoalId) -> Result<()> {
    while let Some(head) = s.roster.goal_list(goal)?.head() {
        let actor = s.roster.goal_list(goal)?.actor(head).ok_or_else(null)?;
        if !world.actor_dead(s, actor)? {
            break;
        }
        let list = s.roster.goal_list_mut(goal)?;
        list.unlink_head();
        list.count -= 1;
    }
    let Some(mut keep) = s.roster.goal_list(goal)?.head() else {
        return Ok(());
    };
    let mut probe = s.roster.goal_list(goal)?.next(keep);
    while let Some(node) = probe {
        let actor = s.roster.goal_list(goal)?.actor(node).ok_or_else(null)?;
        let dead = world.actor_dead(s, actor)?;
        probe = s.roster.goal_list(goal)?.next(node);
        if dead {
            let list = s.roster.goal_list_mut(goal)?;
            list.count -= 1;
            list.unlink_after(keep);
        } else {
            keep =
                s.roster.goal_list(goal)?.next(keep).ok_or_else(|| {
                    Error::World("SAI goal list changed during compaction".into())
                })?;
        }
    }
    Ok(())
}

/// Complete `7A3B00`: the class priority `+7C(add, remove)` adjusted by the
/// goal type's SAIData modifiers over the hypothetical actor set and by the
/// travel weighting of the hypothetical average travel time.
pub fn priority_with_actors(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    goal: GoalId,
    add: Option<ActorId>,
    remove: Option<ActorId>,
    p: Precision,
) -> Result<Binary> {
    let mut v = store(world.priority_hook(s, goal, add, remove)?)?;
    let goal_type = world.goal_type(s, goal)?;
    if let Some(modifiers) = world.modifiers(s, goal_type)? {
        let mut sum = 0_u32;
        let mut product = ONE;
        for modifier in modifiers {
            let mut matched = false;
            if let Some(a) = add
                && !world.actor_dead(s, a)?
                && !world.actor_flag4(s, a)?
            {
                world.apply_modifier(s, modifier, a, &mut sum, &mut product, &mut matched)?;
            }
            let mut node = s.roster.goal_list(goal)?.head();
            while let Some(at) = node {
                if let Some(a) = s.roster.goal_list(goal)?.actor(at)
                    && !world.actor_dead(s, a)?
                    && !world.actor_flag4(s, a)?
                    && Some(a) != remove
                {
                    world.apply_modifier(s, modifier, a, &mut sum, &mut product, &mut matched)?;
                }
                node = s.roster.goal_list(goal)?.next(at);
            }
        }
        let adjusted = num(n(sum)?.add(&n(v)?, p))?;
        v = store(num(adjusted.mul(&n(product)?, p))?)?;
    }
    let mut hypothetical = travel(s, goal)?;
    if let Some(a) = add
        && !world.actor_dead(s, a)?
        && counts_for_travel(world, s, a)?
    {
        let n_before = count(s, goal)?;
        let location = world.location(s, goal)?;
        let t = world.travel_time(s, a, location)?;
        let total = num(Binary::from_i32(n_before).mul(&n(travel(s, goal)?)?, p))?;
        let n_after = count(s, goal)?.wrapping_add(1);
        let total = num(t.add(&total, p))?;
        hypothetical = store(num(total.div(&Binary::from_i32(n_after), p))?)?;
    }
    if let Some(r) = remove
        && !world.actor_dead(s, r)?
        && counts_for_travel(world, s, r)?
    {
        let n_before = count(s, goal)?;
        if n_before > 1 {
            let total = store(num(
                Binary::from_i32(n_before).mul(&n(travel(s, goal)?)?, p)
            )?)?;
            let location = world.location(s, goal)?;
            let t = world.travel_time(s, r, location)?;
            let rest = store(num(n(total)?.sub(&t, p))?)?;
            let divisor = world.count(s, goal)?.wrapping_sub(1);
            hypothetical = store(num(n(rest)?.div(&Binary::from_i32(divisor), p))?)?;
        } else {
            hypothetical = 0;
        }
    }
    let weight = travel_weight(world, s, hypothetical, p)?;
    num(weight.mul(&n(v)?, p))
}

/// Complete `4A7130` (SAIData): `pow(clamp(1 - travel/norm*0.8, 0.2, 1), exp)`
/// with the clamp tested on the unstored value for the floor and on the
/// stored binary32 for the ceiling.
pub fn travel_weight(
    world: &mut impl GoalBaseWorld,
    s: &mut State<'_>,
    travel_bits: u32,
    p: Precision,
) -> Result<Binary> {
    let w = world.travel_weighting(s)?;
    let scaled = num(n(travel_bits)?.div(&n(w.normaliser)?, p))?;
    let scaled = num(scaled.mul(&n(TRAVEL_SCALE)?, p))?;
    let raw = num(n(ONE)?.sub(&scaled, p))?;
    let mut base = store(raw.clone())?;
    if raw.compare(&n(TRAVEL_FLOOR)?) == Ordering::Less {
        base = TRAVEL_FLOOR;
    } else if host(base) > 1.0 {
        base = ONE;
    }
    let outcome = ci_pow(f64::from(host(base)), f64::from(host(w.exponent)), w.crt)
        .map_err(|e| Error::World(format!("CRT pow 96D890: {e}")))?;
    if outcome.libm_error.is_some() {
        return Err(Error::World(format!(
            "CRT pow 96D890 reported libm error {:?}",
            outcome.libm_error
        )));
    }
    num(Binary::from_f64_bits(outcome.bits))
}
