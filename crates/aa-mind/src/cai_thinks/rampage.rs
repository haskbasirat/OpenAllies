//! Original `KKC_CAIStateRampage::Think` (`5267E0`, regiment state
//! `cai_rampage` at Mind+54, vtable A32A14 slot +C) with its Tai helpers
//! `51D6E0` (go to sleep), `51E910` (attack a target) and `526740` (wander).
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`).
//! 2. An active kill state (Mind+C0, status 2) or move state (Mind+68)
//!    thinks instead (tail jumps).
//! 3. Organization component +5C below +60 (binary32, ordered): go to sleep.
//! 4. Not rampaging (state+10 byte clear): when `1.0 - 578790()` (live x87
//!    precision) is above the config threshold (definition +314 config +18),
//!    go to sleep, then every listed member (component +28 array, signed
//!    count +2C read once) that is non-null and not exempt (+80 object with
//!    its +1C byte set) has its Mind (+70) virtual +FC called; the state
//!    starts rampaging and the Think ends. Otherwise the byte is reread.
//! 5. Rampaging: go to sleep; `1.0 - 578790()` above 1e-4 ends the Think,
//!    else the state stops rampaging.
//! 6. A hunting config (+1C byte) asks `51E450(&organization+20, radius)`
//!    with the `577F20` radius stored to binary32; a target is attacked.
//! 7. Otherwise one world RNG draw (`45D400`) below the config wander chance
//!    (+0) goes to sleep, else the regiment wanders.
//!
//! Helpers: go to sleep leaves an active kill and move state (`529C20`) and
//! activates sleep in mode 0 unless it is active, then sleep thinks and Mind
//! virtual +B8. Attack: with an active kill state whose target (Mind+D0)
//! resolves to the new target nothing changes; otherwise the kill state (or
//! the sleep and move states when kill is inactive) is left and `51D920(0,
//! target)` runs; then kill thinks and Mind virtual +B8. Wander: an active
//! move state thinks; else sleep and kill are left, `525100` picks a point
//! and `520440(0, point, 0, null point)` orders the move, then move thinks
//! and Mind virtual +B8.
use super::{KILL_OFFSET, MOVE_OFFSET, RegimentWorld, SLEEP_OFFSET, leave_active, x87_copy};
use crate::{
    Arena, Error, MindId, Result, StateId, activate, think_base,
    x87::{n, num, unit_random},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// `[9F32E0]` binary32 1.0.
const ONE_BITS: u32 = 0x3f80_0000;
/// `[A329E4]` binary32 1e-4: calm-down margin.
pub const CALM_BITS: u32 = 0x38d1_b717;

/// Services in native order.
pub trait RampageThinkWorld: RegimentWorld {
    /// Rampage config (organization definition +314) +0, +18 (binary32) and
    /// +1C byte.
    fn rampage_wander_chance(&mut self, mind: MindId) -> Result<u32>;
    fn rampage_threshold(&mut self, mind: MindId) -> Result<u32>;
    fn rampage_hunts(&mut self, mind: MindId) -> Result<bool>;
    /// Organization component +5C/+60 (binary32).
    fn component_5c_60(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// Rampage state+10 byte.
    fn rampaging(&mut self, state: StateId) -> Result<bool>;
    fn set_rampaging(&mut self, state: StateId, value: bool) -> Result<()>;
    /// `578790` on the organization component (ST0 result).
    fn component_578790(&mut self, mind: MindId, arena: &mut Arena) -> Result<Binary>;
    /// Organization component +2C (signed member count).
    fn component_member_count(&mut self, mind: MindId) -> Result<i32>;
    /// Organization component +28 array entry `index` (array reread).
    fn component_member(&mut self, mind: MindId, index: u32) -> Result<u32>;
    /// Member +80, `None` when null.
    fn member_word80(&mut self, member: u32) -> Result<Option<u32>>;
    /// That object's +1C byte.
    fn word80_byte1c(&mut self, object: u32) -> Result<bool>;
    /// Member +70 (its Mind), `None` when null.
    fn member_mind(&mut self, member: u32) -> Result<Option<u32>>;
    /// Member Mind virtual +FC.
    fn member_mind_virtual_fc(&mut self, member_mind: u32, arena: &mut Arena) -> Result<()>;
    /// `577F20` on the organization component: the binary32 it loads.
    fn component_577f20(&mut self, mind: MindId, arena: &mut Arena) -> Result<u32>;
    /// `51E450(&organization+20, radius)` on the Mind: a target or `None`.
    fn nearest_target(
        &mut self,
        mind: MindId,
        radius: u32,
        arena: &mut Arena,
    ) -> Result<Option<u32>>;
    /// Kill state+10 (Mind+D0) target GTID.
    fn kill_target(&mut self, mind: MindId) -> Result<u32>;
    /// `51D920(mode, target)` on the kill state; result unused here.
    fn kill_activate(
        &mut self,
        mind: MindId,
        mode: u32,
        target: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// One `85D840` draw on the world RNG (`4F02B0`).
    fn random(&mut self) -> Result<u32>;
    /// `525100(&out)` on the rampage state: the wander point words written.
    fn wander_point(&mut self, state: StateId, arena: &mut Arena) -> Result<[u32; 2]>;
    /// `[B279C0]/[B279C4]`.
    fn null_point(&mut self) -> Result<[u32; 2]>;
    /// `520440` on the move state with its six stack words.
    fn move_order(&mut self, mind: MindId, args: [u32; 6], arena: &mut Arena) -> Result<()>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn status_at(
    world: &mut impl RampageThinkWorld,
    mind: MindId,
    offset: u32,
    arena: &Arena,
) -> Result<u32> {
    let state = world.embedded_state(mind, offset)?;
    Ok(arena.state(state)?.status)
}

/// `51D6E0` on the rampage state.
pub fn go_to_sleep(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl RampageThinkWorld,
) -> Result<()> {
    let mind = arena.state(state)?.parent;
    if status_at(world, mind, SLEEP_OFFSET, arena)? != 2 {
        leave_active(world, mind, KILL_OFFSET, arena)?;
        leave_active(world, mind, MOVE_OFFSET, arena)?;
        let sleep = world.embedded_state(mind, SLEEP_OFFSET)?;
        activate(sleep, 0, arena, world)?;
    }
    world.embedded_think(mind, SLEEP_OFFSET, arena)?;
    world.mind_virtual_b8(mind, arena)
}

/// `51E910(target)` on the rampage state.
pub fn attack(
    state: StateId,
    target: u32,
    arena: &mut Arena,
    world: &mut impl RampageThinkWorld,
) -> Result<()> {
    let mind = arena.state(state)?.parent;
    if status_at(world, mind, KILL_OFFSET, arena)? == 2 {
        let current = world.kill_target(mind)?;
        if world.resolve(current)? != Some(target) {
            leave_active(world, mind, KILL_OFFSET, arena)?;
            world.kill_activate(mind, 0, target, arena)?;
        }
    } else {
        leave_active(world, mind, SLEEP_OFFSET, arena)?;
        leave_active(world, mind, MOVE_OFFSET, arena)?;
        world.kill_activate(mind, 0, target, arena)?;
    }
    world.embedded_think(mind, KILL_OFFSET, arena)?;
    world.mind_virtual_b8(mind, arena)
}

/// `526740` on the rampage state.
pub fn wander(state: StateId, arena: &mut Arena, world: &mut impl RampageThinkWorld) -> Result<()> {
    let mind = arena.state(state)?.parent;
    if status_at(world, mind, MOVE_OFFSET, arena)? == 2 {
        return world.embedded_think(mind, MOVE_OFFSET, arena);
    }
    leave_active(world, mind, SLEEP_OFFSET, arena)?;
    leave_active(world, mind, KILL_OFFSET, arena)?;
    let point = world.wander_point(state, arena)?;
    let null = world.null_point()?;
    world.move_order(mind, [0, point[0], point[1], 0, null[0], null[1]], arena)?;
    world.embedded_think(mind, MOVE_OFFSET, arena)?;
    world.mind_virtual_b8(mind, arena)
}

/// `1.0 - 578790()` at precision `p`.
fn morale(
    mind: MindId,
    arena: &mut Arena,
    world: &mut impl RampageThinkWorld,
    p: Precision,
) -> Result<Binary> {
    let fraction = world.component_578790(mind, arena)?;
    num(n(ONE_BITS)?.sub(&fraction, p))
}

/// Complete `5267E0` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag. A null member Mind is the native null dereference.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl RampageThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    if status_at(world, mind, KILL_OFFSET, arena)? == 2 {
        world.embedded_think(mind, KILL_OFFSET, arena)?;
        return Ok(wrong);
    }
    if status_at(world, mind, MOVE_OFFSET, arena)? == 2 {
        world.embedded_think(mind, MOVE_OFFSET, arena)?;
        return Ok(wrong);
    }
    let [low, high] = world.component_5c_60(mind)?;
    if host(low) < host(high) {
        go_to_sleep(state, arena, world)?;
        return Ok(wrong);
    }
    let mut rampaging = world.rampaging(state)?;
    if !rampaging {
        let value = morale(mind, arena, world, p)?;
        let threshold = world.rampage_threshold(mind)?;
        if value.compare(&n(threshold)?) == Ordering::Greater {
            go_to_sleep(state, arena, world)?;
            let count = world.component_member_count(mind)?;
            for index in 0..count.max(0) as u32 {
                let member = world.component_member(mind, index)?;
                if member == 0 {
                    continue;
                }
                if let Some(object) = world.member_word80(member)?
                    && world.word80_byte1c(object)?
                {
                    continue;
                }
                let member_mind = world
                    .member_mind(member)?
                    .ok_or_else(|| Error::World("null member Mind dereference".into()))?;
                world.member_mind_virtual_fc(member_mind, arena)?;
            }
            world.set_rampaging(state, true)?;
            return Ok(wrong);
        }
        rampaging = world.rampaging(state)?;
    }
    if rampaging {
        go_to_sleep(state, arena, world)?;
        if morale(mind, arena, world, p)?.compare(&n(CALM_BITS)?) == Ordering::Greater {
            return Ok(wrong);
        }
        world.set_rampaging(state, false)?;
    }
    if world.rampage_hunts(mind)? {
        let radius = x87_copy(world.component_577f20(mind, arena)?);
        if let Some(target) = world.nearest_target(mind, radius, arena)? {
            attack(state, target, arena, world)?;
            return Ok(wrong);
        }
    }
    let draw = unit_random(world.random()?, p)?;
    if draw.compare(&n(world.rampage_wander_chance(mind)?)?) == Ordering::Less {
        go_to_sleep(state, arena, world)?;
    } else {
        wander(state, arena, world)?;
    }
    Ok(wrong)
}
