//! The computer opponent on the original data (ignored: needs AA_GAME_ROOT,
//! e.g. `E:\Games\Axis and Alies`). Run with
//! `cargo test -p aa-play --release --test ai_real_data -- --ignored --nocapture`.
use aa_play::ai::{self, AiData};
use aa_play::{Command, Controller, GameData, GameMap, PlayerSetup, SkirmishSetup, World};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn root() -> PathBuf {
    PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"E:\Games\Axis and Alies".into()))
}

fn data() -> Arc<GameData> {
    static DATA: OnceLock<Arc<GameData>> = OnceLock::new();
    DATA.get_or_init(|| Arc::new(GameData::load(&root()).expect("GameData::load"))).clone()
}

fn sai() -> Arc<AiData> {
    static SAI: OnceLock<Arc<AiData>> = OnceLock::new();
    SAI.get_or_init(|| {
        let t = std::time::Instant::now();
        let d = AiData::load(&root()).expect("AiData::load");
        eprintln!("AiData::load {:?}: {} profiles, {} egos, {} handicaps", t.elapsed(), d.profiles.len(), d.egos.len(), d.handicaps.len());
        Arc::new(d)
    })
    .clone()
}

#[test]
#[ignore]
fn sai_data_loads_every_shipped_profile_and_ego() {
    let d = sai();
    assert_eq!(d.profiles.len(), 25);
    assert_eq!(d.egos.len(), 75);
    let zhukov = &d.profiles[d.profile("zhukov_sai").expect("zhukov_sai")];
    assert_eq!(zhukov.factions, vec!["zhukov".to_string()]);
    assert_eq!(zhukov.nations, vec!["nation_rus".to_string()]);
    let low = &d.egos[zhukov.egos[0]];
    assert_eq!(low.ids, "zhukov_low_income_ego");
    assert_eq!(low.think_frequency, 30.0);
    assert_eq!(low.reactive.think_frequency, 2.0);
    assert_eq!(low.goal(aa_play::ai::data::ATTACK_REGION).bonus, 750.0);
    assert_eq!(low.goal(aa_play::ai::data::RECRUIT).multiplier, 0.4);
    assert_eq!(low.build_queue_max_items, 1);
    assert_eq!(low.recruiting.max_goals_per_think, 3);
    assert_eq!(low.offensive.cv_advantage_target, 1.2);
    assert_eq!(low.filters[0].stat, "money_rate");
    assert_eq!(low.filters[0].max, 40.0);
    for e in &d.egos {
        eprintln!(
            "{}: prio {} filters {:?} think {} reqs {:?} mil {} max {} off {:?}/{:?}",
            e.ids,
            e.priority,
            e.filters.iter().map(|f| (f.min, f.max)).collect::<Vec<_>>(),
            e.think_frequency,
            e.recruiting.requests.iter().map(|r| (r.property.clone().or(r.recruit.clone()).unwrap_or_default(), r.base_priority, r.hq_type_not_available_bonus)).collect::<Vec<_>>(),
            e.recruiting.num_military_goals,
            e.recruiting.max_goals_per_think,
            e.offensive.cv_advantage_target,
            e.defensive.min_cv_to_assign
        );
    }
    for h in &d.handicaps {
        eprintln!("handicap {} {:?} {:?}", h.ids, h.property, h.modifiers);
    }
    assert_eq!(ai::handicap(&d, 0), vec![("recruit_cost".to_string(), -0.25), ("health".to_string(), -0.25)]);
    assert!(ai::handicap(&d, 1).is_empty());
}

