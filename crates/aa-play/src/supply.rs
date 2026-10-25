//! Supply zones of HQs, depots and cities: healing and member
//! replacement for attached regiments inside a zone. Owner: core.
//!
//! Behaviour-faithful port of the original supply (aa-economy `supply.rs`):
//! - SupplyComponent Tick 0x58AE10: a relay (division HQ, depot) searches
//!   the supply zones of its kingdom it stands in for a supplier that reaches
//!   a primary (non-relay) source 0x58ADB0; a relay without one is out of
//!   supply, drains its stockpile one second per second and keeps providing
//!   supply while it lasts (0x4D0C90); a connected relay recharges at
//!   SupplyStockpileRechargeRate up to stockpile_max; the zone (centre,
//!   supply_range) is active while 0x58ACE0 provides_supply holds (owned,
//!   not packing/unpacking, relay stock, radius >= 0);
//! - actor in supply 0x4D0A80 and state 19 update 0x4D7CE0: suppliers that
//!   are primary sources always, members of a supply_used organization with
//!   their organization, an organization only while attached to a living HQ
//!   (unless in_supply_detached) and standing in a zone of its kingdom,
//!   everything else when its tile is in a zone of its kingdom;
//! - regeneration 0x57B5A0 behind the 0x579110 gate (supply_used, in supply,
//!   not engaged): one slot per tick, a damaged primary first, then the first
//!   damaged member, then an absent primary, then the first empty slot with a
//!   Body; members heal resupply_rate * max_health * (1 - shortage) * dt, an
//!   empty slot builds up the same amount until it reaches the slot
//!   definition's health and the member is created (0x57AD30).
//!
//! (fog owner) The zones themselves run on aa-zones ([`crate::zones`]): each
//! provider's zone type 0 is placed, moved and released by the native rules
//! (`0x58AE10` tail, `0x4D6350` on its tile changes) and flooded by the
//! exact ZoneSolver port; `books.zone_tiles` lists the tiles whose zone set
//! holds it. [`flood_tiles`] (the earlier f32 flood) stays for maps aa-zones
//! cannot describe and for the comparison tests.
use crate::books::SupplyZone;
use crate::world::{ActorId, KingdomId, OrgId, World};
use glam::Vec2;

fn pos2(world: &World, a: ActorId) -> Vec2 {
    world.actors[a as usize].pos.truncate()
}

fn live(world: &World, a: ActorId) -> bool {
    world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed)
}

/// True when `p` lies in an active supply zone of `kingdom`.
pub fn supplied_at(world: &World, kingdom: KingdomId, p: Vec2) -> bool {
    (0..world.books.zones.len()).any(|i| world.books.zones[i].kingdom == kingdom && zone_covers(world, i, p))
}

/// The tile under a ground point (the `0x552640` clamp into the map, then
/// the tile index).
fn tile_index(map: &crate::map::GameMap, p: Vec2) -> Option<u32> {
    if map.width == 0 || map.height == 0 || map.cell_size <= 0.0 {
        return None;
    }
    let x = ((p.x / map.cell_size).floor() as i64).clamp(0, map.width as i64 - 1);
    let y = ((p.y / map.cell_size).floor() as i64).clamp(0, map.height as i64 - 1);
    Some((y * map.width as i64 + x) as u32)
}

/// Zone `i` of `books.zones` covers the tile under `p` (its tile set, the
/// supply word of the tile, 004CF210/00554CC0). Without a tile map: the
/// zone circle.
pub fn zone_covers(world: &World, i: usize, p: Vec2) -> bool {
    let Some(z) = world.books.zones.get(i) else { return false };
    match (tile_index(&world.map, p), world.books.zone_tiles.get(i)) {
        (Some(t), Some(tiles)) => tiles.binary_search(&t).is_ok(),
        _ => (p - z.centre).length_squared() <= z.radius * z.radius,
    }
}

