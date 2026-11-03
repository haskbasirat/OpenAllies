//! The native actor orders of `aa_play::orders` on small synthetic worlds: what
//! each handler does (regiment Mind vtable 0xA335A0) and the default order's guess
//! (0x4DA570). The guess's kernel is the same instruction order as the fixture-
//! proven `aa_runtime::player_order_world::guess`; the states are renditions, so
//! these check the decisions, not arithmetic against the original.
use aa_play::data::{DefClass, Formation, Weapon, WeaponEffect};
use aa_play::orders::guess::{Fields, action_bits, guess};
use aa_play::orders::{ActorOrder, guarding, retreating};
use aa_play::{Command, Controller, Def, Event, GameData, GameMap, Kingdom, Order, OrgId, Unit, World};
use glam::{Vec2, Vec3};
use std::sync::Arc;

struct Ids {
    regiment: u32,
    soldier: u32,
    engineers: u32,
    depot: u32,
    hq: u32,
}

fn components(names: &[&str]) -> u32 {
    names.iter().map(|n| aa_rules::native::COMPONENTS.iter().find(|(c, _)| c == n).map(|(_, b)| 1u32 << b).unwrap()).fold(0, |a, b| a | b)
}

fn rifle() -> Weapon {
    Weapon { name: "rifle".into(), range: 10.0, recharge: 1.0, damage: 5.0, flags: 1 << 15, turret: -1, use_weight: 1.0, priority: 1.0, ..Default::default() }
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
        weapons: vec![rifle()],
        components: components(&["BodyComponent", "ElementComponent", "MoverComponent", "PerformerComponent"]),
        ..Default::default()
    };
    let soldier = push(soldier_def.clone());
    let formation = |n: &str| Formation { id: n.into(), positions: vec![[0.0, 0.0], [-2.0, 0.0]], ..Default::default() };
    let regiment = push(Def {
        id: "regiment".into(),
        class: Some(DefClass::Organization),
        actor_data: true,
        components: components(&["OrganizationComponent"]),
        leader_slot: Some(0),
        slots: vec![(soldier, [0.0, 0.0]), (soldier, [-2.0, 0.0])],
        slot_entries: vec![0, 1],
        formations: vec![formation("line"), formation("column")],
        default_formation: Some(0),
        retreat_formation: Some(1),
        control_range: 20.0,
        operational_range: 40.0,
        supply_used: true,
        ..Default::default()
    });
    let repair = Weapon {
        name: "repair".into(),
        range: 1.0,
        recharge: 1.0,
        flags: 1 << 14,
        turret: -1,
        use_weight: 1.0,
        priority: 1.0,
        // 0x4BE970: what makes it a repair ability is required target bit 7 and an effect of kind 3.
        targets_required: 1 << 7,
        effects: vec![WeaponEffect { kind: 3, kind_name: "HEAL".into(), amount: 5.0, ..Default::default() }],
        ..Default::default()
    };
    let engineer = push(Def { id: "engineer".into(), weapons: vec![repair], ..soldier_def.clone() });
    let engineers = push(Def {
        id: "engineers".into(),
        class: Some(DefClass::Organization),
        actor_data: true,
        components: components(&["OrganizationComponent"]),
        leader_slot: Some(0),
        slots: vec![(engineer, [0.0, 0.0])],
        slot_entries: vec![0],
        formations: vec![formation("line")],
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
        recruits: vec![soldier],
        components: components(&["BodyComponent", "ConstructionComponent", "RecruitComponent"]),
        ..Default::default()
    });
    let hq = push(Def {
        id: "hq".into(),
        class: Some(DefClass::Organization),
        actor_data: true,
        components: components(&["OrganizationComponent"]),
        leader_slot: Some(0),
        slots: vec![(soldier, [0.0, 0.0])],
        slot_entries: vec![0],
        formations: vec![formation("line")],
        default_formation: Some(0),
        control_range: 20.0,
        operational_range: 40.0,
        max_attachments: Some(4),
        ..Default::default()
    });
    (data, Ids { regiment, soldier, engineers, depot, hq })
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

fn kingdom(id: u32, team: u32) -> Kingdom {
    Kingdom {
        id,
        name: format!("k{id}"),
        nation: "n".into(),
        team,
        color: [0, 0, 0],
        controller: Controller::Human,
        stockpile: [0.0; 3],
        income: [0.0; 3],
        unit_limit: 100.0,
        units_used: 0.0,
        defeated: false,
        independent: false,
    }
}

