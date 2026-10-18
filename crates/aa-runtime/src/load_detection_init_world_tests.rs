use aa_actors::{ActorId, components::ComponentSlot};
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::Precision;
use serde_json::{Value, json};

#[test]
fn native_shared_564d80_gate_and_required_queue_prefix_match_all90_rows() {
    use super::{World, initialize};
    use aa_actors::actor_creation::{Error, Result};
    let fixture: Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261016-load-detection-init/native-scheduled-init.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 90);
    struct Probe {
        version: u32,
        pending: bool,
        head_reads: usize,
        queue_calls: Vec<(u8, u32)>,
    }
    impl World for Probe {
        fn version(&mut self) -> Result<u32> {
            Ok(self.version)
        }
        fn pending(&mut self) -> Result<bool> {
            self.head_reads += 1;
            Ok(self.pending)
        }
        fn add_periodic(&mut self) -> Result<()> {
            self.queue_calls.push((1, 0));
            Err(Error::Unsupported("required50AC60"))
        }
    }
    for row in rows {
        let c = &row["input"];
        let mut world = Probe {
            version: c["version"].as_u64().unwrap() as u32,
            pending: c["scheduled"].as_bool().unwrap(),
            head_reads: 0,
            queue_calls: vec![],
        };
        let result = initialize(&mut world);
        let native = &row["native"];
        assert_eq!(
            world.head_reads,
            native["component_reads"].as_array().unwrap().len(),
            "{row}"
        );
        if native["outcome"] == "complete" {
            assert_eq!(result.unwrap(), true);
            assert!(world.queue_calls.is_empty());
        } else {
            assert!(result.unwrap_err().to_string().contains("required50AC60"));
            assert_eq!(world.queue_calls, [(1, 0)]);
            assert_eq!(native["queue_child"]["this_offset"], 8);
            assert_eq!(native["queue_child"]["data"], 0);
        }
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT actual source and immediate-world/session owners"]
fn capture_actual_detection_6477_at_reached_component_boundary() {
    use crate::{
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources, sha256},
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
    let bytes = std::fs::read(
        game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
    )
    .unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(bytes.clone()).unwrap();
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    // Same real retained Session startup fields as the root immediate-world
    // fixture. Default does not certify the rest of the full Session constructor.
    let session = aa_netcode::lobby::Session::default();
    struct SessionRef<'a>(&'a aa_netcode::lobby::Session);
    impl crate::load_world_globals::SessionPhase for SessionRef<'_> {
        fn current_phase(&self) -> Result<u32, String> {
            Ok(self.0.field_f8)
        }
        fn application_state(&self) -> Result<u32, String> {
            Ok(self.0.state)
        }
    }
    let attempt = Attempt::load_immediate(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
        },
        &mut sources,
        &SessionRef(&session),
        false,
    )
    .unwrap();
    let owner = ActorId::from_raw(6477);
    let RestoredRecord::Actor(entity) = &attempt.registry.get(owner).unwrap().value else {
        panic!("actual6477 is not an Actor")
    };
    let actor = entity.shell.actor.as_ref().unwrap();
    let Some(RestoredComponent::Detection(detection)) =
        actor.components.get(ComponentSlot::Detection)
    else {
        panic!("actual6477 lacks typed Detection")
    };
    assert_eq!(detection.header.owner, owner);
    let source = attempt.definitions.combat_source().unwrap();
    let result = json!({"scope":"Actual tutorial02 ImmediateAttempt owner6477 at its reached Detection boundary. InspectPartial remains a diagnostic census; earlier unrelated failed records may retain prefixes. This is input provenance, not successful Detection or full-world execution.",
        "map_sha256":sha256(&bytes),"source_precision":source.source_precision(),
        "world_version":attempt.report.world_version,"owner":owner.raw(),"definition":entity.definition.proof,
        "definition_flags":entity.definition.flags,"component_mask":entity.definition.component_mask,
        "thing_flags":entity.shell.thing.flags,"position_bits":entity.shell.thing.position_bits,
        "actor_state":actor.components.state,"actor_words":actor.words,"detection":detection,
        "components":actor.components.iter().map(|(s,_)|format!("{s:?}")).collect::<Vec<_>>(),
        "events":attempt.report.events.iter().filter(|e|e.saved_id==Some(owner.raw())).collect::<Vec<_>>(),
        "session_phase":session.field_f8,"session_state":session.state});
    if let Some(path) = std::env::var_os("AA_DETECTION_INITIAL_CAPTURE") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap(),
            &result,
        )
        .unwrap();
    } else {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../research/skirmish-20261016-load-detection-init/actual-6477-before-init.json",
        );
        let expected: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(result, expected);
    }
}