/// Zone `i` covers a tile of the rectangle `lo..=hi` (a footprint).
fn zone_covers_rect(world: &World, i: usize, lo: Vec2, hi: Vec2) -> bool {
    let map = &world.map;
    let (Some(a), Some(b), Some(tiles)) = (tile_index(map, lo), tile_index(map, hi), world.books.zone_tiles.get(i)) else {
        let Some(z) = world.books.zones.get(i) else { return false };
        let d = (z.centre - (lo + hi) * 0.5).abs() - (hi - lo) * 0.5;
        return d.max(Vec2::ZERO).length_squared() <= z.radius * z.radius;
    };
    let w = map.width;
    let (x0, y0, x1, y1) = (a % w, a / w, b % w, b / w);
    (y0..=y1).any(|y| (x0..=x1).any(|x| tiles.binary_search(&(y * w + x)).is_ok()))
}

/// ZoneSolver neighbour tables (`0xB322E0` deltas; `0xAB7804` next,
/// `0xAB7828` previous, `0xAB784C` / `0xAB7870` two steps on, `0xAB7894`
/// opposite): direction 0 is none, odd ones are diagonal.
const DELTA: [(i32, i32); 9] = [(0, 0), (-1, -1), (-1, 0), (-1, 1), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1)];
const NEXT: [usize; 9] = [0, 2, 3, 4, 5, 6, 7, 8, 1];
const PREVIOUS: [usize; 9] = [0, 8, 1, 2, 3, 4, 5, 6, 7];
const NEXT2: [usize; 9] = [0, 3, 4, 5, 6, 7, 8, 1, 2];
const PREVIOUS2: [usize; 9] = [0, 7, 8, 1, 2, 3, 4, 5, 6];
const OPPOSITE: [usize; 9] = [0, 5, 6, 7, 8, 1, 2, 3, 4];

/// A ZoneSolver open-list entry, smallest cost first.
#[derive(PartialEq)]
struct Open {
    cost: f32,
    order: u32,
    x: i32,
    y: i32,
}
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.total_cmp(&self.cost).then(other.order.cmp(&self.order))
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The tiles a supply zone covers (zone type 0 is not fast-built): the
/// ZoneSolver Dijkstra flood `0x5402E0` (functor `0xA36910`, visit
/// `0x53F9F0`, aa-zones `flood::add`) from the zone's tile. A step costs one
/// tile, sqrt 2 diagonally (`0x53C5C0`); from a tile only the five
/// directions ahead of the one it was first reached from are tried
/// (`0x554ED0`), never an impassable tile (flags20 bit 0) nor a diagonal
/// past an impassable corner; a tile joins while its cost is within the
/// radius and the step does not cross between land and water (flags20 bit
/// 8), and spreads on while half a tile plus its cost is below the radius.
/// Supply zones pass every blocker (only ZOC zones stop at them). Sorted.
pub fn flood_tiles(map: &crate::map::GameMap, sx: i32, sy: i32, radius: f32) -> Vec<u32> {
    use crate::map::BLOCKED;
    let (w, h) = (map.width as i32, map.height as i32);
    let tile = map.cell_size;
    let flags = |x: i32, y: i32| if x < 0 || y < 0 || x >= w || y >= h { BLOCKED } else { map.flags[(y * w + x) as usize] };
    let index = |x: i32, y: i32| (y * w + x) as u32;
    let mut out = Vec::new();
    if w <= 0 || h <= 0 || tile <= 0.0 || map.flags.len() < (w * h) as usize || !(radius >= 0.0) || flags(sx, sy) & BLOCKED != 0 {
        return out;
    }
    // per reached tile: the direction it was first reached from, its cost
    let mut seen: std::collections::HashMap<u32, (usize, f32)> = std::collections::HashMap::new();
    seen.insert(index(sx, sy), (0, 0.0));
    out.push(index(sx, sy));
    let mut open = std::collections::BinaryHeap::new();
    let mut order = 0u32;
    open.push(Open { cost: 0.0, order, x: sx, y: sy });
    let half = tile * 0.5;
    let blocked = |x: i32, y: i32| flags(x, y) & BLOCKED != 0;
    while let Some(Open { cost, x, y, .. }) = open.pop() {
        let came = seen.get(&index(x, y)).map_or(0, |s| s.0);
        let valid = |dir: usize| {
            let (dx, dy) = DELTA[dir];
            if OPPOSITE[came] == dir || blocked(x + dx, y + dy) {
                return false;
            }
            if dir & 1 == 0 {
                return true;
            }
            let (px, py) = DELTA[PREVIOUS[dir]];
            let (nx, ny) = DELTA[NEXT[dir]];
            !blocked(x + px, y + py) && !blocked(x + nx, y + ny)
        };
        let (first, end) = if came == 0 { (1, 8) } else { (PREVIOUS2[came], NEXT2[came]) };
        let mut dirs = Vec::with_capacity(8);
        let mut dir = first;
        loop {
            if valid(dir) {
                dirs.push(dir);
            }
            if dir == end {
                break;
            }
            dir = NEXT[dir];
            if dir == first {
                break;
            }
        }
        for dir in dirs {
            let (dx, dy) = DELTA[dir];
            let (nx, ny) = (x + dx, y + dy);
            let nt = index(nx, ny);
            let d = if dir & 1 != 0 { std::f32::consts::SQRT_2 } else { 1.0 } * tile + cost;
            let mut within_before = false;
            match seen.get(&nt) {
                None => {
                    seen.insert(nt, (dir, d));
                }
                Some(&(first_dir, old)) => {
                    if radius < d || !(d + 1.0e-4 < old) {
                        continue;
                    }
                    within_before = old <= radius;
                    seen.insert(nt, (first_dir, d));
                }
            }
            let same_ground = ((flags(nx, ny) >> 8) ^ (flags(x, y) >> 8)) & 1 == 0;
            if !same_ground || radius < d {
                if let Some(s) = seen.get_mut(&nt) {
                    s.1 = f32::MAX;
                }
                continue;
            }
            if !within_before {
                out.push(nt);
            }
            if half + d < radius {
                order += 1;
                open.push(Open { cost: d, order, x: nx, y: ny });
            }
        }
    }
    out.sort_unstable();
    out
}

