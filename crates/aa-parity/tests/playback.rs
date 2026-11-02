//! The library the game client plays films with (`aa_parity::playback`), against
//! the gate, against the native stop, and over every real film.
//!
//! ```text
//! AA_GAME_ROOT=<game root> cargo test -p aa-parity --release --test playback -- --ignored --nocapture
//! ```
//! `AA_FILM_SWEEP_ONLY=<sha prefix>,...` limits the sweep. `AA_FILM_SWEEP_OUT=<file>`
//! writes its aggregates (no paths, no player names) as Markdown.
#![cfg(feature = "play")]
use aa_parity::Decree;
use aa_parity::film::Film;
use aa_parity::playback::{FilmData, FilmEvent, FilmFailure, FilmPlayer, SyncMode, open_film};
use aa_parity::registries::Registries;
use aa_parity::replay;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const QA_FILMS: &str = "reconstruction/qa-runs/compare-20261022/game/Profile/data/Films";
const BERLIN: &str = "archive-qa-20261025/Berlin2v2_0.RFM";
const BERLIN_MAP: &str = "(2v2)Berlin";

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}
fn berlin() -> PathBuf {
    game_root().join(QA_FILMS).join(BERLIN)
}

fn assert_send_sync<T: Send + Sync>() {}

/// The client keeps a `FilmPlayer` in a resource and opens it on a thread.
#[test]
fn a_player_can_cross_threads() {
    assert_send_sync::<FilmPlayer>();
    assert_send_sync::<FilmEvent>();
    assert_send_sync::<FilmFailure>();
}

#[test]
fn a_missing_film_is_the_native_file_missing() {
    let data = FilmData::new("does-not-exist");
    let Err(failure) = open_film(Path::new("does-not-exist/nothing.RFM"), &data, SyncMode::Strict)
    else {
        panic!("a film that is not there opens");
    };
    assert_eq!(failure.key(), "load_error_missing_map|s");
    assert_eq!(failure, FilmFailure::MissingFile { name: "nothing".into() });
}

/// The library and the gate cannot drift: the gate's run and the library's give
/// the same first difference, the same matched count and the same world.
#[test]
#[ignore = "reads the QA Berlin film and the original data; set AA_GAME_ROOT"]
fn the_library_and_the_gate_agree_on_berlin() {
    let root = game_root();
    let registries = Registries::load_v14(&root, true).unwrap();
    let film = Film::read(&berlin(), &registries).unwrap();
    replay::set_formation_ids(registries.formation_ids.clone());
    replay::set_property_ids(registries.property_ids.clone());
    aa_parity::mapping::set_ability_ids(registries.ability_ids.clone());
    aa_parity::mapping::set_layout_ids(registries.layout_ids.clone());
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let map = Arc::new(aa_play::GameMap::load_with(&root, BERLIN_MAP, &data).unwrap());
    let plan = replay::plan(&film, &data, &map).unwrap();
    let mut gate_world = replay::build(&plan, &root, data, map).unwrap();
    let gate = replay::run(&film, &mut gate_world, &registries.object_ids, None, false);

    let (mut world, mut player) =
        open_film(&berlin(), &FilmData::for_film(&root, &berlin()), SyncMode::Report).unwrap();
    assert_eq!(player.film_ticks() as usize, film.world_ticks());
    let mut events = Vec::new();
    for _ in 0..player.film_ticks() {
        events.extend(player.step(&mut world));
    }
    let stats = player.stats();
    eprintln!("gate: {} matched of {}, first difference {:?}", gate.matched, gate.checked, gate.first_difference.as_ref().map(|d| d.check));
    eprintln!("library: {stats:?}");
    assert_eq!(stats.checkpoints_checked as usize, gate.checked);
    assert_eq!(stats.checkpoints_matched as usize, gate.matched);
    let first = gate.first_difference.as_ref().map(|d| (d.check.tick, d.check.film.0 as u32));
    assert_eq!(stats.first_mismatch, first);
    assert_eq!(stats.ticks, gate.steps as u64);
    assert_eq!(world.native_rng.state, gate_world.native_rng.state, "the same world stream");
    assert_eq!(world.trace.checkpoint(), gate_world.trace.checkpoint(), "the same S");
    assert_eq!(world.actors.len(), gate_world.actors.len());
    for (a, b) in world.actors.iter().zip(&gate_world.actors) {
        assert_eq!(
            (a.def, a.pos.x.to_bits(), a.pos.y.to_bits(), a.health.to_bits(), a.alive),
            (b.def, b.pos.x.to_bits(), b.pos.y.to_bits(), b.health.to_bits(), b.alive),
        );
    }
    // The natural end comes one tick after the last record.
    assert!(player.film_running());
    let end = player.step(&mut world);
    assert_eq!(end, vec![FilmEvent::EndOfFilm { out_of_sync: stats_mismatched(&events) }]);
    assert!(!player.film_running());
    // The world keeps stepping with the film gone.
    let before = world.clock.tick;
    for _ in 0..32 {
        assert!(player.step(&mut world).is_empty());
    }
    assert_eq!(world.clock.tick, before + 32);
}

