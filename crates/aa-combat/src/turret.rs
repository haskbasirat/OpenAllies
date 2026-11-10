//! Performer (component +74) turret turn and aim, and the table arctangent
//! they use.
//!
//! - `0x9C8660` -> `0x408F80` / `0x408DE0`: the static arctangent table at
//!   `0xB39A78` (`[0xB39B00]`): 16384 binary32 entries `atan(i * 100 / 16384)`
//!   (x87 `fpatan` of an exact binary32 argument, stored once), limit 100.0.
//! - `0x85D9F0` / `0x85D8D0`: the table arctangent of a direction (x, y),
//!   counter-clockwise in [0, 2 pi), returned on the x87 stack.
//! - `0x57DEE0`: one turn step of turret `i` toward an angle, the short way
//!   round (inclusive half-turn test), by `rate * tick` at the running
//!   precision; true when it reached the angle.
//! - `0x57E730`: the aim of turret `i` at a point: the yaw in the owner's
//!   frame (`atan(direction) - atan(facing) - pi/2`), the reset clock and the
//!   Performer update latch, then one turn step.
//! - `0x57EF00`: the Performer's turret update: pending aims in their arc,
//!   then the turn back to rest of every turret that may reset.
//! - `0x57EE40`: stage 0 of an ability (its turret turns to the target).
//!
//! Proven bit for bit at PC24, PC53 and PC64 by
//! research/phase8-combat/native_turret_turn.py
//! (tests/native_turret_turn.rs).
use crate::{Error, Result, constructors::OwnedPerformer, constructors::Turret};
use aa_body::F32;
use aa_numeric::{Fast, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::OnceLock;

/// `0x9F32DC`
const ZERO: u32 = 0x0000_0000;
/// `0x9F32E0`
const ONE: u32 = 0x3F80_0000;
/// `0xA028D4`
const HALF_PI: u32 = 0x3FC9_0FDB;
/// `0x9FFFFC`
const PI: u32 = 0x4049_0FDB;
/// `0xAB585C`
const THREE_HALVES_PI: u32 = 0x4096_CBE4;
/// `0xA00000`
const TWO_PI: u32 = 0x40C9_0FDB;
/// `0xA03078`: below this a direction is taken as lying on an axis.
const AXIS_LIMIT: u32 = 0x3C23_D70A;
/// `0xA3DDBC`: at most this long a direction is zero.
const MIN_LENGTH: u32 = 0x38D1_B717;

/// Static table count (`0x9C8660` pushes 0x4000) and limit (`0x408F80`
/// stores 100.0 at +C).
pub const TABLE_COUNT: i32 = 0x4000;
pub const TABLE_LIMIT: F32 = F32(0x42C8_0000);

fn num(value: F32) -> Result<Fast> {
    Ok(Fast::from_f32_bits(value.0)?)
}
fn word(value: u32) -> Fast {
    Fast::from_f32_bits(value).expect("finite constant")
}
fn store(value: &Fast) -> Result<F32> {
    Ok(F32(value.store_f32()?))
}

/// The arctangent table object (`0xB39A78`): +4 count, +8 entries, +C limit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtanTable {
    pub count: i32,
    pub limit: F32,
    pub entries: Vec<F32>,
}

/// `0x408DE0` for the static table: entry `i` is the binary32 store of the
/// x87 `fpatan` (`0x409000`, unaffected by precision control) of the exact
/// binary32 `i * 100 / 16384`. The host binary64 arctangent stands in for the
/// extended one: the oracle test proves every entry equal to the native table
/// and that each lies far enough from a binary32 rounding boundary for any
/// faithful arctangent to give the same store.
pub fn build_table() -> AtanTable {
    let entries = (0..TABLE_COUNT)
        .map(|i| {
            let x = f64::from(i) * 100.0 / f64::from(TABLE_COUNT);
            F32((x.atan() as f32).to_bits())
        })
        .collect();
    AtanTable {
        count: TABLE_COUNT,
        limit: TABLE_LIMIT,
        entries,
    }
}

/// The static table, built once (`0x9C8660`).
pub fn atan_table() -> &'static AtanTable {
    static TABLE: OnceLock<AtanTable> = OnceLock::new();
    TABLE.get_or_init(build_table)
}

