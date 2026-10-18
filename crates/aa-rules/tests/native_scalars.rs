use aa_numeric::Precision;
use aa_rules::native::scalar_literal;
use serde_json::Value;

#[test]
fn original_tgi_assignment_preserves_double_then_float_and_percentage_stores() {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-actors/typed-scalar-native.json"
    ))
    .unwrap();
    assert_eq!(proof["cases"].as_array().unwrap().len(), 786);
    for row in proof["cases"].as_array().unwrap() {
        let text = row["text"].as_str().unwrap();
        let precision: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let actual = scalar_literal(text, precision);
        if row["value_bits"].is_null() {
            assert!(actual.is_err(), "{row}");
        } else {
            assert_eq!(
                actual.unwrap(),
                row["value_bits"].as_u64().unwrap() as u32,
                "{row}"
            );
        }
    }
}
