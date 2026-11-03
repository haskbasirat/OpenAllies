//! The recording round trip (FILM-PLAYBACK.md F11): a headless battle in aa-play
//! is recorded, the film is read back by [`aa_parity::Film`] with zero failures
//! and replayed with the SAI off in Strict mode; every checkpoint must match to
//! the natural end and the world must be the same at the end.
//!
//! ```text
//! AA_GAME_ROOT=<game root> cargo test -p aa-parity --release --test film_round_trip -- --ignored --nocapture
//! ```
//! `AA_ROUND_TRIP_TICKS` sets the length of the battles (default 2000).
//! These films are ours, on our own S: they prove the recorder, the decree mapping
//! and aa-play's determinism, not parity with the original.
#![cfg(feature = "play")]
use aa_parity::film_writer;
use aa_parity::recording::{self, FilmSession, FilmSource, FilmTarget, GameVersion, Recorder};
use aa_parity::registries::Registries;
use aa_parity::Film;
use aa_parity::playback::{FilmData, FilmEvent, SyncMode, open_film};
use aa_play::creation::StartPick;
use aa_play::orders::ActorOrder;
use aa_play::trace::{TickDigest, digest};
use aa_play::{AbilityTarget, ActorId, Command, Controller, GameData, GameMap, PlayerSetup, SkirmishSetup, Unit, World};
use glam::Vec2;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}
fn ticks() -> u32 {
    std::env::var("AA_ROUND_TRIP_TICKS").ok().and_then(|v| v.parse().ok()).unwrap_or(2000)
}
fn data() -> Arc<GameData> {
    static DATA: OnceLock<Arc<GameData>> = OnceLock::new();
    DATA.get_or_init(|| Arc::new(GameData::load(&game_root()).unwrap())).clone()
}
fn registries() -> Arc<Registries> {
    static R: OnceLock<Arc<Registries>> = OnceLock::new();
    R.get_or_init(|| Arc::new(Registries::load_v14(&game_root(), true).unwrap())).clone()
}
fn ai() -> Arc<aa_play::ai::AiData> {
    static A: OnceLock<Arc<aa_play::ai::AiData>> = OnceLock::new();
    A.get_or_init(|| Arc::new(aa_play::ai::AiData::load(&game_root()).unwrap())).clone()
}
fn generals() -> Arc<aa_play::special_ops::GeneralData> {
    static G: OnceLock<Arc<aa_play::special_ops::GeneralData>> = OnceLock::new();
    G.get_or_init(|| Arc::new(aa_play::special_ops::GeneralData::load(&game_root(), &data()).unwrap())).clone()
}
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aa-round-trip-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const NATIONS: [&str; 4] = ["rus", "ger", "usa", "gbr"];

/// The battle's setup: the first `computers` slots after the first are computers,
/// the first is `first`.
fn setup(map: &GameMap, first: Controller, seed: u64, level: u8) -> SkirmishSetup {
    let players = map
        .player_slots
        .iter()
        .enumerate()
        .take(4)
        .map(|(start, &slot)| PlayerSetup {
            name: map.slots[slot].id.clone(),
            nation: NATIONS[start % 4].into(),
            team: map.slots[slot].team,
            color: [10 + start as u8, 20, 30],
            controller: if start == 0 { first } else { Controller::Computer(level) },
            start,
        })
        .collect();
    SkirmishSetup {
        map_id: map.id.clone(),
        players,
        starting_resources: None,
        seed,
    }
}

/// The world as the client builds it: skirmish, SAI, generals.
fn build(map: Arc<GameMap>, setup: &SkirmishSetup, picks: &[StartPick]) -> World {
    let mut world = World::new_skirmish_with(data(), map, setup, picks).unwrap();
    aa_play::ai::attach(&mut world, ai());
    aa_play::special_ops::attach(&mut world, generals());
    for k in 0..setup.players.len() as u32 {
        aa_play::special_ops::set_general(&mut world, k, None);
    }
    client_start(&mut world);
    world
}

/// What the client's first frame does before the first tick: the FogOfWar rule
/// and the fog's state for the first draw (`ui/fog.rs`).
fn client_start(world: &mut World) {
    aa_play::fog::set_mode(world, aa_play::fog::FogMode::Explored);
    aa_play::fog::ensure(world);
    world.trace.record(true);
}

fn session(source: FilmSource, setup: &SkirmishSetup, picks: &[StartPick], local: Option<u32>) -> FilmSession {
    FilmSession {
        source,
        seed: setup.seed,
        local,
        rules: Vec::new(),
        random_generals: picks
            .iter()
            .enumerate()
            .filter(|(_, p)| p.random_general)
            .map(|(k, _)| k as u32)
            .collect(),
        random_nations: BTreeSet::new(),
        sai_profiles: BTreeMap::new(),
        player_names: BTreeMap::new(),
        version: GameVersion::load(&game_root()).unwrap(),
        filetime: recording::filetime_now(),
        extras: Default::default(),
    }
}

fn order(kingdom: u32, unit: Unit, order: ActorOrder) -> Command {
    Command::Order { kingdom, units: vec![unit], order }
}

