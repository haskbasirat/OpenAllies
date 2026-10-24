//! Movement owner tests: pathfinding, regiment formation movement, speeds,
//! terrain limits, orders and the 150-regiment performance budget.
//! Synthetic data uses original values: rus_infantry speed_max 2.5,
//! rus_tank_medium 3.5 / acceleration 1 / turn_radius 0.9 IN_PLACE
//! (Data/Units/RUS, Templates/template_unit_aa.tgi), the regiment_infantry
//! layout with its regiment_defensive positions
//! (Data/Organizations/Layouts/regiment_infantry.tgi).
use aa_play::data::{Def, DefClass, Formation, GameData, Weapon};
use aa_play::formation::{rotate_offset, slot_position};
use aa_play::map::{BLOCKED, GameMap, NOT_LAND_PASSABLE, NOT_WATER_PASSABLE};
use aa_play::path::{Grid, MoverClass, PathRules, Pathfinder, Request, Status};
use aa_play::{ActorId, Command, Controller, Kingdom, Order, OrgId, Organization, Unit, World, movement};
use glam::Vec2;
use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

/// regiment_infantry slots in expanded order with their regiment_defensive
/// positions: front x3 (locked), front, front, front, support x2, captain.
const INFANTRY_POSITIONS: [[f32; 2]; 9] = [[-3.0, 2.5], [0.0, 3.25], [2.75, 2.75], [0.25, -3.0], [-3.25, -2.75], [3.25, -2.75], [2.25, 0.25], [-2.5, -0.25], [0.0, 0.0]];
const INFANTRY_ENTRIES: [u32; 9] = [0, 0, 0, 1, 2, 3, 4, 4, 5];

struct Ids {
    infantry: u32,
    slow: u32,
    tank: u32,
    truck: u32,
    regiment: u32,
    slow_regiment: u32,
    tank_regiment: u32,
}

fn member(id: &str, speed: f32, acceleration: f32, turn_type: u32, turn_radius: f32, radius: f32, range: f32) -> Def {
    Def {
        id: id.into(),
        class: Some(DefClass::Member),
        health: 200.0,
        speed_max: speed,
        acceleration,
        turn_type,
        turn_radius,
        blocking_shape: Some("CIRCLE".into()),
        blocking_size: Some([radius, radius]),
        weapons: vec![Weapon { name: "rifle".into(), range, ..Default::default() }],
        ..Default::default()
    }
}
fn regiment(id: &str, members: [u32; 9]) -> Def {
    Def {
        id: id.into(),
        class: Some(DefClass::Organization),
        slots: members.iter().zip(INFANTRY_POSITIONS).map(|(&m, p)| (m, p)).collect(),
        slot_entries: INFANTRY_ENTRIES.to_vec(),
        leader_slot: Some(8),
        formations: vec![Formation { id: "regiment_defensive".into(), positions: INFANTRY_POSITIONS.to_vec() }],
        default_formation: Some(0),
        // Templates/template_organization_aa.tgi
        control_range: 18.0,
        ..Default::default()
    }
}
fn data() -> (GameData, Ids) {
    let mut data = GameData::default();
    let mut add = |d: Def| {
        data.by_id.insert(d.id.clone(), data.defs.len() as u32);
        data.defs.push(d);
        data.defs.len() as u32 - 1
    };
    let infantry = add(member("rus_infantry", 2.5, 0.0, 0, 0.0, 0.35, 30.0));
    let slow = add(member("slow_infantry", 1.5, 0.0, 0, 0.0, 0.35, 30.0));
    let tank = add(member("rus_tank_medium", 3.5, 1.0, 1, 0.9, 0.8, 40.0));
    // rus_truck_*: speed_max 3, no acceleration, AAUnitVehicleHQTruck
    // turn_radius 3 WHILE_MOVING, blocking_radius 0.65.
    let mut truck_def = member("rus_truck", 3.0, 0.0, 2, 3.0, 0.65, 0.0);
    truck_def.class = Some(DefClass::Truck);
    truck_def.weapons.clear();
    let truck = add(truck_def);
    let regiment_id = add(regiment("regiment_rus_infantry", [infantry; 9]));
    let mut slow_members = [infantry; 9];
    slow_members[4] = slow;
    let slow_regiment = add(regiment("regiment_slow", slow_members));
    let tank_regiment = add(regiment("regiment_tanks", [tank; 9]));
    (data, Ids { infantry, slow, tank, truck, regiment: regiment_id, slow_regiment, tank_regiment })
}

