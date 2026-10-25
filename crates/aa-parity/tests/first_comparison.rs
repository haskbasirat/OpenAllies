//! First S-log comparison (roadmap §5.3 step 4): aa-play's trace of a
//! generated `rmc_southerneurope` battle set up like QA film 2 (seed
//! 972257557, side01 ger human, side02 jpn computer) against the film's 15
//! `KKC_SetSimSyncValueOrderDecree` checkpoints. A report, not a parity
//! claim: the film decrees are not replayed yet (they need the native id
//! table), the random map is not the native generator, and aa-play logs only
//! the clock and its own world-RNG getters.
//!
//! `AA_GAME_ROOT='E:\Games\Axis and Alies' cargo test -p aa-parity --test first_comparison -- --ignored --nocapture`
#![cfg(feature = "play")]
use aa_parity::Film;
use aa_parity::registries::Registries;
use aa_parity::slog::{Outcome, SEntryKind, SLog};
use std::path::PathBuf;
use std::sync::Arc;

const FILM: &str = r"E:\Games\Axis and Alies\reconstruction\qa-runs\compare-20261022\game\Profile\data\Films\archive-qa-20261022\LastFilm_2.RFM";

#[test]
#[ignore = "reads QA film 2 and the original data; set AA_GAME_ROOT"]
fn film_2_checkpoints_against_aa_play_trace() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let film_path =
        std::env::var_os("AA_PARITY_FILM").map_or_else(|| PathBuf::from(FILM), PathBuf::from);
    let film = Film::read(&film_path, &Registries::builtin()).unwrap();
    assert_eq!(film.header.seed, 972_257_557);

    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let rmg = aa_play::mapgen::RandomMapData::load(&root).unwrap();
    let info = rmg
        .catalog()
        .rmcs
        .into_iter()
        .find(|r| r.ids == "rmc_southerneurope")
        .unwrap();
    let generated = rmg
        .generate(&aa_play::mapgen::RandomMapParams::defaults(
            &info,
            2,
            u64::from(film.header.seed),
        ))
        .unwrap();
    let setup = aa_play::SkirmishSetup {
        map_id: "generated".into(),
        players: vec![
            aa_play::PlayerSetup {
                name: "side01".into(),
                nation: "ger".into(),
                team: 0,
                color: [0, 76, 255],
                controller: aa_play::Controller::Human,
                start: 0,
            },
            aa_play::PlayerSetup {
                name: "side02".into(),
                nation: "jpn".into(),
                team: 1,
                color: [217, 0, 25],
                controller: aa_play::Controller::Computer(1),
                start: 1,
            },
        ],
        starting_resources: None,
        seed: u64::from(film.header.seed),
    };
    let mut world = aa_play::World::new_skirmish(data, Arc::new(generated.map), &setup).unwrap();
    aa_play::ai::attach(
        &mut world,
        Arc::new(aa_play::ai::AiData::load(&root).unwrap()),
    );
    assert_eq!(
        world.trace.checkpoint(),
        (0, 0),
        "battle start: 0x77D900(0, 0)"
    );
    world.trace.record(true);
    for _ in 0..film.world_ticks() {
        world.step(Vec::new());
    }
    let log = SLog::from_play_trace(&world.trace);
    assert_eq!(
        log.checkpoint(),
        world.trace.checkpoint(),
        "the harness recomputes aa-play's own S state"
    );
    let clocks = (1..=log.len() as i32)
        .filter(|&i| matches!(log.entry(i), Some((_, SEntryKind::Clock, _, _))))
        .count();
    assert_eq!(clocks, film.world_ticks(), "one clock entry per world tick");

    let comparison = log.compare(&film.checkpoints);
    eprintln!(
        "film tick | film count | film hash | aa-play count at tick | aa-play hash at film count | outcome"
    );
    for r in &comparison.results {
        eprintln!(
            "{:>9} | {:>10} | {:>9} | {:>21} | {:>26} | {:?}",
            r.film_tick,
            r.count,
            r.film_hash,
            r.our_count_at_film_tick,
            r.our_hash.map_or("-".into(), |h| h.to_string()),
            r.outcome
        );
    }
    eprintln!(
        "matched {} of {}",
        comparison.matched(),
        comparison.results.len()
    );
    assert_eq!(comparison.results.len(), film.checkpoints.len());
    // Recorded status (2026-10-24): aa-play logs fewer entries than the
    // original in the same ticks, so no checkpoint matches yet.
    assert!(
        comparison
            .results
            .iter()
            .all(|r| r.outcome != Outcome::Forgotten)
    );
}
