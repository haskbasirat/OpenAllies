//! ZoneSolver (`.\Source\Sim\Solver\ZoneSolver.cpp`): tile coverage of a zone.
//!
//! Entry points: `0x540860` (add/move, `this` = GWorld+0x248 ZoneSolver,
//! vtable `0xA36954`), `0x540590` (remove, `this` = GWorld+0x244 flood solver,
//! vtable `0xA36048`), `0x5406D0` (reveal-mask change of a type-1 zone,
//! called by the detection component `0x56EFC0`).
//!
//! Fast circle builder (zones whose virtual `+8` is true, i.e. types 1, 2, 6,
//! when SVar `ZoneBuilderFastDetectionZones` is set): a stack object
//! `{moved +0, old centre +4/+8, new centre +0xC/+0x10, old r^2 +0x14,
//! new r^2 +0x18, map +0x1C, zone +0x20, kingdom +0x24, set +0x28}` filled by
//! `0x540020` / `0x5401A0` and a midpoint circle rasteriser `0x53F730` that
//! calls a span callback (`0x53FD30` add, `0x53FDE0` remove, `0x53FF00`
//! reveal/hide). Coverage is "tile centre within the (adjusted) radius":
//! `0x53F650` (old circle) / `0x53F6C0` (new circle).
//!
//! Native quirk kept: span callbacks clip the end column to `width - 1`
//! exclusively, so the last map column never receives a fast-built zone.
//!
//! The other zone types use the flood-fill solvers (`0x5402E0` with functor
//! `0xA36910`, `0x4F4050` with `0xA368C0`); see
//! [`crate::flood`].
use crate::fpu;
use crate::query::{KINGDOM_BITS, tile_has_zone};
use crate::tile::{ZoneMap, ZoneWorld, hide, reveal};
use crate::flood;
use crate::zone::{Zone, ZoneObjectWorld, on_tile_added, on_tile_removed};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};

/// Span callback selected by the builder (`0x53FF90`, `0x53FFC0`, `0x53FFF0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Span {
    Add,
    Remove,
    Fog,
}

/// The fast builder's stack object.
struct Builder {
    moved: bool,
    old_centre: [Binary; 2],
    new_centre: [Binary; 2],
    old_r2: Binary,
    new_r2: Binary,
    kingdom: u32,
    set: bool,
}

/// `0x53FE90(r)`: unless `r == -1`, raise the radius so that its remainder
/// modulo the integer tile size `q` is at least `q / 4`:
/// `c = fmod(r, q); c < q/4 ? (q/4 - c) + r : r` (register result at PC).
pub fn adjust_radius(map: &ZoneMap, r_bits: u32, p: Precision) -> Result<Binary> {
    let r = fpu::load(r_bits)?;
    if r_bits == fpu::MINUS_ONE_BITS || fpu::equal(&r, &fpu::load(fpu::MINUS_ONE_BITS)?) {
        return Ok(r);
    }
    let t = fpu::ftol(&fpu::load(map.geometry.tile_size_bits)?)?;
    let q32 = fpu::f32_store(&Binary::from_i32(t))?;
    let q = fpu::load(q32)?;
    let a32 = fpu::f32_store(&fpu::mul(&Binary::from_i32(t), &fpu::load(fpu::QUARTER_BITS)?, p)?)?;
    let a = fpu::load(a32)?;
    let c = fpu::fmod(&r, &q)?;
    if fpu::less(&c, &a) {
        fpu::add(&fpu::sub(&a, &c, p)?, &r, p)
    } else {
        Ok(r)
    }
}

/// `0x53F650` (old circle, `moved` builders) / `0x53F6C0` (new circle):
/// squared distance from the tile centre, x term in registers, y centre
/// stored as binary32, compared with the stored binary32 r^2 (`<=`).
fn in_circle(map: &ZoneMap, centre: &[Binary; 2], r2: &Binary, x: i32, y: i32, p: Precision) -> Result<bool> {
    let g = &map.geometry;
    let half = fpu::half_tile(g.tile_size_bits, p)?;
    let tx = fpu::cell_center(x, g.shift, &half, p)?;
    let ty32 = fpu::f32_store(&fpu::cell_center(y, g.shift, &half, p)?)?;
    let dx = fpu::sub(&centre[0], &tx, p)?;
    let dy = fpu::sub(&centre[1], &fpu::load(ty32)?, p)?;
    let d2 = fpu::add(&fpu::mul(&dy, &dy, p)?, &fpu::mul(&dx, &dx, p)?, p)?;
    Ok(!fpu::less(r2, &d2))
}