/// A battle of orders that touches every `Command` of the first kingdom, on
/// whatever stands at that moment; the world may refuse some, the same way twice.
struct Script;
impl Script {
    fn commands(world: &World, tick: u32, k: u32) -> Vec<Command> {
        let mine: Vec<ActorId> = world
            .actors
            .iter()
            .filter(|a| a.alive && !a.removed && a.kingdom == Some(k))
            .map(|a| a.id)
            .collect();
        let orgs: Vec<u32> = world.orgs.iter().filter(|o| o.alive && o.kingdom == k).map(|o| o.id).collect();
        let enemy = world
            .actors
            .iter()
            .find(|a| a.alive && !a.removed && a.kingdom.is_some() && a.kingdom != Some(k) && aa_play::setup::relation(world, Some(k), a.kingdom) == aa_play::books::ENEMY)
            .map(|a| a.id);
        let is_building = |a: ActorId| world.actors[a as usize].org.is_none() && world.data.def(world.actors[a as usize].def).category.eq_ignore_ascii_case("BUILDING");
        let recruiter = mine.iter().copied().find(|&a| is_building(a) && !world.data.def(world.actors[a as usize].def).recruits.is_empty());
        let building = recruiter.or(mine.iter().copied().find(|&a| is_building(a)));
        let researcher = mine.iter().copied().find_map(|a| world.data.def(world.actors[a as usize].def).research.first().map(|r| (a, r.property.clone())));
        let truck = mine.iter().copied().find(|&a| world.data.def(world.actors[a as usize].def).id.to_ascii_lowercase().contains("truck"));
        let unit = orgs.first().map(|&o| Unit::Org(o));
        let units: Vec<Unit> = orgs.iter().map(|&o| Unit::Org(o)).collect();
        let anchor = orgs.first().map_or(Vec2::new(64.0, 64.0), |&o| world.orgs[o as usize].anchor);
        let at = anchor + Vec2::new(8.0 + (tick % 5) as f32, -6.0);
        let recruit = recruiter.and_then(|b| world.data.def(world.actors[b as usize].def).recruits.first().copied());
        let formation = orgs.first().map(|&o| world.data.def(world.orgs[o as usize].def).formations.len()).unwrap_or(0);
        // The late orders only need something to name: the world may refuse them.
        let any_building = building.or(mine.first().copied());
        let any_def = recruit.or(mine.first().map(|&a| world.actors[a as usize].def));
        let friend = world.kingdoms.iter().find(|x| x.id != k && !x.independent && x.team == world.kingdoms[k as usize].team).map_or(k, |x| x.id);
        let mut out = Vec::new();
        match tick {
            20 => out.extend(truck.map(|truck| Command::Deploy { kingdom: k, truck, at: world.actors[truck as usize].pos.truncate() + Vec2::new(12.0, 12.0), facing: -std::f32::consts::FRAC_PI_2 })),
            40 | 60 | 80 => out.extend(recruiter.zip(recruit).map(|(building, def)| Command::Recruit { kingdom: k, building, def })),
            70 => out.extend(recruiter.zip(recruit).map(|(building, def)| Command::RecruitWith { kingdom: k, building, def, parameters: recording::own_parameters(world, def) })),
            90 => out.extend(recruiter.map(|building| Command::CancelRecruit { kingdom: k, building, index: 1 })),
            100 => out.extend(building.map(|building| Command::SetRallyPoint { kingdom: k, building, at })),
            // Only what the building has: 0x5849A0 looks the name up in the
            // building and the original dereferences what it finds (it crashed
            // on a film of this test that named something else).
            110 => out.extend(researcher.map(|(building, property)| Command::Research { kingdom: k, building, property })),
            1000 => out.extend((!units.is_empty()).then(|| Command::Move { kingdom: k, units: units.clone(), to: at, attack_move: false })),
            1030 => out.extend(unit.map(|u| Command::Move { kingdom: k, units: vec![u], to: at, attack_move: true })),
            1060 => out.extend(unit.map(|u| Command::Stop { kingdom: k, units: vec![u] })),
            1090 => out.extend(unit.zip(enemy).map(|(u, target)| Command::Attack { kingdom: k, units: vec![u], target })),
            1120 => out.extend(unit.zip(enemy).map(|(u, target)| Command::Follow { kingdom: k, units: vec![u], target })),
            1150 => out.extend(unit.map(|u| Command::Bombard { kingdom: k, units: vec![u], target: AbilityTarget::Point(at) })),
            1180 => out.extend(unit.zip(building).map(|(u, target)| Command::Repair { kingdom: k, units: vec![u], target })),
            1210 if formation > 0 => out.extend(orgs.first().map(|&o| Command::SetFormation { kingdom: k, orgs: vec![o], formation: 1.min(formation - 1) })),
            1240 => out.extend(orgs.first().copied().zip(building).map(|(org, hq)| Command::Attach { kingdom: k, org, hq })),
            1270 => out.extend(orgs.first().map(|&org| Command::Detach { kingdom: k, org })),
            1300 => out.push(Command::UseAbility { kingdom: k, ability: 0, target: Some(AbilityTarget::Point(at)) }),
            1330 => out.push(Command::UseAbility { kingdom: k, ability: 1, target: enemy.map(AbilityTarget::Actor) }),
            1360 => out.extend(building.map(|building| Command::AirAttack { kingdom: k, building, target: AbilityTarget::Point(at) })),
            1390 => out.extend(building.map(|building| Command::AirRecon { kingdom: k, building, target: AbilityTarget::Point(at) })),
            1420 => out.extend(unit.zip(recruit).map(|(u, def)| Command::Construct { kingdom: k, units: vec![u], def, at, facing: 0.75 })),
            1480 => out.extend(building.map(|building| Command::Pack { kingdom: k, building })),
            1560 => out.extend(unit.map(|u| order(k, u, ActorOrder::Guard { target: None, at, aux: None }))),
            1590 => out.extend(unit.zip(building).map(|(u, b)| order(k, u, ActorOrder::Guard { target: Some(Unit::Actor(b)), at, aux: None }))),
            1620 => out.extend(unit.map(|u| order(k, u, ActorOrder::Retreat { target: None, at }))),
            1650 => out.extend(unit.map(|u| order(k, u, ActorOrder::Explore { at, scalar: None }))),
            1680 => out.extend(unit.map(|u| order(k, u, ActorOrder::Rout))),
            1710 => out.extend(unit.map(|u| order(k, u, ActorOrder::HqCycle))),
            1740 => out.extend(unit.map(|u| order(k, u, ActorOrder::PackOrganization))),
            1745 => out.extend(unit.map(|u| order(k, u, ActorOrder::Unpack { at, facing: 0.75 }))),
            1770 => out.extend(unit.map(|u| order(k, u, ActorOrder::Escort { on: true }))),
            1775 => out.extend(unit.map(|u| order(k, u, ActorOrder::Escort { on: false }))),
            1780 => out.extend(unit.map(|u| order(k, u, ActorOrder::DetachElement))),
            1785 => out.extend(unit.zip(any_def).map(|(u, def)| order(k, u, ActorOrder::InventoryAttach { def }))),
            1790 => out.extend(any_building.zip(any_def).map(|(b, def)| order(k, Unit::Actor(b), ActorOrder::RecruitAt { def, at, parameters: Some(recording::own_parameters(world, def)) }))),
            1795 => out.extend(any_building.map(|b| order(k, Unit::Actor(b), ActorOrder::CancelQueued { id: 1 }))),
            1800 => out.extend(any_building.zip(any_def).map(|(b, def)| order(k, Unit::Actor(b), ActorOrder::Commission { def }))),
            1805 => out.extend(any_building.zip(any_def).map(|(b, def)| order(k, Unit::Actor(b), ActorOrder::BuildStructure { def, at }))),
            1810 => out.extend(any_building.map(|b| order(k, Unit::Actor(b), ActorOrder::ConstructionCancel))),
            1815 => out.extend(any_building.zip(any_def).map(|(b, def)| order(k, Unit::Actor(b), ActorOrder::Upgrade { def }))),
            1820 => out.extend(any_building.map(|b| order(k, Unit::Actor(b), ActorOrder::Raze))),
            1825 => out.extend(any_building.map(|b| order(k, Unit::Actor(b), ActorOrder::Sally { forth: true }))),
            1830 => out.extend(any_building.map(|b| order(k, Unit::Actor(b), ActorOrder::Sally { forth: false }))),
            1835 => out.extend(unit.map(|u| order(k, u, ActorOrder::CancelFlare))),
            1840 => out.extend((!units.is_empty()).then(|| Command::Order { kingdom: k, units: units.clone(), order: ActorOrder::GroupBehavior { enable: true, boolean: None } })),
            1845 => out.extend((!units.is_empty()).then(|| Command::Order { kingdom: k, units: units.clone(), order: ActorOrder::GroupBehavior { enable: false, boolean: None } })),
            1850 => out.extend(unit.map(|u| order(k, u, ActorOrder::Explore { at, scalar: Some(1.0) }))),
            1855 => out.extend(unit.map(|u| order(k, u, ActorOrder::Guard { target: None, at, aux: Some(Vec2::new(1.0, 0.0)) }))),
            1860 => out.push(Command::Order { kingdom: k, units: Vec::new(), order: ActorOrder::GiveResources { to: friend, amount: 25.0 } }),
            1865 => out.extend(mine.last().map(|&a| Command::Order { kingdom: k, units: Vec::new(), order: ActorOrder::GiveActor { to: friend, actor: Unit::Actor(a) } })),
            1900 => out.extend(orgs.last().map(|&o| Command::Disband { kingdom: k, units: vec![Unit::Org(o)] })),
            1930 => out.extend(building.or(mine.first().copied()).map(|building| Command::Sell { kingdom: k, building })),
            _ => {}
        }
        out
    }
}

