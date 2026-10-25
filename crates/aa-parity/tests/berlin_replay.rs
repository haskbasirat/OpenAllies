//! H4: the 2026-10-25 QA film on the stock map (2v2)Berlin (one idle human,
//! three computer players, 4539 world ticks) replayed into aa-play through
//! `Mapper` + `PlayIds`, checkpoint by checkpoint. The film stays in the QA
//! copy; `fixtures/qa_film_berlin_20261025.json` holds its aggregates.
//!
//! `AA_GAME_ROOT='E:\Games\Axis and Alies' cargo test -p aa-parity --test berlin_replay -- --ignored --nocapture`
//! (`AA_PARITY_BERLIN_FILM` overrides the film path).
#![cfg(feature = "play")]
use aa_parity::registries::Registries;
use aa_parity::{Film, replay};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

const FILM: &str = r"E:\Games\Axis and Alies\reconstruction\qa-runs\compare-20261022\game\Profile\data\Films\archive-qa-20261025\Berlin2v2_0.RFM";
const MAP: &str = "(2v2)Berlin";

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}
fn film_path() -> PathBuf {
    std::env::var_os("AA_PARITY_BERLIN_FILM").map_or_else(|| PathBuf::from(FILM), PathBuf::from)
}
fn fixture() -> Value {
    // The fixtures are local (crates/**/tests/fixtures/ is git-ignored), so
    // they are read at run time, not compiled in.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/qa_film_berlin_20261025.json");
    serde_json::from_str(
        &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    )
    .unwrap()
}
fn checkpoint_json(c: &aa_parity::Checkpoint) -> Value {
    serde_json::json!({"tick": c.tick, "count": c.count, "hash": c.hash})
}

#[test]
#[ignore = "reads the QA Berlin film and the original data; set AA_GAME_ROOT"]
fn berlin_film_decodes_to_its_recorded_aggregates() {
    let root = game_root();
    let film = Film::read(&film_path(), &Registries::load_v14(&root, true).unwrap()).unwrap();
    let row = fixture()["film"].clone();
    assert_eq!(film.source.sha256, row["sha256"].as_str().unwrap());
    assert_eq!(film.source.bytes, row["bytes"].as_u64().unwrap());
    assert_eq!(
        film.header.creator_class,
        row["creator_class"].as_str().unwrap()
    );
    assert_eq!(u64::from(film.header.seed), row["seed"].as_u64().unwrap());
    assert_eq!(film.players.len() as u64, row["players"].as_u64().unwrap());
    assert_eq!(
        film.world_ticks() as u64,
        row["world_ticks"].as_u64().unwrap()
    );
    assert_eq!(film.decree_count() as u64, row["decrees"].as_u64().unwrap());
    assert_eq!(
        film.failures().count() as u64,
        row["decode_failures"].as_u64().unwrap()
    );
    assert_eq!(
        film.checkpoints.len() as u64,
        row["checkpoints"].as_u64().unwrap()
    );
    assert_eq!(
        checkpoint_json(&film.checkpoints[0]),
        row["first_checkpoint"]
    );
    assert_eq!(
        checkpoint_json(film.checkpoints.last().unwrap()),
        row["last_checkpoint"]
    );
    assert_eq!(
        serde_json::to_value(film.histogram()).unwrap(),
        row["histogram"]
    );
    let first = aa_parity::slog::account(&film)[0];
    assert_eq!(
        (first.clock, first.decree_entries, first.residual),
        (17, 18, 3),
        "17 clocks, the 3 kingdom names and 3 x (SelectionSet + recruit), 3 getters"
    );
}

/// The original's world-RNG state at this film's first battle getter (the
/// record-2 recruit), measured with RngTrace on two playbacks of the film
/// (research/byte-exact/SYNC_LOG.md section 10): 6231 draws after the map's
/// saved GWORLD state. The diagnostic replay below starts from it until
/// aa-play models those draws.
const MEASURED_FIRST_GETTER_STATE: u32 = 0x4F1E_05E7;

