//! Army display positions on the strategic map: `KKC_TerritoryPositionSolver`
//! (`.\Source\Sim\TerritoryPositionSolver.cpp`) and its callers.
//!
//! Ported from the original machine code (addresses of the 32-bit AA.exe) and
//! proven by `tests/native_positions.rs` against `research/phase8-metagame/
//! native-positions.json` (original code under Unicorn at x87 PC53, PC24 and
//! PC64) and the InitCreate positions of `native-init.json`.
//!
//! Positions are engine actor positions (display data): the native code hands
//! them to the engine (army actor creation descriptor 4FA030, set position
//! 4EE5C0, move-order waypoints 9627D0) and keeps none of them in metagame
//! objects, so they are returned here and not stored in [`MgState`].
//!
//! Native objects:
//! - solver (ctor 959220, vtable AC68D4, 0x0C bytes): `+0x04` MG territory,
//!   `+0x08` owner kingdom actor (stored, never read by 959420).
//! - spacing layouts: the `actor_spacing_type` enum table (static init 959B20
//!   on B4A524 -> 959A50; entry `+0x08` = layout object, indexed by the enum
//!   value through `[B4A528]`): SIMPLE = 959760 (vtable ACC408, 8 offsets),
//!   CIRCLE = vtable ACC2E8 with ring base 6 (959560), SQUARE = vtable
//!   ACC2F0 with ring base 8 (959240). Slot `+0x04` is the layout call
//!   `(out, center, n, distance)`.
//! - 4096-entry binary32 cos/sin tables B39A48/B39A60 (static init 9C8620 /
//!   9C8640), read by 85DA80/85DAA0 -> 85DA10: ported by `aa_trig`.
//!
//! Every float rule takes the x87 precision explicitly; the game passes
//! [`crate::GAME_PRECISION`]. Binary32 values are passed as bit patterns.
use crate::data::{ActorSpacing, MapDefinition, MetaGameData, TerritoryDefinition, TerritoryKind};
use crate::path::PathGraph;
use crate::state::{ArmyId, MgState};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// 2*pi as binary32 (A00000), the circle angle period.
pub const TWO_PI_BITS: u32 = 0x40c9_0fdb;
/// Ring base count of the CIRCLE layout (959A50: object ACC2E8 `+0x04 = 6`).
pub const CIRCLE_RING_BASE: i32 = 6;
/// Ring base count of the SQUARE layout (959A50: object ACC2F0 `+0x04 = 8`).
pub const SQUARE_RING_BASE: i32 = 8;
/// SIMPLE layout offsets (959760, appended in this order by 958050).
pub const SIMPLE_OFFSETS: [[u32; 3]; 8] = [
    [0, 0x3f80_0000, 0],
    [0, 0xbf80_0000, 0],
    [0x3f80_0000, 0, 0],
    [0xbf80_0000, 0, 0],
    [0x3f80_0000, 0x3f80_0000, 0],
    [0x3f80_0000, 0xbf80_0000, 0],
    [0xbf80_0000, 0x3f80_0000, 0],
    [0xbf80_0000, 0xbf80_0000, 0],
];
/// Height 2.0 written over the solved z by 955B70/955820 into the army
/// creation descriptor (`+0x10`).
pub const PLACEMENT_Z_BITS: u32 = 0x4000_0000;
/// Facing written by 955B70/955820 into the creation descriptor
/// (`+0x14`, `+0x18`: about 0.7071, -0.7071).
pub const PLACEMENT_FACING_BITS: [u32; 2] = [0x3f34_fdf4, 0xbf34_fdf4];
/// The index argument of 959420 that selects the first free display slot.
pub const FIRST_FREE: i32 = -1;