/// The world after one tick, for naming the first thing that differs.
#[derive(Clone, PartialEq)]
struct Snapshot {
    stockpiles: Vec<[u32; 3]>,
    actors: Vec<(u32, Option<u32>, [u32; 4], u32, bool, bool)>,
    orgs: Vec<(usize, [u32; 2], bool, String)>,
}
fn snapshot(w: &World) -> Snapshot {
    Snapshot {
        stockpiles: w.kingdoms.iter().map(|k| k.stockpile.map(f32::to_bits)).collect(),
        actors: w
            .actors
            .iter()
            .map(|a| (a.def, a.kingdom, [a.pos.x.to_bits(), a.pos.y.to_bits(), a.facing.to_bits(), a.health.to_bits()], a.health.to_bits(), a.alive, a.removed))
            .collect(),
        orgs: w.orgs.iter().map(|o| (o.members.len(), [o.anchor.x.to_bits(), o.anchor.y.to_bits()], o.alive, format!("{:?}", o.order))).collect(),
    }
}
fn first_difference(a: &Snapshot, b: &Snapshot, w: &World) -> String {
    if a.stockpiles != b.stockpiles {
        return format!("stockpiles {:?} / {:?}", a.stockpiles, b.stockpiles);
    }
    if a.actors.len() != b.actors.len() {
        return format!("actors {} / {}", a.actors.len(), b.actors.len());
    }
    for (i, (x, y)) in a.actors.iter().zip(&b.actors).enumerate() {
        if x != y {
            return format!("actor {i} {}: {x:?} / {y:?}", w.data.def(x.0).id);
        }
    }
    if a.orgs.len() != b.orgs.len() {
        return format!("orgs {} / {}", a.orgs.len(), b.orgs.len());
    }
    for (i, (x, y)) in a.orgs.iter().zip(&b.orgs).enumerate() {
        if x != y {
            return format!("org {i}: {x:?} / {y:?}");
        }
    }
    "the snapshots agree (S or the RNG differ)".into()
}

struct Recorded {
    path: PathBuf,
    digests: Vec<TickDigest>,
    snapshots: Vec<Snapshot>,
    world: World,
    stats: recording::RecordingStats,
    kinds: BTreeSet<&'static str>,
}

fn kind(c: &Command) -> &'static str {
    match c {
        Command::Move { .. } => "Move",
        Command::Attack { .. } => "Attack",
        Command::Stop { .. } => "Stop",
        Command::Recruit { .. } => "Recruit",
        Command::CancelRecruit { .. } => "CancelRecruit",
        Command::Deploy { .. } => "Deploy",
        Command::Pack { .. } => "Pack",
        Command::Research { .. } => "Research",
        Command::Attach { .. } => "Attach",
        Command::SetFormation { .. } => "SetFormation",
        Command::SetRallyPoint { .. } => "SetRallyPoint",
        Command::Sell { .. } => "Sell",
        Command::Detach { .. } => "Detach",
        Command::Disband { .. } => "Disband",
        Command::UseAbility { .. } => "UseAbility",
        Command::Follow { .. } => "Follow",
        Command::Bombard { .. } => "Bombard",
        Command::Repair { .. } => "Repair",
        Command::Construct { .. } => "Construct",
        Command::AirAttack { .. } => "AirAttack",
        Command::AirRecon { .. } => "AirRecon",
        Command::Select { .. } => "Select",
        Command::Hot { .. } => "Hot",
        Command::Order { order, .. } => match order {
            ActorOrder::Guard { .. } => "Guard",
            ActorOrder::Retreat { .. } => "Retreat",
            ActorOrder::Rout => "Rout",
            ActorOrder::Explore { .. } => "Explore",
            ActorOrder::PackOrganization => "PackOrganization",
            ActorOrder::HqCycle => "HqCycle",
            ActorOrder::DetachElement => "DetachElement",
            ActorOrder::InventoryAttach { .. } => "InventoryAttach",
            ActorOrder::RecruitAt { .. } => "RecruitAt",
            ActorOrder::Commission { .. } => "Commission",
            ActorOrder::BuildStructure { .. } => "BuildStructure",
            ActorOrder::CancelQueued { .. } => "CancelQueued",
            ActorOrder::ConstructionCancel => "ConstructionCancel",
            ActorOrder::Upgrade { .. } => "Upgrade",
            ActorOrder::Raze => "Raze",
            ActorOrder::Sally { .. } => "Sally",
            ActorOrder::Escort { .. } => "Escort",
            ActorOrder::CancelFlare => "CancelFlare",
            ActorOrder::GroupBehavior { .. } => "GroupBehavior",
            ActorOrder::GroupFormed => "GroupFormed",
            ActorOrder::Facing { .. } => "Facing",
            ActorOrder::MoveOnto => "MoveOnto",
            ActorOrder::Unpack { .. } => "Unpack",
            ActorOrder::GiveResources { .. } => "GiveResources",
            ActorOrder::GiveActor { .. } => "GiveActor",
            // New since this list was made.
            #[allow(unreachable_patterns)]
            _ => "other",
        },
        Command::RecruitWith { .. } => "RecruitWith",
    }
}

const SNAPSHOT_TICKS: u32 = 3000;

/// A recorded battle: `first`'s kingdom on the script when it is human.
fn record(mut world: World, session: &FilmSession, name: &str, ticks: u32, scripted: bool) -> Recorded {
    let dir = scratch(name);
    println!("record start: rng {:#x}, S {:?}, generals {:?}", world.native_rng.state, world.trace.checkpoint(), (0..world.kingdoms.len() as u32).map(|k| aa_play::special_ops::general(&world, k).map(|g| g.id.clone())).collect::<Vec<_>>());
    let prelude = recording::prelude(&world, &game_root(), session).unwrap();
    let target = FilmTarget {
        folder: dir,
        base: "LastFilm".into(),
        last_film_index: 0,
        queue: 5,
    };
    let (file, opened) = recording::open_film_file(&prelude, &target).unwrap_or_else(|e| panic!("{e:?}"));
    let path = opened.path.clone();
    let mut recorder = Recorder::new(&world, registries(), Some((file, opened)));
    let mut digests = Vec::new();
    let mut snapshots = Vec::new();
    let mut kinds = BTreeSet::new();
    for t in 0..ticks {
        let commands = if scripted { Script::commands(&world, t, 0) } else { Vec::new() };
        kinds.extend(commands.iter().chain(world.sai_orders.iter()).map(kind));
        recorder.step(&mut world, commands);
        digests.push(digest(&world));
        // Only to name a first difference; a long battle keeps the first ones.
        if t < SNAPSHOT_TICKS {
            snapshots.push(snapshot(&world));
        }
        // A film has no end of its own; the trace log need not grow for hours.
        world.trace.digests.clear();
    }
    recorder.close().unwrap();
    // AA_ROUND_TRIP_KEEP=<dir>: keep the films (the proof with the original).
    if let Some(keep) = std::env::var_os("AA_ROUND_TRIP_KEEP") {
        let keep = PathBuf::from(keep);
        std::fs::create_dir_all(&keep).unwrap();
        std::fs::copy(&path, keep.join(format!("{name}.RFM"))).unwrap();
        std::fs::copy(path.with_extension("TGI"), keep.join(format!("{name}.TGI"))).unwrap();
    }
    Recorded {
        path,
        digests,
        snapshots,
        world,
        stats: recorder.stats().clone(),
        kinds,
    }
}