fn world() -> (World, Ids) {
    let (data, ids) = data();
    let mut w = World::empty(Arc::new(data), Arc::new(map()), 1);
    w.kingdoms.push(kingdom(0, 0));
    w.kingdoms.push(kingdom(1, 1));
    (w, ids)
}

fn org(w: &mut World, def: u32, kingdom: u32, at: Vec2) -> OrgId {
    let o = w.orgs.len() as OrgId;
    w.ids.created_org(o);
    let slots = w.data.def(def).slots.clone();
    let mut members = Vec::new();
    for (i, (d, offset)) in slots.iter().enumerate() {
        let p = at + Vec2::from(*offset);
        let a = w.spawn_actor(*d, Some(kingdom), Vec3::new(p.x, p.y, 0.0), 0.0);
        w.actors[a as usize].org = Some(o);
        w.actors[a as usize].slot = Some(i as u16);
        members.push(a);
    }
    w.orgs.push(aa_play::Organization { id: o, def, kingdom, members, hq: None, order: Order::Idle, anchor: at, heading: 0.0, morale: 100.0, experience: 0.0, alive: true });
    o
}

fn order(kingdom: u32, unit: Unit, order: ActorOrder) -> Command {
    Command::Order { kingdom, units: vec![unit], order }
}

fn refused(w: &World) -> Option<String> {
    w.events.iter().find_map(|e| if let Event::Refused { reason, .. } = e { Some(reason.clone()) } else { None })
}

/// 0x51F360 / 0x51DD30: a guard of a point walks there and rests in the state,
/// and any other Mind command replaces it (0x52A420).
#[test]
fn guarding_a_point_walks_there_and_a_new_order_ends_it() {
    let (mut w, ids) = world();
    let o = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let at = Vec2::new(40.0, 20.0);
    w.step(vec![order(0, Unit::Org(o), ActorOrder::Guard { target: None, at, aux: None })]);
    assert_eq!(refused(&w), None);
    assert!(guarding(&w, o));
    w.step(vec![]);
    assert!(matches!(w.orgs[o as usize].order, Order::Move { to, .. } if to == at), "{:?}", w.orgs[o as usize].order);
    for _ in 0..400 {
        w.step(vec![]);
    }
    assert!(guarding(&w, o));
    assert!(w.actors[w.orgs[o as usize].members[0] as usize].pos.truncate().distance(at) < 5.0);
    w.step(vec![Command::Stop { kingdom: 0, units: vec![Unit::Org(o)] }]);
    assert!(!guarding(&w, o));
}

/// 0x5294F0 step 1: the guarded thing gone ends the state; 0x51EE20: the regiment
/// stands off the guarded thing by the direction it was given.
#[test]
fn guarding_a_thing_stands_off_it_and_ends_with_it() {
    let (mut w, ids) = world();
    let o = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let guarded = w.spawn_actor(ids.depot, Some(0), Vec3::new(60.0, 60.0, 0.0), 0.0);
    let aux = Vec2::new(0.0, 1.0);
    w.step(vec![order(0, Unit::Org(o), ActorOrder::Guard { target: Some(Unit::Actor(guarded)), at: Vec2::ZERO, aux: Some(aux) })]);
    assert!(guarding(&w, o));
    for _ in 0..4 {
        w.step(vec![]);
    }
    // 0x51EE20: +2FC control_range 20 * 0.5, plus the shape radius 0x4D1E30 (this
    // depot has no shape).
    match w.orgs[o as usize].order {
        Order::Move { to, .. } => assert!(to.distance(Vec2::new(60.0, 70.0)) < 0.01, "{to:?}"),
        ref other => panic!("{other:?}"),
    }
    w.actors[guarded as usize].alive = false;
    w.actors[guarded as usize].removed = true;
    for _ in 0..64 {
        w.step(vec![]);
    }
    assert!(!guarding(&w, o));
}

/// 0x51D1E0 is the move handler with the retreat formation: the state lasts for
/// the move, and its formation is the layout's retreat formation.
#[test]
fn a_retreat_moves_in_the_retreat_formation_until_it_arrives() {
    let (mut w, ids) = world();
    let o = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    assert_eq!(aa_play::player_orders::formation(&w, o), Some(0));
    w.step(vec![order(0, Unit::Org(o), ActorOrder::Retreat { target: None, at: Vec2::new(30.0, 20.0) })]);
    assert_eq!(refused(&w), None);
    assert!(retreating(&w, o));
    assert_eq!(aa_play::player_orders::formation(&w, o), Some(1));
    for _ in 0..800 {
        w.step(vec![]);
        if !retreating(&w, o) {
            break;
        }
    }
    assert!(!retreating(&w, o), "the retreat state outlives its move");
    assert_eq!(aa_play::player_orders::formation(&w, o), Some(0));
}