fn berlin(profile: Option<&str>, level: u8) -> World {
    let data = data();
    let map = Arc::new(GameMap::load_with(&root(), "(2v2)Berlin", &data).expect("Berlin"));
    let setup = SkirmishSetup {
        map_id: "(2v2)Berlin".into(),
        players: vec![
            PlayerSetup { name: "Player".into(), nation: "rus".into(), team: 0, color: [0, 76, 255], controller: Controller::Human, start: 0 },
            PlayerSetup { name: "Computer".into(), nation: "ger".into(), team: 1, color: [217, 0, 25], controller: Controller::Computer(level), start: 2 },
        ],
        starting_resources: None,
        seed: 1,
    };
    let mut w = World::new_skirmish(data, map, &setup).expect("new_skirmish");
    ai::attach(&mut w, sai());
    if let Some(p) = profile {
        ai::set_profile(&mut w, 1, p);
    }
    w.ai.logging = true;
    w
}

#[derive(Default)]
struct Snapshot {
    regiments: usize,
    buildings: usize,
    trucks: usize,
    cv: f32,
    money: f32,
}

fn snapshot(w: &World, k: u32) -> Snapshot {
    let mut s = Snapshot { money: w.kingdoms[k as usize].stockpile[0], ..Default::default() };
    let cv = &w.ai.bound.cv;
    for o in w.orgs.iter().filter(|o| o.alive && o.kingdom == k) {
        let def = w.data.def(o.def);
        if def.properties.iter().any(|p| p == "hq") {
            continue;
        }
        if o.members.iter().any(|&m| w.actors[m as usize].alive) {
            s.regiments += 1;
            s.cv += o.members.iter().filter(|&&m| w.actors[m as usize].alive).map(|&m| cv.get(w.actors[m as usize].def as usize).copied().unwrap_or(0.0)).sum::<f32>();
        }
    }
    for a in w.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom == Some(k)) {
        match w.data.def(a.def).class {
            Some(aa_play::DefClass::Building) => s.buildings += 1,
            Some(aa_play::DefClass::Truck) => s.trucks += 1,
            _ => {}
        }
    }
    s
}

/// One human (idle) and one computer on (2v2)Berlin: after `minutes` the
/// computer has deployed HQs, recruited regiments and attacked.
fn play(profile: Option<&str>, level: u8, minutes: u32) -> World {
    let mut w = berlin(profile, level);
    let ticks = minutes as u64 * 60 * aa_play::TICKS_PER_SECOND as u64;
    let t = std::time::Instant::now();
    let mut refused = std::collections::BTreeMap::<String, usize>::new();
    for tick in 0..ticks {
        w.step(Vec::<Command>::new());
        for e in &w.events {
            if let aa_play::Event::Refused { kingdom: 1, reason } = e {
                *refused.entry(reason.chars().take(90).collect()).or_default() += 1;
            }
        }
        if tick % (60 * aa_play::TICKS_PER_SECOND as u64) == 0 {
            let s = snapshot(&w, 1);
            let h = snapshot(&w, 0);
            let k = &w.kingdoms[1];
            eprintln!(
                "t={:>4.0}s computer: regiments {} (cv {:.0}) buildings {} trucks {} money {:.0} income {:?} units {}/{} | human regiments {} buildings {}",
                w.time, s.regiments, s.cv, s.buildings, s.trucks, s.money, k.income, k.units_used, k.unit_limit, h.regiments, h.buildings
            );
        }
    }
    let p = &w.ai.players[0];
    eprintln!(
        "profile {} ego {} | {} ticks in {:?}; AI avg {:.3} ms/tick, max {:.2} ms, GE {} RAI {} commands {}",
        w.ai.data.as_ref().unwrap().profiles[p.profile].ids,
        w.ai.data.as_ref().unwrap().egos[p.ego].ids,
        ticks,
        t.elapsed(),
        w.ai.stats.total_ns as f64 / w.ai.stats.ticks.max(1) as f64 / 1e6,
        w.ai.stats.max_ns as f64 / 1e6,
        w.ai.stats.ge_thinks,
        w.ai.stats.rai_thinks,
        w.ai.stats.commands
    );
    for (r, n) in &refused {
        eprintln!("refused x{n}: {r}");
    }
    w
}