fn film_of(path: &Path) -> Film {
    Film::read(path, &registries()).unwrap()
}

/// Play the film through the playback library with the SAI off, Strict: every
/// checkpoint must pass, to the natural end, and the world must be the same.
fn replay_film(path: &Path, recorded: &Recorded) {
    let clock = std::time::Instant::now();
    let (mut world, mut player) = open_film(path, &FilmData::new(game_root()), SyncMode::Strict).unwrap_or_else(|e| panic!("open_film: {e:?}"));
    println!("open_film took {:?}", clock.elapsed());
    println!("replay start: rng {:#x}, S {:?}, generals {:?}", world.native_rng.state, world.trace.checkpoint(), (0..world.kingdoms.len() as u32).map(|k| aa_play::special_ops::general(&world, k).map(|g| g.id.clone())).collect::<Vec<_>>());
    println!("notes: {:?}", player.notes());
    assert_eq!(player.film_ticks() as usize, recorded.digests.len());
    let mut first = None;
    let mut end = None;
    for step in 0..=recorded.digests.len() {
        let events = player.step(&mut world);
        for e in events {
            match e {
                FilmEvent::EndOfFilm { out_of_sync } => end = end.or(Some((step, out_of_sync))),
                FilmEvent::FirstMismatch { index } => println!("step {step}: FILM OUT OF SYNC({index})"),
                _ => {}
            }
        }
        if let (None, Some(r)) = (&first, recorded.digests.get(step)) {
            let d = digest(&world);
            if (r.world, r.rng_state, r.s_end, r.s_hash) != (d.world, d.rng_state, d.s_end, d.s_hash) {
                let at = recorded.snapshots.get(step).map_or_else(|| "(no snapshot kept)".to_owned(), |s| first_difference(s, &snapshot(&world), &world));
                first = Some((step, *r, d, at));
            }
        }
        world.trace.digests.clear();
    }
    let stats = player.stats().clone();
    println!(
        "played {} ticks, checkpoints {}/{}, first mismatch {:?}, end {:?}",
        stats.ticks, stats.checkpoints_matched, stats.checkpoints_checked, stats.first_mismatch, end
    );
    println!("unresolved: {:?}", stats.unresolved);
    println!("refused: {:?}", stats.refused);
    println!("dropped: {:?}", stats.dropped);
    if let Some((step, r, d, at)) = &first {
        println!("first world difference after step {step}:\n recorded {r:?}\n replayed {d:?}\n {at}");
    }
    assert!(first.is_none(), "the worlds differ");
    assert!(stats.checkpoints_checked > 0, "no checkpoints");
    assert_eq!(stats.checkpoints_matched, stats.checkpoints_checked, "checkpoints");
    assert_eq!(end, Some((recorded.digests.len(), false)), "the natural end");
    assert!(stats.unresolved.is_empty(), "unresolved orders");
}

fn check_film(recorded: &Recorded, ticks: u32) -> Film {
    let film = film_of(&recorded.path);
    assert_eq!(film.failures().count(), 0, "decode failures");
    assert_eq!(film.world_ticks() as u32, ticks);
    println!(
        "film: {} bytes, {} ticks, {} decrees, {} checkpoints, unmapped {:?}",
        film.source.bytes,
        film.world_ticks(),
        film.decree_count(),
        film.checkpoints.len(),
        recorded.stats.unmapped
    );
    println!("histogram {:?}", film.histogram());
    println!("command kinds issued: {:?}", recorded.kinds);
    assert!(recorded.stats.unmapped.is_empty(), "commands the film cannot carry: {:?}", recorded.stats.unmapped);
    assert_eq!(film.checkpoints.first().map(|c| c.tick), Some(17), "the first checkpoint");
    film
}

const MAP: &str = "(2v2)Berlin";

/// A stock map: three computers and a human on the script, every Command kind.
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn stock_map_round_trip() {
    let root = game_root();
    let map = Arc::new(GameMap::load_with(&root, MAP, &data()).unwrap());
    let setup = setup(&map, Controller::Human, 0x1234_5678, 1);
    let picks = vec![StartPick { random_nation: false, random_general: true }; setup.players.len()];
    let world = build(map.clone(), &setup, &picks);
    let session = session(FilmSource::Map { id: MAP.into() }, &setup, &picks, Some(0));
    let ticks = ticks();
    let recorded = record(world, &session, "stock", ticks, true);
    check_film(&recorded, ticks);
    // Every Command the client or the computer can issue went through.
    for k in ["Move", "Attack", "Stop", "Recruit", "RecruitWith", "CancelRecruit", "Deploy", "Pack", "Research", "Attach", "SetFormation", "SetRallyPoint", "Sell", "Detach", "Disband", "UseAbility", "Follow", "Bombard", "Repair", "Construct", "AirAttack", "AirRecon", "Guard", "Retreat", "Rout", "Explore", "PackOrganization", "Unpack", "HqCycle", "DetachElement", "InventoryAttach", "RecruitAt", "Commission", "BuildStructure", "CancelQueued", "ConstructionCancel", "Upgrade", "Raze", "Sally", "Escort", "CancelFlare", "GroupBehavior", "GiveResources", "GiveActor"] {
        assert!(recorded.kinds.contains(k), "the battle never issued {k}");
    }
    replay_film(&recorded.path, &recorded);
}

/// A stock map with only the computers giving orders (the first slot idle): the
/// film for the proof with the original, which then checks every checkpoint with
/// its own S on the orders of our SAI.
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn stock_map_computers_round_trip() {
    let root = game_root();
    let map = Arc::new(GameMap::load_with(&root, MAP, &data()).unwrap());
    let setup = setup(&map, Controller::Human, 0x1234_5678, 1);
    let picks = vec![StartPick { random_nation: false, random_general: true }; setup.players.len()];
    let world = build(map.clone(), &setup, &picks);
    let session = session(FilmSource::Map { id: MAP.into() }, &setup, &picks, Some(0));
    let ticks = ticks();
    let recorded = record(world, &session, "stock-computers", ticks, false);
    let film = check_film(&recorded, ticks);
    // The computers' orders in the shapes of the real films (the census of
    // `order_shapes_against_the_original_s`): an organization's recruit holds its
    // ParameterSet, an unpack no direction, and nobody writes a scalar.
    let mut recruits = 0;
    for d in film.ticks.iter().flat_map(|t| &t.decrees) {
        let aa_parity::Decree::TellSelectedCommand(w) = &d.decree else { continue };
        let name = aa_parity::classify::actor_command(&d.decree).map_or("default", |(_, n, _)| n);
        assert_eq!(w.flags & 0x10, 0, "{name} {:#x}: a computer's order has no scalar", w.flags);
        match name {
            "recruit" => {
                let id = &registries().object_ids[w.object.unwrap() as usize];
                let org = data().find(id).is_some_and(|def| data().def(def).layout.is_some());
                assert_eq!(w.flags, if org { 0x62 } else { 0x42 }, "recruit of {id}");
                recruits += 1;
            }
            "unpack" => assert_eq!(w.flags, 0x44),
            "construct" => assert_eq!(w.flags, 0x46),
            "hq_attach" => assert_eq!(w.flags, 0x41),
            _ => {}
        }
    }
    assert!(recruits > 0, "no recruit in the film");
    replay_film(&recorded.path, &recorded);
}

