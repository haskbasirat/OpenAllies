//! The battle's zone engine (aa_play::zones over aa-zones) against direct
//! aa-zones calls and against the earlier f32 paths:
//!
//! * supply zones: the engine's tile sets and supply words equal a direct
//!   replay of the same native operations (`0x515F40` moves, `0x514CE0`
//!   releases) on a fresh `ZoneMap` with a minimal reference world, after
//!   every step of random walks over walled, watery, bordered maps;
//! * supply zones against `supply::flood_tiles` (the f32 flood);
//! * fog: after every tick of a moving battle the engine's tile fog words,
//!   visible halves and tile zone sets equal a from-scratch evaluation of its
//!   active detection zones with the aa-zones circle primitives
//!   (`0x53F730` spans, `0x53F6C0` centre test, the native column clip).
use aa_numeric::Precision;
use aa_play::data::DefClass;
use aa_play::fog::{self, FogMode};
use aa_play::map::{BLOCKED, NOT_LAND_PASSABLE};
use aa_play::zones::{self, Engine};
use aa_play::{Controller, Def, GameData, GameMap, Kingdom, Order, Organization, World};
use aa_zones::flood::FloodWorld;
use aa_zones::query::{Kingdom as ZKingdom, VisitWorld, ZoneActors};
use aa_zones::tile::{ComponentEffect, SaiHook, ZoneMap, ZoneWorld};
use aa_zones::zone::{self, Zone, ZoneObjectWorld};
use aa_zones::{ActorKey, Result};
use glam::{Vec2, Vec3};
use std::collections::BTreeSet;
use std::sync::Arc;

const PC: Precision = Precision::Double53;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// A map with the native GMap border (the outermost tiles impassable, as
/// `map.rs` builds every real map), optional walls and a lake.
fn bordered_map(size: u32, cell: f32, rng: Option<&mut Rng>) -> GameMap {
    let n = (size * size) as usize;
    let mut flags = vec![0u32; n];
    for y in 0..size {
        for x in 0..size {
            if x == 0 || y == 0 || x + 1 == size || y + 1 == size {
                flags[(y * size + x) as usize] |= BLOCKED;
            }
        }
    }
    if let Some(r) = rng {
        for _ in 0..6 {
            let (x0, y0) = ((r.next() % size as u64) as u32, (r.next() % size as u64) as u32);
            let horizontal = r.next() % 2 == 0;
            let len = 8 + (r.next() % 24) as u32;
            for i in 0..len {
                let (x, y) = if horizontal { (x0 + i, y0) } else { (x0, y0 + i) };
                if x < size && y < size {
                    flags[(y * size + x) as usize] |= BLOCKED;
                }
            }
        }
        let (cx, cy, rr) = ((r.next() % size as u64) as i32, (r.next() % size as u64) as i32, 4 + (r.next() % 10) as i32);
        for y in 0..size as i32 {
            for x in 0..size as i32 {
                if (x - cx).pow(2) + (y - cy).pow(2) <= rr * rr {
                    flags[(y as u32 * size + x as u32) as usize] |= NOT_LAND_PASSABLE;
                }
            }
        }
    }
    GameMap {
        id: "zones-parity".into(),
        width: size,
        height: size,
        cell_size: cell,
        heights: vec![0.0; ((size + 1) * (size + 1)) as usize],
        flags,
        speed_limit: vec![f32::MAX; n],
        ..Default::default()
    }
}

fn kingdom(id: u32, team: u32) -> Kingdom {
    Kingdom {
        id,
        name: format!("k{id}"),
        nation: "rus".into(),
        team,
        color: [255, 0, 0],
        controller: Controller::Human,
        stockpile: [0.0; 3],
        income: [0.0; 3],
        unit_limit: 0.0,
        units_used: 0.0,
        defeated: false,
        independent: false,
    }
}

fn supply_world(map: GameMap) -> (World, u32) {
    let mut data = GameData::default();
    data.by_id.insert("truck".into(), 0);
    data.defs.push(Def { id: "truck".into(), class: Some(DefClass::Truck), health: 10.0, speed_max: 4.0, ..Default::default() });
    let mut w = World::empty(Arc::new(data), Arc::new(map), 3);
    w.kingdoms.push(kingdom(0, 0));
    w.kingdoms.push(kingdom(1, 1));
    (w, 0)
}

/// The minimal world of a direct supply-zone replay: owners, tile flags,
/// no fog side effects.
struct RefWorld {
    flags: Vec<u32>,
    owners: Vec<u32>,
}

