//! `KKC_CAIStateExplore` (regiment Mind +220, vtable 0xA32BEC): explore around
//! a waypoint. A rendition of the Think 0x525D70 (exact port:
//! `aa_mind::cai_thinks::explore`) over aa-play's orders.
//!
//! Activation 0x524100: a coordinate below 1.0 becomes 1.0, then state +10/+14 =
//! +18/+1C = the point, the target's kind +28 = 1 and it is scored (0x5213E0).
//!
//! The Think: the move keeps running until the target is outside the map or
//! within `ExploreInertiaCloseEnough` of the organization. Then
//! `ExploreNumActorLocations` candidates around the actor (0x524170) and
//! `ExploreNumWaypointLocations` candidates around the waypoint (0x5242C0) are
//! drawn; the candidate of the highest fog count (2 unexplored, 1 grey, 0 visible,
//! 0x54F3E0 / 0x54ED10), then the highest score, becomes the target. None inside
//! the map: the sleep state.
//!
//! The draws are native: the direction (0x4F02B0 then `% table count`) and radius
//! of 0x524170, the direction of 0x5242C0, and the fuzz of each scored candidate
//! (`ExploreFuzzyFactor`, a candidate of too low a count takes none).
//!
//! Compare against the original: the walk of 0x5242C0 into the fog is reduced
//! to its bisection along the ray (0x54ECF0's region and tile +20 bit 0 read the
//! terrain flags here, and the `ExploreWaypointTilesIntoFog` steps are one point);
//! `ExploreFinishAreaBonus` (the regions of 0x7AA3D0) is not added; 0x4F46E0's
//! ring search is the open-tile test; group following (Organization +164) has no
//! host.
use super::MindState;
use crate::combat::CombatState;
use crate::fog::TileView;
use crate::map::{BLOCKED, NOT_LAND_PASSABLE};
use crate::world::{Order, OrgId, World};
use glam::Vec2;

/// A32690: the far end of 0x5242C0's ray, in map widths.
const RAY_WIDTHS: f32 = 3.0;

/// The Explore state.
#[derive(Clone, Debug)]
pub struct Explore {
    /// State +10 / +14: the waypoint.
    waypoint: Vec2,
    /// State +18 / +1C: the target.
    target: Vec2,
    moving: bool,
    next_think: f32,
}

/// 0x5249A0 and 0x524100.
pub(super) fn begin(world: &mut World, o: OrgId, at: Vec2, scalar: Option<f32>) {
    let at = Vec2::new(if at.x < 1.0 { 1.0 } else { at.x }, if at.y < 1.0 { 1.0 } else { at.y });
    if scalar == Some(1.0)
        && let Some(MindState::Explore(e)) = world.orders.orgs.get_mut(&o)
    {
        e.waypoint = at;
        return;
    }
    let now = world.time;
    if let Some(c) = world.combat.orgs.get_mut(o as usize) {
        c.task = None;
        c.task_order = None;
    }
    world.orders.orgs.insert(o, MindState::Explore(Explore { waypoint: at, target: at, moving: false, next_think: now }));
}

fn in_map(world: &World, p: Vec2) -> bool {
    let (w, h) = (world.map.width as f32 * world.map.cell_size, world.map.height as f32 * world.map.cell_size);
    p.x >= 0.0 && p.x < w && p.y >= 0.0 && p.y < h
}
fn open(world: &World, p: Vec2) -> bool {
    world.map.cell_flags(p) & (BLOCKED | NOT_LAND_PASSABLE) == 0
}

/// The fog count of a point for `kingdom`.
fn fog_count(world: &World, kingdom: u32, p: Vec2) -> i32 {
    let cell = world.map.cell_size;
    match crate::fog::tile_view(world, Some(kingdom), (p.x / cell) as i32, (p.y / cell) as i32) {
        TileView::Visible => 0,
        TileView::Explored => 1,
        TileView::Unexplored => 2,
    }
}

/// 0x5213E0: Some((count, score)); None when refused (a closed tile, or a lower
/// fog count than the best). The last draw is the fuzz, as the original.
fn score(world: &mut World, o: OrgId, state: &Explore, p: Vec2, kind_base: f32, best_count: i32) -> Option<(i32, f32)> {
    let kingdom = world.orgs[o as usize].kingdom;
    if !open(world, p) {
        return None;
    }
    let count = fog_count(world, kingdom, p);
    if count < best_count {
        return None;
    }
    let rule = |world: &World, name: &str, v: f32| world.data.rule_ci(name).unwrap_or(v);
    let own = world.combat.org(o).map_or(world.orgs[o as usize].anchor, |c| c.position);
    let mut value = kind_base;
    value += (p - state.waypoint).length() * rule(world, "ExploreDistanceMultiplier", -1.0);
    if in_map(world, state.target) {
        let (a, b) = ((state.target - own).normalize_or_zero(), (p - own).normalize_or_zero());
        value += a.dot(b) * rule(world, "ExploreDotProductMultiplier", 20.0);
    }
    let fuzz = rule(world, "ExploreFuzzyFactor", 20.0);
    let u = world.random_at(crate::trace::sites::EXPLORE);
    Some((count, value + u * fuzz))
}

