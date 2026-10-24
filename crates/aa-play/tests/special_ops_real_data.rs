//! The generals' special operations on the original data (ignored: needs
//! AA_GAME_ROOT, e.g. `E:\Games\Axis and Alies`). Run with
//! `cargo test -p aa-play --release --test special_ops_real_data -- --ignored --nocapture`.
use aa_play::special_ops::{self, GeneralData};
use aa_play::{AbilityTarget, Command, Controller, Event, GameData, GameMap, PlayerSetup, SkirmishSetup, World};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn root() -> PathBuf {
    PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"E:\Games\Axis and Alies".into()))
}

fn data() -> Arc<GameData> {
    static DATA: OnceLock<Arc<GameData>> = OnceLock::new();
    DATA.get_or_init(|| Arc::new(GameData::load(&root()).expect("GameData::load"))).clone()
}

fn generals() -> Arc<GeneralData> {
    static G: OnceLock<Arc<GeneralData>> = OnceLock::new();
    G.get_or_init(|| {
        let t = std::time::Instant::now();
        let g = GeneralData::load(&root(), &data()).expect("GeneralData::load");
        eprintln!("GeneralData::load {:?}: {} generals, {} egos with powers, notes {:?}", t.elapsed(), g.generals.len(), g.ego_powers.len(), g.notes);
        Arc::new(g)
    })
    .clone()
}

#[test]
#[ignore]
fn generals_load_with_their_abilities() {
    let g = generals();
    for general in &g.generals {
        eprintln!("{} ({}) public {} max {} icon {:?}", general.id, general.name, general.public, general.experience_max, general.icon);
        for op in &general.abilities {
            eprintln!(
                "   {} {:?} cost {} recharge {} area {} volley {} max_targets {} req {:#x} forb {:#x} flags {:#x} power {} proj {:?} life {} type {:?} icon {:?}",
                op.id, op.name, op.cost, op.recharge, op.area, op.volley_area, op.max_targets, op.required, op.forbidden, op.flags, op.power, op.projectile, op.projectile_life, op.ability_type, op.icon
            );
            for e in &op.effects {
                eprintln!(
                    "      kind {} req {:#x} forb {:#x} amount {} dt {:?} ench {:?} summon {:?} steal {:?}",
                    e.kind,
                    e.required,
                    e.forbidden,
                    e.amount,
                    e.damage_type,
                    e.enchantment.as_ref().map(|x| (&x.id, x.duration, &x.kind, &x.required_properties, &x.modifiers)),
                    e.summon.as_ref().map(|s| (&s.object, s.controllable, &s.transport)),
                    e.steal
                );
            }
        }
    }
    for (ego, powers) in g.ego_powers.iter().take(6) {
        eprintln!("{ego}: {:?}", powers.iter().map(|p| (&p.ability, p.location, p.base, p.minimum, p.radius, p.candidates)).collect::<Vec<_>>());
    }
    assert!(g.notes.is_empty(), "{:?}", g.notes);
    assert!(g.generals.len() >= 10, "{} generals", g.generals.len());
    for general in g.generals.iter().filter(|g| g.public) {
        assert_eq!(general.abilities.len(), 4, "{} has {} abilities", general.id, general.abilities.len());
        assert_eq!(general.experience_max, 5000.0, "{}", general.id);
        for op in &general.abilities {
            assert!(op.cost > 0.0 && op.recharge > 0.0, "{} {}", general.id, op.id);
        }
    }
    for nation in ["usa", "gbr", "rus", "ger", "jpn"] {
        let list = special_ops::nation_generals(&g, &data(), nation);
        eprintln!("{nation}: {:?}", list.iter().map(|g| &g.id).collect::<Vec<_>>());
        assert!(!list.is_empty(), "{nation} has no general");
    }
}

fn berlin() -> World {
    let data = data();
    let map = Arc::new(GameMap::load_with(&root(), "(2v2)Berlin", &data).expect("Berlin"));
    let setup = SkirmishSetup {
        map_id: "(2v2)Berlin".into(),
        players: vec![
            PlayerSetup { name: "Player".into(), nation: "rus".into(), team: 0, color: [0, 76, 255], controller: Controller::Human, start: 0 },
            PlayerSetup { name: "Enemy".into(), nation: "ger".into(), team: 1, color: [217, 0, 25], controller: Controller::Human, start: 2 },
        ],
        starting_resources: None,
        seed: 1,
    };
    let mut w = World::new_skirmish(data, map, &setup).expect("new_skirmish");
    special_ops::attach(&mut w, generals());
    w
}

/// One side of the battle, for before/after comparisons.
#[derive(Debug, Default, PartialEq)]
struct Side {
    actors: usize,
    health: f32,
    morale: f32,
    enchanted: usize,
    money: f32,
}