fn centre(map: &ZoneMap, x: i32, y: i32, p: Precision) -> Result<[Binary; 2]> {
    let g = &map.geometry;
    let half32 = fpu::f32_store(&fpu::half_tile(g.tile_size_bits, p)?)?;
    let half = fpu::load(half32)?;
    let cx = fpu::f32_store(&fpu::cell_center(x, g.shift, &half, p)?)?;
    let cy = fpu::f32_store(&fpu::cell_center(y, g.shift, &half, p)?)?;
    Ok([fpu::load(cx)?, fpu::load(cy)?])
}

/// `0x53F730(cx, cy, r_tiles, callback)`: midpoint circle spans
/// `(x, y, width)` in native order (pairs of rows `cy + dy`, `cy - dy` from
/// the outermost row inwards, then the centre row).
pub fn circle_spans(cx: i32, cy: i32, r_tiles_bits: u32, p: Precision) -> Result<Vec<(i32, i32, i32)>> {
    let r = fpu::load(r_tiles_bits)?;
    let rad = fpu::ftol(&r)?;
    let rr = fpu::ftol(&fpu::mul(&r, &r, p)?)?;
    let mut out = Vec::new();
    let neg = rad.wrapping_neg();
    let mut err = neg.wrapping_mul(neg).wrapping_sub(1);
    let mut dx: i32 = 0;
    let mut dy = neg;
    if neg < 0 {
        loop {
            loop {
                let next = err.wrapping_sub(dx.wrapping_mul(2)).wrapping_add(1);
                if next > rr {
                    break;
                }
                err = next;
                dx = dx.wrapping_sub(1);
            }
            let w = 1i32.wrapping_sub(dx.wrapping_mul(2));
            out.push((cx.wrapping_add(dx), cy.wrapping_add(dy), w));
            out.push((cx.wrapping_add(dx), cy.wrapping_sub(dy), w));
            err = err.wrapping_add(dy.wrapping_mul(2)).wrapping_add(1);
            dy = dy.wrapping_add(1);
            if dy >= 0 {
                break;
            }
        }
        out.push((cx.wrapping_add(neg), cy.wrapping_add(dy), 1i32.wrapping_sub(neg.wrapping_mul(2))));
    } else {
        out.push((cx.wrapping_add(neg), cy.wrapping_add(dy), 1i32.wrapping_sub(neg.wrapping_mul(2))));
    }
    Ok(out)
}

fn span<W: ZoneWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    b: &Builder,
    zone: &mut Zone,
    kind: Span,
    (x0, y, width): (i32, i32, i32),
    p: Precision,
) -> Result<()> {
    if y < 0 || y >= map.geometry.height {
        return Ok(());
    }
    let mut end = x0.wrapping_add(width);
    let mut x = x0.max(0);
    let last = map.geometry.width.wrapping_sub(1);
    if last < end {
        end = last;
    }
    while x < end {
        let t = map.index(x, y).ok_or(Error::TileOutside(i64::from(x)))?;
        match kind {
            Span::Add => {
                let skip = b.moved && in_circle(map, &b.old_centre, &b.old_r2, x, y, p)?;
                if !skip && in_circle(map, &b.new_centre, &b.new_r2, x, y, p)? {
                    tile_add(map, w, zone, t)?;
                }
            }
            Span::Remove => {
                if b.moved && in_circle(map, &b.old_centre, &b.old_r2, x, y, p)? {
                    let gone = zone.radius_bits == fpu::MINUS_ONE_BITS
                        || !in_circle(map, &b.new_centre, &b.new_r2, x, y, p)?;
                    if gone {
                        tile_remove(map, w, zone, t)?;
                    }
                }
            }
            Span::Fog => {
                if in_circle(map, &b.new_centre, &b.new_r2, x, y, p)? {
                    if b.set {
                        reveal(map, w, t, b.kingdom, false, true)?;
                    } else {
                        hide(map, w, t, b.kingdom)?;
                    }
                }
            }
        }
        x += 1;
    }
    Ok(())
}

