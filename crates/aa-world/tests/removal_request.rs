//! Replays research/phase8-tick/native-removal-request.json: the original
//! 0x4F6760 request, 0x542A00 prepend, 0x4F53D0/0x4F5490 immediate paths and
//! the 0x4F65B0 drain, executed under Unicorn.
use aa_world::{Callbacks, ListKind as L, ObjectId, Operation as O, World};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const ORACLE: &str = include_str!("../../../research/phase8-tick/native-removal-request.json");

#[derive(Default)]
struct Leaves {
    names: BTreeMap<ObjectId, String>,
    ids: BTreeMap<String, ObjectId>,
    retain: BTreeMap<ObjectId, bool>,
    class: BTreeMap<ObjectId, u8>,
    b4: BTreeMap<ObjectId, bool>,
    handles: BTreeMap<ObjectId, Option<ObjectId>>,
}
impl Leaves {
    fn actor(
        &mut self,
        w: &mut World,
        name: &str,
        retain: bool,
        class: u8,
        flags: u32,
    ) -> ObjectId {
        let id = self.names.len() as ObjectId + 1;
        self.names.insert(id, name.into());
        self.ids.insert(name.into(), id);
        self.retain.insert(id, retain);
        self.class.insert(id, class);
        w.object_flags.insert(id, flags);
        id
    }
}
impl Callbacks for Leaves {
    fn fog_callback(&mut self, _: &mut World, _: bool) -> aa_world::Result<()> {
        unreachable!()
    }
    fn first_tick_callback(&mut self, _: &mut World) -> aa_world::Result<()> {
        unreachable!()
    }
    fn deferred_payload(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
        unreachable!()
    }
    fn queue_update(&mut self, _: &mut World, _: u8) -> aa_world::Result<()> {
        unreachable!()
    }
    fn manager_13c_update(&mut self, _: &mut World) -> aa_world::Result<()> {
        unreachable!()
    }
    fn pending_100_component(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
        unreachable!()
    }
    // Native fixture: B0 false; B4 (if true) meets actor+0xD4 == 0, so the
    // 0x4F4C40 leaf returns without list effects in every case.
    fn availability_leaf(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
        Ok(())
    }
    fn registry_unlink(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
        Ok(())
    }
    fn actor_8c(&mut self, _: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        Ok(self.retain[&id])
    }
    fn actor_90(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
        Ok(())
    }
    fn actor_bc(&mut self, _: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        Ok(self.class[&id] == 0xbc)
    }
    fn actor_c0(&mut self, _: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        Ok(self.class[&id] == 0xc0)
    }
    fn actor_f0(&mut self, _: &mut World, _: ObjectId, _: bool) -> aa_world::Result<()> {
        Ok(())
    }
    fn actor_e0(&mut self, _: &mut World, _: ObjectId, _: bool) -> aa_world::Result<()> {
        Ok(())
    }
    fn actor_b4(&mut self, _: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        Ok(self.b4.get(&id).copied().unwrap_or(false))
    }
    fn resolve_handle(
        &mut self,
        _: &mut World,
        id: ObjectId,
    ) -> aa_world::Result<Option<ObjectId>> {
        Ok(self.handles[&id])
    }
    fn actor_2c(&mut self, _: &mut World, _: ObjectId, _: u32) -> aa_world::Result<()> {
        Ok(())
    }
}

fn trace(w: &World, l: &Leaves) -> Vec<Value> {
    let n = |id: &ObjectId| l.names[id].clone();
    w.events()
        .iter()
        .filter_map(|e| {
            let f = match &e.operation {
                O::RequestRemoval { object, destroy } => {
                    json!({"event":"request_removal","name":n(object),"destroy":u8::from(*destroy)})
                }
                O::RequestPhase { requested } => {
                    json!({"event":"request_phase","requested":requested})
                }
                O::AvailabilityLeaf { object } => {
                    json!({"event":"availability_leaf","name":n(object)})
                }
                O::ActorBC { object, result } => {
                    json!({"event":"actor_bc","name":n(object),"result":result})
                }
                O::ActorC0 { object, result } => {
                    json!({"event":"actor_c0","name":n(object),"result":result})
                }
                O::ClassificationRemove { list, object } => json!({"event":"classification_remove",
                    "list_offset":if *list==L::Class178 {"0x178"} else {"0x188"},"name":n(object)}),
                O::Actor90 { object } => json!({"event":"actor_90","name":n(object)}),
                O::Actor8C { object, result } => {
                    json!({"event":"actor_8c","name":n(object),"result":result})
                }
                O::ActorF0 { object, argument } => {
                    json!({"event":"actor_f0","name":n(object),"argument":u8::from(*argument)})
                }
                O::RegistryUnlink { object } => json!({"event":"registry_unlink","name":n(object)}),
                O::ActorE0 { object, argument } => {
                    json!({"event":"actor_e0","name":n(object),"argument":u8::from(*argument)})
                }
                O::ActorB4 { object, result } => {
                    json!({"event":"actor_b4","name":n(object),"result":result})
                }
                O::Actor2C { object, argument } => {
                    json!({"event":"actor_2c","name":n(object),"argument":argument})
                }
                // A memory lookup natively (no call); traced only in Rust.
                O::ResolveHandle { .. } => return None,
                other => panic!("unexpected {other:?}"),
            };
            let mut out = json!({"phase": e.phase, "tick": e.tick});
            out.as_object_mut()
                .unwrap()
                .extend(f.as_object().unwrap().clone());
            Some(out)
        })
        .collect()
}