fn side(w: &World, k: u32) -> Side {
    let mut s = Side { money: w.kingdoms[k as usize].stockpile.first().copied().unwrap_or(0.0), ..Default::default() };
    for a in w.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom == Some(k)) {
        s.actors += 1;
        s.health += a.health;
        s.enchanted += special_ops::enchantments_on(w, special_ops::OpTarget::Actor(a.id)).len();
    }
    for o in w.orgs.iter().filter(|o| o.alive && o.kingdom == k) {
        s.morale += o.morale;
        s.enchanted += special_ops::enchantments_on(w, special_ops::OpTarget::Org(o.id)).len();
    }
    s
}

fn fmt(s: &Side) -> String {
    format!("{}a {}hp {}m {}e ${}", s.actors, s.health as i32, s.morale as i32, s.enchanted, s.money as i32)
}

/// Berlin with an infantry and a tank regiment per side, 40 m apart next to
/// the first player's start.
fn berlin_armies() -> (World, Vec<u32>, Vec<u32>) {
    let mut w = berlin();
    let d = w.data.clone();
    let base = w.actors.iter().find(|a| a.kingdom == Some(0)).map(|a| a.pos.truncate()).unwrap();
    let at = base + glam::Vec2::new(30.0, 0.0);
    let mut own = Vec::new();
    let mut enemy = Vec::new();
    for (k, id, p) in [
        (0, "regiment_rus_infantry", at),
        (0, "regiment_rus_tank_medium", at + glam::Vec2::new(0.0, 14.0)),
        (1, "regiment_ger_infantry", at + glam::Vec2::new(40.0, 0.0)),
        (1, "regiment_ger_tank_medium", at + glam::Vec2::new(40.0, 14.0)),
    ] {
        let o = aa_play::production::spawn_organization(&mut w, d.find(id).expect(id), k, p, 0.0, None);
        if k == 0 { own.push(o) } else { enemy.push(o) }
    }
    w.step(Vec::new());
    (w, own, enemy)
}

/// Every public general's every ability, aimed at the first actor its target
/// filter admits (the enemy's regiments for hostile ones, the own
/// otherwise), on Berlin with full experience: what it did.
#[test]
#[ignore]
fn every_ability_runs_on_berlin() {
    let g = generals();
    let mut used = 0;
    for general in g.generals.iter().filter(|g| g.public) {
        for i in 0..general.abilities.len() {
            let op = &general.abilities[i];
            let (mut w, own_orgs, enemy_orgs) = berlin_armies();
            special_ops::set_general(&mut w, 0, Some(&general.id));
            special_ops::set_general(&mut w, 1, None);
            special_ops::assign_missing_generals(&mut w);
            special_ops::set_starting_experience(&mut w, 5000.0);
            let members = |orgs: &[u32]| orgs.iter().flat_map(|&o| w.orgs[o as usize].members.clone()).collect::<Vec<_>>();
            let others = |k: u32| w.actors.iter().filter(|a| a.alive && a.kingdom == Some(k) && a.org.is_none()).map(|a| a.id).collect::<Vec<_>>();
            let own: Vec<_> = members(&own_orgs).into_iter().chain(others(0)).collect();
            let enemy: Vec<_> = members(&enemy_orgs).into_iter().chain(others(1)).collect();
            let hostile = op.required & 2 != 0 || op.effects.iter().any(|e| matches!(e.kind, 1 | 4 | 5 | 8 | 9));
            let candidates: Vec<_> = if hostile { enemy.iter().chain(&own).copied().collect() } else { own.iter().chain(&enemy).copied().collect() };
            let self_target = special_ops::view(&w, 0, i).unwrap().self_target;
            let target = candidates
                .iter()
                .filter(|_| !self_target)
                .copied()
                .find(|&a| special_ops::admit(&w, 0, i, &mut Some(special_ops::OpTarget::Actor(a))).is_ok())
                .map(AbilityTarget::Actor)
                .or_else(|| (op.accepts_location() && !self_target).then(|| AbilityTarget::Point(w.actors[enemy[0] as usize].pos.truncate())));
            let before = (side(&w, 0), side(&w, 1));
            w.step(vec![Command::UseAbility { kingdom: 0, ability: i as u32, target }]);
            let refused: Vec<_> = w.events.iter().filter_map(|e| if let Event::Refused { reason, .. } = e { Some(reason.clone()) } else { None }).collect();
            let ok = w.events.iter().any(|e| matches!(e, Event::SpecialOperation { .. }));
            used += ok as usize;
            let mut mid = None;
            let mut no_fog = false;
            for t in 0..16 * 10 {
                w.step(Vec::new());
                if t == 16 * 4 {
                    mid = Some((side(&w, 0), side(&w, 1)));
                    no_fog = special_ops::kingdom_has_flag(&w, 0, "MINIMAP_NO_FOG");
                }
            }
            let mid = mid.unwrap();
            let (xp, _) = special_ops::experience(&w, 0).unwrap();
            eprintln!(
                "{} #{i} {} (cost {} recharge {}) {:?}: {} | own {} -> {} | enemy {} -> {} | xp {xp:.0}",
                general.id,
                op.id,
                op.cost,
                op.recharge,
                target,
                if ok { "used".to_string() } else { format!("refused {refused:?}") },
                fmt(&before.0),
                fmt(&mid.0),
                fmt(&before.1),
                fmt(&mid.1),
            );
            match op.id.as_str() {
                "gbr_carpet_bomb" | "usa_carpet_bomb" | "rus_carpet_bomb" | "v_weapons" | "kamikaze" | "atomic_bomb" => {
                    assert!(mid.1.health < before.1.health && mid.1.actors < before.1.actors, "{}", op.id)
                }
                "courage" | "banzai_charge" | "communist_zeal" | "forced_march" | "blitzkrieg" => assert!(mid.0.enchanted > before.0.enchanted, "{}", op.id),
                "fear" | "propaganda_war_ger" | "propaganda_war_usa" => assert!(mid.1.morale < before.1.morale && mid.1.enchanted > 0, "{}", op.id),
                "smoke_barrage" => assert!(mid.1.enchanted > 0, "{}", op.id),
                "radar_scan" => assert!(self_target && no_fog && !special_ops::kingdom_has_flag(&w, 0, "MINIMAP_NO_FOG"), "{}", op.id),
                "espionage" => assert!((mid.0.money - before.0.money - 50.0).abs() < 2.0 && (before.1.money - mid.1.money - 50.0).abs() < 2.0, "{}", op.id),
                "ger_supply_drop" | "rus_supply_drop" | "usa_supply_drop" | "panzer_hohle" | "maskirovka" | "code_breaker" => {
                    assert!(mid.0.actors > before.0.actors, "{}", op.id)
                }
                _ => {}
            }
            if ok {
                // the cost was paid (the regiments' gains and +1/s come on top)
                assert!(xp >= 5000.0 - op.cost && xp <= 5000.0, "{} xp {xp}", op.id);
                // used again at once: recharging
                w.step(vec![Command::UseAbility { kingdom: 0, ability: i as u32, target }]);
                assert!(!w.events.iter().any(|e| matches!(e, Event::SpecialOperation { .. })), "{} used twice", op.id);
            }
        }
    }
    eprintln!("{used} abilities used");
}

