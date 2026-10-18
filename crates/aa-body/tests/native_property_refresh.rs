use aa_body::{
    F32, credit::Object, properties::PropertiesState, property_effects::Instances,
    property_flags::Definitions, property_refresh,
};
use serde_json::{Value, json};
#[test]
fn original_refresh_dispatch_includes_disabled_duplicates_without_reaggregation() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actor-live/native-property-refresh.json"
    ))
    .unwrap();
    assert_eq!(doc["cases"].as_array().unwrap().len(), 54);
    for row in doc["cases"].as_array().unwrap() {
        let c = &row["input"];
        let before = &row["output"]["before"];
        let expected = &row["output"]["after"];
        let defs: Definitions = serde_json::from_value(c["definitions"].clone()).unwrap();
        let mut instances: Instances = serde_json::from_value(before["instances"].clone()).unwrap();
        let mut state = PropertiesState::new(Object(1), 19).unwrap();
        state.effects = serde_json::from_value(before["effects"].clone()).unwrap();
        state.modifier.scalars = serde_json::from_value(before["scalars"].clone()).unwrap();
        state.modifier.flags = before["flags"].as_u64().unwrap() as u32;
        state.modifier_flags = before["modifier_flags"].as_u64().unwrap() as u32;
        state.modifier.damage_factors =
            serde_json::from_value(before["damage_factors"].clone()).unwrap();
        let mut events = vec![];
        property_refresh::refresh(c["attribute"].as_u64().unwrap()as u32,&mut state,&mut instances,&defs,|m,state,_|{events.push(json!({"NumericChanged":{"kind":m.kind,"index":m.index,"amount":m.amount,"repair":m.repair,"value":state.modifier.scalars[m.index as usize]}}));Ok(())}).unwrap();
        assert_eq!(
            json!({"effects":state.effects,"scalars":state.modifier.scalars,"modifier_flags":state.modifier_flags,"instances":instances,"events":events,"flags":state.modifier.flags,"mover_byte_4c":0,"damage_factors":state.modifier.damage_factors}),
            *expected
        );
    }
}
#[test]
fn constructor_empty_refresh_needs_no_effect_defaults() {
    let mut state = PropertiesState::new(Object(1), 0).unwrap();
    let mut instances = Instances::new();
    property_refresh::refresh(
        0,
        &mut state,
        &mut instances,
        &Definitions::new(),
        |_, _, _| panic!("emptynativebranchmustnotcallback"),
    )
    .unwrap();
    assert!(property_refresh::require_empty(&state).is_ok());
    state.effects.push(Object(1));
    assert!(property_refresh::require_empty(&state).is_err());
    assert_eq!(state.modifier.scalars[0], F32::ZERO);
}
