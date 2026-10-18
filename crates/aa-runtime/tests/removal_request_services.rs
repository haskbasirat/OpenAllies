//! The clocked-runtime adapter forwards the 0x4F6760 removal-request leaves to
//! Services in native order (native trace: research/phase8-tick
//! native-removal-request.json, case immediate_destroy_paths).
use aa_numeric::Precision;
use aa_runtime::clocked::{Effect, Predicate, Runtime, Services, request_removal};
use aa_simulation::NativeRng;
use aa_world::{
    ObjectId, World,
    initialization::ClockPlan,
    queues::{Delivery, Queues},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Default)]
struct Leaves {
    names: BTreeMap<ObjectId, &'static str>,
    log: Vec<Value>,
}
impl Services for Leaves {
    fn effect(
        &mut self,
        e: Effect,
        _: &mut World,
        _: &mut Queues,
        _: &mut NativeRng,
    ) -> aa_world::Result<()> {
        let n = |id: &ObjectId| self.names[id];
        let v = match e {
            Effect::Availability(id) => json!(["availability_leaf", n(&id)]),
            Effect::Actor2C(id, a) => json!(["actor_2c", n(&id), a]),
            Effect::ActorF0(id, a) => json!(["actor_f0", n(&id), u8::from(a)]),
            Effect::ActorE0(id, a) => json!(["actor_e0", n(&id), u8::from(a)]),
            Effect::RegistryUnlink(id) => json!(["registry_unlink", n(&id)]),
            other => panic!("{other:?}"),
        };
        self.log.push(v);
        Ok(())
    }
    fn predicate(
        &mut self,
        p: Predicate,
        id: ObjectId,
        _: &mut World,
        _: &mut Queues,
        _: &mut NativeRng,
    ) -> aa_world::Result<bool> {
        let name = self.names[&id];
        let (event, result) = match p {
            Predicate::ClassB4 => ("actor_b4", name == "B"),
            Predicate::ClassBC => ("actor_bc", false),
            Predicate::ClassC0 => ("actor_c0", name == "X"),
            other => panic!("{other:?}"),
        };
        self.log.push(json!([event, name, result]));
        Ok(result)
    }
    fn update_component(
        &mut self,
        _: Delivery,
        _: &mut World,
        _: &mut Queues,
        _: &mut NativeRng,
    ) -> aa_world::queues::Result<()> {
        unreachable!()
    }
    fn resolve_handle(
        &mut self,
        id: ObjectId,
        _: &mut World,
        _: &mut Queues,
        _: &mut NativeRng,
    ) -> aa_world::Result<Option<ObjectId>> {
        Ok((self.names[&id] == "B").then_some(3))
    }
}

#[test]
fn destroy_request_leaves_reach_services_in_native_order() {
    let plan = ClockPlan::from_tick_rate(16.0_f32.to_bits(), Precision::Extended64).unwrap();
    let mut leaves = Leaves::default();
    leaves.names.extend([(2, "B"), (3, "X")]);
    let mut rt = Runtime::new(&plan, NativeRng::default(), leaves, Precision::Extended64).unwrap();
    rt.world.phase = 3;
    rt.world.object_flags.extend([(2, 0), (3, 0x40)]);
    rt.world
        .enqueue_back(aa_world::ListKind::Class178, 3)
        .unwrap();
    let Runtime {
        world,
        queues,
        rng,
        services,
        ..
    } = &mut rt;
    assert!(request_removal(services, world, queues, rng, 2, true, 0).unwrap());
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-tick/native-removal-request.json"
    ))
    .unwrap();
    let case = native["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "immediate_destroy_paths")
        .unwrap();
    // Native events for request B (resolved to X), excluding list removals
    // which are World bookkeeping rather than Services leaves.
    let expected: Vec<Value> = case["events"]
        .as_array()
        .unwrap()
        .iter()
        .skip_while(|e| !(e["event"] == "request_removal" && e["name"] == "B"))
        .skip(1)
        .take_while(|e| e["event"] != "request_removal")
        .filter(|e| e["event"] != "classification_remove")
        .map(|e| {
            let mut row = vec![e["event"].clone(), e["name"].clone()];
            for k in ["result", "argument"] {
                if let Some(v) = e.get(k) {
                    row.push(v.clone());
                }
            }
            Value::Array(row)
        })
        .collect();
    assert_eq!(json!(services.log), json!(expected));
    assert_eq!(world.object_flags[&3], 0x42);
    assert!(world.list_values(aa_world::ListKind::Class178).is_empty());
}
