//! The supply and fog rules on the exact ports (aa-economy `supply`,
//! aa-zones), on small synthetic worlds: allied supply zones, who takes the
//! out-of-supply state, the SUPPLY_RANGE / DETECTION_RANGE modifiers, the
//! reverse RZOD of an attacker, and the city siege byte.
use aa_play::data::{DefClass, Modifier, PropertyDef};
use aa_play::fog::{self, FogMode};
use aa_play::{Controller, Def, GameData, GameMap, Kingdom, World};
use glam::{Vec2, Vec3};
use std::sync::Arc;

fn components(names: &[&str]) -> u32 {
    names.iter().map(|n| aa_rules::native::COMPONENTS.iter().find(|(c, _)| c == n).map(|(_, b)| 1u32 << b).unwrap()).fold(0, |a, b| a | b)
}

struct Ids {
    hq: u32,
    regiment: u32,
    farm: u32,
    scout: u32,
    soldier: u32,
}

fn data() -> (GameData, Ids) {
    let mut data = GameData::default();
    data.rules.insert("SupplyStockpileRechargeRate".into(), 1.0);
    data.rules.insert("ModifierAttributeClampAdd".into(), 1.0);
    data.rules.insert("SupplyProvideUnderSiege".into(), 1.0);
    data.rules.insert("EngagementShowReverseRZOD".into(), 1.0);
    let mut push = |d: Def| -> u32 {
        data.by_id.insert(d.id.clone(), data.defs.len() as u32);
        data.defs.push(d);
        data.defs.len() as u32 - 1
    };
    let soldier = push(Def {
        id: "soldier".into(),
        class: Some(DefClass::Member),
        health: 100.0,
        speed_max: 2.5,
        actor_data: true,
        components: components(&["BodyComponent", "ElementComponent", "MoverComponent"]),
        ..Default::default()
    });
    let regiment = push(Def {
        id: "regiment".into(),
        class: Some(DefClass::Organization),
        actor_data: true,
        components: components(&["OrganizationComponent"]),
        supply_used: true,
        in_supply_detached: true,
        leader_slot: Some(0),
        slots: vec![(soldier, [0.0, 0.0])],
        slot_entries: vec![0],
        control_range: 18.0,
        ..Default::default()
    });
    let hq = push(Def {
        id: "hq".into(),
        class: Some(DefClass::Building),
        health: 2000.0,
        actor_data: true,
        components: components(&["BodyComponent", "PropertiesComponent", "SupplyComponent"]),
        has_supply: true,
        supply_range: 60.0,
        supply_relay: false,
        blocking_size: Some([10.0, 10.0]),
        ..Default::default()
    });
    let farm = push(Def {
        id: "farm".into(),
        class: Some(DefClass::Building),
        health: 500.0,
        actor_data: true,
        components: components(&["BodyComponent", "EconomyComponent"]),
        has_economy: true,
        blocking_size: Some([6.0, 6.0]),
        ..Default::default()
    });
    let scout = push(Def {
        id: "scout".into(),
        class: Some(DefClass::Member),
        health: 100.0,
        speed_max: 2.5,
        actor_data: true,
        components: components(&["BodyComponent", "MoverComponent", "PropertiesComponent", "DetectionComponent"]),
        detection_range: 27.0,
        remote_detection_range: 10.0,
        properties: vec!["recon".into()],
        ..Default::default()
    });
    data.properties.insert(
        "recon".into(),
        PropertyDef {
            id: "recon".into(),
            modifiers: vec![Modifier { kind: "ADD".into(), attribute: Some("DETECTION_RANGE".into()), amount: 12.0, ..Default::default() }],
            ..Default::default()
        },
    );
    data.properties.insert(
        "tech_improved_logistics".into(),
        PropertyDef {
            id: "tech_improved_logistics".into(),
            is_technology: true,
            modifiers: vec![
                Modifier { kind: "ADD".into(), attribute: Some("DETECTION_RANGE".into()), amount: 5.0, ..Default::default() },
                Modifier { kind: "ADD".into(), attribute: Some("SUPPLY_RANGE".into()), amount: 5.0, ..Default::default() },
            ],
            ..Default::default()
        },
    );
    (data, Ids { hq, regiment, farm, scout, soldier })
}

