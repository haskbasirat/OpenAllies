//! Replays research/phase8-combat/native-experience-amount-fixtures.json:
//! original 571820/571960/5719E0/5718E0/5718B0 executed under Unicorn.
use aa_body::{
    F32, Result,
    credit::Object,
    experience_amounts::{self, Kind, Settings, World},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::cmp::Ordering;

fn extended(row: &Value) -> Binary {
    let sig = row["mantissa"].as_u64().unwrap();
    let se = row["sign_exponent"].as_u64().unwrap() as u16;
    let p = Precision::Extended64;
    let mut value = Binary::from_u32((sig >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0_0000_0000_0000).unwrap(), p)
        .unwrap()
        .add(&Binary::from_u32(sig as u32), p)
        .unwrap();
    if sig != 0 {
        let exponent = i32::from(se & 0x7fff) - 16383 - 63;
        value = value
            .mul(
                &Binary::from_f64_bits(((exponent + 1023) as u64) << 52).unwrap(),
                p,
            )
            .unwrap();
    }
    if se & 0x8000 != 0 {
        value = value.negated();
    }
    value
}

struct Host {
    values: Value,
    definition4f4: u32,
    events: Vec<Value>,
}
impl World for Host {
    fn actor_value(&mut self, actor: Object, _: Precision) -> Result<Binary> {
        self.events.push(json!({"ActorValue": actor.0}));
        let list = self.values[actor.0.to_string()].as_array_mut().unwrap();
        let bits = list.remove(0).as_u64().unwrap();
        Ok(Binary::from_f64_bits(bits).unwrap())
    }
    fn definition4f4(&self, actor: Object) -> Result<F32> {
        assert_eq!(actor, Object(2));
        Ok(F32(self.definition4f4))
    }
}

fn settings(v: &Value) -> Settings {
    serde_json::from_value(v.clone()).unwrap()
}

#[test]
fn original_experience_amounts_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-experience-amount-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 400);
    for row in cases {
        let input = &row["input"];
        let p: Precision = serde_json::from_value(input["precision"].clone()).unwrap();
        let s = settings(&input["settings"]);
        let mut host = Host {
            values: input["values"].clone(),
            definition4f4: input["definition4f4"].as_u64().unwrap() as u32,
            events: vec![],
        };
        let victim = (input["victim_present"] == true).then_some(Object(2));
        let source = Object(1);
        let actual = match input["operation"].as_str().unwrap() {
            "ratio" => experience_amounts::ratio(source, victim, &s, &mut host, p).unwrap(),
            "engagement" => {
                experience_amounts::engagement(source, victim, &s, &mut host, p).unwrap()
            }
            op => {
                let kind = match op {
                    "kill" => Kind::Kill,
                    "capture" => Kind::Capture,
                    "rout" => Kind::Rout,
                    _ => unreachable!(),
                };
                let value =
                    experience_amounts::clamped(kind, source, victim, &s, &mut host, p).unwrap();
                Binary::from_f32_bits(value.0).unwrap()
            }
        };
        let expected = extended(&row["output"]["result"]);
        assert_eq!(
            actual.compare(&expected),
            Ordering::Equal,
            "{} {}",
            input["name"],
            input["precision"]
        );
        assert_eq!(
            json!(host.events),
            row["output"]["events"],
            "{}",
            input["name"]
        );
    }
}

#[test]
fn kill_variant_agrees_with_kill_credit_adapter_inputs() {
    // The clamped kill routine is the same native 571960 that credit.rs
    // inlines; a nonpositive source returns +0 before the multiplier.
    let s = Settings {
        minimum: F32(0),
        maximum: F32(10f32.to_bits()),
        kill: F32(1f32.to_bits()),
        capture: F32(1f32.to_bits()),
        rout: F32(1f32.to_bits()),
        engagement: F32(1f32.to_bits()),
        default_value: F32(1f32.to_bits()),
    };
    let mut host = Host {
        values: json!({"1": [0f64.to_bits()], "2": [3f64.to_bits()]}),
        definition4f4: 0,
        events: vec![],
    };
    let value = experience_amounts::clamped(
        Kind::Kill,
        Object(1),
        Some(Object(2)),
        &s,
        &mut host,
        Precision::Extended64,
    )
    .unwrap();
    assert_eq!(value, F32(0));
}
