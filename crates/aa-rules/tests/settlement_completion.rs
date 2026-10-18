use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    objects::Shape,
    settlement_completion::{Level, SharedActor, SharedDefinitions, WallSet, initial_radius},
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn pc(v: &Value) -> Precision {
    match v.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!("unknown fixture precision"),
    }
}
fn piece_refs(v: &Value) -> [String; 3] {
    std::array::from_fn(|i| format!("a{}", v[i].as_u64().unwrap()))
}
fn actor(blocking: [u32; 6], soft: [u32; 6]) -> SharedActor {
    SharedActor {
        component_mask: 1 << 13,
        blocking: Shape { words: blocking },
        soft: Shape { words: soft },
        settlement_owner: None,
        settlement_wall: 0,
    }
}

#[test]
fn shared_definition_prefix_matches_native_aliases_and_error_order() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-settlement-completion/native-prefix.json"
    ))
    .unwrap();
    assert_eq!(
        doc["original_sha256"],
        "5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26"
    );
    let mut checked = 0;
    for row in doc["prefix_cases"].as_array().unwrap() {
        let mut data = SharedDefinitions::default();
        if row["mode"] == "error" {
            let case = row["case"].as_str().unwrap();
            let mut state = actor([0; 6], [0; 6]);
            if case == "not_structure" {
                state.component_mask = 0;
            }
            if case == "already_owned" {
                state.settlement_owner = Some("old".into());
            }
            data.actors.insert("center".into(), state);
            let center = if case == "null_center" {
                None
            } else {
                Some("center".into())
            };
            let mut levels = Vec::new();
            let result = data.append_level(
                "settlement",
                &mut levels,
                Level {
                    maximum_structures: 7,
                    center,
                    wall_set: None,
                },
            );
            assert!(result.is_err(), "{row}");
            assert_eq!(levels.len(), 1, "failed Level record remains appended");
            assert_eq!(
                data.actors["center"].settlement_owner.as_deref() == Some("settlement"),
                row["owner_written"].as_bool().unwrap(),
                "{row}"
            );
            assert_eq!(
                data.actors["center"].settlement_owner.as_deref() == Some("old"),
                row["old_owner_preserved"].as_bool().unwrap(),
                "{row}"
            );
            assert_eq!(
                data.actors["center"].settlement_owner.is_none(),
                case == "null_center" || case == "not_structure"
            );
        } else {
            for i in 0..6 {
                let shapes = &row["initial_shapes"][i];
                data.actors.insert(
                    format!("a{i}"),
                    actor(
                        std::array::from_fn(|j| word(&shapes[0][j])),
                        std::array::from_fn(|j| word(&shapes[1][j])),
                    ),
                );
            }
            let first = WallSet {
                id: "first".into(),
                pieces: piece_refs(&row["source"]),
            };
            let current = WallSet {
                id: if row["same_wallset"].as_bool().unwrap() {
                    "first"
                } else {
                    "current"
                }
                .into(),
                pieces: piece_refs(&row["target"]),
            };
            if row["mode"] == "level" {
                let mut levels = vec![Level {
                    maximum_structures: 0,
                    center: None,
                    wall_set: Some(first),
                }];
                data.append_level(
                    "settlement",
                    &mut levels,
                    Level {
                        maximum_structures: 7,
                        center: Some("a5".into()),
                        wall_set: Some(current),
                    },
                )
                .unwrap();
            } else {
                data.initialize_placeholders(&first, &piece_refs(&row["target"]))
                    .unwrap();
            }
            let actual:Vec<_>=(0..6).map(|i| {
                let a=&data.actors[&format!("a{i}")];
                json!({"blocking":a.blocking.words,"soft":a.soft.words,"settlement_wall":a.settlement_wall,"owner":a.settlement_owner.as_deref()==Some("settlement")})
            }).collect();
            assert_eq!(json!(actual), row["result"], "{row}");
        }
        checked += 1;
    }
    assert_eq!(checked, 384);
    for row in doc["initial_radius_cases"].as_array().unwrap() {
        assert_eq!(
            initial_radius(word(&row["diameter"]), pc(&row["precision"])).unwrap(),
            word(&row["initial_radius"]),
            "{row}"
        );
    }
}

fn catalog(text: &str) -> ActorCatalog {
    load_files(
        vec![ResolvedFile {
            logical_path: "settlement-test.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(text.as_bytes()),
            bytes: text.as_bytes().to_vec(),
        }],
        Limits::default(),
        &mut NativeLoadReport::default(),
    )
}
fn source(absent: i64, missing_wall: i64, corner_targetable: bool) -> String {
    let mut text = String::new();
    for i in 0..3 {
        text.push_str(&format!("[Thing]\n{{\nstring IDS = a{i}\nstring category = ACTOR\n[Size]\n{{\nfixed height = 3\nfixed scale_min = 1\nfixed scale_max = 1\nstring blocking_shape = CIRCLE\nfixed blocking_radius = 2\nstring soft_shape = CIRCLE\nfixed soft_radius = 2\nstring target_shape = null\n}}\n[Flags]\n{{\n"));
        for &(field, _) in aa_rules::native::THING_FLAGS {
            let value = field == "targetable" && i == 1 && corner_targetable;
            text.push_str(&format!("bool {field} = {value}\n"));
        }
        text.push_str("}\n");
        if i != missing_wall {
            text.push_str("[WallComponent]\n{\n}\n");
        }
        text.push_str("}\n");
    }
    text.push_str("[WallSet]\n{\nstring IDS = main\nstring name = main\n");
    for (i, field) in ["wall_segment_ids", "wall_corner_ids", "gate_ids"]
        .iter()
        .enumerate()
    {
        let id = if i as i64 == absent {
            "missing".into()
        } else {
            format!("a{i}")
        };
        text.push_str(&format!("string {field} = {id}\n"));
    }
    text.push_str("}\n[Translate]\n{\nstring node = WallSet\nstring ids_old = alias\nstring ids_new = main\n}\n");
    text
}

