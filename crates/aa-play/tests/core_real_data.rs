//! Real-data checks of the core systems (ignored: need AA_GAME_ROOT, e.g.
//! `E:\Games\Axis and Alies`). Run with
//! `cargo test -p aa-play --test core_real_data -- --ignored --nocapture`.
use aa_play::{GameData, GameMap};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn root() -> PathBuf {
    PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"E:\Games\Axis and Alies".into()))
}

fn data() -> Arc<GameData> {
    static DATA: OnceLock<Arc<GameData>> = OnceLock::new();
    DATA.get_or_init(|| {
        let t = std::time::Instant::now();
        let d = GameData::load(&root()).expect("GameData::load");
        eprintln!("GameData::load {:?}", t.elapsed());
        Arc::new(d)
    })
    .clone()
}

fn show(data: &GameData, id: &str) {
    let Some(d) = data.find(id) else {
        eprintln!("{id}: MISSING");
        return;
    };
    let d = data.def(d);
    let names = |v: &[u32]| v.iter().map(|&i| data.def(i).id.clone()).collect::<Vec<_>>();
    eprintln!(
        "{id}: class={:?} nation={:?} invisible={} cat={} health={} defense={} armor={} ({}) speed={} accel={} sight={} cost={:?} upkeep={:?} prod={:?} ul+={} ul-={} recruit_time={} recruitable={} supply_required={} supply_range={} relay={} model={:?} icon={:?}",
        d.class, d.nation, d.invisible, d.category, d.health, d.defense, d.armor_type, data.armor_types.get(d.armor_type as usize).cloned().unwrap_or_default(),
        d.speed_max, d.acceleration, d.sight_range, d.cost_all, d.upkeep_all, d.production_all, d.unit_limit_provided, d.unit_limit_consumed,
        d.recruit_time, d.recruitable, d.supply_required, d.supply_range, d.supply_relay, d.model, d.icon
    );
    eprintln!("   recruits={:?}", names(&d.recruits));
    eprintln!(
        "   slots={:?} leader={:?} formations={:?} unpacks_to={:?} packs_to={:?} create={:?} deploy={:?} pack={} unpack={} props={:?} factor={:?}",
        d.slots.iter().map(|(s, o)| (if *s == u32::MAX { "null".to_string() } else { data.def(*s).id.clone() }, *o)).collect::<Vec<_>>(),
        d.leader_slot,
        d.formations.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
        d.unpacks_to.map(|i| data.def(i).id.clone()),
        d.packs_to.map(|i| data.def(i).id.clone()),
        d.creation_offset,
        d.deployment_offset,
        d.packing_length,
        d.unpacking_length,
        d.properties,
        d.damage_factor
    );
    for w in &d.weapons {
        eprintln!(
            "   weapon {} defender={} range={} min={} recharge={} (time {} reload {}) dmg={} type={} area={} max_targets={} flags={:#x} req={:#x} forb={:#x} anims={:?}",
            w.name, w.defender, w.range, w.min_range, w.recharge, w.recharge_time, w.reload_time, w.damage,
            data.damage_types.get(w.damage_type as usize).cloned().unwrap_or_default(), w.area, w.max_targets, w.flags, w.targets_required, w.targets_forbidden, w.animations
        );
    }
}