/// A flat map with the blocked outer tile ring of every real map.
fn map() -> GameMap {
    let (w, h) = (128, 128);
    let border = |i: u32| {
        let (x, y) = (i % w, i / w);
        if x == 0 || y == 0 || x + 1 == w || y + 1 == h { aa_play::map::BLOCKED } else { 0 }
    };
    GameMap {
        id: "flat".into(),
        width: w,
        height: h,
        cell_size: 2.0,
        heights: vec![0.0; ((w + 1) * (h + 1)) as usize],
        flags: (0..w * h).map(border).collect(),
        speed_limit: vec![aa_play::map::NO_SPEED_LIMIT; (w * h) as usize],
        ..Default::default()
    }
}

fn world(teams: &[u32]) -> (World, Ids) {
    let (data, ids) = data();
    let mut w = World::empty(Arc::new(data), Arc::new(map()), 7);
    for (i, &team) in teams.iter().enumerate() {
        w.kingdoms.push(Kingdom {
            id: i as u32,
            name: format!("k{i}"),
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
        });
    }
    (w, ids)
}

/// The queues (queue 1: supply, organization and detection ticks) and the
/// supply pass of one update.
fn step(w: &mut World) {
    aa_play::scheduler::periodic(w);
    aa_play::supply::tick(w);
    w.clock.advance_fragment();
    w.tick = u64::from(w.clock.tick);
    w.time = f32::from_bits(w.clock.end.0);
}

fn seconds(w: &mut World, s: u32) {
    for _ in 0..16 * s + 1 {
        step(w);
    }
}

/// 0x54F8B0: a supply zone of an allied kingdom supplies; an enemy's does not.
#[test]
fn allied_supply_zones_supply_and_enemy_ones_do_not() {
    for (teams, supplied) in [(vec![0, 0], true), (vec![0, 1], false)] {
        let (mut w, ids) = world(&teams);
        let _hq = aa_play::production::spawn_single(&mut w, ids.hq, Some(1), Vec2::new(100.0, 100.0), 0.0);
        let o = aa_play::production::spawn_organization(&mut w, ids.regiment, 0, Vec2::new(120.0, 100.0), 0.0, None);
        seconds(&mut w, 2);
        assert_eq!(!w.books.org(o).unwrap().out_of_supply, supplied, "teams {teams:?}");
        // the members never take the state (0x4D0A80 asks their
        // organization; nothing sets their own bit)
        let m = w.orgs[o as usize].members[0];
        assert!(!w.books.out_of_supply(m));
    }
}

/// Only the things that need supply take state 19: a building without a
/// supply_required Economy, far from every zone, stays in supply.
#[test]
fn things_that_need_no_supply_never_leave_it() {
    let (mut w, ids) = world(&[0]);
    let farm = aa_play::production::spawn_single(&mut w, ids.farm, Some(0), Vec2::new(200.0, 200.0), 0.0);
    let o = aa_play::production::spawn_organization(&mut w, ids.regiment, 0, Vec2::new(40.0, 200.0), 0.0, None);
    seconds(&mut w, 2);
    assert!(!w.books.out_of_supply(farm));
    assert!(w.books.org(o).unwrap().out_of_supply, "a supply_used regiment far from the zones is cut off");
}

/// 0x58AA20 / 0x45D550 / 0x501BD0: tech_improved_logistics adds 5 m to the
/// supply component radius of a supplier with a modifier context.
#[test]
fn supply_range_follows_the_technologies() {
    let (mut w, ids) = world(&[0]);
    let hq = aa_play::production::spawn_single(&mut w, ids.hq, Some(0), Vec2::new(100.0, 100.0), 0.0);
    seconds(&mut w, 1);
    let radius = |w: &World| f32::from_bits(fog::engine(w).unwrap().supply_zone(hq).unwrap().radius_bits);
    assert_eq!(radius(&w), 60.0);
    w.books.kingdom_mut(0, 0).technologies.push("tech_improved_logistics".into());
    seconds(&mut w, 2);
    assert_eq!(radius(&w), 65.0);
}