fn stats_mismatched(events: &[FilmEvent]) -> bool {
    events.iter().any(|e| matches!(e, FilmEvent::FirstMismatch { .. }))
}

/// Opens Berlin in Strict with its first checkpoint's hash altered, which the
/// world matches untouched. Returns the checkpoint's record.
fn berlin_with_altered_hash() -> (aa_play::World, FilmPlayer, u32, i32) {
    let root = game_root();
    let (world, mut player) =
        open_film(&berlin(), &FilmData::for_film(&root, &berlin()), SyncMode::Strict).unwrap();
    let c = player.film().checkpoints[0];
    let tick = player.film_mut().ticks.get_mut(c.tick as usize).unwrap();
    let decree = tick
        .decrees
        .iter_mut()
        .find(|d| matches!(d.decree, Decree::SimSyncValue { .. }))
        .unwrap();
    let Decree::SimSyncValue { hash, .. } = &mut decree.decree else {
        unreachable!()
    };
    *hash ^= 0x55;
    (world, player, c.tick, c.count)
}

/// An altered hash at a checkpoint the world matches: the native stop, once, at
/// that record, and the world keeps stepping.
#[test]
#[ignore = "reads the QA Berlin film and the original data; set AA_GAME_ROOT"]
fn an_altered_hash_is_the_native_stop() {
    let (mut world, mut player, record, index) = berlin_with_altered_hash();
    let mut seen = Vec::new();
    for tick in 0..=record {
        assert!(player.film_running(), "running until the record");
        let events = player.step(&mut world);
        if tick < record {
            assert!(
                !events.iter().any(|e| matches!(e, FilmEvent::EndOfFilm { .. } | FilmEvent::FirstMismatch { .. })),
                "nothing before it: {events:?}"
            );
        } else {
            seen = events;
        }
    }
    assert!(seen.contains(&FilmEvent::FirstMismatch { index: index as u32 }), "{seen:?}");
    assert!(seen.contains(&FilmEvent::EndOfFilm { out_of_sync: true }), "{seen:?}");
    assert!(!player.film_running(), "Admin+0x26C = 0");
    assert_eq!(player.stats().first_mismatch, Some((record, index as u32)));
    assert_eq!(player.next_record(), record + 1, "the record itself still ran");
    let (ticks, s) = (world.clock.tick, world.trace.checkpoint());
    for _ in 0..64 {
        assert!(player.step(&mut world).is_empty(), "the film is gone");
    }
    assert_eq!(world.clock.tick, ticks + 64, "the world keeps stepping");
    assert!(world.trace.checkpoint().0 > s.0, "S grows behind the box");
    assert_eq!(player.next_record(), record + 1, "no more records");
}

/// 0x75D710: a SetPlayer of a player the session lacks stops the film and ends
/// the record's decree loop (Admin+0x2E5).
#[test]
#[ignore = "reads the QA Berlin film and the original data; set AA_GAME_ROOT"]
fn an_unknown_ordering_player_is_the_same_stop() {
    let root = game_root();
    let (mut world, mut player) =
        open_film(&berlin(), &FilmData::for_film(&root, &berlin()), SyncMode::Strict).unwrap();
    let (record, at) = player
        .film()
        .ticks
        .iter()
        .find_map(|t| {
            let at = t.decrees.iter().position(|d| matches!(d.decree, Decree::SetPlayer { .. }))?;
            (at + 1 < t.decrees.len()).then_some((t.tick, at))
        })
        .unwrap();
    let decrees = &mut player.film_mut().ticks[record as usize].decrees;
    decrees[at].decree = Decree::SetPlayer { player: 777 };
    for _ in 0..record {
        assert!(player.step(&mut world).is_empty());
    }
    let events = player.step(&mut world);
    assert!(events.contains(&FilmEvent::NoOrderingPlayer { player: 777 }), "{events:?}");
    assert!(events.contains(&FilmEvent::EndOfFilm { out_of_sync: true }), "{events:?}");
    assert!(!player.film_running());
}