fn open_map(width: u32, height: u32, cell: f32) -> GameMap {
    let mut flags = vec![0; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                flags[(y * width + x) as usize] = BLOCKED;
            }
        }
    }
    GameMap { id: "test".into(), width, height, cell_size: cell, flags, speed_limit: vec![f32::MAX; (width * height) as usize], ..Default::default() }
}
fn set(map: &mut GameMap, x: u32, y: u32, flag: u32) {
    let w = map.width;
    map.flags[(y * w + x) as usize] |= flag;
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
fn world(map: GameMap, data: GameData) -> World {
    let mut w = World::empty(Arc::new(data), Arc::new(map), 7);
    w.kingdoms.push(kingdom(0, 0));
    w.kingdoms.push(kingdom(1, 1));
    w
}
fn spawn_regiment(w: &mut World, def: u32, kingdom: u32, center: Vec2, heading: f32) -> OrgId {
    let id = w.orgs.len() as OrgId;
    let slots = w.data.def(def).slots.clone();
    let mut members = Vec::new();
    for (slot, (member_def, offset)) in slots.into_iter().enumerate() {
        let p = w.ground(slot_position(center, heading, offset));
        let a = w.spawn_actor(member_def, Some(kingdom), p, heading);
        w.actors[a as usize].org = Some(id);
        w.actors[a as usize].slot = Some(slot as u16);
        members.push(a);
    }
    w.orgs.push(Organization { id, def, kingdom, members, hq: None, order: Order::Idle, anchor: center, heading, morale: 1.0, experience: 0.0, alive: true });
    id
}
fn advance(w: &mut World, ticks: u32) {
    for _ in 0..ticks {
        movement::tick(w);
        w.tick += 1;
        w.time = w.tick as f32 * aa_play::DT;
    }
}
fn order(w: &mut World, c: Command) {
    movement::command(w, &c).expect("command accepted");
}
fn pos(w: &World, a: ActorId) -> Vec2 {
    w.actors[a as usize].pos.truncate()
}
fn org_settled(w: &World, o: OrgId) -> bool {
    w.orgs[o as usize].order == Order::Idle && w.orgs[o as usize].members.iter().all(|&m| !movement::actor_moving(w, m))
}
fn run_until(w: &mut World, seconds: f32, mut done: impl FnMut(&World) -> bool) -> bool {
    let ticks = (seconds * aa_play::TICKS_PER_SECOND as f32) as u32;
    for _ in 0..ticks {
        advance(w, 1);
        if done(w) {
            return true;
        }
    }
    false
}
/// Max distance between the members and the nearest distinct slot position.
fn formation_error(w: &World, o: OrgId) -> f32 {
    let org = &w.orgs[o as usize];
    let facing = Vec2::new(org.heading.cos(), org.heading.sin());
    let mut slots: Vec<Vec2> = INFANTRY_POSITIONS.iter().map(|&p| org.anchor + rotate_offset(Vec2::from(p), facing)).collect();
    let mut worst = 0.0f32;
    for &m in &org.members {
        let p = pos(w, m);
        let (i, d) = slots.iter().enumerate().map(|(i, s)| (i, (*s - p).length())).min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
        worst = worst.max(d);
        slots.remove(i);
    }
    worst
}

// ---------------------------------------------------------------------------
// Pathfinding.

fn solve(map: &GameMap, from: Vec2, to: Vec2, class: MoverClass) -> aa_play::path::Route {
    Pathfinder::default().solve(map, &PathRules::default(), &Request { from, to, class, max_speed: 2.5 })
}
fn legs_clear(map: &GameMap, from: Vec2, route: &aa_play::path::Route, class: MoverClass) -> bool {
    let grid = Grid::new(map, class);
    let mut previous = from;
    route.waypoints.iter().all(|w| {
        let ok = grid.line_clear(previous, w.pos);
        previous = w.pos;
        ok
    })
}

#[test]
fn path_goes_around_a_wall() {
    let mut map = open_map(48, 48, 4.0);
    for y in 1..40 {
        set(&mut map, 24, y, BLOCKED);
    }
    let (from, to) = (Vec2::new(40.0, 80.0), Vec2::new(152.0, 80.0));
    let route = solve(&map, from, to, MoverClass::Land);
    assert_eq!(route.status, Status::Reached);
    assert_eq!(route.waypoints.last().unwrap().pos, to);
    assert!(legs_clear(&map, from, &route, MoverClass::Land), "{:?}", route.waypoints);
    assert!(route.waypoints.iter().any(|w| w.pos.y >= 160.0), "must pass the wall end: {:?}", route.waypoints);
    // Smoothing leaves few nodes (around the wall end and to the goal).
    assert!(route.waypoints.len() <= 4, "{:?}", route.waypoints);
    for w in &route.waypoints {
        assert!((w.dir.length() - 1.0).abs() < 1e-4);
    }
}

#[test]
fn unreachable_goal_gives_partial_route_to_closest_cell() {
    let mut map = open_map(48, 48, 4.0);
    for y in 1..47 {
        set(&mut map, 24, y, BLOCKED);
    }
    let from = Vec2::new(40.0, 80.0);
    let route = solve(&map, from, Vec2::new(152.0, 80.0), MoverClass::Land);
    assert_eq!(route.status, Status::Partial);
    let end = route.waypoints.last().unwrap().pos;
    // The reachable cell closest to the goal is beside the wall on the goal's row.
    assert_eq!(end, Vec2::new(23.0 * 4.0 + 2.0, 20.0 * 4.0 + 2.0));
    assert!(legs_clear(&map, from, &route, MoverClass::Land));
}

#[test]
fn water_blocks_land_units_and_carries_boats() {
    let mut map = open_map(64, 64, 4.0);
    // A river across the map with one ford.
    for y in 1..63 {
        for x in 28..34 {
            if !(40..44).contains(&y) {
                set(&mut map, x, y, NOT_LAND_PASSABLE);
            }
        }
    }
    // Land elsewhere is not water passable.
    for y in 1..63 {
        for x in 1..63 {
            if !(28..34).contains(&x) {
                set(&mut map, x, y, NOT_WATER_PASSABLE);
            }
        }
    }
    let from = Vec2::new(40.0, 40.0);
    let to = Vec2::new(200.0, 40.0);
    let land = solve(&map, from, to, MoverClass::Land);
    assert_eq!(land.status, Status::Reached);
    assert!(legs_clear(&map, from, &land, MoverClass::Land));
    assert!(land.waypoints.iter().any(|w| (160.0..=176.0).contains(&w.pos.y)), "land route uses the ford: {:?}", land.waypoints);
    // A boat goes along the river but cannot leave it.
    let boat = solve(&map, Vec2::new(122.0, 20.0), Vec2::new(122.0, 230.0), MoverClass::Water);
    assert_eq!(boat.status, Status::Reached, "{:?}", boat);
    // A goal ashore is walked back onto the water (0x53D340).
    let ashore = solve(&map, Vec2::new(122.0, 20.0), Vec2::new(40.0, 20.0), MoverClass::Water);
    let grid = Grid::new(&map, MoverClass::Water);
    assert!(!grid.blocked_at(ashore.goal) && ashore.goal.x >= 112.0, "{:?}", ashore.goal);
    assert!(legs_clear(&map, Vec2::new(122.0, 20.0), &ashore, MoverClass::Water));
}

#[test]
fn depth_cap_cuts_long_routes_and_movers_continue() {
    let map = open_map(320, 12, 2.0);
    let from = Vec2::new(5.0, 12.0);
    let to = Vec2::new(633.0, 12.0);
    let route = solve(&map, from, to, MoverClass::Land);
    assert_eq!(route.status, Status::Capped, "{:?}", route.status);
    assert!(route.waypoints.last().unwrap().pos.x < to.x);
    // A truck still drives all the way by continuing from each cut end.
    let (data, ids) = data();
    let mut w = world(map, data);
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(from), 0.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(truck)], to, attack_move: false });
    let arrived = run_until(&mut w, 400.0, |w| w.actors[truck as usize].order == Order::Idle);
    assert!(arrived, "truck at {:?}", pos(&w, truck));
    assert!((pos(&w, truck) - to).length() < 0.6, "truck at {:?}", pos(&w, truck));
    assert!(!movement::actor_moving(&w, truck));
}

// ---------------------------------------------------------------------------
// Regiments.

#[test]
fn regiment_keeps_formation_and_faces_the_move_direction() {
    let (data, ids) = data();
    let mut w = world(open_map(128, 128, 4.0), data);
    let o = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(100.0, 100.0), 0.0);
    let to = Vec2::new(100.0, 300.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Org(o)], to, attack_move: false });
    // While marching the members stay near their slots.
    advance(&mut w, 16 * 20);
    assert!(w.orgs[o as usize].members.iter().all(|&m| w.actors[m as usize].activity == aa_play::Activity::Moving));
    assert!((w.orgs[o as usize].heading - FRAC_PI_2).abs() < 1e-3, "heading {}", w.orgs[o as usize].heading);
    let marching = formation_error(&w, o);
    assert!(marching < 2.0, "formation error while marching {marching}");
    assert!(run_until(&mut w, 120.0, |w| org_settled(w, o)), "regiment did not settle");
    let org = &w.orgs[o as usize];
    assert!((org.anchor - to).length() < 0.6, "anchor {:?}", org.anchor);
    assert!((org.heading - FRAC_PI_2).abs() < 1e-3);
    let error = formation_error(&w, o);
    assert!(error < 0.6, "formation error on arrival {error}");
    for &m in &org.members {
        let a = &w.actors[m as usize];
        assert!((a.facing - FRAC_PI_2).abs() < 1e-3, "member facing {}", a.facing);
        assert_eq!(a.activity, aa_play::Activity::Idle);
        assert!(a.motion.path.is_empty());
    }
}