fn f(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

fn trig(e: aa_trig::Error) -> Error {
    Error::Unsupported(format!("85DA10 trig lookup: {e}"))
}

/// Rings shared by 959560/959240: ring `r` holds `base * (r + 1)` entries;
/// returns `(r, index within the ring, ring size)`. The native loop
/// (`cmp total, n; jg`) never ends when `n` is so large that the running
/// total wraps; that input is rejected.
fn ring(base: i32, n: i32) -> Result<(i32, i32, i32)> {
    let mut total = i64::from(base);
    let mut r = 0i64;
    let mut m = 1i64;
    while total <= i64::from(n) {
        r += 1;
        m += 1;
        total += i64::from(base) * m;
        if total > i64::from(i32::MAX) {
            return Err(Error::Unsupported(format!("spacing ring loop with n = {n} never ends natively")));
        }
    }
    let r = r as i32;
    // k*((r+2)(r-1)+2), then cdq/sub/sar: truncating halving.
    let before = base.wrapping_mul((r + 2).wrapping_mul(r - 1).wrapping_add(2)) / 2;
    Ok((r, n.wrapping_sub(before), base.wrapping_mul(r + 1)))
}

/// `FUN_00959560` (CIRCLE, ring base 6): ring `r`, index `i`, ring size `c`;
/// radius = f32((r+1) * distance); angle = f32(-(i * (2pi / c))) (`fdivr`,
/// `fmulp`, `fchs`); sin/cos by the 4096-entry lookups (85DAA0/85DA80);
/// x = f32((radius*cos - sin*0.0) + cx) with the difference unrounded, y =
/// f32(f32(sin*radius + cos*0.0) + cy), z = 0.
pub fn circle_position(center: [u32; 3], n: i32, distance_bits: u32, precision: Precision) -> Result<[u32; 3]> {
    let (r, i, c) = ring(CIRCLE_RING_BASE, n)?;
    let radius = f(Binary::from_i32(r + 1).mul(&f(distance_bits)?, precision)?.store_f32()?)?;
    let step = f(TWO_PI_BITS)?.div(&Binary::from_i32(c), precision)?;
    let angle = Binary::from_i32(i).mul(&step, precision)?.negated().store_f32()?;
    let sin = f(aa_trig::lookup_sin(angle, precision).map_err(trig)?)?;
    let cos = f(aa_trig::lookup_cos(angle, precision).map_err(trig)?)?;
    let zero = f(0)?;
    let x = radius
        .mul(&cos, precision)?
        .sub(&sin.mul(&zero, precision)?, precision)?
        .add(&f(center[0])?, precision)?
        .store_f32()?;
    let y = sin
        .mul(&radius, precision)?
        .add(&cos.mul(&zero, precision)?, precision)?
        .store_f32()?;
    let y = f(y)?.add(&f(center[1])?, precision)?.store_f32()?;
    Ok([x, y, 0])
}

/// `FUN_00959240` (SQUARE, ring base 8): ring `r`, index `i`, ring size
/// `c`; side = c/4, side q = i / side, j = i % side (`idiv`); A = (r+1) *
/// distance, B = j * distance (x87, unrounded). Sides (jump table 95937C):
/// 0: (f32(B - A), f32(-A)); 1: (A, f32(B - A)); 2: (f32(A - B), f32(A));
/// 3: (-A, f32(A - B)); other: (0.0, 0.0). Then x + cx, y + cy (binary32
/// stores), z = 0. A side of 0 is a native divide fault (not reachable with
/// base 8).
pub fn square_position(center: [u32; 3], n: i32, distance_bits: u32, precision: Precision) -> Result<[u32; 3]> {
    let (r, i, c) = ring(SQUARE_RING_BASE, n)?;
    let side = c / 4;
    if side == 0 {
        return Err(Error::Unsupported("959240: side length 0 (native divide fault)".into()));
    }
    let (q, j) = (i / side, i % side);
    let d = f(distance_bits)?;
    let a = Binary::from_i32(r.wrapping_add(1)).mul(&d, precision)?;
    let b = Binary::from_i32(j).mul(&d, precision)?;
    let minus_a = a.clone().negated();
    let (x, y_bits) = match q as u32 {
        0 => (f(b.add(&minus_a, precision)?.store_f32()?)?, minus_a.store_f32()?),
        1 => (a.clone(), b.add(&minus_a, precision)?.store_f32()?),
        2 => (f(a.sub(&b, precision)?.store_f32()?)?, a.store_f32()?),
        3 => (minus_a, a.sub(&b, precision)?.store_f32()?),
        _ => (f(0)?, 0),
    };
    let x = x.add(&f(center[0])?, precision)?.store_f32()?;
    let y = f(y_bits)?.add(&f(center[1])?, precision)?.store_f32()?;
    Ok([x, y, 0])
}

/// `FUN_00959390` (SIMPLE): offset `n % 8` of [`SIMPLE_OFFSETS`] scaled by
/// m = n/8 + 1 and the distance: x = f32((off.x*m)*d + cx) with the product
/// unrounded, y = f32(f32((m*off.y)*d) + cy), z = off.z.
pub fn simple_position(center: [u32; 3], n: i32, distance_bits: u32, precision: Precision) -> Result<[u32; 3]> {
    let count = SIMPLE_OFFSETS.len() as i32;
    let (q, r) = (n / count, n % count);
    if r < 0 {
        return Err(Error::Unsupported("959390: negative offset index (native reads before the array)".into()));
    }
    let off = SIMPLE_OFFSETS[r as usize];
    let m = Binary::from_i32(q.wrapping_add(1));
    let d = f(distance_bits)?;
    let x = f(off[0])?
        .mul(&m, precision)?
        .mul(&d, precision)?
        .add(&f(center[0])?, precision)?
        .store_f32()?;
    let y = m.mul(&f(off[1])?, precision)?.mul(&d, precision)?.store_f32()?;
    let y = f(y)?.add(&f(center[1])?, precision)?.store_f32()?;
    Ok([x, y, off[2]])
}

/// The layout call (layout vtable `+0x04`) of the `actor_spacing_type` value.
pub fn layout_position(
    spacing: ActorSpacing,
    center: [u32; 3],
    n: i32,
    distance_bits: u32,
    precision: Precision,
) -> Result<[u32; 3]> {
    match spacing {
        ActorSpacing::Simple => simple_position(center, n, distance_bits, precision),
        ActorSpacing::Circle => circle_position(center, n, distance_bits, precision),
        ActorSpacing::Square => square_position(center, n, distance_bits, precision),
    }
}

fn map<'a>(state: &MgState, data: &'a MetaGameData) -> Result<&'a MapDefinition> {
    data.campaigns
        .get(state.campaign)
        .and_then(|c| c.map)
        .and_then(|m| data.maps.get(m))
        .ok_or_else(|| Error::Unsupported("campaign without map".into()))
}