/// Strict: the first differing checkpoint is the native stop (0x752F60, 0x751EB0):
/// END_OF_FILM with out-of-sync set, the record's decrees still run, and the world
/// keeps stepping. Berlin's own first difference is a real one, so the film needs no
/// altering until it passes; the altered case is `altered_hash_stops`.
#[test]
#[ignore = "reads the QA Berlin film and the original data; set AA_GAME_ROOT"]
fn strict_stops_at_the_first_difference_and_the_world_runs_on() {
    let root = game_root();
    let (mut world, mut player) =
        open_film(&berlin(), &FilmData::for_film(&root, &berlin()), SyncMode::Strict).unwrap();
    let mut end = None;
    let mut first = None;
    for tick in 0..player.film_ticks() + 1 {
        for e in player.step(&mut world) {
            match e {
                FilmEvent::EndOfFilm { out_of_sync } => end = Some((tick, out_of_sync)),
                FilmEvent::FirstMismatch { index } => first = Some((tick, index)),
                _ => {}
            }
        }
        if end.is_some() {
            break;
        }
    }
    let (tick, out_of_sync) = end.expect("the film ends");
    assert!(out_of_sync, "a difference, or the film matches whole and this test needs its altered case");
    assert!(!player.film_running());
    let (at, index) = first.expect("the mismatch was reported");
    assert_eq!(at, tick, "reported where it stops");
    assert_eq!(player.stats().first_mismatch.map(|m| m.1), Some(index));
    let checked = player.stats().checkpoints_checked;
    let before = world.clock.tick;
    let s_before = world.trace.checkpoint();
    for _ in 0..64 {
        assert!(player.step(&mut world).is_empty());
    }
    assert_eq!(world.clock.tick, before + 64, "the world runs on");
    assert!(world.trace.checkpoint().0 > s_before.0, "S goes on growing behind the box");
    assert_eq!(player.stats().checkpoints_checked, checked, "no film, no more checks");
}

/// Report: the same mismatch is told once and the film plays to its natural end,
/// whose box is the out-of-sync one.
#[test]
#[ignore = "reads the QA Berlin film and the original data; set AA_GAME_ROOT"]
fn report_plays_to_the_end_and_tells_the_mismatch_once() {
    let root = game_root();
    let (mut world, mut player) =
        open_film(&berlin(), &FilmData::for_film(&root, &berlin()), SyncMode::Report).unwrap();
    let mut mismatches = 0;
    let mut ends = Vec::new();
    for _ in 0..player.film_ticks() + 8 {
        for e in player.step(&mut world) {
            match e {
                FilmEvent::FirstMismatch { .. } => mismatches += 1,
                FilmEvent::EndOfFilm { out_of_sync } => ends.push(out_of_sync),
                _ => {}
            }
        }
    }
    assert!(mismatches <= 1);
    assert_eq!(ends, vec![mismatches == 1], "one end, out of sync if anything differed");
    assert_eq!(player.next_record(), player.film_ticks());
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/film-corpus")
}

/// What a film of the corpus plays on: the census's data set, from the profile
/// folder the film was found in.
fn film_data(root: &Path, row: &Value, inventory: &BTreeMap<String, Option<PathBuf>>) -> FilmData {
    let mut data = FilmData::new(root);
    let set = row["data_set"].as_str().unwrap_or("");
    if let Some(Some(folder)) = inventory.get(set) {
        data.overlay = Some(folder.clone());
    } else if let Some(names) = row["names"].as_array() {
        // A film on the stock data found in a profile: its saves are there.
        for name in names {
            let label = name[0].as_str().unwrap_or("");
            if let Some(Some(folder)) = inventory.get(label) {
                data.source_folder = Some(folder.clone());
                break;
            }
        }
    }
    data
}

#[derive(Default, Debug)]
struct SweepRow {
    label: String,
    result: String,
    ticks: u64,
    world_ticks: u64,
    checked: u64,
    matched: u64,
    first_mismatch: Option<(u32, u32)>,
    commands: u64,
    unresolved: u64,
    no_equivalent: u64,
    refused: u64,
    no_ordering_player: u64,
    /// The class of a world's refusal, without any name of the film's.
    why: &'static str,
    seconds: f64,
    load_seconds: f64,
    actors: usize,
    alive: usize,
    panicked: Option<String>,
}