/// A random map: all computers, the host watching.
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn random_map_round_trip() {
    let root = game_root();
    let maps = aa_play::mapgen::RandomMapData::load(&root).unwrap();
    let catalog = maps.catalog();
    let info = catalog.rmcs.iter().find(|r| r.ids == catalog.default_rmc).unwrap();
    let params = aa_play::mapgen::RandomMapParams::defaults(info, 3, 0x0BAD_F00D);
    // The slots to give the players, then the battle as the client starts it.
    let clock = std::time::Instant::now();
    let slots = maps.generate(&params).unwrap().map;
    println!("generation took {:?}", clock.elapsed());
    let setup = setup(&slots, Controller::Computer(1), params.seed, 1);
    let battle = recording::random_battle(&maps, &data(), &params, &setup, &[]).unwrap();
    let map = Arc::new(battle.generated.map.clone());
    // A random map's creator draws its picks before the generator, not at the
    // restore; the creator keeps them open.
    let open = vec![StartPick { random_nation: false, random_general: true }; setup.players.len()];
    let none = vec![StartPick::default(); setup.players.len()];
    let mut world = World::new_skirmish_with(data(), map.clone(), &setup, &none).unwrap();
    aa_play::ai::attach(&mut world, ai());
    aa_play::special_ops::attach(&mut world, generals());
    for (k, general) in battle.generals.iter().enumerate() {
        aa_play::special_ops::set_general(&mut world, k as u32, general.as_deref());
    }
    client_start(&mut world);
    let picks = open;
    let parts = maps.creator_parts(&params).unwrap();
    let params_weights: Vec<f32> = parts.terrain_weights.iter().map(|t| t.1).collect();
    let session = session(FilmSource::Random { params: params.clone(), parts }, &setup, &picks, None);
    let ticks = ticks();
    let recorded = record(world, &session, "random", ticks, false);
    let film = check_film(&recorded, ticks);
    assert_eq!(film.header.creator_class, "KKC_WorldCreatorRandom");
    // The film's settings are the battle's: same generator input.
    let read = aa_parity::playback::random_params(&film, &maps).unwrap();
    assert_eq!(read.rmc, params.rmc);
    assert_eq!((read.width, read.height, read.sides, read.heroes, read.seed), (params.width, params.height, params.sides, params.heroes, params.seed));
    assert_eq!(read.teams, params.teams);
    let again = maps.generate_world(&read, &aa_parity::playback::world_spec(&film), &data()).unwrap().map;
    println!("rng state: generated {:?}, from the film's settings {:?}", map.rng_state, again.rng_state);
    assert_eq!(read.terrain_weights, params_weights, "terrain weights");
    assert_eq!(again.rng_state, map.rng_state, "the same map from the film's settings");
    replay_film(&recorded.path, &recorded);
}

/// A scripted map of the campaign: its triggers run in the battle and in the film.
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn scripted_map_round_trip() {
    let root = game_root();
    const MISSION: &str = "Campaign/allies01_el_alamein";
    let map = Arc::new(GameMap::load_with(&root, MISSION, &data()).unwrap());
    let setup = setup(&map, Controller::Human, 0x55AA_1234, 1);
    let picks = vec![StartPick { random_nation: false, random_general: true }; setup.players.len()];
    let world = build(map.clone(), &setup, &picks);
    let session = session(FilmSource::Map { id: MISSION.into() }, &setup, &picks, Some(0));
    let ticks = ticks().min(1500);
    let recorded = record(world, &session, "scripted", ticks, false);
    check_film(&recorded, ticks);
    replay_film(&recorded.path, &recorded);
}

/// A metagame battle: a random map with the launch orders' extras, which the film's
/// rows must bring back. Our own reading of the rows: no film of the original to
/// compare with.
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn metagame_battle_round_trip() {
    let root = game_root();
    let maps = aa_play::mapgen::RandomMapData::load(&root).unwrap();
    let catalog = maps.catalog();
    let info = catalog.rmcs.iter().find(|r| r.ids == catalog.default_rmc).unwrap();
    let params = aa_play::mapgen::RandomMapParams::defaults(info, 3, 0x0BAD_F00D);
    let slots = maps.generate(&params).unwrap().map;
    let setup = setup(&slots, Controller::Computer(1), params.seed, 1);
    let battle = recording::random_battle(&maps, &data(), &params, &setup, &[]).unwrap();
    let map = Arc::new(battle.generated.map.clone());
    let none = vec![StartPick::default(); setup.players.len()];
    let open = vec![StartPick { random_nation: false, random_general: true }; setup.players.len()];
    let mut world = World::new_skirmish_with(data(), map, &setup, &none).unwrap();
    aa_play::ai::attach(&mut world, ai());
    aa_play::special_ops::attach(&mut world, generals());
    for (k, general) in battle.generals.iter().enumerate() {
        aa_play::special_ops::set_general(&mut world, k as u32, general.as_deref());
    }
    client_start(&mut world);
    let actor = world.orgs.first().map(|o| world.data.def(o.def).id.clone()).unwrap_or_else(|| world.data.def(world.actors[0].def).id.clone());
    let resource = world.data.resources[0].id.clone();
    let extras = recording::BattleExtras {
        properties: vec![(0, "metagame_test_technology".to_owned())],
        bonus: vec![(1, resource, 250f32.to_bits())],
        actors: vec![(0, actor.clone()), (2, actor)],
    };
    recording::apply_battle_extras(&mut world, &extras);
    let parts = maps.creator_parts(&params).unwrap();
    let mut session = session(FilmSource::Random { params: params.clone(), parts }, &setup, &open, None);
    session.extras = extras.clone();
    let ticks = ticks().min(600);
    let recorded = record(world, &session, "metagame", ticks, false);
    let film = check_film(&recorded, ticks);
    let names: Vec<String> = setup.players.iter().map(|p| p.name.clone()).collect();
    assert_eq!(recording::film_extras(&film.header.map_header, &names), extras);
    replay_film(&recorded.path, &recorded);
}