#[test]
fn regiment_turns_its_formation_at_corners() {
    let (data, ids) = data();
    let mut map = open_map(128, 128, 4.0);
    // An L: a wall forces the route east first, then north.
    for x in 1..60 {
        set(&mut map, x, 40, BLOCKED);
    }
    let mut w = world(map, data);
    let o = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(100.0, 120.0), 0.0);
    let to = Vec2::new(100.0, 220.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Org(o)], to, attack_move: false });
    assert!(run_until(&mut w, 200.0, |w| org_settled(w, o)), "regiment did not settle");
    let org = &w.orgs[o as usize];
    assert!((org.anchor - to).length() < 0.6, "anchor {:?}", org.anchor);
    let error = formation_error(&w, o);
    assert!(error < 0.6, "formation error after corners {error}");
    // Nobody stands inside the wall.
    let grid = Grid::new(&w.map, MoverClass::Land);
    for &m in &org.members {
        assert!(!grid.blocked_at(pos(&w, m)));
    }
}

#[test]
fn slowest_member_sets_the_regiment_speed() {
    let (data, ids) = data();
    let mut w = world(open_map(160, 160, 4.0), data);
    let fast = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(100.0, 100.0), FRAC_PI_2);
    let slow = spawn_regiment(&mut w, ids.slow_regiment, 0, Vec2::new(200.0, 100.0), FRAC_PI_2);
    let tanks = spawn_regiment(&mut w, ids.tank_regiment, 0, Vec2::new(300.0, 100.0), FRAC_PI_2);
    for (o, x) in [(fast, 100.0), (slow, 200.0), (tanks, 300.0)] {
        order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Org(o)], to: Vec2::new(x, 500.0), attack_move: false });
    }
    advance(&mut w, 4);
    let start: Vec<f32> = [fast, slow, tanks].iter().map(|&o| w.orgs[o as usize].anchor.y).collect();
    advance(&mut w, 16 * 20);
    let speed: Vec<f32> = [fast, slow, tanks].iter().zip(&start).map(|(&o, s)| (w.orgs[o as usize].anchor.y - s) / 20.0).collect();
    eprintln!("regiment speeds m/s: infantry {:.3} slowed {:.3} tanks {:.3}", speed[0], speed[1], speed[2]);
    assert!((speed[0] - 2.5).abs() < 0.05, "infantry {}", speed[0]);
    assert!((speed[1] - 1.5).abs() < 0.05, "slow member caps the regiment: {}", speed[1]);
    // Tanks accelerate at 1 m/s^2 to 3.5 m/s.
    assert!(speed[2] > 3.2 && speed[2] < 3.5, "tanks {}", speed[2]);
    let _ = (ids.slow, ids.infantry, ids.tank);
}

#[test]
fn terrain_speed_limit_caps_movers() {
    let (data, ids) = data();
    let mut map = open_map(64, 64, 4.0);
    for y in 1..63 {
        for x in 1..63 {
            map.speed_limit[(y * 64 + x) as usize] = if x < 32 { 1.0 } else { 0.2 };
        }
    }
    let mut w = world(map, data);
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(20.0, 120.0)), 0.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(truck)], to: Vec2::new(240.0, 120.0), attack_move: false });
    advance(&mut w, 16 * 5);
    let speed = w.actors[truck as usize].motion.speed;
    assert!((speed - 1.0).abs() < 1e-4, "limited to 1 m/s: {speed}");
    run_until(&mut w, 200.0, |w| pos(w, truck).x > 140.0);
    advance(&mut w, 16);
    let speed = w.actors[truck as usize].motion.speed;
    // Below TerrainMinimumMoveSpeed (0.5) the limit is raised to it.
    assert!((speed - 0.5).abs() < 1e-4, "minimum move speed: {speed}");
}

#[test]
fn attack_moves_into_weapon_range_and_holds() {
    let (data, ids) = data();
    let mut w = world(open_map(128, 128, 4.0), data);
    let o = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(100.0, 100.0), 0.0);
    let enemy = w.spawn_actor(ids.tank, Some(1), w.ground(Vec2::new(250.0, 100.0)), PI);
    order(&mut w, Command::Attack { kingdom: 0, units: vec![Unit::Org(o)], target: enemy });
    assert!(run_until(&mut w, 120.0, |w| w.orgs[o as usize].members.iter().all(|&m| !movement::actor_moving(w, m)) && w.time > 5.0));
    let d = (w.orgs[o as usize].anchor - pos(&w, enemy)).length();
    assert!(d <= 30.5 && d > 25.0, "holds at rifle range: {d}");
    assert_eq!(w.orgs[o as usize].order, Order::Attack { target: enemy });
    assert!(w.orgs[o as usize].members.iter().all(|&m| w.actors[m as usize].combat.target == Some(enemy)));
    // The target drives away: the regiment follows.
    let p = w.ground(Vec2::new(350.0, 100.0));
    w.actors[enemy as usize].pos = p;
    advance(&mut w, 16 * 3);
    assert!(w.orgs[o as usize].members.iter().any(|&m| movement::actor_moving(&w, m)));
}

#[test]
fn stop_halts_a_regiment() {
    let (data, ids) = data();
    let mut w = world(open_map(128, 128, 4.0), data);
    let o = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(100.0, 100.0), 0.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Org(o)], to: Vec2::new(400.0, 100.0), attack_move: false });
    advance(&mut w, 16 * 4);
    order(&mut w, Command::Stop { kingdom: 0, units: vec![Unit::Org(o)] });
    let before: Vec<Vec2> = w.orgs[o as usize].members.iter().map(|&m| pos(&w, m)).collect();
    advance(&mut w, 16);
    let after: Vec<Vec2> = w.orgs[o as usize].members.iter().map(|&m| pos(&w, m)).collect();
    assert_eq!(before, after);
    assert_eq!(w.orgs[o as usize].order, Order::Idle);
    // Commands for another kingdom's units are refused.
    assert!(movement::command(&mut w, &Command::Stop { kingdom: 1, units: vec![Unit::Org(o)] }).is_err());
}

#[test]
fn deploy_order_drives_there_without_pivoting() {
    let (data, ids) = data();
    let mut w = world(open_map(64, 64, 4.0), data);
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(40.0, 40.0)), 0.0);
    let at = Vec2::new(160.0, 120.0);
    w.actors[truck as usize].order = Order::Deploy { at, facing: -FRAC_PI_2 };
    assert!(run_until(&mut w, 120.0, |w| movement::deploy_ready(w, truck)), "truck at {:?} facing {}", pos(&w, truck), w.actors[truck as usize].facing);
    assert!((pos(&w, truck) - at).length() < 0.5);
    // The truck keeps the facing of its last leg; the building takes the
    // commanded facing.
    let leg = (at - Vec2::new(40.0, 40.0)).normalize();
    assert!((w.actors[truck as usize].facing - leg.y.atan2(leg.x)).abs() < 0.05, "facing {}", w.actors[truck as usize].facing);
    let f = w.actors[truck as usize].facing;
    advance(&mut w, 32);
    assert_eq!(w.actors[truck as usize].facing, f);
}

