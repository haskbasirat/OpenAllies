//! Core systems on small synthetic worlds (no original data needed): the
//! economy books, recruiting, deployment, supply, regeneration and victory.
use aa_play::books::{DeployKind, PauseReason};
use aa_play::data::{DefClass, Formation, Modifier, PropertyDef, ResearchItem, ResourceDef, ResourceKind};
use aa_play::{Command, Controller, Def, Event, GameData, GameMap, Kingdom, Order, World};
use glam::{Vec2, Vec3};
use std::sync::Arc;

const MONEY: usize = 0;
const AMMO: usize = 1;
const UL_PROVIDED: usize = 3;
const UL_CONSUMED: usize = 4;

fn resources() -> Vec<ResourceDef> {
    let r = |id: &str, kind, cost: f32, limited_by| ResourceDef {
        id: id.into(),
        kind,
        purchase_cost: vec![cost, 0.0, 0.0, 0.0, 0.0],
        limited_by,
        limited_provider: false,
    };
    let mut v = vec![
        r("money", ResourceKind::Stockpiled, 0.0, None),
        r("ammo", ResourceKind::Upkeep, 2.0, None),
        r("oil", ResourceKind::Upkeep, 3.0, None),
        r("unit_limit_provided", ResourceKind::Upkeep, 0.0, None),
        r("unit_limit_consumed", ResourceKind::LimitedConsumed, 0.0, Some(3)),
    ];
    v[3].limited_provider = true;
    v
}

fn vec5(values: &[(usize, f32)]) -> Vec<f32> {
    let mut v = vec![0.0; 5];
    for &(i, x) in values {
        v[i] = x;
    }
    v
}

struct Ids {
    hq: u32,
    regiment: u32,
    soldier: u32,
    captain: u32,
    truck: u32,
    depot: u32,
}