impl ZoneActors for RefWorld {
    fn owner(&mut self, a: ActorKey) -> Result<u32> {
        Ok(zones::kingdom_key(self.owners[(a - 1) as usize]))
    }
    fn allied(&mut self, a: ActorKey, k: u32) -> Result<bool> {
        Ok(self.owner(a)? == k)
    }
    fn enemy(&mut self, a: ActorKey, k: u32) -> Result<bool> {
        Ok(self.owner(a)? != k)
    }
    fn zone_slot(&mut self, _a: ActorKey, _i: u32) -> Result<u32> {
        Ok(0)
    }
    fn word4(&mut self, _a: ActorKey) -> Result<u32> {
        Ok(0)
    }
    fn component_b0(&mut self, _a: ActorKey) -> Result<u32> {
        Ok(0)
    }
    fn reveal_mask(&mut self, _a: ActorKey) -> Result<Option<u32>> {
        Ok(None)
    }
}
impl VisitWorld for RefWorld {
    fn kingdom(&mut self, key: u32) -> Result<ZKingdom> {
        Ok(ZKingdom { key, index: key - zones::KINGDOM, nonmajor: 0, shared_mask: 0, supply_mask: 0 })
    }
    fn kingdom_slot(&mut self, i: u32) -> Result<u32> {
        Ok(zones::kingdom_key(i))
    }
}
impl ZoneWorld for RefWorld {
    fn fog_enabled(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn sai_enabled(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn sai(&mut self, _h: SaiHook, _t: usize, _k: u32) -> Result<()> {
        Ok(())
    }
    fn kingdom_list(&mut self) -> Result<Vec<u32>> {
        Ok(vec![zones::kingdom_key(0), zones::kingdom_key(1)])
    }
    fn tile_actors(&mut self, _t: usize) -> Result<Vec<ActorKey>> {
        Ok(Vec::new())
    }
    fn actor_reveal(&mut self, _a: ActorKey, _k: u32, _e: bool, _v: bool) -> Result<()> {
        Ok(())
    }
    fn actor_hide(&mut self, _a: ActorKey, _k: u32) -> Result<()> {
        Ok(())
    }
    fn actor_unexplore(&mut self, _a: ActorKey, _k: u32) -> Result<()> {
        Ok(())
    }
    fn zone_effects_enabled(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn actor_b4(&mut self, _a: ActorKey) -> Result<bool> {
        Ok(true)
    }
    fn actor_multi_tile(&mut self, _a: ActorKey) -> Result<bool> {
        Ok(false)
    }
    fn actor_tile(&mut self, _a: ActorKey) -> Result<i64> {
        Ok(-1)
    }
    fn actor_flags(&mut self, _a: ActorKey) -> Result<u32> {
        Ok(0)
    }
    fn actor_redirect(&mut self, _a: ActorKey) -> Result<Option<ActorKey>> {
        Ok(None)
    }
    fn component(&mut self, _a: ActorKey, _o: u32) -> Result<u32> {
        Ok(0)
    }
    fn component_effect(&mut self, _e: ComponentEffect, _c: u32, _t: ActorKey) -> Result<()> {
        Ok(())
    }
}
impl FloodWorld for RefWorld {
    fn tile_flags(&mut self, t: usize) -> Result<u32> {
        Ok(self.flags[t])
    }
    fn tile_blocker(&mut self, _t: usize) -> Result<u32> {
        Ok(0)
    }
    fn assert_blocked_start(&mut self) -> Result<bool> {
        Ok(true)
    }
}
impl ZoneObjectWorld for RefWorld {
    fn solver_present(&mut self, _r: bool) -> Result<bool> {
        Ok(true)
    }
    fn fast_detection_zones(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn presentation(&mut self, _o: u32, _x: u32, _y: u32, _r: u32, _f: u16) -> Result<()> {
        Ok(())
    }
    fn log_implicit_clear(&mut self, _a: u32, _f: u16) -> Result<()> {
        Ok(())
    }
    fn actor_id(&mut self, a: ActorKey) -> Result<u32> {
        Ok(a)
    }
}

fn tile_of(p: Vec2, cell: f32) -> (i32, i32) {
    ((p.x / cell).floor() as i32, (p.y / cell).floor() as i32)
}

/// Random walks of supply providers: the engine (observe + supply tick)
/// against the same native operations replayed directly on a fresh map.
#[test]
fn supply_zones_equal_a_direct_aa_zones_replay() {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut checked = 0usize;
    for case in 0..6 {
        let cell = if case % 2 == 0 { 4.0 } else { 2.0 };
        let size = 96u32;
        let map = bordered_map(size, cell, Some(&mut rng));
        let (mut w, truck) = supply_world(map.clone());
        let mut engine = Engine::new(&w).expect("power-of-two tiles");
        let mut reference = ZoneMap::new(engine.map.geometry).unwrap();
        let n = 6;
        let mut providers = Vec::new();
        let mut radius = Vec::new();
        let span = size as f32 * cell;
        // providers stand on passable tiles, as the movement keeps them (a
        // zone started on an impassable tile cannot be released: see
        // released_on_an_impassable_tile)
        let passable = |p: Vec2| {
            let (x, y) = tile_of(p, cell);
            x >= 0 && y >= 0 && x < size as i32 && y < size as i32 && map.flags[(y as u32 * size + x as u32) as usize] & BLOCKED == 0
        };
        for i in 0..n {
            let p = loop {
                let p = Vec2::new(cell + rng.f() * (span - 2.0 * cell), cell + rng.f() * (span - 2.0 * cell));
                if passable(p) {
                    break p;
                }
            };
            providers.push(w.spawn_actor(truck, Some((i % 2) as u32), p.extend(0.0), 0.0));
            radius.push(8.0 + rng.f() * 30.0 * cell);
        }
        let mut rw = RefWorld { flags: map.flags.clone(), owners: providers.iter().map(|&a| w.actors[a as usize].kingdom.unwrap()).collect() };
        let mut zones_ref: Vec<Zone> = (0..n).map(|i| Zone::new(0x100 + i as u32, i as u32 + 1, 1)).collect();
        let mut tiles_ref: Vec<Option<(i32, i32)>> = vec![None; n];
        let mut on = vec![false; n];
        for step in 0..60 {
            // move: small steps mostly, sometimes a jump; toggle providing
            for i in 0..n {
                let a = &mut w.actors[providers[i] as usize];
                let jump = rng.next() % 9 == 0;
                let d = if jump { Vec2::new(rng.f() - 0.5, rng.f() - 0.5) * span * 0.5 } else { Vec2::new(rng.f() - 0.5, rng.f() - 0.5) * cell * 1.5 };
                let p = (a.pos.truncate() + d).clamp(Vec2::splat(-cell), Vec2::splat(span + cell));
                if passable(p) {
                    a.pos = p.extend(0.0);
                }
                if rng.next() % 7 == 0 || step == 0 {
                    on[i] = !on[i] || step == 0;
                }
            }
            let providing: Vec<(u32, f32)> = (0..n).filter(|&i| on[i]).map(|i| (providers[i], radius[i])).collect();
            engine.observe(&w);
            engine.supply_tick(&w, &providing);
            engine.refresh_supply_tiles();
            // the same operations, directly: 0x4D6350 moves the active zone
            // on a tile change, then 0x58AE10 starts / stops it
            for i in 0..n {
                let p = w.actors[providers[i] as usize].pos.truncate();
                let t = tile_of(p, cell);
                let bits = [p.x.to_bits(), p.y.to_bits()];
                if tiles_ref[i].is_some() && tiles_ref[i] != Some(t) && zones_ref[i].active() {
                    let r = zones_ref[i].radius_bits;
                    zone::move_to_position(&mut reference, &mut rw, &mut zones_ref[i], bits, r, PC).unwrap();
                }
                tiles_ref[i] = Some(t);
            }
            for i in 0..n {
                let p = w.actors[providers[i] as usize].pos.truncate();
                let bits = [p.x.to_bits(), p.y.to_bits()];
                match (on[i], zones_ref[i].active()) {
                    (true, false) => zone::move_to_position(&mut reference, &mut rw, &mut zones_ref[i], bits, radius[i].to_bits(), PC).unwrap(),
                    (false, true) => zone::release(&mut reference, &mut rw, &mut zones_ref[i], PC).unwrap_or_else(|e| panic!("case {case} step {step} release {i} at {:?}: {e}", zones_ref[i])),
                    _ => {}
                }
            }
            assert!(engine.parts.stats.refusals.is_empty(), "{:?}", engine.parts.stats.refusals);
            for t in 0..reference.tiles.len() {
                let a = engine.map.pool.entries(engine.map.tiles[t].set).unwrap();
                let b = reference.pool.entries(reference.tiles[t].set).unwrap();
                assert_eq!(a, b, "case {case} step {step} tile {t} zone sets");
                assert_eq!(engine.map.tiles[t].supply, reference.tiles[t].supply, "case {case} step {step} tile {t} supply word");
            }
            for i in 0..n {
                let want: Vec<u32> = (0..reference.tiles.len() as u32)
                    .filter(|&t| reference.tiles[t as usize].set.is_some_and(|s| aa_zones::query::contains(&reference.pool, s, i as u32 + 1, 1).unwrap()))
                    .collect();
                let got = engine.supply_tiles.get(&providers[i]).map(|v| v.to_vec()).unwrap_or_default();
                assert_eq!(got, want, "case {case} step {step} provider {i} tiles");
                checked += got.len();
            }
        }
    }
    assert!(checked > 10_000, "{checked}");
}

/// A refusal kept from the native code: a flood zone started on an
/// impassable tile holds no tile (`0x5402E0` returns at once), and its
/// release walks from that tile and removes the zone from it (`0x53F890`)
/// on a null zone set: the original dereferences null (`0x54FA80`). The
/// engine counts the refusal, keeps running and the zone stays active.
#[test]
fn released_on_an_impassable_tile() {
    let mut map = bordered_map(32, 4.0, None);
    map.flags[(10 * 32 + 10) as usize] |= BLOCKED;
    let (mut w, truck) = supply_world(map);
    let a = w.spawn_actor(truck, Some(0), Vec3::new(42.0, 42.0, 0.0), 0.0);
    let mut e = Engine::new(&w).unwrap();
    e.observe(&w);
    e.supply_tick(&w, &[(a, 20.0)]);
    e.refresh_supply_tiles();
    assert!(e.supply_tiles.get(&a).is_some_and(|t| t.is_empty()));
    e.supply_tick(&w, &[]);
    let r: Vec<&String> = e.parts.stats.refusals.keys().collect();
    assert_eq!(r, ["native null zone-set dereference (remove on an empty tile)"]);
}

/// `0x4D8450`: a captured building's zones are released under the old owner
/// and put back for the new one: its ZOD tiles become visible to the new
/// owner and stop being visible to the old one.
#[test]
fn a_captured_building_sees_for_its_new_owner() {
    let (data, ids) = fog_data();
    let mut w = World::empty(Arc::new(data), Arc::new(bordered_map(96, 4.0, None)), 2);
    w.kingdoms.push(kingdom(0, 0));
    w.kingdoms.push(kingdom(1, 1));
    fog::set_mode(&mut w, FogMode::Full);
    let corps = w.spawn_actor(ids.corps, Some(0), Vec3::new(180.0, 180.0, 0.0), 0.0);
    fog::tick(&mut w);
    let near = Vec2::new(200.0, 180.0);
    assert_eq!(fog::point_view(&w, Some(0), near), fog::TileView::Visible);
    assert_eq!(fog::point_view(&w, Some(1), near), fog::TileView::Unexplored);
    w.actors[corps as usize].kingdom = Some(1);
    fog::tick(&mut w);
    assert_eq!(fog::point_view(&w, Some(1), near), fog::TileView::Visible);
    assert_eq!(fog::point_view(&w, Some(0), near), fog::TileView::Explored);
    let e = fog::engine(&w).unwrap();
    assert!(e.parts.stats.refusals.is_empty(), "{:?}", e.parts.stats.refusals);
}
/// The exact ZoneSolver flood against the f32 flood (`supply::flood_tiles`)
/// for fresh zones (no incremental move) on random bordered maps.
#[test]
fn supply_zones_against_the_f32_flood() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut same, mut differ) = (0, 0);
    let mut first_difference = None;
    for case in 0..40 {
        let cell = [4.0f32, 2.0][case % 2];
        let map = bordered_map(96, cell, Some(&mut rng));
        let (mut w, truck) = supply_world(map.clone());
        let span = 96.0 * cell;
        let p = Vec2::new(cell + rng.f() * (span - 2.0 * cell), cell + rng.f() * (span - 2.0 * cell));
        let a = w.spawn_actor(truck, Some(0), p.extend(0.0), 0.0);
        let r = [60.0, 40.0, 25.0 + rng.f() * 100.0, (4 + case as u32) as f32 * cell][case % 4];
        let mut e = Engine::new(&w).unwrap();
        e.observe(&w);
        e.supply_tick(&w, &[(a, r)]);
        e.refresh_supply_tiles();
        let got = e.supply_tiles.get(&a).map(|v| v.to_vec()).unwrap_or_default();
        let (tx, ty) = tile_of(p, cell);
        let want = aa_play::supply::flood_tiles(&map, tx, ty, r);
        if got == want {
            same += 1;
        } else {
            differ += 1;
            first_difference.get_or_insert((case, r, p, got.len(), want.len()));
        }
    }
    println!("exact vs f32 flood: {same} equal, {differ} differ (first {first_difference:?})");
    assert_eq!(differ, 0, "first difference {first_difference:?}");
}

/// Core's relay case: an HQ zone of 60 m at (40, 40) on 2 m tiles reaches
/// the depot footprint tiles at x = 49..51 of row 20 (cost 58 and 60 m).
#[test]
fn hq_zone_extent_on_two_metre_tiles() {
    let map = bordered_map(128, 2.0, None);
    let (mut w, truck) = supply_world(map.clone());
    let hq = w.spawn_actor(truck, Some(0), Vec3::new(40.0, 40.0, 0.0), 0.0);
    let mut e = Engine::new(&w).unwrap();
    e.observe(&w);
    e.supply_tick(&w, &[(hq, 60.0)]);
    e.refresh_supply_tiles();
    let tiles = e.supply_tiles.get(&hq).unwrap();
    let row: Vec<u32> = tiles.iter().filter(|t| *t / 128 == 20).map(|t| t % 128).collect();
    println!("row 20: {}..={}", row[0], row[row.len() - 1]);
    assert_eq!((row[0], row[row.len() - 1]), (1, 50));
    assert_eq!(**tiles, aa_play::supply::flood_tiles(&map, 20, 20, 60.0));
}

// ---------------------------------------------------------------------------
// Fog.

struct Ids {
    soldier: u32,
    regiment: u32,
    corps: u32,
}

fn fog_data() -> (GameData, Ids) {
    let mut data = GameData::default();
    let mut push = |d: Def| -> u32 {
        let id = data.defs.len() as u32;
        data.by_id.insert(d.id.clone(), id);
        data.defs.push(d);
        id
    };
    let soldier = push(Def { id: "soldier".into(), class: Some(DefClass::Member), health: 10.0, detection_range: 18.0, remote_detection_range: 10.0, speed_max: 4.0, ..Default::default() });
    let regiment = push(Def { id: "regiment".into(), class: Some(DefClass::Organization), detection_range: 26.0, remote_detection_range: 12.0, control_range: 9.0, ..Default::default() });
    let corps = push(Def {
        id: "corps".into(),
        class: Some(DefClass::Building),
        health: 1000.0,
        detection_range: 65.0,
        remote_detection_range: 20.0,
        supply_range: 60.0,
        structure: true,
        blocking_size: Some([10.0, 10.0]),
        actor_data: true,
        ..Default::default()
    });
    (data, Ids { soldier, regiment, corps })
}

/// From scratch: the tiles each active fast-built zone covers (`0x53F730`
/// spans clipped to the map, the last column excluded, `0x53F6C0`), in the
/// Binary x87 model at PC53 (independent of the engine's binary64 path).
fn circle_tiles(map: &ZoneMap, z: &Zone) -> Vec<usize> {
    let g = map.geometry;
    let plan = aa_zones::solver::move_plan(map, z.x, z.y, aa_zones::zone::INACTIVE, z.x, z.y, z.radius_bits, PC).unwrap();
    let mut out = Vec::new();
    if !plan.add {
        return out;
    }
    for (x0, y, width) in aa_zones::solver::circle_spans_exact(z.x, z.y, plan.rn_tiles, PC).unwrap() {
        if y < 0 || y >= g.height {
            continue;
        }
        let end = (x0 + width).min(g.width - 1);
        for x in x0.max(0)..end {
            if aa_zones::solver::in_circle(map, &plan.new, x, y, PC).unwrap() {
                out.push((y * g.width + x) as usize);
            }
        }
    }
    out
}

/// After every tick of a moving, fighting battle: each tile's zone set holds
/// exactly the active detection zones whose circle covers it, and its
/// visible half is exactly the owners of those ZODs plus the reveal-mask
/// kingdoms of those RZODs.
#[test]
fn fog_tiles_equal_a_from_scratch_evaluation_of_the_zones() {
    let (data, ids) = fog_data();
    let map = bordered_map(128, 4.0, None);
    let mut w = World::empty(Arc::new(data), Arc::new(map), 9);
    w.kingdoms.push(kingdom(0, 0));
    w.kingdoms.push(kingdom(1, 1));
    w.kingdoms.push(kingdom(2, 0));
    fog::set_mode(&mut w, FogMode::Full);
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    let mut velocity = Vec::new();
    for o in 0..24u32 {
        let k = o % 3;
        let centre = Vec2::new(40.0 + rng.f() * 430.0, 40.0 + rng.f() * 430.0);
        let mut members = Vec::new();
        for _ in 0..6 {
            let p = centre + Vec2::new(rng.f() * 24.0 - 12.0, rng.f() * 24.0 - 12.0);
            members.push(w.spawn_actor(ids.soldier, Some(k), p.extend(0.0), 0.0));
            let h = rng.f() * std::f32::consts::TAU;
            velocity.push(Vec2::new(h.cos(), h.sin()) * (1.0 + rng.f() * 6.0) * aa_play::DT);
        }
        let id = w.orgs.len() as u32;
        for &m in &members {
            w.actors[m as usize].org = Some(id);
        }
        w.orgs.push(Organization { id, def: ids.regiment, kingdom: k, members, hq: None, order: Order::Idle, anchor: centre, heading: 0.0, morale: 1.0, experience: 0.0, alive: true });
    }
    for k in 0..3 {
        w.spawn_actor(ids.corps, Some(k), Vec3::new(60.0 + 180.0 * k as f32, 60.0 + 150.0 * k as f32, 0.0), 0.3);
    }
    let soldiers = velocity.len();
    let mut checked = 0usize;
    for tick in 0..200 {
        for (i, v) in velocity.iter_mut().enumerate() {
            let a = &mut w.actors[i];
            let mut p = a.pos.truncate() + *v;
            if p.x < 8.0 || p.y < 8.0 || p.x > 504.0 || p.y > 504.0 {
                *v = -*v;
                p = p.clamp(Vec2::splat(8.0), Vec2::splat(504.0));
            }
            a.pos = p.extend(0.0);
        }
        if tick % 20 == 0 {
            for i in 0..soldiers {
                let t = (i * 7919 + tick) % soldiers;
                w.actors[i].combat.target = (i % 4 == 0 && w.actors[t].kingdom != w.actors[i].kingdom).then_some(t as u32);
            }
        }
        if tick == 120 {
            // deaths and a destruction
            w.actors[5].alive = false;
            w.actors[11].alive = false;
            w.actors[11].removed = true;
        }
        fog::tick(&mut w);
        let e = fog::engine(&w).unwrap();
        assert!(e.parts.stats.refusals.is_empty(), "{:?}", e.parts.stats.refusals);
        let n = e.map.tiles.len();
        let mut want_sets: Vec<BTreeSet<(u32, u16)>> = vec![BTreeSet::new(); n];
        let mut want_visible = vec![0u32; n];
        let mut want_explored = vec![0u32; n];
        let slots = e.parts.actors.iter().enumerate().map(|(i, s)| (zones::actor_key(i as u32), s)).chain(e.parts.orgs.iter().enumerate().map(|(o, s)| (zones::org_key(o as u32), s)));
        for (key, s) in slots {
            let owner = match key & zones::ORG {
                0 => w.actors[(key - 1) as usize].kingdom.unwrap(),
                _ => w.orgs[((key & !zones::ORG) - 1) as usize].kingdom,
            };
            let mut flags_by_tile: Vec<(usize, u16)> = Vec::new();
            for zt in [1u32, 6] {
                let Some(z) = s.zone(zt).filter(|z| z.active()) else { continue };
                for t in circle_tiles(&e.map, &z) {
                    flags_by_tile.push((t, z.flags));
                    want_visible[t] |= if zt == 6 { 1 << owner } else { s.comp.unwrap().reveal_mask };
                    if zt == 6 {
                        want_explored[t] |= 1 << owner;
                    }
                }
            }
            flags_by_tile.sort();
            let mut i = 0;
            while i < flags_by_tile.len() {
                let t = flags_by_tile[i].0;
                let mut f = 0;
                while i < flags_by_tile.len() && flags_by_tile[i].0 == t {
                    f |= flags_by_tile[i].1;
                    i += 1;
                }
                want_sets[t].insert((key, f));
            }
        }
        for t in 0..n {
            let got: BTreeSet<(u32, u16)> = e.map.pool.entries(e.map.tiles[t].set).unwrap().into_iter().map(|x| (x.actor, x.flags)).filter(|x| x.1 & 0x42 != 0).collect();
            assert_eq!(got, want_sets[t], "tick {tick} tile {t} zone set");
            assert_eq!(e.tile_fog(t) >> 16, want_visible[t], "tick {tick} tile {t} visible half");
            // a ZOD explores for its owner (sticky)
            assert_eq!(e.tile_fog(t) & want_explored[t], want_explored[t], "tick {tick} tile {t} explored");
            checked += got.len();
        }
    }
    assert!(checked > 50_000, "{checked}");
}

// ---------------------------------------------------------------------------
// Real data (ignored: needs AA_GAME_ROOT). Two computer players fight on
// Berlin and on a generated map; every second the aa-zones fog and supply
// zones are compared with the earlier f32 paths run on the same world, and
// every difference is attributed to a native rule.

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"E:\Games\Axis and Alies".into()))
}