/// (fog owner) The tile changes of this tick's movement (`0x4D6350`, the
/// supply zones follow their providers) before the relay searches read the
/// zone tiles.
fn zone_observe(world: &mut World) {
    let Some(mut e) = crate::fog::take_engine(world) else { return };
    e.observe(world);
    e.refresh_supply_tiles();
    crate::fog::put_engine(world, e);
    publish_zone_tiles(world);
}

/// (fog owner) The supply component zones of `books.zones` on aa-zones
/// (`0x58AE10` tail), then their tiles; without an engine (a map aa-zones
/// cannot describe) the f32 flood.
fn zone_tick(world: &mut World) {
    let Some(mut e) = crate::fog::take_engine(world) else {
        flood_zones(world);
        return;
    };
    let providing: Vec<(ActorId, f32)> = world.books.zones.iter().map(|z| (z.provider, z.radius)).collect();
    e.supply_tick(world, &providing);
    e.refresh_supply_tiles();
    crate::fog::put_engine(world, e);
    publish_zone_tiles(world);
}

/// (fog owner) `books.zone_tiles` from the engine's supply zone tiles, in
/// `books.zones` order.
pub fn publish_zone_tiles(world: &mut World) {
    let Some(e) = world.fog.engine.as_ref() else { return };
    let tiles = world.books.zones.iter().map(|z| e.supply_tiles.get(&z.provider).cloned().unwrap_or_default()).collect();
    world.books.zone_tiles = tiles;
}

/// The tile set of each zone in `books.zones` (cached floods).
fn flood_zones(world: &mut World) {
    let map = world.map.clone();
    let mut tiles = Vec::with_capacity(world.books.zones.len());
    if world.books.zone_floods.len() > 4096 {
        world.books.zone_floods.clear();
    }
    for z in world.books.zones.clone() {
        let Some(t) = tile_index(&map, z.centre) else {
            tiles.push(std::sync::Arc::new(Vec::new()));
            continue;
        };
        let key = (t, z.radius.to_bits());
        let set = world
            .books
            .zone_floods
            .entry(key)
            .or_insert_with(|| std::sync::Arc::new(flood_tiles(&map, (t % map.width) as i32, (t / map.width) as i32, z.radius)))
            .clone();
        tiles.push(set);
    }
    world.books.zone_tiles = tiles;
}

