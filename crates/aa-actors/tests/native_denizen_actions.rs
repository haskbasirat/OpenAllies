//! Native569C70 original/v14 replay, including exact action store ordering.
use aa_actors::{
    ActorId,
    ability_flags::{ActorFields, Error, Result},
    denizen_actions::{self, World},
};
use serde_json::{Value, json};
use std::cell::RefCell;

fn id(i: u32) -> ActorId {
    ActorId::from_raw(i)
}
struct Fixture {
    c: Value,
    actions: u64,
    trace: RefCell<Vec<Value>>,
    fail_byte: Option<u32>,
}
impl Fixture {
    fn new(c: &Value) -> Self {
        Self {
            c: c.clone(),
            actions: c["actions"].as_u64().unwrap(),
            trace: RefCell::new(vec![]),
            fail_byte: None,
        }
    }
    fn flag(&self, name: &str) -> bool {
        self.c[name].as_bool().unwrap()
    }
}
impl World for Fixture {
    fn denizen_owner(&self, _: ActorId) -> Result<ActorId> {
        Ok(id(1))
    }
    fn fields(&self, _: ActorId) -> Result<ActorFields> {
        Ok(ActorFields {
            thing_flags: 0,
            state_bits: self.c["state"].as_u64().unwrap(),
            action_bits: self.actions,
        })
    }
    fn set_action_bits(&mut self, _: ActorId, bits: u64) -> Result<()> {
        self.actions = bits;
        self.trace.get_mut().push(json!(["write", bits]));
        Ok(())
    }
    fn denizen_byte(&self, _: ActorId, offset: u32) -> Result<u8> {
        if self.fail_byte == Some(offset) {
            return Err(Error::Missing("injected Denizen field failure"));
        }
        let key = match offset {
            0x20 => "byte20",
            0x21 => "byte21",
            0x38 => "flags38",
            0x3c => "flags3c",
            _ => panic!(),
        };
        Ok(self.c[key].as_u64().unwrap() as u8)
    }
    fn settlement_present(&self, _: ActorId) -> Result<bool> {
        self.trace.borrow_mut().push(json!(["call", "0x4ced40"]));
        let present = self.flag("settlement");
        if !present {
            self.trace.borrow_mut().push(json!(["call", "0x4ced60"]));
        }
        Ok(present)
    }
    fn structure_parent(&self, _: ActorId) -> Result<Option<Option<ActorId>>> {
        Ok(self
            .flag("structure")
            .then(|| self.flag("parent").then_some(id(2))))
    }
    fn wall_reference(&self, _: ActorId) -> Result<Option<Option<ActorId>>> {
        if self.flag("wall") {
            self.trace.borrow_mut().push(json!(["call", "0x58cb30"]));
        }
        Ok(self
            .flag("wall")
            .then(|| self.flag("wall_target").then_some(id(2))))
    }
}
fn fixture() -> Value {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-denizen-actions/native-denizen-actions.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn denizen_matches_native_flags_truth_bytes_parent_precedence_and_action_write_order() {
    let doc = fixture();
    let rows = doc["cases"].as_array().unwrap();
    for row in rows {
        let mut world = Fixture::new(&row["input"]);
        denizen_actions::contribute(&mut world, id(1)).unwrap();
        let label = format!(
            "{} {} {}",
            row["image"], row["precision"], row["input"]["name"]
        );
        assert_eq!(
            world.actions,
            row["output"]["actions"].as_u64().unwrap(),
            "{label}"
        );
        assert_eq!(
            json!(world.trace.into_inner()),
            row["output"]["trace"],
            "{label}"
        );
    }
    assert_eq!(rows.len(), 2154);
}

#[test]
fn missing_later_field_preserves_completed_stores_and_razing_never_reads_flags() {
    let doc = fixture();
    let mut world = Fixture::new(&doc["cases"][0]["input"]);
    world.fail_byte = Some(0x38);
    let initial = world.actions;
    let error = denizen_actions::contribute(&mut world, id(1)).unwrap_err();
    assert!(error.to_string().contains("injected"));
    assert_eq!(world.actions, initial & !((1 << 23) | (1 << 24)));
    world.c["state"] = json!(1 << 30);
    denizen_actions::contribute(&mut world, id(1)).unwrap();
    assert_eq!(world.actions, initial & !(0x3f << 23));
}
