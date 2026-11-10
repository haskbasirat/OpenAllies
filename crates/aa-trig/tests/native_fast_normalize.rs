//! 0x8693B0 and 0x43A970 against
//! research/phase8-combat/native-fast-normalize-fixtures.json: the table
//! built at every precision and every vector at PC24, PC53 and PC64.
use aa_numeric::Precision;
use aa_trig::fast_normalize::{fast_normalize, sqrt_table};
use serde_json::Value;

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

const PRECISIONS: [(&str, Precision); 3] = [
    ("Single24", Precision::Single24),
    ("Double53", Precision::Double53),
    ("Extended64", Precision::Extended64),
];

#[test]
fn table_and_normalize_match_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-fast-normalize-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let table = sqrt_table().unwrap();
    for (name, _) in PRECISIONS {
        let native: Vec<u32> = doc["tables"][name].as_array().unwrap().iter().map(word).collect();
        assert_eq!(table.to_vec(), native, "table built at {name}");
    }
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 1500);
    for (index, case) in cases.iter().enumerate() {
        let v = [0, 1, 2].map(|k| word(&case["vector"][k]));
        for (name, precision) in PRECISIONS {
            let expected = [0, 1, 2].map(|k| word(&case["result"][name][k]));
            assert_eq!(fast_normalize(v, &table, precision).unwrap(), expected, "case {index} at {name}");
        }
    }
}