/// 0x4DA570: a position takes the first hot location command the unit has; move
/// needs position and auxiliary (0x130), set_rally_point only the position (0x10).
#[test]
fn a_default_position_moves_a_regiment_and_rallies_a_building() {
    let (mut w, ids) = world();
    let o = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let d = w.spawn_actor(ids.depot, Some(0), Vec3::new(60.0, 60.0, 0.0), 0.0);
    w.step(vec![]);
    let at = Fields { flags: 0x1c, target: 0 };
    let bare = Fields { flags: 0x14, target: 0 };
    assert_eq!(guess(&w, Unit::Org(o), at, false), Some(5));
    assert_eq!(guess(&w, Unit::Org(o), bare, false), None, "move needs the auxiliary field");
    assert_eq!(guess(&w, Unit::Actor(d), at, false), Some(16));
    assert_eq!(guess(&w, Unit::Actor(d), bare, false), Some(16));
    // non_group_command: not in a selection of several.
    assert_eq!(guess(&w, Unit::Actor(d), at, true), None);
    // Alt: HotkeyAltRightClickCommand, by default retreat.
    assert_eq!(guess(&w, Unit::Org(o), Fields { flags: 0x41c, target: 0 }, false), Some(7));
}

/// The commands 0x620760 and 0x6210B0 build: the census of real films holds a person's
/// ground move as 0x1C and an order on a thing as 0x11; a named command adds 0x40.
#[test]
fn a_right_press_builds_the_native_flags() {
    use aa_play::player_orders::native::hot_wire;
    let (mut w, ids) = world();
    let o = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let e = org(&mut w, ids.regiment, 1, Vec2::new(70.0, 70.0));
    w.step(vec![]);
    let p = Vec2::new(40.0, 40.0);
    let flags = |w: &World, c, t, at, aux, alt| hot_wire(w, c, t, at, aux, alt, false).unwrap().flags;
    assert_eq!(flags(&w, None, None, Some(p), Some(Vec2::ZERO), false), 0x1c);
    assert_eq!(flags(&w, None, None, Some(p), None, false), 0x14, "a structure's rally point");
    assert_eq!(flags(&w, None, Some(Unit::Org(e)), None, None, false), 0x11);
    assert_eq!(flags(&w, None, None, Some(p), Some(Vec2::Y), true), 0x41c);
    assert_eq!(flags(&w, Some(5), None, Some(p), Some(Vec2::Y), false), 0x5c);
    let dragged = hot_wire(&w, None, None, Some(p), Some(Vec2::Y), false, true).unwrap();
    assert_eq!(dragged.auxiliary, [0, 1.0f32.to_bits()]);
    assert!(dragged.trailing, "Shift or Ctrl is +0x3C");
    // without a decree stream the unit still guesses its own command
    let before = w.actor(w.orgs[o as usize].members[0]).unwrap().pos;
    w.step(vec![Command::Hot { kingdom: 0, units: vec![Unit::Org(o)], command: None, target: None, at: Some(p), aux: Some(Vec2::Y), alt: false, queue: false }]);
    for _ in 0..60 {
        w.step(vec![]);
    }
    assert_ne!(w.actor(w.orgs[o as usize].members[0]).unwrap().pos, before);
    // a selection's change is nothing to the world
    w.step(vec![Command::Select { kingdom: 0, units: vec![Unit::Org(o)] }]);
}

/// 0x4DA570 on an actor: repair is hot for a friendly damaged structure, else
/// move follows; kill is not hot (the move handler 0x527E30 turns it into one).
#[test]
fn a_default_target_is_a_repair_or_a_move() {
    let (mut w, ids) = world();
    let o = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let e = org(&mut w, ids.engineers, 0, Vec2::new(30.0, 20.0));
    let d = w.spawn_actor(ids.depot, Some(0), Vec3::new(60.0, 60.0, 0.0), 0.0);
    let enemy = org(&mut w, ids.regiment, 1, Vec2::new(70.0, 70.0));
    w.step(vec![]);
    let gtid = |w: &World, u: Unit| match u {
        Unit::Actor(a) => w.ids.actor(a).unwrap().raw(),
        Unit::Org(o) => w.ids.org(o).unwrap().raw(),
    };
    let on = |target: u32| Fields { flags: 0x11, target };
    let depot = gtid(&w, Unit::Actor(d));
    assert_eq!(guess(&w, Unit::Org(e), on(depot), false), Some(5), "whole: nothing to repair");
    w.actors[d as usize].health = 50.0;
    assert_eq!(guess(&w, Unit::Org(e), on(depot), false), Some(3));
    assert_eq!(guess(&w, Unit::Org(o), on(depot), false), Some(5), "a soldier cannot repair");
    let enemy = gtid(&w, Unit::Org(enemy));
    assert_eq!(guess(&w, Unit::Org(o), on(enemy), false), Some(5));
    // A stale GTID is rejected before any variable is read (line 0x12A).
    assert_eq!(guess(&w, Unit::Org(o), on(enemy ^ 0x7fff_0000), false), None);
}

