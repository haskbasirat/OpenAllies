//! Restoration's adapters must preserve the identities and fields from the
//! original 4C3610/4C34A0 fixtures, not just accept a layout's IDS.
use aa_mapworld::restore::RestoreDefinitions;
use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use aa_runtime::catalog::Catalog;
use serde_json::{Value, json};
use std::sync::Arc;

fn oracle() -> Value {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-layouts/native-layout-data.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn fixture_catalog(doc: &Value, case: &Value) -> ActorCatalog {
    let files = doc["templates"]
        .as_array()
        .unwrap()
        .iter()
        .chain(case["files"].as_array().unwrap())
        .map(|file| {
            let bytes: Vec<u8> = file["text"]
                .as_str()
                .unwrap()
                .chars()
                .map(|c| u8::try_from(u32::from(c)).unwrap())
                .collect();
            ResolvedFile {
                logical_path: file["path"].as_str().unwrap().into(),
                origin: Origin::V14Loose,
                sha256: sha256(&bytes),
                bytes,
            }
        })
        .collect();
    load_files(files, Limits::default(), &mut NativeLoadReport::default())
}

fn case<'a>(doc: &'a Value, name: &str) -> &'a Value {
    doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap()
}

/// The portable handle convention replaces native addresses, preserving
/// registration and embedded-property allocation order from the oracle.
fn property_handle(native: &Value, property: &Value) -> u32 {
    if property.is_null() {
        return 0;
    }
    if let Some(ordinal) = property["registered"].as_u64() {
        return ordinal as u32 + 1;
    }
    let index = native["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| {
            event["event"] == "property_load" && event["property"]["registered"].is_null()
        })
        .position(|event| &event["property"] == property)
        .expect("embedded property allocated by the original loader");
    0x8000_0000 | (index as u32 + 1)
}