/// 0x56EA10 / 0x45D790 / 0x501C30: the recon property adds 12 m to the ZOD,
/// a technology 5 m more.
#[test]
fn detection_range_follows_the_modifiers() {
    let (mut w, ids) = world(&[0, 1]);
    fog::set_mode(&mut w, FogMode::Full);
    let s = w.spawn_actor(ids.scout, Some(0), Vec3::new(100.0, 100.0, 0.0), 0.0);
    fog::tick(&mut w);
    let zod = |w: &World| f32::from_bits(fog::engine(w).unwrap().parts.actors[s as usize].comp.unwrap().zod_radius_bits);
    assert_eq!(zod(&w), 39.0);
    w.books.kingdom_mut(0, 0).technologies.push("tech_improved_logistics".into());
    seconds(&mut w, 2);
    fog::tick(&mut w);
    assert_eq!(zod(&w), 44.0);
}

/// 0x56EDC0 with EngagementShowReverseRZOD: the owner of the thing's Mind's target
/// (+0x70 virtual +0x90) sees the thing's RZOD, without exploring. Such a target is
/// the Mind's, not a weapon's: here a member the engage state handed one (its kill
/// state, UAI +0x13C). A member out of its regiment's ZOC carries its own Detection.
#[test]
fn an_attacker_is_revealed_to_its_target() {
    let (mut w, ids) = world(&[0, 1]);
    let mut d = (*w.data).clone();
    let s = &mut d.defs[ids.soldier as usize];
    s.detection_range = 18.0;
    s.remote_detection_range = 10.0;
    s.components |= components(&["DetectionComponent"]);
    w.data = Arc::new(d);
    fog::set_mode(&mut w, FogMode::Full);
    let target = w.spawn_actor(ids.soldier, Some(1), Vec3::new(100.0, 100.0, 0.0), 0.0);
    let leader = w.spawn_actor(ids.soldier, Some(0), Vec3::new(160.0, 200.0, 0.0), 0.0);
    let shooter = w.spawn_actor(ids.soldier, Some(0), Vec3::new(160.0, 100.0, 0.0), 0.0);
    let o = w.orgs.len() as u32;
    for (i, &m) in [leader, shooter].iter().enumerate() {
        w.actors[m as usize].org = Some(o);
        w.actors[m as usize].slot = Some(i as u16);
    }
    w.orgs.push(aa_play::Organization { id: o, def: ids.regiment, kingdom: 0, members: vec![leader, shooter], hq: None, order: aa_play::Order::Idle, anchor: Vec2::new(160.0, 200.0), heading: 0.0, morale: 1.0, experience: 0.0, alive: true });
    seconds(&mut w, 2);
    fog::tick(&mut w);
    assert_eq!(fog::engine(&w).unwrap().attached(shooter), Some(false));
    assert!(!fog::actor_visible(&w, shooter, Some(1)));
    aa_play::combat::tick(&mut w);
    w.actors[shooter as usize].combat.target = Some(target);
    w.combat.actors[shooter as usize].engage_target = Some(aa_play::combat::TargetRef::Actor(target));
    seconds(&mut w, 1);
    fog::tick(&mut w);
    assert!(fog::actor_visible(&w, shooter, Some(1)), "the target's side sees the shooter");
    // a weapon target is no engager (0x4D3990 is the organization's engage
    // list and the Defender's), so the shooter's side does not see the target
    assert!(!fog::actor_visible(&w, target, Some(0)));
    // the reveal mask drops at the next Detection Tick: visible, never explored
    w.actors[shooter as usize].combat.target = None;
    seconds(&mut w, 1);
    fog::tick(&mut w);
    assert!(!fog::actor_visible(&w, shooter, Some(1)));
    assert_eq!(fog::point_view(&w, Some(1), Vec2::new(160.0, 100.0)), fog::TileView::Unexplored);
}

