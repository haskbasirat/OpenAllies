//! Original/v14 Recruit582C40 replay, with explicit eligibility dependency.
use aa_actors::{
    ActorId,
    ability_flags::{ActorFields, Error, Result},
    recruit_actions::{self, Entry, World},
};
use serde_json::{Value, json};

fn id(i: u32) -> ActorId {
    ActorId::from_raw(i)
}
struct Fixture {
    c: Value,
    owner: ActorId,
    actions: [u64; 2],
    next: Vec<Option<usize>>,
    trace: Vec<Value>,
}
impl Fixture {
    fn new(c: &Value) -> Self {
        let len = c["entries"].as_array().unwrap().len();
        Self {
            c: c.clone(),
            owner: id(1),
            actions: [
                c["actions"].as_u64().unwrap(),
                c["parent_actions"].as_u64().unwrap(),
            ],
            next: (0..len).map(|i| (i + 1 < len).then_some(i + 1)).collect(),
            trace: vec![],
        }
    }
    fn trigger(&self, name: &str, index: usize) -> bool {
        self.c[name].as_u64() == Some(index as u64)
    }
}
impl World for Fixture {
    type Cursor = usize;
    type Definition = usize;
    type Layout = u32;
    fn recruit_owner(&self, _: ActorId) -> Result<ActorId> {
        Ok(self.owner)
    }
    fn fields(&self, actor: ActorId) -> Result<ActorFields> {
        let slot = (actor.raw() - 1) as usize;
        Ok(ActorFields {
            thing_flags: 0,
            state_bits: self.c[if slot == 0 { "state" } else { "parent_state" }]
                .as_u64()
                .unwrap(),
            action_bits: self.actions[slot],
        })
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()> {
        self.actions[(actor.raw() - 1) as usize] = bits;
        self.trace.push(json!(["write", actor.raw(), bits]));
        Ok(())
    }
    fn state_type(&self, index: u8) -> Result<u32> {
        Ok(self.c["types"][index as usize].as_u64().unwrap() as u32)
    }
    fn settlement_reference(&self, actor: ActorId) -> Result<Option<Option<ActorId>>> {
        Ok(
            (actor == id(1) && self.c["settlement"].as_bool().unwrap()).then(|| {
                self.c["settlement_target"]
                    .as_bool()
                    .unwrap()
                    .then_some(id(2))
            }),
        )
    }
    fn recruit_enabled(&self, _: ActorId, location: bool) -> Result<u8> {
        Ok(self.c[if location { "byte10" } else { "byte11" }]
            .as_u64()
            .unwrap() as u8)
    }
    fn recruit_list_head(&self, actor: ActorId) -> Result<Option<usize>> {
        Ok((actor == id(1) && !self.next.is_empty()).then_some(0))
    }
    fn recruit_entry(&self, cursor: &usize) -> Result<Entry<usize, u32>> {
        let e = &self.c["entries"][*cursor];
        Ok(Entry {
            definition: *cursor,
            location_class: e["location"].as_u64().unwrap() != 0,
            default_layout: e["layout"].as_u64().unwrap() as u32,
        })
    }
    fn next_recruit_entry(&self, cursor: &usize) -> Result<Option<usize>> {
        Ok(self.next[*cursor])
    }
    fn eligibility(&mut self, _: ActorId, definition: &usize, layout: &u32) -> Result<u32> {
        let i = *definition;
        self.trace
            .push(json!(["eligibility", i, self.owner.raw(), layout, 1, 0]));
        if self.trigger("fail_at", i) {
            return Err(Error::World("injected5827B0 dependency failure".into()));
        }
        if self.trigger("rebind_at", i) {
            self.owner = id(2);
        }
        if self.trigger("skip_at", i) {
            self.next[i] = (self.next.len() > 2).then_some(2);
        }
        if self.trigger("flip_class_at", i) {
            let e = &mut self.c["entries"][i];
            e["location"] = json!(u32::from(e["location"].as_u64().unwrap() == 0));
        }
        if self.trigger("change_actions_at", i) {
            self.actions[0] ^= 1 << 42;
        }
        Ok(self.c["entries"][i]["result"].as_u64().unwrap() as u32)
    }
}
fn fixture() -> Value {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-recruit-actions/native-recruit-actions.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn contributor_matches_original_and_v14_fields_call_order_and_failure_prefixes() {
    let doc = fixture();
    let mut count = 0;
    for row in doc["cases"].as_array().unwrap() {
        let c = &row["input"];
        let expected = &row["output"];
        let mut world = Fixture::new(c);
        let result = recruit_actions::contribute(&mut world, id(1));
        let label = format!("{} {} {}", row["image"], row["precision"], c["name"]);
        if expected["outcome"] == "ok" {
            result.unwrap_or_else(|e| panic!("{label}: {e}"));
        } else {
            assert!(result.is_err(), "{label}: expected explicit failure");
        }
        assert_eq!(
            world.actions[0],
            expected["actions"].as_u64().unwrap(),
            "{label}: owner actions"
        );
        assert_eq!(
            world.actions[1],
            expected["parent_actions"].as_u64().unwrap(),
            "{label}: rebound owner actions"
        );
        assert_eq!(
            json!(world.trace),
            expected["trace"],
            "{label}: writes/callback order"
        );
        count += 1;
    }
    assert_eq!(count, 906);
}

#[test]
fn cyclic_source_list_is_explicitly_unsupported_after_earlier_writes() {
    let doc = fixture();
    let mut world = Fixture::new(&doc["cases"][0]["input"]);
    world.next[0] = Some(0);
    let error = recruit_actions::contribute(&mut world, id(1)).unwrap_err();
    assert!(error.to_string().contains("cycle"));
    assert_eq!(world.trace.len(), 2); // bit14 then the first eligibility call.
    assert_eq!(world.trace[0][0], "write");
    assert_eq!(world.trace[1][0], "eligibility");
}
