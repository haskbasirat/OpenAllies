//! An air base's recon order end to end on the real data (ignored: needs
//! AA_GAME_ROOT): the Denizen command 0x56E730 -> 0x56DA00, the aircraft
//! Mind's states, and the base aircraft's flight on the exact MoveTick
//! (`aa_play::combat::air::flight`). Run with
//! `cargo test -p aa-play --test air_recon --release -- --ignored --nocapture`.
use aa_play::combat::air;
use aa_play::fog::TileView;
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
    assert_ne!(aa_play::fog::point_view(&w, Some(ger), target), TileView::Visible, "the recon point is not seen before the order");
    w.events.clear();
    w.apply(Command::AirRecon { kingdom: ger, building: a, target: AbilityTarget::Point(target) });
    assert!(!w.events.iter().any(|e| matches!(e, aa_play::world::Event::Refused { .. })), "{:?}", w.events);
    assert_eq!(before - w.books.kingdom(ger).unwrap().stockpiled[money], 25.0, "AirReconCost money 25");
    let fighter = w.data.find("ger_fighter").expect("ger_fighter");
    let f = (first..w.actors.len() as u32).find(|&x| w.actors[x as usize].def == fighter).expect("the recon fighter");
    assert!(air::flight::flies(&w, f));
    assert!(w.is_scheduled(aa_play::scheduler::Owner::Mover(f)), "0x576B20 registers the Mover");
    let mover = w.books.actor(f).and_then(|b| b.flight_mover.clone()).expect("the Mover");
    assert_eq!(mover.requested.to_bits(), 0.1f32.to_bits(), "0x55E600: a new Mover's +0x24 is 0, below 0.1");
    assert_eq!(air::current_state(&w, f), Some(QUEUED), "waits in the hangar behind aai_queued");
    let hangar = w.actors[f as usize].pos;
    w.step(vec![]);
    let start = w.actors[f as usize].pos;
    let operational = w.data.def(fighter).aircraft.as_ref().unwrap().operational_height;
    let mut seen = Vec::new();
    let mut nearest = f32::MAX;
    let mut highest = f32::MIN;
    let mut detecting = false;
    let mut revealed = None;
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
        if revealed.is_none() && aa_play::fog::point_view(&w, Some(ger), target) == TileView::Visible {
            revealed = Some((w.time, s, p.truncate().distance(target)));
        }
    }
    eprintln!("the recon point first seen by the kingdom: {revealed:?}");
    eprintln!("{steps} steps, nearest {nearest:.2} m, highest {highest:.2} m over ground, detection on its queue {detecting}, notes {:?}", w.books.minds.notes);
    assert!(w.books.minds.notes.is_empty(), "{:?}", w.books.minds.notes);
    let mover = w.books.actor(f).and_then(|b| b.flight_mover.clone()).expect("the Mover");
    assert!(!mover.outside_map && mover.failed.is_none(), "inside the map all the way (0x576BB0's +0x88), no flight error: {mover:?}");
    assert!(w.actors[f as usize].removed, "the fighter landed and entered its base");
    assert!(hangar.distance(start) < 1e-3, "Mind +0x1B0: no move while queued");
    for s in [TAKEOFF, RECON, JOIN, LANDING] {
        assert!(seen.contains(&Some(s)), "state {s:#x} in {seen:x?}");
    }
    assert!(nearest < 5.0, "reached the recon point: {nearest}");
    assert!(detecting, "the fighter's Detection (sight 18) is on its queue while it scouts");
    assert!(revealed.is_some(), "the kingdom sees the recon point through the fighter's Detection");
    assert!(start.truncate().distance(target) > 100.0);
    assert!(highest > 0.5 * operational, "{highest} vs operational_height {operational}");
    let d = w.books.actor(a).and_then(|q| q.denizen.clone()).unwrap();
    assert!(d.slots.iter().all(|s| s.occupant != Some(f)), "housed again");
}

