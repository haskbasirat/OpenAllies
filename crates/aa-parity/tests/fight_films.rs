//! Films of the original with real fighting, recorded for what the Berlin film
//! cannot prove (combat, morale, retreats, special abilities). Each is replayed
//! into aa-play the way `tests/berlin_replay.rs` does and read against its
//! native trace (QA tool `FightSampler`, a copy of `NativeSampler` that adds the
//! owner kingdom, the Mind state and the organization morale fields). The films
//! and traces stay in the QA copy; the aggregates are in the git-ignored
//! `fixtures/qa_films_fight_20261101.json`. Recording recipe, coverage and the
//! measured results: `research/byte-exact/FIGHT-FILMS.md`.
//!
//! ```text
//! AA_GAME_ROOT=<game root> cargo test -p aa-parity --release --test fight_films -- --ignored --nocapture
//! ```
//! `AA_FIGHT_FILMS_ONLY=<label>` limits the replay and trace tests to one film.
//! `AA_FIGHT_FILMS_WRITE=1` makes `fight_films_refresh_fixture` rewrite the
//! fixture from the archived films and traces (only after recording a film).
#![cfg(feature = "play")]
use aa_parity::native_trace::NativeTrace;
use aa_parity::registries::Registries;
use aa_parity::state_compare::StateComparison;
use aa_parity::{Film, classify, replay, slog};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const ARCHIVE: &str = "game/Profile/data/Films/archive-qa-20261101";
const BERLIN: &str = "game/Profile/data/Films/archive-qa-20261025/Berlin2v2_0.RFM";

/// One recorded film: its archive label, the aa-play map id and the trace of its
/// playback under `traces\`.
struct Case {
    label: &'static str,
    map: &'static str,
    trace: &'static str,
}
const FILMS: &[Case] = &[
    Case {
        label: "Farmhouses4FFA_0",
        map: "DeveloperMaps/(4FFA)Farmhouses",
        trace: "fight-farmhouses4ffa_0-native.txt",
    },
    Case {
        label: "Comrades2v2_0",
        map: "DeveloperMaps/(2v2)Comrades",
        trace: "fight-comrades2v2_0-native.txt",
    },
    Case {
        label: "Kursk2v2_0",
        map: "(2v2)Kursk",
        trace: "fight-kursk2v2_0-native.txt",
    },
    Case {
        label: "Berlin2v2_1",
        map: "(2v2)Berlin",
        trace: "fight-berlin2v2_1-native.txt",
    },
];

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}
/// The QA copy of the original, under the game root.
fn qa() -> PathBuf {
    game_root().join("reconstruction/qa-runs/compare-20261022")
}
fn film_path(c: &Case) -> PathBuf {
    qa().join(ARCHIVE).join(format!("{}.RFM", c.label))
}
fn trace_path(c: &Case) -> PathBuf {
    qa().join("traces").join(c.trace)
}
fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/qa_films_fight_20261101.json")
}
fn fixture_row(label: &str) -> Value {
    let path = fixture_path();
    let all: Value = serde_json::from_str(
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
            .trim_start_matches('\u{feff}'),
    )
    .unwrap();
    all["films"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["label"] == label)
        .cloned()
        .unwrap_or_else(|| panic!("no fixture row {label}"))
}
fn checkpoint_json(c: &aa_parity::Checkpoint) -> Value {
    json!({"tick": c.tick, "count": c.count, "hash": c.hash})
}
fn actor_commands(film: &Film) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for d in film.ticks.iter().flat_map(|t| &t.decrees) {
        if let Some((index, name, _)) = classify::actor_command(&d.decree) {
            *out.entry(format!("{index:02} {name}")).or_insert(0usize) += 1;
        }
    }
    out
}
/// The first world tick of every order kind (`classify::order_kind`).
fn first_order_ticks(film: &Film) -> BTreeMap<String, u32> {
    let mut out = BTreeMap::new();
    for t in &film.ticks {
        for d in &t.decrees {
            if let Some(kind) = classify::order_kind(d.descriptor as usize, &d.decree) {
                out.entry(kind).or_insert(t.tick);
            }
        }
    }
    out
}
/// The orders a film issues that Berlin never does, and the decree kinds too.
fn beyond_berlin(film: &Film, berlin: &Film) -> Value {
    let b = actor_commands(berlin);
    let commands: BTreeMap<_, _> = actor_commands(film)
        .into_iter()
        .filter(|(k, _)| !b.contains_key(k))
        .collect();
    let bh = berlin.histogram();
    let decrees: BTreeMap<_, _> = film
        .histogram()
        .into_iter()
        .filter(|(k, _)| !bh.contains_key(k))
        .collect();
    json!({"commands": commands, "decrees": decrees})
}
fn film_row(c: &Case, film: &Film, berlin: &Film, trace: Option<&TraceStats>) -> Value {
    json!({
        "label": c.label,
        "map": c.map,
        "file": format!("{}.RFM", c.label),
        "trace": c.trace,
        "bytes": film.source.bytes,
        "sha256": film.source.sha256,
        "creator_class": film.header.creator_class,
        "seed": film.header.seed,
        "players": film.players.len(),
        "world_ticks": film.world_ticks(),
        "non_empty_ticks": film.non_empty_ticks(),
        "decrees": film.decree_count(),
        "decode_failures": film.failures().count(),
        "checkpoints": film.checkpoints.len(),
        "first_checkpoint": film.checkpoints.first().map(checkpoint_json),
        "last_checkpoint": film.checkpoints.last().map(checkpoint_json),
        "histogram": film.histogram(),
        "actor_commands": actor_commands(film),
        "first_order_ticks": first_order_ticks(film),
        "beyond_berlin": beyond_berlin(film, berlin),
        "native_trace": trace.map(|t| serde_json::to_value(t).unwrap()),
    })
}