fn real_data() -> Arc<GameData> {
    static DATA: std::sync::OnceLock<Arc<GameData>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| Arc::new(GameData::load(&root()).expect("GameData::load"))).clone()
}

fn two_computers(map: GameMap, map_id: &str, nations: [&str; 2], starts: [usize; 2]) -> World {
    use aa_play::{PlayerSetup, SkirmishSetup};
    let setup = SkirmishSetup {
        map_id: map_id.into(),
        players: vec![
            PlayerSetup { name: "A".into(), nation: nations[0].into(), team: 0, color: [0, 76, 255], controller: Controller::Computer(2), start: starts[0] },
            PlayerSetup { name: "B".into(), nation: nations[1].into(), team: 1, color: [217, 0, 25], controller: Controller::Computer(2), start: starts[1] },
        ],
        starting_resources: None,
        seed: 5,
    };
    let mut w = World::new_skirmish(real_data(), Arc::new(map), &setup).expect("new_skirmish");
    aa_play::ai::attach(&mut w, Arc::new(aa_play::ai::AiData::load(&root()).expect("AiData::load")));
    w
}

/// Why a tile's visible bit differs between the two paths.
#[derive(Debug, Default)]
struct Causes {
    compared: u64,
    equal: u64,
    /// legacy only, from an organization member's own zone (or a dead
    /// former member's): natively a member inside its organization's ZOC
    /// carries no zone (0x56EA30).
    member_zone: u64,
    /// from a building's ZOD while it unpacks: natively an actor under
    /// construction uses ActorData +0x1D0 (0x56F140), the f32 path +0x1CC.
    construction: u64,
    /// from an organization's zone: natively it stands on its leader
    /// (57C800), the f32 path used the members' centre.
    org_position: u64,
    /// from an RZOD (reveal masks: members' engagers only through byte
    /// +0x1C, 0x56EFC0).
    rzod: u64,
    /// not attributed to one of the rules above (printed with
    /// AA_ZONES_DEBUG=1).
    other: u64,
}

