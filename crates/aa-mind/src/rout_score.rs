//! Original rout/recover destination score `52A930` (Source/Sim/Tai).
//!
//! The CAI rout state (`52B530`, from `KKC_CAIStateRout::Think` `521240`) and
//! the SAI recover goal (`52BB40`, from `7C8AF0`) score every candidate
//! location with this function and keep the best. The score combines:
//! distance from (and angle away from) each engaged enemy, distance moved,
//! enemy ZOC/ZOG and friendly ZOS at the candidate tile, allied supply sources
//! in range and the nearest allied supply distance, plus a fuzzy random term.
//! Weights are the `Rout*` / `Recover*` SVars (Data/Game/SVars.tgi).
//!
//! Every binary32 store of the original is reproduced; unstored x87 terms use
//! the explicit precision-control mode. The function result is returned
//! unstored (ST0), exactly as the native caller receives it.
use crate::{
    Error, Result,
    x87::{n, num, store, unit_random},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// `0xA33D8C` binary32 1e-4: minimum length for a normalized direction.
const MIN_LENGTH_BITS: u32 = 0x38d1_b717;
/// `0x9F32E0` binary32 1.0.
const ONE_BITS: u32 = 0x3f80_0000;
/// Fallback direction (0, -1) written when a length is too small.
const FALLBACK_DIRECTION: [u32; 2] = [0, 0xbf80_0000];
/// `0x9FDA94` binary32 57.29578 (radians to degrees).
const DEGREES_BITS: u32 = 0x4265_2ee1;
/// Initial nearest-supply squared distance (binary32 FLT_MAX).
const FLT_MAX_BITS: u32 = 0x7f7f_ffff;

/// One SVar weight set (binary32 bits), in SVars.tgi order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Weights {
    pub base_value: u32,
    pub factor_enemy_dist: u32,
    pub factor_enemy_dist_cap: u32,
    pub factor_enemy_angle: u32,
    pub factor_moved_dist: u32,
    pub factor_moved_dist_cap: u32,
    pub factor_supply_dist: u32,
    pub factor_supply_dist_cap: u32,
    pub penalty_enemy_zoc: u32,
    pub penalty_enemy_zog: u32,
    pub bonus_friendly_zos: u32,
    pub bonus_supply_in_range: u32,
    pub fuzziness: u32,
}

/// `Rout*` (native byte argument 0) and `Recover*` (nonzero) SVars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub rout: Weights,
    pub recover: Weights,
}

/// World map geometry used to derive the candidate tile (`[B36534]+30`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapGeometry {
    /// map+4, arithmetic right shift from world units to tiles.
    pub shift: u32,
    /// map+1C, tiles per row.
    pub width: i32,
}

/// One node of a Kingdom+264 supply list: the thing position (+20/+24) and
/// its supply component+10 radius when the thing+8C component exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplySource {
    pub position: [u32; 2],
    pub radius: Option<u32>,
}

/// Snapshot state read (never written) by `52A930`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub settings: Settings,
    pub map: MapGeometry,
    /// Organization+20/+24.
    pub organization: [u32; 2],
    /// Organization+118 engaged-target GTID list, head first.
    pub engaged: Vec<u32>,
    /// `[B36534]+148` sparse Kingdom array (null slots are skipped), each with
    /// its +264 supply list.
    pub kingdoms: Vec<Option<Vec<SupplySource>>>,
}

/// Candidate arguments of `52A930`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub position: [u32; 2],
    /// Optional object (native `param_5`) whose +54/+58 position measures the
    /// moved distance; absent means distance from the organization.
    pub moved_from: Option<[u32; 2]>,
    pub recover: bool,
}

