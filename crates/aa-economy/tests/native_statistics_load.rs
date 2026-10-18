//! Replays research/phase8-economy/native_statistics_load.py: original
//! statistics load 0x58EE80, relation statistics load 0x58DBE0, promotion
//! 0x4E3690 and mission score reset 0x58EA00.
use aa_economy::statistics::*;
use aa_economy::{F32Bits, X87Precision};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn words(v: &Value) -> Vec<F32Bits> {
    serde_json::from_value(v.clone()).unwrap()
}

fn groups(v: &Value, kingdom: bool) -> Vec<Vec<SavedRelationEntry>> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|g| {
            g.as_array()
                .unwrap()
                .iter()
                .map(|e| {
                    let raw = e[0].as_u64().unwrap() as u32;
                    let key = if kingdom {
                        (raw >> 16 == 1).then_some(raw & 0xFFFF)
                    } else {
                        (raw != 0).then_some(raw)
                    };
                    SavedRelationEntry {
                        key,
                        value: F32Bits(e[1].as_u64().unwrap() as u32),
                    }
                })
                .collect()
        })
        .collect()
}

#[test]
fn statistics_load_matches_original_x86() {
    let document: Value = serde_json::from_str(include_str!(
        "fixtures/native-statistics-load-fixtures.json"
    ))
    .unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let expected = &fixture["expected"];
        let _: X87Precision = serde_json::from_value(fixture["precision"].clone()).unwrap();
        let label = format!("{} {}", fixture["name"], fixture["precision"]);

        let prior = words(&case["prior"]);
        let mut statistics = Statistics::new(3);
        statistics.production.copy_from_slice(&prior[..3]);
        statistics.scores.copy_from_slice(&prior[10..66]);
        let tail = statistics
            .load_saved(
                case["version"].as_u64().unwrap() as u32,
                &words(&case["words"]),
            )
            .expect(&label);
        let native_production = words(&expected["statistics"]["production"]);
        assert_eq!(statistics.production[..], native_production[..3], "{label}");
        assert_eq!(tail[..], native_production[3..], "{label}");
        assert_eq!(
            statistics.scores,
            words(&expected["statistics"]["scores"]),
            "{label}"
        );

        let version = case["relation_version"].as_u64().unwrap() as u32;
        let families = case["groups"].as_array().unwrap();
        let mut relations = RelationStatistics::default();
        relations
            .load_saved(
                version,
                &groups(&families[0], false),
                &groups(&families[1], true),
            )
            .expect(&label);
        for (entry, native) in relations
            .entries
            .iter()
            .zip(expected["relations"].as_array().unwrap())
        {
            let values: BTreeMap<String, u32> = entry
                .values
                .iter()
                .map(|(k, v)| (k.to_string(), v.0))
                .collect();
            let native_values: BTreeMap<String, u32> =
                serde_json::from_value(native["values"].clone()).unwrap();
            assert_eq!(values, native_values, "{label}");
            assert_eq!(json!(entry.maximum.0), native["maximum"], "{label}");
            assert_eq!(json!(entry.leader), native["leader"], "{label}");
        }

        let p = &case["promote"];
        let scores = |v: &Value| {
            (!v.is_null()).then(|| {
                let mut s = Statistics::new(3);
                s.scores = words(v);
                s
            })
        };
        let mut kingdom = KingdomStatistics {
            active: scores(&p["active"]),
            loaded: scores(&p["loaded"]),
            relations: (!p["relations"].is_null()).then(RelationStatistics::default),
            loaded_relations: (!p["loaded_relations"].is_null()).then(RelationStatistics::default),
        };
        let had_loaded = kingdom.loaded.is_some();
        kingdom.start_world(3);
        let e = &expected["promote"];
        assert_eq!(e["active"] == "loaded", had_loaded, "{label}");
        assert_eq!(
            kingdom.active.as_ref().unwrap().scores,
            words(&e["active_scores"]),
            "{label}"
        );
        assert!(kingdom.loaded.is_none() && e["loaded"] == 0, "{label}");
        assert!(kingdom.relations.is_some() && kingdom.loaded_relations.is_none());
        assert_eq!(e["loaded_relations"], 0, "{label}");
        executions += 4;
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
