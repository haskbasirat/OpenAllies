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
            Ok(map) => {
                // owned placed things that provide a unit limit (thing_unit_resource, HQs...)
                let limits: Vec<&str> = map
                    .placed
                    .iter()
                    .filter(|p| p.owner_slot.is_some() && data.find(&p.thing_id).is_some_and(|d| data.def(d).unit_limit_provided > 0.0))
                    .map(|p| p.thing_id.as_str())
                    .collect();
                eprintln!("ok   {} starts {} slots {} unit-limit things {limits:?} notes {:?}", m.id, map.start_locations.len(), map.slots.len(), map.load_notes)
            }
            Err(e) => {
                eprintln!("FAIL {}: {e:#}", m.id);
                failed.push(m.id.clone());
            }
        }
    }
    eprintln!("{} of {} maps failed: {failed:?}", failed.len(), index.len());
}

/// Supply of the computer's (and an idle human's) HQs over a real game:
/// relays next to the Corps HQ must connect and fill their stockpile, their
/// regiments stay in supply; the limits are filled from the start.
#[test]
#[ignore]
fn berlin_supply_and_limits_with_the_computer() {
    let mut w = berlin_world();
    w.kingdoms[1].controller = aa_play::Controller::Computer(1);
    aa_play::ai::attach(&mut w, Arc::new(aa_play::ai::AiData::load(&root()).expect("AiData")));
    let data = w.data.clone();
    let idx = |id: &str| data.resource_index(id).unwrap();
    for k in 0..2u32 {
        let b = w.books.kingdom(k).unwrap();
        eprintln!(
            "start k{k}: units {}/{} buildings {}/{} (Kingdom {}/{})",
            b.consumed[idx("unit_limit_consumed")],
            b.provided[idx("unit_limit_provided")],
            b.consumed[idx("building_limit_consumed")],
            b.provided[idx("building_limit_provided")],
            w.kingdoms[k as usize].units_used,
            w.kingdoms[k as usize].unit_limit
        );
    }
    assert!(w.books.kingdom(0).unwrap().provided[idx("building_limit_provided")] > 0.0);
    assert!(w.books.kingdom(0).unwrap().consumed[idx("building_limit_consumed")] > 0.0);
    let mut bad = 0;
    for minute in 0..10 {
        for _ in 0..(16 * 60) {
            w.step(vec![]);
        }
        for a in w.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom.is_some_and(|k| k < 2) && data.def(a.def).has_supply) {
            let b = w.books.actor(a.id).unwrap();
            let corps = w.actors.iter().find(|c| c.kingdom == a.kingdom && data.def(c.def).id.ends_with("_corps") && !c.removed).map(|c| c.pos.truncate().distance(a.pos.truncate()));
            eprintln!(
                "t={minute}m k{:?} {} at {:.0},{:.0} corps {:?} m: out {} relay {:?} stock {:.1} zone {}",
                a.kingdom,
                data.def(a.def).id,
                a.pos.x,
                a.pos.y,
                corps.map(|d| d.round()),
                b.out_of_supply,
                b.supply_relay,
                b.supply_stock,
                b.zone_active
            );
        }
        for o in w.orgs.iter().filter(|o| o.alive && o.kingdom < 2 && data.def(o.def).supply_used) {
            let out = w.books.org(o.id).is_some_and(|b| b.out_of_supply);
            let hq_near = o.hq.and_then(|h| w.actor(h)).map(|h| h.pos.truncate().distance(aa_play::supply::org_position(&w, o.id)));
            if out && hq_near.is_some_and(|d| d < 40.0) {
                bad += 1;
            }
            eprintln!("t={minute}m org {} k{} out_of_supply {out} hq {:?} dist {:?}", o.id, o.kingdom, o.hq, hq_near.map(|d| d.round()));
        }
        let b = w.books.kingdom(1).unwrap();
        eprintln!("t={minute}m k1 units {}/{} buildings {}/{}", b.consumed[idx("unit_limit_consumed")], b.provided[idx("unit_limit_provided")], b.consumed[idx("building_limit_consumed")], b.provided[idx("building_limit_provided")]);
    }
    assert_eq!(bad, 0, "regiments next to their HQ were out of supply");
}

