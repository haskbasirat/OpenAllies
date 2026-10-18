use aa_numeric::Precision;
use aa_triggers::{
    decode::{Element, Parameter, SavedManager, Span, Trigger, Value, Variable},
    manager::{Elements, Manager},
    program::CoreProgram,
};
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Deserialize)]
struct InputParameter {
    argument: String,
    item: String,
    item_index: u32,
    storage: u32,
    flag: u8,
    value: serde_json::Value,
}
#[derive(Deserialize)]
struct InputElement {
    name: String,
    opcode: u32,
    parameters: Vec<InputParameter>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct State {
    id: u32,
    enabled: u8,
    activation_bits: u32,
}
#[derive(Deserialize)]
struct Input {
    name: String,
    time_bits: u32,
    variables: BTreeMap<String, u32>,
    triggers: Vec<State>,
    element: InputElement,
    precision: Precision,
}
#[derive(Deserialize)]
struct Output {
    result: bool,
    variables: BTreeMap<String, u32>,
    triggers: Vec<State>,
    current_element: u32,
    diagnostics: usize,
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
fn span() -> Span {
    Span {
        offset: 0,
        length: 0,
    }
}
fn prepare(c: Input) -> (String, Manager, CoreProgram, Element) {
    let element = Element {
        source: span(),
        name: c.element.name,
        opcode: c.element.opcode,
        word_10: 0,
        parameters: c
            .element
            .parameters
            .into_iter()
            .map(|p| Parameter {
                source: span(),
                argument: p.argument,
                item: p.item,
                item_index: p.item_index,
                storage: p.storage,
                flag: p.flag,
                value: match p.storage {
                    1 | 2 => Value::Word(p.value.as_u64().unwrap() as u32),
                    6 => Value::Text(p.value.as_str().unwrap().encode_utf16().collect()),
                    _ => panic!("unexpected fixture storage"),
                },
            })
            .collect(),
    };
    let manager = Manager::from_saved(SavedManager {
        next_id: 1,
        triggers: c
            .triggers
            .iter()
            .map(|t| Trigger {
                source: span(),
                id: t.id,
                mode: 1,
                enabled: t.enabled,
                repeat: 0,
                activation_bits: t.activation_bits,
                name: vec![],
                description: vec![],
                word_2c: 0,
                events: vec![],
                conditions: vec![],
                actions: vec![],
            })
            .collect(),
        variables: c
            .variables
            .iter()
            .map(|(name, bits)| Variable {
                name: name.encode_utf16().collect(),
                bits: *bits,
            })
            .collect(),
        strings: vec![],
    });
    (
        c.name,
        manager,
        CoreProgram::new(c.time_bits, 6, c.precision),
        element,
    )
}
#[test]
fn native_intrinsics_validate_convert_compare_modify_and_activate() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-triggers/native-core-program.json"
    ))
    .unwrap();
    assert_eq!(fixture.cases.len(), 327);
    for case in fixture.cases {
        let (name, mut manager, mut program, element) = prepare(case.input);
        let expected = case.output;
        assert_eq!(
            program.evaluate(&mut manager, 0, &element).unwrap(),
            expected.result,
            "{name}"
        );
        let state = manager.snapshot();
        let variables = state
            .variables
            .iter()
            .map(|v| (String::from_utf16(&v.name).unwrap(), v.bits))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(variables, expected.variables, "{name}");
        assert_eq!(
            state
                .triggers
                .iter()
                .map(|t| State {
                    id: t.id,
                    enabled: t.enabled,
                    activation_bits: t.activation_bits
                })
                .collect::<Vec<_>>(),
            expected.triggers,
            "{name}"
        );
        assert_eq!(program.diagnostics.len(), expected.diagnostics, "{name}");
        assert_eq!(expected.current_element, 0);
    }
}
#[test]
fn missing_world_services_and_nonfinite_inputs_are_not_success() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-triggers/native-core-program.json"
    ))
    .unwrap();
    let case = fixture
        .cases
        .into_iter()
        .find(|c| c.input.element.opcode == 1 && c.input.element.parameters[0].item_index == 2)
        .unwrap();
    let (_, mut manager, mut program, mut element) = prepare(case.input);
    element.parameters[0].value = Value::Word(0x7fc0_0000);
    assert!(program.evaluate(&mut manager, 0, &element).is_err());
    element.opcode = 62;
    element.name = "sai_disabled_for_kingdom".into();
    element.parameters = vec![Parameter {
        source: span(),
        argument: "ma_kingdom".into(),
        item: "mi_kingdom_id".into(),
        item_index: 19,
        storage: 1,
        flag: 1,
        value: Value::Word(1),
    }];
    assert!(program.evaluate(&mut manager, 0, &element).is_err());
    element.opcode = 1000;
    assert!(program.evaluate(&mut manager, 0, &element).is_err());
}
