//! The Berlin film replayed into aa-play and compared, tick by tick and field
//! by field, with the native sampler's snapshots of the original playing the
//! same film (QA tool `NativeSampler`; the trace stays in the QA copy).
//! Reports the first divergence of every lane: rng, existence, economy,
//! production, movement, combat, supply, fog.
//!
//! `AA_GAME_ROOT='E:\Games\Axis and Alies' cargo test -p aa-parity --test berlin_native_state -- --ignored --nocapture`
//! (`AA_PARITY_BERLIN_FILM` / `AA_PARITY_BERLIN_NATIVE` override the film and
//! trace paths).
#![cfg(feature = "play")]
use aa_parity::native_trace::NativeTrace;
use aa_parity::registries::Registries;
use aa_parity::state_compare::StateComparison;
use aa_parity::{Film, replay};
use std::path::PathBuf;
use std::sync::Arc;

const FILM: &str = r"E:\Games\Axis and Alies\reconstruction\qa-runs\compare-20261022\game\Profile\data\Films\archive-qa-20261025\Berlin2v2_0.RFM";
const NATIVE: &str = r"E:\Games\Axis and Alies\reconstruction\qa-runs\compare-20261022\traces\berlin2v2_0-native.txt";
const MAP: &str = "(2v2)Berlin";

fn env_path(var: &str, default: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(|| PathBuf::from(default), PathBuf::from)
}

#[test]
#[ignore = "reads the QA Berlin film, its native trace and the original data; set AA_GAME_ROOT"]
fn berlin_native_state_first_divergence_per_lane() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let registries = Registries::load_v14(&root, true).unwrap();
    let film = Film::read(&env_path("AA_PARITY_BERLIN_FILM", FILM), &registries).unwrap();
    let trace = NativeTrace::read(&env_path("AA_PARITY_BERLIN_NATIVE", NATIVE)).unwrap();
    eprintln!(
        "native trace: {} sampled ticks ({:?}..{:?}), {} named things, {} getters",
        trace.ticks.len(),
        trace.ticks.keys().next(),
        trace.ticks.keys().last(),
        trace.names.len(),
        trace.getters.len()
    );
    // The trace's own consistency with the film: the first battle getter.
    if let Some(&(tick, state, _)) = trace.getters.iter().find(|g| g.0 >= 1) {
        eprintln!("native first battle getter: tick {tick} state {state:#010x}");
    }

    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let map = Arc::new(aa_play::GameMap::load_with(&root, MAP, &data).unwrap());
    let plan = replay::plan(&film, &data, &map).unwrap();
    let mut world = replay::build(&plan, &root, data, map).unwrap();
    eprintln!(
        "random picks against the film header: {:?}",
        replay::pick_mismatches(&plan, &world)
    );

    let mut comparison = StateComparison::new();
    let last = trace.ticks.keys().last().copied().unwrap_or(0);
    let report = replay::run_observed(
        &film,
        &mut world,
        &registries.object_ids,
        Some(last),
        false,
        &replay::Aliases::new(),
        &mut |step, world| comparison.observe(step, world, &trace),
    );
    eprintln!(
        "replay: {} steps, {} of {} checkpoints matched; first checkpoint difference at {:?}",
        report.steps,
        report.matched,
        report.checked,
        report.first_difference.as_ref().map(|d| d.check)
    );
    eprintln!("{} sampled ticks compared", comparison.ticks_observed);
    for (lane, l) in &comparison.lanes {
        match &l.first {
            None => eprintln!(
                "{lane:>10}: no divergence in {} sampled ticks",
                l.ticks_compared
            ),
            Some(d) => {
                eprintln!(
                    "{lane:>10}: first divergence at tick {} ({} fields differ there): {} {:?} {}: native {} / aa-play {}",
                    d.tick,
                    l.differing_at_first_tick,
                    d.gtid.map_or("-".into(), |g| format!("{g:#x}")),
                    d.name,
                    d.field,
                    d.native,
                    d.ours
                );
                for x in l.at_first_tick.iter().skip(1) {
                    eprintln!(
                        "{:>12}{} {:?} {}: native {} / aa-play {}",
                        "",
                        x.gtid.map_or("-".into(), |g| format!("{g:#x}")),
                        x.name,
                        x.field,
                        x.native,
                        x.ours
                    );
                }
            }
        }
    }
    assert!(
        comparison.ticks_observed > 0,
        "the native trace samples the replayed ticks"
    );
}