/// Recording does not change the world: the same battle on the decree stream with
/// a file and without one (a test launch), the world and S the same at every tick.
/// The plain `World::step` is told apart on purpose: orders do not pass the Mind
/// handlers' shaping there, so its battle is another one; with no orders at all it
/// is the same world.
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn the_world_is_the_same_with_recording_on_and_off() {
    let root = game_root();
    let map = Arc::new(GameMap::load_with(&root, MAP, &data()).unwrap());
    let setup = setup(&map, Controller::Human, 77, 1);
    let picks = vec![StartPick { random_nation: false, random_general: true }; setup.players.len()];
    let ticks = ticks();
    // The stream, no file (a test launch).
    let mut stream = build(map.clone(), &setup, &picks);
    let mut recorder = Recorder::new(&stream, registries(), None);
    let mut stream_digests = Vec::new();
    for t in 0..ticks {
        let c = Script::commands(&stream, t, 0);
        recorder.step(&mut stream, c);
        stream_digests.push(digest(&stream));
        stream.trace.digests.clear();
    }
    // With a file.
    let world = build(map.clone(), &setup, &picks);
    let session = session(FilmSource::Map { id: MAP.into() }, &setup, &picks, Some(0));
    let recorded = record(world, &session, "onoff", ticks, true);
    for t in 0..ticks as usize {
        let (b, c) = (&stream_digests[t], &recorded.digests[t]);
        assert_eq!((b.world, b.rng_state, b.s_end, b.s_hash), (c.world, c.rng_state, c.s_end, c.s_hash), "tick {t}: stream with and without a file");
    }
    // Nobody orders anything (no computers): the plain step and the empty stream.
    let quiet = SkirmishSetup { players: setup.players.iter().map(|p| PlayerSetup { controller: Controller::Human, ..p.clone() }).collect(), ..setup.clone() };
    let mut plain = build(map.clone(), &quiet, &picks);
    let mut empty = build(map, &quiet, &picks);
    let mut recorder = Recorder::new(&empty, registries(), None);
    for t in 0..ticks.min(1000) {
        plain.step(Vec::new());
        recorder.step(&mut empty, Vec::new());
        let (a, b) = (digest(&plain), digest(&empty));
        assert_eq!((a.world, a.rng_state, a.time_bits), (b.world, b.rng_state, b.time_bits), "tick {t}: plain step and empty stream");
        plain.trace.digests.clear();
        empty.trace.digests.clear();
    }
    println!("{ticks} ticks: the same world and S with a file and without one");
}

/// The same for a random-map film of the original: the film's world, the header
/// written again from the world and the settings.
#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn film_header_of_a_random_map_is_the_original_s() {
    let root = game_root();
    let dir = root.join("reconstruction/qa-runs/compare-20261022/game/Profile/data/Films");
    let maps = aa_play::mapgen::RandomMapData::load(&root).unwrap();
    let mut compared = 0;
    let mut differ = Vec::new();
    // Every random-map film whose settings fit this data (0x505B00), the rest was
    // recorded on other data.
    let mut paths = vec![dir.join("rmgf2.RFM")];
    if let Some(corpus) = std::env::var_os("AA_FILM_CORPUS") {
        let mut more: Vec<PathBuf> = std::fs::read_dir(corpus).unwrap().filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("RFM"))).collect();
        more.sort();
        paths.extend(more);
    }
    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let name = name.as_str();
        let Ok(film) = Film::read(path, &registries()) else { continue };
        let aa_mapworld::header::DerivedCreator::Random(s) = &film.header.creator else { continue };
        let bits = |list: &[(Vec<u16>, u32)]| -> Vec<(String, f32)> { list.iter().map(|(i, b)| (String::from_utf16_lossy(i), f32::from_bits(*b))).collect() };
        let fits = maps.film_settings(&String::from_utf16_lossy(&s.rmc), &bits(&s.terrain_weights), &bits(&s.object_counts), s.num_heroes).is_ok_and(|f| f.fits);
        if !fits || film.failures().next().is_some() {
            println!("{name}: other data");
            continue;
        }
        let (mut world, player) = match open_film(path, &FilmData::new(root.clone()), SyncMode::Report) {
            Ok(x) => x,
            Err(e) => {
                println!("{name}: open_film {e:?}");
                continue;
            }
        };
        let params = player.scene().random.clone().expect("a random map");
        // The plan numbers a kingdom with no team past the map's teams, which here
        // are the sides' and not the RMC's: as the client does, past the RMC's.
        for k in 0..world.kingdoms.len() {
            let slot = world.ids.kingdom_slot(k as u32).map(|s| world.map.slots[s].id.clone());
            if slot.is_some_and(|id| film.header.map_header.parameters.kingdoms.iter().any(|r| String::from_utf16_lossy(&r.id).eq_ignore_ascii_case(&id) && r.string_14.is_empty())) {
                world.kingdoms[k].team = 1000 + k as u32;
            }
        }
        let parts = maps.creator_parts(&params).unwrap();
        let header = &film.header.map_header;
        let text = |u: &[u16]| String::from_utf16_lossy(u);
        let rules = header.creator.rules.rules.iter().map(|r| (text(&r.key), r.kind, r.value)).collect();
        let mut sai_profiles = BTreeMap::new();
        let mut random_generals = BTreeSet::new();
        let mut random_nations = BTreeSet::new();
        let mut player_names = BTreeMap::new();
        let mut local = None;
        for k in world.kingdoms.iter().filter(|k| !k.independent) {
            let slot = world.ids.kingdom_slot(k.id).map(|s| world.map.slots[s].id.clone()).unwrap();
            let row = header.parameters.kingdoms.iter().find(|r| text(&r.id).eq_ignore_ascii_case(&slot)).unwrap();
            let creator_row = header.creator.parameters.kingdoms.iter().find(|r| text(&r.id).eq_ignore_ascii_case(&slot)).unwrap();
            if !row.string_2c.is_empty() {
                sai_profiles.insert(k.id, text(&row.string_2c));
            }
            if creator_row.faction.is_empty() {
                random_generals.insert(k.id);
            }
            if creator_row.nation.is_empty() {
                random_nations.insert(k.id);
            }
            if let Some(entry) = header.entries.iter().find(|e| text(&e.string_20).eq_ignore_ascii_case(&slot)) {
                player_names.insert(k.id, text(&entry.string_18));
                if entry.flag_v5 == Some(1) {
                    local = Some(k.id);
                }
            }
        }
        let session = FilmSession {
            source: FilmSource::Random { params, parts },
            seed: u64::from(film.header.seed),
            local,
            rules,
            random_generals,
            random_nations,
            sai_profiles,
            player_names,
            version: GameVersion::load(&root).unwrap(),
            filetime: header.words_18_1c,
            extras: Default::default(),
        };
        let mut prelude = recording::prelude(&world, &root, &session).unwrap();
        // The entries are in the order the session's players were made in, which
        // staging decides and a world does not hold.
        prelude.header.entries.sort_by_key(|e| header.entries.iter().position(|x| x.string_20 == e.string_20));
        let ours = film_writer::map_header_bytes(&prelude.header, &prelude.creator);
        let theirs = film_writer::map_header_bytes(header, &film.header.creator);
        compared += 1;
        if ours == theirs {
            println!("{name}: the film header is the original's");
            continue;
        }
        println!("{name}: {} / {} bytes", ours.len(), theirs.len());
        differ.push(name.to_owned());
        let (a, b) = (format!("{:#?}", prelude.header), format!("{header:#?}"));
        let (a, b): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
        for (i, (x, y)) in a.iter().zip(&b).enumerate().filter(|(_, (x, y))| x != y).take(60) {
            println!("  line {i}: ours {} theirs {}", x.trim(), y.trim());
        }
        let (a, b) = (format!("{:#?}", prelude.creator), format!("{:#?}", film.header.creator));
        for (x, y) in a.lines().zip(b.lines()).filter(|(x, y)| x != y).take(30) {
            println!("  creator: ours {} theirs {}", x.trim(), y.trim());
        }
        let settings = |c: &aa_mapworld::header::DerivedCreator| match c {
            aa_mapworld::header::DerivedCreator::Random(s) => format!(
                "rmc {} weights {:?} total {} objects {:?} heroes {}",
                text(&s.rmc),
                s.terrain_weights.iter().map(|(i, b)| (text(i), f32::from_bits(*b))).collect::<Vec<_>>(),
                f32::from_bits(s.total_terrain_weight_bits),
                s.object_counts.iter().map(|(i, c)| (text(i), f32::from_bits(*c))).collect::<Vec<_>>(),
                s.num_heroes
            ),
            _ => String::new(),
        };
        println!("  ours settings {}", settings(&prelude.creator));
        println!("  theirs settings {}", settings(&film.header.creator));
        for (label, h) in [("ours", &prelude.header), ("theirs", header)] {
            println!("  {label} rules {:?}", h.rules.rules.iter().map(|r| (text(&r.key), r.enabled, r.kind, r.value)).collect::<Vec<_>>());
            println!("  {label} creator rules {:?}", h.creator.rules.rules.iter().map(|r| (text(&r.key), r.enabled, r.kind, r.value)).collect::<Vec<_>>());
            println!("  {label} misc {:?} {:?} {:?} {:?} {:?} {:?} {:?}", h.display_extents_bits, h.words_0c_10, text(&h.string_3c), h.word_40, h.entries.len(), text(&h.name_key), text(&h.description_key));
        }
        // The rows by name, what a person can read.
        for (label, h) in [("ours", &prelude.header), ("theirs", header)] {
            for k in h.parameters.kingdoms.iter().chain(&h.creator.parameters.kingdoms) {
                println!(
                    "  {label} {} b10 {} b29 {} b28 {} b31 {} b32 {} n {} f {} c {} h {} key {} 0c {:?} 2c {} t {} res {}",
                    text(&k.id), k.byte_10, k.byte_29, k.byte_28, k.byte_31, k.byte_32, text(&k.nation), text(&k.faction), text(&k.color), text(&k.handicap), text(&k.display_key), k.string_0c.as_deref().map(text), text(&k.string_2c), text(&k.string_14), k.resources.len()
                );
            }
            println!("  {label} teams {:?}", h.parameters.teams.iter().map(|t| (text(&t.id), text(&t.display_key))).collect::<Vec<_>>());
            println!("  {label} names {} / {} / {} / {}", text(&h.string_38), text(&h.creator.biome), text(&h.creator.weather), h.creator.version);
        }
    }
    assert!(compared > 0);
    assert!(differ.is_empty(), "{differ:?}");
}