/// `0x85D8D0` on `table`: the angle of (x, y) in [0, 2 pi) as the x87 value
/// it leaves on the stack. A component under 0.01 in magnitude on the side it
/// divides by gives the axis angle; a ratio at or above the table limit gives
/// the quadrant base plus pi/2.
pub fn table_atan_with(table: &AtanTable, x: F32, y: F32, precision: Precision) -> Result<Fast> {
    let fx = num(x)?;
    let fy = num(y)?;
    let zero = word(ZERO);
    let limit = word(AXIS_LIMIT);
    let less = |a: &Fast, b: &Fast| a.compare(b) == Ordering::Less;
    let (ratio, base) = if less(&fx, &zero) {
        if less(&fy, &zero) {
            if less(&fx.negated(), &limit) {
                return Ok(word(THREE_HALVES_PI));
            }
            (store(&fy.div(&fx, precision)?)?, PI)
        } else {
            if less(&fy, &limit) {
                return Ok(word(PI));
            }
            (store(&fx.div(&fy, precision)?.negated())?, HALF_PI)
        }
    } else if less(&fy, &zero) {
        if less(&fy.negated(), &limit) {
            return Ok(word(ZERO));
        }
        (store(&fx.div(&fy, precision)?.negated())?, THREE_HALVES_PI)
    } else {
        if less(&fx, &limit) {
            return Ok(word(HALF_PI));
        }
        (store(&fy.div(&fx, precision)?)?, ZERO)
    };
    let ratio = num(ratio)?;
    let base = word(base);
    let table_limit = num(table.limit)?;
    if !less(&ratio, &table_limit) {
        return Ok(base.add(&word(HALF_PI), precision)?);
    }
    // fild count; fdiv limit; fmul ratio; 0x96CC4C truncates toward zero.
    let slot = Fast::from_i32(table.count)
        .div(&table_limit, precision)?
        .mul(&ratio, precision)?
        .truncate_i32()?;
    let entry = usize::try_from(slot)
        .ok()
        .and_then(|slot| table.entries.get(slot))
        .ok_or(Error::Invalid("arctangent table index outside the table"))?;
    Ok(base.add(&num(*entry)?, precision)?)
}

/// `0x85D9F0`: [`table_atan_with`] on the static table (`[0xB39B00]`).
pub fn table_atan(x: F32, y: F32, precision: Precision) -> Result<Fast> {
    table_atan_with(atan_table(), x, y, precision)
}

/// `0x4C76A0`: the turret mount offset (definition +24, +28) turned by the
/// facing vector; each word is stored once after the x87 chain.
fn turned_offset(offset: [F32; 2], facing: [F32; 2], precision: Precision) -> Result<[F32; 2]> {
    let (ox, oy) = (num(offset[0])?, num(offset[1])?);
    let (x, y) = (num(facing[0])?, num(facing[1])?);
    let first = x.mul(&ox, precision)?.sub(&y.mul(&oy, precision)?, precision)?;
    let second = ox.mul(&y, precision)?.add(&x.mul(&oy, precision)?, precision)?;
    Ok([store(&first)?, store(&second)?])
}

/// The words of a turret definition (owner ActorData +1F8[index]) the turn
/// and aim read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurretDefinition {
    /// +4 (`0x45DC30`): the rest direction in the owner's frame.
    pub yaw_start: F32,
    /// +8 (`0x45DC10`): yaw rotation per second.
    pub rate: F32,
    /// +24, +28: the mount offset.
    pub offset: [F32; 2],
    /// +2C (`0x45DCF0`): seconds without an aim before the turn back to rest.
    pub reset_time: F32,
    /// +30 (`0x45DD10`): the turret turns back to rest.
    pub can_reset: bool,
}

