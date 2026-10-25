//! Native 0x53cb70 wall-entry and terrain-change search cost.
//! Resolved terrain/actor identities remain required world inputs. This module
//! preserves the original binary32 spills and callback load order.

use crate::search::{DIRECTIONS, QueueEntry};
// At PC53 each arithmetic block first runs in host binary64 (`aa_numeric::pc53`),
// falling back to `Fast` from the same operands at the same point.
use aa_numeric::pc53::{add, div, load, mul, store, sub};
use aa_numeric::{Fast, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("path cost requires an adjacent native tile: {0:?}")]
    InvalidNeighbor([i32; 2]),
    #[error("native path cost world boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Mutable fields reread after the destination actor's virtual+100 callback.
/// A host may update these synchronously; the current tile actor was already
/// resolved by address before that call, while terrain sampling uses fresh XY.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct State {
    pub current: QueueEntry,
    /// Strategy+30; zero disables terrain speed penalties.
    pub maximum_speed_bits: u32,
    /// Collision descriptor+28, an actor identity rather than a nation name.
    pub collision_owner: Option<u32>,
}

pub trait World {
    /// 45BC70 generation-resolves the tile+1C GTID.
    fn tile_actor(&mut self, cell: [i32; 2]) -> Result<Option<u32>>;
    /// Native actor+90 presence, used as the wall-component gate.
    fn has_wall_component(&self, actor: u32) -> Result<bool>;
    /// Native actor virtual+100. Its result is compared to a fresh read of
    /// state.collision_owner; mutations are committed before subsequent reads.
    fn outer_actor(&mut self, actor: u32, state: &mut State) -> Result<Option<u32>>;
    /// Original SVar PathingWallCost, loaded before the health fraction.
    fn pathing_wall_cost_bits(&mut self) -> Result<u32>;
    /// 4D0230(actor,false): Body health/maximum or None if actor has no Body.
    /// No clamping is performed by the native getter.
    fn body_health_bits(&mut self, actor: u32) -> Result<Option<[u32; 2]>>;
    /// Resolve/cache the original PathingTerrainChangePenalty setting when the
    /// terrain branch is first used; errors must not silently select a default.
    fn prepare_terrain_penalty(&mut self) -> Result<()>;
    /// 54ECE0 follows tile->variation->terrain definition, reading +38 speed.
    /// The source terrain is queried twice when its first speed is positive.
    fn terrain_speed_bits(&mut self, cell: [i32; 2]) -> Result<u32>;
    fn terrain_change_penalty_bits(&mut self) -> Result<u32>;
}

fn number(bits: u32) -> Result<Fast> {
    Ok(Fast::from_f32_bits(bits)?)
}
fn stored(value: Fast) -> Result<Fast> {
    number(value.store_f32()?)
}

/// Complete finite 53CB70 followed by caller53C9D6's binary32 store. The base
/// grid cell size and destination are captured before the virtual callback.
pub fn accumulated_cost(
    cell_size_bits: u32,
    destination: [i32; 2],
    state: &mut State,
    precision: Precision,
    world: &mut impl World,
) -> Result<u32> {
    let original_from = state.current.cell;
    let delta = [
        destination[0].wrapping_sub(original_from[0]),
        destination[1].wrapping_sub(original_from[1]),
    ];
    let direction = DIRECTIONS
        .iter()
        .position(|&value| value == delta)
        .ok_or(Error::InvalidNeighbor(destination))?;
    let multiplier = if direction & 1 != 0 {
        0x3fb5_04f3
    } else {
        0x3f80_0000
    };
    let pc53 = precision == Precision::Double53;
    //53C5C0 adds the existing cost before53CB70 subtracts it and stores f32.
    let fast_base = pc53.then(|| {
        let previous = load(state.current.cost_bits)?;
        store(sub(
            add(mul(load(multiplier)?, load(cell_size_bits)?)?, previous)?,
            previous,
        )?)
    });
    let mut step = match fast_base.flatten() {
        Some(step) => Fast::from_f32(step)?,
        None => {
            let previous = number(state.current.cost_bits)?;
            let base = number(multiplier)?
                .mul(&number(cell_size_bits)?, precision)?
                .add(&previous, precision)?;
            stored(base.sub(&previous, precision)?)?
        }
    };
    if let Some(actor) = world.tile_actor(destination)?
        && world.has_wall_component(actor)?
        && state.collision_owner.is_some()
        && world.outer_actor(actor, state)? == state.collision_owner
        && world.tile_actor(original_from)? != Some(actor)
    {
        let factor = number(world.pathing_wall_cost_bits()?)?;
        let health = if let Some([health, maximum]) = world.body_health_bits(actor)? {
            number(health)?.div(&number(maximum)?, precision)?
        } else {
            Fast::from_i32(0)
        };
        let one = Fast::from_i32(1);
        let weight = factor
            .sub(&one, precision)?
            .mul(&health, precision)?
            .add(&one, precision)?;
        step = stored(weight.mul(&step, precision)?)?;
    }
    if number(state.maximum_speed_bits)?.compare(&Fast::from_i32(0)) == Ordering::Greater {
        world.prepare_terrain_penalty()?;
        let source = number(world.terrain_speed_bits(state.current.cell)?)?;
        let reference = if source.compare(&Fast::from_i32(0)) == Ordering::Greater {
            let source = number(world.terrain_speed_bits(state.current.cell)?)?;
            let maximum = number(state.maximum_speed_bits)?;
            if maximum.compare(&source) == Ordering::Less {
                maximum
            } else {
                source
            }
        } else {
            number(state.maximum_speed_bits)?
        };
        let destination_speed = number(world.terrain_speed_bits(destination)?)?;
        if destination_speed.compare(&Fast::from_i32(0)) == Ordering::Greater
            && destination_speed.compare(&reference) == Ordering::Less
        {
            let penalty = number(world.terrain_change_penalty_bits()?)?;
            let fast = pc53.then(|| {
                let quotient = div(penalty.to_f64_exact()?, destination_speed.to_f64_exact()?)?;
                add(
                    mul(quotient, reference.to_f64_exact()?)?,
                    step.to_f64_exact()?,
                )
            });
            step = match fast.flatten() {
                Some(value) => Fast::from_f64(value)?,
                None => penalty
                    .div(&destination_speed, precision)?
                    .mul(&reference, precision)?
                    .add(&step, precision)?,
            };
        }
    }
    let fast = pc53.then(|| store(add(step.to_f64_exact()?, load(state.current.cost_bits)?)?));
    match fast.flatten() {
        Some(total) => Ok(total.to_bits()),
        None => Ok(step
            .add(&number(state.current.cost_bits)?, precision)?
            .store_f32()?),
    }
}
