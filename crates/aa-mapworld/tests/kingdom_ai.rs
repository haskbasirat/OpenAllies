use aa_mapworld::{actor_payload::CountedList, kingdom_ai};
use serde_json::Value;
#[test]
fn original_kingdom_ai_constructors_and_bounded_loads_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-kingdom-ai.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 25);
    for case in cases {
        if case["kind"] == "Strategic" {
            let mut state = kingdom_ai::StrategicState::new(case["initialize"].as_bool().unwrap());
            assert_eq!(
                serde_json::to_value(state.words.to_vec()).unwrap(),
                case["before"]
            );
            if let Some(version) = case["version"].as_u64() {
                let words = serde_json::from_value(case["source_words"].clone()).unwrap();
                state
                    .apply_saved(&aa_mapworld::kingdoms::StrategicState {
                        version: version as u32,
                        words,
                    })
                    .unwrap();
            }
            assert_eq!(
                serde_json::to_value(state.words.to_vec()).unwrap(),
                case["after"]
            );
        } else {
            let mut state = kingdom_ai::TacticalState::new().unwrap();
            if let Some(version) = case["version"].as_u64() {
                let count = 4 + usize::from(version > 1);
                let n = case["empty_count_word"].as_u64().unwrap() as u32;
                let source = aa_mapworld::kingdoms::TacticalState {
                    version: version as u32,
                    definition_groups: (0..count)
                        .map(|_| CountedList {
                            serialized_count: n,
                            values: vec![],
                        })
                        .collect(),
                    handle_groups: (0..if version > 0 { count } else { 0 })
                        .map(|_| CountedList {
                            serialized_count: n,
                            values: vec![],
                        })
                        .collect(),
                };
                state.apply_empty_saved(&source).unwrap();
            }
            let tables: Vec<_> = state
                .definitions
                .iter()
                .map(|t| {
                    (
                        t.buckets.len(),
                        t.count,
                        t.owns_entries,
                        t.buckets.iter().all(Option::is_none),
                    )
                })
                .chain(state.handles.iter().map(|t| {
                    (
                        t.buckets.len(),
                        t.count,
                        t.owns_entries,
                        t.buckets.iter().all(Option::is_none),
                    )
                }))
                .collect();
            for ((buckets, count, owns, zero), native) in tables
                .into_iter()
                .zip(case["after"]["tables"].as_array().unwrap())
            {
                assert_eq!(buckets, native["buckets"].as_u64().unwrap() as usize);
                assert_eq!(count, native["count"].as_u64().unwrap() as u32);
                assert_eq!(owns, native["owns"].as_u64().unwrap() != 0);
                assert_eq!(zero, native["zero_buckets"].as_bool().unwrap());
            }
            assert_eq!(
                serde_json::to_value(state.counters).unwrap(),
                case["after"]["counters"]
            );
        }
    }
}
