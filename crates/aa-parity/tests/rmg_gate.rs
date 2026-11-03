//! The random map generator's gate: for every random-map film of the corpus
//! (`target/film-corpus/films`, a film stores only the settings), generation
//! runs without failure, the stream at the end of generation fits the film's
//! first getter chain, the header's random picks come from the stream, and the
//! things (definition, position, height, facing), base and overlay heights and
//! tiles equal the original's where a dump of the original's world exists (RmgProbe
//! `NS_WORLD=1` in the QA copy's `traces/rmg-gate-<first 10 of the sha>.txt`).
//!
//! `CARGO_TARGET_DIR=target/n-rmg AA_GAME_ROOT='<game root>' cargo test -p aa-parity --features play --test rmg_gate --release --offline -- --ignored --nocapture`
//!
//! `AA_RMG_GATE_ONLY=<prefix>` runs the films whose name starts with it,
//! `AA_RMG_GATE_SETS=v14,<folder>` the films of those data sets.
#![cfg(feature = "play")]
use aa_parity::registries::Registries;
use aa_parity::{Film, playback};
use aa_play::mapgen::RandomMapData;
use std::path::{Path, PathBuf};

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}

/// The QA copy of the original under the game root.
fn qa_copy() -> PathBuf {
    game_root().join("reconstruction").join("qa-runs").join("compare-20261022")
}

fn qa_traces() -> PathBuf {
    qa_copy().join("traces")
}

/// The original's world dump: things (GTID slot, ids lower case, x, y, z, facing)
/// and the base heights.
struct NativeWorld {
    /// The world stream at the dump (RNG line).
    end: Option<u32>,
    things: Vec<(u32, String, [u32; 3], [u32; 2])>,
    base: Vec<u32>,
    overlay: Vec<u32>,
    tiles: Vec<String>,
}

fn native_world(path: &Path) -> Option<NativeWorld> {
    let text = String::from_utf8_lossy(&std::fs::read(path).ok()?).into_owned();
    let hex = |s: &str| u32::from_str_radix(s, 16).unwrap_or(0);
    let mut vt = std::collections::BTreeMap::new();
    let mut w = NativeWorld { end: None, things: Vec::new(), base: Vec::new(), overlay: Vec::new(), tiles: Vec::new() };
    for line in text.lines() {
        let mut p = line.splitn(2, ' ');
        let (tag, rest) = (p.next().unwrap_or(""), p.next().unwrap_or(""));
        match tag {
            "VT" => {
                let f: Vec<&str> = rest.split_whitespace().collect();
                vt.insert(f[0].parse::<usize>().unwrap(), f.get(2).copied().unwrap_or("").to_ascii_lowercase());
            }
            "TL" => {
                let (_, cells) = rest.split_once(' ').unwrap();
                w.tiles.extend(cells.split(',').map(|c| vt[&c.split(':').next().unwrap().parse::<usize>().unwrap()].clone()));
            }
            "HB" | "HO" => {
                let (_, cells) = rest.split_once(' ').unwrap();
                let v = if tag == "HB" { &mut w.base } else { &mut w.overlay };
                v.extend(cells.split(',').map(hex));
            }
            "RNG" => {
                w.end = rest.split_whitespace().nth(1).map(hex);
            }
            "TH" => {
                let f: Vec<&str> = rest.split_whitespace().collect();
                if let Some(q) = f.iter().find_map(|s| s.strip_prefix("p=")) {
                    let c: Vec<u32> = q.split(',').map(hex).collect();
                    let d: Vec<u32> = f.iter().find_map(|s| s.strip_prefix("d=")).map_or(vec![0, 0], |d| d.split(',').map(hex).collect());
                    w.things.push((hex(f[0]), f[1].to_ascii_lowercase(), [c[0], c[1], c[2]], [d[0], d[1]]));
                }
            }
            _ => {}
        }
    }
    (!w.base.is_empty()).then_some(w)
}

/// How a stream start fits a film's checkpoints.
struct ChainFit {
    /// Evidence in bits: each interval explained is worth 16 less log2 of its
    /// placements (65536 low halves against that many chances).
    bits: f64,
    /// Intervals with getters the walk explained.
    explained: usize,
    /// The walk stopped on an interval nothing explains before any evidence.
    refused: bool,
}

