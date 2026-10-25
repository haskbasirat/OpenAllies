//! The three QA films of 2026-10-22/24 (AA.exe runtime-expanded-v14,
//! `rmc_southerneurope` random maps), decoded and checked against the
//! aggregate numbers in `fixtures/qa_films_20261022.json`. The films stay in
//! the QA copy; nothing of them is committed except these aggregates.
//!
//! Run with
//! `AA_GAME_ROOT='E:\Games\Axis and Alies' cargo test -p aa-parity --test qa_films -- --ignored`
//! (`AA_PARITY_FILMS` overrides the film directory).
use aa_parity::registries::Registries;
use aa_parity::{Film, classify, slog};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

const DEFAULT_FILMS: &str = r"E:\Games\Axis and Alies\reconstruction\qa-runs\compare-20261022\game\Profile\data\Films\archive-qa-20261022";

fn films_dir() -> PathBuf {
    std::env::var_os("AA_PARITY_FILMS").map_or_else(|| PathBuf::from(DEFAULT_FILMS), PathBuf::from)
}
fn game_root() -> PathBuf {
    PathBuf::from(
        std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT (the v14 data for registry counts)"),
    )
}
fn fixture() -> Value {
    // The fixtures are local (crates/**/tests/fixtures/ is git-ignored), so
    // they are read at run time, not compiled in.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/qa_films_20261022.json");
    serde_json::from_str(
        &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    )
    .unwrap()
}
fn histogram_json(film: &Film) -> Value {
    serde_json::to_value(film.histogram()).unwrap()
}
fn actor_commands(film: &Film) -> Value {
    let mut out = BTreeMap::new();
    for d in film.ticks.iter().flat_map(|t| &t.decrees) {
        if let Some((index, name, _)) = classify::actor_command(&d.decree) {
            *out.entry(format!("{index:02} {name}")).or_insert(0usize) += 1;
        }
    }
    serde_json::to_value(out).unwrap()
}
fn checkpoint_json(c: &aa_parity::Checkpoint) -> Value {
    serde_json::json!({"tick": c.tick, "count": c.count, "hash": c.hash})
}

#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn qa_films_decode_to_their_recorded_aggregates() {
    let root = game_root();
    let inferred = Registries::load_v14(&root, true).unwrap();
    let proven = Registries::load_v14(&root, false).unwrap();
    let fixture = fixture();
    let rows = fixture["films"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    if std::env::var_os("AA_QA_FILMS_PRINT").is_some() {
        // The decoded aggregates of every film, for refreshing the fixture.
        for row in rows {
            let path = films_dir().join(row["file"].as_str().unwrap());
            let (film, strict) = (Film::read(&path, &inferred).unwrap(), Film::read(&path, &proven).unwrap());
            eprintln!(
                "{}: {}",
                row["file"],
                serde_json::json!({
                    "decrees": film.decree_count(),
                    "decode_failures": film.failures().count(),
                    "proven_only_decrees": strict.decree_count(),
                    "proven_only_decode_failures": strict.failures().count(),
                    "histogram": histogram_json(&film),
                    "actor_commands": actor_commands(&film),
                })
            );
        }
    }
    for row in rows {
        let path = films_dir().join(row["file"].as_str().unwrap());
        let film = Film::read(&path, &inferred).unwrap();
        let name = row["file"].as_str().unwrap();
        assert_eq!(
            film.source.sha256,
            row["sha256"].as_str().unwrap(),
            "{name} is the recorded film"
        );
        assert_eq!(film.source.bytes, row["bytes"].as_u64().unwrap());
        assert_eq!(
            film.chunks.len(),
            5,
            "{name}: MAP, MAP_HEADER, MAPSTRINGS, SessionPlayers, Orders"
        );
        assert_eq!(
            film.header.creator_class,
            row["creator_class"].as_str().unwrap()
        );
        assert_eq!(u64::from(film.header.seed), row["seed"].as_u64().unwrap());
        assert_eq!(film.header.rmc.as_deref(), row["rmc"].as_str());
        assert_eq!(film.players.len() as u64, row["players"].as_u64().unwrap());
        assert_eq!(
            film.world_ticks() as u64,
            row["world_ticks"].as_u64().unwrap()
        );
        assert_eq!(
            film.non_empty_ticks() as u64,
            row["non_empty_ticks"].as_u64().unwrap()
        );
        // Since the measured SharedProperty / Ability counts
        // (registries::MEASURED_V14) every decree of the three films decodes:
        // film 1's former failure, a TellSelectedCommand carrying research,
        // now counts as a decree (fixture 771 -> 772 decrees, 1 -> 0 failures).
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
        let at34 = film.checkpoints.iter().find(|c| c.tick == 34).unwrap();
        assert_eq!(checkpoint_json(at34), row["checkpoint_tick_34"]);
        assert_eq!(
            checkpoint_json(film.checkpoints.last().unwrap()),
            row["last_checkpoint"]
        );
        assert_eq!(histogram_json(&film), row["histogram"]);
        assert_eq!(actor_commands(&film), row["actor_commands"]);
        // Checkpoints come every 17 world ticks, counts never decrease.
        assert!(
            film.checkpoints
                .windows(2)
                .all(|w| w[1].count >= w[0].count && w[1].tick > w[0].tick)
        );

        let strict = Film::read(&path, &proven).unwrap();
        assert_eq!(
            strict.decree_count() as u64,
            row["proven_only_decrees"].as_u64().unwrap()
        );
        assert_eq!(
            strict.failures().count() as u64,
            row["proven_only_decode_failures"].as_u64().unwrap()
        );
    }
}

#[test]
#[ignore = "reads the QA films; set AA_GAME_ROOT"]
fn qa_film_checkpoints_are_not_clock_only() {
    // Every interval logs more S entries than world ticks: the native S log
    // holds more than one clock entry per tick (RNG getters, markers).
    let registries = Registries::load_v14(&game_root(), true).unwrap();
    for name in ["LastFilm_0.RFM", "LastFilm_1.RFM", "LastFilm_2.RFM"] {
        let film = Film::read(&films_dir().join(name), &registries).unwrap();
        let first = film.checkpoints[0];
        assert_eq!(
            (first.tick, first.count),
            (17, 30),
            "{name}: 17 ticks, 30 S entries"
        );
        assert!(
            slog::clock_only_intervals(&film.checkpoints).is_empty(),
            "{name}"
        );
    }
}