/// 0x58ADB0: follow relay links until a primary source.
fn reaches_source(world: &World, start: ActorId) -> bool {
    let mut seen = Vec::new();
    let mut a = start;
    loop {
        if !live(world, a) || !world.data.def(world.actors[a as usize].def).has_supply {
            return false;
        }
        if !world.data.def(world.actors[a as usize].def).supply_relay {
            return true;
        }
        if seen.contains(&a) {
            return false;
        }
        seen.push(a);
        match world.books.actor(a).and_then(|b| b.supply_relay) {
            Some(next) => a = next,
            None => return false,
        }
    }
}

/// Half extents of the axis-aligned rectangle around a structure's blocking
/// shape (the actor's tile bounds +0x20..+0x30), zero without one. Same
/// convention as the movement footprint: the shape's width lies across the
/// facing, its height along it; a CIRCLE's size is its radius.
fn footprint_half(def: &crate::data::Def, facing: f32) -> Vec2 {
    let Some(size) = def.blocking_size else { return Vec2::ZERO };
    if def.blocking_shape.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("CIRCLE")) {
        return Vec2::splat(size[0].max(0.0));
    }
    let (hw, hh) = (size[0].max(0.0) * 0.5, size[1].max(0.0) * 0.5);
    let (s, c) = facing.sin_cos();
    Vec2::new(hw * s.abs() + hh * c.abs(), hw * c.abs() + hh * s.abs())
}

fn deploying(world: &World, a: ActorId) -> bool {
    world.books.actor(a).is_some_and(|b| b.deploy.is_some())
}

/// SupplyComponent 0x58AE10 for supplier `a`: its queue-1 Tick (`elapsed`
/// = 1 s, in the bucket its InitCreate registration or the saved list gave
/// it, `crate::scheduler`) and the immediate call of its InitExisting
/// 0x58B0E0 (`elapsed` = 0; a created thing runs it after its InitCreate).
/// A relay searches the zones of its kingdom over its footprint for a
/// supplier that reaches a primary source; out of supply it drains `elapsed`
/// of its stockpile, connected it recharges SupplyStockpileRechargeRate x
/// `elapsed` up to stockpile_max. Then 0x58ACE0 decides whether its zone
/// provides until its next tick: owned by a major kingdom (kingdom +0x190 ==
/// 1, a city before its capture, provides none: the Berlin film's slot-0
/// zones stay at radius -1), not packing or unpacking, a relay's stock,
/// radius >= 0.
pub fn supplier_tick(world: &mut World, a: ActorId, elapsed: f32) {
    if !live(world, a) || !world.data.def(world.actors[a as usize].def).has_supply {
        return;
    }
    let recharge = world.data.rule_ci("SupplyStockpileRechargeRate").unwrap_or(1.0);
    let def = world.data.def(world.actors[a as usize].def);
    let (relay, max) = (def.supply_relay, def.supply_stockpile_max);
    let kingdom = world.actors[a as usize].kingdom;
    let p = pos2(world, a);
    if relay {
        let mut link = None;
        if let Some(k) = kingdom {
            let half = footprint_half(def, world.actors[a as usize].facing);
            for (i, z) in world.books.zones.iter().enumerate() {
                // 004CF210 + 00554CC0: the zones over the tiles of the
                // relay's footprint rectangle, not only its centre.
                if z.kingdom != k || z.provider == a || !zone_covers_rect(world, i, p - half, p + half) {
                    continue;
                }
                if world.books.out_of_supply(z.provider) {
                    continue;
                }
                if reaches_source(world, z.provider) {
                    link = Some(z.provider);
                    break;
                }
            }
        }
        world.books.actor_mut(a).supply_relay = link;
    }
    let in_supply = kingdom.is_some() && (!relay || world.books.actor(a).and_then(|b| b.supply_relay).is_some_and(|r| live(world, r)));
    let b = world.books.actor_mut(a);
    b.supply_started = true;
    b.out_of_supply = !in_supply;
    if relay {
        if !in_supply {
            b.supply_stock = (b.supply_stock - elapsed).max(0.0);
        } else if b.supply_stock < max {
            b.supply_stock = (recharge * elapsed + b.supply_stock).min(max);
        }
    }
    let provides = kingdom.is_some_and(|k| {
        let stock_ok = !relay || world.books.actor(a).is_some_and(|b| !b.out_of_supply || b.supply_stock > 0.0);
        let independent = world.kingdoms.iter().any(|x| x.id == k && x.independent);
        !(independent || deploying(world, a) || !stock_ok || def.supply_range < 0.0)
    });
    world.books.actor_mut(a).zone_active = provides;
}