/// Walks the film's checkpoint intervals from the low half `x` of the first
/// getter (`aa_parity::chain`). The walk's hypothesis, one draw per getter and
/// nothing else, breaks somewhere in every film, so a refusal only counts while
/// no evidence came first. An interval with too many placements to enumerate
/// ends the walk.
fn chain_fit(film: &Film, x: u32) -> ChainFit {
    let prepared = aa_parity::chain::prepare(film);
    let mut state = x;
    let mut fit = ChainFit { bits: 0.0, explained: 0, refused: false };
    for iv in &prepared {
        if iv.residual > 0 {
            let combinations = (0..iv.residual).fold(1f64, |c, k| c * (iv.slots.len() as f64 + f64::from(k)) / (f64::from(k) + 1.0));
            if combinations > 2e6 {
                break;
            }
            if aa_parity::chain::placements(iv, state, 1, 1).is_empty() {
                fit.refused = fit.bits < 16.0;
                break;
            }
            fit.explained += 1;
            fit.bits += (16.0 - combinations.log2()).max(0.0);
            if fit.bits >= 64.0 {
                break;
            }
        }
        state = aa_parity::chain::lcg16_n(state, iv.residual);
    }
    fit
}

/// The low half of the first getter's state after a generator ending at `end`:
/// the world's sync dump draws once (0x4F62B8), then the first getter of the
/// first tick reads the state.
fn first_getter(end: u32) -> u32 {
    let mut state = aa_simulation::NativeRng { state: end };
    state.next_u32();
    state.state & 0xFFFF
}

/// One film's data as the gate reads it.
struct DataSet {
    rmg: RandomMapData,
    game: aa_play::data::GameData,
    registries: Registries,
}

/// The film's data set by the corpus census: the installed game or a profile's
/// data folder over it. `widths` films' data is not held: the installed game
/// stands in and the gate says so.
fn load_set(root: &Path, label: &str) -> anyhow::Result<DataSet> {
    if label == "v14" || label == "widths" {
        return Ok(DataSet { rmg: RandomMapData::load(root)?, game: aa_play::data::GameData::load(root)?, registries: Registries::load_v14(root, true)? });
    }
    let documents = PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default()).join("Documents");
    let data = documents.join(label).join("data");
    let folder = aa_play::data::DataFolder::new(&data)?;
    let game = aa_play::data::GameData::load_on(root, &folder)?;
    let rmg = RandomMapData::load_on(root, &folder)?;
    Ok(DataSet { rmg, game, registries: Registries::cached(root, Some(&data))? })
}

#[derive(Default)]
struct Tally {
    films: usize,
    ran: usize,
    chain_strong: usize,
    chain_weak: usize,
    chain_refused: usize,
    compared: usize,
    state_equal: usize,
    things_equal: usize,
    heights_equal: usize,
    tiles_equal: usize,
    /// The stored settings fit the RMC (0x505B00).
    fit: usize,
    /// Films with a staging "Random" nation or general, and those whose world
    /// start draws reproduce the header's picks (0x50ECA6, 0x474C14).
    random_picks: usize,
    picks_equal: usize,
    /// The chain of the world generated from the stored settings of a film that
    /// does not fit.
    recorded_strong: usize,
    recorded_weak: usize,
    recorded_refused: usize,
}

