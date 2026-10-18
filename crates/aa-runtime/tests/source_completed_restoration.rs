//! Actual saved Load consumes the very source cache computed before its first
//! record. This remains a stopped record diagnostic, not a battle-start test.
use aa_mapworld::restore::RestoreMode;
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog, resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources}, tgi::Limits};
use aa_runtime::{combat_source_completion as completion,
    combat_source_receipt::{TargetFiles, VerifiedStartupReceipt},
    localization::Localization, restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind}, settings::Settings};
use serde_json::json;
use std::{cell::Cell, sync::Arc};

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets and frozen capture03"]
fn saved_records_read_the_same_verified_source_bank_before_staging() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native = game.join("reconstruction/native-game/runtime-expanded-v14");
    let capture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/development10/native-source-capture-03");
    let exe = std::fs::read(native.join("AA.exe")).unwrap();
    let dll = std::fs::read(native.join("D3D9.dll")).unwrap();
    let config = std::fs::read(native.join("dgVoodoo.conf")).unwrap();
    let receipt = std::fs::read(capture.join("capture.json")).unwrap();
    let manifest = std::fs::read(capture.join("runtime-files.json")).unwrap();
    let mut sources = V14Sources::open(game.join("Data.rwd"), &native,
        SourceLimits {max_member_bytes: 64 * 1024 * 1024}).unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let mut results = Vec::new();
    for name in ["Campaign/tutorial02", "(2v2)Kursk", "DeveloperMaps/(3v3)Firefight"] {
        let map = aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join(format!(
            "reconstruction/assets/original/Data/Maps/{name}.RMP"))).unwrap()).unwrap();
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0).unwrap()
            .begin(TaskKind::Normal).unwrap();
        let address = Cell::new(0usize);
        let expected = Cell::new(None);
        let attempt = Attempt::load_with_source_setup(&map, &actors, settings.clone(), text.clone(),
            Precision::Double53, u64::MAX, RestoreMode::InspectPartial,
            LoadBindings {native_load_mode: Some(1), inactive_load: Some(inactive),
                resources: Some(Arc::clone(&resources))},
            |catalog| {
                assert!(catalog.completed_actor_combat_values("usa_infantry").is_err());
                let proof = VerifiedStartupReceipt::verify(catalog, &mut sources, TargetFiles {
                    native_exe: &exe, renderer_dll: &dll, renderer_config: &config,
                    capture_receipt: &receipt, file_manifest_receipt: &manifest})?;
                completion::install(catalog, &mut sources, proof.context(), proof.csv(), proof.globals())?;
                assert!(catalog.completed_actor_combat_values("usa_infantry").is_err());
                completion::compute(catalog)?;
                expected.set(Some(catalog.completed_actor_combat_values("usa_infantry")?));
                address.set(catalog.definition_bank.borrow().combat_source()? as *const _ as usize);
                Ok(())
            }).unwrap();
        let state = attempt.definitions.combat_source().unwrap();
        assert_eq!(state as *const _ as usize, address.get());
        assert_eq!(state.phase(), completion::Phase::Complete);
        let token = attempt.definitions.token("ActorDefinition", "usa_infantry").unwrap();
        assert_eq!(state.completed_values(token).unwrap(), expected.get().unwrap());
        assert!(!attempt.report.runtime_publication_ready);
        // Neither of the old unsupported boundaries may remain after the
        // actual member-property adapter and source-value cache are installed.
        for event in &attempt.report.events {
            if let Some(error) = &event.error {
                assert!(!error.contains("5784C0 member added is unported"), "{name}: {error}");
                assert!(!error.contains("4618B0 completed canonical ActorData+1E4 source value is unavailable"), "{name}: {error}");
            }
        }
        results.push(json!({"map":name, "cached_definitions":state.manager_order().len(),
            "result_sha256":state.result_fingerprint().unwrap(), "same_box_moved":true,
            "record_errors":attempt.report.events.iter().filter(|e|e.error.is_some()).count(),
            "first_error":attempt.report.events.iter().find(|e|e.error.is_some()),
            "immediate_init_complete":false,"battle_ready":false}));
    }
    let result = json!({"scope":"actual record diagnostics with verified reference source completion; no battle readiness", "maps":results});
    if let Some(path) = std::env::var_os("AA_DEV10_SOURCE_REPORT") {
        let file = std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        serde_json::to_writer_pretty(file, &result).unwrap();
    } else { println!("{result}"); }
}