fn compare_fog(w: &World, legacy: &aa_play::fog_legacy::LegacyFog, causes: &mut Causes, actors: &mut [u64; 3]) {
    let e = fog::engine(w).expect("engine");
    let (width, height) = (w.map.width as i32, w.map.height as i32);
    let majors: Vec<u32> = w.kingdoms.iter().filter(|k| !k.independent).map(|k| k.id).collect();
    let structure = |a: usize| e.def_of(w, zones::Key::Actor(a as u32)).structure;
    for y in 0..height {
        for x in 0..width {
            let t = (y * width + x) as usize;
            let (n, l) = (e.tile_fog(t) >> 16, legacy.tile_word(t) >> 16);
            for &k in &majors {
                causes.compared += 1;
                let b = 1u32 << k;
                if n & b == l & b {
                    causes.equal += 1;
                    continue;
                }
                if l & b != 0 {
                    // legacy only: which legacy zone saw it
                    let zs: Vec<_> = legacy.zones_at(x, y).into_iter().filter(|z| z.3 & b != 0).collect();
                    // (a dead soldier left its organization; while alive it was a
                    // member without zones, the f32 path kept its own zone)
                    if zs.iter().any(|z| !z.0 && z.2 && (w.actors[z.1].org.is_some() || (!w.actors[z.1].alive && !e.parts.actors[z.1].zone(6).is_some_and(|z| z.active())))) {
                        causes.member_zone += 1;
                    } else if zs.iter().any(|z| !z.0 && structure(z.1) && z.2) {
                        causes.construction += 1;
                        if std::env::var_os("AA_ZONES_DEBUG").is_some() && causes.construction % 50_000 == 1 {
                            let z = zs.iter().find(|z| !z.0 && structure(z.1) && z.2).unwrap();
                            let a = &w.actors[z.1];
                            let s = &e.parts.actors[z.1];
                            eprintln!(
                                "t={:.0} legacy-only tile ({x},{y}) k{k}: actor {} {} at {:?} alive {} kingdom {:?}; native z6 {:?} z1 {:?} comp {:?} tile {:?}; def {:?}",
                                w.time, a.id, w.data.def(a.def).id, a.pos, a.alive, a.kingdom, s.zone(6), s.zone(1), s.comp, s.tile, e.def_of(w, zones::Key::Actor(a.id))
                            );
                        }
                    } else if zs.iter().any(|z| z.0 && z.2) {
                        causes.org_position += 1;
                    } else if zs.iter().any(|z| !z.2) {
                        causes.rzod += 1;
                    } else {
                        causes.other += 1;
                        if std::env::var_os("AA_ZONES_DEBUG").is_some() && causes.other % 40 == 1 {
                            let near: Vec<String> = zs.iter().map(|z| if z.0 { format!("org{} {:?}", z.1, e.parts.orgs[z.1].zone(6)) } else { format!("actor{} {} org {:?} alive {} pos {:?} z6 {:?}", z.1, w.data.def(w.actors[z.1].def).id, w.actors[z.1].org, w.actors[z.1].alive, w.actors[z.1].pos, e.parts.actors[z.1].zone(6)) }).collect();
                            eprintln!("t={:.0} other legacy-only ({x},{y}) k{k}: {near:?}", w.time);
                        }
                    }
                } else {
                    // native only: which native zone sees it
                    let entries = e.map.pool.entries(e.map.tiles[t].set).unwrap();
                    let owner = |key: u32| if key & zones::ORG != 0 { Some(w.orgs[((key & !zones::ORG) - 1) as usize].kingdom) } else { w.actors[(key - 1) as usize].kingdom };
                    let zod: Vec<u32> = entries.iter().filter(|x| x.flags & 0x40 != 0 && owner(x.actor) == Some(k)).map(|x| x.actor).collect();
                    if zod.iter().any(|&a| a & zones::ORG == 0 && structure((a - 1) as usize)) {
                        causes.construction += 1;
                        if std::env::var_os("AA_ZONES_DEBUG").is_some() && causes.construction % 50_000 == 2 {
                            let id = zod.iter().find(|&&a| a & zones::ORG == 0 && structure((a - 1) as usize)).unwrap() - 1;
                            let a = &w.actors[id as usize];
                            let s = &e.parts.actors[id as usize];
                            eprintln!(
                                "t={:.0} native-only tile ({x},{y}) k{k}: actor {} {} at {:?} alive {} kingdom {:?}; native z6 {:?} z1 {:?} comp {:?}; legacy zones here {:?}",
                                w.time, a.id, w.data.def(a.def).id, a.pos, a.alive, a.kingdom, s.zone(6), s.zone(1), s.comp, legacy.zones_at(x, y)
                            );
                        }
                    } else if zod.iter().any(|&a| a & zones::ORG != 0) {
                        causes.org_position += 1;
                    } else if zod.is_empty() {
                        causes.rzod += 1;
                    } else {
                        causes.other += 1;
                        if std::env::var_os("AA_ZONES_DEBUG").is_some() && causes.other % 40 == 2 {
                            let near: Vec<String> = zod.iter().map(|&a| { let i = (a - 1) as usize; format!("actor{} {} org {:?} alive {} pos {:?} z6 {:?}", i, w.data.def(w.actors[i].def).id, w.actors[i].org, w.actors[i].alive, w.actors[i].pos, e.parts.actors[i].zone(6)) }).collect();
                            eprintln!("t={:.0} other native-only ({x},{y}) k{k}: {near:?} legacy {:?}", w.time, legacy.zones_at(x, y));
                        }
                    }
                }
            }
        }
    }
    for a in w.actors.iter().filter(|a| a.alive && !a.removed) {
        for &k in &majors {
            let shared = e.kingdom(k).map_or(0, |x| x.shared_mask);
            let n = e.actor_word(a.id) & shared & 0xffff_0000 != 0;
            let l = legacy.actor_word(a.id) & legacy.shared_mask(k) & 0xffff_0000 != 0;
            actors[0] += 1;
            if n != l {
                // the same tiles look the same to both paths: a word
                // difference with equal tile words is an event rule
                let (tx, ty) = ((a.pos.x / w.map.cell_size).floor() as i32, (a.pos.y / w.map.cell_size).floor() as i32);
                let t = (ty.clamp(0, height - 1) * width + tx.clamp(0, width - 1)) as usize;
                if (e.tile_fog(t) ^ legacy.tile_word(t)) & 0xffff_0000 == 0 {
                    actors[2] += 1;
                } else {
                    actors[1] += 1;
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct SupplyCheck {
    zones: u64,
    equal_f32: u64,
    /// the aa-zones zone equals a fresh aa-zones flood, the f32 flood differs
    f32_flood: u64,
    /// the zone moved incrementally (0x515970 `moved`: at most one tile):
    /// it differs from a fresh flood at its place, natively
    incremental: u64,
    tiles: u64,
}

fn compare_supply(w: &World, check: &mut SupplyCheck) {
    let e = fog::engine(w).expect("engine");
    let g = e.map.geometry;
    let cell = w.map.cell_size;
    for (i, z) in w.books.zones.iter().enumerate() {
        let Some(zone) = e.parts.actors[z.provider as usize].zone(0).filter(|z| z.active()) else { continue };
        check.zones += 1;
        let got: Vec<u32> = w.books.zone_tiles.get(i).map(|v| v.to_vec()).unwrap_or_default();
        check.tiles += got.len() as u64;
        // the f32 flood from the zone's own tile
        let f32_tiles = aa_play::supply::flood_tiles(&w.map, zone.x, zone.y, f32::from_bits(zone.radius_bits));
        // a fresh aa-zones flood at the same tile and radius
        let mut scratch = ZoneMap::new(g).unwrap();
        let mut rw = RefWorld { flags: w.map.flags.clone(), owners: vec![0] };
        let mut fresh = Zone::new(1, 1, 1);
        let pos = [((zone.x as f32 + 0.5) * cell).to_bits(), ((zone.y as f32 + 0.5) * cell).to_bits()];
        zone::move_to_position(&mut scratch, &mut rw, &mut fresh, pos, zone.radius_bits, PC).unwrap();
        let fresh_tiles: Vec<u32> = (0..scratch.tiles.len() as u32).filter(|&t| scratch.tiles[t as usize].set.is_some()).collect();
        if got == f32_tiles {
            check.equal_f32 += 1;
        } else if got == fresh_tiles {
            check.f32_flood += 1;
        } else {
            check.incremental += 1;
        }
    }
}

fn run_comparison(name: &str, mut w: World, seconds: u32) {
    let mut legacy = aa_play::fog_legacy::LegacyFog::new(FogMode::Explored);
    let mut causes = Causes::default();
    let mut actors = [0u64; 3];
    let mut supply = SupplyCheck::default();
    let (mut fog_us, mut fog_max, mut legacy_us, mut supply_us, mut ticks) = (0.0f64, 0.0f32, 0.0f64, 0.0f64, 0u64);
    let mut peak_actors = 0;
    for tick in 0..seconds as u64 * aa_play::TICKS_PER_SECOND as u64 {
        w.step(Vec::new());
        let t = std::time::Instant::now();
        legacy.step(&w);
        legacy_us += t.elapsed().as_secs_f64() * 1e6;
        fog_us += f64::from(w.fog.stats.last_us);
        fog_max = fog_max.max(w.fog.stats.last_us);
        let s = &fog::engine(&w).unwrap().parts.stats;
        supply_us += f64::from(s.supply_us + s.observe_us);
        ticks += 1;
        peak_actors = peak_actors.max(w.actors.iter().filter(|a| a.alive && !a.removed).count());
        if tick % aa_play::TICKS_PER_SECOND as u64 == 0 {
            compare_fog(&w, &legacy, &mut causes, &mut actors);
            compare_supply(&w, &mut supply);
        }
    }
    let e = fog::engine(&w).unwrap();
    println!("{name}: {seconds} s, peak {peak_actors} live actors, {} organizations", w.orgs.len());
    println!("  tile visible bits: {causes:?}");
    let explained = causes.member_zone + causes.construction + causes.org_position + causes.rzod;
    println!(
        "  differing {} of {} ({:.3}%), explained by native rules {explained}, other {}",
        causes.compared - causes.equal,
        causes.compared,
        100.0 * (causes.compared - causes.equal) as f64 / causes.compared.max(1) as f64,
        causes.other
    );
    println!("  actor visibility: compared {}, differ where tiles differ {}, differ on equal tiles {}", actors[0], actors[1], actors[2]);
    println!("  supply zones: {supply:?}");
    println!(
        "  cost per tick: fog (aa-zones) {:.0} us mean, {:.0} us max; supply zones {:.0} us; legacy f32 fog {:.0} us",
        fog_us / ticks as f64,
        fog_max,
        supply_us / ticks as f64,
        legacy_us / ticks as f64
    );
    println!("  engine: {:?}", e.parts.stats);
    assert!(e.parts.stats.refusals.is_empty(), "{:?}", e.parts.stats.refusals);
}

#[test]
#[ignore]
fn berlin_fog_and_supply_against_the_f32_paths() {
    let data = real_data();
    let map = GameMap::load_with(&root(), "(2v2)Berlin", &data).expect("Berlin");
    let seconds = std::env::var("AA_ZONES_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(240);
    run_comparison("Berlin", two_computers(map, "(2v2)Berlin", ["rus", "ger"], [0, 2]), seconds);
}

#[test]
#[ignore]
fn generated_map_fog_and_supply_against_the_f32_paths() {
    use aa_play::mapgen::{RandomMapData, RandomMapParams};
    let rmg = RandomMapData::load(&root()).expect("random map data");
    let info = rmg.catalog().rmcs.into_iter().find(|r| r.ids == "rmc_southerneurope").expect("rmc");
    let generated = rmg.generate(&RandomMapParams::defaults(&info, 2, 3)).expect("generate");
    let seconds = std::env::var("AA_ZONES_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(240);
    run_comparison("generated rmc_southerneurope", two_computers(generated.map, "generated", ["usa", "rus"], [0, 1]), seconds);
}