#[test]
#[ignore]
fn game_data_loads_with_original_values() {
    let data = data();
    let classes = |c| data.defs.iter().filter(|d| d.class == Some(c)).count();
    eprintln!(
        "defs {} actor_data {} buildings {} trucks {} orgs {} members {} other {} with weapons {} notes {}",
        data.defs.len(),
        data.defs.iter().filter(|d| d.actor_data).count(),
        classes(aa_play::DefClass::Building),
        classes(aa_play::DefClass::Truck),
        classes(aa_play::DefClass::Organization),
        classes(aa_play::DefClass::Member),
        classes(aa_play::DefClass::Other),
        data.defs.iter().filter(|d| !d.weapons.is_empty()).count(),
        data.load_notes.len()
    );
    eprintln!("resources {:?}", data.resources.iter().map(|r| (r.id.clone(), r.kind, r.purchase_cost.clone(), r.limited_by)).collect::<Vec<_>>());
    eprintln!("damage types {:?} armor types {:?}", data.damage_types, data.armor_types);
    eprintln!("nations {:?}", data.nations.iter().map(|n| (n.id.clone(), n.placeholders.iter().map(|(k, v)| (k.clone(), data.def(*v).id.clone())).collect::<Vec<_>>())).collect::<Vec<_>>());
    for rule in ["ResourceStockpileDefault", "EconomyLimitedResourceMax", "EconomyStockpileResourceMax", "SupplyStockpileRechargeRate", "CombatDamageMinimum", "MoraleSupplyBonus", "RegroupStealHealthRatio", "UnitSupplyEnabled", "SupplyProvideUnderSiege", "ModifierAttributeClampMultiply", "StartingExperience", "HQAttachCostPercentage"] {
        eprintln!("rule {rule} = {:?}", data.rule_ci(rule));
    }
    for n in data.load_notes.iter().take(40) {
        eprintln!("note: {n}");
    }
    for id in ["rus_corps", "rus_truck_corps", "regiment_rus_hq_infantry", "rus_truck_infantry", "rus_hq_infantry", "regiment_rus_infantry", "rus_infantry", "rus_infantry_mg", "rus_infantry_co", "rus_tank_medium", "regiment_rus_armor_medium", "rus_truck_supply", "rus_depot_supply", "rus_bunker", "ger_corps", "berlin_city_center", "western_europe_city_center01_small", "resource_ammo", "weurope_farm6x8_01", "placeholder_corpsunpacked", "start_loc"] {
        show(&data, id);
    }
    // Combat data: experience levels, death clips, ability weights, projectiles.
    let reg = data.def(data.find("regiment_rus_infantry").unwrap());
    for (n, l) in reg.experience_levels.iter().enumerate() {
        eprintln!("regiment_rus_infantry level {n}: required {} {:?}", l.required, l.modifiers.iter().map(|m| (m.kind.clone(), m.attribute.clone(), m.amount)).collect::<Vec<_>>());
    }
    assert!(reg.experience_levels.len() >= 2);
    assert_eq!(data.rule("regiment_rus_infantry.experience1.required"), Some(150.0));
    assert_eq!(data.rule("regiment_rus_infantry.experience1.AV"), Some(4.0));
    eprintln!("regiment fake_cv {}", reg.fake_cv_value);
    for id in ["rus_infantry", "rus_tank_medium", "ger_infantry", "rus_corps"] {
        let d = data.def(data.find(id).unwrap());
        eprintln!(
            "{id}: death {:?} clips {:?} primary_min_health {} ttd {} perf bonuses {} {} {} cv_bias {}",
            d.death_time,
            d.death_clips.iter().map(|c| (c.id.clone(), c.length, c.weight, c.damage_filter)).collect::<Vec<_>>(),
            d.primary_minimum_health,
            d.primary_time_to_death,
            d.performer_use_bonus,
            d.performer_wait_bonus,
            d.performer_move_bonus,
            d.cv_bias
        );
        for w in &d.weapons {
            eprintln!(
                "   {} priority {} weights use {} wait {} move {} bonuses {} {} {} projectile {:?} speed {} {:?}",
                w.name, w.priority, w.use_weight, w.wait_weight, w.move_weight, w.use_bonus, w.wait_bonus, w.move_bonus, w.projectile, w.projectile_speed, w.projectile_type
            );
        }
    }
    let missing_death = data.defs.iter().filter(|d| d.class == Some(aa_play::DefClass::Member) && d.death_time.is_none()).map(|d| d.id.clone()).collect::<Vec<_>>();
    eprintln!("members without a die clip: {} e.g. {:?}", missing_death.len(), &missing_death[..missing_death.len().min(12)]);
    for p in ["infantry_entrench", "vehicle_entrench", "tech_conscription", "tech_ammo_production"] {
        eprintln!("property {p}: {:?}", data.property(p).map(|d| (d.is_technology, d.required_properties.clone(), d.modifiers.iter().map(|m| (m.kind.clone(), m.attribute.clone(), m.amount, m.damage_type, m.production.clone())).collect::<Vec<_>>())));
    }
    eprintln!("state organization_entrenched {:?}", data.actor_states.get("organization_entrenched"));
    eprintln!("properties {} actor states {} rules {}", data.properties.len(), data.actor_states.len(), data.rules.len());
    eprintln!("rus_corps research {:?}", data.def(data.find("rus_corps").unwrap()).research.iter().map(|r| (r.property.clone(), r.kind.clone(), r.research_time, r.cost[0])).collect::<Vec<_>>());

    let mut movers = std::collections::BTreeMap::<String, Vec<String>>::new();
    for d in data.defs.iter().filter(|d| d.mover_class != aa_play::data::MoverClass::None) {
        movers.entry(format!("{:?}", d.mover_class)).or_default().push(d.id.clone());
    }
    for (k, v) in &movers {
        eprintln!("mover {k}: {} e.g. {:?}", v.len(), &v[..v.len().min(8)]);
    }
    assert_eq!(data.def(data.find("rus_battleship").unwrap()).mover_class, aa_play::data::MoverClass::Water);
    assert_eq!(data.def(data.find("rus_fighter").unwrap()).mover_class, aa_play::data::MoverClass::Air);
    assert_eq!(data.def(data.find("rus_infantry").unwrap()).mover_class, aa_play::data::MoverClass::Land);
    assert_eq!(data.def(data.find("rus_corps").unwrap()).mover_class, aa_play::data::MoverClass::None);

    let corps = data.def(data.find("rus_corps").unwrap());
    assert_eq!(corps.health, 3500.0);
    assert_eq!(corps.cost[aa_play::data::MONEY], 500.0);
    assert!(!corps.recruits.is_empty());
    assert_eq!(data.rule_ci("ResourceStockpileDefault"), Some(500.0));
}