/// Every nation's starting income is its Corps HQ's production
/// (Data/Buildings/<NAT>/<nat>_corps.tgi), on Berlin and on a generated map.
#[test]
#[ignore]
fn every_nation_corps_income_at_start() {
    use aa_play::mapgen::{RandomMapData, RandomMapParams};
    let data = data();
    let rmg = RandomMapData::load(&root()).expect("random map data");
    let info = rmg.catalog().rmcs.into_iter().find(|r| r.ids == "rmc_southerneurope").expect("rmc");
    let nations = ["usa", "gbr", "rus", "ger", "jpn"];
    let mut bad = Vec::new();
    for map_kind in ["(2v2)Berlin", "generated"] {
        for (n, nation) in nations.iter().enumerate() {
            let other = nations[(n + 1) % nations.len()];
            let (map, starts) = if map_kind == "generated" {
                (Arc::new(rmg.generate(&RandomMapParams::defaults(&info, 2, 3)).expect("generate").map), [0, 1])
            } else {
                (Arc::new(GameMap::load_with(&root(), map_kind, &data).expect("Berlin")), [0, 2])
            };
            let setup = aa_play::SkirmishSetup {
                map_id: map_kind.into(),
                players: vec![
                    aa_play::PlayerSetup { name: "A".into(), nation: (*nation).into(), team: 0, color: [0, 76, 255], controller: aa_play::Controller::Human, start: starts[0] },
                    aa_play::PlayerSetup { name: "B".into(), nation: other.into(), team: 1, color: [217, 0, 25], controller: aa_play::Controller::Human, start: starts[1] },
                ],
                starting_resources: None,
                seed: 3,
            };
            let mut w = aa_play::World::new_skirmish(data.clone(), map, &setup).expect("new_skirmish");
            for _ in 0..16 {
                w.step(vec![]);
            }
            for k in 0..2usize {
                let nat = &w.kingdoms[k].nation;
                let corps = data.find(&format!("{nat}_corps")).map(|d| data.def(d)).expect("corps");
                let owned: Vec<String> = w.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom == Some(k as u32)).map(|a| data.def(a.def).id.clone()).collect();
                let b = w.books.kingdom(k as u32).unwrap();
                let want = [corps.production_all[0], corps.production_all[1], corps.production_all[2]];
                eprintln!(
                    "{map_kind} {nat}: income {:?} want {want:?} provided {:?} consumed {:?} raw {:?} owned {owned:?}",
                    w.kingdoms[k].income, b.provided, b.consumed, b.raw_provided
                );
                if w.kingdoms[k].income != want {
                    bad.push(format!("{map_kind} {nat}: {:?} != {want:?}", w.kingdoms[k].income));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// What a division HQ truck adds to the income while packed, unpacking and
/// deployed (56F9A0 / 56F960 / 4D0C90): the packed truck produces its own
/// EconomyComponent (rus_truck_infantry 12/5/2), nothing while unpacking, the
/// deployed HQ in supply its own (rus_hq_infantry 25/10/5), and a deployed HQ
/// cut off with an empty stockpile its packed truck's values. The Corps is
/// 25/20/10. Supply trucks and depots give no money.
#[test]
#[ignore]
fn hq_truck_income_packed_unpacking_and_deployed() {
    use aa_play::{Command, Event};
    let data = data();
    let corps_income = [25.0, 20.0, 10.0];
    let truck_income = [12.0, 5.0, 2.0];
    let hq_income = [25.0, 10.0, 5.0];
    let sum = |a: [f32; 3], b: [f32; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    for (label, offset, deployed_want) in [("near", glam::Vec2::new(0.0, -30.0), sum(corps_income, hq_income)), ("far", glam::Vec2::new(0.0, -150.0), sum(corps_income, truck_income))] {
        let mut w = berlin_world();
        let corps = w.actors.iter().find(|a| a.kingdom == Some(0) && data.def(a.def).id == "rus_corps").unwrap().id;
        let cpos = w.actors[corps as usize].pos.truncate();
        w.step(vec![]);
        assert_eq!(w.kingdoms[0].income, corps_income, "{label}: the Corps alone");
        w.step(vec![Command::Recruit { kingdom: 0, building: corps, def: data.find("regiment_rus_hq_infantry").unwrap() }]);
        let mut truck = None;
        for _ in 0..(16 * 60) {
            w.step(vec![]);
            if let Some(Event::Recruited { org: Some(o), .. }) = w.events.iter().find(|e| matches!(e, Event::Recruited { .. })) {
                truck = Some(w.orgs[*o as usize].members[0]);
                break;
            }
        }
        let truck = truck.expect("HQ truck");
        w.step(vec![]);
        eprintln!("{label} t={:.1}s packed truck: income {:?} out {}", w.time, w.kingdoms[0].income, w.books.out_of_supply(truck));
        assert_eq!(w.kingdoms[0].income, sum(corps_income, truck_income), "{label}: packed truck");
        w.step(vec![Command::Deploy { kingdom: 0, truck, at: cpos + offset, facing: -std::f32::consts::FRAC_PI_2 }]);
        let mut hq = None;
        // The economy runs before the deployment in a tick: check the second
        // tick of the unpacking.
        let mut unpacking_ticks = 0;
        let mut cut_off_ticks = 0;
        for _ in 0..(16 * 120) {
            w.step(vec![]);
            if let Some(Event::Deployed { building }) = w.events.iter().find(|e| matches!(e, Event::Deployed { .. })) {
                hq = Some(*building);
                break;
            }
            // A packed truck outside every zone still produces as itself
            // (it has no +548 packed form).
            if w.actors[truck as usize].alive && !w.actors[truck as usize].removed && w.books.out_of_supply(truck) {
                cut_off_ticks += 1;
                if cut_off_ticks == 2 {
                    eprintln!("{label} t={:.1}s truck out of supply: income {:?}", w.time, w.kingdoms[0].income);
                    assert_eq!(w.kingdoms[0].income, sum(corps_income, truck_income), "{label}: packed truck out of supply");
                }
            }
            if w.books.actors.iter().any(|b| b.deploy.is_some()) {
                unpacking_ticks += 1;
                if unpacking_ticks == 2 {
                    eprintln!("{label} t={:.1}s unpacking: income {:?}", w.time, w.kingdoms[0].income);
                    assert_eq!(w.kingdoms[0].income, corps_income, "{label}: nothing while unpacking");
                }
            }
        }
        assert!(unpacking_ticks >= 2, "{label}: unpacking state");
        if label == "far" {
            assert!(cut_off_ticks >= 2, "far: the truck drives out of the Corps zone");
        }
        let hq = hq.expect("deployed");
        w.step(vec![]);
        let b = w.books.actor(hq).unwrap();
        eprintln!(
            "{label} t={:.1}s deployed {:.0} m from the Corps: income {:?} out {} stock {:.1}",
            w.time,
            w.actors[hq as usize].pos.truncate().distance(cpos),
            w.kingdoms[0].income,
            b.out_of_supply,
            b.supply_stock
        );
        assert_eq!(w.kingdoms[0].income, deployed_want, "{label}: deployed");
    }
    // Supply trucks and depots: no money.
    for nat in ["usa", "gbr", "rus", "ger", "jpn"] {
        for id in [format!("{nat}_truck_supply"), format!("{nat}_depot_supply")] {
            let d = data.def(data.find(&id).unwrap());
            assert_eq!(d.production_all[0], 0.0, "{id} money");
        }
    }
}

/// Probe: the balanced villages next to each start on a generated map
/// (RMCVillageActorGroups, side_enemy), who owns them over the first 40 s.
#[test]
#[ignore]
fn generated_map_start_villages() {
    use aa_play::mapgen::{RandomMapData, RandomMapParams};
    let data = data();
    let rmg = RandomMapData::load(&root()).expect("random map data");
    let info = rmg.catalog().rmcs.into_iter().find(|r| r.ids == "rmc_southerneurope").expect("rmc");
    for seed in [3u64, 7, 11] {
        let generated = rmg.generate(&RandomMapParams::defaults(&info, 2, seed)).expect("generate");
        let setup = aa_play::SkirmishSetup {
            map_id: "generated".into(),
            players: vec![
                aa_play::PlayerSetup { name: "A".into(), nation: "rus".into(), team: 0, color: [0, 76, 255], controller: aa_play::Controller::Human, start: 0 },
                aa_play::PlayerSetup { name: "B".into(), nation: "ger".into(), team: 1, color: [217, 0, 25], controller: aa_play::Controller::Human, start: 1 },
            ],
            starting_resources: None,
            seed,
        };
        let mut w = aa_play::World::new_skirmish(data.clone(), Arc::new(generated.map), &setup).expect("new_skirmish");
        for second in [0, 1, 10, 40] {
            while w.time < second as f32 {
                w.step(vec![]);
            }
            for k in 0..2u32 {
                let corps = w.actors.iter().find(|a| a.kingdom == Some(k) && data.def(a.def).id.ends_with("_corps")).map(|a| a.pos.truncate()).unwrap();
                let mut near: Vec<(f32, String, Option<u32>, f32)> = w
                    .actors
                    .iter()
                    .filter(|a| a.alive && !a.removed && data.def(a.def).city && data.def(a.def).city_zone_radius > 0.0)
                    .map(|a| (a.pos.truncate().distance(corps), data.def(a.def).id.clone(), a.kingdom, data.def(a.def).city_zone_radius))
                    .collect();
                near.sort_by(|a, b| a.0.total_cmp(&b.0));
                eprintln!("seed {seed} t={:.1}s k{k} income {:?} nearest cities {:?}", w.time, w.kingdoms[k as usize].income, &near[..near.len().min(2)]);
            }
        }
    }
}

/// Probe: combat values (Performer+60, 57E430) of the start actors, the
/// value 4D01E0 counts for city admission.
#[test]
#[ignore]
fn start_actor_combat_values() {
    let mut w = berlin_world();
    w.step(vec![]);
    let data = w.data.clone();
    let extras = w.combat.extras.clone().expect("combat extras");
    for id in ["rus_truck_supply", "rus_depot_supply", "rus_truck_infantry", "rus_hq_infantry", "rus_corps", "rus_bunker", "rus_bunker_aa", "rus_bunker_artillery", "regiment_rus_hq_infantry", "regiment_rus_infantry", "rus_rifleman", "start_loc"] {
        if let Some(d) = data.find(id) {
            eprintln!("{id}: combat_value {} weapons {}", extras.def(d).combat_value, data.def(d).weapons.len());
        }
    }
}

/// The human recruit chain for every nation and every building that
/// recruits: the Corps recruits the division HQ (or truck), the truck
/// deploys next to the Corps, the building is in supply and provides its
/// unit limit, and every organization it lists is recruited from it
/// (Command::Recruit targets the deployed building actor) and spawns
/// attached to it.
#[test]
#[ignore]
fn every_nation_recruits_from_every_division_hq() {
    use aa_play::{Command, Event};
    let data = data();
    let mut failures = Vec::new();
    for nation in ["usa", "gbr", "rus", "ger", "jpn"] {
        let corps_id = format!("{nation}_corps");
        let corps_def = data.find(&corps_id).expect("corps");
        let unit_limit = data.resource_index("unit_limit_provided").unwrap();
        for &item in &data.def(corps_def).recruits.clone() {
            let item_id = data.def(item).id.clone();
            // The Corps item: an HQ regiment (its truck member) or a lone truck.
            let truck_def = if data.def(item).class == Some(aa_play::data::DefClass::Organization) {
                data.def(item).slots.iter().map(|(d, _)| *d).find(|&d| d != aa_play::data::NO_DEF && data.def(d).unpacks_to.is_some())
            } else {
                data.def(item).unpacks_to.map(|_| item)
            };
            let Some(truck_def) = truck_def else { continue };
            let Some(building_def) = data.def(truck_def).unpacks_to else { continue };
            if data.def(building_def).recruits.is_empty() {
                continue;
            }
            let map = Arc::new(GameMap::load_with(&root(), "(2v2)Berlin", &data).expect("Berlin"));
            let setup = aa_play::SkirmishSetup {
                map_id: "(2v2)Berlin".into(),
                players: vec![
                    aa_play::PlayerSetup { name: "P".into(), nation: nation.into(), team: 0, color: [0, 76, 255], controller: aa_play::Controller::Human, start: 0 },
                    aa_play::PlayerSetup { name: "Q".into(), nation: "ger".into(), team: 1, color: [217, 0, 25], controller: aa_play::Controller::Human, start: 2 },
                ],
                starting_resources: Some(50000.0),
                seed: 1,
            };
            let mut w = aa_play::World::new_skirmish(data.clone(), map, &setup).expect("new_skirmish");
            let corps = w.actors.iter().find(|a| a.kingdom == Some(0) && a.def == corps_def).unwrap().id;
            let cpos = w.actors[corps as usize].pos.truncate();
            let refused = |w: &aa_play::World| w.events.iter().filter_map(|e| if let Event::Refused { kingdom: 0, reason } = e { Some(reason.clone()) } else { None }).collect::<Vec<_>>();
            w.step(vec![Command::Recruit { kingdom: 0, building: corps, def: item }]);
            let r = refused(&w);
            if !r.is_empty() {
                failures.push(format!("{nation}: Corps refuses {item_id}: {r:?}"));
                continue;
            }
            let mut truck = None;
            for _ in 0..(16 * 120) {
                w.step(vec![]);
                if let Some(a) = w.actors.iter().find(|a| a.alive && !a.removed && a.kingdom == Some(0) && a.def == truck_def) {
                    truck = Some(a.id);
                    break;
                }
            }
            let Some(truck) = truck else {
                failures.push(format!("{nation}: {item_id} never appeared"));
                continue;
            };
            let at = cpos + glam::Vec2::new(0.0, -35.0);
            w.step(vec![Command::Deploy { kingdom: 0, truck, at, facing: -std::f32::consts::FRAC_PI_2 }]);
            let r = refused(&w);
            if !r.is_empty() {
                failures.push(format!("{nation}: deploy {item_id} refused: {r:?}"));
                continue;
            }
            let mut building = None;
            for _ in 0..(16 * 120) {
                w.step(vec![]);
                if let Some(Event::Deployed { building: b }) = w.events.iter().find(|e| matches!(e, Event::Deployed { .. })) {
                    building = Some(*b);
                    break;
                }
            }
            let Some(building) = building else {
                failures.push(format!("{nation}: {item_id} never deployed"));
                continue;
            };
            for _ in 0..16 {
                w.step(vec![]);
            }
            let bdef = data.def(building_def);
            let provided = w.books.kingdom(0).unwrap().provided[unit_limit];
            eprintln!(
                "{nation} {} (from {item_id}): out of supply {} unit limit {} (def {}) org {:?} recruits {:?}",
                bdef.id,
                w.books.out_of_supply(building),
                provided,
                bdef.unit_limit_provided,
                w.actors[building as usize].org.map(|o| data.def(w.orgs[o as usize].def).id.clone()),
                bdef.recruits.iter().map(|&d| data.def(d).id.clone()).collect::<Vec<_>>()
            );
            if w.books.out_of_supply(building) && bdef.supply_required {
                failures.push(format!("{nation}: {} out of supply next to the Corps", bdef.id));
            }
            if w.actors[building as usize].org.is_none() {
                failures.push(format!("{nation}: {} is not in its HQ organization", bdef.id));
            }
            if bdef.unit_limit_provided > 0.0 && provided < bdef.unit_limit_provided {
                failures.push(format!("{nation}: {} unit limit {provided} < {}", bdef.id, bdef.unit_limit_provided));
            }
            // HQComponent max_attachments of the HQ organization the building
            // belongs to: a full HQ refuses further regiments (0x574A10).
            let max_attachments = aa_play::production::hq_org(&w, building).and_then(|o| data.def(w.orgs[o as usize].def).max_attachments);
            let mut recruited = 0u32;
            for &unit in &bdef.recruits {
                let unit_id = data.def(unit).id.clone();
                let orgs_before = w.orgs.len();
                let actors_before = w.actors.len();
                w.step(vec![Command::Recruit { kingdom: 0, building, def: unit }]);
                let r = refused(&w);
                let full = max_attachments.is_some_and(|m| recruited >= m) && data.def(unit).class == Some(aa_play::data::DefClass::Organization);
                if full {
                    if r != ["the HQ has no free attachment".to_string()] {
                        failures.push(format!("{nation}: full {} gives {r:?} for {unit_id}", bdef.id));
                    }
                    continue;
                }
                if !r.is_empty() {
                    failures.push(format!("{nation}: {} refuses {unit_id} with {recruited} attached: {r:?}", bdef.id));
                    continue;
                }
                let mut done = false;
                for _ in 0..(16 * 180) {
                    w.step(vec![]);
                    if w.events.iter().any(|e| matches!(e, Event::Recruited { .. })) {
                        done = true;
                        break;
                    }
                }
                if !done {
                    let q = &w.books.actor(building).unwrap().queue;
                    failures.push(format!("{nation}: {} -> {unit_id} never spawned (queue {:?})", bdef.id, q.items.iter().map(|i| data.def(i.def).id.clone()).collect::<Vec<_>>()));
                    // Clear the stuck item so the next one is tried.
                    let n = q.items.len();
                    for i in (0..n).rev() {
                        aa_play::production::cancel(&mut w, building, i);
                    }
                    continue;
                }
                let spawned_org = w.orgs.len() > orgs_before;
                let attached = w.orgs.iter().skip(orgs_before).any(|o| o.hq == Some(building));
                recruited += 1;
                if spawned_org && !attached {
                    failures.push(format!("{nation}: {unit_id} is not attached to {}", bdef.id));
                }
                eprintln!("  {unit_id}: spawned org {spawned_org} attached {attached} actors +{}", w.actors.len() - actors_before);
                if data.def(unit).class == Some(aa_play::data::DefClass::Organization) && !spawned_org {
                    failures.push(format!("{nation}: {} -> {unit_id} spawned no organization", bdef.id));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The same supply check on a generated map with two computers.
#[test]
#[ignore]
fn generated_map_supply_with_two_computers() {
    use aa_play::mapgen::{RandomMapData, RandomMapParams};
    let rmg = RandomMapData::load(&root()).expect("random map data");
    let info = rmg.catalog().rmcs.into_iter().find(|r| r.ids == "rmc_southerneurope").expect("rmc");
    let generated = rmg.generate(&RandomMapParams::defaults(&info, 2, 3)).expect("generate");
    let setup = aa_play::SkirmishSetup {
        map_id: "generated".into(),
        players: vec![
            aa_play::PlayerSetup { name: "A".into(), nation: "usa".into(), team: 0, color: [0, 76, 255], controller: aa_play::Controller::Computer(1), start: 0 },
            aa_play::PlayerSetup { name: "B".into(), nation: "rus".into(), team: 1, color: [217, 0, 25], controller: aa_play::Controller::Computer(1), start: 1 },
        ],
        starting_resources: None,
        seed: 3,
    };
    let mut w = aa_play::World::new_skirmish(data(), Arc::new(generated.map), &setup).expect("new_skirmish");
    aa_play::ai::attach(&mut w, Arc::new(aa_play::ai::AiData::load(&root()).expect("AiData")));
    let data = w.data.clone();
    let mut bad = 0;
    for minute in 0..8 {
        for _ in 0..(16 * 60) {
            w.step(vec![]);
        }
        for a in w.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom.is_some_and(|k| k < 2) && data.def(a.def).supply_relay) {
            let b = w.books.actor(a.id).unwrap();
            let corps = w.actors.iter().find(|c| c.kingdom == a.kingdom && data.def(c.def).id.ends_with("_corps") && !c.removed).map(|c| c.pos.truncate().distance(a.pos.truncate()));
            eprintln!("t={minute}m k{:?} {} corps {:?} m: out {} relay {:?} stock {:.1}", a.kingdom, data.def(a.def).id, corps.map(|d| d.round()), b.out_of_supply, b.supply_relay, b.supply_stock);
        }
        for o in w.orgs.iter().filter(|o| o.alive && o.kingdom < 2 && data.def(o.def).supply_used) {
            let out = w.books.org(o.id).is_some_and(|b| b.out_of_supply);
            let d = o.hq.and_then(|h| w.actor(h)).map(|h| h.pos.truncate().distance(aa_play::supply::org_position(&w, o.id)));
            if out && d.is_some_and(|d| d < 40.0) {
                bad += 1;
            }
            eprintln!("t={minute}m org {} k{} out {out} hq dist {:?}", o.id, o.kingdom, d.map(|d| d.round()));
        }
    }
    assert_eq!(bad, 0);
}

/// The human side through `World::step`: recruit the infantry HQ, deploy it
/// next to the Corps, recruit a regiment; the HQ relay connects, fills, and
/// keeps the regiment supplied.
#[test]
#[ignore]
fn berlin_human_hq_supplies_its_regiment() {
    use aa_play::{Command, Event};
    let mut w = berlin_world();
    let data = w.data.clone();
    let corps = w.actors.iter().find(|a| a.kingdom == Some(0) && data.def(a.def).id == "rus_corps").unwrap().id;
    let cpos = w.actors[corps as usize].pos.truncate();
    w.step(vec![Command::Recruit { kingdom: 0, building: corps, def: data.find("regiment_rus_hq_infantry").unwrap() }]);
    let mut truck = None;
    for _ in 0..(16 * 40) {
        w.step(vec![]);
        if let Some(Event::Recruited { org: Some(o), .. }) = w.events.iter().find(|e| matches!(e, Event::Recruited { .. })) {
            truck = Some(w.orgs[*o as usize].members[0]);
        }
    }
    let truck = truck.expect("HQ truck");
    let at = cpos + glam::Vec2::new(0.0, -30.0);
    w.step(vec![Command::Deploy { kingdom: 0, truck, at, facing: -std::f32::consts::FRAC_PI_2 }]);
    let mut hq = None;
    for _ in 0..(16 * 60) {
        w.step(vec![]);
        if let Some(Event::Deployed { building }) = w.events.iter().find(|e| matches!(e, Event::Deployed { .. })) {
            hq = Some(*building);
            break;
        }
    }
    let hq = hq.expect("deployed");
    w.step(vec![Command::Recruit { kingdom: 0, building: hq, def: data.find("regiment_rus_infantry").unwrap() }]);
    let mut org = None;
    for second in 0..120 {
        for _ in 0..16 {
            w.step(vec![]);
            if let Some(Event::Recruited { org: Some(o), .. }) = w.events.iter().find(|e| matches!(e, Event::Recruited { .. })) {
                org = Some(*o);
            }
        }
        if second % 10 == 9 {
            let b = w.books.actor(hq).unwrap();
            eprintln!(
                "t={:.0}s hq {:.0} m from corps: out {} relay {:?} stock {:.1} zone {} | org {:?} out {:?}",
                w.time,
                w.actors[hq as usize].pos.truncate().distance(cpos),
                b.out_of_supply,
                b.supply_relay,
                b.supply_stock,
                b.zone_active,
                org,
                org.map(|o| w.books.org(o).map(|b| b.out_of_supply))
            );
        }
    }
    let o = org.expect("regiment");
    assert!(!w.books.out_of_supply(hq));
    assert!(w.books.actor(hq).unwrap().supply_stock > 80.0);
    assert!(!w.books.org(o).unwrap().out_of_supply);
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