fn data() -> (GameData, Ids) {
    let mut data = GameData { resources: resources(), damage_types: vec!["gunfire".into()], ..Default::default() };
    data.rules.insert("EconomyLimitedResourceMax".into(), 20.0);
    data.rules.insert("EconomyStockpileResourceMax".into(), 99999.0);
    data.rules.insert("SupplyStockpileRechargeRate".into(), 1.0);
    data.rules.insert("ResourceStockpileDefault".into(), 500.0);
    data.rules.insert("TeamVictory".into(), 1.0);
    let components = |names: &[&str]| -> u32 {
        names.iter().map(|n| aa_rules::native::COMPONENTS.iter().find(|(c, _)| c == n).map(|(_, b)| 1u32 << b).unwrap()).fold(0, |a, b| a | b)
    };
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
        resupply_rate: 0.1,
        actor_data: true,
        components: components(&["BodyComponent", "ElementComponent", "MoverComponent"]),
        morale_value: 10.0,
        ..Default::default()
    };
    let soldier = push(soldier_def.clone());
    let captain = push(Def { id: "captain".into(), ..soldier_def });
    let regiment = push(Def {
        id: "regiment".into(),
        class: Some(DefClass::Organization),
        actor_data: true,
        components: components(&["OrganizationComponent", "EconomyComponent"]),
        has_economy: true,
        recruitable: 2,
        cost_all: vec5(&[(MONEY, 50.0)]),
        upkeep_all: vec5(&[(AMMO, 10.0), (UL_CONSUMED, 1.0)]),
        production_all: vec5(&[]),
        recruit_time: 15.0,
        supply_used: true,
        in_supply_detached: false,
        shortfall_damage_rate: 0.01,
        leader_slot: Some(2),
        slots: vec![(soldier, [-3.0, 2.5]), (soldier, [3.0, 2.5]), (captain, [0.0, 0.0])],
        slot_entries: vec![0, 0, 1],
        formations: vec![Formation { id: "defensive".into(), positions: vec![[-3.0, 2.5], [3.0, 2.5], [0.0, 0.0]] }],
        default_formation: Some(0),
        ..Default::default()
    });
    let hq = push(Def {
        id: "hq".into(),
        class: Some(DefClass::Building),
        health: 2000.0,
        actor_data: true,
        components: components(&["BodyComponent", "EconomyComponent", "RecruitComponent", "StructureComponent", "SupplyComponent"]),
        has_economy: true,
        cost_all: vec5(&[(MONEY, 250.0)]),
        upkeep_all: vec5(&[]),
        production_all: vec5(&[(MONEY, 60.0), (AMMO, 20.0), (UL_PROVIDED, 2.0)]),
        recruits: vec![regiment],
        has_supply: true,
        supply_range: 60.0,
        supply_relay: false,
        creation_offset: Some([0.0, 0.0]),
        deployment_offset: Some([-10.0, 0.0]),
        packing_length: 10.0,
        unpacking_length: 10.0,
        pack_health_min: 0.5,
        ..Default::default()
    });
    let depot = push(Def {
        id: "depot".into(),
        class: Some(DefClass::Building),
        health: 750.0,
        actor_data: true,
        // Its properties (depotprop) live in its PropertiesComponent, which
        // also holds the modifier context technologies apply through.
        components: components(&["BodyComponent", "EconomyComponent", "PropertiesComponent", "SupplyComponent"]),
        has_economy: true,
        cost_all: vec5(&[(MONEY, 50.0)]),
        upkeep_all: vec5(&[]),
        production_all: vec5(&[(AMMO, 5.0)]),
        supply_required: true,
        has_supply: true,
        supply_range: 50.0,
        supply_relay: true,
        supply_stockpile_max: 90.0,
        packing_length: 7.0,
        unpacking_length: 7.0,
        pack_health_min: 0.5,
        ..Default::default()
    });
    let truck = push(Def {
        id: "truck".into(),
        class: Some(DefClass::Truck),
        health: 500.0,
        speed_max: 3.0,
        actor_data: true,
        components: components(&["BodyComponent", "EconomyComponent", "MobileDeploymentComponent", "MoverComponent"]),
        has_economy: true,
        cost_all: vec5(&[(MONEY, 25.0)]),
        upkeep_all: vec5(&[]),
        production_all: vec5(&[]),
        recruitable: 1,
        unpacks_to: Some(depot),
        packed_orientation: [1.0, 0.0],
        ..Default::default()
    });
    data.defs[depot as usize].packs_to = Some(truck);
    data.defs[hq as usize].recruits.push(truck);
    // ActorData +17C (460310 / 460000) and the regiment's control_range: its
    // members come out of the HQ door (4FBF40).
    data.defs[regiment as usize].control_range = 18.0;
    for d in [regiment, soldier, captain, truck] {
        data.defs[d as usize].recruited_at = vec![hq];
    }
    // Research: a production technology for depots and a recruit-cost one.
    data.defs[hq as usize].properties = vec!["hqprop".into()];
    data.defs[depot as usize].properties = vec!["depotprop".into()];
    data.defs[depot as usize].tile_locked = true;
    data.defs[depot as usize].axis_locked = true;
    data.defs[depot as usize].blocking_size = Some([4.0, 4.0]);
    data.defs[hq as usize].blocking_size = Some([10.0, 10.0]);
    data.defs[hq as usize].research = vec![
        ResearchItem { property: "tech_ammo".into(), kind: "GLOBAL".into(), research_time: 5.0, cost: vec5(&[(MONEY, 40.0)]), ..Default::default() },
        ResearchItem { property: "tech_cheap".into(), kind: "GLOBAL".into(), research_time: 5.0, cost: vec5(&[(MONEY, 10.0)]), ..Default::default() },
    ];
    let tech = |id: &str, required: &str, m: Modifier| PropertyDef { id: id.into(), is_technology: true, required_properties: vec![required.into()], modifiers: vec![m], ..Default::default() };
    data.properties.insert("tech_ammo".into(), tech("tech_ammo", "depotprop", Modifier { kind: "RESOURCE_PRODUCTION".into(), production: vec5(&[(AMMO, 5.0)]), ..Default::default() }));
    data.properties.insert("tech_cheap".into(), tech("tech_cheap", "hqprop", Modifier { kind: "MULTIPLY".into(), attribute: Some("RECRUIT_COST".into()), amount: 0.2, ..Default::default() }));
    // An HQ organization (the HQ building as its captain) taking one attachment.
    let hq_org = data.defs.len() as u32;
    data.by_id.insert("hq_org".into(), hq_org);
    data.defs.push(Def {
        id: "hq_org".into(),
        class: Some(DefClass::Organization),
        actor_data: true,
        recruitable: 2,
        max_attachments: Some(1),
        leader_slot: Some(0),
        slots: vec![(hq, [0.0, 0.0])],
        ..Default::default()
    });
    (data, Ids { hq, regiment, soldier, captain, truck, depot })
}

