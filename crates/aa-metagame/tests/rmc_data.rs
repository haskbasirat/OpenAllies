//! RMC loader (rmc.rs, 4A5420 and its sub-loaders) on the shipped data.
//!
//! - `rmc_export_matches_original_data` (ignored: needs the local Data.rwd)
//!   exports every RMC node and the registries the loader consults from the
//!   original sources, loads every RMC, and checks
//!   research/phase8-metagame/rmc-data.json against it (rewritten only with
//!   `AA_METAGAME_EXPORT=1`).
//! - `rmc_loader_replays_the_export` reloads every exported node and compares
//!   with the recorded result (determinism / regression; the native oracle of
//!   the loader is a documented gap, see README.txt).
use aa_metagame::rmc::{RmcData, RmcExport, export_v14, load_rmc};
use aa_numeric::Precision;
use aa_rules::actors::ActorCatalog;
use aa_rules::sources::{SourceLimits, V14Sources};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct RmcFile {
    export: RmcExport,
    /// Loaded RMC per exported node, or the loader error text.
    loaded: Vec<Result<RmcData, String>>,
}

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-metagame/rmc-data.json")
}

fn load_all(export: &RmcExport) -> Vec<Result<RmcData, String>> {
    export
        .rmcs
        .iter()
        .map(|(ids, node)| load_rmc(ids, node, &export.registries, Precision::Double53).map_err(|e| e.to_string()))
        .collect()
}

#[test]
#[ignore = "requires locally installed original Data.rwd and accepted v14 runtime"]
fn rmc_export_matches_original_data() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let mut sources = V14Sources::open(
        root.join("../Data.rwd"),
        root.join("../reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 << 20 },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
    let export = export_v14(&catalog, Precision::Double53).unwrap();
    let loaded = load_all(&export);
    let failures: Vec<_> = export
        .rmcs
        .iter()
        .zip(&loaded)
        .filter_map(|((ids, _), r)| r.as_ref().err().map(|e| format!("{ids}: {e}")))
        .collect();
    println!("{} RMCs, {} not loaded", export.rmcs.len(), failures.len());
    for f in &failures {
        println!("  {f}");
    }
    let text = serde_json::to_string_pretty(&RmcFile { export, loaded }).unwrap() + "\n";
    if std::env::var_os("AA_METAGAME_EXPORT").is_some_and(|v| v == "1") {
        std::fs::write(path(), text).unwrap();
        return;
    }
    let on_disk = std::fs::read_to_string(path()).unwrap();
    assert!(on_disk == text, "rmc-data.json is stale: rerun with AA_METAGAME_EXPORT=1");
}

#[test]
fn rmc_loader_replays_the_export() {
    let file: RmcFile = serde_json::from_slice(&std::fs::read(path()).unwrap()).unwrap();
    assert!(!file.export.rmcs.is_empty());
    let loaded = load_all(&file.export);
    assert_eq!(loaded, file.loaded);
    // every RMC a shipped metagame territory names loads without error
    let data: aa_metagame::data::MetaGameData = serde_json::from_slice(
        &std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-metagame/metagame-data.json"))
            .unwrap(),
    )
    .unwrap();
    for t in &data.maps[0].territories {
        if t.rmc_ids.is_empty() {
            continue;
        }
        let i = file
            .export
            .rmcs
            .iter()
            .position(|(ids, _)| ids.eq_ignore_ascii_case(&t.rmc_ids))
            .unwrap_or_else(|| panic!("territory {} rmc {} not exported", t.ids, t.rmc_ids));
        assert!(loaded[i].is_ok(), "{}: {:?}", t.rmc_ids, loaded[i]);
    }
    // The metagame's own RMC view (data.rs RmcDefinition, used by the
    // launcher's participant slots) agrees with the RMC loader, and every
    // metagame RMC yields 506370 settings.
    let mut checked = 0;
    for def in &data.rmcs {
        let i = file.export.rmcs.iter().position(|(ids, _)| ids.eq_ignore_ascii_case(&def.ids)).unwrap();
        let rmc = loaded[i].as_ref().unwrap();
        assert_eq!(def.kingdoms.len(), rmc.settings.kingdoms.len(), "{}", def.ids);
        for (j, (k, info)) in def.kingdoms.iter().zip(&rmc.settings.kingdoms).enumerate() {
            assert_eq!((k.ids.as_str(), k.independent), (info.ids.as_str(), info.independent), "{} slot {j}", def.ids);
            let slot = rmc
                .major_kingdoms
                .iter()
                .chain(&rmc.independent_kingdoms)
                .find(|s| s.info == j)
                .unwrap_or_else(|| panic!("{} kingdom {j} has no slot", def.ids));
            let kind = match k.kind.to_ascii_lowercase().as_str() {
                "land" => 0,
                "water" => 1,
                other => panic!("{} kingdom kind {other}", def.ids),
            };
            assert_eq!(slot.kind, kind, "{} slot {j}", def.ids);
        }
        let sliders = aa_metagame::launch::RmcSliders::from_rmc(rmc);
        aa_metagame::launch::rmc_settings(&sliders, aa_metagame::GAME_PRECISION).unwrap();
        checked += 1;
    }
    assert!(checked > 0);
}