fn timeline(w: &World) {
    for l in w.ai.log.iter().filter(|l| !l.text.starts_with("  goal") || l.time < 70.0 || l.text.contains("Recruit")) {
        eprintln!("{:>6.1}s k{} {}", l.time, l.kingdom, l.text);
    }
}

#[test]
#[ignore]
fn computer_builds_recruits_and_attacks_on_berlin() {
    let minutes = std::env::var("AA_AI_MINUTES").ok().and_then(|v| v.parse().ok()).unwrap_or(15);
    let w = play(Some("rommel_sai"), 1, minutes);
    timeline(&w);
    let p = &w.ai.players[0];
    for o in w.orgs.iter().filter(|o| o.alive && o.kingdom == 1) {
        let lead = o.members.iter().find(|&&m| w.actors[m as usize].alive).map(|&m| &w.actors[m as usize]);
        eprintln!("org {} {} order {:?} pos {:?} target {:?}", o.id, w.data.def(o.def).id, o.order, lead.map(|a| a.pos), lead.and_then(|a| a.combat.target));
        if let aa_play::Order::Attack { target } = o.order {
            let t = &w.actors[target as usize];
            eprintln!("   target {} {} kingdom {:?} health {}/{} pos {:?}", target, w.data.def(t.def).id, t.kingdom, t.health, w.data.def(t.def).health, t.pos);
        }
    }
    for (o, a) in &p.memory.assigned {
        let t = a.target_actor.map(|t| (w.data.def(w.actors[t as usize].def).id.clone(), w.actors[t as usize].kingdom, w.actors[t as usize].health, w.actors[t as usize].pos, w.data.def(w.actors[t as usize].def).captureable));
        eprintln!("assigned org {o} {:?} target {:?} actor {:?}", a.key, a.target, t);
    }
    let recruited = p.memory.recruited.len();
    let deployed = p.memory.deployed.len();
    let attacks = p.memory.attacks.len();
    eprintln!("recruited {recruited} deployed {deployed} attacks {attacks} first attack {:?}", p.memory.attacks.first());
    assert!(deployed > 0, "the computer deployed no truck");
    assert!(recruited > 2, "the computer recruited {recruited}");
    let s = snapshot(&w, 1);
    assert!(s.buildings > 1, "only {} buildings", s.buildings);
    assert!(s.regiments > 0, "no regiments");
    assert!(attacks > 0, "no attack in {minutes} minutes");
}

/// Without capturable independents (their capture is not playable yet) the
/// computer's attack goals can only aim at the human: record when and with
/// what the first attack comes.
#[test]
#[ignore]
fn computer_attacks_the_human_base_when_no_city_is_capturable() {
    let minutes: u32 = std::env::var("AA_AI_MINUTES").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let mut w = berlin(Some("rommel_sai"), 1);
    for a in w.actors.iter_mut() {
        if a.kingdom.is_none() && w.data.def(a.def).captureable {
            a.removed = true;
        }
    }
    let ticks = minutes as u64 * 60 * aa_play::TICKS_PER_SECOND as u64;
    for tick in 0..ticks {
        w.step(Vec::new());
        if tick % (150 * aa_play::TICKS_PER_SECOND as u64) == 0 {
            let s = snapshot(&w, 1);
            eprintln!("t={:>4.0}s computer: regiments {} (cv {:.0}) buildings {} money {:.0}", w.time, s.regiments, s.cv, s.buildings, s.money);
        }
    }
    for l in w.ai.log.iter().filter(|l| (l.text.starts_with("send") && l.time > 280.0 && l.time < 330.0) || l.text.starts_with("attack") || l.text.starts_with("recruit") || l.text.starts_with("deploy") || l.text.starts_with("retreat") || l.text.starts_with("explore")) {
        eprintln!("{:>6.1}s {}", l.time, l.text);
    }
    let p = &w.ai.players[0];
    eprintln!("first attack {:?}", p.memory.attacks.first());
    for o in w.orgs.iter().filter(|o| o.alive && o.kingdom == 1) {
        let lead = o.members.iter().find(|&&m| w.actors[m as usize].alive).map(|&m| &w.actors[m as usize]);
        eprintln!("org {} {} order {:?} pos {:?} member target {:?} engage {:?}", o.id, w.data.def(o.def).id, o.order, lead.map(|a| a.pos), lead.and_then(|a| a.combat.target), w.combat.org(o.id).map(|c| (c.engage.len(), c.explicit)));
    }
    for a in w.actors.iter().filter(|a| a.kingdom == Some(0) && w.data.def(a.def).class == Some(aa_play::DefClass::Building)) {
        eprintln!("human building {} {} health {} alive {} pos {:?}", a.id, w.data.def(a.def).id, a.health, a.alive, a.pos);
    }
}

