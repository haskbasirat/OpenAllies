//! A film's sources beyond the game's own data: a saved game and a profile's data
//! folder (`aa_parity::playback`, FILM-CORPUS.md "Sources").
//!
//! Everything here reads the frozen corpus and the original data, so it is ignored
//! by default. `a_saved_world_against_the_original` also needs a native trace of the
//! film's first ticks from the QA copy: `AA_FILM_SOURCES_TRACE=<trace>`,
//! `AA_FILM_SOURCES_FILM=<sha prefix>`, `AA_FILM_SOURCES_FOLDER=<data folder with the
//! save>`. Traces stay outside the tree.
use aa_parity::native_trace::NativeTrace;
use aa_parity::playback::{FilmData, FilmEvent, SyncMode, open_film};
use aa_parity::state_compare::StateComparison;
use std::path::{Path, PathBuf};

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"))
}

fn corpus_film(sha: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/film-corpus/films");
    std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(sha)))
        .unwrap_or_else(|| panic!("no corpus film {sha}"))
}

/// The world a saved game's film starts from, against the original playing the same
/// film under the sampler. Holds the world stream: after the load it is the state the
/// original's first getter of the battle reads. Prints the things the original has at
/// its first sample that the restore has no thing for (none, on the two films tried),
/// then the first difference of each lane and its causes, for the census.
#[test]
#[ignore = "needs a native trace of a saved game's film; see the module comment"]
fn a_saved_world_against_the_original() {
    let root = game_root();
    let sha = std::env::var("AA_FILM_SOURCES_FILM").expect("AA_FILM_SOURCES_FILM");
    let folder = PathBuf::from(std::env::var_os("AA_FILM_SOURCES_FOLDER").expect("AA_FILM_SOURCES_FOLDER"));
    let trace = NativeTrace::read(Path::new(&std::env::var_os("AA_FILM_SOURCES_TRACE").expect("AA_FILM_SOURCES_TRACE"))).unwrap();
    let mut data = FilmData::new(&root);
    data.source_folder = Some(folder);
    let (mut world, mut player) = open_film(&corpus_film(&sha), &data, SyncMode::Report).unwrap();
    for note in player.notes() {
        eprintln!("note: {note}");
    }
    let start = world.clock.tick;
    // The record pass's draws: thousands of them end on the state the original's first
    // getter of the battle reads.
    let native_first = trace.getters.iter().find(|g| g.tick > start).expect("a getter of the battle");
    assert_eq!(world.native_rng.state, native_first.state, "the stream after the load");
    let first = *trace.ticks.keys().next().expect("a sampled tick");
    eprintln!(
        "restored at tick {start}, {} actors, {} organizations; native first sample {first}, rng {:08x}, {} actors",
        world.actors.iter().filter(|a| a.alive).count(),
        world.orgs.len(),
        trace.ticks[&first].rng,
        trace.ticks[&first].actors.len()
    );
    let last = *trace.ticks.keys().last().unwrap();
    let mut comparison = StateComparison::new();
    while world.clock.tick < last {
        let events = player.step(&mut world);
        let tick = world.clock.tick;
        if tick == first {
            eprintln!("ours at {tick}: rng {:08x}", world.native_rng.state);
            let native = &trace.ticks[&first];
            let missing: Vec<u32> = native.actors.keys().copied().filter(|g| world.ids.thing(*g).is_none()).collect();
            let mut by_name: std::collections::BTreeMap<String, u32> = Default::default();
            for g in &missing {
                *by_name.entry(trace.names.get(g).cloned().unwrap_or_default()).or_default() += 1;
            }
            eprintln!("native actors we have no thing for: {} {by_name:?}", missing.len());
        }
        comparison.observe(tick, &world, &trace, None);
        comparison.remember(tick, &world, &trace, None);
        for e in events {
            if matches!(e, FilmEvent::EndOfFilm { .. }) {
                eprintln!("the film ended at {tick}");
            }
        }
    }
    eprintln!("{} sampled ticks compared, stats {:?}", comparison.ticks_observed, player.stats().first_mismatch);
    for (lane, l) in &comparison.lanes {
        eprintln!("--- {lane}: first at {:?}; {} sampled ticks differ; {} causes", l.first.as_ref().map(|d| d.tick), l.ticks_differing, l.causes.len());
        let mut causes: Vec<_> = l.causes.values().collect();
        causes.sort_by_key(|c| (c.first_tick, c.signature.clone()));
        for c in causes.iter().take(30) {
            eprintln!(
                "  {:>6}..{:<6} ticks {:>4} fields {:>6} things {:>4} first-on {:>4} {} | e.g. {:#x} {:?} {}: native {} / ours {}",
                c.first_tick,
                c.last_tick,
                c.ticks,
                c.fields,
                c.things.len(),
                c.first_on.len(),
                c.signature,
                c.sample.gtid.unwrap_or(0),
                c.sample.name,
                c.sample.field,
                c.sample.native,
                c.sample.ours
            );
        }
    }
}

/// One film on one data folder, a few ticks: what the sweep does, for one film and
/// faster. `AA_FILM_SOURCES_FILM=<sha prefix>`, `AA_FILM_SOURCES_OVERLAY=<a profile's
/// data folder>` and / or `AA_FILM_SOURCES_FOLDER=<a folder with its source>`,
/// `AA_FILM_SOURCES_TICKS`.
#[test]
#[ignore = "reads a corpus film and the original data; set AA_GAME_ROOT and AA_FILM_SOURCES_FILM"]
fn one_film_opens_and_plays() {
    let root = game_root();
    let sha = std::env::var("AA_FILM_SOURCES_FILM").expect("AA_FILM_SOURCES_FILM");
    let ticks: u64 = std::env::var("AA_FILM_SOURCES_TICKS").ok().and_then(|t| t.parse().ok()).unwrap_or(300);
    let mut data = FilmData::new(&root);
    data.overlay = std::env::var_os("AA_FILM_SOURCES_OVERLAY").map(PathBuf::from);
    data.source_folder = std::env::var_os("AA_FILM_SOURCES_FOLDER").map(PathBuf::from);
    data.draws_scene = true;
    let start = std::time::Instant::now();
    let (mut world, mut player) = match open_film(&corpus_film(&sha), &data, SyncMode::Report) {
        Ok(opened) => opened,
        Err(failure) => panic!("{failure:?}"),
    };
    eprintln!("opened in {:.1} s", start.elapsed().as_secs_f32());
    for note in player.notes() {
        eprintln!("note: {note}");
    }
    for (k, kingdom) in world.kingdoms.iter().enumerate() {
        eprintln!("kingdom {k}: {:?} {:?} team {} {:?}", kingdom.name, kingdom.nation, kingdom.team, kingdom.controller);
    }
    let start = std::time::Instant::now();
    for _ in 0..ticks {
        let events = player.step(&mut world);
        if events.iter().any(|e| matches!(e, FilmEvent::EndOfFilm { .. })) {
            break;
        }
    }
    let s = player.stats();
    eprintln!(
        "{} ticks in {:.1} s, {} checkpoints, {} matched, first mismatch {:?}, {} commands, {} actors, {} organizations",
        s.ticks,
        start.elapsed().as_secs_f32(),
        s.checkpoints_checked,
        s.checkpoints_matched,
        s.first_mismatch,
        s.commands,
        world.actors.iter().filter(|a| a.alive && !a.removed).count(),
        world.orgs.len()
    );
    eprintln!("unresolved {:?}\nrefused {:?}\nno equivalent {:?}", s.unresolved, s.refused, s.no_equivalent);
}
