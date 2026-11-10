//! An air base's recon order end to end on the real data (ignored: needs
//! AA_GAME_ROOT): the Denizen command 0x56E730 -> 0x56DA00, the aircraft
//! Mind's states, and the base aircraft's flight on the exact MoveTick
//! (`aa_play::combat::air::flight`). Run with
//! `cargo test -p aa-play --test air_recon --release -- --ignored --nocapture`.
use aa_play::combat::air;
use aa_play::world::AbilityTarget;
use aa_play::{Command, GameData, GameMap};
use glam::Vec2;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

/// Mind offsets of the aircraft states (aa-mind `uai_thinks::aircraft`).
const QUEUED: u32 = 0x188;
const TAKEOFF: u32 = 0x15c;
const RECON: u32 = 0xb4;
const JOIN: u32 = 0x12c;
const LANDING: u32 = 0x174;

fn root() -> PathBuf {
    PathBuf::from(std::env::var("AA_GAME_ROOT").expect("set AA_GAME_ROOT to the folder of the installed game"))
}

fn data() -> Arc<GameData> {
    static DATA: OnceLock<Arc<GameData>> = OnceLock::new();
    DATA.get_or_init(|| Arc::new(GameData::load(&root()).expect("GameData::load"))).clone()
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

/// A ger_airfield next to the ger corps, its inventory ready, and the
/// kingdom's money at 1000.
fn airfield(w: &mut aa_play::World) -> (u32, aa_play::ActorId, Vec2, usize) {
    let airfield = w.data.find("ger_airfield").expect("ger_airfield");
    let ger = (0..w.kingdoms.len() as u32).find(|&k| w.kingdoms[k as usize].nation == "ger").expect("ger kingdom");
    let corps = (0..w.actors.len() as u32).find(|&a| w.actors[a as usize].kingdom == Some(ger) && w.data.def(w.actors[a as usize].def).id.ends_with("_corps")).expect("ger corps");
    let base = w.actors[corps as usize].pos.truncate();
    let at = (0..40)
        .flat_map(|r| (0..8).map(move |k| base + Vec2::from_angle(k as f32 * std::f32::consts::FRAC_PI_4) * (40.0 + 2.0 * r as f32)))
        .find(|&p| aa_play::deploy::spot_clear(w, airfield, p, None).is_ok())
        .expect("a clear spot");
    let a = aa_play::production::spawn_single(w, airfield, Some(ger), at, 0.0);
    for _ in 0..20 {
        w.step(vec![]);
    }
    let money = w.data.resources.iter().position(|r| r.id.eq_ignore_ascii_case("money")).expect("money");
    let n = w.data.resources.len();
    w.books.kingdom_mut(ger, n).stockpiled[money] = 1000.0;
    (ger, a, at, money)
}

/// The HUD's air_recon on the airfield: 0x56DA00 pays the AirReconCost
/// (money 25) and a fighter of the AIR_RECON group leaves the hangar on the
/// runway release; its Mover is on queue 0 and stands while aai_queued is
/// current (Mind +0x1B0). It takes off, flies to the point at its
/// operational height, comes back to the runway and lands into its slot.
#[test]
#[ignore]
fn air_recon_order_flies_to_the_point_and_lands() {
    let mut w = berlin_world();
    let (ger, a, at, money) = airfield(&mut w);
    // 120 m toward the map's middle (0x560DB0 clamps a point outside the
    // map into it, 0x552640).
    let middle = Vec2::new(w.map.width as f32, w.map.height as f32) * w.map.cell_size * 0.5;
    let target = at + (middle - at).normalize() * 120.0;
    let before = w.books.kingdom(ger).unwrap().stockpiled[money];
    let first = w.actors.len() as u32;
    w.events.clear();
    w.apply(Command::AirRecon { kingdom: ger, building: a, target: AbilityTarget::Point(target) });
    assert!(!w.events.iter().any(|e| matches!(e, aa_play::world::Event::Refused { .. })), "{:?}", w.events);
    assert_eq!(before - w.books.kingdom(ger).unwrap().stockpiled[money], 25.0, "AirReconCost money 25");
    let fighter = w.data.find("ger_fighter").expect("ger_fighter");
    let f = (first..w.actors.len() as u32).find(|&x| w.actors[x as usize].def == fighter).expect("the recon fighter");
    assert!(air::flight::flies(&w, f));
    assert!(w.is_scheduled(aa_play::scheduler::Owner::Mover(f)), "0x576B20 registers the Mover");
    assert_eq!(air::current_state(&w, f), Some(QUEUED), "waits in the hangar behind aai_queued");
    let hangar = w.actors[f as usize].pos;
    w.step(vec![]);
    let start = w.actors[f as usize].pos;
    let operational = w.data.def(fighter).aircraft.as_ref().unwrap().operational_height;
    let mut seen = Vec::new();
    let mut nearest = f32::MAX;
    let mut highest = f32::MIN;
    let mut detecting = false;
    let mut steps = 0;
    while steps < 20_000 && !w.actors[f as usize].removed {
        w.step(vec![]);
        steps += 1;
        let s = air::current_state(&w, f);
        if seen.last() != Some(&s) {
            eprintln!("{:.2} s: state {s:x?} at {:?}", w.time, w.actors[f as usize].pos);
            seen.push(s);
        }
        let p = w.actors[f as usize].pos;
        nearest = nearest.min(p.truncate().distance(target));
        if s == Some(RECON) {
            highest = highest.max(p.z - w.map.height_at(p.truncate()));
            detecting |= w.is_scheduled(aa_play::scheduler::Owner::Component(f, aa_play::zones::DETECTION_SLOT));
        }
    }
    eprintln!("{steps} steps, nearest {nearest:.2} m, highest {highest:.2} m over ground, detection on its queue {detecting}, notes {:?}", w.books.minds.notes);
    assert!(w.books.minds.notes.is_empty(), "{:?}", w.books.minds.notes);
    assert!(w.actors[f as usize].removed, "the fighter landed and entered its base");
    assert!(hangar.distance(start) < 1e-3, "Mind +0x1B0: no move while queued");
    for s in [TAKEOFF, RECON, JOIN, LANDING] {
        assert!(seen.contains(&Some(s)), "state {s:#x} in {seen:x?}");
    }
    assert!(nearest < 5.0, "reached the recon point: {nearest}");
    assert!(detecting, "the fighter's Detection (sight 18) is on its queue while it scouts");
    assert!(start.truncate().distance(target) > 100.0);
    assert!(highest > 0.5 * operational, "{highest} vs operational_height {operational}");
    let d = w.books.actor(a).and_then(|q| q.denizen.clone()).unwrap();
    assert!(d.slots.iter().all(|s| s.occupant != Some(f)), "housed again");
}

/// The aircraft of the ger_airfield as the loader keeps them: what an air
/// attack would release (the bomber's abilities) and what a recon fighter
/// sees with (its speed and operational height). Prints only.
#[test]
#[ignore]
fn air_base_aircraft_abilities() {
    let data = data();
    for id in ["ger_bomber", "ger_fighter"] {
        let d = data.def(data.find(id).expect(id));
        let air = d.aircraft.clone().unwrap_or_default();
        eprintln!(
            "{id}: speed_max {} turn_radius {} operational_height {} taxi_speed {} health {} sight_range {} detection_range {} detection component {}",
            d.speed_max,
            d.turn_radius,
            air.operational_height,
            air.taxi_speed,
            d.health,
            d.sight_range,
            d.detection_range,
            d.has_component("DetectionComponent")
        );
        for w in &d.weapons {
            let effects: Vec<_> = w.effects.iter().map(|e| e.kind_name.clone()).collect();
            eprintln!(
                "  {} ({}): range {} min {} damage {} type {} area {} max_targets {} flags {:#x} recharge {} projectile {:?} speed {} life {} effects {effects:?}",
                w.id, w.name, w.range, w.min_range, w.damage, w.damage_type, w.area, w.max_targets, w.flags, w.recharge, w.projectile, w.projectile_speed, w.projectile_life
            );
        }
    }
}

/// 0x56E730's refusals: air_recon carries a location only; air_attack is
/// not switched on yet; an airfield with no recon aircraft ready does not
/// take the command (0x4D9B60's mask).
#[test]
#[ignore]
fn air_orders_refused() {
    let mut w = berlin_world();
    let (ger, a, _, _) = airfield(&mut w);
    let first = w.actors.len();
    let refused = |w: &aa_play::World| w.events.iter().any(|e| matches!(e, aa_play::world::Event::Refused { .. }));
    let corps = (0..w.actors.len() as u32).find(|&x| w.actors[x as usize].kingdom == Some(ger) && w.data.def(w.actors[x as usize].def).id.ends_with("_corps")).unwrap();
    w.step(vec![Command::AirRecon { kingdom: ger, building: a, target: AbilityTarget::Actor(corps) }]);
    assert!(refused(&w));
    w.step(vec![Command::AirAttack { kingdom: ger, building: a, target: AbilityTarget::Point(Vec2::new(10.0, 10.0)) }]);
    assert!(refused(&w));
    // Not the owner.
    let rus = (0..w.kingdoms.len() as u32).find(|&k| w.kingdoms[k as usize].nation == "rus").unwrap();
    w.step(vec![Command::AirRecon { kingdom: rus, building: a, target: AbilityTarget::Point(Vec2::new(10.0, 10.0)) }]);
    assert!(refused(&w));
    let aircraft = (first..w.actors.len()).filter(|&x| w.data.def(w.actors[x].def).aircraft.is_some()).count();
    assert_eq!(aircraft, 0, "no aircraft created");
}