/// Every film of the corpus opens or refuses with a native failure, never panics,
/// and an opened film plays to its natural end in Report mode.
#[test]
#[ignore = "reads every frozen film and the original data; set AA_GAME_ROOT"]
fn sweep_every_film_of_the_corpus() {
    let root = game_root();
    let dir = corpus_dir();
    let corpus: Value = serde_json::from_slice(&std::fs::read(dir.join("corpus.json")).unwrap()).unwrap();
    let inventory: Value = serde_json::from_slice(&std::fs::read(dir.join("inventory.json")).unwrap()).unwrap();
    // Profile label -> its data folder, from the census's inventory.
    let mut folders: BTreeMap<String, Option<PathBuf>> = BTreeMap::new();
    for row in inventory.as_array().unwrap() {
        for p in row["paths"].as_array().unwrap() {
            let (Some(label), Some(path)) = (p["folder"].as_str(), p["path"].as_str()) else {
                continue;
            };
            let path = path.replace('\\', "/");
            let data = path
                .to_ascii_lowercase()
                .find("/data/films")
                .map(|at| PathBuf::from(&path[..at + "/data".len()]));
            folders.entry(label.to_owned()).or_insert(data);
        }
    }
    let only = std::env::var("AA_FILM_SWEEP_ONLY").ok();
    let only: Option<Vec<String>> = only.map(|o| o.split(',').map(str::to_owned).collect());
    let mut rows = Vec::new();
    for (index, film) in corpus["films"].as_array().unwrap().iter().enumerate() {
        let sha = film["sha256"].as_str().unwrap();
        if only.as_ref().is_some_and(|o| !o.iter().any(|p| sha.starts_with(p.as_str()))) {
            continue;
        }
        // The census's numbering (FILM-CORPUS.md).
        let label = format!("F{:02}", index + 1);
        eprintln!("{label} is {}", &sha[..8]);
        let path = dir.join("films").join(format!("{sha}.RFM"));
        let data = film_data(&root, film, &folders);
        let mut row = SweepRow { label: label.clone(), ..SweepRow::default() };
        eprintln!("{label}: opening");
        let start = std::time::Instant::now();
        let opened = std::panic::catch_unwind(|| open_film(&path, &data, SyncMode::Report));
        row.load_seconds = start.elapsed().as_secs_f64();
        match opened {
            Err(panic) => row.panicked = Some(panic_text(panic)),
            Ok(Err(failure)) => {
                row.result = failure.key().to_owned();
                if let FilmFailure::CreateWorld { detail } = &failure {
                    eprintln!("{label}: {detail}");
                    row.why = refusal_class(detail);
                }
            }
            Ok(Ok((mut world, mut player))) => {
                let total = player.film_ticks() as u64;
                row.world_ticks = total;
                let start = std::time::Instant::now();
                let mut ended = false;
                let cap = std::env::var("AA_FILM_SWEEP_CAP").ok().and_then(|v| v.parse::<u64>().ok());
                let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    for _ in 0..total + 1 {
                        for e in player.step(&mut world) {
                            if matches!(e, FilmEvent::EndOfFilm { .. }) {
                                ended = true;
                            }
                        }
                        if cap.is_some_and(|c| player.stats().ticks >= c) {
                            break;
                        }
                    }
                }));
                row.seconds = start.elapsed().as_secs_f64();
                let s = player.stats();
                row.ticks = s.ticks;
                row.checked = s.checkpoints_checked;
                row.matched = s.checkpoints_matched;
                row.first_mismatch = s.first_mismatch;
                row.commands = s.commands;
                row.unresolved = s.unresolved.values().sum();
                row.no_equivalent = s.no_equivalent.values().sum();
                row.refused = s.refused.values().sum();
                row.no_ordering_player = s.no_ordering_player.values().sum();
                eprintln!("{label}: no equivalent {:?}", s.no_equivalent);
                eprintln!("{label}: unresolved {:?}", s.unresolved);
                eprintln!("{label}: refused {:?}", s.refused);
                eprintln!("{label}: dropped {:?}", s.dropped);
                row.actors = world.actors.len();
                row.alive = world.actors.iter().filter(|a| a.alive && !a.removed).count();
                row.result = match (&played, ended, cap) {
                    (Err(_), ..) => "panicked in play".to_owned(),
                    (Ok(()), true, _) => "played to its natural end".to_owned(),
                    (Ok(()), false, Some(_)) => "stopped at the sweep's cap".to_owned(),
                    (Ok(()), false, None) => "did not end".to_owned(),
                };
                if let Err(panic) = played {
                    row.panicked = Some(panic_text(panic));
                }
            }
        }
        eprintln!("{label}: {row:?}");
        // The failures the census knows of must come out as the native ones.
        if film["v14_check"].as_str().is_some_and(|c| c.starts_with("WrongVersion")) {
            assert_eq!(row.result, "failed_film_wrong_version|s", "{label}");
        }
        if film["status"].as_str().is_some_and(|c| c.starts_with("unreadable")) {
            assert_eq!(row.result, "load_error_missing_map|s", "{label}");
        }
        rows.push(row);
    }
    print_sweep(&rows);
    let panics: Vec<_> = rows.iter().filter(|r| r.panicked.is_some()).map(|r| (&r.label, r.ticks, &r.panicked)).collect();
    assert!(panics.is_empty(), "panics: {panics:#?}");
    assert!(
        rows.iter().all(|r| r.result != "did not end"),
        "every opened film ends"
    );
}

/// Why a world could not be created, as a class.
fn refusal_class(detail: &str) -> &'static str {
    if detail.contains("saved game") {
        "a saved game"
    } else if detail.contains("unknown nation") {
        "a nation of the profile's data"
    } else if detail.contains("does not decode") {
        "records that do not decode on this data"
    } else if detail.starts_with("map ") {
        "a map of the profile's data"
    } else if detail.contains("random map") || detail.contains("unknown RMC") {
        "the generator refused"
    } else if detail.contains("no map slot") || detail.contains("no kingdom") || detail.contains("start location") {
        "a player the map has no slot for"
    } else {
        "other"
    }
}