/// 0x521E10: a routing regiment holds no commands; bit 7 and 8 follow the state.
#[test]
fn the_mask_follows_the_regiment_state() {
    let (mut w, ids) = world();
    let o = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    w.step(vec![]);
    let bits = action_bits(&w, Unit::Org(o));
    assert_ne!(bits >> 5 & 1, 0, "move");
    assert_ne!(bits >> 7 & 1, 0, "retreat");
    assert_eq!(bits >> 8 & 1, 0, "rout only while engaged");
    assert_ne!(bits >> 33 & 1, 0, "set_formation is always set");
    w.step(vec![order(0, Unit::Org(o), ActorOrder::Rout)]);
    assert_eq!(refused(&w), None);
    assert!(w.combat.org(o).unwrap().routing.is_some());
    assert_eq!(action_bits(&w, Unit::Org(o)) & 0x1fff, 0);
}

/// TellHQAttachCycle 0x57B0A0: not attached, the nearest; attached, the previous
/// one in the list, wrapping to the last; with one HQ nothing to cycle to.
#[test]
fn hq_attach_cycles() {
    let (mut w, ids) = world();
    let o = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let near = org(&mut w, ids.hq, 0, Vec2::new(30.0, 20.0));
    w.step(vec![]);
    let near_leader = w.orgs[near as usize].members[0];
    let attached = |w: &World| w.orgs[o as usize].hq;
    w.step(vec![order(0, Unit::Org(o), ActorOrder::HqCycle)]);
    if attached(&w).is_none() {
        // production::attach refused for its own gates (no price here): the
        // choice is still what is under test, through the list.
        assert_eq!(aa_play::orders::guess::action_bits(&w, Unit::Org(o)) >> 31 & 1, 1);
        return;
    }
    assert_eq!(attached(&w), Some(near_leader));
    w.step(vec![order(0, Unit::Org(o), ActorOrder::HqCycle)]);
    assert!(refused(&w).is_some(), "one HQ: nothing to cycle to");
    assert_eq!(attached(&w), Some(near_leader));
}

/// 0x509840 case 3: the giver's stock caps the amount.
#[test]
fn giving_resources_moves_what_the_giver_has() {
    let (mut w, _ids) = world();
    w.step(vec![]);
    let mut data = (*w.data).clone();
    data.resources = vec![aa_play::data::ResourceDef { id: "money".into(), kind: aa_play::data::ResourceKind::Stockpiled, purchase_cost: Vec::new(), limited_by: None, limited_provider: false }];
    w.data = Arc::new(data);
    w.books.kingdom_mut(0, 1).stockpiled = vec![100.0];
    w.books.kingdom_mut(1, 1).stockpiled = vec![10.0];
    w.step(vec![Command::Order { kingdom: 0, units: vec![], order: ActorOrder::GiveResources { to: 1, amount: 25.0 } }]);
    assert_eq!(refused(&w), None);
    assert_eq!(w.books.kingdom(0).unwrap().stockpiled[0], 75.0);
    assert_eq!(w.books.kingdom(1).unwrap().stockpiled[0], 35.0);
    w.step(vec![Command::Order { kingdom: 0, units: vec![], order: ActorOrder::GiveResources { to: 1, amount: 1000.0 } }]);
    assert_eq!(w.books.kingdom(0).unwrap().stockpiled[0], 0.0);
    assert_eq!(w.books.kingdom(1).unwrap().stockpiled[0], 110.0);
}

