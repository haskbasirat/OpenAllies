use aa_numeric::Precision;
use aa_runtime::clocked::{Effect, Predicate, Runtime, Services};
use aa_simulation::NativeRng;
use aa_world::{
    ObjectId, World,
    initialization::ClockPlan,
    queues::{Delivery, Queues},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn pc(value: u64) -> Precision {
    match value {
        24 => Precision::Single24,
        53 => Precision::Double53,
        64 => Precision::Extended64,
        _ => panic!(),
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-runtime/native-runtime.json"
    ))
    .unwrap()
}
fn operation(queues: &mut Queues, rng: &mut NativeRng, op: &Value) -> aa_world::queues::Result<()> {
    let n = |key: &str| op[key].as_u64().unwrap();
    match op["type"].as_str().unwrap() {
        "cancel" => queues.cancel_owner(n("owner"))?,
        "add" => {
            let id = n("queue") as u8;
            if id < 3 {
                queues.add_periodic(id, n("owner"), n("data") as u32, rng)?;
            } else {
                queues.add_after(id, n("owner"), n("data") as u32, n("delay_bits") as u32)?;
            }
        }
        other => panic!("unhandled native fixture operation {other}"),
    }
    Ok(())
}
#[derive(Default)]
struct Leaves {
    first: Vec<Value>,
    manager: Vec<Value>,
    callbacks: BTreeMap<String, Vec<Value>>,
    timeline: Vec<Value>,
    outer: Vec<Value>,
    fail: bool,
    clock_mutation: Option<u8>,
}
impl Services for Leaves {
    fn effect(
        &mut self,
        e: Effect,
        w: &mut World,
        q: &mut Queues,
        rng: &mut NativeRng,
    ) -> aa_world::Result<()> {
        let actions = match e {
            Effect::Fog(value) => {
                self.outer
                    .push(json!(["fog", w.clock.tick, u8::from(value)]));
                Vec::new()
            }
            Effect::FirstTick => {
                self.outer.push(json!(["first", w.clock.tick]));
                std::mem::take(&mut self.first)
            }
            Effect::Manager => {
                self.outer.push(json!(["manager", w.clock.tick]));
                std::mem::take(&mut self.manager)
            }
            other => {
                return Err(aa_world::Error::Callback(format!(
                    "unhandled fixture effect {other:?}"
                )));
            }
        };
        for op in actions {
            operation(q, rng, &op).map_err(|e| aa_world::Error::Callback(e.to_string()))?;
        }
        Ok(())
    }
    fn predicate(
        &mut self,
        _: Predicate,
        _: ObjectId,
        _: &mut World,
        _: &mut Queues,
        _: &mut NativeRng,
    ) -> aa_world::Result<bool> {
        Err(aa_world::Error::Callback(
            "fixture has no live actor predicates".into(),
        ))
    }
    fn update_component(
        &mut self,
        event: Delivery,
        w: &mut World,
        q: &mut Queues,
        rng: &mut NativeRng,
    ) -> aa_world::queues::Result<()> {
        self.timeline
            .push(json!({"tick":w.clock.tick,"phase":w.phase,"event":event}));
        if let Some(change) = self.clock_mutation {
            match change {
                0 => w.clock.tick = w.clock.tick.wrapping_add(1),
                1 => q.clock.begin_bits = 1,
                2 => w.clock_precision = Precision::Single24,
                3 => q.precision = Precision::Single24,
                _ => panic!(),
            }
        }
        if self.fail {
            return Err(aa_world::queues::Error::Callback(
                "explicit fixture failure after delivery".into(),
            ));
        }
        for op in self
            .callbacks
            .remove(&event.data.to_string())
            .unwrap_or_default()
        {
            operation(q, rng, &op)?;
        }
        Ok(())
    }
}

#[test]
fn original_world_clock_and_queue_initialization() {
    let f = fixture();
    assert_eq!(f["plans"].as_array().unwrap().len(), 24);
    for row in f["plans"].as_array().unwrap() {
        let plan = ClockPlan::from_tick_rate(
            row["rate_bits"].as_u64().unwrap() as u32,
            pc(row["precision"].as_u64().unwrap()),
        )
        .unwrap();
        assert_eq!(json!(plan.step_bits), row["step_bits"]);
        assert_eq!(json!(plan.queues.map(|p| p.bucket_count)), row["counts"]);
    }
}

