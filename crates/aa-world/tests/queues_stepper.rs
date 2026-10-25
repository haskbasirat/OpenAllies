//! The 69 native queue sequences (research/phase6-queues) dispatched through
//! the step-by-step API (`Queues::begin_update` / `next_delivery` /
//! `end_update`) a host uses when the queues live inside its own world.
use aa_numeric::Precision;
use aa_simulation::NativeRng;
use aa_world::queues::{Callbacks, Clock, Delivery, Kind, Queues, Result};
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
        "update" => {
            // The host keeps the queues and dispatches step by step (begin/next/end),
            // running each leaf between deliveries.
            let mut d = queues.begin_update(uint("queue") as u8)?;
            while let Some(event) = queues.next_delivery(&mut d)? {
                host.events.push(event);
                for action in host
                    .actions
                    .remove(&event.data.to_string())
                    .unwrap_or_default()
                {
                    apply(queues, host, &action)?;
                }
            }
            queues.end_update(d)?;
        }
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
fn native_stateful_queue_sequences_through_the_stepper() {
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