/// What a native trace shows of the fighting (a streaming reader: the traces
/// are too large to keep every snapshot here).
#[derive(Clone, Debug, Default, Serialize)]
struct TraceStats {
    sampled_ticks: usize,
    last_tick: u32,
    last_rng: String,
    getters: usize,
    draws: usize,
    actors_seen: usize,
    /// Definition -> actors whose Body +10 reached zero.
    zero_health: BTreeMap<String, usize>,
    first_zero_health_tick: Option<u32>,
    /// Owner kingdom index -> actors whose Body +10 reached zero.
    zero_health_by_owner: BTreeMap<String, usize>,
    /// Definition -> actors that left the GTID table after being sampled alive.
    vanished: BTreeMap<String, usize>,
    /// Actors sampled with less health than in their previous sample.
    damaged_actors: usize,
    /// Organizations sampled with morale below their maximum / at or below the
    /// breakpoint (`578140`: wavering) / at zero (exhausted).
    orgs_below_max_morale: usize,
    orgs_wavering: usize,
    orgs_exhausted: usize,
    /// Mind state (vtable of `0x529F70`'s result, named where aa-mind documents
    /// it) -> actor samples in it.
    mind_states: BTreeMap<String, usize>,
    /// Ticks of the G lines, for [`getters_against_film`].
    #[serde(skip)]
    getter_ticks: Vec<u32>,
}

/// Mind state vtables named in the aa-mind module docs.
fn state_name(vtable: u32) -> String {
    let name = match vtable {
        0xA329F0 => "cai_sleep",
        0xA32A14 => "cai_rampage",
        0xA32A38 => "cai_construct",
        0xA32A80 => "cai_reform",
        0xA32AC8 => "cai_rout",
        0xA32B10 => "cai_unpack",
        0xA32B34 => "cai_pack",
        0xA32B5C => "cai_kill",
        0xA32BA4 => "cai_repair",
        0xA32BC8 => "cai_guard",
        0xA32BEC => "cai_explore",
        0xA32CC8 => "cai_siege",
        0xA32EC4 => "cai_join",
        0xA33140 => "cai_engage",
        0xA3348C => "cai_bombard",
        0xA34774 => "uai_unload",
        0xA34798 => "uai_sleep",
        0xA347BC => "uai_reattach",
        0xA347E0 => "uai_unpack",
        0xA34B14 => "uai_engage",
        0xA34B68 => "uai_repair",
        0xA34B8C => "uai_join",
        0xA35480 => "uai_bombard",
        0xA3246C => "aai_queued",
        0xA32490 => "aai_transport",
        0xA324B4 => "aai_guard",
        0xA32600 => "aai_12c",
        0xA32624 => "aai_takeoff",
        0xA32648 => "aai_landing",
        0xA3266C => "aai_escort",
        _ => return format!("{vtable:06x}"),
    };
    name.to_owned()
}