#[test]
#[ignore]
fn maps_load() {
    let root = root();
    for id in ["(2v2)Berlin", "DeveloperMaps/(3v3)Firefight"] {
        let t = std::time::Instant::now();
        let map = GameMap::load(&root, id).unwrap_or_else(|e| panic!("{id}: {e:#}"));
        eprintln!(
            "{id}: {:?} {}x{} cell {} starts {:?} placed {} slots {:?}",
            t.elapsed(),
            map.width,
            map.height,
            map.cell_size,
            map.start_locations,
            map.placed.len(),
            map.slots
        );
        let blocked = map.flags.iter().filter(|f| **f & aa_play::map::BLOCKED != 0).count();
        let no_land = map.flags.iter().filter(|f| **f & aa_play::map::NOT_LAND_PASSABLE != 0).count();
        eprintln!("   blocked {blocked} not-land {no_land} capped {}", map.speed_limit.iter().filter(|s| **s < f32::MAX).count());
    }
}

fn berlin_world() -> aa_play::World {
    let data = data();
    let map = Arc::new(GameMap::load_with(&root(), "(2v2)Berlin", &data).expect("Berlin"));
    let setup = aa_play::SkirmishSetup {
        map_id: "(2v2)Berlin".into(),
        players: vec![
            aa_play::PlayerSetup { name: "Player".into(), nation: "rus".into(), team: 0, color: [0, 76, 255], controller: aa_play::Controller::Human, start: 0 },
            aa_play::PlayerSetup { name: "Computer".into(), nation: "ger".into(), team: 1, color: [217, 0, 25], controller: aa_play::Controller::Computer(1), start: 2 },
        ],
        starting_resources: None,
        seed: 1,
    };
    aa_play::World::new_skirmish(data, map, &setup).expect("new_skirmish")
}

/// The core systems alone, like `World::step` minus the other owners'
/// systems (so these checks do not depend on movement/combat/AI progress).
fn core_step(w: &mut aa_play::World, commands: Vec<aa_play::Command>) {
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
    w.tick += 1;
    w.time = w.tick as f32 * aa_play::DT;
}

fn recruited(w: &mut aa_play::World, seconds: u32) -> Option<(Option<u32>, Option<u32>)> {
    for _ in 0..(16 * seconds) {
        core_step(w, vec![]);
        if let Some(aa_play::Event::Recruited { org, actor, .. }) = w.events.iter().find(|e| matches!(e, aa_play::Event::Recruited { .. })) {
            return Some((*org, *actor));
        }
    }
    None
}

