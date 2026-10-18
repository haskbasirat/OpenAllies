use aa_rules::{
    actors::ActorCatalog,
    native::actor_command::{PREDEFINED_ACTOR_COMMANDS, predefined_actor_command},
    sources::{SourceLimits, V14Sources},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf};

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/resume-20261016-actor-commands/native-actor-commands.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn every_builtin_matches_original_registration_at_all_precisions() {
    let doc = fixture();
    let mut checked = 0;
    for run in doc["runs"].as_array().unwrap() {
        let rows = run["definitions"].as_array().unwrap();
        assert_eq!(rows.len(), PREDEFINED_ACTOR_COMMANDS.len());
        for (ordinal, row) in rows.iter().enumerate() {
            let id = row["ids"].as_str().unwrap();
            let d = predefined_actor_command(id).unwrap();
            assert_eq!(d, PREDEFINED_ACTOR_COMMANDS[ordinal]);
            assert_eq!(d.mind_type as u64, row["mind_type"], "{id}");
            assert_eq!(d.component_slot as u64, row["component_slot"], "{id}");
            assert_eq!(d.behavior_flags as u64, row["behavior_flags"], "{id}");
            checked += 1;
        }
    }
    assert_eq!(checked, 117);
}

#[test]
fn native_lookup_cases_and_invalid_ids() {
    for run in fixture()["runs"].as_array().unwrap() {
        for row in run["lookups"].as_array().unwrap() {
            let d = predefined_actor_command(row["query"].as_str().unwrap());
            if row["id"].is_null() {
                assert!(d.is_err(), "{row:?}");
            } else {
                assert_eq!(d.unwrap().id, row["id"].as_str().unwrap());
            }
        }
    }
    // Saved readers represent null separately. Never coerce it to Stop.
    assert!(predefined_actor_command("").is_err());
}

#[test]
fn authored_ui_flag_reads_preserve_native_dispatch_fields() {
    let mut count = 0;
    let mut saw_aggressive = false;
    let mut saw_offset = false;
    for run in fixture()["runs"].as_array().unwrap() {
        for row in run["authored_flag_loads"].as_array().unwrap() {
            assert_eq!(row["before"], row["after"], "{row:?}");
            let d = predefined_actor_command(row["ids"].as_str().unwrap()).unwrap();
            assert_eq!(
                row["after"],
                json!([d.mind_type, d.component_slot, d.behavior_flags])
            );
            saw_aggressive |= row["ui_flags_3c_to_41_and_position_44"][2] == 1;
            saw_offset |= row["ui_flags_3c_to_41_and_position_44"][1] == 1;
            count += 1;
        }
    }
    assert!(count >= 90);
    assert!(saw_aggressive && saw_offset);
}

#[test]
#[ignore = "requires original Data.rwd and v14 runtime via AA_GAME_ROOT"]
fn v14_authored_commands_match_pinned_native_inputs() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let doc = fixture();
    for source in doc["source_files"].as_array().unwrap() {
        let path = source["path"].as_str().unwrap();
        assert_eq!(
            sources.read(path).unwrap().sha256,
            source["sha256"].as_str().unwrap(),
            "{path}"
        );
    }
    let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
    let registry = catalog.native_data_registry("ActorCommand").unwrap();
    let mut authored = BTreeSet::new();
    for entry in &registry.entries {
        let d = predefined_actor_command(&entry.ids).unwrap();
        authored.insert(d.id);
    }
    let absent: Vec<_> = PREDEFINED_ACTOR_COMMANDS
        .iter()
        .filter(|d| !authored.contains(d.id))
        .map(|d| d.id)
        .collect();
    // These built-ins must remain usable even without authored UI roots.
    for id in &absent {
        assert!(predefined_actor_command(id).is_ok());
    }
    println!(
        "{} predefined commands; {} authored command roots; no authored UI root: {absent:?}",
        PREDEFINED_ACTOR_COMMANDS.len(),
        registry.entries.len()
    );
    assert!(authored.len() >= 30);
    if let Some(path) = std::env::var_os("AA_ACTOR_COMMAND_CENSUS") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&json!({
                "predefined": PREDEFINED_ACTOR_COMMANDS,
                "authored_ids": authored,
                "predefined_without_authored_ui": absent,
                "source_files": doc["source_files"],
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
