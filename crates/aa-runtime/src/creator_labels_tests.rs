use super::*;
use aa_mapworld::header::KingdomParameter;
use aa_rules::{
    actors::ActorCatalog,
    resources::Provenance,
    tgi::{Limits, Span},
};

fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}
fn parameter(id: &str, key: &str) -> KingdomParameter {
    KingdomParameter {
        id: units(id),
        display_key: units(key),
        string_0c: None,
        byte_10: 0,
        nation: vec![],
        faction: vec![],
        color: vec![],
        handicap: vec![],
        string_2c: vec![],
        byte_29: 0,
        byte_28: 0,
        string_14: vec![],
        localized_entries: vec![],
        definitions: vec![],
        string_58: vec![],
        properties: vec![],
        byte_31: 0,
        byte_32: 0,
        resources: vec![],
    }
}
fn parameters() -> WorldParameters {
    WorldParameters {
        version: 13,
        teams: vec![],
        kingdoms: vec![
            parameter("K\0first", "MapKey"),
            parameter("K", "OtherKey"),
            parameter("k", "LowerKey"),
        ],
    }
}
fn table() -> Localization {
    let mut table = Localization::default();
    table.declarations.insert(
        "mapkey".into(),
        vec![crate::localization::Text::new(
            Provenance {
                path: "explicit laboratory registered text".into(),
                sha256: "not an actual-source receipt".into(),
                span: Span { start: 0, end: 0 },
            },
            Ok(units("old-global")),
        )],
    );
    table
}
fn state(parameters: &WorldParameters, table: &Localization) -> State {
    let actors = ActorCatalog::from_files(vec![], Limits::default());
    let mut catalog = Catalog::new(&actors);
    catalog.replace_current_text(table.clone());
    State::from_parameters(&catalog, parameters).unwrap()
}

#[test]
fn first_exact_nul_prefix_match_mutates_same_pair_table_and_glyph_and_rejects_clone() {
    let mut parameters = parameters();
    let mut table = table();
    let mut state = state(&parameters, &table);
    let handle = state
        .set_name_fields(
            &parameters,
            &units("K\0ignored"),
            &units("A\0B"),
            &mut table,
        )
        .unwrap();
    assert_eq!(handle.index, 0);
    assert_eq!(state.current_name(&handle).unwrap(), units("A\0B"));
    assert_eq!(
        table.lookup(&units("MapKey")).unwrap().unwrap(),
        units("A\0B")
    );
    assert!(state.kingdoms[1].text.is_none());
    state
        .set_glyph_fields(&mut parameters, &handle, &[0x03a9, 0, 0xd800])
        .unwrap();
    assert_eq!(
        parameters.kingdoms[0].string_0c,
        Some(vec![0x03a9, 0, 0xd800])
    );
    assert_eq!(parameters.kingdoms[1].string_0c, None);
    let copied = state.copy_for_creator();
    assert!(copied.current_name(&handle).is_err());
    assert!(
        copied
            .set_glyph_fields(&mut parameters, &handle, &[1])
            .is_err()
    );
}

#[test]
fn failed_table_lookup_preserves_real_first_pair_overwrite_and_unknown_source_is_not_empty() {
    let parameters = parameters();
    let mut table = Localization::default();
    let mut state = state(&parameters, &table);
    let handle = KingdomHandle {
        owner: Arc::clone(&state.owner),
        index: 0,
    };
    assert!(state.current_name(&handle).is_err());
    assert!(
        state
            .set_name_fields(&parameters, &units("K"), &units("new-player"), &mut table)
            .is_err()
    );
    assert_eq!(state.current_name(&handle).unwrap(), units("new-player"));
    let lower = state.set_name_fields(&parameters, &units("k"), &units("lower"), &mut table);
    assert!(lower.is_err());
    assert_eq!(
        state.kingdoms[2].text.as_deref(),
        Some(units("lower").as_slice())
    );
    assert!(state.kingdoms[1].text.is_none());
    assert!(
        state
            .set_name_fields(&parameters, &units("unknown"), &units("never"), &mut table)
            .is_err()
    );
    assert!(table.declarations.is_empty());
}

#[test]
fn original_and_v14_lookup_and_assignment_rows_replay_on_current_storage() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/development10/creator-labels/native-creator-labels.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 54);
    for row in rows {
        let input = &row["input"];
        let parameters = WorldParameters {
            version: 13,
            teams: vec![],
            kingdoms: input["ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| parameter(id.as_str().unwrap(), input["key"].as_str().unwrap()))
                .collect(),
        };
        let mut table = table();
        //Explicit laboratory memory from the native row, not a source-load
        //completion receipt. Only the reached current pair is overwritten.
        let mut state = state(&parameters, &table);
        for pair in &mut state.kingdoms {
            pair.text = Some(units("old-pair"));
            pair.unbound = None;
        }
        if input["fail_lookup"].as_bool().unwrap() {
            //A required lookup error occurs after the same first pair store.
            //The fixture supplies an external leaf cutoff; this Rust bounded
            //adapter reaches its real missing-entry refusal at that boundary.
            table.declarations.clear();
        }
        let result = state.set_name_fields(
            &parameters,
            &units(input["query"].as_str().unwrap()),
            &units(input["supplied"].as_str().unwrap()),
            &mut table,
        );
        let expected = row["output"]["found_index"].as_u64().map(|v| v as usize);
        if let Some(index) = expected {
            let handle = KingdomHandle {
                owner: Arc::clone(&state.owner),
                index,
            };
            let pair: Vec<u16> = row["output"]["pair"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect();
            assert_eq!(state.current_name(&handle).unwrap(), pair);
            if input["fail_lookup"].as_bool().unwrap() {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap().index, index);
                if !input["key"].as_str().unwrap().is_empty() {
                    let global: Vec<u16> = row["output"]["global_text"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap() as u16)
                        .collect();
                    assert_eq!(table.lookup(&units("MapKey")).unwrap().unwrap(), global);
                }
            }
        } else {
            assert!(result.is_err());
        }
    }
}

#[test]
fn key_only_native_load_values_are_bound_only_when_the_current_companion_is_proved() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/development10/creator-labels/native-key-only-pair.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 36);
    for row in fixture["rows"].as_array().unwrap() {
        let key = row["key"].as_str().unwrap();
        let mut table = table();
        table.declarations.clear();
        if let Some(value) = row["unique_text"].as_str() {
            let mut entry = self::table();
            let mut rows = entry.declarations.remove("mapkey").unwrap();
            rows[0].plain_utf16 = Ok(units(value));
            table.declarations.insert(key.to_ascii_lowercase(), rows);
        }
        let parameters = WorldParameters {
            version: 13,
            teams: vec![],
            kingdoms: vec![parameter("K", key)],
        };
        let state = state(&parameters, &table);
        if row["output"]["error"].is_null() {
            let expected: Vec<u16> = row["output"]["text"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect();
            assert_eq!(state.kingdom_pair(&parameters, 0).unwrap().text(), expected);
        } else {
            assert!(state.kingdom_pair(&parameters, 0).is_err());
            assert!(state.kingdoms[0].text.is_none());
        }
    }
    let mut fresh = WorldParameters {
        version: 13,
        teams: vec![],
        kingdoms: vec![parameter("", "")],
    };
    fresh.kingdoms[0].string_0c = Some(vec![]);
    let known = State::from_native_constructor(&fresh).unwrap();
    assert!(known.kingdom_pair(&fresh, 0).unwrap().text().is_empty());
    fresh.kingdoms[0].id = units("assigned-later");
    assert!(State::from_native_constructor(&fresh).is_err());
}