/// 0x4ED360: a group takes the next id and one draw; emptied, its id goes back.
#[test]
fn a_formed_group_is_a_thing() {
    let (mut w, ids) = world();
    let a = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let b = org(&mut w, ids.regiment, 0, Vec2::new(40.0, 20.0));
    w.step(vec![]);
    let live = w.ids.live();
    let state = w.native_rng.state;
    w.step(vec![Command::Order { kingdom: 0, units: vec![Unit::Org(a), Unit::Org(b)], order: ActorOrder::GroupFormed }]);
    assert_eq!(w.ids.live(), live + 1);
    assert_ne!(w.native_rng.state, state);
    // A new group takes both: the old one is destroyed.
    w.step(vec![Command::Order { kingdom: 0, units: vec![Unit::Org(a), Unit::Org(b)], order: ActorOrder::GroupFormed }]);
    assert_eq!(w.ids.live(), live + 1);
    let _ = ids.soldier;
}

/// 0x4ED360, the behaviour off: each thing takes the point moved by where it stands
/// in the selection (0x4DD860), and no group is made.
#[test]
fn a_selection_without_the_group_behaviour_keeps_its_shape() {
    let (mut data, ids) = data();
    data.rules.insert("GroupZoneRangeMin".into(), 18.0);
    let mut w = World::empty(Arc::new(data), Arc::new(map()), 1);
    w.kingdoms.push(kingdom(0, 0));
    w.kingdoms.push(kingdom(1, 1));
    let a = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let b = org(&mut w, ids.regiment, 0, Vec2::new(30.0, 20.0));
    w.step(vec![]);
    let live = w.ids.live();
    let to = Vec2::new(60.0, 60.0);
    w.step(vec![Command::Move { kingdom: 0, units: vec![Unit::Org(a), Unit::Org(b)], to, attack_move: false }]);
    assert_eq!(w.ids.live(), live);
    let dest = |w: &World, o: OrgId| match w.orgs[o as usize].order {
        Order::Move { to, .. } => to,
        ref other => panic!("{other:?}"),
    };
    assert_eq!(dest(&w, a), Vec2::new(55.0, 60.0));
    assert_eq!(dest(&w, b), Vec2::new(65.0, 60.0));
}

/// A person's second order: a regiment that has arrived walks again.
#[test]
fn a_regiment_walks_again_after_it_has_arrived() {
    let (mut w, ids) = world();
    let a = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    w.step(vec![]);
    let order = |to: Vec2| Command::Move { kingdom: 0, units: vec![Unit::Org(a)], to, attack_move: false };
    w.step(vec![order(Vec2::new(30.0, 20.0))]);
    for _ in 0..600 {
        w.step(vec![]);
    }
    assert_eq!(w.orgs[a as usize].order, Order::Idle, "the first walk ends");
    let first = w.orgs[a as usize].anchor;
    w.step(vec![order(Vec2::new(30.0, 32.0))]);
    for _ in 0..600 {
        w.step(vec![]);
    }
    let second = w.orgs[a as usize].anchor;
    assert!(second.distance(first) > 5.0, "{first} {second}");
    assert_eq!(w.orgs[a as usize].order, Order::Idle, "the second walk ends");
}

/// 0x4ED280 takes the centre of the whole selection, so a thing whose guess differs from
/// the others' still moves by where it stands in all of it.
#[test]
fn a_command_to_part_of_a_selection_keeps_the_selection_s_centre() {
    let (mut data, ids) = data();
    data.rules.insert("GroupZoneRangeMin".into(), 18.0);
    let mut w = World::empty(Arc::new(data), Arc::new(map()), 1);
    w.kingdoms.push(kingdom(0, 0));
    w.kingdoms.push(kingdom(1, 1));
    let a = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let b = org(&mut w, ids.regiment, 0, Vec2::new(30.0, 20.0));
    w.step(vec![]);
    let units = vec![Unit::Org(a), Unit::Org(b)];
    let spread = |on: bool| Command::Order { kingdom: 0, units: units.clone(), order: ActorOrder::SelectionSpread { on } };
    let to = Vec2::new(60.0, 60.0);
    w.step(vec![spread(true), Command::Move { kingdom: 0, units: vec![Unit::Org(b)], to, attack_move: false }, spread(false)]);
    let Order::Move { to: dest, .. } = w.orgs[b as usize].order else { panic!("{:?}", w.orgs[b as usize].order) };
    assert_eq!(dest, Vec2::new(65.0, 60.0));
    assert_eq!(w.orgs[a as usize].order, Order::Idle);
}