fn definition<'a>(state: &MgState, map: &'a MapDefinition, territory: usize) -> Result<&'a TerritoryDefinition> {
    let t = state
        .territories
        .get(territory)
        .ok_or_else(|| Error::Unsupported(format!("MG territory {territory} out of range")))?;
    map.territories
        .get(t.definition)
        .ok_or_else(|| Error::Unsupported(format!("definition {} out of range", t.definition)))
}

/// `FUN_00954A10`: first free display slot of a territory (length of the
/// prefix of the slot-ordered army list whose slots are 0, 1, 2, ...).
pub fn first_free_slot(state: &MgState, territory: usize) -> i32 {
    let refs = &state.territories[territory].armies;
    refs.iter().enumerate().take_while(|(i, r)| r.slot == *i as i32).count() as i32
}

/// `FUN_00954CC0` (954C60 by actor): the display slot of `army` in the
/// territory list, or -1 when it is not there.
pub fn army_slot(state: &MgState, territory: usize, army: ArmyId) -> i32 {
    state.territories[territory]
        .armies
        .iter()
        .find(|r| r.army == army)
        .map_or(-1, |r| r.slot)
}

/// `FUN_009552A0` -> `FUN_00957DD0`: territory reference position. Normal
/// definitions: location + first actor slot (binary32 per component), or
/// the location without slots. ALIAS: the location with x replaced by
/// `bounds_max.x` when the partner's x is `<=` this x (`fcomp`, `test
/// ah,0x41`), else `bounds_min.x`. 9552A0 then takes z from the territory
/// visual object (`+0x20` handle -> `+0x28`) when the handle resolves:
/// `visual_z` (engine data).
pub fn territory_position(
    state: &MgState,
    data: &MetaGameData,
    territory: usize,
    visual_z: Option<u32>,
    precision: Precision,
) -> Result<[u32; 3]> {
    let map = map(state, data)?;
    let def = definition(state, map, territory)?;
    let mut pos = def.location;
    if def.kind == TerritoryKind::Alias {
        let partner = def
            .alias_partner
            .and_then(|p| map.territories.get(p))
            .ok_or_else(|| Error::Unsupported("957DD0: alias without partner (native null dereference)".into()))?;
        pos[0] = if f(partner.location[0])?.compare(&f(def.location[0])?) == Ordering::Greater {
            def.bounds_min[0]
        } else {
            def.bounds_max[0]
        };
    } else if let Some(slot) = def.actor_slots.first() {
        for i in 0..3 {
            pos[i] = f(slot[i])?.add(&f(def.location[i])?, precision)?.store_f32()?;
        }
    }
    if let Some(z) = visual_z {
        pos[2] = z;
    }
    Ok(pos)
}

