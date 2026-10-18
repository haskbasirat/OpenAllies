use aa_actors::{
    ActorId,
    ability_flags::{Error, Result},
    structure_actions::{self, StructureReference, World},
};
use serde_json::{Value, json};
fn id(n: u32) -> ActorId {
    ActorId::from_raw(n)
}
struct Fixture {
    c: Value,
    owner: ActorId,
    parent: Option<ActorId>,
    actions: [u64; 3],
    queue: u32,
    trace: Vec<Value>,
    missing_queue: bool,
}
impl Fixture {
    fn new(c: &Value) -> Self {
        Self {
            c: c.clone(),
            owner: id(1),
            parent: c["parent"].as_bool().unwrap().then_some(id(3)),
            actions: [
                c["actions"].as_u64().unwrap(),
                c["other_actions"].as_u64().unwrap(),
                c["parent_actions"].as_u64().unwrap(),
            ],
            queue: c["queue"].as_u64().unwrap() as u32,
            trace: vec![],
            missing_queue: false,
        }
    }
}
impl World for Fixture {
    fn structure_owner(&self, _: ActorId) -> Result<ActorId> {
        Ok(self.owner)
    }
    fn structure_reference(&self, actor: ActorId) -> Result<Option<StructureReference>> {
        Ok(
            if actor == id(1) && self.c["structure"].as_bool().unwrap() {
                Some(StructureReference {
                    owner: self.owner,
                    parent: self.parent,
                })
            } else if actor == id(2) {
                Some(StructureReference {
                    owner: id(2),
                    parent: None,
                })
            } else {
                None
            },
        )
    }
    fn settlement_reference(&self, actor: ActorId) -> Result<Option<Option<ActorId>>> {
        Ok(
            if actor == id(3) && self.c["settlement"].as_bool().unwrap() {
                Some(match self.c["center"].as_str().unwrap() {
                    "self" => Some(id(1)),
                    "other" => Some(id(2)),
                    _ => None,
                })
            } else {
                None
            },
        )
    }
    fn action_bits(&self, actor: ActorId) -> Result<u64> {
        Ok(self.actions[(actor.raw() - 1) as usize])
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()> {
        self.actions[(actor.raw() - 1) as usize] = bits;
        self.trace.push(json!(["write", actor.raw(), bits]));
        Ok(())
    }
    fn queue_nonempty(&self, _: ActorId) -> Result<bool> {
        if self.missing_queue {
            Err(Error::Missing("queue"))
        } else {
            Ok(self.queue != 0)
        }
    }
    fn rebuild_actions(&mut self, actor: ActorId) -> Result<()> {
        assert_eq!(actor, id(3));
        self.trace.push(json!(["rebuild", 3]));
        self.actions[2] = self.c["rebuilt_actions"].as_u64().unwrap();
        if self.c["rebind"] == true {
            self.owner = id(2);
        }
        if let Some(n) = self.c["change_queue"].as_u64() {
            self.queue = n as u32;
        }
        if let Some(n) = self.c["change_actions"].as_u64() {
            self.actions[0] = n;
        }
        if self.c["change_parent"] == true {
            self.parent = Some(id(2));
        }
        if self.c["fail"] == true {
            Err(Error::World(
                "injected parent rebuild dependency failure".into(),
            ))
        } else {
            Ok(())
        }
    }
}
#[test]
fn original_and_v14_structure_actions_match_owner_rereads_and_failure_prefixes() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-structure-actions/native-structure-actions.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 498);
    for row in rows {
        let mut world = Fixture::new(&row["input"]);
        let output = &row["output"];
        let result = structure_actions::contribute(&mut world, id(1));
        assert_eq!(result.is_ok(), output["outcome"] == "ok", "{row}");
        assert_eq!(
            world.actions,
            [
                output["actions"].as_u64().unwrap(),
                output["other_actions"].as_u64().unwrap(),
                output["parent_actions"].as_u64().unwrap()
            ],
            "{row}"
        );
        assert_eq!(json!(world.trace), output["trace"], "{row}");
        assert_eq!(world.owner.raw(), output["owner"].as_u64().unwrap() as u32);
        assert_eq!(world.queue, output["queue"].as_u64().unwrap() as u32);
    }
}
#[test]
fn missing_queue_preserves_successful_parent_merge_prefix() {
    let c = json!({"structure":true,"parent":true,"settlement":true,"center":"self","queue":1,
        "actions":0xffffffffffffffffu64,"other_actions":0u64,"parent_actions":0u64,"rebuilt_actions":1u64<<13});
    let mut world = Fixture::new(&c);
    world.missing_queue = true;
    let error = structure_actions::contribute(&mut world, id(1)).unwrap_err();
    assert!(error.to_string().contains("queue"));
    assert_eq!(world.trace.len(), 3);
    assert_eq!(world.trace[0][0], "rebuild");
    assert_eq!(world.actions[0], u64::MAX, "parent merge may restore bit13");
}
