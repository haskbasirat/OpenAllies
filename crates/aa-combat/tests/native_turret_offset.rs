//! 0x4C76A0 against research/phase8-combat/native-turret-offset-fixtures.json
//! (every case at PC24, PC53 and PC64, both output words bit for bit).
use aa_body::F32;
use aa_combat::turret_geometry::turned_offset;
use aa_numeric::Precision;
use serde_json::Value;

fn word(v: &Value) -> F32 {
    F32(v.as_u64().unwrap() as u32)
}

#[test]
fn turned_offset_matches_native_at_every_precision() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-turret-offset-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 1000);
    let mut checked = 0;
    for (index, case) in cases.iter().enumerate() {
        let offset = [word(&case["offset"][0]), word(&case["offset"][1])];
        let facing = [word(&case["facing"][0]), word(&case["facing"][1])];
        for (name, precision) in [
            ("Single24", Precision::Single24),
            ("Double53", Precision::Double53),
            ("Extended64", Precision::Extended64),
        ] {
            let expected = &case["result"][name];
            let got = turned_offset(offset, facing, precision).unwrap();
            assert_eq!(
                [got[0].0, got[1].0],
                [
                    expected[0].as_u64().unwrap() as u32,
                    expected[1].as_u64().unwrap() as u32
                ],
                "case {index} at {name}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, cases.len() * 3);
}
