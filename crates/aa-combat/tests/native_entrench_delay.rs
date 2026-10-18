//! 0x45D520 against research/phase8-combat/native-entrench-delay-fixtures.json.
use aa_body::{F32, properties::ModifierState};
use aa_combat::organization::entrench_delay;
use aa_numeric::Precision;
use serde_json::Value;

#[test]
fn entrench_delay_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-entrench-delay-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 360);
    for case in cases {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let base = F32(case["base"].as_u64().unwrap() as u32);
        let modifier = case["scalar_40"].as_u64().map(|bits| {
            let mut state = ModifierState::new(1);
            state.scalars[0x40 / 4 - 1] = F32(bits as u32);
            state
        });
        let value = entrench_delay(base, modifier.as_ref(), p).unwrap();
        let expected = (u128::from(case["sign_exponent"].as_u64().unwrap() as u16) << 64)
            | u128::from(u64::from_str_radix(case["mantissa"].as_str().unwrap(), 16).unwrap());
        assert_eq!(value.to_x87_extended_bits(), Some(expected), "{case}");
    }
}
