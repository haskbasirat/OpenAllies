//! Replays research/phase8-ai/native-goal-qsort.json (original CRT qsort
//! 96CE20/96CDB0 with comparator 7A30B0) against aa_mind::sai::crt_qsort.
use aa_mind::sai::{crt_qsort::qsort, engine::base_priority_order};
use serde_json::{Value, json};

#[test]
fn native_qsort_with_goal_comparator_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-goal-qsort.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 52);
    for c in cases {
        let keys: Vec<u32> = c["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_u64().unwrap() as u32)
            .collect();
        let mut order: Vec<usize> = (0..keys.len()).collect();
        let mut compares = vec![];
        qsort(&mut order, |a, b| {
            let r = base_priority_order(keys[*a], keys[*b]);
            compares.push(json!([a, b, r]));
            Ok(r)
        })
        .unwrap();
        assert_eq!(json!(order), c["order"], "{}", c["name"]);
        assert_eq!(json!(compares), c["compares"], "{}", c["name"]);
    }
}