fn panic_text(panic: Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_else(|| "panic".to_owned())
}

fn print_sweep(rows: &[SweepRow]) {
    let mut by_result: BTreeMap<String, usize> = BTreeMap::new();
    for r in rows {
        let result = if r.why.is_empty() {
            r.result.clone()
        } else {
            format!("{} ({})", r.result, r.why)
        };
        *by_result.entry(result).or_default() += 1;
    }
    let ticks: u64 = rows.iter().map(|r| r.ticks).sum();
    let seconds: f64 = rows.iter().map(|r| r.seconds).sum();
    let mut out = String::new();
    out.push_str("| result | films |\n|---|---:|\n");
    for (result, n) in &by_result {
        out.push_str(&format!("| {} | {n} |\n", if result.is_empty() { "panicked at open" } else { result.as_str() }));
    }
    out.push_str(&format!(
        "\n{} world ticks stepped in {seconds:.0} s of simulation: {:.0} ticks per second over the corpus. {} checkpoints checked, {} matched. {} commands reached the world, {} orders unresolved, {} without an aa-play command, {} commands refused.\n",
        ticks,
        ticks as f64 / seconds.max(0.001),
        rows.iter().map(|r| r.checked).sum::<u64>(),
        rows.iter().map(|r| r.matched).sum::<u64>(),
        rows.iter().map(|r| r.commands).sum::<u64>(),
        rows.iter().map(|r| r.unresolved).sum::<u64>(),
        rows.iter().map(|r| r.no_equivalent).sum::<u64>(),
        rows.iter().map(|r| r.refused).sum::<u64>(),
    ));
    out.push_str("\n| film | result | ticks | ticks/s | checkpoints | matched | first mismatch tick | commands | unresolved | refused | load s |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for r in rows {
        let result = if r.why.is_empty() { r.result.clone() } else { format!("{} ({})", r.result, r.why) };
        out.push_str(&format!(
            "| {} | {} | {} | {:.0} | {} | {} | {} | {} | {} | {} | {:.1} |\n",
            r.label,
            result,
            r.ticks,
            r.ticks as f64 / r.seconds.max(0.001),
            r.checked,
            r.matched,
            r.first_mismatch.map_or("-".to_owned(), |m| m.0.to_string()),
            r.commands,
            r.unresolved,
            r.refused,
            r.load_seconds
        ));
    }
    eprintln!("{out}");
    if let Some(path) = std::env::var_os("AA_FILM_SWEEP_OUT") {
        std::fs::write(path, &out).unwrap();
    }
}


/// What a native trace shows at its last whole sampled tick.
#[derive(Debug, Default)]
struct NativeEnd {
    tick: u32,
    /// Owner kingdom index (Actor+E8 -> +188), `?` for a trace without it ->
    /// actors with a Body above zero health.
    alive: BTreeMap<String, usize>,
    /// Owner -> actors whose Body +10 reached zero after being above it.
    reached_zero: BTreeMap<String, usize>,
    /// Actors with a Body ever sampled.
    with_body: usize,
}

fn native_end(path: &Path) -> NativeEnd {
    use std::io::BufRead;
    let reader = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path).unwrap());
    let mut out = NativeEnd::default();
    let mut health: BTreeMap<u32, f32> = BTreeMap::new();
    let mut current: (u32, BTreeMap<String, usize>) = (0, BTreeMap::new());
    let mut previous: (u32, BTreeMap<String, usize>) = (0, BTreeMap::new());
    for line in reader.lines() {
        let line = line.unwrap();
        let mut it = line.split(' ');
        match it.next() {
            Some("T") => {
                let tick = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                previous = std::mem::replace(&mut current, (tick, BTreeMap::new()));
            }
            Some("A") => {
                let _tick = it.next();
                let gtid = it.next().map_or(0, |v| u32::from_str_radix(v, 16).unwrap_or(0));
                let (mut owner, mut hp) = ("?".to_owned(), None);
                for t in it {
                    match t.split_once('=') {
                        Some(("own", v)) => owner = v.to_owned(),
                        Some(("hp", v)) => {
                            let bits = v.split(',').next().unwrap_or("0");
                            hp = Some(f32::from_bits(u32::from_str_radix(bits, 16).unwrap_or(0)));
                        }
                        _ => {}
                    }
                }
                let Some(hp) = hp else {
                    continue;
                };
                if hp > 0.0 {
                    *current.1.entry(owner.clone()).or_default() += 1;
                }
                let before = health.insert(gtid, hp);
                if hp <= 0.0 && before.is_some_and(|b| b > 0.0) {
                    *out.reached_zero.entry(owner).or_default() += 1;
                }
            }
            _ => {}
        }
    }
    out.tick = previous.0;
    out.alive = previous.1;
    out.with_body = health.len();
    out
}