#[test]
fn truck_turns_on_an_arc_of_its_turn_radius() {
    let (data, ids) = data();
    let mut w = world(open_map(64, 64, 4.0), data);
    // Facing east, ordered due north: it drives on while turning at
    // speed / turn radius = 1 rad/s, so it curves on a 3 m radius.
    let start = Vec2::new(100.0, 100.0);
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(start), 0.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(truck)], to: Vec2::new(100.0, 160.0), attack_move: false });
    let mut previous = w.actors[truck as usize].facing;
    let mut max_rate = 0.0f32;
    let mut speeds = Vec::new();
    for _ in 0..16 * 2 {
        advance(&mut w, 1);
        let f = w.actors[truck as usize].facing;
        max_rate = max_rate.max((f - previous).abs() * aa_play::TICKS_PER_SECOND as f32);
        previous = f;
        speeds.push(w.actors[truck as usize].motion.speed);
    }
    assert!(max_rate <= 1.0 + 1e-3 && max_rate > 0.9, "turn rate {max_rate} rad/s");
    assert!(speeds.iter().skip(1).all(|&v| (v - 3.0).abs() < 1e-3), "constant 3 m/s while turning (no acceleration): {speeds:?}");
    // A quarter turn on a 3 m radius moves it about 3 m east before heading north.
    let p = pos(&w, truck);
    assert!(p.x > 102.0 && p.x < 104.0, "{p:?}");
    assert!(run_until(&mut w, 60.0, |w| w.actors[truck as usize].order == Order::Idle));
}

#[test]
fn vehicles_pitch_and_roll_with_the_ground_infantry_stay_upright() {
    let (mut data, ids) = data();
    // Flags as the original templates give them: the vehicle template sets
    // conform_to_terrain, infantry do not.
    data.defs[ids.truck as usize].conform_to_terrain = true;
    let mut map = open_map(32, 32, 4.0);
    // Ground rising 0.25 m per metre toward +X.
    map.heights = (0..33 * 33).map(|i| (i % 33) as f32 * 4.0 * 0.25).collect();
    let mut w = world(map, data);
    let east = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(60.0, 60.0)), 0.0);
    let north = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(60.0, 60.0)), FRAC_PI_2);
    let man = w.spawn_actor(ids.infantry, Some(0), w.ground(Vec2::new(60.0, 60.0)), 0.0);
    let slope = glam::Vec3::new(-0.25, 0.0, 1.0).normalize();
    // Driving uphill: pitched nose up.
    let o = movement::orientation(&w, east).unwrap();
    assert!((o.up - slope).length() < 1e-3, "{:?}", o.up);
    assert!((o.forward - glam::Vec3::new(1.0, 0.0, 0.25).normalize()).length() < 1e-3, "{:?}", o.forward);
    assert!((o.position.z - w.actors[east as usize].pos.z).abs() < 1e-3);
    // Across the slope: rolled, nose level.
    let o = movement::orientation(&w, north).unwrap();
    assert!((o.up - slope).length() < 1e-3, "{:?}", o.up);
    assert!(o.forward.z.abs() < 1e-3 && (o.forward.y - 1.0).abs() < 1e-3, "{:?}", o.forward);
    // Infantry stand upright.
    let o = movement::orientation(&w, man).unwrap();
    assert_eq!(o.up, glam::Vec3::Z);
    assert_eq!(o.position, w.actors[man as usize].pos);
}

#[test]
fn trucks_queue_behind_a_truck_going_the_same_way() {
    let (mut data, ids) = data();
    // A slightly slower leader (2.9 m/s): the follower (3 m/s) is not faster
    // by more than 0.25 m/s, so it waits behind instead of overtaking.
    let mut slow = data.defs[ids.truck as usize].clone();
    slow.id = "slow_truck".into();
    slow.speed_max = 2.9;
    data.by_id.insert(slow.id.clone(), data.defs.len() as u32);
    data.defs.push(slow);
    let slow_def = data.defs.len() as u32 - 1;
    let mut w = world(open_map(160, 32, 2.0), data);
    let lead = w.spawn_actor(slow_def, Some(0), w.ground(Vec2::new(30.0, 32.0)), 0.0);
    let follower = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(27.0, 32.0)), 0.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(lead)], to: Vec2::new(300.0, 32.0), attack_move: false });
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(follower)], to: Vec2::new(296.0, 32.0), attack_move: false });
    let mut closest = f32::MAX;
    let mut passed = false;
    run_until(&mut w, 120.0, |w| {
        closest = closest.min((pos(w, lead) - pos(w, follower)).length());
        passed |= pos(w, follower).x > pos(w, lead).x;
        w.actors[follower as usize].order == Order::Idle && w.actors[lead as usize].order == Order::Idle
    });
    assert!(!passed, "the follower overtook");
    assert!(closest >= 1.2, "kept its distance: {closest}");
    assert!((pos(&w, follower) - Vec2::new(296.0, 32.0)).length() < 0.6);
}

#[test]
fn truck_facing_a_building_wall_turns_away_and_drives_on() {
    let (mut data, ids) = data();
    let hq = Def { id: "rus_corps".into(), class: Some(DefClass::Building), health: 1000.0, blocking_shape: Some("SQUARE".into()), blocking_size: Some([20.0, 20.0]), ..Default::default() };
    data.by_id.insert(hq.id.clone(), data.defs.len() as u32);
    data.defs.push(hq);
    let hq_def = data.defs.len() as u32 - 1;
    let mut w = world(open_map(64, 64, 2.0), data);
    w.spawn_actor(hq_def, Some(0), w.ground(Vec2::new(60.0, 60.0)), 0.0);
    // Just north of the footprint's top edge (y = 70), nose to the south.
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(61.0, 70.6)), -FRAC_PI_2);
    for to in [Vec2::new(73.0, 70.6), Vec2::new(45.0, 70.6), Vec2::new(45.0, 45.0)] {
        order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(truck)], to, attack_move: false });
        assert!(run_until(&mut w, 60.0, |w| w.actors[truck as usize].order == Order::Idle), "stuck at {:?}", pos(&w, truck));
        assert!((pos(&w, truck) - to).length() < 0.6, "stopped at {:?} for {to:?}", pos(&w, truck));
    }
}

#[test]
fn deploy_to_a_blocked_spot_is_refused() {
    let (data, ids) = data();
    let mut map = open_map(64, 64, 4.0);
    for y in 1..63 {
        for x in 40..50 {
            set(&mut map, x, y, NOT_LAND_PASSABLE);
        }
    }
    let mut w = world(map, data);
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(40.0, 40.0)), 0.0);
    let water = Vec2::new(180.0, 40.0);
    assert!(movement::deploy_spot_problem(&w, truck, water).is_some());
    assert!(movement::deploy_spot_problem(&w, truck, Vec2::new(60.0, 40.0)).is_none());
    w.actors[truck as usize].order = Order::Deploy { at: water, facing: 0.0 };
    advance(&mut w, 1);
    assert_eq!(w.actors[truck as usize].order, Order::Idle);
    assert!(w.events.iter().any(|e| matches!(e, aa_play::Event::Refused { kingdom: 0, reason } if reason.contains("blocked"))), "{:?}", w.events);
    // A land spot across a river that spans the map: driven toward, then
    // refused once the move fails (3 tries, CAIMoveMinSecondsBeforeFail).
    let behind = Vec2::new(220.0, 40.0);
    assert!(movement::deploy_spot_problem(&w, truck, behind).is_none());
    w.events.clear();
    w.actors[truck as usize].order = Order::Deploy { at: behind, facing: 0.0 };
    assert!(run_until(&mut w, 60.0, |w| w.actors[truck as usize].order == Order::Idle));
    assert!(w.events.iter().any(|e| matches!(e, aa_play::Event::Refused { reason, .. } if reason.contains("cannot be reached"))), "{:?}", w.events);
}