/// A flat map with the native GMap border (the outermost tile ring BLOCKED,
/// as `map.rs` marks every real map): the exact ZoneSolver flood relies on it.
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
    let n = data.resources.len();
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
        w.books.kingdom_mut(i as u32, n).stockpiled[MONEY] = 500.0;
    }
    (w, ids)
}

/// One tick of the core systems only (the other owners' systems are not
/// needed by these tests), with the time advance of `World::step`.
fn core_step(w: &mut World, commands: Vec<Command>) {
    w.events.clear();
    w.books.notices.clear();
    for c in commands {
        w.apply(c);
    }
    aa_play::economy::tick(w);
    aa_play::production::tick(w);
    aa_play::deploy::tick(w);
    aa_play::supply::tick(w);
    aa_play::victory::tick(w);
    w.clock.advance_fragment();
    w.tick = u64::from(w.clock.tick);
    w.time = f32::from_bits(w.clock.end.0);
}

fn spawn(w: &mut World, def: u32, k: u32, at: Vec2, facing: f32) -> u32 {
    aa_play::production::spawn_single(w, def, Some(k), at, facing)
}

#[test]
fn economy_flows_per_minute_and_buys_upkeep_shortfalls() {
    let (mut w, ids) = world(&[0]);
    let hq = spawn(&mut w, ids.hq, 0, Vec2::new(100.0, 100.0), 0.0);
    let _ = hq;
    for _ in 0..16 {
        core_step(&mut w, vec![]);
    }
    // 60 money per minute -> 1 money per second.
    let money = w.books.kingdom(0).unwrap().stockpiled[MONEY];
    assert!((money - 501.0).abs() < 1e-3, "{money}");
    assert_eq!(w.kingdoms[0].income[0], 60.0);
    assert_eq!(w.kingdoms[0].unit_limit, 2.0);
    // A regiment needing 10 ammo with 20 produced: no charge. Three of them
    // need 30: the 10 ammo deficit costs 2 money each = 20 money per minute.
    for _ in 0..3 {
        aa_play::production::spawn_organization(&mut w, ids.regiment, 0, Vec2::new(100.0, 110.0), 0.0, Some(hq));
    }
    core_step(&mut w, vec![]);
    let k = w.books.kingdom(0).unwrap();
    assert_eq!(k.consumed[AMMO], 30.0);
    assert_eq!(k.consumed[MONEY], 20.0);
    assert_eq!(w.kingdoms[0].income[0], 40.0);
    assert_eq!(w.kingdoms[0].units_used, 3.0);
}

#[test]
fn running_out_of_money_sets_the_shortage_ratio() {
    let (mut w, ids) = world(&[0]);
    let hq = spawn(&mut w, ids.hq, 0, Vec2::new(100.0, 100.0), 0.0);
    w.actors[hq as usize].alive = false; // no production at all
    aa_play::production::spawn_organization(&mut w, ids.regiment, 0, Vec2::new(100.0, 110.0), 0.0, None);
    w.books.kingdom_mut(0, 5).stockpiled[MONEY] = 0.0;
    core_step(&mut w, vec![]);
    let k = w.books.kingdom(0).unwrap();
    // 10 ammo unpaid -> 20 money consumed, nothing provided: ratio 1.
    assert_eq!(k.consumed[MONEY], 20.0);
    assert!((k.shortage_ratio - 1.0).abs() < 1e-6, "{}", k.shortage_ratio);
    assert!(w.books.notices.iter().any(|n| matches!(n, aa_play::books::Notice::UpkeepShortage { percent: 100, .. })));
}