/// `books.zones` from the suppliers whose last 0x58AE10 found them
/// providing, at their current positions, then their tiles.
fn publish_zones(world: &mut World) {
    let mut zones = Vec::new();
    for a in 0..world.actors.len() as ActorId {
        if !live(world, a) || !world.books.actor(a).is_some_and(|b| b.zone_active) {
            continue;
        }
        let actor = &world.actors[a as usize];
        let def = world.data.def(actor.def);
        let Some(k) = actor.kingdom.filter(|_| def.has_supply) else { continue };
        zones.push(SupplyZone { provider: a, kingdom: k, centre: actor.pos.truncate(), radius: def.supply_range });
    }
    world.books.zones = zones;
    zone_tick(world);
}

/// The per-update supply pass: the InitExisting 0x58AE10(0) of suppliers
/// new this update, the zones of the suppliers whose last 0x58AE10 found
/// them providing (following their providers' tiles), then every actor's
/// and organization's in-supply state.
pub fn tick(world: &mut World) {
    zone_observe(world);
    // The new suppliers' InitExisting in creation (index) order, each seeing
    // the zones of those before it, as each runs inside its own creation.
    for a in 0..world.actors.len() as ActorId {
        if live(world, a) && world.data.def(world.actors[a as usize].def).has_supply && !world.books.actor(a).is_some_and(|b| b.supply_started) {
            supplier_tick(world, a, 0.0);
            if world.books.actor(a).is_some_and(|b| b.zone_active) {
                publish_zones(world);
            }
        }
    }
    publish_zones(world);
    // 0x4D0A80 for organizations, their members and lone actors.
    for o in 0..world.orgs.len() as OrgId {
        let org = &world.orgs[o as usize];
        if !org.alive {
            continue;
        }
        let def = world.data.def(org.def);
        if !def.supply_used {
            continue;
        }
        let attached = def.in_supply_detached || org.hq.is_some_and(|h| live(world, h));
        let at = org_position(world, o);
        // (combat owner) a Resupply special operation's SUPPLIED flag
        // keeps the organization in supply while it lasts
        let in_supply = (attached && supplied_at(world, org.kingdom, at)) || crate::special_ops::org_has_flag(world, o, "SUPPLIED");
        world.books.org_mut(o).out_of_supply = !in_supply;
        for m in world.orgs[o as usize].members.clone() {
            world.books.actor_mut(m).out_of_supply = !in_supply;
        }
    }
    for a in 0..world.actors.len() as ActorId {
        let actor = &world.actors[a as usize];
        if !actor.alive || actor.removed || world.data.def(actor.def).has_supply {
            continue;
        }
        if let Some(o) = actor.org
            && world.data.def(world.orgs[o as usize].def).supply_used
        {
            continue;
        }
        let in_supply = actor.kingdom.is_some_and(|k| supplied_at(world, k, actor.pos.truncate()));
        world.books.actor_mut(a).out_of_supply = !in_supply;
    }
}

/// KKC_OrganizationComponent::TickMaintenance 0x57C8E0 for organization
/// `o` (queue 1, `elapsed` = 1 s): the shortfall attrition 0x578660 of an
/// out-of-supply supply_used organization of a kingdom short of upkeep,
/// then behind the 0x579110 gate the regeneration 0x57B5A0 of one slot.
pub fn org_maintenance(world: &mut World, o: OrgId, elapsed: f32) {
    if !world.orgs.get(o as usize).is_some_and(|x| x.alive) {
        return;
    }
    crate::economy::shortfall(world, o, elapsed);
    regenerate(world, o, elapsed);
}

/// One city zone candidate: a lone actor or an organization (as the native
/// organization actor at its own position).
struct CityCandidate {
    kingdom: KingdomId,
    pos: Vec2,
    /// 4D0180: the current value the city tick weighs.
    current: f32,
}