fn trace_stats(path: &Path) -> std::io::Result<TraceStats> {
    let mut s = TraceStats::default();
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::with_capacity(1 << 20, file);
    let mut names: BTreeMap<u32, String> = BTreeMap::new();
    let mut health: BTreeMap<u32, f32> = BTreeMap::new();
    let mut zero: BTreeSet<u32> = BTreeSet::new();
    let mut damaged: BTreeSet<u32> = BTreeSet::new();
    let (mut below, mut wavering, mut exhausted) =
        (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    let mut seen_now: BTreeSet<u32> = BTreeSet::new();
    let mut alive: BTreeSet<u32> = BTreeSet::new();
    let mut in_tick = false;
    let close = |seen: &mut BTreeSet<u32>,
                     alive: &mut BTreeSet<u32>,
                     names: &BTreeMap<u32, String>,
                     out: &mut BTreeMap<String, usize>| {
        for gtid in alive.difference(seen) {
            let n = names.get(gtid).cloned().unwrap_or_default();
            *out.entry(n).or_default() += 1;
        }
        std::mem::swap(alive, seen);
        seen.clear();
    };
    for line in reader.lines() {
        let line = line?;
        let mut it = line.split(' ');
        match it.next() {
            Some("T") => {
                if in_tick {
                    close(&mut seen_now, &mut alive, &names, &mut s.vanished);
                }
                in_tick = true;
                s.sampled_ticks += 1;
                s.last_tick = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                s.last_rng = it.nth(1).unwrap_or("").to_owned();
            }
            Some("N") => {
                let gtid = it.next().map_or(0, hex);
                names.insert(gtid, it.next().unwrap_or("").to_owned());
            }
            Some("D") => s.draws += 1,
            Some("G") => {
                let toks: Vec<&str> = it.collect();
                let tick = toks
                    .iter()
                    .position(|t| *t == "tick")
                    .and_then(|i| toks.get(i + 1))
                    .and_then(|v| v.parse::<u32>().ok());
                if let Some(t) = tick.filter(|t| *t < 0x0100_0000) {
                    s.getters += 1;
                    s.getter_ticks.push(t);
                }
            }
            Some("A") => {
                let tick: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                let gtid = it.next().map_or(0, hex);
                seen_now.insert(gtid);
                let mut owner = None;
                for t in it {
                    let Some((key, value)) = t.split_once('=') else {
                        continue;
                    };
                    match key {
                        "own" => owner = Some(value.to_owned()),
                        "hp" => {
                            let bits = value.split(',').next().map_or(0, hex);
                            let now = f32::from_bits(bits);
                            if health.get(&gtid).is_some_and(|before| now < *before) {
                                damaged.insert(gtid);
                            }
                            if now <= 0.0
                                && health.get(&gtid).is_some_and(|before| *before > 0.0)
                                && zero.insert(gtid)
                            {
                                s.first_zero_health_tick.get_or_insert(tick);
                                let n = names.get(&gtid).cloned().unwrap_or_default();
                                *s.zero_health.entry(n).or_default() += 1;
                                *s.zero_health_by_owner
                                    .entry(owner.clone().unwrap_or_else(|| "?".into()))
                                    .or_default() += 1;
                            }
                            health.insert(gtid, now);
                        }
                        "mor" => {
                            let v: Vec<u32> = value.split(',').map(hex).collect();
                            if let [morale, max, breakpoint, ..] = v[..] {
                                let (m, x, b) = (
                                    f32::from_bits(morale),
                                    f32::from_bits(max),
                                    f32::from_bits(breakpoint),
                                );
                                if m < x {
                                    below.insert(gtid);
                                }
                                if x > 0.0 && m <= b {
                                    wavering.insert(gtid);
                                }
                                if x > 0.0 && m == 0.0 {
                                    exhausted.insert(gtid);
                                }
                            }
                        }
                        "ms" => {
                            if let Some(state) = value.split(',').nth(1).map(hex)
                                && state != 0
                            {
                                *s.mind_states.entry(state_name(state)).or_default() += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    s.actors_seen = names.len();
    s.damaged_actors = damaged.len();
    s.orgs_below_max_morale = below.len();
    s.orgs_wavering = wavering.len();
    s.orgs_exhausted = exhausted.len();
    Ok(s)
}
fn hex(v: &str) -> u32 {
    u32::from_str_radix(v, 16).unwrap_or(0)
}

/// The film's own S accounting against the getters the sampler saw: at every
/// checkpoint the cumulative residual (entries that are neither clocks nor
/// decrees: the world-RNG getters) must lie between the getters before that
/// tick and the getters up to it. Returns (checkpoints checked, first mismatch).
fn getters_against_film(film: &Film, stats: &TraceStats) -> (usize, Option<Value>) {
    let mut ticks = stats.getter_ticks.clone();
    ticks.sort_unstable();
    // A getter logged at tick 0 belongs to the world creation (before the S
    // reset of session launch 0x775BF0).
    let base = ticks.partition_point(|t| *t < 1);
    // The sampler detaches in the first tick past its maximum: that tick is partial.
    let last = ticks.last().copied().unwrap_or(0);
    let mut cumulative = 0i64;
    let mut checked = 0;
    for iv in slog::account(film) {
        if iv.to_tick >= last {
            break;
        }
        cumulative += i64::from(iv.residual);
        let low = (ticks.partition_point(|t| *t < iv.to_tick) - base) as i64;
        let high = (ticks.partition_point(|t| *t <= iv.to_tick) - base) as i64;
        if !(low..=high).contains(&cumulative) {
            return (
                checked,
                Some(json!({"tick": iv.to_tick, "film_cumulative_residual": cumulative, "trace_low": low, "trace_high": high})),
            );
        }
        checked += 1;
    }
    (checked, None)
}

fn install_id_tables(registries: &Registries) {
    replay::set_formation_ids(registries.formation_ids.clone());
    aa_parity::mapping::set_ability_ids(registries.ability_ids.clone());
    replay::set_property_ids(registries.property_ids.clone());
}

/// Rewrites the fixture. Only with `AA_FIGHT_FILMS_WRITE=1`, after recording.
#[test]
#[ignore = "rewrites the local fixture from the QA films; set AA_GAME_ROOT and AA_FIGHT_FILMS_WRITE"]
fn fight_films_refresh_fixture() {
    if std::env::var_os("AA_FIGHT_FILMS_WRITE").is_none() {
        eprintln!("AA_FIGHT_FILMS_WRITE is not set: nothing written");
        return;
    }
    let registries = Registries::load_v14(&game_root(), true).unwrap();
    let berlin = Film::read(&qa().join(BERLIN), &registries).unwrap();
    let mut rows = Vec::new();
    for c in FILMS {
        let film = Film::read(&film_path(c), &registries).unwrap();
        let stats = trace_stats(&trace_path(c)).ok();
        rows.push(film_row(c, &film, &berlin, stats.as_ref()));
    }
    let out = json!({
        "schema": "aa-parity qa fight films 20261101 aggregates v1",
        "source": "reconstruction/qa-runs/compare-20261022/game/Profile/data/Films/archive-qa-20261101 and traces/ (AA.exe runtime-expanded-v14 QA copy, not committed)",
        "registries": "load_v14 with inferred counts",
        "films": rows,
    });
    std::fs::write(fixture_path(), serde_json::to_string_pretty(&out).unwrap()).unwrap();
}

/// Every archived fight film decodes to its recorded aggregates, without a
/// decode failure, and has really fought (kill orders and deaths in its trace).
#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn fight_films_decode_to_their_recorded_aggregates() {
    let registries = Registries::load_v14(&game_root(), true).unwrap();
    let berlin = Film::read(&qa().join(BERLIN), &registries).unwrap();
    for c in FILMS {
        let film = Film::read(&film_path(c), &registries).unwrap();
        let mut row = film_row(c, &film, &berlin, None);
        let want = fixture_row(c.label);
        // Filled from the trace by the refresh; the order names are the reports' own.
        row["native_trace"] = want["native_trace"].clone();
        row["first_order_ticks"] = want["first_order_ticks"].clone();
        assert_eq!(row, want, "{} is the recorded film", c.label);
        assert_eq!(film.failures().count(), 0, "{}: every record decodes", c.label);
        assert_eq!(film.header.creator_class, "KKC_WorldCreatorLoad", "{}: a stock map", c.label);
        eprintln!(
            "{}: {} ticks, {} checkpoints, {} decrees; orders {:?}; beyond Berlin {}",
            c.label,
            film.world_ticks(),
            film.checkpoints.len(),
            film.decree_count(),
            actor_commands(&film),
            row["beyond_berlin"],
        );
        assert!(
            actor_commands(&film).keys().any(|k| k.ends_with(" kill")),
            "{}: the computer players attack",
            c.label
        );
    }
}

/// The gate: each film into aa-play, printing the matched checkpoints, the first
/// difference and what the mapping could not carry. Nothing about the fighting
/// is asserted yet; the plan, the step count and the recorded floor are.
#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn fight_films_replay_into_aa_play() {
    let root = game_root();
    let registries = Registries::load_v14(&root, true).unwrap();
    install_id_tables(&registries);
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let only = std::env::var("AA_FIGHT_FILMS_ONLY").ok();
    for c in FILMS {
        if only.as_deref().is_some_and(|o| o != c.label) {
            continue;
        }
        let film = Film::read(&film_path(c), &registries).unwrap();
        let map = Arc::new(aa_play::GameMap::load_with(&root, c.map, &data).unwrap());
        eprintln!("{}: map load notes: {} {:?}", c.label, map.load_notes.len(), map.load_notes.iter().take(6).collect::<Vec<_>>());
        let plan = replay::plan(&film, &data, &map).unwrap();
        eprintln!(
            "{}: plan players {:?} notes {:?}",
            c.label,
            plan.players
                .iter()
                .map(|p| format!("{} {} {} team {} computer {}", p.slot_id, p.nation, p.general, p.team_id, p.computer))
                .collect::<Vec<_>>(),
            plan.notes
        );
        eprintln!(
            "{}: rules: resources {:?}, starting experience {:?}, controllers {:?}",
            c.label,
            plan.setup.starting_resources,
            plan.starting_experience,
            plan.setup.players.iter().map(|p| p.controller).collect::<Vec<_>>()
        );
        let mut world = replay::build(&plan, &root, data.clone(), map).unwrap();
        eprintln!(
            "{}: random picks against the film header: {:?}; {} actors, rng {:#010x}",
            c.label,
            replay::pick_mismatches(&plan, &world),
            world.actors.len(),
            world.native_rng.state
        );
        let report = replay::run(&film, &mut world, &registries.object_ids, None, false);
        eprintln!(
            "{}: {} steps, {} checkpoints checked, {} matched; first difference {}",
            c.label,
            report.steps,
            report.checked,
            report.matched,
            serde_json::to_string(&report.first_difference.as_ref().map(|d| json!({
                "check": d.check, "from_tick": d.from_tick, "film": d.film,
                "ours_clock": d.ours.clock, "ours_decree": d.ours.decree, "ours_getters": d.ours.getters, "ours_other": d.ours.other,
            }))).unwrap()
        );
        eprintln!("{}: outcomes {:?}", c.label, report.outcomes);
        eprintln!(
            "{}: {} unresolved (first {:?}); {} refused (first {:?})",
            c.label,
            report.unresolved.len(),
            report.unresolved.iter().take(6).collect::<Vec<_>>(),
            report.refused.len(),
            report.refused.iter().take(6).collect::<Vec<_>>()
        );
        assert_eq!(report.steps as usize, film.world_ticks(), "{}", c.label);
        assert_eq!(report.checked, film.checkpoints.len(), "{}", c.label);
    }
}

/// Each trace belongs to its film: the getters the sampler saw account for every
/// S entry of the film's checkpoints that is neither a clock nor a decree. Prints
/// what the trace shows of the fighting.
#[test]
#[ignore = "reads the QA films and traces and the original data; set AA_GAME_ROOT"]
fn fight_film_traces_account_for_their_checkpoints() {
    let registries = Registries::load_v14(&game_root(), true).unwrap();
    let only = std::env::var("AA_FIGHT_FILMS_ONLY").ok();
    for c in FILMS {
        if only.as_deref().is_some_and(|o| o != c.label) {
            continue;
        }
        let film = Film::read(&film_path(c), &registries).unwrap();
        let stats = trace_stats(&trace_path(c)).unwrap();
        eprintln!("{}: {}", c.label, serde_json::to_string_pretty(&stats).unwrap());
        let (checked, mismatch) = getters_against_film(&film, &stats);
        eprintln!(
            "{}: {checked} of {} checkpoints agree with the trace's getters; first mismatch {mismatch:?}",
            c.label,
            film.checkpoints.len()
        );
        assert!(mismatch.is_none(), "{}", c.label);
        assert!(checked + 1 >= film.checkpoints.len(), "{}: the trace covers the film", c.label);
        if only.is_none() {
            assert_eq!(
                serde_json::to_value(&stats).unwrap(),
                fixture_row(c.label)["native_trace"],
                "{}: the trace of the recorded playback",
                c.label
            );
        }
    }
}

/// The films replayed into aa-play against their native traces, lane by lane
/// (`tests/berlin_native_state.rs`): the first divergence of each.
#[test]
#[ignore = "reads the QA films and traces and the original data; set AA_GAME_ROOT"]
fn fight_films_native_state_first_divergence_per_lane() {
    let root = game_root();
    let registries = Registries::load_v14(&root, true).unwrap();
    install_id_tables(&registries);
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let only = std::env::var("AA_FIGHT_FILMS_ONLY").ok();
    for c in FILMS {
        if only.as_deref().is_some_and(|o| o != c.label) {
            continue;
        }
        let film = Film::read(&film_path(c), &registries).unwrap();
        let trace = NativeTrace::read(&trace_path(c)).unwrap();
        let map = Arc::new(aa_play::GameMap::load_with(&root, c.map, &data).unwrap());
        let plan = replay::plan(&film, &data, &map).unwrap();
        let mut world = replay::build(&plan, &root, data.clone(), map).unwrap();
        let mut comparison = StateComparison::new();
        let mut pairing = replay::Pairing::new(world.ids.next_slot(), &trace.names);
        let last = trace.ticks.keys().last().copied().unwrap_or(0);
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
                comparison.remember(step, world, &trace, pairing);
            },
        );
        eprintln!(
            "{}: {} of {} checkpoints matched; {} sampled ticks compared",
            c.label, report.matched, report.checked, comparison.ticks_observed
        );
        for (lane, l) in &comparison.lanes {
            match &l.first {
                None => eprintln!("{lane:>10}: no divergence in {} sampled ticks", l.ticks_compared),
                Some(d) => eprintln!(
                    "{lane:>10}: first divergence at tick {} ({} fields differ there): {} {:?} {}: native {} / aa-play {}",
                    d.tick,
                    l.differing_at_first_tick,
                    d.gtid.map_or("-".into(), |g| format!("{g:#x}")),
                    d.name,
                    d.field,
                    d.native,
                    d.ours
                ),
            }
        }
        assert!(comparison.ticks_observed > 0, "{}", c.label);
    }
}