#[test]
fn actual_native_world_phases_dispatch_actual_native_queues() {
    let f = fixture();
    assert_eq!(f["composed"].as_array().unwrap().len(), 12);
    for row in f["composed"].as_array().unwrap() {
        let precision = pc(row["precision"].as_u64().unwrap());
        let plan = ClockPlan::from_tick_rate(16f32.to_bits(), precision).unwrap();
        let services = Leaves {
            first: serde_json::from_value(row["first"].clone()).unwrap(),
            manager: serde_json::from_value(row["manager"].clone()).unwrap(),
            callbacks: serde_json::from_value(row["callbacks"].clone()).unwrap(),
            ..Default::default()
        };
        let mut runtime =
            Runtime::new(&plan, NativeRng::new(0x12345678), services, precision).unwrap();
        for op in row["initial"].as_array().unwrap() {
            operation(&mut runtime.queues, &mut runtime.rng, op).unwrap();
        }
        for _ in 0..row["ticks"].as_u64().unwrap() {
            runtime.step().unwrap();
        }
        assert_eq!(
            json!(runtime.services.timeline),
            row["timeline"],
            "delivery order {}",
            row["name"]
        );
        assert_eq!(
            json!(runtime.services.outer),
            row["outer"],
            "outer callbacks {}",
            row["name"]
        );
        assert_eq!(
            json!(runtime.queues.snapshot()),
            row["queues"],
            "queue contents {}",
            row["name"]
        );
        assert_eq!(json!(runtime.rng.state), row["rng_state"]);
        let c = runtime.world.clock;
        assert_eq!(json!([c.tick, c.begin.0, c.end.0, c.step.0]), row["clock"]);
        assert_eq!(json!(runtime.world.phase), row["phase"]);
        assert!(!runtime.stopped());
    }
}

#[test]
fn failed_delivery_stops_world_without_replaying_committed_effects() {
    let precision = Precision::Double53;
    let plan = ClockPlan::from_tick_rate(16f32.to_bits(), precision).unwrap();
    let mut runtime = Runtime::new(
        &plan,
        NativeRng::new(42),
        Leaves {
            fail: true,
            ..Default::default()
        },
        precision,
    )
    .unwrap();
    runtime.queues.add_after(3, 1, 17, 0).unwrap();
    assert!(runtime.step().is_err());
    assert!(runtime.stopped());
    assert_eq!(runtime.world.clock.tick, 1);
    assert_eq!(runtime.world.phase, 1);
    assert_eq!(runtime.services.timeline.len(), 1);
    let queues = runtime.queues.snapshot();
    assert!(matches!(
        runtime.step(),
        Err(aa_runtime::clocked::Error::Stopped)
    ));
    assert_eq!(runtime.queues.snapshot(), queues);
    assert_eq!(runtime.services.timeline.len(), 1);
}

#[test]
fn service_clock_mutation_stops_the_runtime() {
    let precision = Precision::Double53;
    let plan = ClockPlan::from_tick_rate(16f32.to_bits(), precision).unwrap();
    for change in 0..4 {
        let services = Leaves {
            clock_mutation: Some(change),
            ..Default::default()
        };
        let mut runtime = Runtime::new(&plan, NativeRng::new(42), services, precision).unwrap();
        runtime.queues.add_after(3, 1, 17, 0).unwrap();
        let error = runtime.step().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("authoritative world/queue clock")
        );
        assert!(runtime.stopped());
        assert_eq!(runtime.services.timeline.len(), 1);
    }
}

#[test]
fn original_large_tick_precision_and_signed_rollover() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-runtime/native-clock-edges.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 117);
    for row in rows {
        let precision = pc(row["precision"].as_u64().unwrap());
        let plan = ClockPlan::from_tick_rate(row["rate_bits"].as_u64().unwrap() as u32, precision)
            .unwrap();
        let mut runtime =
            Runtime::new(&plan, NativeRng::new(42), Leaves::default(), precision).unwrap();
        let before: [u32; 4] = serde_json::from_value(row["before"].clone()).unwrap();
        runtime.world.clock = aa_simulation::NativeClock {
            tick: before[0],
            begin: aa_simulation::FloatBits(before[1]),
            end: aa_simulation::FloatBits(before[2]),
            step: aa_simulation::FloatBits(before[3]),
        };
        let after = runtime.step().unwrap();
        assert_eq!(
            json!([after.tick, after.begin.0, after.end.0, after.step.0]),
            row["after"],
            "{row}"
        );
    }
}
