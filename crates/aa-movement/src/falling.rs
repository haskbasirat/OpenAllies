//! Original Mover falling motion: starts `0x576740` / `0x5770a0`, the falling
//! step `0x577a70`, its pose commit `0x576bb0`, map interior test `0x5766a0`,
//! interior clamp `0x552640` and vector normalization `0x4453a0`.
//!
//! The falling state uses Mover+0x20 (`MoverState::speed`) as its scalar speed
//! and Mover+0x56 (`MoverState::falling`) as its flag; the remaining fields
//! live in `FallingState`. All arithmetic uses explicit x87 precision and the
//! native binary32 store points. Every world effect is a required callback.
use crate::mover::{bin, epsilon, store};
use crate::{ActorMotion, Error, MoverState, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Mover +0x58..+0x88 falling fields.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FallingState {
    /// +0x58 per-second speed damping factor.
    pub drag: f32,
    /// +0x5C byte; nonzero makes the actor face along the fall direction.
    pub orient_to_motion: u8,
    /// +0x60..+0x68 unit direction (initially the actor facing).
    pub direction: [f32; 3],
    /// +0x6C byte; set by `0x5770a0`, which aims the fall at a ground point.
    pub aimed: u8,
    /// +0x70..+0x78 aimed ground point (Z from the terrain height query).
    pub aim_point: [f32; 3],
    /// +0x7C..+0x84 aimed point minus the start position.
    pub aim_delta: [f32; 3],
    /// +0x88 byte; the last pose commit was clamped into the map interior.
    pub outside_map: u8,
}

/// GWorld+0x30 map fields read by `0x5766a0` and `0x552640`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MapBounds {
    /// map+0x08: grid cell size, also the interior minimum.
    pub cell_size: f32,
    /// map+0x0C/+0x10: world extents.
    pub extent: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FallContext {
    /// The Tick argument passed to `0x577a70`.
    pub delta: f32,
    /// SVar cache+0x2C (`Gravity`; original SVars.tgi authors -49.05).
    pub gravity: f32,
    /// Resolved `PlaneFallGravityTweak` SVar (native registration default
    /// binary32 0x3ba3d70a at 0x508eed). Required only for aircraft.
    pub plane_fall_gravity_tweak: Option<f32>,
    pub map: MapBounds,
    pub precision: Precision,
}

/// Falling boundaries, in native call order. Callbacks may mutate the passed
/// snapshots; the routines reread fields exactly where the original does.
pub trait FallingWorld {
    /// actor+0xBC AIRCRAFT component presence (read twice by `0x577a70`).
    fn has_aircraft(&mut self, actor: &mut ActorMotion) -> Result<bool>;
    /// ActorDefinition (actor+4) +0x40C, added to the aircraft ground height.
    fn aircraft_height_offset(&mut self, actor: &mut ActorMotion) -> Result<f32>;
    /// `0x4f0e50` with flags 0x122 at the given XY; returns the extended value.
    fn ground_height(
        &mut self,
        xy: [f32; 2],
        mover: &mut MoverState,
        falling: &mut FallingState,
        actor: &mut ActorMotion,
    ) -> Result<Binary>;
    /// `0x4d04b0` (first clamped commit) and `0x4d7ea0` (back inside).
    fn left_map(&mut self, actor: &mut ActorMotion) -> Result<()>;
    fn entered_map(&mut self, actor: &mut ActorMotion) -> Result<()>;
    /// `0x4ee5c0` position commit and actor virtual +0x54 facing setter.
    fn commit_position(&mut self, position: [f32; 3], actor: &mut ActorMotion) -> Result<()>;
    fn set_facing(&mut self, facing: [f32; 3], actor: &mut ActorMotion) -> Result<()>;
    /// `0x4d64f0(state, value)`; falling uses state 0x18.
    fn actor_state(&mut self, state: u32, value: bool, actor: &mut ActorMotion) -> Result<()>;
    /// `0x50ac00(0)` / `0x50b250` on Mover+8.
    fn schedule_immediate(&mut self, actor: &mut ActorMotion) -> Result<()>;
    fn cancel_scheduled(&mut self, actor: &mut ActorMotion) -> Result<()>;
    /// `0x4d64a0` landing handler.
    fn landed(&mut self, actor: &mut ActorMotion) -> Result<()>;
}

/// Original `0x5766a0`: `cell <= x <= (extent_x - cell) - 1e-4` and the same
/// for Y. The limits stay extended (no binary32 store); unordered fails.
pub fn inside_map(map: MapBounds, xy: [f32; 2], precision: Precision) -> Result<bool> {
    let cell = bin(map.cell_size)?;
    for (axis, value) in xy.into_iter().enumerate() {
        let value = bin(value)?;
        if value.compare(&cell) == Ordering::Less {
            return Ok(false);
        }
        let limit = bin(map.extent[axis])?
            .sub(&cell, precision)?
            .sub(&epsilon()?, precision)?;
        if limit.compare(&value) == Ordering::Less {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Original `0x552640`: the upper limit is stored to binary32 first; a value
/// below the cell size is replaced and skips that axis' upper test.
pub fn clamp_to_interior(map: MapBounds, xy: &mut [f32; 2], precision: Precision) -> Result<()> {
    let cell = bin(map.cell_size)?;
    for (axis, value) in xy.iter_mut().enumerate() {
        let limit = store(
            &bin(map.extent[axis])?
                .sub(&cell, precision)?
                .sub(&epsilon()?, precision)?,
        )?;
        if cell.compare(&bin(*value)?) == Ordering::Greater {
            *value = map.cell_size;
        } else if bin(*value)?.compare(&bin(limit)?) == Ordering::Greater {
            *value = limit;
        }
    }
    Ok(())
}

/// Original `0x4453a0`: `sqrt((x*x + y*y) + z*z)`; above binary32 1e-4 each
/// component is multiplied by the extended reciprocal and stored, otherwise
/// the vector is zeroed. Returns the extended length or +0.0.
pub fn normalize(vector: &mut [f32; 3], precision: Precision) -> Result<Binary> {
    let [x, y, z] = vector.map(bin);
    let (x, y, z) = (x?, y?, z?);
    let length = x
        .mul(&x, precision)?
        .add(&y.mul(&y, precision)?, precision)?
        .add(&z.mul(&z, precision)?, precision)?
        .sqrt(precision)?;
    if length.compare(&epsilon()?) != Ordering::Greater {
        *vector = [0.0; 3];
        return bin(0.0);
    }
    let inverse = bin(1.0)?.div(&length, precision)?;
    for component in vector.iter_mut() {
        *component = store(&inverse.mul(&bin(*component)?, precision)?)?;
    }
    Ok(length)
}

fn begin_common<W: FallingWorld>(
    mover: &mut MoverState,
    falling: &mut FallingState,
    actor: &mut ActorMotion,
    drag: f32,
    orient_to_motion: u8,
    world: &mut W,
) -> Result<()> {
    falling.orient_to_motion = orient_to_motion;
    falling.drag = drag;
    mover.falling = true;
    falling.direction = actor.facing;
    falling.aimed = 0;
    world.actor_state(0x18, true, actor)?;
    world.schedule_immediate(actor)
}

/// Original `0x576740`: start an unaimed fall along the current facing.
pub fn begin_fall<W: FallingWorld>(
    mover: &mut MoverState,
    falling: &mut FallingState,
    actor: &mut ActorMotion,
    drag: f32,
    orient_to_motion: u8,
    world: &mut W,
) -> Result<bool> {
    begin_common(mover, falling, actor, drag, orient_to_motion, world)?;
    Ok(true)
}

/// Original `0x5770a0`: the unaimed prefix, then aim at (x, y, ground height).
/// The aim point is written before the height query and the deltas use the
/// actor position read after it.
#[allow(clippy::too_many_arguments)]
pub fn begin_aimed_fall<W: FallingWorld>(
    mover: &mut MoverState,
    falling: &mut FallingState,
    actor: &mut ActorMotion,
    drag: f32,
    target: [f32; 2],
    orient_to_motion: u8,
    precision: Precision,
    world: &mut W,
) -> Result<bool> {
    begin_common(mover, falling, actor, drag, orient_to_motion, world)?;
    falling.aim_point = [target[0], target[1], 0.0];
    let height = world.ground_height(target, mover, falling, actor)?;
    falling.aim_point[2] = store(&height)?;
    for axis in 0..3 {
        falling.aim_delta[axis] =
            store(&bin(falling.aim_point[axis])?.sub(&bin(actor.position[axis])?, precision)?)?;
    }
    falling.aimed = 1;
    Ok(true)
}

/// Original `0x576bb0` pose commit (position, facing and speed argument):
/// interior test/clamp with the +0x88 left/entered-map transitions, Mover
/// +0x20 speed write, position commit and actor facing virtual. Shared by the
/// falling step and the aircraft step (`crate::aircraft`).
#[allow(clippy::too_many_arguments)]
pub fn commit_pose<W: FallingWorld>(
    mover: &mut MoverState,
    falling: &mut FallingState,
    actor: &mut ActorMotion,
    position: [f32; 3],
    facing: [f32; 3],
    speed: f32,
    map: MapBounds,
    precision: Precision,
    world: &mut W,
) -> Result<()> {
    let mut xy = [position[0], position[1]];
    if !inside_map(map, xy, precision)? {
        clamp_to_interior(map, &mut xy, precision)?;
        if falling.outside_map == 0 {
            world.left_map(actor)?;
            falling.outside_map = 1;
        }
    } else if falling.outside_map != 0 {
        world.entered_map(actor)?;
        falling.outside_map = 0;
    }
    mover.speed = speed;
    world.commit_position([xy[0], xy[1], position[2]], actor)?;
    world.set_facing(facing, actor)
}

/// Complete falling step `0x577a70` (returns the native `true`).
pub fn fall_step<W: FallingWorld>(
    mover: &mut MoverState,
    falling: &mut FallingState,
    actor: &mut ActorMotion,
    context: &FallContext,
    world: &mut W,
) -> Result<bool> {
    let pc = context.precision;
    let delta = bin(context.delta)?;
    let zero = bin(0.0)?;
    // Damped scalar speed: stored, then raised to zero when negative.
    let mut speed = bin(1.0)?
        .sub(&delta.mul(&bin(falling.drag)?, pc)?, pc)?
        .mul(&bin(mover.speed)?, pc)?;
    mover.speed = store(&speed)?;
    if zero.compare(&speed) == Ordering::Greater {
        speed = zero.clone();
    }
    mover.speed = store(&speed)?;
    let velocity_z = store(&speed.mul(&bin(falling.direction[2])?, pc)?)?;
    let velocity_y = store(&speed.mul(&bin(falling.direction[1])?, pc)?)?;
    let velocity_x = store(&speed.mul(&bin(falling.direction[0])?, pc)?)?;
    let vertical = if world.has_aircraft(actor)? {
        let tweak = context
            .plane_fall_gravity_tweak
            .ok_or_else(|| Error::World("PlaneFallGravityTweak value is required".into()))?;
        bin(tweak)?
            .mul(&bin(mover.speed)?, pc)?
            .mul(&bin(context.gravity)?, pc)?
            .mul(&delta, pc)?
            .add(&bin(velocity_z)?, pc)?
    } else {
        delta.mul(&bin(context.gravity)?, pc)?
    };
    let x = bin(velocity_x)?;
    let y = bin(velocity_y)?;
    let magnitude = store(
        &x.mul(&x, pc)?
            .add(&y.mul(&y, pc)?, pc)?
            .add(&vertical.mul(&vertical, pc)?, pc)?
            .sqrt(pc)?,
    )?;
    let step_x = x.mul(&delta, pc)?;
    mover.speed = magnitude;
    let step_y = y.mul(&delta, pc)?;
    let start = actor.position;
    let step_z = store(&vertical.mul(&delta, pc)?)?;
    let mut next = [
        store(&step_x.add(&bin(start[0])?, pc)?)?,
        store(&bin(start[1])?.add(&step_y, pc)?)?,
        store(&bin(start[2])?.add(&bin(step_z)?, pc)?)?,
    ];
    let mut direction;
    if falling.aimed != 0 {
        let t = bin(next[2])?
            .sub(&bin(falling.aim_point[2])?, pc)?
            .div(&bin(falling.aim_delta[2])?, pc)?
            .negated();
        let offset_x = t.mul(&bin(falling.aim_delta[0])?, pc)?;
        let offset_y = store(&t.mul(&bin(falling.aim_delta[1])?, pc)?)?;
        let offset_z = store(&t.mul(&bin(falling.aim_delta[2])?, pc)?)?;
        next = [
            store(&bin(falling.aim_point[0])?.sub(&offset_x, pc)?)?,
            store(&bin(falling.aim_point[1])?.sub(&bin(offset_y)?, pc)?)?,
            store(&bin(falling.aim_point[2])?.sub(&bin(offset_z)?, pc)?)?,
        ];
        direction = [0.0; 3];
        for axis in 0..3 {
            direction[axis] = store(&bin(next[axis])?.sub(&bin(start[axis])?, pc)?)?;
        }
    } else if magnitude != 0.0 {
        let inverse = bin(1.0)?.div(&bin(magnitude)?, pc)?;
        direction = [
            store(&x.mul(&inverse, pc)?)?,
            store(&y.mul(&inverse, pc)?)?,
            store(&inverse.mul(&vertical, pc)?)?,
        ];
    } else {
        direction = actor.facing;
    }
    let chosen = if normalize(&mut direction, pc)?.is_zero() {
        actor.facing
    } else {
        direction
    };
    let mut height = world.ground_height([next[0], next[1]], mover, falling, actor)?;
    if world.has_aircraft(actor)? {
        height = height.add(&bin(world.aircraft_height_offset(actor)?)?, pc)?;
    }
    let landed = bin(next[2])?.compare(&height) != Ordering::Greater;
    if landed {
        next[2] = store(&height)?;
    }
    falling.direction = chosen;
    let facing = if falling.orient_to_motion != 0 {
        chosen
    } else {
        actor.facing
    };
    let speed_argument = mover.speed;
    commit_pose(
        mover,
        falling,
        actor,
        next,
        facing,
        speed_argument,
        context.map,
        pc,
        world,
    )?;
    if landed {
        mover.falling = false;
        world.actor_state(0x18, false, actor)?;
        world.cancel_scheduled(actor)?;
        mover.speed = 0.0;
        world.landed(actor)?;
    }
    Ok(true)
}