/// The viewer's world after each fight film and Berlin, played through the
/// library, against the original's trace: actors alive by kingdom and how many
/// reached zero health. A report, not a check, until the films pass.
#[test]
#[ignore = "reads the QA films and native traces and the original data; set AA_GAME_ROOT"]
fn watch_films_against_their_native_traces() {
    let root = game_root();
    let traces = root.join("reconstruction/qa-runs/compare-20261022/traces");
    let films = root.join(QA_FILMS);
    let cases = [
        ("Berlin2v2_0", films.join(BERLIN), traces.join("berlin2v2_0-native.txt")),
        ("Comrades2v2_0", films.join("archive-qa-20261101/Comrades2v2_0.RFM"), traces.join("fight-comrades2v2_0-native.txt")),
        ("Farmhouses4FFA_0", films.join("archive-qa-20261101/Farmhouses4FFA_0.RFM"), traces.join("fight-farmhouses4ffa_0-native.txt")),
        ("Kursk2v2_0", films.join("archive-qa-20261101/Kursk2v2_0.RFM"), traces.join("fight-kursk2v2_0-native.txt")),
        ("Berlin2v2_1", films.join("archive-qa-20261101/Berlin2v2_1.RFM"), traces.join("fight-berlin2v2_1-native.txt")),
    ];
    for (label, film, trace) in cases {
        let native = native_end(&trace);
        let data = FilmData::for_film(&root, &film);
        let (mut world, mut player) = open_film(&film, &data, SyncMode::Report).unwrap();
        let start = std::time::Instant::now();
        let mut steps = 0u32;
        while steps < native.tick {
            player.step(&mut world);
            steps += 1;
        }
        let seconds = start.elapsed().as_secs_f64();
        let index_of = |k: Option<u32>| -> String {
            let Some(k) = k else {
                return "none".to_owned();
            };
            world
                .ids
                .kingdom_slot(k)
                .and_then(|s| world.map.slots.get(s))
                .and_then(|s| s.kingdom_index)
                .map_or_else(|| format!("k{k}"), |i| i.to_string())
        };
        let mut alive: BTreeMap<String, usize> = BTreeMap::new();
        let mut zero: BTreeMap<String, usize> = BTreeMap::new();
        let mut zero_kinds: BTreeMap<String, usize> = BTreeMap::new();
        for a in &world.actors {
            if a.alive && !a.removed && a.health > 0.0 {
                *alive.entry(index_of(a.kingdom)).or_default() += 1;
            } else if a.health <= 0.0 {
                *zero.entry(index_of(a.kingdom)).or_default() += 1;
                *zero_kinds.entry(world.data.def(a.def).id.clone()).or_default() += 1;
            }
        }
        let s = player.stats();
        eprintln!("{label}: at tick {} of {} ({:.0} ticks/s)", native.tick, player.film_ticks(), f64::from(steps) / seconds.max(0.001));
        eprintln!("  checkpoints {} of {}, first mismatch {:?}", s.checkpoints_matched, s.checkpoints_checked, s.first_mismatch);
        eprintln!("  alive by kingdom: native {:?} ours {alive:?}", native.alive);
        eprintln!("  reached zero health: native {:?} ({} with a body) ours {zero:?} ({} actors)", native.reached_zero, native.with_body, world.actors.len());
        eprintln!("  ours at zero health: {zero_kinds:?}");
        eprintln!("  orders: {} commands; no equivalent {:?}; refused {:?}", s.commands, s.no_equivalent, s.refused);
        eprintln!("  unresolved {:?}", s.unresolved);
    }
}

/// The dialog's rows and preview against the original's own dialog
/// (qa-output/original-ui/film/03-film-file-dialog.png, on an en-US machine).
#[test]
#[ignore = "reads the QA films; set AA_GAME_ROOT"]
fn the_rows_of_the_dialog_are_the_originals() {
    use aa_parity::playback::{FilmDir, list_films};
    let films = game_root().join(QA_FILMS);
    let rows = list_films(&[FilmDir::original(films.join("archive-qa-20261022"))]);
    assert!(rows.iter().all(|r| r.read_only));
    let mut sorted: Vec<String> = rows.iter().map(|r| r.text.clone()).collect();
    sorted.sort_by_key(|t| t.to_ascii_lowercase());
    assert_eq!(rows.iter().map(|r| r.text.clone()).collect::<Vec<_>>(), sorted, "in the order of their text");
    let row = rows.iter().find(|r| r.preview.name == "LastFilm_2").expect("the film of the screenshot");
    assert_eq!(row.text, "LastFilm 3:39:08 PM 10/24/2026");
    assert_eq!(row.preview.source_map, "#rmc_southerneurope_name");
    assert_eq!(row.preview.file, "LastFilm_2.RFM");
    assert_eq!(row.preview.time, "3:39:08 PM 10/24/2026");
    assert_eq!(row.preview.size, "384x384");
    let kingdoms: Vec<(&str, &str)> = row.preview.kingdoms.iter().map(|k| (k.name.as_str(), k.color_id.as_str())).collect();
    assert_eq!(kingdoms, [("Player", "blue"), ("Random", "red")]);
    let berlin = list_films(&[FilmDir::own(films.join("archive-qa-20261025"))]);
    let row = berlin.iter().find(|r| r.preview.file == "Berlin2v2_0.RFM").unwrap();
    assert!(!row.read_only);
    assert_eq!(row.text, "Berlin2v2_0", "a kept film keeps its name");
    assert_eq!(row.preview.time, "12:53:07 AM 10/25/2026");
    assert_eq!(row.preview.source_map, "(2v2)Berlin.RMP", "the file's name, without the map");
    let berlin = list_films(&[FilmDir::own(films.join("archive-qa-20261025")).with_game_root(game_root())]);
    let row = berlin.iter().find(|r| r.preview.file == "Berlin2v2_0.RFM").unwrap();
    eprintln!("Source of a Berlin film: {:?}", row.preview.source_map);
    assert!(!row.preview.source_map.ends_with(".RMP"), "the map's own name");
}

