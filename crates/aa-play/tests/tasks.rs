//! The HUD orders on the ability and builder ports (`combat::tasks`) on small
//! synthetic worlds: bombard (0x5207F0 / 0x5353E0), repair (0x5359D0) and
//! construct (0x51DB80 -> 0x57ACC0 -> BuildActor 0x563160).
use aa_play::combat::tasks::BOMBARD_FLAG;
use aa_play::data::{DefClass, Formation, Weapon, WeaponEffect};
use aa_play::{AbilityTarget, Command, Controller, Def, Event, GameData, GameMap, Kingdom, Order, Unit, World};
use glam::{Vec2, Vec3};
use std::sync::Arc;

struct Ids {
    soldier: u32,
    gunner: u32,
    battery: u32,
    bunker: u32,
    engineers: u32,
    depot: u32,
}

fn components(names: &[&str]) -> u32 {
    names.iter().map(|n| aa_rules::native::COMPONENTS.iter().find(|(c, _)| c == n).map(|(_, b)| 1u32 << b).unwrap()).fold(0, |a, b| a | b)
}

/// A bombard ability (flag bit 3, MAIN_DAMAGE effect bit 15) with an area.
fn shell(range: f32, min_range: f32) -> Weapon {
    Weapon {
        name: "shell".into(),
        range,
        min_range,
        recharge: 2.0,
        damage: 30.0,
        area: 5.0,
        flags: BOMBARD_FLAG | (1 << 15),
        turret: -1,
        use_weight: 1.0,
        priority: 1.0,
        ..Default::default()
    }
}

fn data() -> (GameData, Ids) {
    let mut data = GameData { damage_types: vec!["gunfire".into()], ..Default::default() };
    let mut push = |d: Def| -> u32 {
        data.by_id.insert(d.id.clone(), data.defs.len() as u32);
        data.defs.push(d);
        data.defs.len() as u32 - 1
    };
    let soldier_def = Def {
        id: "soldier".into(),
        class: Some(DefClass::Member),
        health: 100.0,
        speed_max: 2.5,
        actor_data: true,
        destructible: true,
        components: components(&["BodyComponent", "ElementComponent", "MoverComponent"]),
        ..Default::default()
    };
    let soldier = push(soldier_def.clone());
    let gunner = push(Def { id: "gunner".into(), weapons: vec![shell(30.0, 0.0)], components: soldier_def.components | components(&["PerformerComponent"]), ..soldier_def });
    let battery = push(Def {
        id: "battery".into(),
        class: Some(DefClass::Organization),
        actor_data: true,
        components: components(&["OrganizationComponent"]),
        leader_slot: Some(0),
        slots: vec![(gunner, [0.0, 0.0]), (gunner, [-3.0, 0.0]), (gunner, [3.0, 0.0])],
        slot_entries: vec![0, 1, 1],
        formations: vec![Formation { id: "line".into(), positions: vec![[0.0, 0.0], [-3.0, 0.0], [3.0, 0.0]] }],
        default_formation: Some(0),
        control_range: 20.0,
        operational_range: 40.0,
        ..Default::default()
    });
    let bunker = push(Def {
        id: "bunker".into(),
        class: Some(DefClass::Building),
        health: 1000.0,
        actor_data: true,
        weapons: vec![shell(25.0, 6.0)],
        components: components(&["BodyComponent", "PerformerComponent"]),
        ..Default::default()
    });
    // An engineer's repair ability: HEAL 5, range 1 (effect bit 14).
    let repair = Weapon {
        name: "repair".into(),
        range: 1.0,
        recharge: 1.0,
        flags: 1 << 14,
        turret: -1,
        use_weight: 1.0,
        priority: 1.0,
        effects: vec![WeaponEffect { kind: 3, kind_name: "HEAL".into(), amount: 5.0, ..Default::default() }],
        ..Default::default()
    };
    let engineer = push(Def {
        id: "engineer".into(),
        class: Some(DefClass::Member),
        health: 100.0,
        speed_max: 2.5,
        actor_data: true,
        weapons: vec![repair],
        components: components(&["BodyComponent", "ElementComponent", "MoverComponent", "PerformerComponent"]),
        ..Default::default()
    });
    let engineers = push(Def {
        id: "engineers".into(),
        class: Some(DefClass::Organization),
        actor_data: true,
        components: components(&["OrganizationComponent"]),
        leader_slot: Some(0),
        slots: vec![(engineer, [0.0, 0.0]), (engineer, [-2.0, 0.0])],
        slot_entries: vec![0, 1],
        formations: vec![Formation { id: "line".into(), positions: vec![[0.0, 0.0], [-2.0, 0.0]] }],
        default_formation: Some(0),
        control_range: 20.0,
        operational_range: 40.0,
        ..Default::default()
    });
    let depot = push(Def {
        id: "depot".into(),
        class: Some(DefClass::Building),
        health: 200.0,
        actor_data: true,
        blocking_size: Some([4.0, 4.0]),
        components: components(&["BodyComponent", "ConstructionComponent"]),
        ..Default::default()
    });
    data.defs[engineer as usize].builds = vec![depot];
    (data, Ids { soldier, gunner, battery, bunker, engineers, depot })
}

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