#[test]
fn recruit_pays_waits_and_spawns_members_in_their_slots() {
    let (mut w, ids) = world(&[0]);
    let hq = spawn(&mut w, ids.hq, 0, Vec2::new(100.0, 100.0), 0.0);
    core_step(&mut w, vec![Command::Recruit { kingdom: 0, building: hq, def: ids.regiment }]);
    assert!((w.books.kingdom(0).unwrap().stockpiled[MONEY] - 450.0).abs() < 0.1);
    assert_eq!(w.actors[hq as usize].queue.len(), 1);
    // Not a recruit of this building.
    core_step(&mut w, vec![Command::Recruit { kingdom: 0, building: hq, def: ids.soldier }]);
    assert!(w.events.iter().any(|e| matches!(e, Event::Refused { .. })));
    let mut done = None;
    for _ in 0..(16 * 16) {
        core_step(&mut w, vec![]);
        if let Some(Event::Recruited { org: Some(o), .. }) = w.events.iter().find(|e| matches!(e, Event::Recruited { .. })) {
            done = Some((*o, w.time));
            break;
        }
    }
    let (o, t) = done.expect("recruited");
    assert!((t - 15.0).abs() <= 2.0 * aa_play::DT, "recruited after {t}");
    let org = &w.orgs[o as usize];
    // 582E80: a supply_used regiment without recruit_full gets its leader
    // only; the organization stands at the deployment point (facing 0, +X:
    // offset (-10, 0) turns to (0, +10)).
    assert_eq!(org.members.len(), 1);
    assert_eq!(org.hq, None, "hq building is not in an HQ organization here");
    assert!((org.anchor - Vec2::new(100.0, 110.0)).length() < 1e-3, "{}", org.anchor);
    let leader = org.members[0];
    assert_eq!(w.actors[leader as usize].slot, Some(2));
    assert_eq!(w.actors[leader as usize].def, ids.captain);
    // 4FBF40: out of the HQ door (creation offset 0, 0).
    assert!((w.actors[leader as usize].pos.truncate() - Vec2::new(100.0, 100.0)).length() < 1e-3);
    // Attached to the HQ (as an HQ organization's recruit is) it is in supply;
    // regeneration 57B5A0 brings the rest one at a time:
    // resupply_rate 0.1 -> 10 s per soldier, each at the door again.
    w.orgs[o as usize].hq = Some(hq);
    let mut joined = Vec::new();
    for _ in 0..(16 * 25) {
        core_step(&mut w, vec![]);
        let n = w.orgs[o as usize].members.len();
        if n > joined.len() + 1 {
            joined.push(w.time);
            let m = *w.orgs[o as usize].members.last().unwrap();
            assert!((w.actors[m as usize].pos.truncate() - Vec2::new(100.0, 100.0)).length() < 1e-3, "at the door");
        }
    }
    assert_eq!(w.orgs[o as usize].members.len(), 3, "{joined:?}");
    assert!((joined[0] - t - 10.0).abs() < 0.5 && (joined[1] - joined[0] - 10.0).abs() < 0.5, "one every 10 s: {joined:?}");
}

#[test]
fn queue_waits_for_unit_limit_and_cancel_refunds() {
    let (mut w, ids) = world(&[0]);
    let hq = spawn(&mut w, ids.hq, 0, Vec2::new(100.0, 100.0), 0.0);
    // Unit limit 2: two regiments already fielded.
    for _ in 0..2 {
        aa_play::production::spawn_organization(&mut w, ids.regiment, 0, Vec2::new(50.0, 50.0), 0.0, None);
    }
    core_step(&mut w, vec![Command::Recruit { kingdom: 0, building: hq, def: ids.regiment }]);
    for _ in 0..(16 * 20) {
        core_step(&mut w, vec![]);
    }
    let q = &w.books.actor(hq).unwrap().queue;
    assert_eq!(q.items.len(), 1, "still waiting");
    assert_eq!(q.pause_reason, Some(PauseReason::LimitedResource));
    let before = w.books.kingdom(0).unwrap().stockpiled[MONEY];
    core_step(&mut w, vec![Command::CancelRecruit { kingdom: 0, building: hq, index: 0 }]);
    let after = w.books.kingdom(0).unwrap().stockpiled[MONEY];
    assert!(after - before > 49.0, "refund {before} -> {after}");
    assert!(w.actors[hq as usize].queue.is_empty());
}

