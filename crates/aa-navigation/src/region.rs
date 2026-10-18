//! Native region-route prepass (0x53dc00), with explicit topology construction.
use crate::{OwnedPath, PathNode};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("region waypoint allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error("native region world boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub struct RegionRoute {
    pub status: i32,
    pub nodes: Vec<i32>,
}
/// Required authoritative region and collision services. The region graph is
/// rebuilt during native world loading; it must not be inferred from preview
/// meshes or connected by invented straight-line edges.
pub trait World {
    ///7aa3d0/7b9300, followed by the returned region's ID field.
    fn region_at(&mut self, point: [u32; 2]) -> Result<i32>;
    ///53f320 with cached-path flag0. Returns the native ordered region chain,
    /// including its start/end nodes. This graph solver remains a separate leaf.
    fn plan_regions(&mut self, start: i32, goal: i32) -> Result<RegionRoute>;
    /// Native region fields+54/+58. Snapshot positions remain stable for this call.
    fn region_center(&self, id: i32) -> Result<[u32; 2]>;
    ///5586d0 copy of the current collision descriptor.
    fn begin_probe(&mut self) -> Result<()>;
    ///552250 on the original descriptor, preserving any shape snapping.
    fn place_context(&mut self, point: [u32; 2]) -> Result<()>;
    ///4f3c40: zero means this placement passes. Nonzero reasons are not merged.
    fn blocked_here(&mut self) -> Result<i32>;
    ///522dd0 with the original start and candidate region center.
    fn line_clear(&mut self, start: [u32; 2], goal: [u32; 2]) -> Result<bool>;
    ///53d340 with start used for both original start and fallback. Implementations
    /// can call endpoint::adjust with the authoritative bounds/descriptor.
    fn adjust_endpoint(&mut self, start: [u32; 2], goal: &mut [u32; 2]) -> Result<bool>;
}
fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn direction(
    from: [u32; 2],
    to: [u32; 2],
    p: Precision,
    stored_reciprocal: bool,
) -> Result<[u32; 2]> {
    let mut dx = b(to[0])?.sub(&b(from[0])?, p)?;
    let dy = b(b(to[1])?.sub(&b(from[1])?, p)?.store_f32()?)?;
    if !stored_reciprocal {
        dx = b(dx.store_f32()?)?;
    }
    let length = dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?.sqrt(p)?;
    if length.compare(&b(0x38d1b717)?) != Ordering::Greater {
        return Ok([0, 0]);
    }
    let mut reciprocal = Binary::from_u32(1).div(&length, p)?;
    if stored_reciprocal {
        reciprocal = b(reciprocal.store_f32()?)?;
    }
    Ok([
        dx.mul(&reciprocal, p)?.store_f32()?,
        dy.mul(&reciprocal, p)?.store_f32()?,
    ])
}
fn prepend(path: &mut OwnedPath, point: [u32; 2], direction: [u32; 2]) -> Result<()> {
    path.nodes.try_reserve(1)?;
    path.nodes.insert(
        0,
        PathNode {
            words: [
                point[0],
                point[1],
                direction[0],
                direction[1],
                2,
                0,
                point[0],
                point[1],
            ],
        },
    );
    path.kind2_count = path.kind2_count.wrapping_add(1);
    path.dirty = true;
    Ok(())
}

/// Complete non-diagnostic53dc00 behavior after world services supply the region
/// graph. Earlier nodes/metadata remain on failure; no implicit path reset or
/// retry occurs here. The native caller resets the path before entering.
pub fn prepass<W: World>(
    path: &mut OwnedPath,
    start: [u32; 2],
    goal: [u32; 2],
    p: Precision,
    world: &mut W,
) -> Result<i32> {
    let start_region = world.region_at(start)?;
    let goal_region = world.region_at(goal)?;
    let mut route = world.plan_regions(start_region, goal_region)?;
    if route.status == 0 {
        return Ok(0);
    }
    // Original removes the last region and then the first before probing. Each
    // probe iteration removes the current first again, then tests its successor.
    route.nodes.pop();
    let mut first = usize::from(!route.nodes.is_empty());
    world.begin_probe()?;
    while first < route.nodes.len() {
        first += 1;
        if first == route.nodes.len() {
            break;
        }
        let center = world.region_center(route.nodes[first])?;
        world.place_context(center)?;
        if world.blocked_here()? != 0 {
            continue;
        }
        if !world.line_clear(start, center)? {
            break;
        }
    }
    let remaining = &route.nodes[first..];
    let id = remaining.first().copied().unwrap_or(-1);
    path.word18 = id as u32;
    let mut adjusted = goal;
    if id >= 0 {
        adjusted = world.region_center(id)?;
        let last = world.region_center(*remaining.last().expect("nonempty region chain"))?;
        prepend(path, goal, direction(last, goal, p, true)?)?;
        let mut previous = goal;
        for &id in remaining.iter().rev() {
            let point = world.region_center(id)?;
            prepend(path, point, direction(point, previous, p, true)?)?;
            previous = point;
        }
    }
    if !world.adjust_endpoint(start, &mut adjusted)? {
        return Ok(0);
    }
    if let Some(head) = path.nodes.first_mut()
        && (b(head.words[0])?.compare(&b(adjusted[0])?) != Ordering::Equal
            || b(head.words[1])?.compare(&b(adjusted[1])?) != Ordering::Equal)
    {
        head.words[0] = adjusted[0];
        head.words[1] = adjusted[1];
        let direction = direction(start, adjusted, p, false)?;
        head.words[2] = direction[0];
        head.words[3] = direction[1];
        // Original coordinate-copy payload words6/7 are not updated here.
        path.word18 = world.region_at(adjusted)? as u32;
    }
    Ok(1)
}