#[test]
#[ignore]
fn probe_move_to_human_base() {
    let mut w = berlin(Some("rommel_sai"), 1);
    for a in w.actors.iter_mut() {
        if a.kingdom.is_none() && w.data.def(a.def).captureable {
            a.removed = true;
        }
    }
    let mut sent = false;
    for tick in 0..(12 * 60 * 16u64) {
        let mut cmds = Vec::new();
        if !sent && w.orgs.iter().any(|o| o.alive && o.kingdom == 1 && w.data.def(o.def).properties.iter().all(|p| p != "hq")) {
            let o = w.orgs.iter().find(|o| o.alive && o.kingdom == 1 && w.data.def(o.def).properties.iter().all(|p| p != "hq")).unwrap().id;
            w.ai.players.clear();
            w.ai.profiles.clear();
            w.kingdoms[1].controller = Controller::Human;
            cmds.push(Command::Move { kingdom: 1, units: vec![aa_play::Unit::Org(o)], to: glam::Vec2::new(64.0, 376.0), attack_move: true });
            sent = true;
            eprintln!("sent move for org {o} at t={}", w.time);
        }
        w.step(cmds);
        for e in &w.events {
            if let aa_play::Event::Refused { kingdom: 1, reason } = e {
                eprintln!("refused {reason}");
            }
        }
        if sent && tick % 160 == 0 {
            for o in w.orgs.iter().filter(|o| o.alive && o.kingdom == 1 && w.data.def(o.def).properties.iter().all(|p| p != "hq")) {
                let lead = o.members.iter().find(|&&m| w.actors[m as usize].alive).map(|&m| w.actors[m as usize].pos);
                eprintln!("t={:.0} org {} order {:?} pos {:?}", w.time, o.id, o.order, lead);
            }
        }
    }
}

#[test]
#[ignore]
fn computer_plays_a_generated_map() {
    use aa_play::mapgen::{RandomMapData, RandomMapParams};
    let minutes: u32 = std::env::var("AA_AI_MINUTES").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    let rmg = RandomMapData::load(&root()).expect("random map data");
    let info = rmg.catalog().rmcs.into_iter().find(|r| r.ids == "rmc_southerneurope").expect("rmc");
    let params = RandomMapParams::defaults(&info, 2, 3);
    let generated = rmg.generate(&params).expect("generate");
    let setup = SkirmishSetup {
        map_id: "generated".into(),
        players: vec![
            PlayerSetup { name: "Player".into(), nation: "usa".into(), team: 0, color: [0, 76, 255], controller: Controller::Human, start: 0 },
            PlayerSetup { name: "Computer".into(), nation: "rus".into(), team: 1, color: [217, 0, 25], controller: Controller::Computer(2), start: 1 },
        ],
        starting_resources: None,
        seed: 3,
    };
    let mut w = World::new_skirmish(data(), Arc::new(generated.map), &setup).expect("new_skirmish");
    ai::attach(&mut w, sai());
    w.ai.logging = true;
    for tick in 0..(minutes as u64 * 60 * aa_play::TICKS_PER_SECOND as u64) {
        w.step(Vec::new());
        if tick % (60 * aa_play::TICKS_PER_SECOND as u64) == 0 {
            let s = snapshot(&w, 1);
            eprintln!("t={:>4.0}s computer: regiments {} (cv {:.0}) buildings {} trucks {} money {:.0}", w.time, s.regiments, s.cv, s.buildings, s.trucks, s.money);
        }
    }
    let p = &w.ai.players[0];
    eprintln!("profile {}", w.ai.data.as_ref().unwrap().profiles[p.profile].ids);
    for l in w.ai.log.iter().filter(|l| ["recruit", "deploy", "research", "attack", "retreat", "explore", "attach"].iter().any(|k| l.text.starts_with(k))) {
        eprintln!("{:>6.1}s {}", l.time, l.text);
    }
    assert!(!p.memory.recruited.is_empty(), "nothing recruited");
}

