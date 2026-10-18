use aa_body::{
    F32, modifier_load,
    property_effects::{NumericKind, NumericModifier},
};
use aa_numeric::Precision;
use serde::Deserialize;

#[derive(Deserialize)]
struct Input {
    name: String,
    kind: NumericKind,
    attribute: i32,
    amount: F32,
    repair: bool,
    found: bool,
    precision: Precision,
}
#[derive(Deserialize)]
struct Output {
    error: Option<String>,
    modifier: Option<NumericModifier>,
}
#[derive(Deserialize)]
struct Case {
    input: Input,
    output: Output,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[test]
fn authored_numeric_modifier_conversion_matches_native() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-properties/native-modifier-load.json"
    ))
    .unwrap();
    assert_eq!(fixture.cases.len(), 957);
    for case in fixture.cases {
        let c = case.input;
        let result = modifier_load::numeric(
            c.kind,
            if c.found { c.attribute } else { -1 },
            c.amount,
            c.repair,
            c.precision,
        );
        if case.output.error.is_some() {
            assert!(result.is_err(), "{}", c.name);
        } else {
            assert_eq!(result.unwrap(), case.output.modifier.unwrap(), "{}", c.name);
        }
    }
}

#[test]
fn unsupported_loader_inputs_are_explicit_errors() {
    assert!(
        modifier_load::numeric(
            NumericKind::Multiply,
            18,
            F32::ZERO,
            true,
            Precision::Extended64
        )
        .is_err()
    );
    assert!(
        modifier_load::numeric(
            NumericKind::Add,
            4,
            F32(0x7fc0_0000),
            false,
            Precision::Extended64
        )
        .is_err()
    );
}