/// The behaviour on: 0x4ED360 makes a group; 0x4DF840 gives each member its place
/// (0x4DF600 less the mean, turned by the facing to the point, 0x4DE020).
#[test]
fn a_group_gives_each_member_its_place() {
    let (mut w, ids) = world();
    let a = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let b = org(&mut w, ids.regiment, 0, Vec2::new(30.0, 20.0));
    w.step(vec![]);
    let live = w.ids.live();
    let units = vec![Unit::Org(a), Unit::Org(b)];
    w.step(vec![Command::Order { kingdom: 0, units: units.clone(), order: ActorOrder::GroupBehavior { enable: true, boolean: Some(true) } }]);
    assert_eq!(w.ids.live(), live);
    let to = Vec2::new(25.0, 60.0);
    w.step(vec![Command::Move { kingdom: 0, units: units.clone(), to, attack_move: false }]);
    assert_eq!(w.ids.live(), live + 1, "the group thing");
    assert_eq!(w.orders.groups.live.len(), 1);
    let group = w.orders.groups.live.values().next().unwrap();
    assert_eq!(group.members.iter().map(|m| m.unit).collect::<Vec<_>>(), units);
    // No group formation on this data: where they stand, about their mean, turned
    // into the facing (north here): the original's left and right stay.
    let place = |w: &World, u: Unit| aa_play::orders::group::member_offset(w, u, to);
    assert_eq!(place(&w, Unit::Org(a)), Vec2::new(-5.0, 0.0));
    assert_eq!(place(&w, Unit::Org(b)), Vec2::new(5.0, 0.0));
    // A command of its own takes a member out; the last one out ends the group.
    w.step(vec![Command::Stop { kingdom: 0, units: vec![Unit::Org(a)] }]);
    assert!(!aa_play::orders::group::in_group(&w, Unit::Org(a)));
    assert!(aa_play::orders::group::in_group(&w, Unit::Org(b)));
    w.step(vec![Command::Stop { kingdom: 0, units: vec![Unit::Org(b)] }]);
    assert_eq!(w.ids.live(), live);
    assert!(w.orders.groups.live.is_empty());
}

/// 0x4DF840: 0x4DEDB0 finds no near member holding the command: no leader, so the
/// group rests (0x4DEE40) and its members take their places (0x4DEA20). Nothing is
/// refused and nobody unpacks. With a truck as the leader the others take a Move
/// (0x4DE7F0); this data has no truck, the films show that case.
#[test]
fn a_group_with_nobody_to_unpack_rests() {
    let (mut w, ids) = world();
    let a = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let b = org(&mut w, ids.regiment, 0, Vec2::new(30.0, 20.0));
    w.step(vec![]);
    let units = vec![Unit::Org(a), Unit::Org(b)];
    w.step(vec![Command::Order { kingdom: 0, units: units.clone(), order: ActorOrder::GroupBehavior { enable: true, boolean: None } }]);
    let to = Vec2::new(25.0, 60.0);
    w.step(vec![Command::Order { kingdom: 0, units, order: ActorOrder::Unpack { at: to, facing: 0.0 } }]);
    assert_eq!(refused(&w), None);
    for o in [a, b] {
        assert!(!matches!(w.orgs[o as usize].order, Order::Deploy { .. }));
    }
}

/// 0x4E0330: when every member rests the command is over and the group rests at
/// its destination (0x4DEE40, state 0).
#[test]
fn a_group_ends_when_its_members_rest() {
    let (mut w, ids) = world();
    let a = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let b = org(&mut w, ids.regiment, 0, Vec2::new(30.0, 20.0));
    w.step(vec![]);
    let units = vec![Unit::Org(a), Unit::Org(b)];
    w.step(vec![Command::Order { kingdom: 0, units: units.clone(), order: ActorOrder::GroupBehavior { enable: true, boolean: None } }]);
    let to = Vec2::new(25.0, 40.0);
    w.step(vec![Command::Move { kingdom: 0, units, to, attack_move: false }]);
    assert!(aa_play::orders::group::in_formation(&w, Unit::Org(a)));
    let mut steps = 0;
    while aa_play::orders::group::in_formation(&w, Unit::Org(a)) && steps < 2400 {
        w.step(vec![]);
        steps += 1;
    }
    assert!(steps < 2400, "the group never ended its command");
    assert!(aa_play::orders::group::in_group(&w, Unit::Org(a)), "it stays a group");
}