#[test]
fn group_move_spreads_regiments_in_the_group_formation() {
    let (data, ids) = data();
    let mut w = world(open_map(128, 128, 4.0), data);
    let a = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(90.0, 100.0), FRAC_PI_2);
    let b = spawn_regiment(&mut w, ids.slow_regiment, 0, Vec2::new(110.0, 100.0), FRAC_PI_2);
    let to = Vec2::new(100.0, 300.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Org(a), Unit::Org(b)], to, attack_move: false });
    advance(&mut w, 8);
    let start = [w.orgs[a as usize].anchor.y, w.orgs[b as usize].anchor.y];
    advance(&mut w, 16 * 10);
    // Both move at the slower regiment's speed.
    let va = (w.orgs[a as usize].anchor.y - start[0]) / 10.0;
    let vb = (w.orgs[b as usize].anchor.y - start[1]) / 10.0;
    assert!((va - 1.5).abs() < 0.06 && (vb - 1.5).abs() < 0.06, "group speeds {va} {vb}");
    assert!(run_until(&mut w, 300.0, |w| org_settled(w, a) && org_settled(w, b)));
    // group_defensive: (6.5,10) and (-6.5,10) turned to face north.
    let pa = w.orgs[a as usize].anchor;
    let pb = w.orgs[b as usize].anchor;
    let expected = [to + Vec2::new(-6.5, 10.0), to + Vec2::new(6.5, 10.0)];
    assert!((pa - expected[0]).length() < 0.6 && (pb - expected[1]).length() < 0.6, "anchors {pa:?} {pb:?}");
}

#[test]
fn moving_truck_goes_around_a_parked_tank() {
    let (data, ids) = data();
    let mut w = world(open_map(64, 64, 4.0), data);
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(40.0, 100.0)), 0.0);
    let tank = w.spawn_actor(ids.tank, Some(1), w.ground(Vec2::new(100.0, 100.0)), 0.0);
    let to = Vec2::new(200.0, 100.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(truck)], to, attack_move: false });
    let mut closest = f32::MAX;
    let arrived = run_until(&mut w, 120.0, |w| {
        closest = closest.min((pos(w, truck) - pos(w, tank)).length());
        (pos(w, truck) - to).length() < 0.6 && !movement::actor_moving(w, truck)
    });
    assert!(arrived, "truck at {:?}", pos(&w, truck));
    eprintln!("closest approach {closest:.3} m (radii 0.6 + 0.8)");
    assert!(closest > 0.7, "went through the tank: {closest}");
}

#[test]
fn members_pursue_their_own_targets_while_the_regiment_holds() {
    let (data, ids) = data();
    let mut w = world(open_map(128, 128, 4.0), data);
    let o = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(100.0, 100.0), 0.0);
    let enemy = w.spawn_actor(ids.tank, Some(1), w.ground(Vec2::new(200.0, 100.0)), PI);
    let far = w.spawn_actor(ids.tank, Some(1), w.ground(Vec2::new(200.0, 160.0)), PI);
    order(&mut w, Command::Attack { kingdom: 0, units: vec![Unit::Org(o)], target: enemy });
    assert!(run_until(&mut w, 60.0, |w| (w.orgs[o as usize].anchor - Vec2::new(200.0, 100.0)).length() <= 30.5));
    // Combat gives one member another target, out of its rifle range.
    let m = w.orgs[o as usize].members[0];
    w.actors[m as usize].combat.target = Some(far);
    assert!(run_until(&mut w, 60.0, |w| (pos(w, m) - pos(w, far)).length() <= 30.0 && !movement::actor_moving(w, m)), "member at {:?}", pos(&w, m));
    let d = (pos(&w, m) - pos(&w, far)).length();
    assert!(d > 28.0, "stops at weapon range: {d}");
}

#[test]
fn routing_regiment_keeps_its_rout_order() {
    let (data, ids) = data();
    let mut w = world(open_map(128, 128, 4.0), data);
    let o = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(100.0, 100.0), 0.0);
    let rout = Vec2::new(100.0, 400.0);
    w.combat.orgs.resize_with(1, Default::default);
    w.combat.orgs[0].routing = Some(aa_play::combat::Rout { since: 0.0, destination: Some(rout), next_think: 0.0 });
    w.orgs[o as usize].order = Order::Move { to: rout, attack_move: false };
    advance(&mut w, 16);
    // The player cannot order it, and another order is replaced again.
    assert!(movement::command(&mut w, &Command::Move { kingdom: 0, units: vec![Unit::Org(o)], to: Vec2::new(400.0, 100.0), attack_move: false }).is_err());
    w.orgs[o as usize].order = Order::Move { to: Vec2::new(400.0, 100.0), attack_move: false };
    advance(&mut w, 1);
    assert_eq!(w.orgs[o as usize].order, Order::Move { to: rout, attack_move: false });
    advance(&mut w, 16 * 5);
    assert!(w.orgs[o as usize].anchor.y > 105.0 && (w.orgs[o as usize].anchor.x - 100.0).abs() < 1.0, "{:?}", w.orgs[o as usize].anchor);
}

#[test]
fn building_footprints_block_cells() {
    let (mut data, ids) = data();
    let mut hq = Def { id: "rus_corps".into(), class: Some(DefClass::Building), health: 1000.0, blocking_shape: Some("SQUARE".into()), blocking_size: Some([20.0, 20.0]), ..Default::default() };
    hq.speed_max = 0.0;
    data.by_id.insert(hq.id.clone(), data.defs.len() as u32);
    data.defs.push(hq);
    let hq_def = data.defs.len() as u32 - 1;
    let mut w = world(open_map(64, 64, 4.0), data);
    let building = w.spawn_actor(hq_def, Some(1), w.ground(Vec2::new(120.0, 120.0)), 0.0);
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(40.0, 120.0)), 0.0);
    let to = Vec2::new(200.0, 120.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(truck)], to, attack_move: false });
    let mut inside = false;
    let arrived = run_until(&mut w, 120.0, |w| {
        let p = pos(w, truck);
        inside |= (p.x - 120.0).abs() < 10.0 && (p.y - 120.0).abs() < 10.0;
        w.actors[truck as usize].order == Order::Idle
    });
    assert!(arrived && !inside, "drove through the building: arrived {arrived}");
    assert!((pos(&w, truck) - to).length() < 0.6);
    // A truck created at the building's centre drives out of its footprint.
    let recruit = w.spawn_actor(ids.truck, Some(1), w.ground(Vec2::new(120.0, 120.0)), 0.0);
    let out = Vec2::new(120.0, 200.0);
    order(&mut w, Command::Move { kingdom: 1, units: vec![Unit::Actor(recruit)], to: out, attack_move: false });
    assert!(run_until(&mut w, 60.0, |w| w.actors[recruit as usize].order == Order::Idle), "recruit stuck at {:?}", pos(&w, recruit));
    assert!((pos(&w, recruit) - out).length() < 0.6);
    // Once the building is gone the straight line is free again.
    w.actors[building as usize].removed = true;
    let back = Vec2::new(40.0, 121.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(truck)], to: back, attack_move: false });
    advance(&mut w, 3);
    assert_eq!(w.actors[truck as usize].motion.path.len(), 1, "{:?}", w.actors[truck as usize].motion.path);
}

