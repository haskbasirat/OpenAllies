//! Fog of war on synthetic worlds: the detection rules and the per-tick
//! cost for 2000 moving actors (run with `--release -- --nocapture` for
//! the cost numbers).
use aa_play::data::DefClass;
use aa_play::fog::{self, FogMode, Sighting, TileView};
use aa_play::{Controller, Def, GameData, GameMap, Kingdom, Order, Organization, World};
use glam::{Vec2, Vec3};
use std::sync::Arc;

struct Ids {
    soldier: u32,
    regiment: u32,
    corps: u32,
}

fn data() -> (GameData, Ids) {
    let mut data = GameData::default();
    let mut push = |d: Def| -> u32 {
        let id = data.defs.len() as u32;
        data.by_id.insert(d.id.clone(), id);
        data.defs.push(d);
        id
    };
    let soldier = push(Def { id: "soldier".into(), class: Some(DefClass::Member), health: 10.0, detection_range: 18.0, remote_detection_range: 10.0, speed_max: 4.0, ..Default::default() });
    let regiment = push(Def { id: "regiment".into(), class: Some(DefClass::Organization), detection_range: 26.0, remote_detection_range: 12.0, control_range: 18.0, ..Default::default() });
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

fn map(cells: u32) -> GameMap {
    GameMap { id: "fog-test".into(), width: cells, height: cells, cell_size: 4.0, ..Default::default() }
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

fn world(cells: u32, mode: FogMode) -> (World, Ids) {
    let (data, ids) = data();
    let mut w = World::empty(Arc::new(data), Arc::new(map(cells)), 7);
    w.kingdoms.push(kingdom(0, 0));
    w.kingdoms.push(kingdom(1, 1));
    fog::set_mode(&mut w, mode);
    (w, ids)
}

fn org(w: &mut World, def: u32, kingdom: u32, members: Vec<u32>, at: Vec2) -> u32 {
    let id = w.orgs.len() as u32;
    for &m in &members {
        w.actors[m as usize].org = Some(id);
    }
    w.orgs.push(Organization { id, def, kingdom, members, hq: None, order: Order::Idle, anchor: at, heading: 0.0, morale: 1.0, experience: 0.0, alive: true });
    id
}

#[test]
fn a_soldier_sees_within_its_range_only() {
    let (mut w, ids) = world(128, FogMode::Full);
    let me = w.spawn_actor(ids.soldier, Some(0), Vec3::new(200.0, 200.0, 0.0), 0.0);
    let near = w.spawn_actor(ids.soldier, Some(1), Vec3::new(212.0, 200.0, 0.0), 0.0);
    let far = w.spawn_actor(ids.soldier, Some(1), Vec3::new(250.0, 200.0, 0.0), 0.0);
    fog::tick(&mut w);
    assert!(fog::actor_visible(&w, near, Some(0)));
    assert!(!fog::actor_visible(&w, far, Some(0)));
    assert!(fog::actor_visible(&w, me, Some(0)));
    assert_eq!(fog::point_view(&w, Some(0), Vec2::new(200.0, 200.0)), TileView::Visible);
    assert_eq!(fog::point_view(&w, Some(0), Vec2::new(300.0, 300.0)), TileView::Unexplored);
    // walk away: the tile stays explored (grey), not visible
    w.actors[me as usize].pos = Vec3::new(400.0, 400.0, 0.0);
    fog::tick(&mut w);
    assert_eq!(fog::point_view(&w, Some(0), Vec2::new(200.0, 200.0)), TileView::Explored);
    assert!(!fog::actor_visible(&w, near, Some(0)));
    // the enemy sees nothing of ours far away
    assert!(!fog::actor_visible(&w, me, Some(1)));
}

#[test]
fn explored_start_remembers_static_actors_and_none_sees_all() {
    let (mut w, ids) = world(128, FogMode::Explored);
    let hq = w.spawn_actor(ids.corps, Some(1), Vec3::new(400.0, 400.0, 0.0), 0.0);
    let unit = w.spawn_actor(ids.soldier, Some(1), Vec3::new(380.0, 380.0, 0.0), 0.0);
    let _me = w.spawn_actor(ids.soldier, Some(0), Vec3::new(40.0, 40.0, 0.0), 0.0);
    fog::tick(&mut w);
    assert_eq!(fog::point_view(&w, Some(0), Vec2::new(300.0, 300.0)), TileView::Explored);
    // another side's base is not explored at the start; once seen it is
    // remembered
    assert_eq!(fog::sighting(&w, hq, Some(0)), Sighting::Hidden);
    assert_eq!(fog::sighting(&w, unit, Some(0)), Sighting::Hidden);
    let scout = w.spawn_actor(ids.soldier, Some(0), Vec3::new(380.0, 400.0, 0.0), 0.0);
    fog::tick(&mut w);
    assert_eq!(fog::sighting(&w, hq, Some(0)), Sighting::Visible);
    w.actors[scout as usize].pos = Vec3::new(40.0, 400.0, 0.0);
    fog::tick(&mut w);
    assert!(matches!(fog::sighting(&w, hq, Some(0)), Sighting::Remembered(_)));
    let (mut w, ids) = world(128, FogMode::None);
    let unit = w.spawn_actor(ids.soldier, Some(1), Vec3::new(380.0, 380.0, 0.0), 0.0);
    fog::tick(&mut w);
    assert_eq!(fog::sighting(&w, unit, Some(0)), Sighting::Visible);
}

#[test]
fn an_engaged_target_is_revealed_to_the_attacker() {
    let (mut w, ids) = world(128, FogMode::Full);
    let target = w.spawn_actor(ids.soldier, Some(1), Vec3::new(200.0, 200.0, 0.0), 0.0);
    let shooter = w.spawn_actor(ids.soldier, Some(0), Vec3::new(260.0, 200.0, 0.0), 0.0);
    fog::tick(&mut w);
    assert!(!fog::actor_visible(&w, target, Some(0)));
    w.actors[shooter as usize].combat.target = Some(target);
    fog::tick(&mut w);
    assert!(fog::actor_visible(&w, target, Some(0)));
    // the RZOD reveals without exploring
    w.actors[shooter as usize].combat.target = None;
    fog::tick(&mut w);
    assert_eq!(fog::point_view(&w, Some(0), Vec2::new(200.0, 200.0)), TileView::Unexplored);
}

#[test]
fn per_tick_cost_for_2000_moving_actors() {
    let (mut w, ids) = world(256, FogMode::Explored);
    let mut rng = 12345u64;
    let mut rand = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        (rng >> 11) as f32 / (1u64 << 53) as f32
    };
    let mut velocity = Vec::new();
    for o in 0..250 {
        let k = (o % 2) as u32;
        let centre = Vec2::new(40.0 + rand() * 940.0, 40.0 + rand() * 940.0);
        let mut members = Vec::new();
        for _ in 0..8 {
            let p = centre + Vec2::new(rand() * 12.0 - 6.0, rand() * 12.0 - 6.0);
            members.push(w.spawn_actor(ids.soldier, Some(k), p.extend(0.0), 0.0));
        }
        let heading = rand() * std::f32::consts::TAU;
        let v = Vec2::new(heading.cos(), heading.sin()) * 4.0 * aa_play::DT;
        for _ in 0..8 {
            velocity.push(v);
        }
        org(&mut w, ids.regiment, k, members, centre);
    }
    for k in 0..2 {
        w.spawn_actor(ids.corps, Some(k), Vec3::new(100.0 + 800.0 * k as f32, 100.0 + 800.0 * k as f32, 0.0), 0.0);
    }
    assert!(w.actors.len() >= 2000);
    // warm-up tick: every zone is created
    fog::tick(&mut w);
    let first = w.fog.stats.last_us;
    let mut total = 0.0;
    let mut max = 0.0f32;
    let ticks = 320;
    for t in 0..ticks {
        for (i, v) in velocity.iter().enumerate() {
            let a = &mut w.actors[i];
            let mut p = a.pos.truncate() + *v;
            p = p.clamp(Vec2::splat(8.0), Vec2::splat(1016.0));
            a.pos = p.extend(0.0);
        }
        // a few engagements change every second
        if t % 16 == 0 {
            for i in 0..velocity.len() {
                let target = (i * 7919 + t) % velocity.len();
                w.actors[i].combat.target = (i % 10 == 0 && w.actors[target].kingdom != w.actors[i].kingdom).then_some(target as u32);
            }
        }
        fog::tick(&mut w);
        total += w.fog.stats.last_us;
        max = max.max(w.fog.stats.last_us);
    }
    let mean = total / ticks as f32;
    println!("fog cost, {} actors, {} organizations, 256x256 tiles: first tick {first:.0} us, mean {mean:.0} us, max {max:.0} us", w.actors.len(), w.orgs.len());
    if !cfg!(debug_assertions) {
        assert!(mean < 1000.0, "mean fog tick {mean} us is over 1 ms");
    }
}