/// Performer +60 (57E430): the combat value of a definition with a
/// PerformerComponent, None without one.
fn performer_value(world: &World, def: crate::data::DefId) -> Option<f32> {
    let d = world.data.def(def);
    if !d.has_component("PerformerComponent") {
        return None;
    }
    Some(world.combat.extras.as_ref().map_or(0.0, |e| e.def(def).combat_value))
}

/// 57DC40: the Body health ratio (Body +10 / +14), 1 without a Body.
fn health_ratio(world: &World, a: ActorId) -> f32 {
    let actor = &world.actors[a as usize];
    let d = world.data.def(actor.def);
    if !d.has_component("BodyComponent") {
        return 1.0;
    }
    let max = world
        .combat
        .actors
        .get(a as usize)
        .filter(|c| c.initialized && c.max_health > 0.0)
        .map_or_else(|| crate::production::max_health(world, actor.kingdom, actor.def), |c| c.max_health);
    if max > 0.0 { actor.health / max } else { 1.0 }
}

/// 5640A0 kingdom gate: owned by a major (non-independent) kingdom
/// (kingdom +400 == 0).
fn major(world: &World, k: Option<KingdomId>) -> Option<KingdomId> {
    k.filter(|&k| world.kingdoms.get(k as usize).is_some_and(|x| !x.independent))
}

/// City candidate admission 0x5640A0 for every actor and organization:
/// not dead, owned by a major kingdom, not an organization member (4549C0:
/// the organization stands for it), thing flag bit 12 (flying, 0x1000)
/// clear, actor value 4D01E0(0) > 0, and without a Body or captureable or
/// destructible. 4D01E0 is Organization +6C (5788B0 maximum: the members'
/// Performer +60 plus the empty layout slots' ActorData +1E4) + Settlement
/// +34 + Performer +60 + Denizen +14; 4D0180, the weight, uses Organization
/// +68 (the members' Performer +60 times their health ratio), Settlement
/// +30, 57DC40 (Performer +60 times the health ratio) and Denizen +10.
/// aa-play has no Settlement or Denizen occupants: those terms are 0.
fn city_candidates(world: &World) -> Vec<CityCandidate> {
    let mut out = Vec::new();
    for a in 0..world.actors.len() as ActorId {
        let actor = &world.actors[a as usize];
        if !actor.alive || actor.removed || actor.org.is_some() {
            continue;
        }
        let Some(kingdom) = major(world, actor.kingdom) else { continue };
        let d = world.data.def(actor.def);
        if d.flying {
            continue;
        }
        if d.has_component("BodyComponent") && !d.captureable && !d.destructible {
            continue;
        }
        let Some(value) = performer_value(world, actor.def) else { continue };
        if value <= 0.0 {
            continue;
        }
        out.push(CityCandidate { kingdom, pos: actor.pos.truncate(), current: value * health_ratio(world, a) });
    }
    for o in 0..world.orgs.len() as OrgId {
        let org = &world.orgs[o as usize];
        if !org.alive || !org.members.iter().any(|&m| live(world, m)) {
            continue;
        }
        let Some(kingdom) = major(world, Some(org.kingdom)) else { continue };
        let d = world.data.def(org.def);
        if d.flying {
            continue;
        }
        if d.has_component("BodyComponent") && !d.captureable && !d.destructible {
            continue;
        }
        let (mut maximum, mut current) = (0.0f32, 0.0f32);
        for (slot, (slot_def, _)) in d.slots.iter().enumerate() {
            let member = org.members.iter().copied().find(|&m| live(world, m) && world.actors[m as usize].slot == Some(slot as u16));
            match member {
                Some(m) => {
                    if let Some(v) = performer_value(world, world.actors[m as usize].def) {
                        current += health_ratio(world, m) * v;
                        maximum += v;
                    }
                }
                None => {
                    if *slot_def != crate::data::NO_DEF
                        && let Some(v) = performer_value(world, *slot_def)
                    {
                        maximum += v;
                    }
                }
            }
        }
        if maximum <= 0.0 {
            continue;
        }
        out.push(CityCandidate { kingdom, pos: org_position(world, o), current });
    }
    out
}