/// Three computer players (Easy, Normal, Hard) and an idle human on a
/// generated four-side map: the opponent's per-tick cost.
#[test]
#[ignore]
fn three_computers_cost_on_a_generated_map() {
    use aa_play::mapgen::{RandomMapData, RandomMapParams};
    let minutes: u32 = std::env::var("AA_AI_MINUTES").ok().and_then(|v| v.parse().ok()).unwrap_or(15);
    let rmg = RandomMapData::load(&root()).expect("random map data");
    let info = rmg.catalog().rmcs.into_iter().find(|r| r.ids == "rmc_southerneurope").expect("rmc");
    let mut params = RandomMapParams::defaults(&info, 4, 11);
    params.width = 640.0;
    params.height = 640.0;
    params.teams = vec![0, 1, 2, 3];
    let generated = rmg.generate(&params).expect("generate");
    let nations = ["usa", "ger", "rus", "jpn"];
    let players = (0..4)
        .map(|i| PlayerSetup {
            name: format!("P{i}"),
            nation: nations[i].into(),
            team: i as u32,
            color: [40 * i as u8, 0, 0],
            controller: if i == 0 { Controller::Human } else { Controller::Computer(i as u8 - 1) },
            start: i,
        })
        .collect();
    let setup = SkirmishSetup { map_id: "generated".into(), players, starting_resources: None, seed: 11 };
    let mut w = World::new_skirmish(data(), Arc::new(generated.map), &setup).expect("new_skirmish");
    ai::attach(&mut w, sai());
    let t = std::time::Instant::now();
    for tick in 0..(minutes as u64 * 60 * aa_play::TICKS_PER_SECOND as u64) {
        w.step(Vec::new());
        if tick % (5 * 60 * aa_play::TICKS_PER_SECOND as u64) == 0 && tick > 0 {
            for k in 1..4u32 {
                let s = snapshot(&w, k);
                eprintln!("t={:>4.0}s k{k} (level {}) regiments {} cv {:.0} buildings {}", w.time, k - 1, s.regiments, s.cv, s.buildings);
            }
        }
    }
    for k in 1..4u32 {
        let s = snapshot(&w, k);
        eprintln!("t={:>4.0}s k{k} (level {}) regiments {} cv {:.0} buildings {}", w.time, k - 1, s.regiments, s.cv, s.buildings);
    }
    let st = w.ai.stats;
    let avg = st.total_ns as f64 / st.ticks.max(1) as f64 / 1e6;
    eprintln!("{minutes} min in {:?}: AI avg {avg:.4} ms/tick, max {:.2} ms, GE {} RAI {} analyses {} commands {}", t.elapsed(), st.max_ns as f64 / 1e6, st.ge_thinks, st.rai_thinks, st.analyses, st.commands);
    for p in &w.ai.players {
        eprintln!("k{} profile {} ego {}", p.kingdom, w.ai.data.as_ref().unwrap().profiles[p.profile].ids, w.ai.data.as_ref().unwrap().egos[p.ego].ids);
    }
    assert!(avg < 1.0);
}