#[test]
#[ignore = "reads the corpus films and the original data; set AA_GAME_ROOT"]
fn every_random_film_generates_the_original_world() {
    let root = game_root();
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/film-corpus");
    let dir = corpus.join("films");
    let only = std::env::var("AA_RMG_GATE_ONLY").ok();
    let sets_wanted = std::env::var("AA_RMG_GATE_SETS").ok();
    let census: serde_json::Value = serde_json::from_slice(&std::fs::read(corpus.join("corpus.json")).unwrap()).unwrap();
    let mut rows: Vec<(String, String)> = census["films"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["creator_class"] == "KKC_WorldCreatorRandom")
        .map(|f| (f["sha256"].as_str().unwrap().to_owned(), f["data_set"].as_str().unwrap().to_owned()))
        .collect();
    rows.sort_by(|a, b| (&a.1, &a.0).cmp(&(&b.1, &b.0)));
    let mut sets: std::collections::BTreeMap<String, DataSet> = Default::default();
    let mut tallies: std::collections::BTreeMap<String, Tally> = Default::default();
    let mut slowest = 0.0f64;
    for (sha, label) in rows {
        let short = &sha[..10];
        if only.as_ref().is_some_and(|o| !sha.starts_with(o.as_str())) || sets_wanted.as_ref().is_some_and(|w| !w.split(',').any(|x| x == label)) {
            continue;
        }
        let tally = tallies.entry(label.clone()).or_default();
        tally.films += 1;
        if !sets.contains_key(&label) {
            match load_set(&root, &label) {
                Ok(set) => {
                    sets.insert(label.clone(), set);
                }
                Err(e) => {
                    println!("{short} on {label}: the data set does not load ({e:#})");
                    continue;
                }
            }
        }
        let set = &sets[&label];
        let path = dir.join(format!("{sha}.RFM"));
        let film = match Film::read(&path, &set.registries) {
            Ok(f) => f,
            Err(e) => {
                println!("{short} on {label}: not read ({e:#})");
                continue;
            }
        };
        let params = match playback::random_params(&film, &set.rmg) {
            Ok(p) => p,
            Err(e) => {
                println!("{short} on {label}: no settings ({e:#})");
                continue;
            }
        };
        let fits = playback::recorded_params(&film, &set.rmg).is_ok_and(|p| p.1);
        tally.fit += usize::from(fits);
        let spec = playback::world_spec(&film);
        let clock = std::time::Instant::now();
        let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| set.rmg.generate_world_traced(&params, &spec, &set.game)));
        let seconds = clock.elapsed().as_secs_f64();
        slowest = slowest.max(seconds);
        let (map, draws) = match made {
            Ok(Ok(m)) => m,
            Ok(Err(e)) => {
                println!("{short} on {label} {} {}x{} sides {}: FAILED {e:#}", params.rmc, params.width, params.height, params.sides);
                continue;
            }
            Err(_) => {
                println!("{short} on {label} {} {}x{} sides {}: PANIC", params.rmc, params.width, params.height, params.sides);
                continue;
            }
        };
        tally.ran += 1;
        let end = map.map.rng_state.unwrap();
        let fit = chain_fit(&film, first_getter(end));
        if fit.bits >= 32.0 {
            tally.chain_strong += 1;
        } else if fit.refused {
            tally.chain_refused += 1;
        } else {
            tally.chain_weak += 1;
        }
        // The staging "Random" picks draw on the same stream before the generator:
        // the header holds what they came to.
        let in_header = |id: &str| film.header.header_kingdoms.iter().any(|h| h.id.eq_ignore_ascii_case(id));
        let random_kingdoms: Vec<usize> = spec.kingdoms.iter().enumerate().filter(|(_, k)| !k.independent && in_header(&k.ids) && (k.nation.is_empty() || k.general.is_empty())).map(|(i, _)| i).collect();
        if !random_kingdoms.is_empty() {
            let bare = |s: &str| s.trim_start_matches("nation_").to_ascii_lowercase();
            let equal = random_kingdoms.iter().all(|&i| {
                let header = film.header.header_kingdoms.iter().find(|h| h.id.eq_ignore_ascii_case(&spec.kingdoms[i].ids));
                let pick = map.world_picks.get(i);
                let same = |h: &aa_parity::film::KingdomInfo, p: &aa_play::mapgen::WorldPick| {
                    p.nation.as_deref().is_some_and(|n| bare(n) == bare(&h.nation)) && (!spec.kingdoms[i].general.is_empty() || p.general.as_ref().is_some_and(|g| g.eq_ignore_ascii_case(&h.faction)))
                };
                header.zip(pick).is_some_and(|(h, p)| same(h, p))
            });
            tally.random_picks += 1;
            tally.picks_equal += usize::from(equal);
            if !equal {
                println!("   picks differ: header {:?} ours {:?}", film.header.header_kingdoms.iter().map(|h| (h.id.clone(), h.nation.clone(), h.faction.clone())).collect::<Vec<_>>(), map.world_picks);
            }
        }
        let mut line = format!(
            "{short} on {label} {} {}x{} sides {}: {:.2} s, {} draws, end {end:08x}, chain {:.0} bits over {} intervals{}",
            params.rmc,
            params.width,
            params.height,
            params.sides,
            seconds,
            draws.len(),
            fit.bits,
            fit.explained,
            if fit.refused { ", refused" } else { "" }
        );
        if let Some(native) = (label == "v14").then(|| native_world(&qa_traces().join(format!("rmg-gate-{short}.txt")))).flatten() {
            tally.compared += 1;
            let theirs: Vec<&(u32, String, [u32; 3], [u32; 2])> = native.things.iter().filter(|t| t.1 != "team" && t.1 != "kingdom").collect();
            let same = |a: &aa_play::mapgen::GeneratedThing, b: &(u32, String, [u32; 3], [u32; 2])| {
                a.ids.eq_ignore_ascii_case(&b.1) && a.position.map(f32::to_bits) == b.2 && a.facing.map(f32::to_bits) == b.3
            };
            let diff = map.things.iter().zip(&theirs).filter(|(a, b)| !same(a, b)).count() + map.things.len().abs_diff(theirs.len());
            let hdiff = map.terrain.heights.iter().zip(&native.base).filter(|(a, b)| a.to_bits() != **b).count();
            let odiff = map.terrain.mountain_heights.iter().zip(&native.overlay).filter(|(a, b)| a.to_bits() != **b).count();
            let ours: Vec<String> = map.terrain.tiles.iter().map(|v| map.terrain.palette[v].id.to_ascii_lowercase()).collect();
            let tdiff = ours.iter().zip(&native.tiles).filter(|(a, b)| a != b).count();
            tally.state_equal += usize::from(native.end == Some(end));
            tally.things_equal += usize::from(diff == 0);
            tally.heights_equal += usize::from(hdiff == 0 && odiff == 0);
            tally.tiles_equal += usize::from(tdiff == 0);
            let their_fit = native.end.map(|e| chain_fit(&film, first_getter(e)));
            line += &format!(
                "; native end {:08x}{} (its chain {:.0} bits), things {} of {} (native {}) differ, base {hdiff}, overlay {odiff}, tiles {tdiff}",
                native.end.unwrap_or(0),
                if native.end == Some(end) { " same" } else { " DIFFERS" },
                their_fit.map_or(0.0, |f| f.bits),
                diff,
                map.things.len(),
                theirs.len()
            );
            if diff != 0 && std::env::var_os("AA_RMG_GATE_DETAIL").is_some() {
                let mut shown = 0;
                for (a, b) in map.things.iter().zip(&theirs) {
                    if !same(a, b) && shown < 12 {
                        shown += 1;
                        println!("   ours {:x} {} {:08x?} {:08x?} native {:x} {} {:08x?} {:08x?}", a.id, a.ids, a.position.map(f32::to_bits), a.facing.map(f32::to_bits), b.0, b.1, b.2, b.3);
                    }
                }
            }
        }
        if !fits {
            // The world the film was recorded on, as far as the data still says.
            line += "; settings do not fit";
            if let Ok((recorded, _)) = playback::recorded_params(&film, &set.rmg) {
                match set.rmg.generate_world(&recorded, &spec, &set.game) {
                    Ok(m) => {
                        let e = m.map.rng_state.unwrap();
                        let f = chain_fit(&film, first_getter(e));
                        if f.bits >= 32.0 {
                            tally.recorded_strong += 1;
                        } else if f.refused {
                            tally.recorded_refused += 1;
                        } else {
                            tally.recorded_weak += 1;
                        }
                        line += &format!("; as recorded end {e:08x} chain {:.0} bits over {} intervals{}", f.bits, f.explained, if f.refused { ", refused" } else { "" });
                    }
                    Err(e) => line += &format!("; as recorded FAILED {e:#}"),
                }
            }
        }
        println!("{line}");
        for n in map.map.load_notes.iter().filter(|n| !n.starts_with("RMG: Group")).take(6) {
            println!("   note: {n}");
        }
    }
    let mut total = Tally::default();
    for (label, t) in &tallies {
        println!(
            "{label}: films with random picks {}, picks from the stream {}; films {}, generation runs {}, settings fit {}; chain strong {}, weak {}, refused {}; not fitting as recorded: chain strong {}, weak {}, refused {}; compared with a native dump {}: end state equal {}, things equal {}, heights equal {}, tiles equal {}",
            t.random_picks, t.picks_equal, t.films, t.ran, t.fit, t.chain_strong, t.chain_weak, t.chain_refused, t.recorded_strong, t.recorded_weak, t.recorded_refused, t.compared, t.state_equal, t.things_equal, t.heights_equal, t.tiles_equal
        );
        total.films += t.films;
        total.ran += t.ran;
        total.chain_strong += t.chain_strong;
        total.chain_weak += t.chain_weak;
        total.chain_refused += t.chain_refused;
        total.compared += t.compared;
        total.state_equal += t.state_equal;
        total.things_equal += t.things_equal;
        total.heights_equal += t.heights_equal;
        total.tiles_equal += t.tiles_equal;
        total.fit += t.fit;
        total.random_picks += t.random_picks;
        total.picks_equal += t.picks_equal;
        total.recorded_strong += t.recorded_strong;
        total.recorded_weak += t.recorded_weak;
        total.recorded_refused += t.recorded_refused;
    }
    println!(
        "films with random picks {}, picks from the stream {}; random-map films {}; generation runs {}, settings fit {}; chain strong {}, weak {}, refused {}; not fitting as recorded: chain strong {}, weak {}, refused {}; compared with a native dump {}: end state equal {}, things equal {}, heights equal {}, tiles equal {}; slowest {slowest:.2} s",
        total.random_picks, total.picks_equal, total.films, total.ran, total.fit, total.chain_strong, total.chain_weak, total.chain_refused, total.recorded_strong, total.recorded_weak, total.recorded_refused, total.compared, total.state_equal, total.things_equal, total.heights_equal, total.tiles_equal
    );
}