/// Services of the turret turn and aim.
pub trait TurnWorld {
    /// World `+E0`.
    fn time(&self) -> Result<F32>;
    /// World `+E8`.
    fn tick_length(&self) -> Result<F32>;
    /// Owner ActorData `+1FC`.
    fn turret_count(&self, owner: u32) -> Result<i32>;
    /// Owner ActorData `+1F8[index]`.
    fn turret(&self, owner: u32, index: usize) -> Result<TurretDefinition>;
    /// Owner actor `+20`, `+24`.
    fn position(&self, owner: u32) -> Result<[F32; 2]>;
    /// Owner actor `+2C`, `+30` (not normalised here).
    fn facing(&self, owner: u32) -> Result<[F32; 2]>;
    /// The handle-table check (`[0xB364C0]`: generation, live slot, actor
    /// `+8` bit 0 clear): the actor's `+20`, `+24` position when the handle
    /// names a live actor. Handle 0 never does.
    fn live_actor(&self, handle: u32) -> Result<Option<[F32; 2]>>;
    /// `0x50C290(owner, index)` on world `+130`: the yaw changed.
    fn yaw_changed(&mut self, owner: u32, index: usize) -> Result<()>;
    /// `0x50AC00(1)` on the Performer `+8` Updatable.
    fn activate_updates(&mut self) -> Result<()>;
    /// `0x50B1C0(0, 1)` on the Performer `+8` Updatable.
    fn deactivate_updates(&mut self) -> Result<()>;
    /// `0x50ACC0(0, stage)` on the Performer `+8` Updatable.
    fn post_stage(&mut self, stage: i32) -> Result<()>;
    /// `0x4C76E0(&position, &facing, &point)` on the turret definition: the
    /// point lies in the turret's arc.
    fn in_arc(&mut self, owner: u32, index: usize, point: [F32; 2]) -> Result<bool>;
}

fn record(performer: &mut OwnedPerformer, index: usize) -> Result<&mut Turret> {
    performer
        .turrets
        .get_mut(index)
        .ok_or(Error::Invalid("turret index outside the Performer +4C records"))
}

/// Native `0x57DEE0(index, target)`: one step of turret `index` toward
/// `target`. The record's `+4` takes the start angle (after the half-turn
/// correction), `+0` the new angle; `0x50C290` runs when the x87 value
/// differs from that start. True when the turret reached `target`.
pub fn turn_step(
    performer: &mut OwnedPerformer,
    index: usize,
    target: F32,
    world: &mut impl TurnWorld,
    precision: Precision,
) -> Result<bool> {
    let owner = performer.state.owner;
    let start = record(performer, index)?.current_angle;
    let tick = world.tick_length()?;
    let (s, t) = (num(start)?, num(target)?);
    let difference = t.sub(&s, precision)?;
    let distance = store(&if difference.is_negative() {
        difference.negated()
    } else {
        difference
    })?;
    let step = num(world.turret(owner, index)?.rate)?.mul(&num(tick)?, precision)?;
    let two_pi = word(TWO_PI);
    let (mut start, mut target) = (start, target);
    // fld 2pi; fsub d; fcomp d: the long way round when 2pi - d <= d.
    if two_pi
        .sub(&num(distance)?, precision)?
        .compare(&num(distance)?)
        != Ordering::Greater
    {
        if s.compare(&t) != Ordering::Less {
            target = store(&t.add(&two_pi, precision)?)?;
        } else {
            start = store(&s.add(&two_pi, precision)?)?;
        }
    }
    record(performer, index)?.target_angle = start;
    let (s, t) = (num(start)?, num(target)?);
    let (value, reached) = if s.compare(&t) == Ordering::Less {
        let value = step.add(&s, precision)?;
        if value.compare(&t) == Ordering::Less {
            (value, false)
        } else {
            (t, true)
        }
    } else {
        let value = s.sub(&step, precision)?;
        if value.compare(&t) == Ordering::Greater {
            (value, false)
        } else {
            (t, true)
        }
    };
    record(performer, index)?.current_angle = store(&value)?;
    if value.compare(&s) != Ordering::Equal {
        world.yaw_changed(owner, index)?;
    }
    Ok(reached)
}