/// `KKC_TerritoryPositionSolver::Solve` (`FUN_00959420`) for `territory` and
/// display `index` ([`FIRST_FREE`] = 954A10).
///
/// ALIAS territories: 9552A0 ([`territory_position`] with the visual z).
/// Otherwise q = index / slots, s = index % slots (`idiv`, slot count
/// `+0x60`); center = f32(slot[s] + location) per component; q > 0: the
/// layout of the definition spacing (`+0x4C`) with n = q - 1 and the spacing
/// distance (`+0x48`). A slot count of 0 (divide fault) or a negative slot
/// (read before the array) is unsupported.
pub fn solve(
    state: &MgState,
    data: &MetaGameData,
    territory: usize,
    index: i32,
    visual_z: &dyn Fn(usize) -> Option<u32>,
    precision: Precision,
) -> Result<[u32; 3]> {
    let map = map(state, data)?;
    let def = definition(state, map, territory)?;
    if def.kind == TerritoryKind::Alias {
        return territory_position(state, data, territory, visual_z(territory), precision);
    }
    let index = if index == FIRST_FREE { first_free_slot(state, territory) } else { index };
    let count = def.actor_slots.len() as i32;
    if count == 0 {
        return Err(Error::Unsupported("959420: no actor slots (native divide fault)".into()));
    }
    let (q, s) = (index / count, index % count);
    if s < 0 {
        return Err(Error::Unsupported(format!("959420: slot {s} (native reads before the slot array)")));
    }
    let slot = def.actor_slots[s as usize];
    let mut center = [0u32; 3];
    for i in 0..3 {
        center[i] = f(slot[i])?.add(&f(def.location[i])?, precision)?.store_f32()?;
    }
    if q > 0 {
        layout_position(def.actor_spacing, center, q - 1, def.actor_spacing_distance_bits, precision)
    } else {
        Ok(center)
    }
}

/// Army creation position of 955B70 (InitCreate 955D50, purchase 952A00,
/// event 95B220: index [`FIRST_FREE`], taken before the new army enters the
/// territory) and 955820 (respawn 955DA0/955DD0: the stored slot): the
/// solved position with z = [`PLACEMENT_Z_BITS`].
pub fn placement_position(
    state: &MgState,
    data: &MetaGameData,
    territory: usize,
    index: i32,
    visual_z: &dyn Fn(usize) -> Option<u32>,
    precision: Precision,
) -> Result<[u32; 3]> {
    let [x, y, _] = solve(state, data, territory, index, visual_z, precision)?;
    Ok([x, y, PLACEMENT_Z_BITS])
}

/// `FUN_0093EBF0`: distance from the army actor position (engine,
/// actor `+0x20`) to a definition location: dx stored binary32, dy and dz
/// unrounded; sqrt((dz*dz + dy*dy) + dx*dx).
fn actor_distance(actor: [u32; 3], location: [u32; 3], precision: Precision) -> Result<Binary> {
    let dx = f(f(actor[0])?.sub(&f(location[0])?, precision)?.store_f32()?)?;
    let dy = f(actor[1])?.sub(&f(location[1])?, precision)?;
    let dz = f(actor[2])?.sub(&f(location[2])?, precision)?;
    let sum = dz
        .mul(&dz, precision)?
        .add(&dy.mul(&dy, precision)?, precision)?
        .add(&dx.mul(&dx, precision)?, precision)?;
    Ok(sum.sqrt(precision)?)
}

