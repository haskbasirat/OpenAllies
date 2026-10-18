//! Original all-overlaps query `0x555620` (shape tile iterator `0x554dd0` /
//! `0x5541f0`) and the actor overlapping-thing list refresh `0x4d67f0`.
//!
//! Units keep a list (actor +0x110/+0x114) of things they currently overlap;
//! the UAI collision result (`0x536f40`) ignores collisions with those things,
//! which lets overlapping units separate instead of blocking each other.
use crate::{
    Error, Result,
    collision,
    geometry::{Placement, TileRect, tile_rectangle},
    index::{ActorKey, SpatialIndex},
    index_world::CollisionView,
    query::{PreparedQuery, QueryGeometry, QueryPolicy, clear, mark},
};
use aa_numeric::Binary;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

fn world_error(e: collision::Error) -> Error {
    Error::World(e.to_string())
}

/// One actor+0x110 node: the overlapped thing's reference (actor +0x14) and
/// the binary32 expiry time compared against GWorld+0xE0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlapEntry {
    pub reference: u32,
    pub expiry_bits: u32,
}

/// `0x554dd0` start cell: the query position's cell when `0 <= x < extent`
/// on both axes (ordered), otherwise the tile rectangle minimum.
fn start_cell(geometry: &QueryGeometry, index: &SpatialIndex, placement: &Placement, rect: TileRect) -> Result<[i32; 2]> {
    let zero = Binary::from_i32(0);
    let shift = index.grid().coordinate_shift & 31;
    let mut cell = [0; 2];
    for (axis, slot) in cell.iter_mut().enumerate() {
        let value = Binary::from_f32_bits(placement.position[axis])?;
        if value.compare(&zero) == Ordering::Less
            || value.compare(&Binary::from_f32_bits(geometry.extent_bits[axis])?) != Ordering::Less
        {
            return Ok(rect.min);
        }
        *slot = value.truncate_i32()? >> shift;
    }
    Ok(cell)
}

/// `0x5541f0`: from the current cell, stop at the start cell (accepted without
/// a shape test) or the first cell whose map cell placement (+0x30 square,
/// facing (0,-1)) overlaps the query placement; Y-major, X-minor order.
fn advance(
    index: &SpatialIndex,
    geometry: &QueryGeometry,
    placement: &Placement,
    rect: TileRect,
    start: [i32; 2],
    cell: &mut [i32; 2],
) -> Result<()> {
    while cell[1] <= rect.max[1] {
        if *cell == start {
            return Ok(());
        }
        let tile = geometry.tile_placement(index, *cell)?;
        if crate::shape_overlap::overlaps(placement, &tile, index.precision())? {
            return Ok(());
        }
        cell[0] += 1;
        if cell[0] > rect.max[0] {
            cell[1] += 1;
            cell[0] = rect.min[0];
        }
    }
    Ok(())
}

/// Complete `0x555620`: references (actor +0x14) of every actor whose
/// placement overlaps the query, in discovery order. When the query has a
/// source actor, that actor must have flag bit10 or nothing is collected.
/// `flag10_only` keeps only actors with flag bit10. The descriptor is the
/// caller's own (exclusion callbacks may update it). Temporary intrusive marks
/// are cleared on every exit; the next tile-list link is read after each actor
/// is processed, as natively.
pub fn all_overlaps<P: QueryPolicy>(
    index: &mut SpatialIndex,
    geometry: &QueryGeometry,
    query: &mut PreparedQuery,
    flag10_only: bool,
    policy: &mut P,
) -> Result<Vec<u32>> {
    let mut head = None;
    let result = collect(index, geometry, query, flag10_only, policy, &mut head);
    let cleanup = clear(index, &mut head);
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(Error::World(format!("{error}; overlap cleanup failed: {cleanup}"))),
    }
}

fn collect<P: QueryPolicy>(
    index: &mut SpatialIndex,
    geometry: &QueryGeometry,
    query: &mut PreparedQuery,
    flag10_only: bool,
    policy: &mut P,
    head: &mut Option<ActorKey>,
) -> Result<Vec<u32>> {
    let mut found = Vec::new();
    if let Some(source) = query.descriptor.source_actor
        && index.member(source)?.actor.flags & (1 << 10) == 0
    {
        return Ok(found);
    }
    let p = index.precision();
    let rect = tile_rectangle(index.grid(), &query.placement, p)?;
    let start = start_cell(geometry, index, &query.placement, rect)?;
    let mut cell = rect.min;
    advance(index, geometry, &query.placement, rect, start, &mut cell)?;
    while cell[1] <= rect.max[1] {
        let mut cursor = index.tile(cell)?.first_node;
        while let Some(node) = cursor {
            let actor = index.node(node)?.actor;
            if !mark(index, head, actor)?
                && (!flag10_only || index.member(actor)?.actor.flags & (1 << 10) != 0)
                && !collision::excluded(actor, &mut query.descriptor, &mut CollisionView { index, policy })
                    .map_err(world_error)?
            {
                let placement = index.member(actor)?.placement(true);
                if crate::shape_overlap::overlaps(&query.placement, &placement, p)? {
                    found.push(index.member(actor)?.reference);
                }
            }
            cursor = index.node(node)?.next;
        }
        cell[0] += 1;
        if cell[0] > rect.max[0] {
            cell[1] += 1;
            cell[0] = rect.min[0];
        }
        advance(index, geometry, &query.placement, rect, start, &mut cell)?;
    }
    Ok(found)
}

/// Complete `0x4d67f0`. Actor flags are read once at entry: bit29 set or a
/// false `0x4d10a0` (`eligible`) or bit10 clear returns without change.
/// Otherwise entries whose expiry is not ordered after the world time are
/// removed in place, then the ordinary non-strict actor query (`0x554350`)
/// collects overlapping bit10 actors and each is prepended with expiry +0.0.
pub fn refresh_overlaps<P: QueryPolicy>(
    index: &mut SpatialIndex,
    geometry: &QueryGeometry,
    actor: ActorKey,
    entries: &mut Vec<OverlapEntry>,
    world_time_bits: u32,
    eligible: &mut dyn FnMut(&mut SpatialIndex, &mut P) -> Result<bool>,
    policy: &mut P,
) -> Result<bool> {
    let flags = index.member(actor)?.actor.flags;
    if flags & (1 << 29) != 0 || !eligible(index, policy)? || flags & (1 << 10) == 0 {
        return Ok(false);
    }
    let now = Binary::from_f32_bits(world_time_bits)?;
    let mut kept = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        if now.compare(&Binary::from_f32_bits(entry.expiry_bits)?) == Ordering::Less {
            kept.push(*entry);
        }
    }
    *entries = kept;
    let mut query = crate::actor_context::prepare(index, actor, false, policy)?;
    let found = all_overlaps(index, geometry, &mut query, true, policy)?;
    for reference in found {
        entries.insert(0, OverlapEntry { reference, expiry_bits: 0 });
    }
    Ok(true)
}

#[cfg(test)]
#[path = "overlaps_tests.rs"]
mod tests;