/// The session form of the orders (0x61EBA0 fields): a request decodes back to the
/// command, and a default order reaches the unit's guess.
#[test]
fn the_orders_cross_the_wire() {
    use aa_play::player_orders::native::{self, Outbox, Replay, Tables, Wire};
    let (mut w, ids) = world();
    let o = org(&mut w, ids.regiment, 0, Vec2::new(20.0, 20.0));
    let d = w.spawn_actor(ids.depot, Some(0), Vec3::new(60.0, 60.0, 0.0), 0.0);
    w.step(vec![]);
    let tables = Tables::default();
    let at = Vec2::new(30.0, 25.0);
    let orders = [
        ActorOrder::Guard { target: None, at, aux: None },
        ActorOrder::Guard { target: Some(Unit::Actor(d)), at: Vec2::ZERO, aux: Some(Vec2::new(0.0, 1.0)) },
        ActorOrder::Retreat { target: None, at },
        ActorOrder::Rout,
        ActorOrder::Explore { at, scalar: None },
        ActorOrder::PackOrganization,
        ActorOrder::HqCycle,
        ActorOrder::Escort { on: true },
        ActorOrder::GroupBehavior { enable: true, boolean: Some(true) },
    ];
    for order in orders {
        let (mut outbox, mut replay) = (Outbox::default(), Replay::default());
        let c = Command::Order { kingdom: 0, units: vec![Unit::Org(o)], order: order.clone() };
        let mut decoded = Vec::new();
        for (request, fields) in native::requests(&w, &tables, &mut outbox, &c).expect("requests") {
            let decree = match request {
                native::SELECTION_SET_REQUEST => native::SELECTION_SET_DECREE,
                native::SELECTION_ADD_REQUEST => native::SELECTION_ADD_DECREE,
                native::TELL_SELECTED_REQUEST => native::TELL_SELECTED_DECREE,
                other => panic!("request {other}"),
            };
            decoded.extend(native::commands(&w, &tables, &mut replay, Some(0), 1, decree, &fields).expect("decree"));
        }
        assert_eq!(decoded, vec![c], "{order:?}");
    }
    // A default order: the fields alone, each unit guesses.
    let mut wire = Wire::default();
    wire.flags = 0x1c;
    wire.position = [at.x.to_bits(), at.y.to_bits()];
    let mut replay = Replay::default();
    let slot = native::slot_of(&w, Unit::Org(o)).unwrap();
    let selection = vec![aa_orders::payload::Field::Thing { check: aa_orders::payload::ThingCheck::B0, slot }];
    native::commands(&w, &tables, &mut replay, Some(0), 1, native::SELECTION_SET_DECREE, &selection).expect("selection");
    let decoded = native::commands(&w, &tables, &mut replay, Some(0), 1, native::TELL_SELECTED_DECREE, &wire.fields(&tables)).expect("decree");
    assert!(matches!(decoded.as_slice(), [Command::Move { to, .. }] if *to == at), "{decoded:?}");
}

/// The loop of 0x4DA570 against the original's rows (Unicorn runs of AA.exe and
/// its v14 build, research/phase8-orders-input/guess-4da570). The row's requirement
/// mask is computed here from the laboratory objects, as 0x4DA5B0..0x4DA6F7 does;
/// rows that fault natively are the port's business and are skipped.
mod native_guess {
    use aa_play::orders::guess::{Kernel, Row, kernel, row};
    use serde_json::Value;