#[test]
fn truck_unpacks_at_its_spot_and_packs_back() {
    let (mut w, ids) = world(&[0]);
    let truck = spawn(&mut w, ids.truck, 0, Vec2::new(60.0, 60.0), 0.0);
    core_step(&mut w, vec![Command::Deploy { kingdom: 0, truck, at: Vec2::new(60.0, 60.0), facing: 0.0 }]);
    let building = w.actors.iter().find(|a| a.def == ids.depot && !a.removed).map(|a| a.id).expect("depot");
    assert!(w.actors[truck as usize].removed);
    let d = w.books.actor(building).unwrap().deploy.clone().unwrap();
    assert_eq!(d.kind, DeployKind::Unpacking);
    assert!((d.done_at - d.started - 7.0).abs() < 1e-4);
    for _ in 0..(16 * 7 + 2) {
        core_step(&mut w, vec![]);
    }
    assert!(w.books.actor(building).unwrap().deploy.is_none());
    core_step(&mut w, vec![Command::Pack { kingdom: 0, building }]);
    for _ in 0..(16 * 7 + 2) {
        core_step(&mut w, vec![]);
    }
    assert!(w.actors[building as usize].removed);
    let back = w.actors.iter().filter(|a| a.def == ids.truck && !a.removed).count();
    assert_eq!(back, 1);
}

#[test]
fn relay_needs_a_source_and_drains_its_stockpile() {
    let (mut w, ids) = world(&[0]);
    let hq = spawn(&mut w, ids.hq, 0, Vec2::new(40.0, 40.0), 0.0);
    let near = spawn(&mut w, ids.depot, 0, Vec2::new(80.0, 40.0), 0.0);
    let far = spawn(&mut w, ids.depot, 0, Vec2::new(200.0, 200.0), 0.0);
    for _ in 0..32 {
        core_step(&mut w, vec![]);
    }
    assert!(!w.books.out_of_supply(hq));
    assert!(!w.books.out_of_supply(near), "inside the HQ zone");
    assert!(w.books.out_of_supply(far));
    assert!(w.books.actor(near).unwrap().supply_stock > 1.0);
    assert_eq!(w.books.actor(far).unwrap().supply_stock, 0.0);
    // An out-of-supply depot that requires supply produces as its truck (nothing).
    assert_eq!(w.books.kingdom(0).unwrap().raw_provided[AMMO], 20.0 + 5.0);
}

/// 4D0C90: a cut-off relay still counts as supplied (and produces) while its
/// stockpile lasts; empty, a supply_required depot produces as its truck
/// (56F960 via the +548 pack link), here nothing.
#[test]
fn cut_off_relay_produces_while_its_stockpile_lasts() {
    let (mut w, ids) = world(&[0]);
    let hq = spawn(&mut w, ids.hq, 0, Vec2::new(40.0, 40.0), 0.0);
    let depot = spawn(&mut w, ids.depot, 0, Vec2::new(80.0, 40.0), 0.0);
    for _ in 0..(16 * 3) {
        core_step(&mut w, vec![]);
    }
    assert_eq!(w.books.kingdom(0).unwrap().raw_provided[AMMO], 20.0 + 5.0);
    let stock = w.books.actor(depot).unwrap().supply_stock;
    assert!(stock > 2.0, "{stock}");
    w.actors[hq as usize].alive = false;
    core_step(&mut w, vec![]);
    core_step(&mut w, vec![]);
    assert!(w.books.out_of_supply(depot));
    assert_eq!(w.books.kingdom(0).unwrap().raw_provided[AMMO], 5.0, "stock left: the depot still produces");
    for _ in 0..(16 * 4) {
        core_step(&mut w, vec![]);
    }
    assert_eq!(w.books.actor(depot).unwrap().supply_stock, 0.0);
    assert_eq!(w.books.kingdom(0).unwrap().raw_provided[AMMO], 0.0, "empty: as its truck");
}

/// 58AE10 searches the zones over the relay's footprint tiles: a depot
/// (4 x 4) whose centre is 1 m outside the HQ zone (60 m) still links. The
/// zone is its ZoneSolver flood: 30 tiles of 2 m from the HQ's tile, so the
/// footprint of a depot at y 107.5 (tile rows 52..54) lies outside it.
#[test]
fn relay_links_through_its_footprint() {
    let (mut w, ids) = world(&[0]);
    let _hq = spawn(&mut w, ids.hq, 0, Vec2::new(40.0, 40.0), 0.0);
    let edge = spawn(&mut w, ids.depot, 0, Vec2::new(101.0, 40.0), 0.0);
    let out = spawn(&mut w, ids.depot, 0, Vec2::new(40.0, 107.5), 0.0);
    for _ in 0..4 {
        core_step(&mut w, vec![]);
    }
    assert!(!w.books.out_of_supply(edge), "footprint reaches into the zone");
    assert!(w.books.out_of_supply(out), "footprint outside the zone");
}