/// The takeoff speeds against the original's trace of a film. In the corpus
/// film of a saved game (sha256 0c01c516...) a ger kingdom with
/// tech_superchargers (SPEED_MAX +0.25) sends ger_fighters off its
/// airfields; the native sampler read each one's Mover +0x20 per tick:
/// 0, then 3.0 (taxi_speed) for 35 or 36 ticks, then ten values speeding up
/// to the Mover's +0x28 of 13.75 (speed_max 11 x 1.25), in one of two bit
/// families (the slot's position rounds differently). Here the same tech is
/// given to the ger kingdom on Berlin and the recon fighter must fly the
/// same speeds: the top speed bit for bit, the ten steps within 2e-5.
#[test]
#[ignore]
fn takeoff_speeds_match_the_original() {
    const NATIVE: [[u32; 10]; 2] = [
        [0x404f2bf2, 0x406c2b97, 0x40869d2d, 0x409974d7, 0x40aeefb1, 0x40c76c3d, 0x40e35637, 0x41019439, 0x4113b778, 0x412864a5],
        [0x404f2c0c, 0x406c2bb5, 0x40869d3f, 0x409974eb, 0x40aeefc8, 0x40c76c57, 0x40e35655, 0x4101944a, 0x4113b78b, 0x412864bc],
    ];
    let mut w = berlin_world();
    let (ger, a, at, _) = airfield(&mut w);
    let n = w.data.resources.len();
    w.books.kingdom_mut(ger, n).technologies.push("tech_superchargers".into());
    let fighter = w.data.find("ger_fighter").expect("ger_fighter");
    let middle = Vec2::new(w.map.width as f32, w.map.height as f32) * w.map.cell_size * 0.5;
    let target = at + (middle - at).normalize() * 120.0;
    let first = w.actors.len() as u32;
    w.apply(Command::AirRecon { kingdom: ger, building: a, target: AbilityTarget::Point(target) });
    let f = (first..w.actors.len() as u32).find(|&x| w.actors[x as usize].def == fighter).expect("the recon fighter");
    assert_eq!(air::flight::top_speed(&w, f).to_bits(), 13.75f32.to_bits(), "speed_max 11 x (1 + 0.25)");
    let mut speeds = Vec::new();
    let mut last_pos = w.actors[f as usize].pos;
    let mut stalls = Vec::new();
    for step in 0..400 {
        w.step(vec![]);
        if air::current_state(&w, f) == Some(QUEUED) {
            continue;
        }
        let v = w.books.actor(f).and_then(|b| b.flight_mover.as_ref()).map_or(0.0, |m| m.speed);
        let p = w.actors[f as usize].pos;
        if !speeds.is_empty() && v != 0.0 && p == last_pos {
            stalls.push(step);
        }
        last_pos = p;
        speeds.push(v.to_bits());
        if v.to_bits() == 13.75f32.to_bits() && speeds.len() > 60 {
            break;
        }
    }
    let taxi = speeds.iter().filter(|&&v| v == 3.0f32.to_bits()).count();
    let rising: Vec<u32> = speeds.iter().copied().filter(|&v| v != 0 && v != 3.0f32.to_bits() && v != 13.75f32.to_bits()).collect();
    eprintln!("taxi {taxi} ticks, rising {:?}, stalls {stalls:?}", rising.iter().map(|&v| f32::from_bits(v)).collect::<Vec<_>>());
    // The native trace also has ticks where a taxiing fighter stands still at 3.0
    // (the taxi counts of 36 and 37 there are 35 moving ticks plus one or two such
    // ticks), so a stall here is not by itself a difference.
    assert!((35..=37).contains(&taxi), "3.0 for 35 to 37 ticks, as in the original's trace: {taxi}");
    assert_eq!(rising.len(), 10, "ten steps up to the top speed");
    let close = |family: &[u32; 10]| rising.iter().zip(family).all(|(&o, &n)| (f32::from_bits(o) / f32::from_bits(n) - 1.0).abs() < 2e-5);
    assert!(close(&NATIVE[0]) || close(&NATIVE[1]), "the original's takeoff speeds: {rising:x?}");
    assert_eq!(speeds.last().copied(), Some(13.75f32.to_bits()));
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

/// MoveTick 0x5612F0 opens with Mind virtual +0x1B0 (0x51C1D0): while
/// aai_queued is current the tick does nothing, even for an aircraft that
/// has a path. Two recons are ordered at once: the second fighter waits on
/// the runway for the release delay (0x5695B0). It is given legs (0x560DB0)
/// while it waits in the hangar: it stands until the runway releases it,
/// then flies.
#[test]
#[ignore]
fn queued_aircraft_with_a_path_stands() {
    let mut w = berlin_world();
    let (ger, a, at, _) = airfield(&mut w);
    let middle = Vec2::new(w.map.width as f32, w.map.height as f32) * w.map.cell_size * 0.5;
    let target = at + (middle - at).normalize() * 120.0;
    let first = w.actors.len() as u32;
    w.apply(Command::AirRecon { kingdom: ger, building: a, target: AbilityTarget::Point(target) });
    w.apply(Command::AirRecon { kingdom: ger, building: a, target: AbilityTarget::Point(target) });
    let fighter = w.data.find("ger_fighter").expect("ger_fighter");
    let fighters: Vec<u32> = (first..w.actors.len() as u32).filter(|&x| w.actors[x as usize].def == fighter).collect();
    eprintln!("recon fighters {fighters:?}");
    let f = *fighters.last().expect("the recon fighters");
    assert_eq!(air::current_state(&w, f), Some(QUEUED));
    air::flight::fly_to(&mut w, f, glam::Vec3::new(target.x, target.y, 30.0), None);
    let segments = |w: &aa_play::World| w.books.actor(f).and_then(|b| b.aircraft.as_ref()).map_or(0, |x| x.flight.path.segments.len());
    assert!(segments(&w) > 0, "0x560DB0 gave the queued fighter legs");
    let hangar = w.actors[f as usize].pos;
    let mut queued_ticks = 0;
    while air::current_state(&w, f) == Some(QUEUED) && queued_ticks < 400 {
        w.step(vec![]);
        if air::current_state(&w, f) == Some(QUEUED) {
            queued_ticks += 1;
            assert_eq!(w.actors[f as usize].pos, hangar, "tick {queued_ticks}: queued, MoveTick moves nothing");
        }
    }
    assert!(queued_ticks > 0, "the fighter waited in the hangar");
    assert_ne!(air::current_state(&w, f), Some(QUEUED), "the runway released it");
    for _ in 0..100 {
        w.step(vec![]);
    }
    eprintln!("queued {queued_ticks} ticks, then at {:?} from {hangar:?}", w.actors[f as usize].pos);
    // The given leg was built at speed 0 and stays first on the path (the
    // takeoff's 0x516740 appends to it), so the fighter only creeps: what
    // shows here is that MoveTick runs once it is not queued.
    assert!(w.actors[f as usize].pos.distance(hangar) > 1e-3, "released, its MoveTick runs");
}

/// 0x56E730's refusals, each by its reason: air_recon carries a location
/// only; air_attack is not switched on yet; 0x4D9B60's mask refuses another
/// kingdom's order and an airfield with no recon aircraft ready.
#[test]
#[ignore]
fn air_orders_refused() {
    let mut w = berlin_world();
    let (ger, a, _, _) = airfield(&mut w);
    let first = w.actors.len();
    let refusal = |w: &aa_play::World| {
        w.events.iter().find_map(|e| match e {
            aa_play::world::Event::Refused { reason, .. } => Some(reason.clone()),
            _ => None,
        })
    };
    let corps = (0..w.actors.len() as u32).find(|&x| w.actors[x as usize].kingdom == Some(ger) && w.data.def(w.actors[x as usize].def).id.ends_with("_corps")).unwrap();
    w.step(vec![Command::AirRecon { kingdom: ger, building: a, target: AbilityTarget::Actor(corps) }]);
    let r = refusal(&w).expect("air_recon on an actor");
    assert!(r.contains("needs a location"), "{r}");
    w.step(vec![Command::AirAttack { kingdom: ger, building: a, target: AbilityTarget::Point(Vec2::new(10.0, 10.0)) }]);
    let r = refusal(&w).expect("air_attack");
    assert!(r.contains("not ported"), "past the mask, the attack itself: {r}");
    // Not the owner.
    let rus = (0..w.kingdoms.len() as u32).find(|&k| w.kingdoms[k as usize].nation == "rus").unwrap();
    w.step(vec![Command::AirRecon { kingdom: rus, building: a, target: AbilityTarget::Point(Vec2::new(10.0, 10.0)) }]);
    let r = refusal(&w).expect("another kingdom's air_recon");
    assert!(r.contains("does not take air_recon"), "{r}");
    // No recon fighter at full health: 0x5696B0 leaves the recon group out
    // of +0x3C at its next refresh.
    let fighter = w.data.find("ger_fighter").expect("ger_fighter");
    if let Some(d) = w.books.actor_mut(a).denizen.as_mut() {
        for s in d.slots.iter_mut().filter(|s| s.def == fighter) {
            s.health = 1.0;
        }
    }
    let mut steps = 0;
    while aa_play::denizen::enabled(&w, a, aa_play::denizen::COMMAND_AIR_RECON) && steps < 200 {
        w.step(vec![]);
        steps += 1;
    }
    assert!(!aa_play::denizen::enabled(&w, a, aa_play::denizen::COMMAND_AIR_RECON), "no recon aircraft ready after {steps} steps");
    w.step(vec![Command::AirRecon { kingdom: ger, building: a, target: AbilityTarget::Point(Vec2::new(10.0, 10.0)) }]);
    let r = refusal(&w).expect("air_recon with nothing ready");
    assert!(r.contains("does not take air_recon"), "{r}");
    let aircraft = (first..w.actors.len()).filter(|&x| w.data.def(w.actors[x].def).aircraft.is_some()).count();
    assert_eq!(aircraft, 0, "no aircraft created");
}

/// 0x517C60 writes Mind+0x1D0 for a created aircraft: set when any ability
/// has no projectile definition (+0x3C) or one whose ProjectileComponent
/// type (+0x2B4) is 0 (ANTIGRAV; LOB is 1 and BOMB 2). The bombs of the
/// bomber are BOMB projectiles, so it stays a bomber; the fighter is a
/// dog-fighter.
#[test]
#[ignore]
fn created_aircraft_dogfighter_byte() {
    let mut w = berlin_world();
    let ger = (0..w.kingdoms.len() as u32).find(|&k| w.kingdoms[k as usize].nation == "ger").expect("ger kingdom");
    let mut seen = Vec::new();
    for id in ["ger_bomber", "ger_fighter"] {
        let def = w.data.find(id).expect(id);
        let kinds: Vec<_> = w.data.def(def).weapons.iter().map(|x| (x.id.clone(), x.projectile.clone(), x.projectile.as_ref().and_then(|p| w.data.find(p)).map(|p| w.data.def(p).projectile_type.clone()))).collect();
        let a = aa_play::production::spawn_single(&mut w, def, Some(ger), Vec2::new(300.0, 300.0), 0.0);
        let byte = air::dogfighter(&w, a);
        eprintln!("{id}: Mind+0x1D0 = {byte}; abilities {kinds:?}");
        seen.push(byte);
    }
    assert_eq!(seen, [false, true]);
}
