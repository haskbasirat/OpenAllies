//! Replays research/phase8-combat/native-ability-state-fixtures.json:
//! original 45D430, 4C85D0, 4C8100, 4C8120 (signed zeros, denormals,
//! infinities, NaNs) and 45D4F0 with the real 501CF0, all precisions.
use aa_body::{F32, properties::ModifierState};
use aa_combat::ability_state::*;
use aa_numeric::{Binary, Precision};
use serde_json::Value;

fn precision(v: &Value) -> Precision {
    match v.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("{other}"),
    }
}

fn bits(v: &Value) -> F32 {
    F32(v.as_u64().unwrap() as u32)
}

#[test]
fn original_ability_state_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-ability-state-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 750);
    for (i, c) in cases.iter().enumerate() {
        let r = &c["results"];
        let flags = c["flags54"].as_u64().unwrap() as u32;
        assert_eq!(field38_pending(bits(&c["f38"])) as u64, r["45D430"].as_u64().unwrap(), "45D430 {i} {c}");
        assert_eq!(busy(bits(&c["f38"]), bits(&c["f4c"]), flags) as u64, r["4C85D0"].as_u64().unwrap(), "4C85D0 {i} {c}");
        assert_eq!(field4c_positive(bits(&c["f4c"])) as u64, r["4C8100"].as_u64().unwrap(), "4C8100 {i} {c}");
        assert_eq!(
            field10_below_14(bits(&c["f10"]), bits(&c["f14"])) as u64,
            r["4C8120"].as_u64().unwrap(),
            "4C8120 {i} {c}"
        );
    }
    let getters = doc["getters"].as_array().unwrap();
    assert_eq!(getters.len(), 450);
    for (i, c) in getters.iter().enumerate() {
        let state = c["delta24"].as_u64().map(|d| {
            let mut s = ModifierState::new(0);
            s.scalars[0x24 / 4 - 1] = F32(d as u32);
            s
        });
        let value = modified_308(bits(&c["base"]), state.as_ref(), bits(&c["setting"]), precision(&c["precision"])).unwrap();
        let expected = (u128::from(c["sign_exponent"].as_u64().unwrap() as u16) << 64)
            | u128::from(u64::from_str_radix(c["mantissa"].as_str().unwrap(), 16).unwrap());
        assert_eq!(
            Binary::from_f32_bits(value.0).unwrap().to_x87_extended_bits(),
            Some(expected),
            "45D4F0 {i} {c}"
        );
    }
}
