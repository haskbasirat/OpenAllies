use super::*;
use serde_json::{Value, json};
fn synthetic_provenance() -> aa_rules::resources::Provenance {
    aa_rules::resources::Provenance {
        path: "laboratory".into(),
        sha256: "not-a-source-receipt".into(),
        span: aa_rules::tgi::Span { start: 0, end: 0 },
    }
}

#[test]
fn world_constructor_owns_camera_and_start_once_fields_all_pcs() {
    let fixture: Value = serde_json::from_slice(include_bytes!(
        "../../../research/development10/map-resource-mode/native-tail-constructor.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 18);
    for row in rows {
        let state = TailState::from_world_constructor();
        assert_eq!(
            json!(u32::from(state.started_274)),
            row["native"]["started_274"]
        );
        assert_eq!(
            json!(state.camera.look_at.unwrap()),
            row["native"]["look_at"]
        );
        assert!(state.camera.messages.is_empty());
        assert_eq!(row["native"]["global_same"], true);
        assert_eq!(row["native"]["settings_same"], true);
    }
}

#[test]
fn gmap_camera_extents_use_float_words_instead_of_grid_counts_all_pcs() {
    let fixture: Value = serde_json::from_slice(include_bytes!(
        "../../../research/development10/map-resource-mode/native-gmap-extent.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 30);
    for row in rows {
        let input = &row["input_extents"];
        let extents = [
            input[0].as_i64().unwrap() as i32,
            input[1].as_i64().unwrap() as i32,
        ];
        let actual = crate::load_tail_binding::gmap_extent_words(extents).unwrap();
        assert_eq!(json!(actual), row["native"]["map_extent_bits"]);
        assert_eq!(
            json!([extents[0] >> 2, extents[1] >> 2]),
            row["native"]["grid_cells"]
        );
    }
}

#[test]
fn source_priority_required_integer_and_native_negative_gate_all_pcs() {
    let fixture: Value = serde_json::from_slice(include_bytes!(
        "../../../research/development10/map-resource-mode/native-starting-priority.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 42);
    for row in rows {
        let input = &row["input"];
        let n = &row["native"];
        let source = synthetic_provenance();
        let value = TypedValue::Integer {
            value: input["value"].as_u64().unwrap() as u32 as i32,
            source: source.clone(),
            declaration_source: source,
        };
        let present = input["present"].as_bool().unwrap();
        let result = checked_priority(present.then_some(&value), "actor_fixture");
        assert_eq!(n["previous_bool"], 0xa7);
        match n["outcome"].as_str().unwrap() {
            "LoadedPriorityPrefix" => assert_eq!(json!(result.unwrap() as u32), n["priority_word"]),
            "MissingFieldExceptionBranch" => {
                assert!(result.unwrap_err().contains("has no field"));
                assert_eq!(n["priority_word"], 0x12345678);
            }
            "NegativePriorityFatal" => {
                assert!(result.unwrap_err().contains("needs to be positive"));
                assert_eq!(n["priority_word"], input["value"]);
            }
            v => panic!("unexpected {v}"),
        }
    }
    let source = synthetic_provenance();
    let wrong = TypedValue::Boolean {
        value: false,
        source: source.clone(),
        declaration_source: source,
    };
    assert!(
        checked_priority(Some(&wrong), "actor_fixture")
            .unwrap_err()
            .contains("non-integer")
    );
}

#[test]
#[ignore = "requires AA_GAME_ROOT source and original maps"]
fn actual_maps_bind_camera_priority_with_canonical_owner_and_source_pc() {
    use crate::{
        combat_source_completion, combat_source_receipt::VerifiedStartupReceipt, settings::Settings,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(Settings::load_v14(&mut files, aa_rules::tgi::Limits::default()).unwrap());
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    combat_source_completion::compute(&catalog).unwrap();
    let mut reports = vec![];
    let mut last = None;
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
        let inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
        let mut ids = std::collections::BTreeSet::new();
        let mut values = vec![];
        for placement in &inventory.dynamic_things.placements {
            if !matches!(placement.class_code, 3 | 4) {
                continue;
            }
            let id = String::from_utf16(&placement.thing_id_utf16).unwrap();
            let source = aa_mapworld::bootstrap::resolve_definition(
                &actors,
                &placement.thing_id_utf16,
                receipt.context().precision(),
            )
            .unwrap();
            if !ids.insert(source.definition.id.clone()) {
                continue;
            }
            // Narrow source projection. This does not certify this record's
            // component construction, restoration, PostLoad or live readiness.
            let definition = RuntimeDefinition {
                proof: source.definition,
                native_class: source.native_class,
                flags: source.flag_word,
                object: None,
                component_mask: None,
                ground: Default::default(),
            };
            let priority = starting_location_priority(&catalog, &definition).unwrap();
            assert!(priority.value() >= 0);
            assert_eq!(priority.source_precision(), receipt.context().precision());
            assert_eq!(
                catalog
                    .definition_bank
                    .borrow()
                    .token("ActorDefinition", &definition.proof.id),
                Some(priority.token())
            );
            let mut forged = definition.clone();
            forged.proof.source.sha256.push('0');
            assert!(starting_location_priority(&catalog, &forged).is_err());
            values.push(json!({"map_id":id,"canonical":definition.proof.id,"token":priority.token(),"value":priority.value(),"input":priority.input()}));
            last = Some(definition);
        }
        assert!(!values.is_empty(), "{name}");
        reports.push(json!({"map":name,"map_source":map.source(),"definitions":values}));
    }
    // Attempt consumes Catalog and moves this SAME Bank into Staged. A source
    // Catalog constructed for later immutable lookups cannot create a second
    // cache; the additive entry takes the actual moved owner explicitly.
    let last = last.unwrap();
    let expected = starting_location_priority(&catalog, &last).unwrap();
    let address =
        std::ptr::from_ref(catalog.definition_bank.borrow().combat_source().unwrap()).addr();
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    assert_eq!(
        std::ptr::from_ref(bank.combat_source().unwrap()).addr(),
        address
    );
    assert!(starting_location_priority(&catalog, &last).is_err());
    let moved = starting_location_priority_in_bank(&catalog, &last, &bank).unwrap();
    assert_eq!(moved.token(), expected.token());
    assert_eq!(moved.value(), expected.value());
    assert_eq!(moved.input(), expected.input());
    if let Some(path) = std::env::var_os("AA_TAIL_PRIORITY_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
    }
}