#[test]
fn attached_regiment_in_supply_heals_and_rebuilds_members() {
    let (mut w, ids) = world(&[0]);
    let hq = spawn(&mut w, ids.hq, 0, Vec2::new(100.0, 100.0), 0.0);
    let o = aa_play::production::spawn_organization(&mut w, ids.regiment, 0, Vec2::new(110.0, 100.0), 0.0, Some(hq));
    let first = w.orgs[o as usize].members[1];
    let gone = w.orgs[o as usize].members[2];
    w.actors[first as usize].health = 50.0;
    w.actors[gone as usize].alive = false;
    w.actors[gone as usize].removed = true;
    w.orgs[o as usize].members.retain(|&m| m != gone);
    core_step(&mut w, vec![]);
    assert!(!w.books.org(o).unwrap().out_of_supply);
    // 0.1 * 100 health per second, one slot per tick (17 ticks so far).
    for _ in 0..16 {
        core_step(&mut w, vec![]);
    }
    let h = w.actors[first as usize].health;
    assert!((h - (50.0 + 17.0 * 10.0 / 16.0)).abs() < 1e-3, "{h}");
    for _ in 0..(16 * 30) {
        core_step(&mut w, vec![]);
    }
    assert_eq!(w.orgs[o as usize].members.len(), 3, "the fallen soldier was rebuilt");
    // Detached (no HQ) regiments are out of supply.
    w.orgs[o as usize].hq = None;
    core_step(&mut w, vec![]);
    assert!(w.books.org(o).unwrap().out_of_supply);
}

#[test]
fn last_team_standing_wins() {
    let (mut w, ids) = world(&[0, 1]);
    let a = spawn(&mut w, ids.hq, 0, Vec2::new(40.0, 40.0), 0.0);
    let b = spawn(&mut w, ids.hq, 1, Vec2::new(200.0, 200.0), 0.0);
    let _ = a;
    for _ in 0..20 {
        core_step(&mut w, vec![]);
    }
    assert_eq!(w.winner, None);
    w.actors[b as usize].alive = false;
    core_step(&mut w, vec![]);
    assert!(w.kingdoms[1].defeated);
    assert_eq!(w.winner, Some(0));
    assert!(w.events.iter().any(|e| *e == Event::Victory { team: 0 }));
    let _ = Vec3::ZERO;
}

#[test]
fn research_takes_its_time_and_applies_technologies() {
    let (mut w, ids) = world(&[0]);
    let hq = spawn(&mut w, ids.hq, 0, Vec2::new(100.0, 100.0), 0.0);
    let _depot = spawn(&mut w, ids.depot, 0, Vec2::new(130.0, 100.0), 0.0);
    core_step(&mut w, vec![]);
    assert!(aa_play::production::research(&mut w, 0, hq, "nothing").is_err());
    aa_play::production::research(&mut w, 0, hq, "tech_ammo").unwrap();
    assert!(aa_play::production::research(&mut w, 0, hq, "tech_ammo").is_err(), "already being researched");
    assert!((w.books.kingdom(0).unwrap().stockpiled[MONEY] - (500.0 + 1.0 / 16.0 - 40.0)).abs() < 0.01);
    for _ in 0..(16 * 5 + 2) {
        core_step(&mut w, vec![]);
    }
    assert_eq!(w.books.kingdom(0).unwrap().technologies, vec!["tech_ammo".to_string()]);
    core_step(&mut w, vec![]);
    // The depot (in the HQ zone) now produces 5 + 5 ammo.
    assert_eq!(w.books.kingdom(0).unwrap().raw_provided[AMMO], 20.0 + 5.0 + 5.0);
    // A recruit-cost technology makes recruits 20% cheaper (cost x (1 - 0.2)).
    aa_play::production::research(&mut w, 0, hq, "tech_cheap").unwrap();
    for _ in 0..(16 * 5 + 2) {
        core_step(&mut w, vec![]);
    }
    assert_eq!(aa_play::production::recruit_cost(&w, hq, ids.regiment)[MONEY], 40.0);
}