/// `0x54FA10(zone, zone.flags)` on tile `t`, then zone virtual `+0x14`.
pub fn tile_add<W: ZoneWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, zone: &Zone, t: usize) -> Result<()> {
    let mut slot = map.tile(t)?.set;
    map.pool.tile_add(&mut slot, zone.actor, zone.flags)?;
    map.tiles[t].set = slot;
    on_tile_added(map, w, zone, t, zone.flags)
}

/// `0x54FA80(zone)` on tile `t` (XOR of the zone's flags), then zone virtual
/// `+0x10`.
pub fn tile_remove<W: ZoneWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, zone: &Zone, t: usize) -> Result<()> {
    let mut slot = map.tile(t)?.set;
    map.pool.tile_remove(&mut slot, zone.actor, zone.flags)?;
    map.tiles[t].set = slot;
    on_tile_removed(map, w, zone, t)
}

/// `0x540020(zone, x, y, radius, moved)`: store the new placement in the zone,
/// build both circles and run the add pass (new radius > -1) and, for an
/// incremental move, the remove pass over the old circle.
#[allow(clippy::too_many_arguments)]
fn fast_move<W: ZoneWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    zone: &mut Zone,
    x: i32,
    y: i32,
    radius_bits: u32,
    moved: bool,
    p: Precision,
) -> Result<()> {
    let (old_x, old_y, old_r) = (zone.x, zone.y, zone.radius_bits);
    zone.x = x;
    zone.y = y;
    zone.radius_bits = radius_bits;
    let rn32 = fpu::f32_store(&adjust_radius(map, radius_bits, p)?)?;
    let ro = adjust_radius(map, old_r, p)?;
    let old_r2 = fpu::load(fpu::f32_store(&fpu::mul(&ro, &ro, p)?)?)?;
    let rn = fpu::load(rn32)?;
    let new_r2 = fpu::load(fpu::f32_store(&fpu::mul(&rn, &rn, p)?)?)?;
    let b = Builder {
        moved,
        old_centre: centre(map, old_x, old_y, p)?,
        new_centre: centre(map, x, y, p)?,
        old_r2,
        new_r2,
        kingdom: 0,
        set: false,
    };
    let t = fpu::ftol(&fpu::load(map.geometry.tile_size_bits)?)?;
    let inv = fpu::div(&fpu::load(fpu::ONE_BITS)?, &Binary::from_i32(t), p)?;
    let rn_tiles = fpu::f32_store(&fpu::mul(&rn, &inv, p)?)?;
    let ro_tiles = fpu::f32_store(&fpu::mul(&inv, &ro, p)?)?;
    if fpu::less(&fpu::load(fpu::MINUS_ONE_BITS)?, &rn) {
        for s in circle_spans(x, y, rn_tiles, p)? {
            span(map, w, &b, zone, Span::Add, s, p)?;
        }
    }
    if moved {
        for s in circle_spans(old_x, old_y, ro_tiles, p)? {
            span(map, w, &b, zone, Span::Remove, s, p)?;
        }
    }
    Ok(())
}

/// `0x540860(zone, x, y, radius, moved)`: fast builder for eligible zones when
/// the SVar is set; otherwise the flood-fill path (service).
#[allow(clippy::too_many_arguments)]
pub fn add_zone<W: ZoneObjectWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    zone: &mut Zone,
    x: i32,
    y: i32,
    radius_bits: u32,
    moved: bool,
    p: Precision,
) -> Result<()> {
    if zone.fast_eligible() && w.fast_detection_zones()? {
        return fast_move(map, w, zone, x, y, radius_bits, moved, p);
    }
    with_state(map, |map, st| flood::add(map, st, w, zone, x, y, radius_bits, moved, p)).map(|_| ())
}

/// Run a flood path with the map's solver state taken out for the call. The
/// placeholder left in the map has the busy byte set, so a re-entrant solve
/// from a zone callback returns 0 and clears it as the original does
/// (`0xB3D43D`); a second re-entry would run on the outer solve's open list
/// in the original and is rejected here ([`flood`] checks the scratch size).
fn with_state<R>(
    map: &mut ZoneMap,
    f: impl FnOnce(&mut ZoneMap, &mut flood::SolverState) -> Result<R>,
) -> Result<R> {
    let mut placeholder = flood::SolverState::new(0, None);
    placeholder.busy = true;
    let mut st = std::mem::replace(&mut map.solver, placeholder);
    let r = f(map, &mut st);
    map.solver = st;
    r
}

