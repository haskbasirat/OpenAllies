use aa_numeric::Precision;
use aa_simulation::NativeRng;
use aa_world::queues::{Callbacks, Clock, Delivery, Kind, Queues, Result, period_ticks};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Case {
    name: String,
    counts: Vec<i32>,
    seed: u32,
    clock: Clock,
    precision: u8,
    operations: Vec<Value>,
    callback_actions: BTreeMap<String, Vec<Value>>,
    expected: Vec<aa_world::queues::QueueSnapshot>,
    events: Vec<Delivery>,
    rng_state: u32,
}
fn precision(pc: u8) -> Precision {
    match pc {
        24 => Precision::Single24,
        53 => Precision::Double53,
        64 => Precision::Extended64,
        _ => panic!(),
    }
}
struct Host {
    rng: NativeRng,
    actions: BTreeMap<String, Vec<Value>>,
    events: Vec<Delivery>,
}
impl Callbacks for Host {
    fn deliver(&mut self, queues: &mut Queues, event: Delivery) -> Result<()> {
        self.events.push(event);
        for action in self
            .actions
            .remove(&event.data.to_string())
            .unwrap_or_default()
        {
            apply(queues, self, &action)?;
        }
        Ok(())
    }
}
fn apply(queues: &mut Queues, host: &mut Host, op: &Value) -> Result<()> {
    let uint = |key: &str| op[key].as_u64().unwrap();
    match op["type"].as_str().unwrap() {
        "clock" => queues.clock = serde_json::from_value(op["clock"].clone()).unwrap(),
        "cancel" => queues.cancel_owner(uint("owner"))?,
        "update" => queues.update(uint("queue") as u8, host)?,
        "add" => {
            let q = uint("queue") as u8;
            if q < 3 {
                queues.add_periodic(q, uint("owner"), uint("data") as u32, &mut host.rng)?;
            } else {
                queues.add_after(
                    q,
                    uint("owner"),
                    uint("data") as u32,
                    uint("delay_bits") as u32,
                )?;
            }
        }
        "at" => {
            queues.add_at(
                uint("queue") as u8,
                uint("owner"),
                uint("data") as u32,
                uint("when_bits") as u32,
            )?;
        }
        other => panic!("{other}"),
    }
    Ok(())
}

#[test]
fn native_stateful_queue_sequences() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-queues/native-queues.json"
    ))
    .unwrap();
    let cases: Vec<Case> = serde_json::from_value(fixture["cases"].clone()).unwrap();
    assert_eq!(cases.len(), 69);
    for case in cases {
        let mut queues = Queues::new(case.clock, precision(case.precision)).unwrap();
        for (i, count) in case.counts.into_iter().enumerate() {
            queues
                .initialize(
                    i as u8,
                    if i < 3 {
                        Kind::Periodic
                    } else {
                        Kind::Deferred
                    },
                    count,
                )
                .unwrap();
        }
        let mut host = Host {
            rng: NativeRng::new(case.seed),
            actions: case.callback_actions,
            events: Vec::new(),
        };
        for op in case.operations {
            apply(&mut queues, &mut host, &op).unwrap();
        }
        assert_eq!(
            host.events, case.events,
            "{} callback order/data",
            case.name
        );
        assert_eq!(
            queues.snapshot(),
            case.expected,
            "{} final queues",
            case.name
        );
        assert_eq!(host.rng.state, case.rng_state, "{} RNG", case.name);
    }
}

#[test]
fn native_ceil_period_arithmetic() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-queues/native-queues.json"
    ))
    .unwrap();
    let values = fixture["periods"].as_array().unwrap();
    assert_eq!(values.len(), 150);
    for v in values {
        assert_eq!(
            period_ticks(
                v["period_bits"].as_u64().unwrap() as u32,
                v["step_bits"].as_u64().unwrap() as u32,
                precision(v["precision"].as_u64().unwrap() as u8)
            )
            .unwrap(),
            v["ticks"].as_i64().unwrap() as i32,
            "{v}"
        );
    }
}

#[test]
fn callback_errors_preserve_effects_and_allow_explicit_retry() {
    struct Fail;
    impl Callbacks for Fail {
        fn deliver(&mut self, _: &mut Queues, _: Delivery) -> Result<()> {
            Err(aa_world::queues::Error::Callback(
                "deliberate failure".into(),
            ))
        }
    }
    let mut q = Queues::new(
        Clock {
            begin_bits: 0,
            end_bits: 0.0625_f32.to_bits(),
            step_bits: 0.0625_f32.to_bits(),
        },
        Precision::Double53,
    )
    .unwrap();
    q.initialize(3, Kind::Deferred, 3).unwrap();
    q.add_after(3, 1, 10, 0).unwrap();
    assert!(q.has(1, 3, 10));
    assert!(q.update(3, &mut Fail).is_err());
    assert!(!q.has(1, 3, 10));
    assert_eq!(q.snapshot()[0].current, 0);
    q.update(3, &mut Fail).unwrap();
    assert_eq!(q.snapshot()[0].current, 1);
}