fn print_difference(label: &str, report: &replay::ReplayReport) {
    eprintln!(
        "{label}: {} steps, {} checkpoints checked, {} matched; first difference {}",
        report.steps,
        report.checked,
        report.matched,
        serde_json::to_string_pretty(&report.first_difference).unwrap()
    );
    eprintln!("{label} outcomes: {:?}", report.outcomes);
    eprintln!(
        "{label} unresolved (first): {:?}",
        report.unresolved.iter().take(12).collect::<Vec<_>>()
    );
    eprintln!("{label} refused: {:?}", report.refused);
}

/// The gate: replays the whole film, reports the first checkpoint that
/// differs and why (entry kinds, getter slots, the film's state at aa-play's
/// placement). Structural facts already reached are asserted.
#[test]
#[ignore = "reads the QA Berlin film and the original data; set AA_GAME_ROOT"]
fn berlin_film_replays_into_aa_play() {
    let root = game_root();
    let registries = Registries::load_v14(&root, true).unwrap();
    let film = Film::read(&film_path(), &registries).unwrap();
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let map = Arc::new(aa_play::GameMap::load_with(&root, MAP, &data).unwrap());

    let plan = replay::plan(&film, &data, &map).unwrap();
    eprintln!("plan: {}", serde_json::to_string_pretty(&plan).unwrap());
    assert_eq!(plan.players.len(), 4);
    assert_eq!(plan.players.iter().filter(|p| p.computer).count(), 3);

    let mut world = replay::build(&plan, &root, data.clone(), map.clone()).unwrap();
    let mismatches = replay::pick_mismatches(&plan, &world);
    assert!(
        mismatches.is_empty(),
        "the random picks reproduce the film header: {mismatches:?}"
    );
    eprintln!(
        "aa-play at battle start: {} kingdoms, {} actors, {} live ids, next slot {}, rng state {:#010x}; id notes {:?}",
        world.kingdoms.len(),
        world.actors.len(),
        world.ids.live(),
        world.ids.next_slot(),
        world.native_rng.state,
        world.ids.notes
    );
    // Creation order: the computer players' record-2 recruits select their
    // own Corps HQ by the placeholders' saved slots (13389, 13391, 13393).
    let checks = replay::slot_checks(&film, &plan, &world, 3);
    for c in &checks {
        eprintln!(
            "record 2 selection: slot {} by player {:?} (plan kingdom {:?}) -> aa-play {} (kingdom {:?})",
            c.slot, c.player, c.expected_kingdom, c.ours, c.our_kingdom
        );
    }
    assert_eq!(checks.len(), 3);
    assert!(
        checks.iter().all(|c| c.our_kingdom.is_some()
            && c.our_kingdom == c.expected_kingdom
            && c.ours.contains("corps")),
        "every record-2 selection resolves to its player's Corps HQ"
    );

    let report = replay::run(&film, &mut world, &registries.object_ids, None, false);
    print_difference("replay", &report);
    assert_eq!(report.steps as usize, film.world_ticks());
    assert_eq!(report.checked, film.checkpoints.len());
    // Reached 2026-10-25 with the battle-start draws and the random picks
    // (`World::new_skirmish_with`): checkpoints 17..391 match.
    assert!(report.matched >= 23, "checkpoints 17..391 match ({} matched)", report.matched);
    let first = report.first_difference.as_ref().map(|d| d.check);
    if let Some(d) = &report.first_difference {
        // The getters of the first interval are where the film needs them
        // (one after each recruit) once the HQ selections resolve.
        if d.check.tick == 17 {
            assert_eq!(d.check.ours.0, d.check.film.0, "tick 17 entry count");
        }
    }

    // Diagnostic until the battle-start draws are modelled: start from the
    // measured state (aa-play draws nothing between battle start and the
    // first getter) and report the next difference.
    if first.is_some_and(|c| c.tick == 17 && c.ours.0 == c.film.0) {
        let mut world = replay::build(&plan, &root, data, map).unwrap();
        world.native_rng.state = MEASURED_FIRST_GETTER_STATE;
        let next = replay::run(&film, &mut world, &registries.object_ids, None, true);
        print_difference("measured battle-start state", &next);
        assert!(
            next.matched >= 23,
            "checkpoints 17..391 match from the measured state"
        );
    }
}