/// Required services, called in native order.
pub trait RoutWorld {
    /// GTID registry `[B364C0]` lookup and actor+20/+24; `None` when the
    /// generation does not match or the slot is empty.
    fn actor_position(&mut self, gtid: u32) -> Result<Option<[u32; 2]>>;
    /// `445400(from, to)` unstored ST0 result, with the x87 sticky precision
    /// flag set and the current precision control.
    fn angle(&mut self, from: [u32; 2], to: [u32; 2], p: Precision) -> Result<Binary>;
    /// Organization virtual `+108` (owner handle passed to the queries).
    fn owner(&mut self) -> Result<u32>;
    /// `54EFC0(tile, owner, kind)`: kind 4 enemy ZOC, kind 5 enemy ZOG.
    fn zone(&mut self, tile: i32, owner: u32, kind: u32) -> Result<u32>;
    /// `54F8B0(tile, owner, subject)` friendly ZOS. `52A930` passes the
    /// organization actor as subject (`organization == true`); the recover
    /// selection `52BB40` passes null, which takes the tile+18 owner path.
    fn friendly_zone(&mut self, tile: i32, owner: u32, organization: bool) -> Result<bool>;
    /// Kingdom virtual `+138(owner)` for sparse slot `slot`.
    fn kingdom_allied(&mut self, slot: usize, owner: u32) -> Result<bool>;
    /// One `85D840` draw on the caller's RNG word (native `param_2`).
    fn random(&mut self) -> Result<u32>;
}