/// 0x5647A0: over the city's candidates the allied values (owned or a friend)
/// add up, and the city is under siege unless the attack is below that sum
/// (0x5648BC). Under siege it provides no supply zone (0x58ACE0).
#[test]
fn a_city_is_besieged_unless_the_attack_is_below_the_allied_sum() {
    let (mut w, ids) = world(&[0, 0, 1]);
    let mut d = (*w.data).clone();
    let mut city = d.defs[ids.hq as usize].clone();
    city.id = "city".into();
    city.city = true;
    city.city_zone_radius = 60.0;
    city.captureable = true;
    city.destructible = false;
    city.supply_relay = false;
    city.components |= components(&["CityComponent"]);
    let city_def = d.defs.len() as u32;
    d.defs.push(city);
    // Armed things of equal value: each is one candidate of the city.
    let mut armed = d.defs[ids.soldier as usize].clone();
    armed.id = "armed".into();
    armed.destructible = true;
    armed.weapons = vec![aa_play::data::Weapon { range: 10.0, damage: 1.0, ..Default::default() }];
    armed.components |= components(&["PerformerComponent"]);
    let armed_def = d.defs.len() as u32;
    d.defs.push(armed);
    w.data = Arc::new(d);
    let c = aa_play::production::spawn_single(&mut w, city_def, Some(0), Vec2::new(200.0, 200.0), 0.0);
    let at = |i: u32| Vec2::new(180.0 + 5.0 * i as f32, 210.0);
    // Three of the defenders' side, so the friend stands last on the list.
    for (i, k) in [1, 0, 0].into_iter().enumerate() {
        aa_play::production::spawn_single(&mut w, armed_def, Some(k), at(i as u32), 0.0);
    }
    for i in 3..5 {
        aa_play::production::spawn_single(&mut w, armed_def, Some(2), at(i), 0.0);
    }
    // Combat prepares the combat values (Performer +60).
    aa_play::combat::tick(&mut w);
    assert!(w.combat.extras.as_ref().unwrap().def(armed_def).combat_value > 0.0);
    seconds(&mut w, 3);
    assert_eq!(w.actors[c as usize].kingdom, Some(0));
    assert!(!w.books.actor(c).unwrap().city_siege, "an attack of two is below an allied sum of three");
    let active = |w: &World| fog::engine(w).unwrap().supply_zone(c).is_some_and(|z| z.active());
    assert!(active(&w), "the city provides");
    // Two more attackers: four against three.
    for i in 5..7 {
        aa_play::production::spawn_single(&mut w, armed_def, Some(2), at(i), 0.0);
    }
    aa_play::combat::tick(&mut w);
    seconds(&mut w, 3);
    assert_eq!(w.actors[c as usize].kingdom, Some(0), "owned value defends it");
    assert!(w.books.actor(c).unwrap().city_siege, "an attack of four is not below three");
    assert!(!active(&w), "under siege it provides no supply");
}

