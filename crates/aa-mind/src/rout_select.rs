//! Original rout destination selection `52B530` (Source/Sim/Tai, CAI rout).
//!
//! Called by `KKC_CAIStateRout::Think` (`521240`) with the CAI Mind as `this`
//! and the world RNG (`4F02B0`). It generates `RoutRandomCandidates` random
//! candidates around the organization (direction from the global direction
//! table `[B39AF8]`, distance `min + u * allowed`), then
//! `RoutSupplyZoneCandidates` candidates toward every allied Kingdom supply
//! source (direction rotated by a random angle in
//! `[-RoutSupplyZoneMaxAngleDelta, +RoutSupplyZoneMaxAngleDelta]` through the
//! original 4096-entry sin/cos tables). Each candidate that `4F46E0` can place
//! is scored by `52A930` (`crate::rout_score::score`); the strictly highest
//! score wins. Returns `None` when the best score stays -FLT_MAX.
//!
//! Every binary32 store of the original is reproduced, including the
//! asymmetric x/y stores of the two candidate formulas.
use crate::{
    Error, Result,
    rout_score::{self, Candidate, RoutWorld},
    x87::{n, num, store, unit_random},
};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// `0xA33D8C` binary32 1e-4.
const MIN_LENGTH_BITS: u32 = 0x38d1_b717;
/// `0x9F32E0` binary32 1.0.
const ONE_BITS: u32 = 0x3f80_0000;
/// Initial best score and the final "no candidate" constant `0xA342F0`.
const NEGATIVE_FLT_MAX_BITS: u32 = 0xff7f_ffff;
/// Radius argument passed to `4F46E0` (binary32 4.0).
pub const OPEN_POSITION_RADIUS_BITS: u32 = 0x4080_0000;

/// Inputs read by `52B530` besides the `52A930` context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub score: rout_score::Context,
    /// Organization+7C component +AC (binary32): minimum flee distance.
    pub min_distance: u32,
    /// SVar `RoutRandomCandidates` (int).
    pub random_candidates: i32,
    /// SVar `RoutMaxDistanceDelta` (binary32).
    pub max_distance_delta: u32,
    /// SVar `RoutSupplyZoneCandidates` (int).
    pub supply_candidates: i32,
    /// SVar `RoutSupplyZoneMaxAngleDelta` (binary32), used as a table angle.
    pub supply_max_angle_delta: u32,
    /// Global direction table `[B39AF8]` (+4 count, +8 pairs).
    pub directions: Vec<[u32; 2]>,
}

/// Services beyond those of `52A930`, called in native order.
pub trait SelectWorld: RoutWorld {
    /// Organization virtual `+4`; its x87 result is discarded by `52B530`.
    fn organization_virtual_4(&mut self) -> Result<()>;
    /// `52A690(origin, direction, base, delta)` stored to binary32: the
    /// usable extra distance along `direction` before the map edge.
    fn reachable_delta(
        &mut self,
        origin: [u32; 2],
        direction: [u32; 2],
        base: u32,
        delta: u32,
    ) -> Result<u32>;
    /// World `4F46E0(organization, &position, 4.0)`. May move `position`.
    fn find_open_position(&mut self, position: &mut [u32; 2], radius: u32) -> Result<bool>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}

struct Best {
    score: u32,
    position: [u32; 2],
}
impl Best {
    fn offer(
        &mut self,
        mut position: [u32; 2],
        context: &Context,
        world: &mut impl SelectWorld,
        p: Precision,
    ) -> Result<()> {
        if !world.find_open_position(&mut position, OPEN_POSITION_RADIUS_BITS)? {
            return Ok(());
        }
        let candidate = Candidate {
            position,
            moved_from: None,
            recover: false,
        };
        let value = rout_score::score(&candidate, &context.score, world, p)?;
        // `fcom best; test ah,41; jne`: replace only on strict greater-than.
        if value.compare(&n(self.score)?) == Ordering::Greater {
            self.score = store(value)?;
            self.position = position;
        }
        Ok(())
    }
}

