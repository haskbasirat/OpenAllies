use aa_rules::targets::TargetFilter;
use serde_json::Value;
#[test]
fn original_target_tokenizer_prefixes_wrapping_and_ability_defaults() {
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-ability-derived.json"
    ))
    .unwrap();
    for row in d["target_cases"].as_array().unwrap() {
        let a: Vec<u32> = serde_json::from_value(row["initial"].clone()).unwrap();
        let value = TargetFilter::parse(
            row["raw"].as_str().unwrap(),
            TargetFilter {
                required: a[0],
                forbidden: a[1],
            },
            |name| Ok(row["registry"][name].as_u64().unwrap() as u32),
        )
        .unwrap();
        assert_eq!(
            serde_json::json!([value.required, value.forbidden]),
            row["output"],
            "{row}"
        );
    }
    for row in d["default_cases"].as_array().unwrap() {
        let a: Vec<u32> = serde_json::from_value(row["initial"].clone()).unwrap();
        let value = TargetFilter {
            required: a[0],
            forbidden: a[1],
        }
        .for_ability();
        assert_eq!(
            serde_json::json!([value.required, value.forbidden]),
            row["output"]
        );
    }
}
#[test]
fn native_ability_power_recharge_and_resource_cost_stores() {
    use aa_rules::abilities::{dense_cost_words, derived_recharge, effect_flags_and_power};
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-ability-derived.json"
    ))
    .unwrap();
    for row in d["power_cases"].as_array().unwrap() {
        let effects: Vec<(u32, u32)> = serde_json::from_value(row["effects"].clone()).unwrap();
        let value =
            effect_flags_and_power(row["flags"].as_u64().unwrap() as u32, &effects).unwrap();
        assert_eq!(serde_json::json!([value.0, value.1]), row["output"]);
    }
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-ability-arithmetic.json"
    ))
    .unwrap();
    for row in d["recharge_cases"].as_array().unwrap() {
        let pc = serde_json::from_value(row["precision"].clone()).unwrap();
        let input = &row["input"];
        let got = derived_recharge(
            input[0].as_u64().unwrap_or(0) as u32,
            input[1].as_u64().unwrap() as u32,
            input[2].as_u64().unwrap() as u32,
            pc,
        )
        .unwrap();
        assert_eq!(got as u64, row["output_bits"], "{row}");
    }
    for row in d["cost_cases"].as_array().unwrap() {
        let kinds: Vec<u32> = serde_json::from_value(row["kinds"].clone()).unwrap();
        let amounts: Vec<Option<u32>> = row["amounts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| Some(v.as_u64().unwrap() as u32))
            .collect();
        assert_eq!(
            serde_json::to_value(dense_cost_words(&kinds, &amounts).unwrap()).unwrap(),
            row["output_bits"]
        );
    }
    assert!(dense_cost_words(&[0], &[None]).is_err());
}
#[test]
fn native_ability_geometry_thresholds_and_validation() {
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-ability-geometry.json"
    ))
    .unwrap();
    for row in d["cases"].as_array().unwrap() {
        let pc = serde_json::from_value(row["precision"].clone()).unwrap();
        let i: Vec<u32> = serde_json::from_value(row["input_bits"].clone()).unwrap();
        let got = aa_rules::abilities::ability_geometry(i[0], i[1], i[2], i[3], pc);
        if row["output_bits"].is_null() {
            assert!(got.is_err(), "{row}");
        } else {
            let g = got.unwrap();
            assert_eq!(
                serde_json::json!([
                    g.attenuation_min_bits,
                    g.attenuation_factor_bits,
                    g.impulse_area_bits
                ]),
                row["output_bits"],
                "{row}"
            );
        }
    }
}
