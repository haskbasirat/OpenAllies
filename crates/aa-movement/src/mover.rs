//! Remaining original Mover component operations around the ground step:
//! updatable Tick dispatch `0x577e50`, arrival/failure stops `0x5768a0` /
//! `0x5768f0`, the stopped-near predicate `0x577330`, Properties suspension
//! predicate `0x4d03d0`, stop-turning `0x5767f0`, requested-speed setter
//! `0x576b20`, maximum-speed refresh `0x576c70` with turn-rate refresh
//! `0x576850`, post-load falling fixup `0x576700` and the launch-speed helper
//! `0x577380`.
//!
//! Every world effect is an explicit callback, invoked in native order; the
//! callbacks receive the live snapshots so native rereads observe mutations.
//! Nothing here is transactional: an error leaves earlier field writes and
//! callback effects applied, and callers must not automatically retry.
use crate::{ActorMotion, Error, Event, GroundWorld, MoverState, Result, StepOutcome};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub(crate) fn bin(value: f32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.to_bits())?)
}
pub(crate) fn store(value: &Binary) -> Result<f32> {
    Ok(f32::from_bits(value.store_f32()?))
}
/// Native f32 constant `0x38d1b717` (0x9fc0bc / 0x9ffe7c / 0xa3aa10).
pub(crate) fn epsilon() -> Result<Binary> {
    Ok(Binary::from_f32_bits(crate::NATIVE_EPSILON_BITS)?)
}

/// Mover fields that `MoverState` does not carry: +0x28 effective maximum
/// speed and +0x30 turn radius (copied from the definition by 0x576850).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoverLimits {
    pub max_speed: f32,
    pub turn_radius: f32,
}

/// ActorDefinition MoverComponent fields read through `0x529c30`:
/// speed_max +0x228, turn_radius +0x234 and signed turn_type +0x238.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoverDefinition {
    pub speed: f32,
    pub turn_radius: f32,
    pub turn_type: i32,
}

/// Actor fields Tick reads before its first callback: actor+0xE4 is the
/// Mover itself, and the Properties component (actor+0x78) byte +0x1C.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickGate {
    pub mover_is_active_component: bool,
    pub properties_flags_1c: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TickOutcome {
    /// Mover+0x56 set: only the falling step `0x577a70` ran.
    Fell,
    /// Mover is not the active component, or `0x4d03d0` suspended it.
    Idle,
    /// Both speeds were zero and `0x577330` reported the destination near.
    Arrived,
    /// Both speeds were zero and the destination was not near (`0x5768f0`).
    Failed,
    /// Actor flag bit 12 after the overlap refresh: aircraft virtual +0x38.
    Aircraft,
    Ground(StepOutcome),
}

/// Native Tick boundaries beyond `GroundWorld`'s scheduler/Mind events.
pub trait TickWorld: GroundWorld {
    /// `0x577a70`; see `crate::falling::fall_step` for the original body.
    fn fall(&mut self, mover: &mut MoverState, actor: &mut ActorMotion, delta: f32) -> Result<()>;
    /// `0x4d67f0` overlapping-thing list refresh (expiry and spatial query).
    fn refresh_overlaps(&mut self, mover: &mut MoverState, actor: &mut ActorMotion) -> Result<()>;
    /// AIRCRAFT component (actor+0xBC) virtual +0x38 with the Tick argument.
    fn aircraft_step(&mut self, mover: &mut MoverState, actor: &mut ActorMotion, delta: f32)
    -> Result<()>;
    /// `0x5774a0` with the Tick argument as its acceleration delta. The host
    /// composes the rest of `StepContext` (GWorld+0xE8, settings, precision).
    fn ground_step(
        &mut self,
        mover: &mut MoverState,
        actor: &mut ActorMotion,
        delta: f32,
    ) -> Result<StepOutcome>;
    /// Mind virtual +0x128, called without a Mind null check natively.
    fn movement_failed(&mut self, mover: &mut MoverState, actor: &mut ActorMotion) -> Result<()>;
}

/// Original `0x4d03d0`: true only when actor flag bit0 is clear and the
/// Properties component exists with byte +0x1C bit1 set.
pub fn properties_suspend_movement(actor_flags: u32, properties_flags_1c: Option<u8>) -> bool {
    actor_flags & 1 == 0 && properties_flags_1c.is_some_and(|flags| flags & 2 != 0)
}

