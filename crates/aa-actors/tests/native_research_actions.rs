use aa_actors::{
    ActorId,
    ability_flags::{Error, Result},
    research_actions::{self, World},
};
use serde_json::{Value, json};
use std::cell::RefCell;

struct Probe {
    input: Value,
    actions: u64,
    trace: RefCell<Vec<Value>>,
}
impl World for Probe {
    fn research_owner(&self, _: ActorId) -> Result<ActorId> {
        Ok(ActorId::from_raw(1))
    }
    fn state_bits(&self, _: ActorId) -> Result<u64> {
        Ok(self.input["state"].as_u64().unwrap())
    }
    fn state_type(&self, index: u8) -> Result<u32> {
        let value = self.input["types"][index as usize].as_u64().unwrap() as u32;
        self.trace.borrow_mut().push(json!(["type", index, value]));
        Ok(value)
    }
    fn research_items_present(&self, _: ActorId) -> Result<bool> {
        if !self.input["source_present"].as_bool().unwrap() {
            return Err(Error::Missing("completed source"));
        }
        let head = self.input["head"].as_u64().unwrap();
        self.trace.borrow_mut().push(json!(["head", head]));
        Ok(head != 0)
    }
    fn action_bits(&self, _: ActorId) -> Result<u64> {
        Ok(self.actions)
    }
    fn set_action_bits(&mut self, _: ActorId, bits: u64) -> Result<()> {
        let trace = self.trace.get_mut();
        trace.push(json!(["write", 0xf0, bits as u32]));
        trace.push(json!(["write", 0xf4, (bits >> 32) as u32]));
        self.actions = bits;
        Ok(())
    }
}

#[test]
fn research_contributor_matches_original_and_v14_short_circuit_and_stores() {
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../research/skirmish-20261016-research-actions/native-research-actions.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 552);
    for row in rows {
        let mut probe = Probe {
            input: row["input"].clone(),
            actions: row["input"]["actions"].as_u64().unwrap(),
            trace: RefCell::new(Vec::new()),
        };
        let result = research_actions::contribute(&mut probe, ActorId::from_raw(1));
        assert_eq!(
            result.is_ok(),
            row["output"]["outcome"] == "ok",
            "{row}: {result:?}"
        );
        assert_eq!(json!(probe.actions), row["output"]["actions"], "{row}");
        assert_eq!(
            json!(probe.trace.into_inner()),
            row["output"]["trace"],
            "{row}"
        );
    }
}