#[test]
#[ignore]
fn berlin_two_players_start_recruit_and_economy() {
    use aa_play::{Command, Event};
    let mut w = berlin_world();
    let data = w.data.clone();
    let rus_corps = data.find("rus_corps").unwrap();
    let ger_corps = data.find("ger_corps").unwrap();
    let live: Vec<_> = w.actors.iter().filter(|a| !a.removed).collect();
    eprintln!(
        "actors {} scenery {} kingdoms {:?}",
        live.len(),
        w.books.scenery.len(),
        w.kingdoms.iter().map(|k| (k.name.clone(), k.nation.clone(), k.stockpile, k.income, k.unit_limit)).collect::<Vec<_>>()
    );
    let corps: Vec<_> = live.iter().filter(|a| a.def == rus_corps || a.def == ger_corps).map(|a| (a.id, data.def(a.def).id.clone(), a.kingdom, a.pos)).collect();
    eprintln!("corps {corps:?}");
    assert_eq!(corps.len(), 2);
    let rus = corps.iter().find(|c| c.2 == Some(0)).expect("rus corps of kingdom 0").clone();
    assert_eq!(rus.1, "rus_corps");
    assert!((rus.3.truncate() - w.map.start_locations[0]).length() < 1.0);
    let ger = corps.iter().find(|c| c.2 == Some(1)).expect("ger corps of kingdom 1");
    assert_eq!(ger.1, "ger_corps");
    assert!((ger.3.truncate() - w.map.start_locations[2]).length() < 1.0);
    assert!(live.iter().all(|a| !data.def(a.def).id.starts_with("placeholder")), "placeholders replaced or removed");
    let neutral: std::collections::BTreeMap<String, usize> = live.iter().filter(|a| a.kingdom.is_none()).fold(Default::default(), |mut m, a| {
        *m.entry(data.def(a.def).id.clone()).or_default() += 1;
        m
    });
    eprintln!("neutral actors {neutral:?}");
    assert_eq!(w.kingdoms[0].stockpile[0], 500.0);
    // The independent kingdoms own the cities and the other neutral buildings.
    let independents: Vec<_> = w.kingdoms.iter().filter(|k| k.independent).map(|k| (k.id, k.name.clone(), k.team, k.color)).collect();
    eprintln!("independents {independents:?} relations {:?}", w.books.relations);
    assert_eq!(independents.len(), 2);
    let city = w.actors.iter().find(|a| data.def(a.def).id == "berlin_city_center").unwrap();
    let owner = city.kingdom.expect("cities have an owner");
    assert!(w.kingdoms[owner as usize].independent);
    eprintln!("player -> city owner relation {}", aa_play::setup::relation(&w, Some(0), Some(owner)));
    assert_eq!(w.actors.iter().filter(|a| a.kingdom.is_none() && !a.removed).count(), 0);

    // Economy over 60 s with nothing recruited.
    let t = std::time::Instant::now();
    let start = w.books.kingdom(0).unwrap().stockpiled[0];
    for _ in 0..(16 * 60) {
        core_step(&mut w, vec![]);
    }
    let per_tick = t.elapsed() / (16 * 60);
    let end = w.books.kingdom(0).unwrap().stockpiled[0];
    let k = w.books.kingdom(0).unwrap();
    eprintln!("60 s: money {start} -> {end}; provided {:?} consumed {:?} (core tick {per_tick:?})", k.provided, k.consumed);
    assert!((end - start - (k.provided[0] - k.consumed[0])).abs() < 0.05);

    for &r in &data.def(rus_corps).recruits {
        eprintln!(
            "corps recruit {}: cost {:?} time {} s",
            data.def(r).id,
            aa_play::production::quote(&w, r, aa_play::production::Vector::Cost)[0],
            aa_play::production::duration(&w, r)
        );
    }
    let rhq = data.find("rus_hq_infantry").unwrap();
    for &r in &data.def(rhq).recruits {
        eprintln!(
            "hq recruit {}: cost {:?} upkeep {:?} time {} s members {}",
            data.def(r).id,
            aa_play::production::quote(&w, r, aa_play::production::Vector::Cost)[0],
            aa_play::production::quote(&w, r, aa_play::production::Vector::Upkeep),
            aa_play::production::duration(&w, r),
            data.def(r).slots.iter().filter(|s| s.0 != u32::MAX).count()
        );
    }
    // Recruit the corps' first recruit (the infantry division HQ).
    let corps_id = rus.0;
    let first = data.def(rus_corps).recruits[0];
    let cost = aa_play::production::quote(&w, first, aa_play::production::Vector::Cost);
    eprintln!("recruiting {} cost {:?} time {}", data.def(first).id, cost, aa_play::production::duration(&w, first));
    let before = w.books.kingdom(0).unwrap().stockpiled[0];
    core_step(&mut w, vec![Command::Recruit { kingdom: 0, building: corps_id, def: first }]);
    assert!(!w.events.iter().any(|e| matches!(e, Event::Refused { .. })), "{:?}", w.events);
    assert!((before - w.books.kingdom(0).unwrap().stockpiled[0] - cost[0]).abs() < 1.0);
    let (org, _) = recruited(&mut w, 60).expect("division HQ recruited");
    let o = org.expect("an organization");
    eprintln!(
        "recruited at {} s: org {o} order {:?} members {:?}",
        w.time,
        w.orgs[o as usize].order,
        w.orgs[o as usize].members.iter().map(|&m| (data.def(w.actors[m as usize].def).id.clone(), w.actors[m as usize].slot, w.actors[m as usize].pos)).collect::<Vec<_>>()
    );
    let truck = w.orgs[o as usize].members[0];
    // Unpack the HQ truck 20 m south of where it stands.
    let at = w.actors[truck as usize].pos.truncate() + glam::Vec2::new(0.0, -20.0);
    w.actors[truck as usize].pos = w.ground(at);
    let facing = w.actors[truck as usize].facing;
    core_step(&mut w, vec![Command::Deploy { kingdom: 0, truck, at, facing }]);
    let mut hq = None;
    for _ in 0..(16 * 20) {
        core_step(&mut w, vec![]);
        if let Some(Event::Deployed { building }) = w.events.iter().find(|e| matches!(e, Event::Deployed { .. })) {
            hq = Some(*building);
            break;
        }
    }
    let hq = hq.expect("division HQ unpacked");
    eprintln!("unpacked {} at {} s, in supply {}", data.def(w.actors[hq as usize].def).id, w.time, !w.books.out_of_supply(hq));
    let regiment = data.find("regiment_rus_infantry").unwrap();
    for _ in 0..16 {
        core_step(&mut w, vec![]);
    }
    core_step(&mut w, vec![Command::Recruit { kingdom: 0, building: hq, def: regiment }]);
    assert!(!w.events.iter().any(|e| matches!(e, Event::Refused { .. })), "{:?}", w.events);
    let (org, _) = recruited(&mut w, 60).expect("regiment recruited");
    let o = org.expect("an organization");
    let r = &w.orgs[o as usize];
    eprintln!(
        "regiment at {} s: hq {:?} order {:?} members {:?}",
        w.time,
        r.hq,
        r.order,
        r.members.iter().map(|&m| (data.def(w.actors[m as usize].def).id.clone(), w.actors[m as usize].slot, w.actors[m as usize].pos)).collect::<Vec<_>>()
    );
    assert_eq!(r.members.len(), data.def(regiment).slots.len());
    assert_eq!(r.hq, Some(hq));
    for &m in &r.members {
        let a = &w.actors[m as usize];
        let offset = data.def(regiment).slots[a.slot.unwrap() as usize].1;
        let want = r.anchor + aa_play::production::rotate(offset, r.heading);
        assert!((a.pos.truncate() - want).length() < 1e-3, "member in its slot");
    }
    let k = w.books.kingdom(0).unwrap();
    eprintln!("after recruits: money {} income {:?} unit limit {}/{}", k.stockpiled[0], w.kingdoms[0].income, w.kingdoms[0].units_used, w.kingdoms[0].unit_limit);

    // Tick cost of the core systems with a large army (60 regiments, 540 men).
    for i in 0..60 {
        let at = glam::Vec2::new(60.0 + (i % 10) as f32 * 12.0, 300.0 + (i / 10) as f32 * 12.0);
        aa_play::production::spawn_organization(&mut w, regiment, (i % 2) as u32, at, 0.0, Some(hq));
    }
    let t = std::time::Instant::now();
    for _ in 0..160 {
        core_step(&mut w, vec![]);
    }
    eprintln!("core tick with {} actors / {} orgs: {:?}", w.actors.len(), w.orgs.len(), t.elapsed() / 160);
}