/// 0x524170 PickRandomLocationFromActor: the direction table draw and the radius
/// draw, then the score.
fn actor_candidate(world: &mut World, o: OrgId, state: &Explore, best_count: i32) -> Option<(Vec2, i32, f32)> {
    let min = world.data.rule_ci("ExploreActorRadiusMin").unwrap_or(20.0);
    let max = world.data.rule_ci("ExploreActorRadiusMax").unwrap_or(30.0);
    let direction = direction_draw(world);
    let u = world.random_at(crate::trace::sites::EXPLORE);
    let radius = u * (max - min) + min;
    let own = world.combat.org(o).map_or(world.orgs[o as usize].anchor, |c| c.position);
    let p = own + direction * radius;
    if !in_map(world, p) {
        return None;
    }
    let base = world.data.rule_ci("ExploreActorBaseValue").unwrap_or(40.0);
    score(world, o, state, p, base, best_count).map(|(c, s)| (p, c, s))
}

/// 0x5242C0: the direction draw, then the bisection along the ray from the
/// waypoint for the first tile that is not explored.
fn waypoint_candidate(world: &mut World, o: OrgId, state: &Explore, best_count: i32) -> Option<(Vec2, i32, f32)> {
    let direction = direction_draw(world);
    let kingdom = world.orgs[o as usize].kingdom;
    let width = world.map.width as f32 * world.map.cell_size;
    let (mut near, mut far) = (state.waypoint, state.waypoint + direction * width * RAY_WIDTHS);
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
    if !in_map(world, mid) || !open(world, mid) {
        return None;
    }
    let base = world.data.rule_ci("ExploreWaypointBaseValue").unwrap_or(0.0);
    score(world, o, state, mid, base, best_count).map(|(c, s)| (mid, c, s))
}

/// 0x4F02B0 then 0x444A90: one draw `%` the `[B39AF8]` direction table.
fn direction_draw(world: &mut World) -> Vec2 {
    let draw = world.random_u32_at(crate::trace::sites::EXPLORE);
    Vec2::from(aa_movement::formation::random_direction(draw))
}

/// 0x525D70.
pub(super) fn think(world: &mut World, st: &mut CombatState, o: OrgId) {
    let now = world.time;
    let i = o as usize;
    let Some(MindState::Explore(e)) = world.orders.orgs.get(&o) else { return };
    if now < e.next_think {
        return;
    }
    let mut e = e.clone();
    e.next_think = now + crate::combat::ENGAGE_THINK_SECONDS;
    let own = st.orgs[i].position;
    let close = world.data.rule_ci("ExploreInertiaCloseEnough").unwrap_or(3.0);
    // Step 2 and 3: an active move keeps going until it arrives.
    let moving = matches!(world.orgs[i].order, Order::Move { .. });
    if moving && in_map(world, e.target) && own.distance(e.target) > close {
        world.orders.orgs.insert(o, MindState::Explore(e));
        return;
    }
    // Step 4.
    let count_a = world.data.rule_ci("ExploreNumActorLocations").unwrap_or(20.0) as i32;
    let count_b = world.data.rule_ci("ExploreNumWaypointLocations").unwrap_or(20.0) as i32;
    let mut best: Option<(Vec2, i32, f32)> = None;
    let better = |c: &(Vec2, i32, f32), best: &Option<(Vec2, i32, f32)>| best.is_none_or(|b| if c.1 == b.1 { c.2 > b.2 } else { c.1 > b.1 });
    for _ in 0..count_a {
        let floor = best.map_or(0, |b| b.1);
        if let Some(c) = actor_candidate(world, o, &e, floor)
            && better(&c, &best)
        {
            best = Some(c);
        }
    }
    for _ in 0..count_b {
        let floor = best.map_or(0, |b| b.1);
        if let Some(c) = waypoint_candidate(world, o, &e, floor)
            && better(&c, &best)
        {
            best = Some(c);
        }
    }
    // Step 5.
    match best {
        Some((p, _, _)) => {
            e.target = p;
            e.moving = true;
            world.orgs[i].order = Order::Move { to: p, attack_move: false };
        }
        None => {
            e.moving = false;
            if moving {
                world.orgs[i].order = Order::Idle;
            }
        }
    }
    world.orders.orgs.insert(o, MindState::Explore(e));
}
