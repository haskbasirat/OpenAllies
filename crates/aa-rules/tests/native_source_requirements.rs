use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};
fn file(text: &str) -> ResolvedFile {
    let bytes = text.as_bytes().to_vec();
    ResolvedFile {
        logical_path: "requirements-fixture.tgi".into(),
        origin: Origin::V14Loose,
        sha256: sha256(&bytes),
        bytes,
    }
}
fn source(text: &str) -> ActorCatalog {
    load_files(
        vec![file(text)],
        Limits::default(),
        &mut NativeLoadReport::default(),
    )
}
fn actor(id: &str, category: &str, extra: &str) -> String {
    format!(
        "[Thing]\n{{ string IDS = {id}\n string category = {category}\n bool needs_space_for_settlement_zone = false\n{extra}\n}}\n"
    )
}
fn evidence(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-live-placement/source-requirements")
        .join(name)
}
#[test]
fn canonical_registered_owners_do_not_translate_again_and_keep_source_pc() {
    let text = format!(
        "{}{}{}[Translate]\n{{ string node = Thing\n string ids_old = old\n string ids_new = B\n}}\n[Translate]\n{{ string node = Thing\n string ids_old = alias\n string ids_new = old\n}}\n",
        actor(
            "old",
            "ACTOR",
            "[BodyComponent]\n{ bool captureable = true\n}"
        ),
        actor(
            "B",
            "RANDOM_ACTOR",
            "[BodyComponent]\n{ bool captureable = false\n}"
        ),
        actor("object", "OBJECT", "")
    );
    let catalog = source(&text);
    let arena = catalog
        .native_source_requirements(Precision::Double53)
        .unwrap();
    assert_eq!(arena.receipt().registered_actors, 2);
    assert_eq!(arena.receipt().registered_random_actors, 1);
    assert_eq!(arena.byte("OLD", 0x290).unwrap(), 1);
    assert_eq!(arena.byte("B", 0x290).unwrap(), 0);
    assert_eq!(arena.byte("object", 0x34).unwrap(), 0);
    assert!(arena.byte("object", 0x290).is_err());
    assert!(arena.word("object", 0x430).is_err());
    assert!(arena.reference("object", 0x4E0).is_err());
    assert!(arena.definition("alias").is_err());
    assert!(arena.word("old", 0x4E0).is_err());
    assert!(arena.byte("old", 0x1A9).is_err());
    assert_eq!(arena.word("old", 0x430).unwrap(), 0);
    assert_eq!(arena.word("old", 0x444).unwrap(), 0);
    assert_eq!(arena.byte("old", 0x1A8).unwrap(), 0);
    assert_eq!(arena.reference("old", 0x4E0).unwrap(), None);
    let token = arena.definition("old").unwrap().token();
    assert_eq!(arena.definition_by_token(token).unwrap().token(), token);
    assert_eq!(token.source(), &catalog.actors[0].source);
    let different_pc = catalog
        .native_source_requirements(Precision::Single24)
        .unwrap();
    assert!(different_pc.definition_by_token(token).is_err());
}
#[test]
fn inherited_or_random_settlement_and_required_reference_writers_are_refused() {
    for category in ["ACTOR", "RANDOM_ACTOR"] {
        let text = actor("owner", category, "[SettlementComponent]\n{}\n");
        assert!(
            source(&text)
                .native_source_requirements(Precision::Single24)
                .unwrap_err()
                .to_string()
                .contains("SettlementComponent")
        );
        let text = actor(
            "owner",
            category,
            "[ConstructionComponent]\n{ string required_object_ids = marker\n}\n",
        );
        assert!(
            source(&text)
                .native_source_requirements(Precision::Single24)
                .unwrap_err()
                .to_string()
                .contains("historical deferred")
        );
    }
    let inherited = concat!(
        "[template base]\n{ string category = ACTOR\n bool needs_space_for_settlement_zone = false\n [SettlementComponent]\n{}\n}\n",
        "[Thing Template=base]\n{ string IDS = inherited\n}\n"
    );
    assert!(
        source(inherited)
            .native_source_requirements(Precision::Double53)
            .unwrap_err()
            .to_string()
            .contains("SettlementComponent")
    );
    let upgrades = actor(
        "owner",
        "ACTOR",
        "[ConstructionComponent]\n{ string required_object_ids = NULL\n [Upgrade]\n{ string actor_ids = next\n}\n}\n",
    );
    assert!(
        source(&upgrades)
            .native_source_requirements(Precision::Double53)
            .unwrap_err()
            .to_string()
            .contains("nonempty Upgrade")
    );
    let body_missing = actor("owner", "ACTOR", "[BodyComponent]\n{}\n");
    assert!(
        source(&body_missing)
            .native_source_requirements(Precision::Single24)
            .unwrap_err()
            .to_string()
            .contains("Body.captureable missing")
    );
    let unordered = ActorCatalog::from_files(vec![], Limits::default());
    assert!(
        unordered
            .native_source_requirements(Precision::Single24)
            .is_err()
    );
}

#[test]
fn opaque_inputs_cannot_hide_canonical_boolean_reloads() {
    let mut omitted = file(&" ".repeat(1024));
    omitted.logical_path = "opaque-fixture.tgi".into();
    let mut catalog = load_files(
        vec![file(&actor("owner", "ACTOR", "")), omitted],
        Limits {
            max_bytes: 512,
            ..Limits::default()
        },
        &mut NativeLoadReport::default(),
    );
    // Diagnostics are public metadata. Even clearing their labels cannot turn
    // retained private parse omissions into a source-field ownership proof.
    catalog.diagnostics.clear();
    let error = catalog
        .native_source_requirements(Precision::Double53)
        .unwrap_err()
        .to_string();
    assert!(error.contains("opaque source"), "{error}");
}