/// City tick 0x5647A0 (queue 1, the City component) for city `c`. Against
/// the current owner: the value of its friends
/// in the zone defends, the value of its enemies attacks. Attackers with no
/// defender (or no owner at all) let the zone decide: the first kingdom in
/// kingdom order that has value there, no enemy value against it, and no
/// friend with more value or under threat takes the city (0x4D5230 owner
/// change, reason 10). The neutral buildings of the old owner around the
/// city change hands with it.
pub fn city_tick(world: &mut World, c: ActorId) {
    use crate::books::{ENEMY, FRIEND, SELF};
    let n = world.kingdoms.len();
    {
        let Some(city) = world.actors.get(c as usize) else { return };
        if !city.alive || city.removed {
            return;
        }
        let def = world.data.def(city.def);
        if !def.city || def.city_zone_radius <= 0.0 {
            return;
        }
    }
    let candidates = city_candidates(world);
    {
        let city = &world.actors[c as usize];
        let def = world.data.def(city.def);
        let (centre, radius, owner) = (city.pos.truncate(), def.city_zone_radius, city.kingdom);
        // The candidates in the zone (the city itself never is one: it has no
        // Performer and belongs to its owner).
        let mut present = vec![0.0f32; n];
        let mut any = false;
        for cand in &candidates {
            if cand.pos.distance_squared(centre) <= radius * radius && (cand.kingdom as usize) < n {
                present[cand.kingdom as usize] += cand.current;
                any = true;
            }
        }
        if !any {
            return;
        }
        let rel = |a: usize, b: usize| world.books.relation(&world.kingdoms, Some(a as u32), Some(b as u32));
        let decide = match owner {
            None => true,
            Some(o) => {
                let (mut defend, mut attack) = (0.0f32, 0.0f32);
                for k in 0..n {
                    match rel(k, o as usize) {
                        SELF | FRIEND => defend += present[k],
                        ENEMY => attack += present[k],
                        _ => {}
                    }
                }
                attack > 0.0 && defend <= 0.0
            }
        };
        if !decide {
            return;
        }
        let threat: Vec<f32> = (0..n).map(|k| (0..n).filter(|&e| e != k && rel(e, k) == ENEMY).map(|e| present[e]).sum()).collect();
        let winner = (0..n).find(|&k| {
            threat[k] <= 0.0
                && present[k] > 0.0
                && !(0..n).any(|f| f != k && rel(k, f) == FRIEND && (present[k] < present[f] || threat[f] > 0.0))
        });
        let Some(w) = winner.map(|k| k as u32) else { return };
        if Some(w) == owner {
            return;
        }
        world.actors[c as usize].kingdom = Some(w);
        world.events.push(crate::world::Event::Captured { actor: c, kingdom: w });
        // The city's neutral buildings (city blocks) go with it.
        if let Some(o) = owner {
            for b in 0..world.actors.len() {
                let a = &world.actors[b];
                if a.kingdom == Some(o)
                    && !a.removed
                    && b as ActorId != c
                    && world.data.def(a.def).city_zone_radius <= 0.0
                    && world.data.def(a.def).class == Some(crate::data::DefClass::Building)
                    && a.pos.truncate().distance_squared(centre) <= radius * radius
                {
                    world.actors[b].kingdom = Some(w);
                }
            }
        }
    }
}
/// The organization's own position (its GActor +20/+24): set at its
/// creation and moved after its leader only by its maintenance tick
/// 0x57C8E0 -> 0x57C800 -> 0x57C680 (the Berlin film: a new regiment stays
/// at its creation spot while its truck stands 10 m away). The movement
/// owner keeps it in `Organization::anchor`.
pub fn org_position(world: &World, o: OrgId) -> Vec2 {
    world.orgs[o as usize].anchor
}

/// 0x579110 gate inputs owned by combat: engaged (an engage target) or
/// routing (mind_routing has resupply = false); a member shooting counts too.
fn engaged(world: &World, o: OrgId) -> bool {
    if crate::combat::org_state(world, o).is_some_and(|c| c.engaging() || c.routing.is_some()) {
        return true;
    }
    world.orgs[o as usize].members.iter().any(|&m| live(world, m) && world.actors[m as usize].combat.target.is_some())
}

