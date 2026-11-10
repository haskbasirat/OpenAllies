//! A film of the original with an air base order, replayed in aa-play beside
//! the original's own trace of it (the native sampler's A lines): every
//! aircraft the original samples, its pose per tick, next to the thing aa-play
//! has under the same GTID. Prints only; the air lane's evidence for the
//! flight of a base aircraft.
//!
//! ```text
//! AA_GAME_ROOT=<game root> AA_AIR_FILM=<film> AA_AIR_SOURCE=<folder of its saved game>
//! AA_AIR_TRACE=<native trace> AA_AIR_FILM_KEEP=<out folder>
//! cargo test -p aa-parity --features play --release --test air_film -- --ignored --nocapture
//! ```
//!
//! A film recorded by aa-play cannot carry an early air order on a stock
//! map: an airfield truck (`rus_truck_airfield`) needs three structures and
//! the air_support research before its recruit is taken.
#![cfg(feature = "play")]
use aa_parity::native_trace::NativeTrace;
use aa_parity::playback::{FilmData, SyncMode, open_film};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::PathBuf;

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}
fn keep() -> PathBuf {
    let dir = std::env::var_os("AA_AIR_FILM_KEEP").map_or_else(|| std::env::temp_dir().join("aa-air-film"), PathBuf::from);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `AA_AIR_FILM` (the film), `AA_AIR_SOURCE` (the folder holding its saved
/// game, if any), `AA_AIR_TRACE` (the trace); written to
/// `<AA_AIR_FILM_KEEP>/compare.txt`.
#[test]
#[ignore = "reads a film, its trace and the original data"]
fn air_film_against_the_original() {
    let film = PathBuf::from(std::env::var_os("AA_AIR_FILM").expect("AA_AIR_FILM"));
    let trace = NativeTrace::read(&PathBuf::from(std::env::var_os("AA_AIR_TRACE").expect("AA_AIR_TRACE"))).unwrap();
    let mut data = FilmData::new(game_root());
    data.source_folder = std::env::var_os("AA_AIR_SOURCE").map(PathBuf::from);
    let (mut world, mut player) = open_film(&film, &data, SyncMode::Report).unwrap();
    for note in player.notes() {
        eprintln!("note: {note}");
    }
    // AA_AIR_DENIZEN_STANDIN=1: a saved world's restore gives an air base no
    // Denizen books yet (`saved_world`), so the command mask refuses every air
    // order. This stand-in gives each restored base the books of a new one
    // (0x568750, every slot at full health) and its queue 1 entry, only so that
    // the flight can be compared; it is not the saved state.
    if std::env::var_os("AA_AIR_DENIZEN_STANDIN").is_some() {
        let bases: Vec<u32> = world
            .actors
            .iter()
            .filter(|a| a.alive && !a.removed && world.data.def(a.def).denizen.is_some() && world.books.actor(a.id).is_none_or(|b| b.denizen.is_none()))
            .map(|a| a.id)
            .collect();
        for &a in &bases {
            if aa_play::denizen::created(&mut world, a) {
                world.schedule_periodic_at(aa_play::trace::sites::COMPONENT_INIT, aa_play::scheduler::EVERY_SECOND, aa_play::scheduler::Owner::Component(a, 10), 0);
            }
        }
        eprintln!("stand-in denizen books for {} bases: {:?}", bases.len(), bases.iter().map(|&a| (a, world.data.def(world.actors[a as usize].def).id.clone())).collect::<Vec<_>>());
    }
    let aircraft: BTreeSet<u32> = trace
        .names
        .iter()
        .filter(|(_, n)| {
            let n = n.to_ascii_lowercase();
            n.ends_with("_fighter") || n.ends_with("_bomber")
        })
        .map(|(g, _)| *g)
        .collect();
    let path = keep().join("compare.txt");
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    // AA_AIR_TO: replay on past the trace's last sample (ours only there).
    let last = (*trace.ticks.keys().last().expect("a sampled tick")).max(std::env::var("AA_AIR_TO").ok().and_then(|v| v.parse().ok()).unwrap_or(0));
    let start = world.clock.tick;
    let first_actor = world.actors.len();
    eprintln!("restored at {start}; trace to {last}; {} aircraft named in the trace", aircraft.len());
    let mut refused = BTreeMap::new();
    let mut moved_native: BTreeMap<u32, u32> = BTreeMap::new();
    let mut moved_ours: BTreeMap<u32, u32> = BTreeMap::new();
    let mut first_native: BTreeMap<u32, [u32; 3]> = BTreeMap::new();
    let mut first_ours: BTreeMap<u32, [u32; 3]> = BTreeMap::new();
    while (world.clock.tick as u32) < last {
        player.step(&mut world);
        let tick = world.clock.tick as u32;
        if player.stats().refused != refused {
            eprintln!("tick {tick}: refused {:?}", player.stats().refused);
            refused = player.stats().refused.clone();
        }
        let native = trace.ticks.get(&tick);
        // aa-play's own new aircraft, under whatever GTID it gave them.
        for a in world.actors.iter().skip(first_actor).filter(|a| world.data.def(a.def).aircraft.is_some()) {
            let mover = world.books.actor(a.id).and_then(|b| b.flight_mover.as_ref());
            let path = world.books.actor(a.id).and_then(|b| b.aircraft.as_ref()).map(|x| (x.flight.path.segments.len(), x.flight.path.t));
            writeln!(
                out,
                "{tick} ours-new {:08x} {} p={:?} d={:?} v={:?} removed={} state={:x?} step={:08x} path={path:?} mover={}",
                world.ids.actor(a.id).map_or(0, |g| g.raw()),
                world.data.def(a.def).id,
                a.pos.to_array(),
                mover.map(|m| m.facing),
                mover.map(|m| m.speed),
                a.removed,
                aa_play::combat::air::current_state(&world, a.id),
                world.clock.step.0,
                world.is_scheduled(aa_play::scheduler::Owner::Mover(a.id)),
            )
            .unwrap();
        }
        for &g in &aircraft {
            let n = native.and_then(|t| t.actors.get(&g));
            let ours = match world.ids.thing(g) {
                Some(aa_play::ids::Thing::Actor(a)) => Some(&world.actors[a as usize]),
                _ => None,
            };
            if let Some(n) = n {
                let f = *first_native.entry(g).or_insert(n.pos);
                if f != n.pos {
                    moved_native.entry(g).or_insert(tick);
                }
            }
            if let Some(a) = ours {
                let p = [a.pos.x.to_bits(), a.pos.y.to_bits(), a.pos.z.to_bits()];
                let f = *first_ours.entry(g).or_insert(p);
                if f != p {
                    moved_ours.entry(g).or_insert(tick);
                }
            }
            if !moved_native.contains_key(&g) && !moved_ours.contains_key(&g) {
                continue;
            }
            let np = n.map(|n| n.pos.map(f32::from_bits));
            let nd = n.map(|n| n.dir.map(f32::from_bits));
            let ns = n.and_then(|n| n.mover.as_ref()).map(|m| f32::from_bits(m.speed));
            let op = ours.map(|a| a.pos.to_array());
            let mover = ours.and_then(|a| world.books.actor(a.id)).and_then(|b| b.flight_mover.as_ref());
            let exact = match (n, ours) {
                (Some(n), Some(a)) => n.pos == [a.pos.x.to_bits(), a.pos.y.to_bits(), a.pos.z.to_bits()],
                _ => false,
            };
            writeln!(
                out,
                "{tick} {g:08x} {} native p={np:?} d={nd:?} v={ns:?} flags={:?} | ours p={op:?} d={:?} v={:?} removed={:?} state={:x?} | {}",
                trace.names.get(&g).map_or("?", String::as_str),
                n.map(|n| format!("{:08x}", n.flags)),
                mover.map(|m| m.facing),
                mover.map(|m| m.speed),
                ours.map(|a| a.removed),
                ours.and_then(|a| aa_play::combat::air::current_state(&world, a.id)),
                if exact { "exact" } else { "differs" },
            )
            .unwrap();
        }
    }
    out.flush().unwrap();
    eprintln!("first move, native {moved_native:x?}");
    eprintln!("first move, ours {moved_ours:x?}");
    eprintln!(
        "checkpoints {}/{}, first mismatch {:?}; compare in {}",
        player.stats().checkpoints_matched,
        player.stats().checkpoints_checked,
        player.stats().first_mismatch,
        path.display()
    );
}