/// `open_film` on the random films (`AA_RMG_GATE_SETS` chooses the data sets,
/// each film on its own: a profile's data folder over the game): the world stands
/// on the generator's stream after the world's sync dump, and its checkpoints are
/// compared from the first one. `AA_RMG_GATE_TICKS` caps the ticks (default all).
#[test]
#[ignore = "reads the corpus films and the original data; set AA_GAME_ROOT"]
fn open_film_starts_on_the_generators_stream() {
    let root = game_root();
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/film-corpus");
    let census: serde_json::Value = serde_json::from_slice(&std::fs::read(corpus.join("corpus.json")).unwrap()).unwrap();
    let only = std::env::var("AA_RMG_GATE_ONLY").ok();
    let sets_wanted = std::env::var("AA_RMG_GATE_SETS").unwrap_or_else(|_| "v14".to_owned());
    let ticks: u32 = std::env::var("AA_RMG_GATE_TICKS").ok().and_then(|t| t.parse().ok()).unwrap_or(u32::MAX);
    let (mut opened, mut on_stream, mut whole, mut first_passed, mut slowest) = (0, 0, 0, 0, 0f64);
    let mut datas: std::collections::BTreeMap<String, playback::FilmData> = Default::default();
    let documents = PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default()).join("Documents");
    let mut rows: Vec<(String, String)> = census["films"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["creator_class"] == "KKC_WorldCreatorRandom")
        .map(|f| (f["data_set"].as_str().unwrap().to_owned(), f["sha256"].as_str().unwrap().to_owned()))
        .collect();
    rows.sort();
    for (label, sha) in rows {
        if !sets_wanted.split(',').any(|x| x == label) || only.as_ref().is_some_and(|o| !sha.starts_with(o.as_str())) {
            continue;
        }
        let data = datas.entry(label.clone()).or_insert_with(|| {
            let mut data = playback::FilmData::new(&root);
            data.force_playback = true;
            if label != "v14" && label != "widths" {
                data.overlay = Some(documents.join(&label).join("data"));
            }
            data
        });
        let path = corpus.join("films").join(format!("{sha}.RFM"));
        let clock = std::time::Instant::now();
        let opening = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| playback::open_film(&path, data, playback::SyncMode::Report)));
        let seconds = clock.elapsed().as_secs_f64();
        let (mut world, mut player) = match opening {
            Ok(Ok(x)) => x,
            Ok(Err(e)) => {
                println!("{} on {label}: refused {e:?}", &sha[..10]);
                continue;
            }
            Err(_) => {
                println!("{} on {label}: PANIC while opening", &sha[..10]);
                continue;
            }
        };
        slowest = slowest.max(seconds);
        opened += 1;
        let end = world.map.rng_state.unwrap();
        let mut next = aa_simulation::NativeRng { state: end };
        next.next_u32();
        let same = world.native_rng.state == next.state;
        on_stream += usize::from(same);
        let placed = world.map.placed.len();
        let actors = world.actors.len();
        let notes = world.ids.notes.len();
        let sched_notes = world.sched.notes.len();
        let total = player.film_ticks();
        let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for _ in 0..ticks.min(total) {
                player.step(&mut world);
                if player.stats().first_mismatch.is_some() {
                    break;
                }
            }
        }));
        let stats = player.stats();
        whole += usize::from(stats.first_mismatch.is_none() && stats.ticks >= u64::from(total));
        first_passed += usize::from(stats.checkpoints_matched > 0);
        println!(
            "{} on {label}: opened in {seconds:.1} s, stream {} (generator's end {end:08x}), {placed} things, {actors} actors, id notes {notes}, queue notes {sched_notes}; {} of {} ticks, {} of {} checkpoints, first mismatch {:?}{}",
            &sha[..10],
            if same { "same" } else { "DIFFERS" },
            stats.ticks,
            total,
            stats.checkpoints_matched,
            stats.checkpoints_checked,
            stats.first_mismatch,
            if played.is_err() { ", PANIC while playing" } else { "" }
        );
    }
    println!("films opened {opened}, on the generator's stream {on_stream}, a first checkpoint passed {first_passed}, played to the end {whole}; slowest open {slowest:.1} s");
}