/// Complete `52B530`.
pub fn select(
    context: &Context,
    world: &mut impl SelectWorld,
    p: Precision,
) -> Result<Option<[u32; 2]>> {
    world.organization_virtual_4()?;
    let [ox, oy] = context.score.organization;
    let mut best = Best {
        score: NEGATIVE_FLT_MAX_BITS,
        position: [0, 0],
    };
    let mut i = 0;
    while i < context.random_candidates {
        let count = context.directions.len() as u32;
        if count == 0 {
            return Err(Error::World(
                "rout direction table is empty (native divide fault)".into(),
            ));
        }
        let draw = world.random()?;
        let direction = context.directions[(draw % count) as usize];
        let allowed = world.reachable_delta(
            context.score.organization,
            direction,
            context.min_distance,
            context.max_distance_delta,
        )?;
        let u = unit_random(world.random()?, p)?;
        let distance = num(num(u.mul(&n(allowed)?, p))?.add(&n(context.min_distance)?, p))?;
        let offset_y = num(n(direction[1])?.mul(&distance, p))?;
        let offset_x = store(num(n(direction[0])?.mul(&distance, p))?)?;
        let y = num(n(oy)?.add(&offset_y, p))?;
        let x = store(num(n(offset_x)?.add(&n(ox)?, p))?)?;
        // The y equality test uses the unstored sum.
        let unchanged = host(x) == host(ox) && y.compare(&n(oy)?) == Ordering::Equal;
        if !unchanged {
            best.offer([x, store(y)?], context, world, p)?;
        }
        i += 1;
    }
    for (slot, supplies) in context.score.kingdoms.iter().enumerate() {
        let Some(supplies) = supplies else { continue };
        let owner = world.owner()?;
        if !world.kingdom_allied(slot, owner)? || supplies.is_empty() {
            continue;
        }
        for source in supplies {
            if source.radius.is_none() || context.supply_candidates <= 0 {
                continue;
            }
            let [sx, sy] = source.position;
            let mut j = 0;
            while j < context.supply_candidates {
                let dy = store(num(n(sy)?.sub(&n(oy)?, p))?)?;
                let dx = store(num(n(sx)?.sub(&n(ox)?, p))?)?;
                let u = unit_random(world.random()?, p)?;
                let spread = num(u.mul(&n(context.supply_max_angle_delta)?, p))?;
                let spread = num(spread.add(&spread, p))?;
                let angle = store(num(spread.sub(&n(context.supply_max_angle_delta)?, p))?)?;
                let squared =
                    num(num(n(dx)?.mul(&n(dx)?, p))?.add(&num(n(dy)?.mul(&n(dy)?, p))?, p))?;
                let length = num(squared.sqrt(p))?;
                let mut direction = [0, 0xbf80_0000];
                if length.compare(&n(MIN_LENGTH_BITS)?) == Ordering::Greater {
                    let inverse = num(n(ONE_BITS)?.div(&length, p))?;
                    direction = [
                        store(num(n(dx)?.mul(&inverse, p))?)?,
                        store(num(inverse.mul(&n(dy)?, p))?)?,
                    ];
                    if length.is_zero() {
                        direction = [0, 0xbf80_0000];
                    }
                }
                let sin = n(trig(aa_trig::lookup_sin(angle, p))?)?;
                let cos = n(trig(aa_trig::lookup_cos(angle, p))?)?;
                let [dir_x, dir_y] = [n(direction[0])?, n(direction[1])?];
                let turned_x = store(num(
                    num(dir_x.mul(&cos, p))?.sub(&num(dir_y.mul(&sin, p))?, p)
                )?)?;
                let turned_y = store(num(
                    num(dir_y.mul(&cos, p))?.add(&num(sin.mul(&dir_x, p))?, p)
                )?)?;
                let allowed = world.reachable_delta(
                    context.score.organization,
                    [turned_x, turned_y],
                    context.min_distance,
                    context.max_distance_delta,
                )?;
                let u = unit_random(world.random()?, p)?;
                let distance = num(num(u.mul(&n(allowed)?, p))?.add(&n(context.min_distance)?, p))?;
                let offset_y = store(num(n(turned_y)?.mul(&distance, p))?)?;
                let offset_x = num(distance.mul(&n(turned_x)?, p))?;
                let y = store(num(n(oy)?.add(&n(offset_y)?, p))?)?;
                let x = store(num(offset_x.add(&n(ox)?, p))?)?;
                best.offer([x, y], context, world, p)?;
                j += 1;
            }
        }
    }
    Ok((best.score != NEGATIVE_FLT_MAX_BITS).then_some(best.position))
}

fn trig(value: std::result::Result<u32, aa_trig::Error>) -> Result<u32> {
    value.map_err(|e| Error::World(format!("rout supply direction lookup: {e}")))
}
