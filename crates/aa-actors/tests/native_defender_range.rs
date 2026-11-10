//! 0x566F80 / 0x45D6C0 / 0x45D4C0 / 0x501520 against
//! research/phase8-combat/native-defender-range-fixtures.json: the 80-bit
//! getter results and the stored Defender +2C word at PC24, PC53 and PC64.
use aa_actors::defender_range::{RangeDefinition, modified_range, refreshed_range};
use aa_body::F32;
use aa_numeric::Precision;
use serde_json::Value;

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn extended(v: &Value) -> u128 {
    (u128::from(v["sign_exponent"].as_u64().unwrap() as u16) << 64) | u128::from(v["mantissa"].as_u64().unwrap())
}

#[test]
fn defender_range_matches_native_at_every_precision() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-defender-range-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 800);
    for (index, case) in cases.iter().enumerate() {
        let definition = RangeDefinition {
            mask: word(&case["mask"]),
            defender_range: F32(word(&case["range_218"])),
            organization_range: F32(word(&case["range_2fc"])),
        };
        let modifier = case["modifier"].as_u64().map(|w| F32(w as u32));
        for (name, precision) in [
            ("Single24", Precision::Single24),
            ("Double53", Precision::Double53),
            ("Extended64", Precision::Extended64),
        ] {
            let native = &case["result"][name];
            let stored = refreshed_range(&definition, modifier, precision).unwrap();
            assert_eq!(stored.0, word(&native["stored"]), "case {index} at {name}: stored");
            assert_eq!(word(&native["flag"]), 1);
            let defender = modified_range(definition.defender_range, modifier, precision).unwrap();
            assert_eq!(
                defender.to_x87_extended_bits(),
                extended(&native["getter_45d6c0"]),
                "case {index} at {name}: 45D6C0"
            );
            let organization = modified_range(definition.organization_range, modifier, precision).unwrap();
            assert_eq!(
                organization.to_x87_extended_bits(),
                extended(&native["getter_45d4c0"]),
                "case {index} at {name}: 45D4C0"
            );
        }
    }
}