#[test]
fn world_step_drives_movement() {
    let (data, ids) = data();
    let mut w = world(open_map(64, 64, 4.0), data);
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(40.0, 40.0)), 0.0);
    w.step(vec![Command::Move { kingdom: 0, units: vec![Unit::Actor(truck)], to: Vec2::new(120.0, 40.0), attack_move: false }]);
    for _ in 0..16 * 10 {
        w.step(Vec::new());
    }
    assert!(pos(&w, truck).x > 60.0, "truck at {:?}", pos(&w, truck));
}

// ---------------------------------------------------------------------------
// Performance: 150 regiments x 9 members on a 224 x 224 map.

#[test]
fn performance_150_regiments() {
    let (data, ids) = data();
    let mut map = open_map(224, 224, 8.0);
    // A few obstacles: walls and a lake.
    for y in 30..190 {
        set(&mut map, 100, y, BLOCKED);
    }
    for y in 60..120 {
        for x in 140..170 {
            set(&mut map, x, y, NOT_LAND_PASSABLE);
        }
    }
    let mut w = world(map, data);
    // 75 regiments of kingdom 0 on the left march east, 75 of kingdom 1 on
    // the right march west along the same rows: they meet head-on (blocking).
    let mut orgs = Vec::new();
    for i in 0..150u32 {
        let (side, j) = (i % 2, i / 2);
        let (gx, gy) = (j % 5, j / 5);
        let x = if side == 0 { 80.0 + gx as f32 * 40.0 } else { 1700.0 - gx as f32 * 40.0 };
        let center = Vec2::new(x, 120.0 + gy as f32 * 100.0);
        let heading = if side == 0 { 0.0 } else { PI };
        orgs.push((spawn_regiment(&mut w, ids.regiment, side, center, heading), side, gx, gy));
    }
    let t = std::time::Instant::now();
    for &(o, side, gx, gy) in &orgs {
        let x = if side == 0 { 1400.0 + gx as f32 * 40.0 } else { 300.0 - gx as f32 * 40.0 };
        let to = Vec2::new(x, 120.0 + gy as f32 * 100.0);
        movement::command(&mut w, &Command::Move { kingdom: side, units: vec![Unit::Org(o)], to, attack_move: false }).unwrap();
    }
    // Planning (queued and budgeted per tick).
    let mut planning_ticks = 0;
    let mut planning_worst = 0.0f64;
    loop {
        let s = std::time::Instant::now();
        advance(&mut w, 1);
        planning_worst = planning_worst.max(s.elapsed().as_secs_f64() * 1000.0);
        planning_ticks += 1;
        if movement::queued_requests(&w) == 0 {
            break;
        }
    }
    let planning = t.elapsed();
    let stats = w.movement.stats;
    advance(&mut w, 32);
    let ticks = 160;
    let t = std::time::Instant::now();
    advance(&mut w, ticks);
    let per_tick = t.elapsed().as_secs_f64() * 1000.0 / ticks as f64;
    let moving = w.actors.iter().filter(|a| movement::actor_moving(&w, a.id)).count();
    eprintln!(
        "150 regiments x 9 marching: {per_tick:.3} ms per tick ({moving} movers); planning {} searches, {} tiles, {:.1} ms over {} ticks (worst tick {planning_worst:.2} ms)",
        stats.searches,
        stats.expanded,
        planning.as_secs_f64() * 1000.0,
        planning_ticks
    );
    assert!(moving > 1200, "most members are moving: {moving}");
    // Keep going until the columns meet and pass (head-on blocking).
    let t = std::time::Instant::now();
    let mut worst = 0.0f64;
    let meet_ticks = 16 * 150;
    for _ in 0..meet_ticks {
        let s = std::time::Instant::now();
        advance(&mut w, 1);
        worst = worst.max(s.elapsed().as_secs_f64() * 1000.0);
    }
    let meeting = t.elapsed().as_secs_f64() * 1000.0 / meet_ticks as f64;
    eprintln!("columns meeting: {meeting:.3} ms per tick on average, worst tick {worst:.3} ms, {} searches total", w.movement.stats.searches);
    if !cfg!(debug_assertions) {
        assert!(per_tick < 3.0, "movement tick {per_tick:.3} ms");
        assert!(meeting < 3.0, "movement tick while meeting {meeting:.3} ms");
    }
}

// ---------------------------------------------------------------------------
// New members walking out of their structure (movement::walk_member).

/// A member of `org` (slot 0) standing at `door`, facing `to`, as member
/// creation puts it at a structure's door.
fn member_at_door(w: &mut World, org: OrgId, door: Vec2, to: Vec2) -> ActorId {
    let a = w.orgs[org as usize].members[0];
    let d = to - door;
    w.actors[a as usize].pos = w.ground(door);
    w.actors[a as usize].facing = d.y.atan2(d.x);
    a
}

#[test]
fn new_member_walks_to_the_deployment_point_then_to_its_slot() {
    let (data, ids) = data();
    let mut w = world(open_map(64, 64, 2.0), data);
    let center = Vec2::new(60.0, 60.0);
    let o = spawn_regiment(&mut w, ids.regiment, 0, center, 0.0);
    let (door, point) = (center + Vec2::new(0.0, 25.0), center + Vec2::new(0.0, 20.0));
    let a = member_at_door(&mut w, o, door, point);
    let others: Vec<(ActorId, Vec2)> = w.orgs[o as usize].members.iter().filter(|&&m| m != a).map(|&m| (m, pos(&w, m))).collect();
    movement::walk_member(&mut w, a, point);
    let mut top = 0.0f32;
    let reached = run_until(&mut w, 4.0, |w| {
        top = top.max(w.actors[a as usize].motion.speed);
        (pos(w, a) - point).length() < 0.5
    });
    let t = w.time;
    let facing = w.actors[a as usize].facing;
    eprintln!("deployment point reached {reached} at t={t:.2} (5 m), top speed {top:.3}, facing {facing:.3}");
    assert!(reached && t < 3.0, "walk took {t}");
    // Its own top speed (rus_infantry 2.5), no turn at the point.
    assert!((top - 2.5).abs() < 0.05, "speed {top}");
    assert!((facing + FRAC_PI_2).abs() < 0.05, "facing {facing}");
    assert_eq!(w.orgs[o as usize].order, Order::Idle);
    let slot = slot_position(center, 0.0, INFANTRY_POSITIONS[0]);
    let back = run_until(&mut w, 15.0, |w| (pos(w, a) - slot).length() < 0.5 && !movement::actor_moving(w, a));
    eprintln!("back in its slot {back} at t={:.2}, error {:.3}", w.time, (pos(&w, a) - slot).length());
    assert!(back);
    // The regiment stood still all along.
    for (m, p) in others {
        assert!((pos(&w, m) - p).length() < 1e-3, "member {m} moved");
    }
    assert_eq!(w.orgs[o as usize].order, Order::Idle);
    assert!(formation_error(&w, o) < 0.5);
}

