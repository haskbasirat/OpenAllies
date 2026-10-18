use aa_body::*;
use aa_numeric::Precision;
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase4-body/body-native-oracle.json"
    ))
    .unwrap()
}

#[derive(Default)]
struct RecordingHost {
    events: Vec<Value>,
    mutations: Vec<Value>,
}
impl Host for RecordingHost {
    fn event(
        &mut self,
        event: Event,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<(), String> {
        self.events.push(json!({"event":event,"body":body}));
        let encoded = serde_json::to_value(&event).unwrap();
        let name = encoded
            .as_str()
            .unwrap_or_else(|| encoded.as_object().unwrap().keys().next().unwrap());
        if self.mutations.first().is_some_and(|row| row["on"] == name) {
            let row = self.mutations.remove(0);
            if let Some(changes) = row["body"].as_object() {
                let mut value = serde_json::to_value(&*body).unwrap();
                for (key, new) in changes {
                    value[key] = new.clone();
                }
                *body = serde_json::from_value(value).unwrap();
            }
            if let Some(changes) = row["environment"].as_object() {
                let mut value = serde_json::to_value(&*env).unwrap();
                for (key, new) in changes {
                    value[key] = new.clone();
                }
                *env = serde_json::from_value(value).unwrap();
            }
        }
        Ok(())
    }
}

#[test]
fn original_constructor_setter_heal_maximum_and_death_boundaries() {
    let report = oracle();
    assert_eq!(report["passed"], true);
    let rows = report["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 270);
    for row in rows {
        let mut body: Body = serde_json::from_value(row["state"].clone()).unwrap();
        let mut env: Environment = serde_json::from_value(row["environment"].clone()).unwrap();
        let mode: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let mut host = RecordingHost {
            events: vec![],
            mutations: row["mutations"].as_array().cloned().unwrap_or_default(),
        };
        let get = |key: &str| row[key].as_u64().unwrap() as u32;
        match row["operation"].as_str().unwrap() {
            "constructor" => {
                body = Body::from_resolved(env.derived_normalization_maximum, body.defense).unwrap()
            }
            "set" => body
                .set_health(
                    F32(get("requested")),
                    get("token"),
                    row["notify"].as_bool().unwrap(),
                    &mut env,
                    &mut host,
                    mode,
                )
                .unwrap(),
            "maximum" => body
                .set_health_and_maximum(
                    F32(get("requested")),
                    F32(get("maximum")),
                    &mut env,
                    &mut host,
                    mode,
                )
                .unwrap(),
            "heal" => body
                .heal_plain(
                    HealRequest {
                        source_id: get("source_id"),
                        amount: F32(get("amount")),
                        token: get("token"),
                        converts_to_damage: false,
                    },
                    &mut env,
                    &mut host,
                    mode,
                )
                .unwrap(),
            "death" => body.check_death(&mut env, &mut host).unwrap(),
            "recalculate" => body
                .update_resolved_maximum(
                    F32(get("maximum")),
                    row["preserve_ratio"].as_bool().unwrap(),
                    &env,
                    mode,
                )
                .unwrap(),
            "sink" => assert_eq!(
                body.sink_fraction_f32(F32(get("now")), F32(get("rot_time")), mode)
                    .unwrap()
                    .0,
                get("fraction")
            ),
            _ => panic!("unknown case"),
        }
        assert_eq!(
            serde_json::to_value(body).unwrap(),
            row["result"],
            "{} {:?}",
            row["name"],
            mode
        );
        assert_eq!(
            host.events,
            *row["events"].as_array().unwrap(),
            "{} {:?}",
            row["name"],
            mode
        );
        assert!(host.mutations.is_empty());
    }
}

#[test]
fn unsupported_outer_paths_reject_before_mutation_and_do_not_guess_combat() {
    let report = oracle();
    let row = &report["cases"][0];
    let body: Body = serde_json::from_value(row["state"].clone()).unwrap();
    let env: Environment = serde_json::from_value(row["environment"].clone()).unwrap();
    for linked in [false, true] {
        let mut b = body.clone();
        let mut e = env.clone();
        let mut h = RecordingHost::default();
        e.has_linked_body = linked;
        let result = if linked {
            b.set_health(F32::ZERO, 0, true, &mut e, &mut h, Precision::Extended64)
        } else {
            b.heal_plain(
                HealRequest {
                    source_id: 12,
                    amount: F32::ONE,
                    token: 7,
                    converts_to_damage: true,
                },
                &mut e,
                &mut h,
                Precision::Extended64,
            )
        };
        assert_eq!(
            result,
            Err(if linked {
                Error::LinkedActorUnsupported
            } else {
                Error::DamageConvertingHealerUnsupported
            })
        );
        assert_eq!(b, body);
        assert!(h.events.is_empty());
    }
    let mut b = body.clone();
    let mut e = env;
    let mut h = RecordingHost::default();
    assert_eq!(
        b.set_health(
            F32(0x7fc0_0000),
            0,
            true,
            &mut e,
            &mut h,
            Precision::Extended64
        ),
        Err(Error::NonFinite)
    );
    assert_eq!(b, body);
    assert!(h.events.is_empty());
    assert_eq!(
        Body::from_resolved(F32::ZERO, F32::ZERO),
        Err(Error::InvalidMaximum)
    );
}

#[test]
fn callback_failure_is_explicit_and_never_rolled_back_or_replayed() {
    struct Fail(usize);
    impl Host for Fail {
        fn event(
            &mut self,
            _: Event,
            _: &mut Body,
            _: &mut Environment,
        ) -> std::result::Result<(), String> {
            self.0 += 1;
            Err("host failure".into())
        }
    }
    let report = oracle();
    let mut env: Environment =
        serde_json::from_value(report["cases"][0]["environment"].clone()).unwrap();
    let mut body = Body::from_resolved(F32(100.0f32.to_bits()), F32::ZERO).unwrap();
    let mut host = Fail(0);
    assert_eq!(
        body.set_health(
            F32::ZERO,
            0,
            true,
            &mut env,
            &mut host,
            Precision::Extended64
        ),
        Err(Error::Callback("host failure".into()))
    );
    assert_eq!(body.health, F32::ZERO);
    assert_eq!(host.0, 1);
}