#[test]
fn restore_layout_adapter_replays_original_fields_and_handle_relationships() {
    let doc = oracle();
    for name in [
        "basic",
        "two_layouts_one_file",
        "default_combat_retreat",
        "retreat_last_organized",
        "retreat_none_organized",
        "morale_lines",
        "many_slots_and_positions",
        "case_insensitive_refs",
        "formation_flags",
    ] {
        let case = case(&doc, name);
        let actors = fixture_catalog(&doc, case);
        let catalog = Catalog::new(&actors);
        let pc = Precision::Double53;
        let native = &case["result"];
        let mut next_formation = 1;
        for layout in native["layouts"].as_array().unwrap() {
            let ids = layout["ids"].as_str().unwrap();
            let found = catalog
                .organization_layout_data(
                    &ids.to_ascii_uppercase().encode_utf16().collect::<Vec<_>>(),
                    pc,
                )
                .unwrap()
                .expect("native registry member must resolve");
            assert_eq!(found, ids, "{name}: canonical IDS");
            let info = catalog.organization_layout_info(&found, pc).unwrap();
            assert_eq!(info.name, ids, "{name}: name is LayoutData+8 IDS");
            assert_eq!(
                i64::from(info.primary_slot),
                layout["primary_slot"].as_i64().unwrap(),
                "{name}"
            );
            assert_eq!(
                info.slot_groups,
                layout["slot_groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect::<Vec<_>>(),
                "{name}"
            );
            assert_eq!(
                i64::from(info.formation_44),
                layout["combat_formation"].as_i64().unwrap(),
                "{name}"
            );
            assert_eq!(
                i64::from(info.formation_48),
                layout["retreat_formation"].as_i64().unwrap(),
                "{name}"
            );
            assert_eq!(
                info.property_50,
                property_handle(native, &layout["broken_property"]),
                "{name}"
            );
            let entries = layout["entries"].as_array().unwrap();
            assert_eq!(info.entries.len(), entries.len(), "{name}");
            for (actual, expected) in info.entries.iter().zip(entries) {
                assert_eq!(
                    actual.property,
                    property_handle(native, &expected["property"]),
                    "{name}"
                );
                let index = expected["property"]["registered"].as_u64().unwrap() as usize;
                assert_eq!(
                    actual.property_name,
                    native["properties"][index]["ids"].as_str().unwrap(),
                    "{name}"
                );
                assert_eq!(
                    actual.required,
                    expected["required"].as_u64().unwrap() != 0,
                    "{name}"
                );
                assert_eq!(
                    actual.locked,
                    expected["locked"].as_u64().unwrap() != 0,
                    "{name}"
                );
                assert_eq!(
                    actual.morale_line,
                    expected["morale_line"].as_u64().unwrap() != 0,
                    "{name}"
                );
            }
            let formations = layout["formations"].as_array().unwrap();
            assert_eq!(info.formations.len(), formations.len(), "{name}");
            for (actual, expected) in info.formations.iter().zip(formations) {
                assert_eq!(*actual, next_formation, "{name}: creation-order handle");
                next_formation += 1;
                let definition =
                    &native["formations"][expected["formation"].as_u64().unwrap() as usize];
                assert_eq!(
                    catalog.organization_formation_facts(*actual, pc).unwrap(),
                    (
                        definition["aggressive"].as_u64().unwrap() as u8,
                        property_handle(native, &definition["property"]),
                    ),
                    "{name}: formation handle must reach its original definition"
                );
            }
        }
        let cached = catalog.layout_data(pc).unwrap();
        assert!(Arc::ptr_eq(&cached, &catalog.layout_data(pc).unwrap()));
        // Precision is part of the source-load identity, even where the
        // shipped layouts happen to produce equal values at every PC.
        assert!(!Arc::ptr_eq(
            &cached,
            &catalog.layout_data(Precision::Single24).unwrap()
        ));
        assert!(catalog.organization_layout_data(&[], pc).unwrap().is_none());
        assert!(
            catalog
                .organization_layout_data(&[0xd800], pc)
                .unwrap()
                .is_none()
        );
        assert!(
            catalog
                .organization_layout_data(&"absent".encode_utf16().collect::<Vec<_>>(), pc)
                .unwrap()
                .is_none()
        );
        assert!(catalog.organization_formation_facts(0, pc).is_err());
        assert!(catalog.organization_formation_facts(u32::MAX, pc).is_err());
    }
}

#[test]
fn native_partial_layout_registration_is_not_an_usable_layout() {
    let doc = oracle();
    for name in [
        "unknown_slot_property",
        "no_formation",
        "no_broken_property_template",
    ] {
        let actors = fixture_catalog(&doc, case(&doc, name));
        let catalog = Catalog::new(&actors);
        let pc = Precision::Double53;
        let ids = catalog
            .organization_layout_data(&"L1".encode_utf16().collect::<Vec<_>>(), pc)
            .unwrap()
            .expect("native registers before running LoadData");
        let error = catalog.organization_layout_info(&ids, pc).unwrap_err();
        assert!(
            error.contains("partially loaded natively"),
            "{name}: {error}"
        );
    }
}

#[test]
fn canonical_layout_identity_is_not_translated_twice() {
    // Native 48D500 is one-hop: phase8-definitions/
    // native-definition-registry.json lookup cases prove old1 -> alpha,
    // alpha -> beta returns alpha for old1. A resolved pointer must not
    // be passed through Find again merely to read its LayoutData fields.
    let mut doc = oracle();
    let mut case = case(&doc, "basic").clone();
    doc["templates"].as_array_mut().unwrap().push(json!({
        "path": "Templates/template_translate.tgi",
        "text": "[template Translate]\n{\nstring node\nstring ids_old\nstring ids_new\n}\n",
    }));
    let files = case["files"].as_array_mut().unwrap();
    let layout = files.last().unwrap()["text"].as_str().unwrap().to_owned();
    for name in ["L2", "L3"] {
        files.push(json!({"path": format!("Fixture/Layouts/{name}.tgi"), "text": layout.replace("L1", name)}));
    }
    files.push(json!({"path": "Fixture/translations.tgi", "text": concat!(
        "[Translate template=Translate]\n{\nnode = OrganizationLayout\nids_old = L1\nids_new = L2\n}\n",
        "[Translate template=Translate]\n{\nnode = OrganizationLayout\nids_old = L2\nids_new = L3\n}\n",
        "[Thing]\n{\nIDS = alpha\ncategory = ACTOR\n}\n",
        "[Thing]\n{\nIDS = beta\ncategory = ACTOR\n}\n",
        "[Translate template=Translate]\n{\nnode = Thing\nids_old = old1\nids_new = alpha\n}\n",
        "[Translate template=Translate]\n{\nnode = Thing\nids_old = alpha\nids_new = beta\n}\n",
    )}));
    let actors = fixture_catalog(&doc, &case);
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    let first = catalog
        .organization_layout_data(&"L1".encode_utf16().collect::<Vec<_>>(), pc)
        .unwrap()
        .unwrap();
    assert_eq!(first, "L2");
    assert_eq!(
        catalog.organization_layout_info(&first, pc).unwrap().name,
        "L2"
    );
    assert_eq!(
        catalog
            .organization_layout_data(&"L2".encode_utf16().collect::<Vec<_>>(), pc)
            .unwrap()
            .unwrap(),
        "L3"
    );
    assert_eq!(actors.find("old1").unwrap().id, "alpha");
    assert_eq!(actors.find("alpha").unwrap().id, "beta");
    // The current ActorData field resolver is lookup-based. Refuse that
    // unsupported pointer identity rather than testing beta's properties.
    let error = catalog
        .organization_has_property("alpha", 1, pc)
        .unwrap_err();
    assert!(
        error.contains("canonical ActorData alpha is shadowed"),
        "{error}"
    );
}

#[test]
fn organization_modifier_floor_comes_from_loaded_settings_without_a_default() {
    let doc = oracle();
    let actors = fixture_catalog(&doc, case(&doc, "basic"));
    let mut catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    assert!(catalog.organization_modifier_minimum(pc).is_err());
    let bytes = b"[SVars]\n{\nfixed ModifierAttributeClampAdd = 0.375\n}\n".to_vec();
    catalog.settings = Some(
        aa_runtime::settings::Settings::from_files(
            [ResolvedFile {
                logical_path: "fixture-settings.tgi".into(),
                origin: Origin::V14Loose,
                sha256: sha256(&bytes),
                bytes,
            }],
            Limits::default(),
        )
        .unwrap(),
    );
    assert_eq!(
        catalog.organization_modifier_minimum(pc).unwrap(),
        0.375f32.to_bits()
    );
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source assets"]
fn every_shipped_organization_uses_one_restoration_layout_and_property_arena() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&actors);
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let data = catalog.layout_data(pc).unwrap();
        let mut count = 0;
        for actor in &actors.actors {
            let Some(component) = data.organization_component(&actors, &actor.id, pc).unwrap()
            else {
                continue;
            };
            count += 1;
            let info = catalog
                .organization_layout_info(&component.layout_ids, pc)
                .unwrap();
            assert_eq!(
                info.entries.len(),
                component.parameters.len(),
                "{}",
                actor.id
            );
            for (slot, parameter) in component.parameters.iter().enumerate() {
                if let Some(parameter) = parameter {
                    assert!(
                        catalog
                            .organization_has_property(parameter, info.entries[slot].property, pc)
                            .unwrap(),
                        "{} slot {slot}",
                        actor.id
                    );
                }
            }
            for formation in info.formations {
                catalog.organization_formation_facts(formation, pc).unwrap();
            }
        }
        assert_eq!(count, 192);
        assert!(
            catalog
                .organization_has_property("usa_infantry", 0, pc)
                .is_err()
        );
        assert!(
            !catalog
                .organization_has_property("usa_infantry", 0x8000_0001, pc)
                .unwrap()
        );
    }
}