#[test]
fn walking_member_passes_through_others_and_redeploys_off_an_occupied_point() {
    let (data, ids) = data();
    let mut w = world(open_map(64, 64, 2.0), data);
    let (door, point) = (Vec2::new(60.0, 70.0), Vec2::new(60.0, 64.0));
    let walker = w.spawn_actor(ids.infantry, Some(0), w.ground(door), -FRAC_PI_2);
    let on_the_way = Vec2::new(60.0, 67.0);
    let bystander = w.spawn_actor(ids.infantry, Some(0), w.ground(on_the_way), 0.0);
    movement::walk_member(&mut w, walker, point);
    // Someone steps onto the point after the walk began.
    let squatter = w.spawn_actor(ids.infantry, Some(0), w.ground(point), 0.0);
    let mut closest = f32::MAX;
    run_until(&mut w, 10.0, |w| {
        closest = closest.min((pos(w, walker) - on_the_way).length());
        false
    });
    let end = pos(&w, walker);
    let settled = !movement::actor_moving(&w, walker);
    eprintln!("walker settled {settled} at {end:?}; closest to the bystander {closest:.3}; from the point {:.3}", (end - point).length());
    assert!(settled);
    // Ethereal on the way: straight through the bystander, who stays put.
    assert!(closest < 0.3, "went around: {closest}");
    assert!((pos(&w, bystander) - on_the_way).length() < 1e-3);
    // Redeployed to a free spot within 8 m of the occupied point.
    let gap = (end - pos(&w, squatter)).length();
    assert!(gap >= 0.7 - 1e-3 && (end - point).length() <= 8.5, "gap {gap}");
    assert!(w.actors[walker as usize].health > 0.0);
}

#[test]
fn a_short_redeploy_reaches_its_spot() {
    // The walk ends only at the point itself (mode 1, 0.1 m), so a free
    // spot half a metre away is reached instead of creeping toward it.
    let (data, ids) = data();
    let mut w = world(open_map(64, 64, 2.0), data);
    let (door, point) = (Vec2::new(60.0, 70.0), Vec2::new(60.0, 64.0));
    let walker = w.spawn_actor(ids.infantry, Some(0), w.ground(door), -FRAC_PI_2);
    movement::walk_member(&mut w, walker, point);
    let squatter = w.spawn_actor(ids.infantry, Some(0), w.ground(point + Vec2::new(0.0, 0.6)), 0.0);
    advance(&mut w, 8 * aa_play::TICKS_PER_SECOND);
    let end = pos(&w, walker);
    let gap = (end - pos(&w, squatter)).length();
    eprintln!("walker at {end:?}, gap to the squatter {gap:.3}");
    assert!(w.actors[walker as usize].health > 0.0, "died redeploying");
    assert!(gap >= 0.7 - 1e-3, "still overlapping: {gap}");
    assert!((end - (point - Vec2::new(0.0, 0.5))).length() < 0.1, "at {end:?}");
}

#[test]
fn a_regiment_order_replaces_the_walk_out() {
    let (data, ids) = data();
    let mut w = world(open_map(64, 64, 2.0), data);
    let center = Vec2::new(40.0, 50.0);
    let o = spawn_regiment(&mut w, ids.regiment, 0, center, 0.0);
    let (door, point) = (center + Vec2::new(0.0, 20.0), center + Vec2::new(0.0, 16.0));
    let a = member_at_door(&mut w, o, door, point);
    movement::walk_member(&mut w, a, point);
    advance(&mut w, (0.3 * aa_play::TICKS_PER_SECOND as f32) as u32);
    let to = center + Vec2::new(40.0, 0.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Org(o)], to, attack_move: false });
    let mut closest = f32::MAX;
    let settled = run_until(&mut w, 60.0, |w| {
        closest = closest.min((pos(w, a) - point).length());
        org_settled(w, o)
    });
    let error = formation_error(&w, o);
    eprintln!("settled {settled} at t={:.1}, formation error {error:.2}, walker's closest to the point {closest:.2}", w.time);
    assert!(settled && error < 1.5, "error {error}");
    assert!(closest > 1.0, "kept walking to the point");
    assert!((w.orgs[o as usize].anchor - to).length() < 1.0);
}

// ---------------------------------------------------------------------------
// Following an actor (Command::Follow: a right-click on an actor).

const TPS: u32 = aa_play::TICKS_PER_SECOND;

#[test]
fn regiment_follows_a_moving_regiment_and_keeps_following() {
    let (data, ids) = data();
    let mut w = world(open_map(96, 64, 2.0), data);
    let a = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(30.0, 40.0), 0.0);
    let b = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(30.0, 80.0), 0.0);
    let target = w.orgs[b as usize].members[8];
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Org(b)], to: Vec2::new(110.0, 80.0), attack_move: false });
    order(&mut w, Command::Follow { kingdom: 0, units: vec![Unit::Org(a)], target });
    advance(&mut w, 45 * TPS);
    let gap = (w.orgs[a as usize].anchor - pos(&w, target)).length();
    eprintln!("after 45 s: follower at {:?}, {gap:.2} m from the target, order {:?}", w.orgs[a as usize].anchor, w.orgs[a as usize].order);
    // Within CAITargetInRangeZOCMultiplier 0.6 x control_range 18 (and the
    // leader's last leg), still on its move.
    assert!(w.orgs[a as usize].anchor.x > 90.0 && gap < 12.0, "gap {gap}");
    assert!(matches!(w.orgs[a as usize].order, Order::Move { attack_move: false, .. }));
    // The target moves on: the follower goes after it again.
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Org(b)], to: Vec2::new(110.0, 25.0), attack_move: false });
    advance(&mut w, 45 * TPS);
    let gap = (w.orgs[a as usize].anchor - pos(&w, target)).length();
    eprintln!("after 90 s: follower at {:?}, {gap:.2} m from the target", w.orgs[a as usize].anchor);
    assert!(w.orgs[a as usize].anchor.y < 50.0 && gap < 12.0, "gap {gap}");
    assert!(matches!(w.orgs[a as usize].order, Order::Move { .. }));
}

#[test]
fn lone_actor_follows_and_holds_five_metres_short() {
    let (data, ids) = data();
    let mut w = world(open_map(96, 64, 2.0), data);
    let truck = w.spawn_actor(ids.truck, Some(0), w.ground(Vec2::new(30.0, 40.0)), 0.0);
    let runner = w.spawn_actor(ids.infantry, Some(0), w.ground(Vec2::new(50.0, 40.0)), 0.0);
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(runner)], to: Vec2::new(150.0, 40.0), attack_move: false });
    order(&mut w, Command::Follow { kingdom: 0, units: vec![Unit::Actor(truck)], target: runner });
    let gap = |w: &World| (pos(w, truck) - pos(w, runner)).length() - 0.65 - 0.35;
    advance(&mut w, 50 * TPS);
    eprintln!("truck {:?} runner {:?} gap {:.2}, moving {}", pos(&w, truck), pos(&w, runner), gap(&w), movement::actor_moving(&w, truck));
    assert!(gap(&w) < 5.0 && gap(&w) > 1.0 && !movement::actor_moving(&w, truck));
    assert!(matches!(w.actors[truck as usize].order, Order::Move { .. }), "still following");
    order(&mut w, Command::Move { kingdom: 0, units: vec![Unit::Actor(runner)], to: Vec2::new(150.0, 100.0), attack_move: false });
    advance(&mut w, 40 * TPS);
    eprintln!("then truck {:?} runner {:?} gap {:.2}", pos(&w, truck), pos(&w, runner), gap(&w));
    assert!(gap(&w) < 5.0 && pos(&w, truck).y > 80.0);
}