/// What a film costs before its first tick: the registries computed and read back,
/// the rows of a folder, one film's open with everything already in memory.
#[test]
#[ignore = "reads the original data and the QA films; set AA_GAME_ROOT"]
fn what_opening_costs() {
    use aa_parity::playback::{FilmDir, list_films};
    let root = game_root();
    let t = std::time::Instant::now();
    let computed = Registries::load_v14(&root, true).unwrap();
    let computing = t.elapsed();
    let t = std::time::Instant::now();
    let cached = Registries::cached(&root, None).unwrap();
    let first_cached = t.elapsed();
    let t = std::time::Instant::now();
    let again = Registries::cached(&root, None).unwrap();
    let reading = t.elapsed();
    assert_eq!(cached.object_ids, computed.object_ids);
    assert_eq!(cached.property_ids, computed.property_ids);
    assert_eq!(again.ability_ids, computed.ability_ids);
    let films = root.join(QA_FILMS);
    let t = std::time::Instant::now();
    let rows = list_films(&[FilmDir::own(&films), FilmDir::original(films.join("archive-qa-20261101"))]);
    let listing = t.elapsed();
    let data = FilmData::for_film(&root, &berlin());
    let t = std::time::Instant::now();
    open_film(&berlin(), &data, SyncMode::Report).unwrap();
    let first_open = t.elapsed();
    let t = std::time::Instant::now();
    open_film(&berlin(), &data, SyncMode::Report).unwrap();
    let second_open = t.elapsed();
    eprintln!("registries computed {computing:?}, first cached call {first_cached:?}, read back {reading:?}");
    eprintln!("{} rows in {listing:?}", rows.len());
    eprintln!("open: first {first_open:?}, second {second_open:?}");
}

/// The generator's settings read from a film give back the creator the film holds:
/// `RandomMapData::creator_parts` is what the world creator keeps for them.
#[test]
#[ignore = "reads the QA random-map films and the original data; set AA_GAME_ROOT"]
fn a_random_films_settings_are_its_own() {
    use aa_mapworld::header::DerivedCreator;
    let root = game_root();
    let registries = Registries::load_v14(&root, true).unwrap();
    let rmg = aa_play::mapgen::RandomMapData::load(&root).unwrap();
    let dir = root.join(QA_FILMS).join("archive-qa-20261022");
    for name in ["LastFilm_0.RFM", "LastFilm_1.RFM", "LastFilm_2.RFM"] {
        let film = Film::read(&dir.join(name), &registries).unwrap();
        let params = aa_parity::playback::random_params(&film, &rmg).unwrap();
        let parts = rmg.creator_parts(&params).unwrap();
        let DerivedCreator::Random(settings) = &film.header.creator else {
            panic!("{name} is a random-map film");
        };
        let text = |v: &[u16]| String::from_utf16_lossy(v);
        let weights: Vec<(String, f32)> = settings
            .terrain_weights
            .iter()
            .map(|(i, b)| (text(i), f32::from_bits(*b)))
            .collect();
        assert_eq!(parts.terrain_weights, weights, "{name}: terrain weights");
        // The film holds a group per side where the RMC has one (`start_corps_side01`
        // for `start_corps`): the groups both name must agree.
        let objects: BTreeMap<String, f32> = settings
            .object_counts
            .iter()
            .map(|(i, b)| (text(i), f32::from_bits(*b)))
            .collect();
        let shared: Vec<_> = parts
            .object_counts
            .iter()
            .filter(|(id, _)| objects.contains_key(id))
            .collect();
        assert!(!shared.is_empty(), "{name}: the film names its villages");
        for (id, count) in shared {
            assert_eq!(objects[id], *count, "{name}: object count of {id}");
        }
        assert_eq!(
            parts.total_terrain_weight.to_bits(),
            settings.total_terrain_weight_bits,
            "{name}: total weight"
        );
        eprintln!("{name}: {} sides, {}x{}, heroes {}", params.sides, params.width, params.height, params.heroes);
    }
}

