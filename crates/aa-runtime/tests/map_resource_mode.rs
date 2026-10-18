use aa_runtime::map_resource_mode::{self, LoadGameTaskKind, ScenarioTask};
use serde_json::{Value, json};

fn fixture(path: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn actual_mode_registration_and_header_flag_all_pcs_both_images() {
    let f = fixture("research/skirmish-20261016-load-zone-services/native-initial-zones.json");
    assert_eq!(f["registration"].as_array().unwrap().len(), 7);
    for row in f["registration"].as_array().unwrap() {
        let flags =
            map_resource_mode::world_rule_mode(row["mode"].as_u64().unwrap() as u32).unwrap();
        assert_eq!(flags.name, row["name"].as_str().unwrap());
        assert_eq!(json!(u32::from(flags.saved_state)), row["payload"][0]);
        assert_eq!(json!(u32::from(flags.metagame)), row["payload"][1]);
    }
    let mut count = 0;
    for case in f["cases"].as_array().unwrap() {
        for row in case["mode_rows"].as_array().unwrap() {
            let flags =
                map_resource_mode::world_rule_mode(row["mode"].as_u64().unwrap() as u32).unwrap();
            assert_eq!(json!(u32::from(flags.saved_state)), row["gate_al"]);
            count += 1;
        }
    }
    assert_eq!(count, 126);
    assert!(map_resource_mode::world_rule_mode(7).is_err());
    assert!(map_resource_mode::world_rule_mode(u32::MAX).is_err());
}

#[test]
fn actual_scenario_dispatch_task_constructor_and_temporary_resource_mode() {
    let f = fixture("research/development10/map-resource-mode/native-map-resource-mode.json");
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 114);
    for row in rows {
        let input = &row["input"];
        let selected = map_resource_mode::scenario_task(
            input["captured_game_type"].as_u64().unwrap() as u32,
            input["current_game_type"].as_u64().unwrap() as u32,
            input["scenario_present"].as_bool().unwrap(),
        );
        let actual = match selected {
            ScenarioTask::Load(kind) => {
                assert_eq!(
                    json!(kind.map_resource_load_mode()),
                    row["native"]["load"]["mode_a8"]
                );
                assert_eq!(
                    json!(kind.word()),
                    row["native"]["load"]["after_ctor"]["kind_word"]
                );
                assert_eq!(row["native"]["load"]["after_ctor"]["source_same"], true);
                assert_eq!(
                    row["native"]["load"]["after_ctor"]["result"],
                    if kind.word() == 1 {
                        "source_resource"
                    } else {
                        "null"
                    }
                );
                json!({"kind":"Load", "word":kind.word()})
            }
            ScenarioTask::Film { editor } => json!({"kind":"Film", "editor":editor}),
            ScenarioTask::MissingScenario => json!({"kind":"MissingScenario"}),
        };
        assert_eq!(actual, row["native"]["selected"], "{row}");
        assert_eq!(
            json!(selected != ScenarioTask::MissingScenario),
            row["native"]["accepted"]
        );
    }
    let rows = f["raw_kind_rows"].as_array().unwrap();
    assert_eq!(rows.len(), 30);
    for row in rows {
        let kind = LoadGameTaskKind::from_native_word(row["kind"].as_u64().unwrap() as u32);
        assert_eq!(
            json!(kind.map_resource_load_mode()),
            row["native"]["mode_a8"]
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original maps"]
fn loaded_header_is_distinct_from_selected_creator_conditions() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    for name in [
        "Campaign/tutorial02",
        "Campaign/allies01_el_alamein",
        "(2v2)Berlin",
    ] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(root.join(format!(
                "reconstruction/assets/original/Data/Maps/{name}.RMP"
            )))
            .unwrap(),
        )
        .unwrap();
        let mut header = aa_mapworld::header::MapHeader::decode(
            &map.decode_chunk("MAP_HEADER", u64::MAX).unwrap(),
        )
        .unwrap();
        assert_eq!(header.rules.mode, 0);
        assert!(!map_resource_mode::world_start_flag(&header).unwrap());
        // Diagnostic owner-distinction probe, not a mutation to the original map.
        header.creator.rules.mode = 2;
        assert!(!map_resource_mode::world_start_flag(&header).unwrap());
        header.rules.mode = 2;
        assert!(map_resource_mode::world_start_flag(&header).unwrap());
        header.rules.mode = 7;
        assert!(map_resource_mode::world_start_flag(&header).is_err());
    }
}
