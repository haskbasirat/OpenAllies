//! Replays research/phase8-economy/native_kingdom_load.py: the original
//! GKingdom::Load v7+ property-record loop (0x4E8A09..0x4E8C2F, 0x503080).
use aa_economy::kingdom_load::*;
use serde_json::{Value, json};

#[test]
fn loaded_property_records_match_original_x86() {
    let document: Value =
        serde_json::from_str(include_str!("fixtures/native-kingdom-load-fixtures.json")).unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let sources: Vec<Option<u32>> = case["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["source"].as_u64().map(|s| s as u32))
            .collect();
        let outcome =
            loaded_property_records(case["world_version"].as_u64().unwrap() as u32, &sources);
        let mut kept = vec![];
        let mut events = vec![];
        for (index, disposition) in outcome.iter().enumerate() {
            match disposition {
                LoadedPropertyRecord::Kept => kept.push(index),
                LoadedPropertyRecord::IgnoredSource => events.push(json!({"kind": "ignored_log"})),
                LoadedPropertyRecord::LegacyDiscarded { logged } => {
                    if *logged {
                        events.push(json!({"kind": "legacy_log"}));
                    }
                    events.push(json!({"kind": "freed", "record": index}));
                }
            }
        }
        let actual = json!({"kept": kept, "events": events});
        assert_eq!(actual, fixture["expected"], "{}", fixture["name"]);
        executions += 1;
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