/// Recruit, walk out, deploy and unpack through the full `World::step`
/// (the movement owner drives the truck).
#[test]
#[ignore]
fn berlin_full_step_recruit_and_deploy() {
    use aa_play::{Command, Event};
    let mut w = berlin_world();
    let data = w.data.clone();
    let corps = w.actors.iter().find(|a| a.kingdom == Some(0) && data.def(a.def).id == "rus_corps").unwrap().id;
    let hq_def = data.find("regiment_rus_hq_infantry").unwrap();
    w.step(vec![Command::Recruit { kingdom: 0, building: corps, def: hq_def }]);
    let mut org = None;
    for _ in 0..(16 * 40) {
        w.step(vec![]);
        if let Some(Event::Recruited { org: o, .. }) = w.events.iter().find(|e| matches!(e, Event::Recruited { .. })) {
            org = *o;
        }
        if org.is_some() && w.time > 100.0 {
            break;
        }
    }
    let o = org.expect("recruited");
    let truck = w.orgs[o as usize].members[0];
    eprintln!("HQ truck at {:?} order {:?} (time {})", w.actors[truck as usize].pos, w.orgs[o as usize].order, w.time);
    let at = w.actors[truck as usize].pos.truncate() + glam::Vec2::new(0.0, -16.0);
    w.step(vec![Command::Deploy { kingdom: 0, truck, at, facing: -std::f32::consts::FRAC_PI_2 }]);
    eprintln!("deploy events {:?}", w.events.iter().filter(|e| matches!(e, Event::Refused { .. })).collect::<Vec<_>>());
    let mut deployed = None;
    for _ in 0..(16 * 60) {
        w.step(vec![]);
        if let Some(Event::Deployed { building }) = w.events.iter().find(|e| matches!(e, Event::Deployed { .. })) {
            deployed = Some(*building);
            break;
        }
    }
    let truck_pos = w.actors[truck as usize].pos;
    let hq = deployed.unwrap_or_else(|| panic!("not deployed: truck at {truck_pos:?}, removed {}", w.actors[truck as usize].removed));
    eprintln!("deployed {} at {:?} time {}", data.def(w.actors[hq as usize].def).id, w.actors[hq as usize].pos, w.time);
}

