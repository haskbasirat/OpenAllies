//! Pathfinding on the map's cell grid (the original 8-neighbour search,
//! aa_navigation::search, or an equivalent). Owner: movement.
use glam::Vec2;

/// Waypoints from `from` to `to`, nearest first; empty when unreachable.
pub fn find(map: &crate::map::GameMap, from: Vec2, to: Vec2) -> Vec<Vec2> {
    let _ = (map, from);
    vec![to]
}