/// Which order shapes of the stream the original's own films hold: every
/// (descriptor, command, flags) of a recorded film against the real films. Prints
/// the shapes no real film has; a shape of an order the corpus has no example of
/// proves nothing either way.
#[test]
#[ignore = "reads the QA films; set AA_GAME_ROOT, AA_FILM_CORPUS and AA_ROUND_TRIP_KEEP"]
fn order_shapes_against_the_original_s() {
    use aa_parity::{Decree, classify};
    fn shapes(paths: &[PathBuf]) -> BTreeMap<(u16, String, u16), usize> {
        let mut out = BTreeMap::new();
        for path in paths {
            let Ok(film) = Film::read(path, &registries()) else { continue };
            if film.failures().next().is_some() {
                continue;
            }
            for d in film.ticks.iter().flat_map(|t| &t.decrees) {
                let (name, flags) = match &d.decree {
                    Decree::TellSelectedCommand(w) => (classify::actor_command(&d.decree).map_or_else(|| "default".to_owned(), |(_, n, _)| n.to_string()), w.flags),
                    Decree::TellActorCommand { command, .. } => (classify::actor_command(&d.decree).map_or_else(|| "default".to_owned(), |(_, n, _)| n.to_string()), command.flags),
                    _ => (String::new(), 0),
                };
                *out.entry((d.descriptor, name, flags)).or_default() += 1;
            }
        }
        out
    }
    let root = game_root();
    let dir = root.join("reconstruction/qa-runs/compare-20261022/game/Profile/data/Films");
    let mut real = vec![dir.join("archive-qa-20261025/Berlin2v2_0.RFM")];
    for n in ["Berlin2v2_1", "Comrades2v2_0", "Farmhouses4FFA_0", "Kursk2v2_0"] {
        real.push(dir.join("archive-qa-20261101").join(format!("{n}.RFM")));
    }
    if let Some(corpus) = std::env::var_os("AA_FILM_CORPUS") {
        real.extend(std::fs::read_dir(corpus).unwrap().filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("RFM"))));
    }
    let keep = PathBuf::from(std::env::var_os("AA_ROUND_TRIP_KEEP").expect("AA_ROUND_TRIP_KEEP"));
    let ours: Vec<PathBuf> = std::fs::read_dir(keep).unwrap().filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("RFM"))).collect();
    // Who sends what in the real films.
    let mut who: BTreeMap<(u16, String, u16, bool), (usize, BTreeSet<u32>, usize, usize)> = BTreeMap::new();
    let mut recruits: BTreeMap<(String, u16, bool, bool), usize> = BTreeMap::new();
    for path in &real {
        let Ok(film) = Film::read(path, &registries()) else { continue };
        if film.failures().next().is_some() {
            continue;
        }
        let computer = |player: Option<u32>| {
            let Some(p) = film.players.iter().find(|x| Some(x.id) == player) else { return false };
            film.header.map_header.entries.iter().any(|e| e.flag == 1 && String::from_utf16_lossy(&e.string_20).eq_ignore_ascii_case(&p.name))
        };
        for d in film.ticks.iter().flat_map(|t| &t.decrees) {
            let w = match &d.decree {
                Decree::TellSelectedCommand(w) => w,
                Decree::TellActorCommand { command, .. } => command,
                _ => continue,
            };
            let name = classify::actor_command(&d.decree).map_or_else(|| "default".to_owned(), |(_, n, _)| n.to_string());
            if name.starts_with("recruit") {
                let object = w.object.and_then(|o| registries().object_ids.get(o as usize).cloned()).unwrap_or_default();
                let org = data().find(&object).is_some_and(|def| data().def(def).layout.is_some());
                *recruits.entry((name.clone(), w.flags, computer(d.player), org)).or_default() += 1;
            }
            let row = who.entry((d.descriptor, name, w.flags, computer(d.player))).or_default();
            row.0 += 1;
            if w.flags >> 4 & 1 != 0 && row.1.len() < 6 {
                row.1.insert(w.scalar);
            }
            row.2 += usize::from(w.boolean);
            row.3 += usize::from(w.trailing);
        }
    }
    for ((descriptor, name, flags, computer), (n, scalars, boolean, trailing)) in &who {
        println!("REAL {descriptor:3} {name:26} {flags:#05x} {} n={n:6} bool={boolean} trailing={trailing} scalars {:?}", if *computer { "ai   " } else { "human" }, scalars.iter().map(|s| f32::from_bits(*s)).collect::<Vec<_>>());
    }
    for ((name, flags, computer, org), n) in &recruits {
        println!("RECRUIT {name} {flags:#05x} computer {computer} organization {org}: {n}");
    }
    let (real, ours) = (shapes(&real), shapes(&ours));
    println!("real shapes {}, ours {}", real.len(), ours.len());
    for ((descriptor, name, flags), n) in &ours {
        let same = real.get(&(*descriptor, name.clone(), *flags)).copied().unwrap_or(0);
        let others: Vec<u16> = real.keys().filter(|k| k.0 == *descriptor && k.1 == *name).map(|k| k.2).collect();
        println!("{} {descriptor:3} {name:26} {flags:#05x} ours {n:6} real {same:7}  real flags {others:x?}", if same == 0 { "NEW" } else { "   " });
    }
}