/// A random film (`AA_RMG_GATE_ONLY`) as `open_film` builds it against the state
/// trace of the original playing it (FightSampler, `AA_RMG_GATE_TRACE`, default
/// `traces/rmg-state-<first 10 of the sha>.txt`): the first difference of every
/// lane and every cause, tick by tick. `AA_RMG_GATE_FORCE` puts the stream back on
/// the native state at every sampled tick.
#[test]
#[ignore = "reads a corpus film, its native state trace and the original data; set AA_GAME_ROOT and AA_RMG_GATE_ONLY"]
fn random_film_native_state() {
    use aa_parity::native_trace::NativeTrace;
    use aa_parity::replay;
    use aa_parity::state_compare::{self, Forcing, StateComparison};
    let root = game_root();
    let only = std::env::var("AA_RMG_GATE_ONLY").expect("AA_RMG_GATE_ONLY");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/film-corpus/films");
    let path = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).find(|p| p.file_stem().is_some_and(|n| n.to_string_lossy().starts_with(&only))).expect("film");
    let registries = Registries::load_v14(&root, true).unwrap();
    replay::set_property_ids(registries.property_ids.clone());
    replay::set_formation_ids(registries.formation_ids.clone());
    aa_parity::mapping::set_ability_ids(registries.ability_ids.clone());
    aa_parity::mapping::set_layout_ids(registries.layout_ids.clone());
    let film = Film::read(&path, &registries).unwrap();
    let trace_path = std::env::var_os("AA_RMG_GATE_TRACE").map_or_else(|| qa_traces().join(format!("rmg-state-{}.txt", &only[..10.min(only.len())])), PathBuf::from);
    let trace = NativeTrace::read(&trace_path).unwrap();
    let mut data = playback::FilmData::new(&root);
    data.force_playback = true;
    let (mut world, _) = playback::open_film(&path, &data, playback::SyncMode::Strict).unwrap();
    let mut comparison = StateComparison::new();
    let mut pairing = replay::Pairing::new(world.ids.next_slot(), &trace.names);
    let last = trace.ticks.keys().last().copied().unwrap_or(0);
    let forcing = Forcing { rng: std::env::var_os("AA_RMG_GATE_FORCE").is_some(), ..Forcing::default() };
    // `AA_RMG_GATE_WATCH=<native gtid hex>:<from>:<to>[,...]` prints those ticks of the things side by side.
    let watches: Vec<(u32, u32, u32)> = std::env::var("AA_RMG_GATE_WATCH").map_or(Vec::new(), |t| {
        t.split(',')
            .filter_map(|w| {
                let mut p = w.split(':');
                Some((u32::from_str_radix(p.next()?, 16).ok()?, p.next()?.parse().ok()?, p.next()?.parse().ok()?))
            })
            .collect()
    });
    let report = replay::run_observed(
        &film,
        &mut world,
        &registries.object_ids,
        Some(last),
        false,
        &replay::Aliases::new(),
        Some(&mut pairing),
        &mut |step, world, pairing| {
            comparison.observe(step, world, &trace, pairing);
            for (gtid, from, to) in &watches {
                if step < *from || step > *to {
                    continue;
                }
                let f = f32::from_bits;
                let native = trace.ticks.get(&step).and_then(|t| t.actors.get(gtid));
                let n = native.map_or("native -".to_owned(), |a| {
                    format!(
                        "native p {:.5},{:.5} d {:.5},{:.5} v {} dest {} st {:x}",
                        f(a.pos[0]),
                        f(a.pos[1]),
                        f(a.dir[0]),
                        f(a.dir[1]),
                        a.mover.map_or(-1.0, |m| f(m.speed)),
                        a.mover.map_or("-".to_owned(), |m| format!("{:.5},{:.5}", f(m.dest[0]), f(m.dest[1]))),
                        a.state,
                    )
                });
                let thing = pairing.and_then(|p| p.ours(*gtid)).and_then(|g| world.ids.thing_at_slot(g as u16));
                let o = match thing {
                    Some(aa_play::ids::Thing::Actor(a)) => {
                        let x = &world.actors[a as usize];
                        format!(
                            "ours kingdom {:?} p {:.5},{:.5} d {:.5},{:.5} v {} path {:?} goal {:?}",
                            x.kingdom,
                            x.pos.x,
                            x.pos.y,
                            x.facing.cos(),
                            x.facing.sin(),
                            x.motion.speed,
                            x.motion.path.iter().take(4).map(|p| (p.x, p.y)).collect::<Vec<_>>(),
                            x.motion.goal.map(|g| (g.x, g.y)),
                        )
                    }
                    other => format!("ours {other:?}"),
                };
                println!("W {step} {gtid:x} {n} | {o}");
            }
            if let Some(native) = trace.ticks.get(&step) {
                state_compare::force(world, native, forcing, pairing);
            }
            comparison.remember(step, world, &trace, pairing);
        },
    );
    println!("{} of {} checkpoints; first difference {:?}; {} sampled ticks", report.matched, report.checked, report.first_difference.as_ref().map(|d| d.check), comparison.ticks_observed);
    for (lane, l) in &comparison.lanes {
        println!("--- {lane}: first at {:?}; {} sampled ticks differ; {} causes", l.first.as_ref().map(|d| d.tick), l.ticks_differing, l.causes.len());
        let mut causes: Vec<_> = l.causes.values().collect();
        causes.sort_by_key(|c| (c.first_tick, c.signature.clone()));
        for c in causes.iter().take(25) {
            println!(
                "  {:>5}..{:<5} ticks {:>5} fields {:>7} things {:>4} first-on {:>4}{} {} | e.g. {:#x} {:?} {}: native {} / ours {}",
                c.first_tick,
                c.last_tick,
                c.ticks,
                c.fields,
                c.things.len(),
                c.first_on.len(),
                if c.knock_on() { " KNOCK-ON" } else { "" },
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

/// A corpus film (`AA_RMG_GATE_ONLY`) against the draws of its native trace
/// (`traces/rmg-full-<first 10 of the sha>.txt`, RmgProbe without NS_FAST): the
/// first draw whose state or site differs.
#[test]
#[ignore = "reads a corpus film, its native trace and the original data; set AA_GAME_ROOT and AA_RMG_GATE_ONLY"]
fn corpus_film_draws() {
    let root = game_root();
    let only = std::env::var("AA_RMG_GATE_ONLY").expect("AA_RMG_GATE_ONLY");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/film-corpus/films");
    let path = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).find(|p| p.file_stem().is_some_and(|n| n.to_string_lossy().starts_with(&only))).expect("film");
    // `AA_RMG_GATE_SETS` names the film's data set and `AA_RMG_GATE_TRACE` the trace
    // of the original on it (default the stock data and the QA copy's trace).
    let label = std::env::var("AA_RMG_GATE_SETS").unwrap_or_else(|_| "v14".to_owned());
    let set = load_set(&root, &label).unwrap();
    let (registries, rmg, game) = (&set.registries, &set.rmg, &set.game);
    let film = Film::read(&path, registries).unwrap();
    let params = playback::random_params(&film, rmg).unwrap();
    let spec = playback::world_spec(&film);
    println!("{} {}x{} sides {} teams {:?} heroes {} kingdoms {:?}", params.rmc, params.width, params.height, params.sides, params.teams, params.heroes, spec.kingdoms);
    println!("object counts {:?} weights {:?}", params.object_counts, params.terrain_weights);
    let trace = std::env::var_os("AA_RMG_GATE_TRACE").map_or_else(|| qa_traces().join(format!("rmg-full-{}.txt", &only[..10.min(only.len())])), PathBuf::from);
    let text = String::from_utf8_lossy(&std::fs::read(trace).unwrap()).into_owned();
    let mut native = Vec::new();
    for line in text.lines() {
        if line.starts_with("M ") && line.contains("SyncReset_77D900") && line.contains("775d4f") {
            break;
        }
        if !line.starts_with("D ") {
            continue;
        }
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() >= 13 {
            native.push((u32::from_str_radix(p[6], 16).unwrap(), u32::from_str_radix(p[10], 16).unwrap(), p[12].to_owned(), p[1].to_owned()));
        }
    }
    let (map, ours) = rmg.generate_world_traced(&params, &spec, game).unwrap();
    println!("native {} draws, ours {}", native.len(), ours.len());
    let first = native.iter().zip(&ours).position(|(n, o)| n.1 != o.1 || n.0 != o.0);
    match first {
        None => println!("all common draws agree"),
        Some(i) => {
            for j in i.saturating_sub(12)..(i + 12).min(native.len()).min(ours.len()) {
                println!("{j} seq {}: native {:08x} {:x} {} ours {:08x} {:x}", native[j].3, native[j].0, native[j].1, &native[j].2[..native[j].2.len().min(70)], ours[j].0, ours[j].1);
            }
        }
    }
    println!("end state {:08x}, things {}", map.map.rng_state.unwrap(), map.things.len());
    for l in map.log.iter().filter(|l| l.what.starts_with("terrain") || l.what.starts_with("balance")).take(60) {
        println!("log: {} {:?}", l.what, l.values);
    }
}

/// Film 2 of the QA copy against its recorded draws: the first draw whose site differs.
#[test]
#[ignore = "reads QA film 2, its native trace and the original data; set AA_GAME_ROOT"]
fn film_2_all_draws() {
    let qa = qa_copy();
    let root = game_root();
    let registries = Registries::load_v14(&root, true).unwrap();
    let film = Film::read(&qa.join(r"game\Profile\data\Films\archive-qa-20261022\LastFilm_2.RFM"), &registries).unwrap();
    let rmg = RandomMapData::load(&root).unwrap();
    let params = playback::random_params(&film, &rmg).unwrap();
    let spec = playback::world_spec(&film);
    println!("{} {}x{} sides {} teams {:?} heroes {} kingdoms {:?}", params.rmc, params.width, params.height, params.sides, params.teams, params.heroes, spec.kingdoms);
    println!("object counts {:?} weights {:?}", params.object_counts, params.terrain_weights);
    let text = std::fs::read_to_string(qa.join(r"traces\film2-rmg.txt")).unwrap();
    let mut native = Vec::new();
    for line in text.lines() {
        if line.starts_with("M ") && line.contains("SyncReset_77D900") && line.contains("775d4f") {
            break;
        }
        if !line.starts_with("D ") {
            continue;
        }
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() >= 13 {
            native.push((u32::from_str_radix(p[6], 16).unwrap(), u32::from_str_radix(p[10], 16).unwrap(), p[12].to_owned()));
        }
    }
    let game = aa_play::data::GameData::load(&root).unwrap();
    let (map, ours) = rmg.generate_world_traced(&params, &spec, &game).unwrap();
    println!("native {} draws, ours {}", native.len(), ours.len());
    let first = native.iter().zip(&ours).position(|(n, o)| n.1 != o.1 || n.0 != o.0);
    match first {
        None => println!("all common draws agree"),
        Some(i) => {
            for j in i.saturating_sub(6)..(i + 8).min(native.len()).min(ours.len()) {
                println!("{j}: native {:08x} {:x} {} ours {:08x} {:x}", native[j].0, native[j].1, &native[j].2[..native[j].2.len().min(60)], ours[j].0, ours[j].1);
            }
        }
    }
    println!("end state {:08x}, things {}", map.map.rng_state.unwrap(), map.things.len());
    for n in map.map.load_notes.iter().take(30) {
        println!("note: {n}");
    }
}
