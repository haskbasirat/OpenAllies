use super::*;
use serde_json::{Value, json};
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 and real immediate load owners"]
fn actual_farm_late_init_actions_use_structure_then_construction_on_same_storage() {
    use crate::{
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    //Actual retained Session startup E4/F8 is the same bounded root fixture;
    //the rest of its Default is not a full native Session constructor claim.
    let session = aa_netcode::lobby::Session::default();
    //aa-netcode depends on the non-test aa-runtime crate. This local borrow
    //adapts the same Session to the lib-test trait instance, without a copy.
    struct SessionRef<'a>(&'a aa_netcode::lobby::Session);
    impl crate::load_world_globals::SessionPhase for SessionRef<'_> {
        fn current_phase(&self) -> std::result::Result<u32, String> {
            Ok(self.0.field_f8)
        }
        fn application_state(&self) -> std::result::Result<u32, String> {
            Ok(self.0.state)
        }
    }
    let session_ref = SessionRef(&session);
    let mut captures = Vec::new();
    for live_pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        CAPTURE_ENABLED.set(true);
        CAPTURE.with(|capture| capture.borrow_mut().clear());
        let attempt = Attempt::load_immediate(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            live_pc,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::StopOnError,
            LoadBindings {
                native_load_mode: Some(1),
                inactive_load: Some(inactive),
                resources: Some(resources.clone()),
            },
            &mut sources,
            &session_ref,
            false,
        )
        .unwrap();
        CAPTURE_ENABLED.set(false);
        let captured = CAPTURE.with(|c| std::mem::take(&mut *c.borrow_mut()));
        assert_eq!(
            captured.len(),
            1,
            "farm2696 call must be observed, report={:?}",
            attempt.report.events
        );
        let c = &captured[0];
        assert!(c.get("capture_error").is_none(), "{c}");
        assert_eq!(c["same_allocations"], true, "{c}");
        assert_eq!(c["before"]["owner"], 2696);
        assert_eq!(c["before"]["definition"]["id"], "weurope_farm6x6_01");
        assert_eq!(c["result"], json!({"Ok":null}), "{c}");
        for field in [
            "definition",
            "flags",
            "state",
            "owning_kingdom",
            "thing_word_c",
            "object4c",
            "slots",
            "structure",
            "properties_flags",
            "construction",
            "allow_sell_under_siege",
        ] {
            assert_eq!(c["before"][field], c["after"][field], "{field}: {c}");
        }
        //Later ActorInit FX/track/attachment children are separate services;
        //a completed4D1350 must not certify those unimplemented operations.
        assert!(
            attempt
                .report
                .lifecycle
                .iter()
                .any(|e| e.saved_id == 2696 && e.attempted)
        );
        captures.push(captured.into_iter().next().unwrap());
    }
    let document = json!({"scope":"Three actual immediate loads, true post-AfterInit2696 boundary; same held storage and real source/registry, no pre-init substitute. Source and live PC independently recorded.","rows":captures});
    if let Some(path) = std::env::var_os("AA_CAPTURE_ACTOR_ACTION_REFRESH") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap(),
            &document,
        )
        .unwrap();
    } else {
        let p=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261016-load-actor-action-refresh/actual-farm-action-inputs.json");
        let expected: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        assert_eq!(document, expected);
    }
}

#[test]
fn managed_farm_action_outputs_match_full_native_continuation() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-actor-action-refresh");
    let actual: Value = serde_json::from_slice(
        &std::fs::read(folder.join("actual-farm-action-inputs.json")).unwrap(),
    )
    .unwrap();
    let native: Value = serde_json::from_slice(
        &std::fs::read(folder.join("native-farm-action-refresh.json")).unwrap(),
    )
    .unwrap();
    let rows = actual["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(native["rows"].as_array().unwrap().len(), 9);
    for row in native["rows"].as_array().unwrap() {
        let captured = &rows[row["capture_index"].as_u64().unwrap() as usize];
        assert_eq!(row["input"], captured["before"]);
        assert_eq!(row["native"]["outcome"], "complete");
        assert_eq!(row["native"]["actions"], captured["after"]["actions"]);
        let events = row["native"]["events"].as_array().unwrap();
        assert!(
            events.iter().position(|v| v == "0x589a60").unwrap()
                < events.iter().position(|v| v == "0x565140").unwrap()
        );
    }
}