/// 0x57CBA0 -> 0x5705A0 sets the byte +0x1C when a member joins, and 0x5709D0
/// (the Element Tick) then asks the organization's zone 4 of the tile under
/// it: inside, a member has no ZOD (0x56EA30); outside, the next Detection Tick
/// gives it one; back inside it goes again.
#[test]
fn a_member_carries_no_zones_until_it_leaves_the_zoc() {
    let (mut w, ids) = world(&[0, 1]);
    let mut d = (*w.data).clone();
    let s = &mut d.defs[ids.soldier as usize];
    s.detection_range = 18.0;
    s.remote_detection_range = 10.0;
    s.components |= components(&["DetectionComponent"]);
    let r = &mut d.defs[ids.regiment as usize];
    r.control_range = 18.0;
    w.data = Arc::new(d);
    fog::set_mode(&mut w, FogMode::Full);
    let leader = w.spawn_actor(ids.soldier, Some(0), Vec3::new(100.0, 100.0, 0.0), 0.0);
    let other = w.spawn_actor(ids.soldier, Some(0), Vec3::new(104.0, 100.0, 0.0), 0.0);
    let o = w.orgs.len() as u32;
    w.actors[leader as usize].org = Some(o);
    w.actors[other as usize].org = Some(o);
    w.actors[leader as usize].slot = Some(0);
    w.actors[other as usize].slot = Some(1);
    w.orgs.push(aa_play::Organization { id: o, def: ids.regiment, kingdom: 0, members: vec![leader, other], hq: None, order: aa_play::Order::Idle, anchor: Vec2::new(100.0, 100.0), heading: 0.0, morale: 1.0, experience: 0.0, alive: true });
    let zone = |w: &World, a: u32, t: u32| fog::engine(w).and_then(|e| e.parts.actors[a as usize].zone(t)).filter(|z| z.active());
    seconds(&mut w, 2);
    let e = fog::engine(&w).unwrap();
    assert_eq!(e.parts.orgs[o as usize].zone(4).map(|z| f32::from_bits(z.radius_bits)), Some(18.0), "the organization's ZOC");
    assert_eq!(e.attached(other), Some(true));
    assert!(zone(&w, other, 6).is_none(), "attached: no ZOD of its own");
    // out of the ZOC
    w.actors[other as usize].pos = Vec3::new(160.0, 100.0, 0.0);
    seconds(&mut w, 2);
    let e = fog::engine(&w).unwrap();
    assert_eq!(e.attached(other), Some(false));
    assert_eq!(e.attached(leader), Some(true), "the leader stays attached wherever it stands");
    assert!(zone(&w, other, 6).is_some(), "detached: its own ZOD");
    // back in
    w.actors[other as usize].pos = Vec3::new(104.0, 100.0, 0.0);
    seconds(&mut w, 2);
    assert_eq!(fog::engine(&w).unwrap().attached(other), Some(true));
    assert!(zone(&w, other, 6).is_none());
}

/// 0x566DB0: a Defender's zone 4 takes its control range; 0x563FE0: a city's
/// zone 8 its zone radius; both flood from the thing's tile. A candidate the
/// zone 8 covers enters the city through 0x564300 and registers it on queue 1.
#[test]
fn defenders_and_cities_have_their_zones() {
    let (mut w, ids) = world(&[0, 1]);
    let mut d = (*w.data).clone();
    let id = d.defs.len() as u32;
    let mut city = d.defs[ids.soldier as usize].clone();
    city.id = "city".into();
    city.class = Some(DefClass::Building);
    city.city = true;
    city.city_zone_radius = 30.0;
    city.speed_max = 0.0;
    city.captureable = true;
    city.components = components(&["CityComponent", "BodyComponent"]);
    d.by_id.insert("city".into(), id);
    d.defs.push(city);
    let def = id + 1;
    let mut corps = d.defs[ids.soldier as usize].clone();
    corps.id = "corps".into();
    corps.class = Some(DefClass::Building);
    corps.control_range = 20.0;
    corps.speed_max = 0.0;
    corps.components = components(&["DefenderComponent", "BodyComponent"]);
    d.by_id.insert("corps".into(), def);
    d.defs.push(corps);
    w.data = Arc::new(d);
    let c = w.spawn_actor(id, Some(1), Vec3::new(100.0, 100.0, 0.0), 0.0);
    let b = w.spawn_actor(def, Some(0), Vec3::new(300.0, 300.0, 0.0), 0.0);
    fog::tick(&mut w);
    let e = fog::engine(&w).unwrap();
    assert_eq!(e.parts.actors[c as usize].zone(8).map(|z| (f32::from_bits(z.radius_bits), z.flags)), Some((30.0, 0x100)));
    assert_eq!(e.parts.actors[b as usize].zone(4).map(|z| (f32::from_bits(z.radius_bits), z.flags)), Some((20.0, 0x10)));
    assert!(e.in_city_zone(aa_play::zones::Key::Actor(b), c) == false);
}