fn world() -> (World, Ids) {
    let (data, ids) = data();
    let mut w = World::empty(Arc::new(data), Arc::new(map()), 7);
    for (i, team) in [0u32, 1].into_iter().enumerate() {
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

fn refused(w: &World) -> Option<String> {
    w.events.iter().find_map(|e| if let Event::Refused { reason, .. } = e { Some(reason.clone()) } else { None })
}

#[test]
fn bombard_approaches_until_in_range_then_shells_the_point() {
    let (mut w, ids) = world();
    let battery = aa_play::production::spawn_organization(&mut w, ids.battery, 0, Vec2::new(40.0, 128.0), 0.0, None);
    let enemy = aa_play::production::spawn_single(&mut w, ids.soldier, Some(1), Vec2::new(140.0, 128.0), 0.0);
    let point = Vec2::new(140.0, 128.0);
    w.step(vec![Command::Bombard { kingdom: 0, units: vec![Unit::Org(battery)], target: AbilityTarget::Point(point) }]);
    assert_eq!(refused(&w), None);
    // Out of range: the battery moves toward the point (0x520440).
    for _ in 0..4 {
        w.step(vec![]);
    }
    assert!(matches!(w.orgs[battery as usize].order, Order::Move { to, .. } if to == point), "{:?}", w.orgs[battery as usize].order);
    let mut first_shot = None;
    for _ in 0..(16 * 90) {
        w.step(vec![]);
        if let Some(Event::FiredAt { shooter, at, .. }) = w.events.iter().find(|e| matches!(e, Event::FiredAt { .. })) {
            first_shot = Some((*shooter, *at));
            break;
        }
    }
    let (shooter, at) = first_shot.expect("the battery fires at the point");
    assert_eq!(at, point);
    let distance = w.actors[shooter as usize].pos.truncate().distance(point);
    assert!(distance <= 30.0 + 1e-3, "fired from {distance} m");
    // In range the battery stands (the move state is left).
    assert_eq!(w.orgs[battery as usize].order, Order::Idle);
    for _ in 0..(16 * 6) {
        w.step(vec![]);
    }
    assert!(w.actors[enemy as usize].health < 100.0, "the area effect hit the enemy at the point");
}

#[test]
fn bombard_refusals_and_a_new_order_ends_it() {
    let (mut w, ids) = world();
    let battery = aa_play::production::spawn_organization(&mut w, ids.battery, 0, Vec2::new(40.0, 128.0), 0.0, None);
    let friend = aa_play::production::spawn_single(&mut w, ids.soldier, Some(0), Vec2::new(60.0, 128.0), 0.0);
    let lone = aa_play::production::spawn_single(&mut w, ids.soldier, Some(0), Vec2::new(60.0, 140.0), 0.0);
    // A friendly actor is no bombard target; a unit without a bombard
    // ability cannot take the order.
    w.step(vec![Command::Bombard { kingdom: 0, units: vec![Unit::Org(battery)], target: AbilityTarget::Actor(friend) }]);
    assert!(refused(&w).is_some());
    w.step(vec![Command::Bombard { kingdom: 0, units: vec![Unit::Actor(lone)], target: AbilityTarget::Point(Vec2::new(80.0, 128.0)) }]);
    assert_eq!(refused(&w).as_deref(), Some("nothing selected can bombard"));
    // Bombarding, then moved elsewhere: the move replaces the bombard state.
    w.step(vec![Command::Bombard { kingdom: 0, units: vec![Unit::Org(battery)], target: AbilityTarget::Point(Vec2::new(200.0, 128.0)) }]);
    w.step(vec![]);
    assert!(w.combat.org(battery).unwrap().task.is_some());
    let to = Vec2::new(40.0, 60.0);
    w.step(vec![Command::Move { kingdom: 0, units: vec![Unit::Org(battery)], to, attack_move: false }]);
    w.step(vec![]);
    assert!(w.combat.org(battery).unwrap().task.is_none());
    assert!(matches!(w.orgs[battery as usize].order, Order::Move { to: t, .. } if t == to));
}

#[test]
fn an_immobile_bombarder_fires_only_in_range_and_outside_its_minimum() {
    let (mut w, ids) = world();
    let bunker = aa_play::production::spawn_single(&mut w, ids.bunker, Some(0), Vec2::new(100.0, 100.0), 0.0);
    // Out of range without a mover: the state ends (Mind +124).
    w.step(vec![Command::Bombard { kingdom: 0, units: vec![Unit::Actor(bunker)], target: AbilityTarget::Point(Vec2::new(160.0, 100.0)) }]);
    w.step(vec![]);
    assert!(w.combat.actor(bunker).unwrap().task.is_none());
    // Inside the minimum range: 0x4C8460 refuses (relation 2, Mind +144).
    w.step(vec![Command::Bombard { kingdom: 0, units: vec![Unit::Actor(bunker)], target: AbilityTarget::Point(Vec2::new(103.0, 100.0)) }]);
    w.step(vec![]);
    assert!(w.combat.actor(bunker).unwrap().task.is_none());
    // In range: it shells the point every recharge.
    let point = Vec2::new(120.0, 100.0);
    w.step(vec![Command::Bombard { kingdom: 0, units: vec![Unit::Actor(bunker)], target: AbilityTarget::Point(point) }]);
    let mut shots = 0;
    for _ in 0..(16 * 5) {
        w.step(vec![]);
        shots += w.events.iter().filter(|e| matches!(e, Event::FiredAt { at, .. } if *at == point)).count();
    }
    assert!((2..=3).contains(&shots), "{shots} shots in 5 s with a 2 s recharge");
    let _ = (ids.gunner, Vec3::ZERO);
}

#[test]
fn repair_heals_a_damaged_structure_until_whole() {
    let (mut w, ids) = world();
    let crew = aa_play::production::spawn_organization(&mut w, ids.engineers, 0, Vec2::new(60.0, 100.0), 0.0, None);
    let depot = aa_play::production::spawn_single(&mut w, ids.depot, Some(0), Vec2::new(100.0, 100.0), 0.0);
    w.actors[depot as usize].health = 150.0;
    w.step(vec![Command::Repair { kingdom: 0, units: vec![Unit::Org(crew)], target: depot }]);
    assert_eq!(refused(&w), None);
    let mut healed = 0.0;
    for _ in 0..(16 * 60) {
        w.step(vec![]);
        for e in &w.events {
            if let Event::Healed { target, amount, .. } = e {
                assert_eq!(*target, depot);
                healed += amount;
            }
        }
        if w.combat.org(crew).unwrap().task.is_none() {
            break;
        }
    }
    assert_eq!(w.actors[depot as usize].health, 200.0);
    assert!((healed - 50.0).abs() < 1e-3, "{healed}");
    // Whole: the state ended (Mind +168) and the crew rests.
    assert!(w.combat.org(crew).unwrap().task.is_none());
    assert_eq!(w.orgs[crew as usize].order, Order::Idle);
}

#[test]
fn repairing_a_structure_under_construction_finishes_it() {
    let (mut w, ids) = world();
    let crew = aa_play::production::spawn_organization(&mut w, ids.engineers, 0, Vec2::new(96.0, 100.0), 0.0, None);
    let depot = aa_play::production::spawn_single(&mut w, ids.depot, Some(0), Vec2::new(100.0, 100.0), 0.0);
    w.books.actor_mut(depot).constructing = true;
    w.actors[depot as usize].health = 1.0;
    w.step(vec![Command::Repair { kingdom: 0, units: vec![Unit::Org(crew)], target: depot }]);
    let mut finished = false;
    for _ in 0..(16 * 60) {
        w.step(vec![]);
        finished |= w.events.iter().any(|e| matches!(e, Event::ConstructionFinished { actor, .. } if *actor == depot));
        if finished {
            break;
        }
    }
    assert!(finished, "built up to its maximum health");
    assert!(!w.books.actor(depot).unwrap().constructing);
    // Nothing to repair on a whole body; a crew without a repair ability is refused.
    w.step(vec![]);
    w.step(vec![Command::Repair { kingdom: 0, units: vec![Unit::Org(crew)], target: depot }]);
    w.step(vec![]);
    assert!(w.combat.org(crew).unwrap().task.is_none());
    let battery = aa_play::production::spawn_organization(&mut w, ids.battery, 0, Vec2::new(40.0, 40.0), 0.0, None);
    w.actors[depot as usize].health = 10.0;
    w.step(vec![Command::Repair { kingdom: 0, units: vec![Unit::Org(battery)], target: depot }]);
    assert_eq!(refused(&w).as_deref(), Some("nothing selected can repair that"));
}

#[test]
fn construct_goes_to_the_point_places_the_structure_and_builds_it() {
    let (mut w, ids) = world();
    let crew = aa_play::production::spawn_organization(&mut w, ids.engineers, 0, Vec2::new(40.0, 100.0), 0.0, None);
    let at = Vec2::new(100.0, 100.0);
    w.step(vec![Command::Construct { kingdom: 0, units: vec![Unit::Org(crew)], def: ids.depot, at, facing: 0.0 }]);
    assert_eq!(refused(&w), None);
    let (mut placed, mut finished) = (None, false);
    for _ in 0..(16 * 120) {
        w.step(vec![]);
        for e in &w.events {
            match e {
                Event::ConstructionStarted { actor, .. } => placed = Some(*actor),
                Event::ConstructionFinished { actor, .. } if Some(*actor) == placed => finished = true,
                _ => {}
            }
        }
        if finished {
            break;
        }
    }
    let depot = placed.expect("BuildActor placed the depot");
    assert_eq!(w.actors[depot as usize].pos.truncate(), at);
    // Placed far from where the crew started: it walked within its zone first.
    assert!(finished, "the crew built it up to its maximum health");
    assert_eq!(w.actors[depot as usize].health, 200.0);
    assert!(!w.books.actor(depot).unwrap().constructing);
}

#[test]
fn construct_refusals() {
    let (mut w, ids) = world();
    let crew = aa_play::production::spawn_organization(&mut w, ids.engineers, 0, Vec2::new(100.0, 100.0), 0.0, None);
    let battery = aa_play::production::spawn_organization(&mut w, ids.battery, 0, Vec2::new(40.0, 40.0), 0.0, None);
    // Not a builder of it.
    w.step(vec![Command::Construct { kingdom: 0, units: vec![Unit::Org(battery)], def: ids.depot, at: Vec2::new(60.0, 40.0), facing: 0.0 }]);
    assert_eq!(refused(&w).as_deref(), Some("nothing selected can build that"));
    // Unaffordable at the placement: construct_error_affording, the state ends.
    let mut data = (*w.data).clone();
    data.resources = vec![aa_play::data::ResourceDef { id: "money".into(), kind: aa_play::data::ResourceKind::Stockpiled, purchase_cost: vec![0.0], limited_by: None, limited_provider: false }];
    data.defs[ids.depot as usize].has_economy = true;
    data.defs[ids.depot as usize].cost_all = vec![100.0];
    w.data = Arc::new(data);
    w.step(vec![Command::Construct { kingdom: 0, units: vec![Unit::Org(crew)], def: ids.depot, at: Vec2::new(104.0, 100.0), facing: 0.0 }]);
    let mut notice = None;
    for _ in 0..16 {
        w.step(vec![]);
        if let Some(n) = w.books.notices.iter().find(|n| matches!(n, aa_play::books::Notice::ConstructRefused { .. })) {
            notice = Some(n.clone());
        }
    }
    assert!(matches!(notice, Some(aa_play::books::Notice::ConstructRefused { reason: aa_play::books::ConstructRefusal::Unaffordable { .. }, .. })), "{notice:?}");
    assert!(w.combat.org(crew).unwrap().task.is_none());
    assert!(!w.events.iter().any(|e| matches!(e, Event::ConstructionStarted { .. })));
}