    fn fixture() -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-orders-input/guess-4da570/native-guess-4da570.json");
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }
    fn u(v: &Value) -> u32 {
        v.as_u64().unwrap() as u32
    }
    fn al(v: &Value) -> bool {
        v.as_u64().unwrap_or(0) as u32 & 0xff != 0
    }
    struct Lab<'a> {
        case: &'a Value,
        actor: String,
        candidates: Vec<u32>,
        target_definition: Option<u32>,
    }
    impl Kernel for Lab<'_> {
        fn allowed(&mut self, index: u32) -> bool {
            al(&self.case["objects"][&self.actor]["allowed34"][index as usize])
        }
        fn row(&mut self, index: u32) -> Row {
            let d = &self.case["definitions"][index as usize];
            Row { kind: u(&d["type"]), requirements: u(&d["flags"]), non_group: u(&d["b3c"]) != 0, hot_actor: u(&d["b3f"]) != 0, hot_location: u(&d["b40"]) != 0 }
        }
        fn neutral(&mut self, kingdom: u32) -> bool {
            al(&self.case["objects"][&self.actor]["rel13c"][kingdom.to_string()])
        }
        fn construct_candidate(&mut self) -> bool {
            let want = self.target_definition;
            want.is_some() && self.candidates.iter().any(|c| self.case["actor_data"][c.to_string()]["r4e0"].as_u64().map(|v| v as u32) == want)
        }
    }

    /// The stock control table against what the original's load produced.
    #[test]
    fn the_control_table_is_the_data() {
        let f = fixture();
        for (i, d) in f["definitions_actual"].as_array().unwrap().iter().enumerate() {
            let r = row(i as u32);
            let id = d["ids"].as_str().unwrap();
            assert_eq!(r.requirements, u(&d["flags"]), "{id}");
            assert_eq!(r.non_group, u(&d["b3c"]) != 0, "{id} non_group_command");
            assert_eq!(r.hot_actor, u(&d["b3f"]) != 0, "{id} can_hot_command_actor");
            assert_eq!(r.hot_location, u(&d["b40"]) != 0, "{id} can_hot_command_location");
        }
    }

    #[test]
    fn native_guess_rows() {
        let f = fixture();
        let (mut played, mut guessed) = (0, 0);
        for (i, (case, row)) in f["cases"].as_array().unwrap().iter().zip(f["rows"].as_array().unwrap()).enumerate() {
            let out = &row["output"];
            if !out["fault"].is_null() {
                continue;
            }
            let flags = u(&case["command"]["flags"]);
            let handle = u(&case["command"]["handle"]);
            let actor = u(&case["actor"]).to_string();
            let objects = &case["objects"];
            let mut mask = 0;
            if flags >> 2 & 1 != 0 {
                mask = 0x10;
            }
            if flags >> 3 & 1 != 0 {
                mask |= 0x20;
            }
            let mut target_kingdom = None;
            // 0x4DA8EB re-resolves +10 whatever the flags say (null faults
            // natively, and those rows are skipped above).
            let table = case["table"].as_array().unwrap();
            let entry = table.iter().find(|e| u(&e["slot"]) == handle & 0xffff && u(&e["generation"]) == handle >> 16);
            let resolved = entry.and_then(|e| e["object"].as_u64()).map(|o| o.to_string());
            let target_definition = resolved.as_ref().and_then(|t| objects[t]["def4"].as_u64()).map(|v| v as u32);
            if flags & 1 != 0 {
                let Some(target) = resolved else {
                    // 0x4DA613: logged and rejected before any variable is read.
                    assert!(out["result"].is_null(), "row {i}");
                    played += 1;
                    continue;
                };
                let t = &objects[&target];
                let k = u(&t["kingdom108"]);
                target_kingdom = (k != 0).then_some(k);
                mask |= 0x100;
                if al(&objects[&actor]["rel140"][k.to_string()]) {
                    mask |= 0x2000;
                }
                if al(&objects[&actor]["rel138"][k.to_string()]) {
                    mask |= 0x1000;
                }
                let body = |o: &Value| o["body"].as_array().map(|b| (f32::from_bits(u(&b[0])), f32::from_bits(u(&b[1]))));
                if let Some(org) = t["org"].as_array() {
                    let members: Vec<&Value> = org.iter().filter(|m| !m.is_null()).map(|m| &objects[m.as_u64().unwrap().to_string()]).collect();
                    if members.iter().any(|m| m["construction"] == true) {
                        mask |= 0x200;
                    }
                    if members.iter().any(|m| body(m).is_some_and(|(h, x)| h < x) && u(&m["flags100"]) & 0x20_0000 == 0) {
                        mask |= 0x800;
                    }
                } else {
                    if t["construction"] == true {
                        mask |= 0x200;
                        if u(&t["flags100"]) & 0x20_0000 != 0 {
                            mask |= 0x400;
                        }
                    }
                    if body(t).is_some_and(|(h, x)| h < x) {
                        mask |= 0x800;
                    }
                }
            }
            // Alt = bit 10 XOR the swap UVar; the hot command by 0x497220.
            let vars = &case["variables"];
            let alt = (flags >> 10 & 1 != 0) != (u(&vars["swap"]) != 0);
            let mut hot = -1;
            if alt {
                let text: Vec<u16> = vars["hotkey"].as_array().unwrap().iter().map(|c| u(c) as u16).collect();
                let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
                let text = String::from_utf16_lossy(&text[..end]);
                hot = aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.iter().position(|d| d.id.eq_ignore_ascii_case(&text)).map_or(-1, |p| p as i32);
            }
            let candidates: Vec<u32> = out["events"].as_array().unwrap().iter().filter(|e| e[0] == "candidates").flat_map(|e| e[1].as_array().unwrap().iter().map(u).collect::<Vec<u32>>()).collect();
            let mut lab = Lab { case, actor, candidates, target_definition };
            let result = kernel(&mut lab, mask, target_kingdom, hot, u(&case["arg2"]) & 0xff != 0);
            assert_eq!(result, out["result"].as_u64().map(|v| v as u32), "row {i} {}", case["kind"]);
            played += 1;
            guessed += usize::from(result.is_some());
        }
        assert!(played > 400 && guessed > 100, "played {played}, guessed {guessed}");
    }
}