/// 0x57B5A0 behind the 0x579110 gate for organization `o`, `dt` = the
/// maintenance tick's elapsed time.
fn regenerate(world: &mut World, o: OrgId, dt: f32) {
    {
        let org = &world.orgs[o as usize];
        if !org.alive {
            return;
        }
        let def = world.data.def(org.def);
        if !def.supply_used || world.books.org(o).is_none_or(|b| b.out_of_supply) || engaged(world, o) {
            return;
        }
        let shortage = world.books.kingdom(org.kingdom).map_or(0.0, |k| k.shortage_ratio);
        let factor = if shortage > 0.0 { 1.0 - shortage } else { 1.0 };
        let slots = def.slots.len();
        let member_at = |slot: usize| {
            org.members.iter().copied().find(|&m| live(world, m) && world.actors[m as usize].slot == Some(slot as u16))
        };
        // 5617C0 against Body +14, the actual maximum (handicap, technology
        // and experience HEALTH modifiers).
        let damaged = |m: ActorId| world.actors[m as usize].health < member_max_health(world, m);
        let leader = def.leader_slot.map(usize::from).filter(|&l| l < slots);
        let choice = leader
            .filter(|&l| member_at(l).is_some_and(damaged))
            .or_else(|| (0..slots).find(|&s| member_at(s).is_some_and(damaged)))
            .or_else(|| leader.filter(|&l| member_at(l).is_none() && def.slots[l].0 != u32::MAX))
            .or_else(|| (0..slots).find(|&s| member_at(s).is_none() && def.slots[s].0 != u32::MAX && world.data.def(def.slots[s].0).has_component("BodyComponent")));
        let Some(slot) = choice else { return };
        if let Some(m) = member_at(slot) {
            // Element +20 (5704D0: resupply_rate through the member's
            // RESUPPLY_RATE modifier) * Body +14 * (1 - shortage) * delta,
            // one member at a time until it is full (57B5A0 / 562BA0).
            let md = world.data.def(world.actors[m as usize].def);
            let rate = md.resupply_rate * crate::production::multiply_factor(world, org.kingdom, &md.properties, "RESUPPLY_RATE");
            let rate = rate + md.resupply_rate * (crate::special_ops::org_resupply_multiplier(world, o) - 1.0);
            let max = member_max_health(world, m);
            let amount = rate * max * factor * dt;
            let a = &mut world.actors[m as usize];
            a.health = (a.health + amount).min(max);
            return;
        }
        let slot_def = def.slots[slot].0;
        let sd = world.data.def(slot_def);
        // 0x5014E0: the organization modifier context multiplies +2CC
        // (technologies such as tech_plentiful_ammunition: RESUPPLY_RATE).
        let rate = sd.resupply_rate * crate::production::multiply_factor(world, org.kingdom, &def.properties, "RESUPPLY_RATE");
        // (combat owner) the enchantments' RESUPPLY_RATE (Resupply special
        // operations) in the same 502A70 sum
        let rate = rate + sd.resupply_rate * (crate::special_ops::org_resupply_multiplier(world, o) - 1.0);
        let amount = sd.health * rate * factor * dt;
        let b = world.books.org_mut(o);
        if b.progress.len() < slots {
            b.progress.resize(slots, 0.0);
        }
        b.progress[slot] += amount;
        if b.progress[slot] >= sd.health {
            b.progress[slot] = 0.0;
            // 57AD30 -> 4FBF40: out of the nearest recruiting structure's
            // door when one is within control range, else at the slot.
            crate::production::spawn_member(world, o, slot);
        }
    }
}

/// Body +14 of a member: the combat maximum (with the experience level),
/// else the definition health with the kingdom HEALTH modifiers.
fn member_max_health(world: &World, m: ActorId) -> f32 {
    let a = &world.actors[m as usize];
    world
        .combat
        .actors
        .get(m as usize)
        .filter(|c| c.initialized && c.max_health > 0.0)
        .map_or_else(|| crate::production::max_health(world, a.kingdom, a.def), |c| c.max_health)
}