/// `FUN_009626D0` (per waypoint step 940020, before leaving `territory`):
/// when the territory definition has an alias partner and the army actor
/// (engine position `actor_position`) is strictly nearer to the partner
/// (unrounded 93EBF0 against the binary32 stored one), the army is put back
/// on its display slot of `territory` (954CC0 index, -1 = first free when it
/// is not listed): `Some(position)` for 4EE5C0. Otherwise `None`.
pub fn realign_position(
    state: &MgState,
    data: &MetaGameData,
    army: ArmyId,
    territory: usize,
    actor_position: [u32; 3],
    visual_z: &dyn Fn(usize) -> Option<u32>,
    precision: Precision,
) -> Result<Option<[u32; 3]>> {
    let map = map(state, data)?;
    let def = definition(state, map, territory)?;
    let Some(partner) = def.alias_partner else {
        return Ok(None);
    };
    let partner = map
        .territories
        .get(partner)
        .ok_or_else(|| Error::Unsupported("alias partner out of range".into()))?;
    let own = actor_distance(actor_position, def.location, precision)?.store_f32()?;
    let other = actor_distance(actor_position, partner.location, precision)?;
    if other.compare(&f(own)?) != Ordering::Less {
        return Ok(None);
    }
    let index = army_slot(state, territory, army);
    Ok(Some(solve(state, data, territory, index, visual_z, precision)?))
}

/// Move-order waypoints of one step (`FUN_00962880`, called by 940020).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveWaypoints {
    /// MG territory whose first free slot was solved (`to`, or its alias
    /// partner when the partner is strictly nearer to `from`).
    pub territory: usize,
    /// Points handed to 9627D0 in order: the intermediate points of the
    /// first link of `from` to `to` (957910), then the solved position. The
    /// first one carries the caller's flag, the others 1; 9627D0 reads x
    /// and y.
    pub points: Vec<[u32; 3]>,
}

/// `FUN_00962880`: when the `to` definition has an alias partner, compare
/// the distances from the `from` location to the partner (dz stored
/// binary32, dx and dy unrounded) and to `to` (all unrounded), each
/// sqrt((dx*dx + dy*dy) + dz*dz); the partner MG territory (939B60) is used
/// when strictly nearer. Then solver index -1 on that territory, and the
/// waypoints of [`MoveWaypoints`].
pub fn move_waypoints(
    state: &MgState,
    data: &MetaGameData,
    from: usize,
    to: usize,
    visual_z: &dyn Fn(usize) -> Option<u32>,
    precision: Precision,
) -> Result<MoveWaypoints> {
    let map = map(state, data)?;
    let from_def = definition(state, map, from)?;
    let to_index = state.territories[to].definition;
    let to_def = definition(state, map, to)?;
    let mut chosen = to;
    if let Some(partner_index) = to_def.alias_partner {
        let partner = map
            .territories
            .get(partner_index)
            .ok_or_else(|| Error::Unsupported("alias partner out of range".into()))?;
        let a = from_def.location;
        let p = partner.location;
        let t = to_def.location;
        let dz = f(f(p[2])?.sub(&f(a[2])?, precision)?.store_f32()?)?;
        let dy = f(p[1])?.sub(&f(a[1])?, precision)?;
        let dx = f(p[0])?.sub(&f(a[0])?, precision)?;
        let to_partner = dx
            .mul(&dx, precision)?
            .add(&dy.mul(&dy, precision)?, precision)?
            .add(&dz.mul(&dz, precision)?, precision)?
            .sqrt(precision)?;
        let dz = f(t[2])?.sub(&f(a[2])?, precision)?;
        let dy = f(t[1])?.sub(&f(a[1])?, precision)?;
        let dx = f(t[0])?.sub(&f(a[0])?, precision)?;
        let to_target = dx
            .mul(&dx, precision)?
            .add(&dy.mul(&dy, precision)?, precision)?
            .add(&dz.mul(&dz, precision)?, precision)?
            .sqrt(precision)?;
        if to_partner.compare(&to_target) == Ordering::Less {
            chosen = PathGraph::new(state, data)?.territory_of_definition(partner_index)?;
        }
    }
    let last = solve(state, data, chosen, FIRST_FREE, visual_z, precision)?;
    let mut points = Vec::new();
    if let Some(link) = from_def.links.iter().find(|l| l.target == to_index) {
        points.extend_from_slice(&link.intermediate_points);
    }
    points.push(last);
    Ok(MoveWaypoints { territory: chosen, points })
}