/// `0x540590(zone)`: fast builder with radius -1 at the current tile
/// (`moved` = true removes every tile of the old circle); otherwise the
/// flood-fill removal (service).
pub fn remove_zone<W: ZoneObjectWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, zone: &mut Zone, p: Precision) -> Result<()> {
    if zone.fast_eligible() && w.fast_detection_zones()? {
        let (x, y) = (zone.x, zone.y);
        return fast_move(map, w, zone, x, y, fpu::MINUS_ONE_BITS, true, p);
    }
    with_state(map, |map, st| flood::remove(map, st, w, zone)).map(|_| ())
}

/// `0x5401A0(zone, kingdom, set)`: reveal (`set`, `0x54F670` = `0x54F4F0(k, 0,
/// 1)`) or hide (`0x54F690` = `0x54F5E0`) every tile of the zone's circle for
/// one kingdom. The radius in tiles is `adjust(r) / ftol(tile)` (a division
/// here, unlike `0x540020`).
pub fn fast_kingdom_pass<W: ZoneWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    zone: &mut Zone,
    kingdom: u32,
    set: bool,
    p: Precision,
) -> Result<()> {
    let r = adjust_radius(map, zone.radius_bits, p)?;
    let new_r2 = fpu::load(fpu::f32_store(&fpu::mul(&r, &r, p)?)?)?;
    let c = centre(map, zone.x, zone.y, p)?;
    let b = Builder {
        moved: false,
        old_centre: c.clone(),
        new_centre: c,
        old_r2: fpu::load(fpu::MINUS_ONE_BITS)?,
        new_r2,
        kingdom,
        set,
    };
    let t = fpu::ftol(&fpu::load(map.geometry.tile_size_bits)?)?;
    let tiles = fpu::f32_store(&fpu::div(&r, &Binary::from_i32(t), p)?)?;
    for s in circle_spans(zone.x, zone.y, tiles, p)? {
        span(map, w, &b, zone, Span::Fog, s, p)?;
    }
    Ok(())
}

/// `0x5406D0(zone, set_mask, changed_mask)`: for an active zone, every
/// kingdom bit `i < 16` of `changed` (lowest first): kingdom `GWorld+0x154[i]`
/// (0 past the count) revealed when bit `i` of `set_mask` is set, else hidden.
pub fn kingdom_mask_changed<W: ZoneObjectWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    zone: &mut Zone,
    set_mask: u32,
    changed: u32,
    p: Precision,
) -> Result<()> {
    if changed == 0 || !zone.active() {
        return Ok(());
    }
    let mut rest = changed;
    let mut i = 0;
    while i < KINGDOM_BITS {
        if rest == 0 {
            return Ok(());
        }
        let b = 1u32 << i;
        if rest & b != 0 {
            rest &= !b;
            let k = w.kingdom_slot(i)?;
            let on = set_mask & b != 0;
            if zone.fast_eligible() && w.fast_detection_zones()? {
                fast_kingdom_pass(map, w, zone, k, on, p)?;
            } else {
                let z = *zone;
                with_state(map, |map, st| flood::kingdom_pass(map, st, w, &z, k, on))?;
            }
        }
        i += 1;
    }
    Ok(())
}

/// `0x54EF40(zone)` used by the slow add path on the start tile.
pub fn tile_has_zone_entry(map: &ZoneMap, t: usize, zone: &Zone) -> Result<bool> {
    tile_has_zone(&map.pool, map.tile(t)?.set, Some((zone.actor, zone.flags)))
}

/// `0x5147C0`: for an active zone, the effects walk `0x540690` (flood solver,
/// functor `0xA36820`: `0x54F7F0` on every tile holding the zone).
pub fn effects_pass<W: ZoneObjectWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, zone: &Zone) -> Result<()> {
    if !zone.active() {
        return Ok(());
    }
    with_state(map, |map, st| flood::effects_pass(map, st, w, zone)).map(|_| ())
}