/// Native `0x57E730(index, &point, handle)`: turret `index` aims at `point`.
/// The record's `+18` keeps `handle` when it names a live actor (else 0) and
/// `+8` the world time; the first aim of a turret that may reset turns the
/// Performer update on (`+5C`, `0x50AC00`). Then one [`turn_step`]; its
/// result is returned.
pub fn aim(
    performer: &mut OwnedPerformer,
    index: usize,
    point: [F32; 2],
    handle: u32,
    world: &mut impl TurnWorld,
    precision: Precision,
) -> Result<bool> {
    let owner = performer.state.owner;
    let kept = if world.live_actor(handle)?.is_some() {
        handle
    } else {
        0
    };
    record(performer, index)?.target[2] = F32(kept);
    let position = world.position(owner)?;
    let facing = world.facing(owner)?;
    let offset = turned_offset(world.turret(owner, index)?.offset, facing, precision)?;
    let mount_x = num(position[0])?.add(&num(offset[0])?, precision)?;
    let mount_y = num(position[1])?.add(&num(offset[1])?, precision)?;
    let dx = num(store(&num(point[0])?.sub(&mount_x, precision)?)?)?;
    // dy stays on the x87 stack.
    let dy = num(point[1])?.sub(&mount_y, precision)?;
    let length = dx
        .mul(&dx, precision)?
        .add(&dy.mul(&dy, precision)?, precision)?
        .sqrt(precision)?;
    let direction = if length.compare(&word(MIN_LENGTH)) != Ordering::Greater {
        [F32(ZERO), F32(ZERO)]
    } else {
        let inverse = word(ONE).div(&length, precision)?;
        [
            store(&dx.mul(&inverse, precision)?)?,
            store(&inverse.mul(&dy, precision)?)?,
        ]
    };
    let heading = store(&table_atan(direction[0], direction[1], precision)?)?;
    let frame = table_atan(facing[0], facing[1], precision)?;
    let value = num(heading)?
        .sub(&frame, precision)?
        .sub(&word(HALF_PI), precision)?;
    let mut yaw = store(&value)?;
    if value.compare(&word(ZERO)) == Ordering::Less {
        yaw = store(&num(yaw)?.add(&word(TWO_PI), precision)?)?;
    }
    record(performer, index)?.time = world.time()?;
    if !performer.turret_update_pending && world.turret(owner, index)?.can_reset {
        performer.turret_update_pending = true;
        world.activate_updates()?;
    }
    turn_step(performer, index, yaw, world, precision)
}

/// Native `0x57EF00`: the Performer's turret update. For every turret that
/// may reset (the count is reread each pass): a pending aim continues while
/// its point is in the arc (`0x4C76E0`, then [`aim`] with handle 0) and
/// clears when it is out of the arc or on target; once `reset_time` has
/// passed since its last aim it turns back to `yaw_start - pi/2`. `+5C` is
/// left on while any turret is still busy, else the update stops
/// (`0x50B1C0`).
pub fn reset_update(
    performer: &mut OwnedPerformer,
    world: &mut impl TurnWorld,
    precision: Precision,
) -> Result<()> {
    let owner = performer.state.owner;
    let mut busy = false;
    let mut index = 0usize;
    while i32::try_from(index).map_err(|_| Error::Invalid("turret index"))? < world.turret_count(owner)? {
        if world.turret(owner, index)?.can_reset {
            let current = record(performer, index)?;
            if current.pending {
                let point = [current.target[0], current.target[1]];
                if !world.in_arc(owner, index, point)?
                    || aim(performer, index, point, 0, world, precision)?
                {
                    record(performer, index)?.pending = false;
                } else {
                    busy = true;
                }
            }
            let elapsed = store(
                &num(world.time()?)?.sub(&num(record(performer, index)?.time)?, precision)?,
            )?;
            let definition = world.turret(owner, index)?;
            if num(definition.reset_time)?.compare(&num(elapsed)?) == Ordering::Greater {
                busy = true;
            } else {
                let value = num(definition.yaw_start)?.sub(&word(HALF_PI), precision)?;
                let mut rest = store(&value)?;
                if value.compare(&word(ZERO)) == Ordering::Less {
                    rest = store(&num(rest)?.add(&word(TWO_PI), precision)?)?;
                }
                if !turn_step(performer, index, rest, world, precision)? {
                    busy = true;
                }
            }
        }
        index += 1;
    }
    performer.turret_update_pending = busy;
    if !busy {
        world.deactivate_updates()?;
    }
    Ok(())
}

/// Native `0x57EE40`: stage 0 of the active ability. The pending aim of the
/// stage turret (`+58`) clears; the turret aims at the target (`+28`, its
/// position) or, without one, at the point (`+2C`). A target that is no
/// longer a live actor, or a turret on target, posts the stage event
/// (`0x50ACC0(0, +10)`).
pub fn stage0_aim(
    performer: &mut OwnedPerformer,
    world: &mut impl TurnWorld,
    precision: Precision,
) -> Result<()> {
    let index = usize::try_from(performer.state.turret_index)
        .map_err(|_| Error::Invalid("stage 0 without a turret"))?;
    record(performer, index)?.pending = false;
    let handle = performer.state.target_id;
    let point = if handle == 0 {
        performer.state.target_position
    } else {
        match world.live_actor(handle)? {
            Some(position) => position,
            None => return world.post_stage(performer.state.stage),
        }
    };
    if aim(performer, index, point, handle, world, precision)? {
        world.post_stage(performer.state.stage)?;
    }
    Ok(())
}