#[test]
fn wall_set_binding_matches_native_validation_suffix() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-settlement-completion/native-prefix.json"
    ))
    .unwrap();
    for row in doc["wall_set_cases"].as_array().unwrap() {
        let text = source(
            row["absent"].as_i64().unwrap(),
            row["missing_wall"].as_i64().unwrap(),
            row["corner_targetable"].as_bool().unwrap(),
        );
        let cat = catalog(&text);
        let result = cat.native_wall_set_inputs("main", pc(&row["precision"]));
        assert_eq!(
            result.is_ok(),
            row["error_address"].is_null(),
            "{row}: {result:?}"
        );
        if let Ok(value) = result {
            assert_eq!(value.canonical_id, "main");
            assert_eq!(
                value.pieces.each_ref().map(|p| p.canonical_id.as_str()),
                ["a0", "a1", "a2"]
            );
            assert_eq!(value.pieces[0].initial_blocking.words[1], 2.0_f32.to_bits());
            assert_eq!(
                cat.native_wall_set_inputs("alias", pc(&row["precision"]))
                    .unwrap()
                    .canonical_id,
                "main"
            );
        }
    }
}

#[test]
fn registration_census_keeps_settlement_whose_ids_was_translated() {
    let mut text = source(-1, -1, false);
    text = text.replacen(
        "[WallComponent]",
        "[SettlementComponent]\n{\n}\n[WallComponent]",
        1,
    );
    text.push_str(
        "[Translate]\n{\nstring node = Thing\nstring ids_old = a0\nstring ids_new = a1\n}\n",
    );
    let cat = catalog(&text);
    assert_eq!(cat.find("a0").unwrap().id, "a1");
    let census = cat
        .native_settlement_registration_census(Precision::Double53)
        .unwrap();
    assert_eq!(census.actor_registrations, 3);
    assert_eq!(census.registered_actors.len(), 3);
    assert_eq!(census.settlement_actors.len(), 1);
    assert_eq!(census.settlement_actors[0].canonical_id, "a0");
    assert!(!census.settlement_actors[0].component_sources.is_empty());
    // This immutable input query uses the CURRENT registry. Native WallSet
    // references resolved before the later Translate must not be replaced
    // with these current results by a future completed-definition adapter.
    assert_eq!(
        cat.native_wall_set_inputs("main", Precision::Double53)
            .unwrap()
            .pieces[0]
            .canonical_id,
        "a1"
    );
}

#[test]
#[ignore = "requires original/v14 files via AA_GAME_ROOT"]
fn real_wall_sets_are_source_bound_without_completed_radius() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let (cat, load_report) = aa_rules::actors::native_load::load(&mut sources).unwrap();
    let entries = &cat.native_data_registry("WallSet").unwrap().entries;
    let mut bound = 0;
    let mut unsupported = Vec::new();
    for entry in entries {
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            match cat.native_wall_set_inputs(&entry.ids, pc) {
                Ok(value) => {
                    assert_eq!(value.source, entry.source);
                    bound += 1;
                }
                Err(error) => unsupported.push(format!("{} {pc:?}: {error}", entry.ids)),
            }
        }
    }
    let census = cat
        .native_settlement_registration_census(Precision::Double53)
        .unwrap();
    let exceptions: Vec<_> = load_report.exceptions().collect();
    assert!(
        exceptions.is_empty(),
        "source-load exceptions prevent absence census"
    );
    println!(
        "registered ActorData: {}; WallSets: {}; source bound across three PCs: {bound}; unsupported: {unsupported:?}; Settlement actors: {}; syntax diagnostics: {}; source-load exceptions: {}",
        census.actor_registrations,
        entries.len(),
        census.settlement_actors.len(),
        census.source_diagnostics.len(),
        exceptions.len()
    );
    if let Some(path) = std::env::var_os("AA_SETTLEMENT_CENSUS_OUT") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .unwrap();
        file.write_all(serde_json::to_string_pretty(&json!({"scope":"All1028 kind2 registrations by exact source provenance, with actual input source hashes. Native-order Rust parser projection, not original application startup execution. No full source-completion certificate.","census":census,"native_load_report":load_report})).unwrap().as_bytes()).unwrap();
    }
    // This census does not assert source completion or native game startup.
}