/// The rotation 0x76FE60.
#[test]
fn rotation_names_follow_the_original() {
    assert_eq!(recording::rotate_film_name("LastFilm", 0, 5), ("LastFilm_0".to_owned(), 1));
    assert_eq!(recording::rotate_film_name("LastFilm", 4, 5), ("LastFilm_4".to_owned(), 0));
    // clamped to 0..queue-1
    assert_eq!(recording::rotate_film_name("LastFilm", -3, 5), ("LastFilm_0".to_owned(), 1));
    assert_eq!(recording::rotate_film_name("LastFilm", 9, 5), ("LastFilm_4".to_owned(), 0));
    assert_eq!(recording::numeric_version("1.0.5.0"), 1_000_500);
    assert_eq!(
        recording::film_path(Path::new("x"), "LastFilm_1"),
        Path::new("x").join(format!("LastFilm_1.{}", film_writer::FILM_EXTENSION))
    );
}

/// The film header against the original's: a world built from a real stock-map
/// film and the session the film states give the film's own MAP_HEADER back, byte
/// for byte (the session's choices come from the film; everything else is the
/// recorder's: the map's header, the world's kingdoms and generals, the version).
#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn film_header_of_a_stock_map_is_the_original_s() {
    let root = game_root();
    let dir = root.join("reconstruction/qa-runs/compare-20261022/game/Profile/data/Films/archive-qa-20261101");
    let mut compared = 0;
    for name in ["Berlin2v2_1", "Kursk2v2_0", "Comrades2v2_0", "Farmhouses4FFA_0"] {
        let path = dir.join(format!("{name}.RFM"));
        let film = Film::read(&path, &registries()).unwrap();
        let aa_mapworld::header::DerivedCreator::Load { source_resource_ids, .. } = &film.header.creator else {
            panic!("{name}: not a stock-map film");
        };
        let source = String::from_utf16_lossy(source_resource_ids);
        let id = source.trim_start_matches("Maps/").trim_end_matches(".RMP").to_owned();
        let map = Arc::new(GameMap::load_with(&root, &id, &data()).unwrap());
        let plan = aa_parity::replay::plan(&film, &data(), &map).unwrap();
        let world = aa_parity::replay::build_with(&plan, generals(), data(), map.clone()).unwrap();
        let header = &film.header.map_header;
        let text = |u: &[u16]| String::from_utf16_lossy(u);
        let rules = header.creator.rules.rules.iter().map(|r| (text(&r.key), r.kind, r.value)).collect();
        let mut sai_profiles = BTreeMap::new();
        let mut random_generals = BTreeSet::new();
        let mut random_nations = BTreeSet::new();
        let mut player_names = BTreeMap::new();
        let mut local = None;
        for (k, p) in plan.players.iter().enumerate() {
            let k = k as u32;
            let row = header.parameters.kingdoms.iter().find(|r| text(&r.id).eq_ignore_ascii_case(&p.slot_id)).unwrap();
            if !row.string_2c.is_empty() {
                sai_profiles.insert(k, text(&row.string_2c));
            }
            if p.random_general {
                random_generals.insert(k);
            }
            if p.random_nation {
                random_nations.insert(k);
            }
            let entry = header.entries.iter().find(|e| text(&e.string_20).eq_ignore_ascii_case(&p.slot_id)).unwrap();
            player_names.insert(k, text(&entry.string_18));
            if entry.flag_v5 == Some(1) {
                local = Some(k);
            }
        }
        let session = FilmSession {
            source: FilmSource::Map { id },
            seed: u64::from(film.header.seed),
            local,
            rules,
            random_generals,
            random_nations,
            sai_profiles,
            player_names,
            version: GameVersion::load(&root).unwrap(),
            filetime: header.words_18_1c,
            extras: Default::default(),
        };
        let prelude = recording::prelude(&world, &root, &session).unwrap();
        let ours = film_writer::map_header_bytes(&prelude.header, &prelude.creator);
        let theirs = film_writer::map_header_bytes(header, &film.header.creator);
        if ours != theirs {
            let at = ours.iter().zip(&theirs).position(|(a, b)| a != b).unwrap_or(ours.len().min(theirs.len()));
            println!("{name}: {} / {} bytes, first difference at {at}", ours.len(), theirs.len());
            let show = |h: &aa_mapworld::header::MapHeader| {
                for k in h.parameters.kingdoms.iter().chain(&h.creator.parameters.kingdoms) {
                    println!("  res {} {:?}", text(&k.id), k.resources.iter().map(|(i, v)| (text(i), *v)).collect::<Vec<_>>());
                }
                for k in &h.parameters.kingdoms {
                    println!("  hdr {} b10 {} b29 {} b28 {} b31 {} b32 {} n {} f {} c {} h {} 0c {:?} 2c {} t {}", text(&k.id), k.byte_10, k.byte_29, k.byte_28, k.byte_31, k.byte_32, text(&k.nation), text(&k.faction), text(&k.color), text(&k.handicap), k.string_0c.as_deref().map(text), text(&k.string_2c), text(&k.string_14));
                }
                for k in &h.creator.parameters.kingdoms {
                    println!("  cre {} b10 {} b29 {} b28 {} b31 {} b32 {} n {} f {} c {} h {} 0c {:?} 2c {} t {}", text(&k.id), k.byte_10, k.byte_29, k.byte_28, k.byte_31, k.byte_32, text(&k.nation), text(&k.faction), text(&k.color), text(&k.handicap), k.string_0c.as_deref().map(text), text(&k.string_2c), text(&k.string_14));
                }
                println!("  teams {:?} / {:?}", h.parameters.teams.iter().map(|t| text(&t.id)).collect::<Vec<_>>(), h.creator.parameters.teams.iter().map(|t| text(&t.id)).collect::<Vec<_>>());
            };
            println!(" ours:");
            show(&prelude.header);
            println!(" theirs:");
            show(header);
            // What differs, field by field.
            let (a, b) = (format!("{:#?}", prelude.header), format!("{header:#?}"));
            let (a, b): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
            println!(" {} / {} lines of the header:", a.len(), b.len());
            for (i, (x, y)) in a.iter().zip(&b).enumerate().filter(|(_, (x, y))| x != y).take(40) {
                println!("  line {i}:
   ours   {x}
   theirs {y}");
            }
        }
        assert_eq!(ours, theirs, "MAP_HEADER of {name}");
        // The session's id counter runs on through staging (the ids of the film
        // of a second launch are higher): the rows, not their numbers.
        let rows = |p: &[aa_parity::SessionPlayer]| p.iter().map(|x| (x.name.clone(), x.kingdom_index)).collect::<Vec<_>>();
        assert_eq!(rows(&prelude.players), rows(&film.players), "SessionPlayers of {name}");
        compared += 1;
        println!("{name}: the film header and session players are the original's");
    }
    assert!(compared > 0);
}