#[test]
#[ignore]
fn firefight_six_players_start() {
    let data = data();
    let map = Arc::new(GameMap::load_with(&root(), "DeveloperMaps/(3v3)Firefight", &data).expect("Firefight"));
    let nations = ["rus", "usa", "gbr", "ger", "jpn", "ger"];
    let players = (0..6)
        .map(|i| aa_play::PlayerSetup {
            name: format!("P{i}"),
            nation: nations[i].into(),
            team: if i < 3 { 0 } else { 1 },
            color: [0, 0, 0],
            controller: if i == 0 { aa_play::Controller::Human } else { aa_play::Controller::Computer(1) },
            start: i,
        })
        .collect();
    let setup = aa_play::SkirmishSetup { map_id: map.id.clone(), players, starting_resources: Some(1000.0), seed: 3 };
    let mut w = aa_play::World::new_skirmish(data.clone(), map, &setup).expect("new_skirmish");
    let owned: Vec<_> = w.actors.iter().filter(|a| a.kingdom.is_some()).map(|a| (a.kingdom.unwrap(), data.def(a.def).id.clone())).collect();
    eprintln!("owned {owned:?}");
    assert_eq!(owned.iter().filter(|(_, id)| id.ends_with("_corps")).count(), 6);
    assert_eq!(w.kingdoms[0].stockpile[0], 1000.0);
    for _ in 0..32 {
        w.step(vec![]);
    }
}

#[test]
#[ignore]
fn every_map_loads() {
    let data = data();
    let root = root();
    let index: Vec<aa_assets::MapInfo> = serde_json::from_slice(&std::fs::read(root.join(r"reconstruction\game\data\maps\index.json")).unwrap()).unwrap();
    let mut failed = Vec::new();
    for m in &index {
        match GameMap::load_with(&root, &m.id, &data) {
            Ok(map) => eprintln!("ok   {} starts {} slots {} notes {:?}", m.id, map.start_locations.len(), map.slots.len(), map.load_notes),
            Err(e) => {
                eprintln!("FAIL {}: {e:#}", m.id);
                failed.push(m.id.clone());
            }
        }
    }
    eprintln!("{} of {} maps failed: {failed:?}", failed.len(), index.len());
}

#[test]
#[ignore]
fn berlin_full_step_runs() {
    let mut w = berlin_world();
    let t = std::time::Instant::now();
    for _ in 0..(16 * 10) {
        w.step(vec![]);
    }
    eprintln!("World::step x160 {:?} per tick", t.elapsed() / 160);
}