/// Original `0x577330`: extended XY deltas, `dx*dx + dy*dy < r*r` (strict,
/// ordered). The radius is SVar cache+0x54 (`TAIReachedDestinationThreshold`).
pub fn near_destination(
    mover: &MoverState,
    actor: &ActorMotion,
    precision_radius: f32,
    precision: Precision,
) -> Result<bool> {
    let dx = bin(mover.destination[0])?.sub(&bin(actor.position[0])?, precision)?;
    let dy = bin(mover.destination[1])?.sub(&bin(actor.position[1])?, precision)?;
    let sum = dx.mul(&dx, precision)?.add(&dy.mul(&dy, precision)?, precision)?;
    let radius = bin(precision_radius)?;
    Ok(radius.mul(&radius, precision)?.compare(&sum) == Ordering::Greater)
}

/// Original `0x5768a0`: clears turning, optionally speed, reported speed,
/// cancels the Mover schedule then notifies Mind +0x12C or deactivates.
pub fn arrival_stop<W: GroundWorld>(
    mover: &mut MoverState,
    actor: &mut ActorMotion,
    keep_speed: bool,
    world: &mut W,
) -> Result<()> {
    mover.turning = false;
    if !keep_speed {
        mover.speed = 0.0;
    }
    mover.last_step_speed = 0.0;
    world.notify(Event::CancelScheduled, mover, actor)?;
    world.notify(
        if actor.has_mind {
            Event::DestinationReached
        } else {
            Event::DeactivateMover
        },
        mover,
        actor,
    )?;
    Ok(())
}

/// Original `0x5768f0`. The native Mind +0x128 call has no null check; with no
/// Mind this returns `MissingMind` after the preceding effects.
pub fn failure_stop<W: TickWorld>(
    mover: &mut MoverState,
    actor: &mut ActorMotion,
    world: &mut W,
) -> Result<()> {
    mover.turning = false;
    mover.speed = 0.0;
    mover.last_step_speed = 0.0;
    world.notify(Event::CancelScheduled, mover, actor)?;
    world.notify(Event::DeactivateMover, mover, actor)?;
    if !actor.has_mind {
        return Err(Error::MissingMind);
    }
    world.movement_failed(mover, actor)
}

/// Complete Mover updatable Tick `0x577e50`. `delta` is the Tick argument.
/// Falling is tested before activity; the overlap refresh precedes the
/// aircraft/ground choice, and actor flag bit12 is reread after it.
pub fn tick<W: TickWorld>(
    mover: &mut MoverState,
    actor: &mut ActorMotion,
    gate: TickGate,
    delta: f32,
    precision_radius: f32,
    precision: Precision,
    world: &mut W,
) -> Result<TickOutcome> {
    if mover.falling {
        world.fall(mover, actor, delta)?;
        return Ok(TickOutcome::Fell);
    }
    if !gate.mover_is_active_component
        || properties_suspend_movement(actor.flags, gate.properties_flags_1c)
    {
        return Ok(TickOutcome::Idle);
    }
    if mover.speed == 0.0 && mover.requested_speed == 0.0 {
        return if near_destination(mover, actor, precision_radius, precision)? {
            arrival_stop(mover, actor, false, world)?;
            Ok(TickOutcome::Arrived)
        } else {
            failure_stop(mover, actor, world)?;
            Ok(TickOutcome::Failed)
        };
    }
    world.refresh_overlaps(mover, actor)?;
    if actor.flags & (1 << 12) != 0 {
        world.aircraft_step(mover, actor, delta)?;
        mover.turning = true;
        return Ok(TickOutcome::Aircraft);
    }
    Ok(TickOutcome::Ground(world.ground_step(mover, actor, delta)?))
}

/// Original `0x5767f0`; returns the native `true`.
pub fn stop_turning<W: GroundWorld>(
    mover: &mut MoverState,
    actor: &mut ActorMotion,
    world: &mut W,
) -> Result<bool> {
    mover.turning = false;
    world.notify(Event::CancelScheduled, mover, actor)?;
    Ok(true)
}

/// Original `0x576b20`: raw speed/factor words are written only when not
/// falling and Mover virtual +0x38 activation succeeds, then scheduled.
pub fn set_requested_speed<W: GroundWorld>(
    mover: &mut MoverState,
    actor: &mut ActorMotion,
    requested_speed: f32,
    speed_factor: f32,
    world: &mut W,
) -> Result<bool> {
    if mover.falling || !world.activate(mover, actor)? {
        return Ok(false);
    }
    mover.requested_speed = requested_speed;
    mover.speed_factor = speed_factor;
    world.notify(Event::ScheduleImmediate, mover, actor)?;
    Ok(true)
}

