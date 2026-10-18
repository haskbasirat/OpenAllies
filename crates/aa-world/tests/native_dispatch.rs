//! Expected traces come from executing original x86, not from this implementation.
use aa_simulation::FloatBits;
use aa_world::{Callbacks, Event, ListKind as L, ObjectId, Operation as O, World};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};

const ORACLE: &str =
    include_str!("../../../research/phase2-world-order/world-dispatch-oracle.json");

#[derive(Default)]
struct Leaves {
    names: BTreeMap<ObjectId, &'static str>,
    retain: BTreeMap<ObjectId, VecDeque<bool>>,
    classification: BTreeMap<ObjectId, u8>,
    actions: BTreeMap<String, Vec<(L, ObjectId, bool)>>,
}
impl Leaves {
    fn payload(&mut self, w: &mut World, id: ObjectId, name: &'static str) {
        self.names.insert(id, name);
        w.pending100_targets.insert(id, id);
    }
    fn actor(
        &mut self,
        w: &mut World,
        id: ObjectId,
        name: &'static str,
        retain: &[bool],
        class: u8,
        flags: u32,
    ) {
        self.names.insert(id, name);
        self.retain.insert(id, retain.iter().copied().collect());
        self.classification.insert(id, class);
        w.object_flags.insert(id, flags);
    }
    fn action(&mut self, w: &mut World, key: String) {
        if let Some(actions) = self.actions.remove(&key) {
            for (list, object, front) in actions {
                if front {
                    w.enqueue_front(list, object)
                } else {
                    w.enqueue_back(list, object)
                }
                .unwrap();
            }
        }
    }
}
impl Callbacks for Leaves {
    // Explicit original-oracle seams. Production worlds must implement these
    // services; the trait no longer silently substitutes no-op defaults.
    fn fog_callback(&mut self, _: &mut World, _: bool) -> aa_world::Result<()> {
        Ok(())
    }
    fn first_tick_callback(&mut self, _: &mut World) -> aa_world::Result<()> {
        Ok(())
    }
    fn pending_100_component(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
        Ok(())
    }
    fn availability_leaf(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
        Ok(())
    }
    fn registry_unlink(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
        Ok(())
    }
    fn actor_f0(&mut self, _: &mut World, _: ObjectId, _: bool) -> aa_world::Result<()> {
        Ok(())
    }
    fn deferred_payload(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<()> {
        self.action(w, format!("payload:{}", self.names[&id]));
        Ok(())
    }
    fn queue_update(&mut self, w: &mut World, q: u8) -> aa_world::Result<()> {
        self.action(w, format!("queue:{q}"));
        Ok(())
    }
    fn manager_13c_update(&mut self, w: &mut World) -> aa_world::Result<()> {
        self.action(w, "manager".into());
        Ok(())
    }
    fn actor_90(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<()> {
        self.action(w, format!("actor:{}:90", self.names[&id]));
        Ok(())
    }
    fn actor_e0(&mut self, w: &mut World, id: ObjectId, _: bool) -> aa_world::Result<()> {
        self.action(w, format!("actor:{}:e0", self.names[&id]));
        Ok(())
    }
    fn actor_8c(&mut self, _: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        let plan = self.retain.get_mut(&id).unwrap();
        Ok(if plan.len() > 1 {
            plan.pop_front().unwrap()
        } else {
            plan[0]
        })
    }
    fn actor_bc(&mut self, _: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        Ok(self.classification[&id] == 0xbc)
    }
    fn actor_c0(&mut self, _: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        Ok(self.classification[&id] == 0xc0)
    }
}
fn event(e: &Event, leaves: &Leaves) -> Value {
    let mut out = json!({"phase":e.phase,"tick":e.tick});
    let fields = match e.operation {
        O::FogCallback { enabled } => json!({"event":"fog_callback","enabled":u8::from(enabled)}),
        O::RequestPhase { requested } => json!({"event":"request_phase","requested":requested}),
        O::FirstTickCallback { aux } => json!({"event":"first_tick_callback","aux":aux}),
        O::QueueUpdate { queue } => json!({"event":"queue_update","queue":queue}),
        O::Manager13CUpdate => json!({"event":"manager_13c_update"}),
        O::DeferredPayload { object } => {
            json!({"event":"deferred_payload","name":leaves.names[&object]})
        }
        O::Pending100Component { component } => {
            json!({"event":"pending_100_component","name":leaves.names[&component]})
        }
        O::AvailabilityLeaf { object } => {
            json!({"event":"availability_leaf","name":leaves.names[&object]})
        }
        O::RegistryUnlink { object } => {
            json!({"event":"registry_unlink","name":leaves.names[&object]})
        }
        O::RetainedUnlink { object } => {
            json!({"event":"retained_unlink","name":leaves.names[&object]})
        }
        O::ClassificationRemove { list, object } => {
            json!({"event":"classification_remove","list_offset":if list==L::Class178 {"0x178"} else {"0x188"},"name":leaves.names[&object]})
        }
        O::Actor8C { object, result } => {
            json!({"event":"actor_8c","name":leaves.names[&object],"result":result})
        }
        O::ActorBC { object, result } => {
            json!({"event":"actor_bc","name":leaves.names[&object],"result":result})
        }
        O::ActorC0 { object, result } => {
            json!({"event":"actor_c0","name":leaves.names[&object],"result":result})
        }
        O::Actor90 { object } => json!({"event":"actor_90","name":leaves.names[&object]}),
        O::ActorF0 { object, argument } => {
            json!({"event":"actor_f0","name":leaves.names[&object],"argument":u8::from(argument)})
        }
        O::ActorE0 { object, argument } => {
            json!({"event":"actor_e0","name":leaves.names[&object],"argument":u8::from(argument)})
        }
        O::RequestRemoval { .. }
        | O::ActorB4 { .. }
        | O::ResolveHandle { .. }
        | O::Actor2C { .. } => panic!("removal-request operation outside request_removal"),
    };
    out.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    out
}
fn run(name: &str) -> Value {
    let mut w = World::default();
    w.auxiliary_counter = Some(7);
    let mut c = Leaves::default();
    if name != "first_tick" {
        w.clock.tick = 7;
        w.clock.begin = FloatBits::from_f32(6.0 * 0.0625);
        w.clock.end = FloatBits::from_f32(7.0 * 0.0625);
    }
    let mut transition = None;
    match name {
        "first_tick" | "subsequent_tick" => (),
        "deferred_tail_append_during_callback" => {
            for (id, name) in [(1, "A"), (2, "B"), (3, "C")] {
                c.payload(&mut w, id, name);
            }
            w.enqueue_back(L::Deferred1A4, 1).unwrap();
            w.enqueue_back(L::Deferred1A4, 2).unwrap();
            c.actions
                .insert("payload:A".into(), vec![(L::Deferred1A4, 3, false)]);
        }
        "phase_boundaries_drain_then_retained_final_scan" => {
            c.payload(&mut w, 16, "P");
            c.payload(&mut w, 17, "Q");
            c.actor(&mut w, 1, "A", &[false], 0xbc, 0);
            c.actor(&mut w, 2, "B", &[true, false], 0xc0, 0x20);
            w.enqueue_back(L::Class188, 1).unwrap();
            w.enqueue_back(L::Class178, 2).unwrap();
            c.actions.insert(
                "queue:0".into(),
                vec![(L::Pending100, 16, false), (L::Removal108, 1, false)],
            );
            c.actions.insert(
                "manager".into(),
                vec![(L::Pending100, 17, false), (L::Removal108, 2, false)],
            );
        }
        "retained_insertions_are_front_order" => {
            for (id, name) in [(1, "A"), (2, "B")] {
                c.actor(&mut w, id, name, &[true], 0, 0);
                w.enqueue_back(L::Removal108, id).unwrap();
            }
        }
        "retained_scan_preserves_next_before_deleting_current" => {
            for (id, name) in [(1, "A"), (2, "B"), (3, "C")] {
                c.actor(&mut w, id, name, &[id == 2], 0, 0);
                w.enqueue_back(L::Retained110, id).unwrap();
            }
        }
        "new_head_during_retained_cleanup_waits" => {
            for (id, name) in [(1, "A"), (2, "B"), (4, "D")] {
                c.actor(&mut w, id, name, &[id == 2], 0, 0);
            }
            w.enqueue_back(L::Retained110, 1).unwrap();
            w.enqueue_back(L::Retained110, 2).unwrap();
            c.actions
                .insert("actor:A:e0".into(), vec![(L::Retained110, 4, true)]);
        }
        "removal_callback_addition_drained_same_transition" => {
            c.actor(&mut w, 1, "A", &[false], 0, 0);
            c.actor(&mut w, 2, "B", &[false], 0, 0);
            w.enqueue_back(L::Removal108, 1).unwrap();
            c.actions
                .insert("actor:A:90".into(), vec![(L::Removal108, 2, false)]);
        }
        other if other.starts_with("transition_") => {
            let fields: Vec<_> = other.split('_').collect();
            w.phase = fields[1].parse().unwrap();
            transition = Some(fields[3].parse().unwrap());
            c.payload(&mut w, 16, "P");
            c.actor(&mut w, 1, "A", &[false], 0, 0);
            w.enqueue_back(L::Pending100, 16).unwrap();
            w.enqueue_back(L::Removal108, 1).unwrap();
        }
        _ => panic!("unknown external oracle case {name}"),
    }
    if let Some(requested) = transition {
        w.transition(requested, &mut c).unwrap();
    } else {
        w.step(&mut c).unwrap();
    }
    let mut lists = serde_json::Map::new();
    for (key, list) in [
        ("0x100", L::Pending100),
        ("0x108", L::Removal108),
        ("0x110", L::Retained110),
        ("0x178", L::Class178),
        ("0x188", L::Class188),
        ("0x1a4", L::Deferred1A4),
    ] {
        lists.insert(
            key.into(),
            json!(
                w.list_values(list)
                    .iter()
                    .map(|id| c.names[id])
                    .collect::<Vec<_>>()
            ),
        );
    }
    let flags: BTreeMap<_, _> = w
        .object_flags
        .iter()
        .map(|(id, v)| (c.names[id], *v))
        .collect();
    json!({"events":w.events().iter().map(|e|event(e,&c)).collect::<Vec<_>>(),"final":{
        "phase":w.phase,"tick":w.clock.tick,"begin_bits":w.clock.begin.0,"end_bits":w.clock.end.0,"aux":w.auxiliary_counter,"lists":lists,"actor_flags":flags}})
}
#[test]
fn original_x86_dispatch_and_mutation_traces() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    assert_eq!(oracle["passed"], true);
    assert_eq!(oracle["case_count"], 12);
    for case in oracle["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let actual = run(name);
        let expected: Vec<_> = case["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "free_list_node")
            .cloned()
            .collect();
        assert_eq!(
            actual["events"],
            json!(expected),
            "semantic callback trace: {name}"
        );
        assert_eq!(
            actual["final"], case["final"],
            "complete dispatch state: {name}"
        );
    }
}
