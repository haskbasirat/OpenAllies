use aa_body::{
    F32,
    credit::Object,
    properties::{ModifierState, PropertiesState},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::cmp::Ordering;

fn extended(row: &Value) -> Binary {
    let sig = row["extended_significand"].as_u64().unwrap();
    let se = row["extended_sign_exponent"].as_u64().unwrap() as u16;
    let p = Precision::Extended64;
    let mut value = Binary::from_u32((sig >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0_0000_0000_0000).unwrap(), p)
        .unwrap()
        .add(&Binary::from_u32(sig as u32), p)
        .unwrap();
    if sig != 0 {
        let e = i32::from(se & 0x7fff) - 16383 - 63;
        value = value
            .mul(
                &Binary::from_f64_bits(((e + 1023) as u64) << 52).unwrap(),
                p,
            )
            .unwrap();
    }
    if se & 0x8000 != 0 {
        value = value.negated();
    }
    value
}

/// The constructor fixture (502750) records the five native vectors at
/// +80/+8C/+98/+A4/+B0 as `other_tables`; they are typed tables since combat
/// round 6 and must all be empty after construction.
fn legacy_tables(mut value: Value) -> Value {
    let modifier = if value.get("modifier").is_some() {
        &mut value["modifier"]
    } else {
        &mut value["properties"]["modifier"]
    };
    let map = modifier.as_object_mut().unwrap();
    for key in [
        "production",
        "ability_effects",
        "periodic_damage",
        "periodic_heals",
        "reverse_abilities",
    ] {
        assert_eq!(map.remove(key), Some(json!([])), "{key}");
    }
    map.insert("other_tables".into(), json!([[], [], [], [], []]));
    value
}

#[test]
fn native_constructors_scalar_modifiers_and_type_tables() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-properties/native-properties-fixtures.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 420);
    for row in rows {
        let c = &row["input"];
        let output = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let domain = c["domain_count"].as_u64().unwrap() as u32;
        let method = c["method"].as_str().unwrap();
        if method == "properties_new" {
            assert_eq!(
                legacy_tables(
                    json!({"properties":PropertiesState::new(Object(1),domain).unwrap(),"abi_preserved":true})
                ),
                *output,
                "{name}"
            );
            continue;
        }
        if method == "modifier_new" {
            assert_eq!(
                legacy_tables(json!({"modifier":ModifierState::new(domain),"abi_preserved":true})),
                *output,
                "{name}"
            );
            continue;
        }
        let mut modifier = ModifierState::new(domain);
        modifier.scalars = serde_json::from_value(c["scalars"].clone()).unwrap();
        modifier.immunity_types = serde_json::from_value(c["immunities"].clone()).unwrap();
        modifier.damage_factors = serde_json::from_value(c["factors"].clone()).unwrap();
        let offset = c["offset"].as_u64().unwrap() as u32;
        let base = F32(c["base"].as_u64().unwrap() as u32);
        let minimum = F32(c["minimum"].as_u64().unwrap() as u32);
        let id = Object(c["identity"].as_u64().unwrap() as u32);
        if method == "immune" {
            assert_eq!(
                json!({"result":modifier.immune_to(id),"abi_preserved":true}),
                *output,
                "{name}"
            );
            continue;
        }
        let actual = match method {
            "multiply" => modifier.multiply(offset, base, p).unwrap(),
            "divide" => modifier.divide(offset, base, p).unwrap(),
            "add" => Binary::from_f32_bits(
                modifier
                    .adjust_with_floor(offset, base, minimum, p)
                    .unwrap()
                    .0,
            )
            .unwrap(),
            "offense" => Binary::from_f32_bits(modifier.offense_factor().unwrap().0).unwrap(),
            "factor" => Binary::from_f32_bits(modifier.damage_factor(id).0).unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(
            actual.compare(&extended(output)),
            Ordering::Equal,
            "{name} extended"
        );
        assert_eq!(
            actual.store_f32().unwrap(),
            output["stored"].as_u64().unwrap() as u32,
            "{name} f32"
        );
    }
}

#[test]
fn invalid_modifier_offsets_and_arithmetic_are_errors() {
    let mut state = ModifierState::new(1);
    assert!(state.scalar(3).is_err());
    assert!(state.scalar(0x68).is_err());
    assert!(state.multiply(0x40, F32::ONE, Precision::Double53).is_err());
    state.scalars[0x64 / 4 - 1] = F32::ZERO;
    assert!(state.recharge(F32::ONE, Precision::Double53).is_err());
    state.scalars[0x10 / 4 - 1] = F32(0x7fc0_0001);
    assert!(
        state
            .defense(F32::ONE, F32::ZERO, Precision::Double53)
            .is_err()
    );
}