/// Original `0x576850`: copies the definition turn radius, then zeroes both
/// fields for type 0, derives +0x34 = max_speed / radius for types 1 and 2,
/// and leaves +0x34 untouched for any other signed type.
pub fn refresh_turn_rate(
    mover: &mut MoverState,
    limits: &mut MoverLimits,
    definition: MoverDefinition,
    precision: Precision,
) -> Result<()> {
    limits.turn_radius = definition.turn_radius;
    match definition.turn_type {
        0 => {
            limits.turn_radius = 0.0;
            mover.turn_rate = 0.0;
        }
        1 | 2 => {
            mover.turn_rate =
                store(&bin(limits.max_speed)?.div(&bin(definition.turn_radius)?, precision)?)?;
        }
        _ => {}
    }
    Ok(())
}

/// Boundaries of `0x576c70` after its field writes.
pub trait MaxSpeedWorld {
    /// actor+0x80 Element exists and its +0x10 organization actor is nonnull.
    fn has_organization(&mut self, actor: &mut ActorMotion) -> Result<bool>;
    /// `0x57b250` on the organization component, args (actor, old maximum).
    fn organization_speed_changed(&mut self, old_max_speed_bits: u32, actor: &mut ActorMotion)
    -> Result<()>;
    /// Mind virtual +0x118 (old maximum, new stored maximum).
    fn mind_max_speed_changed(
        &mut self,
        old_max_speed_bits: u32,
        new_max_speed_bits: u32,
        actor: &mut ActorMotion,
    ) -> Result<()>;
}

/// Original `0x576c70`. `speed_multiplier` is the modifier context +0x38 used
/// by `0x45d6f0 -> 0x5014c0`; `None` is the null context (definition speed).
/// Returns whether the maximum changed (the native routine is void).
pub fn refresh_max_speed<W: MaxSpeedWorld>(
    mover: &mut MoverState,
    limits: &mut MoverLimits,
    actor: &mut ActorMotion,
    definition: MoverDefinition,
    speed_multiplier: Option<f32>,
    precision: Precision,
    world: &mut W,
) -> Result<bool> {
    let current = match speed_multiplier {
        None => bin(definition.speed)?,
        Some(multiplier) => bin(definition.speed)?.mul(&bin(multiplier)?, precision)?,
    };
    let mut difference = bin(limits.max_speed)?.sub(&current, precision)?;
    if difference.is_negative_nonzero() {
        difference = difference.negated();
    }
    if difference.compare(&epsilon()?) == Ordering::Less {
        return Ok(false);
    }
    let old = limits.max_speed.to_bits();
    limits.max_speed = store(&current)?;
    if current.compare(&bin(mover.requested_speed)?) == Ordering::Less {
        mover.requested_speed = store(&current)?;
    }
    refresh_turn_rate(mover, limits, definition, precision)?;
    if world.has_organization(actor)? {
        world.organization_speed_changed(old, actor)?;
    }
    if actor.has_mind {
        world.mind_max_speed_changed(old, limits.max_speed.to_bits(), actor)?;
    }
    Ok(true)
}

/// Boundaries used by `0x576700`.
pub trait RestoreWorld {
    /// `0x50ac00(0)` on Mover+8.
    fn schedule_immediate(&mut self) -> Result<()>;
    /// `0x4d64f0(state, value)` actor state/property setter.
    fn actor_state(&mut self, state: u32, value: bool) -> Result<()>;
}

/// Original Mover virtual +8 `0x576700`: a falling Mover from a GWorld older
/// than version 0x16 that has no pending schedule entry (Mover+0xC null) is
/// rescheduled; every falling Mover then sets actor state 0x18. Returns true.
pub fn restore_falling<W: RestoreWorld>(
    mover: &MoverState,
    world_version: u32,
    has_schedule_entry: bool,
    world: &mut W,
) -> Result<bool> {
    if mover.falling {
        if world_version < 0x16 && !has_schedule_entry {
            world.schedule_immediate()?;
        }
        world.actor_state(0x18, true)?;
    }
    Ok(true)
}

/// Original cdecl `0x577380`: `sqrt((distance / gravity) * -2) * factor -
/// object+0x1A4`, raised to +0.0 when ordered below zero. The extended result
/// is returned unrounded, as native callers consume st(0).
pub fn launch_speed(
    distance: f32,
    factor: f32,
    object_offset_1a4: f32,
    gravity: f32,
    precision: Precision,
) -> Result<Binary> {
    let value = bin(distance)?
        .div(&bin(gravity)?, precision)?
        .mul(&Binary::from_f32_bits(0xc000_0000)?, precision)?
        .sqrt(precision)?
        .mul(&bin(factor)?, precision)?
        .sub(&bin(object_offset_1a4)?, precision)?;
    if value.compare(&bin(0.0)?) == Ordering::Less {
        return bin(0.0);
    }
    Ok(value)
}