#[test]
fn regiments_attach_to_an_hq_with_a_free_slot() {
    let (mut w, ids) = world(&[0]);
    let hq_org = w.data.find("hq_org").unwrap();
    let ho = aa_play::production::spawn_organization(&mut w, hq_org, 0, Vec2::new(100.0, 100.0), 0.0, None);
    let building = w.orgs[ho as usize].members[0];
    assert_eq!(w.actors[building as usize].def, ids.hq);
    let a = aa_play::production::spawn_organization(&mut w, ids.regiment, 0, Vec2::new(120.0, 100.0), 0.0, None);
    let b = aa_play::production::spawn_organization(&mut w, ids.regiment, 0, Vec2::new(140.0, 100.0), 0.0, None);
    aa_play::production::attach(&mut w, 0, a, building).unwrap();
    assert_eq!(w.orgs[a as usize].hq, Some(building));
    assert!(aa_play::production::attach(&mut w, 0, b, building).is_err(), "max_attachments 1");
    assert!(aa_play::production::attach(&mut w, 0, ho, building).is_err(), "an HQ does not attach");
    // A recruit at the full HQ is refused too.
    core_step(&mut w, vec![Command::Recruit { kingdom: 0, building, def: ids.regiment }]);
    assert!(w.events.iter().any(|e| matches!(e, Event::Refused { .. })));
}

