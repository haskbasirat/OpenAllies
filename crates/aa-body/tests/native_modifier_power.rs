//! 501A30/501AA0 modifier power getters against the Unicorn fixtures of
//! research/phase8-combat/native_combat_value.py (all x87 precisions).
use aa_body::{F32, properties::ModifierState};
use aa_numeric::Precision;
use serde_json::Value;

#[test]
fn modifier_power_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-combat-value-fixtures.json"
    ))
    .unwrap();
    let cases = doc["power_cases"].as_array().unwrap();
    assert!(cases.len() > 2000);
    let u = |v: &Value| v.as_u64().unwrap() as u32;
    for case in cases {
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let ranged = case["ranged"].as_bool().unwrap();
        let mut state = ModifierState::new(1);
        state.scalars[0] = F32(u(&case["s4"]));
        state.scalars[1] = F32(if ranged {
            0x4118_0000
        } else {
            u(&case["kind"])
        });
        state.scalars[2] = F32(if ranged {
            u(&case["kind"])
        } else {
            0x4118_0000
        });
        let got = state
            .power(ranged, F32(u(&case["base"])), F32(u(&case["clamp"])), p)
            .unwrap();
        assert_eq!(got, F32(u(&case["result"])), "{case}");
    }
}