/// What the file dialog skips, silently: what is not a film, a film the game never
/// closed, a film another process is writing (0x59F570; FILM-PLAYBACK.md section 3).
#[test]
fn the_dialog_skips_what_it_cannot_read() {
    use aa_parity::playback::{FilmDir, list_films};
    let dir = std::env::temp_dir().join(format!("aa-parity-film-rows-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("junk.RFM"), b"not a film").unwrap();
    // A chunk file with an empty directory: what the game leaves while recording.
    std::fs::write(dir.join("unclosed.RFM"), b"TGCK\x02\0\0\0\0\0\0\0\0\0\0\0").unwrap();
    std::fs::write(dir.join("notes.TXT"), b"not a film").unwrap();
    std::fs::create_dir_all(dir.join("folder.RFM")).unwrap();
    let rows = list_films(&[FilmDir::own(&dir), FilmDir::original(dir.join("no such folder"))]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(rows, Vec::new());
}

/// Rows with equal text get ` [file]`, every one of them, and a film someone holds
/// open for writing is not listed.
#[test]
#[ignore = "copies a QA film to the temp folder; set AA_GAME_ROOT"]
fn equal_rows_name_their_files_and_a_locked_film_is_skipped() {
    use aa_parity::playback::{FilmDir, list_films};
    let film = game_root().join(QA_FILMS).join("archive-qa-20261022/LastFilm_2.RFM");
    let dir = std::env::temp_dir().join(format!("aa-parity-film-equal-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(&film, dir.join("LastFilm_0.RFM")).unwrap();
    std::fs::copy(&film, dir.join("LastFilm_1 (2).RFM")).unwrap();
    std::fs::copy(&film, dir.join("kept.RFM")).unwrap();
    let rows = list_films(&[FilmDir::own(&dir)]);
    let texts: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            // `__wcsicmp`, so before the capital.
            "kept",
            "LastFilm 3:39:08 PM 10/24/2026 [LastFilm_0]",
            "LastFilm 3:39:08 PM 10/24/2026 [LastFilm_1 (2)]",
        ]
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(dir.join("kept.RFM"))
            .unwrap();
        let rows = list_films(&[FilmDir::own(&dir)]);
        assert!(rows.iter().all(|r| r.preview.file != "kept.RFM"), "{rows:?}");
        assert_eq!(rows.len(), 2);
        drop(locked);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A random-map film's nation and general picks: natively the creator's kingdoms
/// draw them between the seeding and the generator (50ECA6, 474C14, FIGHT-FILMS.md).
/// Do aa-play's draws from the film's seed give the header's picks?
#[test]
#[ignore = "reads the corpus's films and the original data; set AA_GAME_ROOT"]
fn random_film_picks_from_the_seed() {
    use aa_play::creation::{random_general, random_nation};
    let root = game_root();
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let registries = Registries::load_v14(&root, true).unwrap();
    let corpus: Value = serde_json::from_slice(&std::fs::read(corpus_dir().join("corpus.json")).unwrap()).unwrap();
    let (mut same, mut different) = (0, 0);
    for row in corpus["films"].as_array().unwrap() {
        if row["data_set"] != "v14" || row["creator_class"] != "KKC_WorldCreatorRandom" {
            continue;
        }
        let path = corpus_dir().join("films").join(format!("{}.RFM", row["sha256"].as_str().unwrap()));
        let film = Film::read(&path, &registries).unwrap();
        let mut world = aa_play::World::empty(
            data.clone(),
            Arc::new(aa_play::GameMap::default()),
            u64::from(film.header.seed),
        );
        let mut picks = Vec::new();
        for k in &film.header.creator_kingdoms {
            if k.independent {
                continue;
            }
            let Some(h) = film.header.header_kingdoms.iter().find(|h| h.id == k.id) else {
                continue;
            };
            let nation = if k.nation.is_empty() {
                random_nation(&mut world).unwrap_or_default()
            } else {
                k.nation.trim_start_matches("nation_").to_ascii_lowercase()
            };
            let general = if k.faction.is_empty() {
                random_general(&mut world, &nation).unwrap_or_default()
            } else {
                k.faction.clone()
            };
            picks.push((h.nation.clone(), h.faction.clone(), nation, general));
        }
        let ok = picks.iter().all(|(n, f, nation, general)| {
            n.trim_start_matches("nation_").eq_ignore_ascii_case(nation) && f.eq_ignore_ascii_case(general)
        });
        eprintln!("{}: {} {picks:?}", &row["sha256"].as_str().unwrap()[..8], if ok { "same" } else { "DIFFERENT" });
        if ok {
            same += 1;
        } else {
            different += 1;
        }
    }
    eprintln!("{same} films' picks come from the seed, {different} do not");
}