#[test]
fn removal_requests_match_native_traces_lists_and_flags() {
    let report: Value = serde_json::from_str(ORACLE).unwrap();
    assert_eq!(report["passed"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 11);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let setup = &case["setup"];
        let mut w = World::default();
        w.clock.tick = 7;
        w.phase = setup["phase"].as_u64().unwrap() as u32;
        let mode = setup["mode"].as_i64().unwrap() as i32;
        let mut l = Leaves::default();
        // Mirrors the native case construction in native_removal_request.py.
        let ids: Vec<ObjectId> = match name {
            n if n.starts_with("deferred_prepend_dedup") => {
                vec![
                    l.actor(&mut w, "A", false, 0, 0x10),
                    l.actor(&mut w, "B", false, 0, 0x20),
                ]
            }
            "deferred_lifo_then_transition_drain" => {
                let a = l.actor(&mut w, "A", false, 0xbc, 0);
                let b = l.actor(&mut w, "B", true, 0xc0, 0x4);
                let c = l.actor(&mut w, "C", false, 0, 0);
                w.enqueue_back(L::Class188, a).unwrap();
                w.enqueue_back(L::Class178, b).unwrap();
                vec![a, b, c]
            }
            n if n.starts_with("immediate_remove") => {
                let a = l.actor(&mut w, "A", true, 0xbc, 0x8);
                let b = l.actor(&mut w, "B", false, 0xc0, 0);
                let c = l.actor(&mut w, "C", false, 0, 0);
                w.enqueue_back(L::Class188, a).unwrap();
                w.enqueue_back(L::Class178, b).unwrap();
                vec![a, b, c]
            }
            "immediate_destroy_paths" => {
                let a = l.actor(&mut w, "A", false, 0xbc, 0x1);
                let b = l.actor(&mut w, "B", false, 0, 0);
                let x = l.actor(&mut w, "X", false, 0xc0, 0x40);
                let c = l.actor(&mut w, "C", false, 0, 0);
                let d = l.actor(&mut w, "D", false, 0, 0);
                w.enqueue_back(L::Class188, a).unwrap();
                w.enqueue_back(L::Class178, x).unwrap();
                for (id, h) in [(b, Some(x)), (c, None), (d, None)] {
                    l.b4.insert(id, true);
                    l.handles.insert(id, h);
                }
                vec![a, b, c, d]
            }
            "queued_object_ignored_in_immediate_phase" => vec![l.actor(&mut w, "A", false, 0, 0)],
            other => panic!("unknown case {other}"),
        };
        let _ = ids;
        let mut results = Vec::new();
        for req in setup["requests"].as_array().unwrap() {
            let who = req[0].as_str().unwrap();
            if who == "set_phase" {
                w.phase = req[1].as_u64().unwrap() as u32;
                continue;
            }
            let id = l.ids[who];
            let destroy = req[1].as_u64().unwrap() != 0;
            results.push(u8::from(
                w.request_removal(id, destroy, mode, &mut l).unwrap(),
            ));
        }
        if let Some(requested) = setup.get("then_transition") {
            w.transition(requested.as_u64().unwrap() as u32, &mut l)
                .unwrap();
        }
        assert_eq!(json!(results), case["results"], "{name}");
        assert_eq!(json!(trace(&w, &l)), case["events"], "{name}");
        let lists = |k: L| -> Vec<String> {
            w.list_values(k)
                .iter()
                .map(|id| l.names[id].clone())
                .collect()
        };
        let final_ = &case["final"];
        assert_eq!(w.phase as u64, final_["phase"].as_u64().unwrap(), "{name}");
        for (k, off) in [
            (L::Pending100, "0x100"),
            (L::Removal108, "0x108"),
            (L::Retained110, "0x110"),
            (L::Class178, "0x178"),
            (L::Class188, "0x188"),
            (L::Deferred1A4, "0x1a4"),
        ] {
            assert_eq!(json!(lists(k)), final_["lists"][off], "{name} {off}");
        }
        for (id, flags) in &w.object_flags {
            assert_eq!(json!(flags), final_["actor_flags"][&l.names[id]], "{name}");
        }
    }
}

#[test]
fn missing_destroy_callbacks_are_errors_not_success() {
    struct Bare;
    impl Callbacks for Bare {
        fn fog_callback(&mut self, _: &mut World, _: bool) -> aa_world::Result<()> {
            unreachable!()
        }
        fn first_tick_callback(&mut self, _: &mut World) -> aa_world::Result<()> {
            unreachable!()
        }
        fn deferred_payload(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
            unreachable!()
        }
        fn queue_update(&mut self, _: &mut World, _: u8) -> aa_world::Result<()> {
            unreachable!()
        }
        fn manager_13c_update(&mut self, _: &mut World) -> aa_world::Result<()> {
            unreachable!()
        }
        fn pending_100_component(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
            unreachable!()
        }
        fn availability_leaf(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
            unreachable!()
        }
        fn registry_unlink(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
            unreachable!()
        }
        fn actor_8c(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<bool> {
            unreachable!()
        }
        fn actor_90(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<()> {
            unreachable!()
        }
        fn actor_bc(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<bool> {
            unreachable!()
        }
        fn actor_c0(&mut self, _: &mut World, _: ObjectId) -> aa_world::Result<bool> {
            unreachable!()
        }
        fn actor_f0(&mut self, _: &mut World, _: ObjectId, _: bool) -> aa_world::Result<()> {
            unreachable!()
        }
        fn actor_e0(&mut self, _: &mut World, _: ObjectId, _: bool) -> aa_world::Result<()> {
            unreachable!()
        }
    }
    let mut w = World::default();
    w.phase = 3;
    w.object_flags.insert(1, 0);
    assert!(w.request_removal(1, true, 0, &mut Bare).is_err());
    // Flag 2 was committed before the failing +0xB4 virtual (native order).
    assert_eq!(w.object_flags[&1], 2);
}