/// Production binding of [`RoutWorld::angle`]: the real `445400` instruction
/// sequence through `aa_trig::signed_angle_extended`, with the sticky
/// precision flag set (it is set on every simulation tick). The phase8
/// oracle fixtures capture Unicorn's ST0, whose FPATAN is computed through
/// binary64; tests bind those captures and check this function against them
/// (identical binary32 store, within two binary64 ulps).
pub fn native_angle(from: [u32; 2], to: [u32; 2], p: Precision) -> Result<Binary> {
    let wide = aa_trig::signed_angle_extended(from, to, p, true)
        .map_err(|e| Error::World(format!("445400 angle: {e}")))?;
    crate::x87::binary_from_x87(wide as u64, (wide >> 64) as u16)
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn min_by_less(value: Binary, cap: u32) -> Result<Binary> {
    // `fcom cap; test ah,5` keeps the value only for an ordered less-than.
    Ok(if value.compare(&n(cap)?) == Ordering::Less {
        value
    } else {
        n(cap)?
    })
}
/// Length (stored binary32) and normalized direction, as at 52B0A4/52B187.
fn direction(dx: &Binary, dy: &Binary, p: Precision) -> Result<(u32, [u32; 2])> {
    let squared = num(num(dy.mul(dy, p))?.add(&num(dx.mul(dx, p))?, p))?;
    let length = num(squared.sqrt(p))?;
    let stored = store(length.clone())?;
    if length.compare(&n(MIN_LENGTH_BITS)?) != Ordering::Greater {
        return Ok((stored, FALLBACK_DIRECTION));
    }
    let inverse = num(n(ONE_BITS)?.div(&n(stored)?, p))?;
    let direction = [
        store(num(dx.mul(&inverse, p))?)?,
        store(num(dy.mul(&inverse, p))?)?,
    ];
    if host(stored) == 0.0 {
        return Ok((stored, FALLBACK_DIRECTION));
    }
    Ok((stored, direction))
}
/// `96CC4C` (_ftol2) truncation, low dword, then `sar`.
pub(crate) fn tile_coordinate(bits: u32, shift: u32) -> Result<i32> {
    let value = f64::from(host(bits));
    if !value.is_finite() || value.abs() >= 2f64.powi(63) {
        return Err(Error::World(format!(
            "rout candidate coordinate {bits:#x} has no native tile"
        )));
    }
    Ok((value.trunc() as i64 as i32) >> (shift & 31))
}

/// Complete `52A930` (diagnostic-free). Returns the unstored x87 result.
pub fn score(
    candidate: &Candidate,
    context: &Context,
    world: &mut impl RoutWorld,
    p: Precision,
) -> Result<Binary> {
    let w = if candidate.recover {
        context.settings.recover
    } else {
        context.settings.rout
    };
    let [cx, cy] = candidate.position;
    let [ox, oy] = context.organization;
    let mut score = w.base_value;
    let dx = num(n(cx)?.sub(&n(ox)?, p))?;
    let dy = num(n(cy)?.sub(&n(oy)?, p))?;
    let (organization_distance, candidate_direction) = direction(&dx, &dy, p)?;
    let count = context.engaged.iter().filter(|g| **g != 0).count();
    let count = Binary::from_i32(
        i32::try_from(count).map_err(|_| Error::World("engaged list too long".into()))?,
    );
    for &gtid in &context.engaged {
        let Some([ex, ey]) = world.actor_position(gtid)? else {
            continue;
        };
        let dx = num(n(ex)?.sub(&n(ox)?, p))?;
        let dy = num(n(ey)?.sub(&n(oy)?, p))?;
        let (_, enemy_direction) = direction(&dx, &dy, p)?;
        let dx = num(n(ex)?.sub(&n(cx)?, p))?;
        let dy = num(n(ey)?.sub(&n(cy)?, p))?;
        let squared = num(num(dy.mul(&dy, p))?.add(&num(dx.mul(&dx, p))?, p))?;
        let distance = store(num(squared.sqrt(p))?)?;
        let mut angle = world.angle(enemy_direction, candidate_direction, p)?;
        if angle.compare(&Binary::from_i32(0)) == Ordering::Less {
            angle = angle.negated();
        }
        let nearest = if host(distance) < host(w.factor_enemy_dist_cap) {
            distance
        } else {
            w.factor_enemy_dist_cap
        };
        let a = num(num(n(nearest)?.mul(&n(w.factor_enemy_dist)?, p))?.div(&count, p))?;
        let first = num(a.add(&n(score)?, p))?;
        let b = num(angle.mul(&n(DEGREES_BITS)?, p))?;
        let b = num(num(b.mul(&n(w.factor_enemy_angle)?, p))?.div(&count, p))?;
        score = store(num(first.add(&b, p))?)?;
    }
    let moved = match candidate.moved_from {
        Some([mx, my]) => {
            let dy = store(num(n(cy)?.sub(&n(my)?, p))?)?;
            let dx = num(n(cx)?.sub(&n(mx)?, p))?;
            let squared = num(num(dx.mul(&dx, p))?.add(&num(n(dy)?.mul(&n(dy)?, p))?, p))?;
            num(squared.sqrt(p))?
        }
        None => n(organization_distance)?,
    };
    let moved = min_by_less(moved, w.factor_moved_dist_cap)?;
    score = store(num(
        n(score)?.sub(&num(moved.mul(&n(w.factor_moved_dist)?, p))?, p)
    )?)?;

    let tx = tile_coordinate(cx, context.map.shift)?;
    let ty = tile_coordinate(cy, context.map.shift)?;
    let tile = context.map.width.wrapping_mul(ty).wrapping_add(tx);
    let owner = world.owner()?;
    if world.zone(tile, owner, 4)? != 0 {
        score = store(num(n(score)?.sub(&n(w.penalty_enemy_zoc)?, p))?)?;
    }
    let owner = world.owner()?;
    if world.zone(tile, owner, 5)? != 0 {
        score = store(num(n(score)?.sub(&n(w.penalty_enemy_zog)?, p))?)?;
    }
    let owner = world.owner()?;
    if world.friendly_zone(tile, owner, true)? {
        score = store(num(n(score)?.add(&n(w.bonus_friendly_zos)?, p))?)?;
    }

    let mut nearest = FLT_MAX_BITS;
    let mut found = false;
    for (slot, supplies) in context.kingdoms.iter().enumerate() {
        let Some(supplies) = supplies else { continue };
        let owner = world.owner()?;
        if !world.kingdom_allied(slot, owner)? || supplies.is_empty() {
            continue;
        }
        let mut accumulated = n(score)?;
        for source in supplies {
            let Some(radius) = source.radius else {
                continue;
            };
            let [sx, sy] = source.position;
            let dy = num(n(cy)?.sub(&n(sy)?, p))?;
            let dx = num(n(cx)?.sub(&n(sx)?, p))?;
            let squared = num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))?;
            if squared.compare(&n(nearest)?) == Ordering::Less {
                nearest = store(squared.clone())?;
                found = true;
            }
            let reach = num(n(radius)?.mul(&n(radius)?, p))?;
            if squared.compare(&reach) == Ordering::Less {
                accumulated = num(accumulated.add(&n(w.bonus_supply_in_range)?, p))?;
            }
        }
        score = store(accumulated)?;
    }
    if found {
        let distance = min_by_less(num(n(nearest)?.sqrt(p))?, w.factor_supply_dist_cap)?;
        score = store(num(
            n(score)?.sub(&num(distance.mul(&n(w.factor_supply_dist)?, p))?, p)
        )?)?;
    }
    let random = unit_random(world.random()?, p)?;
    num(num(random.mul(&n(w.fuzziness)?, p))?.add(&n(score)?, p))
}
