//! What the Explore state (regiment Mind +220, vtable 0xA32BEC, hosted on
//! `aa_mind::cai_thinks::explore` in `crate::movement`) reads here.
//!
//! Compare against the original: the walk of 0x5242C0 into the fog is reduced to its
//! bisection along the ray (0x54ECF0's region and tile +20 bit 0 read the terrain flags
//! here, and the `ExploreWaypointTilesIntoFog` steps are one point).
use crate::fog::TileView;
use crate::map::{BLOCKED, NOT_LAND_PASSABLE};
use crate::world::{OrgId, World};
use glam::Vec2;

/// A32690: the far end of 0x5242C0's ray, in map widths.
const RAY_WIDTHS: f32 = 3.0;

/// 0x5249A0 and 0x524100.
pub(super) fn begin(world: &mut World, o: OrgId, at: Vec2, scalar: Option<f32>) {
    crate::movement::explore_command(world, o, at, scalar);
}

fn in_map(world: &World, p: Vec2) -> bool {
    let (w, h) = (world.map.width as f32 * world.map.cell_size, world.map.height as f32 * world.map.cell_size);
    p.x >= 0.0 && p.x < w && p.y >= 0.0 && p.y < h
}
fn open(world: &World, p: Vec2) -> bool {
    world.map.cell_flags(p) & (BLOCKED | NOT_LAND_PASSABLE) == 0
}

/// The fog count of a point for `kingdom`.
pub(crate) fn fog_count(world: &World, kingdom: u32, p: Vec2) -> i32 {
    let cell = world.map.cell_size;
    match crate::fog::tile_view(world, Some(kingdom), (p.x / cell) as i32, (p.y / cell) as i32) {
        TileView::Visible => 0,
        TileView::Explored => 1,
        TileView::Unexplored => 2,
    }
}

/// 0x5242C0 after its direction draw: the bisection along the ray from the waypoint for
/// the first tile that is not explored.
pub(crate) fn ray_into_fog(world: &World, o: OrgId, waypoint: Vec2, direction: Vec2) -> Option<Vec2> {
    let kingdom = world.orgs[o as usize].kingdom;
    let width = world.map.width as f32 * world.map.cell_size;
    let (mut near, mut far) = (waypoint, waypoint + direction * width * RAY_WIDTHS);
    let mut mid = near;
    for _ in 0..24 {
        mid = (far + near) * 0.5;
        if !in_map(world, mid) || !open(world, mid) {
            far = mid;
            continue;
        }
        if fog_count(world, kingdom, mid) == 2 {
            break;
        }
        near = mid;
    }
    (in_map(world, mid) && open(world, mid)).then_some(mid)
}