#[test]
fn public_projection_cannot_hide_or_rename_a_private_registered_writer() {
    let text = format!(
        "{}{}",
        actor("ordinary", "ACTOR", ""),
        actor("writer", "RANDOM_ACTOR", "[SettlementComponent]\n{}\n")
    );
    for mutation in 0..6 {
        let mut catalog = source(&text);
        match mutation {
            0 => catalog.actors.clear(),
            1 => {
                catalog.actors.pop();
            }
            2 => {
                catalog.actors[1].id = "forged".into();
            }
            3 => {
                catalog.actors[1].template_chain.clear();
            }
            4 => {
                catalog.actors[1] = catalog.actors[0].clone();
            }
            _ => catalog.sources.clear(),
        }
        assert!(
            catalog
                .native_source_requirements(Precision::Double53)
                .is_err(),
            "mutation {mutation}"
        );
    }
    // Reordering this order-independent guarded slice changes only opaque
    // portable arena slots, not any native order claim or field value.
    let mut catalog = source(&format!(
        "{}{}",
        actor("A", "ACTOR", ""),
        actor("B", "ACTOR", "")
    ));
    catalog.actors.reverse();
    let arena = catalog
        .native_source_requirements(Precision::Double53)
        .unwrap();
    assert_eq!(arena.word("A", 0x430).unwrap(), 0);
    assert_eq!(arena.word("B", 0x444).unwrap(), 0);
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source assets"]
fn actual_registered_source_writer_receipts_and_values() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut source).unwrap();
    let mut captures = vec![];
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let arena = catalog.native_source_requirements(pc).unwrap();
        assert_eq!(arena.source_precision(), pc);
        assert!(arena.receipt().registered_actors >= 1028);
        assert!(arena.receipt().registered_random_actors > 0);
        assert!(arena.receipt().upgrade_census.explicit_instances.is_empty());
        let mut values = vec![];
        for entry in arena.definitions() {
            if !matches!(entry.native_kind(), 2 | 19) {
                continue;
            }
            let id = entry.token().canonical_id();
            assert_eq!(arena.word(id, 0x430).unwrap(), 0);
            assert_eq!(arena.word(id, 0x444).unwrap(), 0);
            assert_eq!(arena.byte(id, 0x1A8).unwrap(), 0);
            assert_eq!(arena.reference(id, 0x4E0).unwrap(), None);
            values.push(json!({"id":id,"native_kind":entry.native_kind(),"source":entry.token().source(),
                "own_settlement_space":arena.byte(id,0x34).unwrap(),"captureable":arena.byte(id,0x290).unwrap(),
                "owner430":arena.word(id,0x430).unwrap(),"head444":arena.word(id,0x444).unwrap(),
                "wall1A8":arena.byte(id,0x1A8).unwrap(),"required4E0":arena.reference(id,0x4E0).unwrap()}));
        }
        let native: Value = serde_json::from_slice(
            &std::fs::read(evidence("native-source-requirements.json")).unwrap(),
        )
        .unwrap();
        let pc_json = json!(pc);
        let native_rows: Vec<_> = native["actual_source_projection_cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["precision"] == pc_json)
            .collect();
        assert_eq!(native_rows.len(), 2 * arena.receipt().registered_actors);
        for row in native_rows {
            let id = row["id"].as_str().unwrap();
            // Under the proved absent-writer guards, actual native45DB10
            // reduces to the source-owned ObjectData byte+34. The oracle still
            // executes the full native short-circuit chain for false values.
            assert_eq!(
                row["output"]["result"],
                json!(arena.byte(id, 0x34).unwrap()),
                "{row}"
            );
        }
        captures.push(json!({"source_precision":pc,"receipt":arena.receipt(),"values":values}));
    }
    let actual = json!({"cases":captures});
    if let Some(path) = std::env::var_os("AA_SOURCE_REQUIREMENTS_CAPTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&actual).unwrap()).unwrap();
        return;
    }
    let expected: Value = serde_json::from_slice(
        &std::fs::read(evidence("actual-source-requirements.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(actual, expected);
    println!(
        "{} registered Things, {} ActorData, {} RandomActorData",
        captures[0]["receipt"]["registered_things"],
        captures[0]["receipt"]["registered_actors"],
        captures[0]["receipt"]["registered_random_actors"]
    );
}

#[test]
fn native_short_circuit_read_order_and_isolated_constructor_stores_are_pinned() {
    let native: Value = serde_json::from_slice(
        &std::fs::read(evidence("native-source-requirements.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(native["synthetic_cases"].as_array().unwrap().len(), 144);
    assert_eq!(
        native["isolated_constructor_stores"]
            .as_array()
            .unwrap()
            .len(),
        30
    );
    assert_eq!(native["guarded_prepasses"].as_array().unwrap().len(), 6);
    assert_eq!(
        native["all_kind_virtual_verifiers"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    for row in native["isolated_constructor_stores"].as_array().unwrap() {
        assert_eq!(row["result"], 0);
    }
    for row in native["guarded_prepasses"].as_array().unwrap() {
        assert_eq!(row["placement_fields_unchanged"], true);
    }
    for row in native["all_kind_virtual_verifiers"].as_array().unwrap() {
        assert_eq!(row["placement_fields_unchanged"], true);
    }
    let native_kingdom: Vec<_> = native["actual_source_projection_cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["id"] == "kingdom")
        .collect();
    assert_eq!(native_kingdom.len(), 6);
    for row in native_kingdom {
        assert_eq!(row["output"]["result"], 0);
        assert_eq!(
            row["output"]["reads"],
            json!([
                ["owner", 0x34, 1],
                ["owner", 0x174, 4],
                ["owner", 0x430, 4],
                ["owner", 0x4E0, 4]
            ])
        );
    }
}