/// The computer opponent's SAI general powers (`7C40C0`): a computer
/// Russia with its SAI profile's general (Chuikov) plays Berlin for 20 minutes
/// against an idle player and uses its general's abilities.
#[test]
#[ignore]
fn computer_uses_general_powers_on_berlin() {
    let data = data();
    let map = Arc::new(GameMap::load_with(&root(), "(2v2)Berlin", &data).expect("Berlin"));
    let setup = SkirmishSetup {
        map_id: "(2v2)Berlin".into(),
        players: vec![
            PlayerSetup { name: "Player".into(), nation: "ger".into(), team: 0, color: [0, 76, 255], controller: Controller::Human, start: 0 },
            PlayerSetup { name: "Computer".into(), nation: "rus".into(), team: 1, color: [217, 0, 25], controller: Controller::Computer(1), start: 2 },
        ],
        starting_resources: None,
        seed: 1,
    };
    let mut w = World::new_skirmish(data, map, &setup).expect("new_skirmish");
    let sai = Arc::new(aa_play::ai::AiData::load(&root()).expect("AiData::load"));
    aa_play::ai::attach(&mut w, sai);
    aa_play::ai::set_profile(&mut w, 1, "chuikov_sai");
    special_ops::attach(&mut w, generals());
    special_ops::set_general(&mut w, 0, Some("rommel"));
    special_ops::set_general(&mut w, 1, Some("chuikov"));
    special_ops::assign_missing_generals(&mut w);
    let mut uses = Vec::new();
    let t = std::time::Instant::now();
    for _ in 0..16 * 60 * 20 {
        w.step(Vec::new());
        for e in &w.events {
            match e {
                Event::SpecialOperation { kingdom, ability, at, target } => uses.push(format!("{:.0}s k{kingdom} {ability} at {at:?} on {target:?}", w.time)),
                Event::Refused { kingdom: 1, reason } if reason.starts_with("special_operation") => uses.push(format!("{:.0}s refused {reason}", w.time)),
                _ => {}
            }
        }
    }
    eprintln!("20 min in {:?}; xp {:?}", t.elapsed(), special_ops::experience(&w, 1));
    for u in &uses {
        eprintln!("{u}");
    }
    assert!(uses.iter().any(|u| u.contains(" k1 ")), "the computer used no general power");
}