#[test]
fn deploy_spot_snaps_and_must_be_clear() {
    let (mut w, ids) = world(&[0]);
    let _hq = spawn(&mut w, ids.hq, 0, Vec2::new(100.0, 100.0), 0.0);
    let truck = spawn(&mut w, ids.truck, 0, Vec2::new(60.0, 60.0), 0.0);
    // Onto the HQ footprint: refused.
    core_step(&mut w, vec![Command::Deploy { kingdom: 0, truck, at: Vec2::new(103.0, 100.0), facing: 0.0 }]);
    assert!(w.events.iter().any(|e| matches!(e, Event::Refused { .. })));
    // tile_locked snaps to the half-cell grid, axis_locked faces native -Y.
    core_step(&mut w, vec![Command::Deploy { kingdom: 0, truck, at: Vec2::new(60.3, 60.7), facing: 0.0 }]);
    // The truck stood within a cell of the spot, so it unpacked at once.
    let depot = w.actors.iter().find(|a| a.def == ids.depot && !a.removed).expect("unpacked");
    assert_eq!(depot.pos.truncate(), Vec2::new(60.0, 61.0));
    assert!((depot.facing + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
}

#[test]
fn cities_change_hands_by_presence_and_independents_never_lose() {
    let (mut w, ids) = world(&[0, 1]);
    // A city of an independent kingdom (its own team, enemy of both players).
    let mut city = w.data.def(ids.depot).clone();
    city.id = "city".into();
    city.city = true;
    city.city_zone_radius = 30.0;
    city.captureable = true;
    city.destructible = false;
    city.supply_relay = false;
    city.production_all = vec5(&[(MONEY, 10.0)]);
    let mut data = (*w.data).clone();
    let city_def = data.defs.len() as u32;
    data.by_id.insert("city".into(), city_def);
    data.defs.push(city);
    w.data = Arc::new(data);
    w.kingdoms.push(Kingdom {
        id: 2,
        name: "independent_2".into(),
        nation: String::new(),
        team: aa_play::setup::INDEPENDENT_TEAM | 2,
        color: [128, 128, 128],
        controller: Controller::Independent,
        stockpile: [0.0; 3],
        income: [0.0; 3],
        unit_limit: 0.0,
        units_used: 0.0,
        defeated: false,
        independent: true,
    });
    use aa_play::books::{ENEMY, FRIEND, SELF};
    w.books.relations = vec![vec![SELF, ENEMY, ENEMY], vec![ENEMY, SELF, ENEMY], vec![ENEMY, ENEMY, SELF]];
    let _ = FRIEND;
    let c = spawn(&mut w, city_def, 2, Vec2::new(200.0, 200.0), 0.0);
    let hq0 = spawn(&mut w, ids.hq, 0, Vec2::new(40.0, 40.0), 0.0);
    let _hq1 = spawn(&mut w, ids.hq, 1, Vec2::new(300.0, 40.0), 0.0);
    for _ in 0..32 {
        core_step(&mut w, vec![]);
    }
    assert_eq!(w.actors[c as usize].kingdom, Some(2), "nobody near the city");
    assert!(!w.kingdoms[2].defeated, "an independent never loses");
    assert_eq!(w.winner, None);
    // A lone soldier with a weapon (PerformerComponent, combat value > 0) of
    // kingdom 0 in the zone takes the city; an unarmed one does not (5640A0
    // needs actor value 4D01E0 > 0).
    let mut soldier = w.data.def(ids.soldier).clone();
    soldier.weapons = vec![aa_play::data::Weapon { range: 10.0, damage: 1.0, ..Default::default() }];
    soldier.destructible = true;
    soldier.components |= aa_rules::native::COMPONENTS.iter().find(|(c, _)| *c == "PerformerComponent").map_or(0, |(_, bit)| 1u32 << bit);
    let mut unarmed_def = soldier.clone();
    unarmed_def.weapons.clear();
    let mut data = (*w.data).clone();
    let armed = data.defs.len() as u32;
    data.defs.push(soldier);
    let unarmed = data.defs.len() as u32;
    data.defs.push(unarmed_def);
    // A regiment of armed soldiers: it counts as one candidate (its members
    // are skipped, 4549C0) with its own value (578C00).
    let mut squad = w.data.def(ids.regiment).clone();
    squad.slots = vec![(armed, [-3.0, 2.5]), (armed, [3.0, 2.5]), (armed, [0.0, 0.0])];
    let squad_def = data.defs.len() as u32;
    data.defs.push(squad);
    w.data = Arc::new(data);
    // Combat prepares the combat values (Performer +60) of the new definitions.
    aa_play::combat::tick(&mut w);
    assert!(w.combat.extras.as_ref().unwrap().def(armed).combat_value > 0.0);
    let u = spawn(&mut w, unarmed, 0, Vec2::new(205.0, 200.0), 0.0);
    core_step(&mut w, vec![]);
    assert_eq!(w.actors[c as usize].kingdom, Some(2), "unarmed: no value");
    w.actors[u as usize].alive = false;
    let a = spawn(&mut w, armed, 0, Vec2::new(210.0, 200.0), 0.0);
    core_step(&mut w, vec![]);
    assert_eq!(w.actors[c as usize].kingdom, Some(0));
    assert!(w.events.iter().any(|e| matches!(e, Event::Captured { kingdom: 0, .. })));
    // An enemy in the zone too: contested, nobody takes it back.
    let b = spawn(&mut w, armed, 1, Vec2::new(190.0, 200.0), 0.0);
    core_step(&mut w, vec![]);
    assert_eq!(w.actors[c as usize].kingdom, Some(0));
    // Kingdom 0 leaves: kingdom 1 takes it.
    w.actors[a as usize].alive = false;
    core_step(&mut w, vec![]);
    assert_eq!(w.actors[c as usize].kingdom, Some(1));
    // Kingdom 1 leaves and a regiment of kingdom 0 arrives: it takes the city.
    w.actors[b as usize].alive = false;
    let o = aa_play::production::spawn_organization(&mut w, squad_def, 0, Vec2::new(205.0, 205.0), 0.0, Some(hq0));
    assert!(w.orgs[o as usize].members.iter().all(|&m| w.actors[m as usize].org == Some(o)));
    core_step(&mut w, vec![]);
    assert_eq!(w.actors[c as usize].kingdom, Some(0), "the regiment counts through its organization");
}

#[test]
fn handicaps_change_recruit_cost_and_health_the_original_way() {
    let (mut w, ids) = world(&[0]);
    let mut data = (*w.data).clone();
    let m = |attribute: &str, amount: f32| Modifier { kind: "MULTIPLY".into(), attribute: Some(attribute.into()), amount, ..Default::default() };
    data.properties.insert("property_handicap_easy".into(), PropertyDef { id: "property_handicap_easy".into(), is_technology: true, modifiers: vec![m("RECRUIT_COST", -0.25), m("health", -0.25)], ..Default::default() });
    w.data = Arc::new(data);
    let hq = spawn(&mut w, ids.hq, 0, Vec2::new(100.0, 100.0), 0.0);
    assert_eq!(aa_play::production::recruit_cost(&w, hq, ids.regiment)[MONEY], 50.0);
    w.books.kingdom_mut(0, 5).handicap = Some("property_handicap_easy".into());
    // 4FED70 negates RECRUIT_COST amounts: easy pays 25% more ("economy penalty").
    assert_eq!(aa_play::production::recruit_cost(&w, hq, ids.regiment)[MONEY], 62.5);
    let s = spawn(&mut w, ids.soldier, 0, Vec2::new(50.0, 50.0), 0.0);
    assert_eq!(w.actors[s as usize].health, 75.0);
}