#[test]
fn following_ends_when_the_target_is_gone_or_dead() {
    let (data, ids) = data();
    let mut w = world(open_map(96, 64, 2.0), data);
    let a = spawn_regiment(&mut w, ids.regiment, 0, Vec2::new(30.0, 40.0), 0.0);
    let enemy = w.spawn_actor(ids.infantry, Some(1), w.ground(Vec2::new(120.0, 40.0)), 0.0);
    order(&mut w, Command::Follow { kingdom: 0, units: vec![Unit::Org(a)], target: enemy });
    advance(&mut w, 5 * TPS);
    assert!(matches!(w.orgs[a as usize].order, Order::Move { .. }));
    // Gone (EventTargetGone): the regiment stops where it is.
    w.actors[enemy as usize].removed = true;
    advance(&mut w, 2 * TPS);
    assert_eq!(w.orgs[a as usize].order, Order::Idle);
    assert!(movement::command(&mut w, &Command::Follow { kingdom: 0, units: vec![Unit::Org(a)], target: enemy }).is_err());
    // A dead target: walked up to, then the order ends.
    let corpse = w.spawn_actor(ids.infantry, Some(1), w.ground(Vec2::new(120.0, 60.0)), 0.0);
    w.actors[corpse as usize].alive = false;
    order(&mut w, Command::Follow { kingdom: 0, units: vec![Unit::Org(a)], target: corpse });
    let ended = run_until(&mut w, 60.0, |w| w.orgs[a as usize].order == Order::Idle);
    let gap = (w.orgs[a as usize].anchor - pos(&w, corpse)).length();
    eprintln!("dead target: order ended {ended} at t={:.1}, {gap:.2} m away", w.time);
    assert!(ended && gap <= 0.6 * 18.0 + 1.0);
}

// ---------------------------------------------------------------------------
// Real data (needs AA_GAME_ROOT).

#[test]
#[ignore]
fn real_berlin_routes_and_a_regiment_marches() {
    let root = std::path::PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"E:\Games\Axis and Alies".into()));
    let data = GameData::load(&root).expect("GameData::load");
    let map = GameMap::load(&root, "(2v2)Berlin").expect("GameMap::load");
    eprintln!("Berlin {}x{} cell {} starts {:?}", map.width, map.height, map.cell_size, map.start_locations);
    let starts = map.start_locations.clone();
    assert!(starts.len() >= 2);
    let t = std::time::Instant::now();
    let route = Pathfinder::default().solve(&map, &PathRules::from_data(&data), &Request { from: starts[0], to: starts[1], class: MoverClass::Land, max_speed: 2.5 });
    eprintln!("start 0 -> 1: {:?}, {} nodes, {} tiles, {:?}", route.status, route.waypoints.len(), route.expanded, t.elapsed());
    assert_ne!(route.status, Status::Failed);
    let regiment = ["regiment_rus_infantry", "regiment_ger_infantry", "regiment_usa_infantry"].into_iter().find_map(|id| data.find(id)).expect("an infantry regiment def");
    let def = data.def(regiment).clone();
    eprintln!("{}: slots {:?} leader {:?} formations {:?}", def.id, def.slots.iter().map(|s| s.1).collect::<Vec<_>>(), def.leader_slot, def.formations.iter().map(|f| f.id.clone()).collect::<Vec<_>>());
    let mut w = World::empty(Arc::new(data), Arc::new(map), 3);
    w.kingdoms.push(kingdom(0, 0));
    let heading = 0.0;
    let id = w.orgs.len() as OrgId;
    let mut members = Vec::new();
    let offsets = def.default_formation.and_then(|i| def.formations.get(i)).map(|f| f.positions.clone()).unwrap_or_else(|| def.slots.iter().map(|s| s.1).collect());
    for (slot, (m, _)) in def.slots.iter().enumerate() {
        if *m == u32::MAX {
            continue;
        }
        let p = w.ground(slot_position(starts[0], heading, offsets[slot]));
        let a = w.spawn_actor(*m, Some(0), p, heading);
        w.actors[a as usize].org = Some(id);
        w.actors[a as usize].slot = Some(slot as u16);
        members.push(a);
    }
    let speeds: Vec<f32> = members.iter().map(|&m| w.data.def(w.actors[m as usize].def).speed_max).collect();
    eprintln!("member speeds {speeds:?}");
    w.orgs.push(Organization { id, def: regiment, kingdom: 0, members, hq: None, order: Order::Idle, anchor: starts[0], heading, morale: 1.0, experience: 0.0, alive: true });
    let to = starts[0] + (starts[1] - starts[0]).normalize() * 150.0;
    movement::command(&mut w, &Command::Move { kingdom: 0, units: vec![Unit::Org(id)], to, attack_move: false }).unwrap();
    advance(&mut w, 16 * 30);
    let moved = (w.orgs[id as usize].anchor - starts[0]).length();
    eprintln!("moved {moved:.1} m in 30 s ({:.3} m/s), formation error {:.2}", moved / 30.0, formation_error_any(&w, id, &offsets));
    assert!(moved > 30.0);
    let settled = run_until(&mut w, 180.0, |w| org_settled(w, id));
    let anchor = w.orgs[id as usize].anchor;
    let error = formation_error_any(&w, id, &offsets);
    eprintln!("settled {settled} at t={:.1} anchor {anchor:?} (goal {to:?}), formation error {error:.2}", w.time);
    assert!(settled && error < 1.5, "error {error}");
    // A whole start-to-start march.
    let goal = starts[1];
    movement::command(&mut w, &Command::Move { kingdom: 0, units: vec![Unit::Org(id)], to: goal, attack_move: false }).unwrap();
    let settled = run_until(&mut w, 400.0, |w| org_settled(w, id));
    let anchor = w.orgs[id as usize].anchor;
    let error = formation_error_any(&w, id, &offsets);
    eprintln!("start 1 march: settled {settled} at t={:.1} anchor {anchor:?} (goal {goal:?}), error {error:.2}", w.time);
    assert!(settled && error < 1.5 && (anchor - goal).length() < 1.0, "error {error}");
}
fn formation_error_any(w: &World, o: OrgId, offsets: &[[f32; 2]]) -> f32 {
    let org = &w.orgs[o as usize];
    let facing = Vec2::new(org.heading.cos(), org.heading.sin());
    let mut slots: Vec<Vec2> = offsets.iter().map(|&p| org.anchor + rotate_offset(Vec2::from(p), facing)).collect();
    let mut worst = 0.0f32;
    for &m in &org.members {
        let p = pos(w, m);
        let Some((i, d)) = slots.iter().enumerate().map(|(i, s)| (i, (*s - p).length())).min_by(|a, b| a.1.total_cmp(&b.1)) else { break };
        worst = worst.max(d);
        slots.remove(i);
    }
    worst
}
