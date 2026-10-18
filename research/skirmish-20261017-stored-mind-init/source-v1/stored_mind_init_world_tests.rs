//! Explicit child-contract probes of the SAME Regiment reset coordinator.
use super::*;
use serde_json::{Value, json};
struct Probe<'a> {
    case: &'a Value,
    trace: Vec<Value>,
    guard: bool,
    failure: Option<&'static str>,
}
impl Probe<'_> {
    fn child(&mut self, name: &'static str, args: Vec<Value>) -> aa_mind::Result<()> {
        let mut row = vec![json!(name)];
        row.extend(args);
        self.trace.push(json!(row));
        if self.case["fail"] == name {
            self.failure = Some(name);
            return required(name);
        }
        Ok(())
    }
}
impl RegimentResetWorld for Probe<'_> {
    fn clear_current(&mut self) -> aa_mind::Result<()> {
        self.trace.push(json!(["clear"]));
        Ok(())
    }
    fn resetting(&mut self) -> bool {
        self.trace.push(json!(["guard_read"]));
        self.guard
    }
    fn store_resetting(&mut self, value: bool) {
        self.trace.push(json!(["guard_store", value]));
        self.guard = value;
    }
    fn resume(&mut self) -> aa_mind::Result<bool> {
        self.child("resume", vec![])?;
        Ok(self.case["resume"].as_bool().unwrap())
    }
    fn home(&mut self) -> aa_mind::Result<Option<ActorId>> {
        self.trace.push(json!(["home"]));
        Ok((self.case["home"] != "none").then(|| ActorId::from_raw(0x01000001)))
    }
    fn home_removed(&mut self, _: ActorId) -> aa_mind::Result<bool> {
        self.trace.push(json!(["home_removed"]));
        Ok(self.case["home"] == "removed")
    }
    fn join(&mut self, _: ActorId) -> aa_mind::Result<()> {
        self.child("join", vec![json!(2), json!(true), json!(0)])
    }
    fn denizen_add(&mut self, _: ActorId) -> aa_mind::Result<()> {
        self.child("denizen", vec![json!(true)])
    }
    fn rampages(&mut self) -> aa_mind::Result<bool> {
        self.trace.push(json!(["rampages"]));
        Ok(self.case["rampages"].as_bool().unwrap())
    }
    fn activate(&mut self, offset: u32) -> aa_mind::Result<()> {
        self.child("activate", vec![json!(offset), json!(2)])
    }
    fn schedule(&mut self) -> aa_mind::Result<()> {
        self.child("schedule", vec![json!(0x3f19999au32), json!(0)])
    }
}
#[test]
fn native_regiment_reset_parent_retains_guard_and_child_failure_order() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-stored-mind-init/native-regiment-reset.json"
    ))
    .unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 864);
    for row in rows {
        let c = &row["input"];
        let mut p = Probe {
            case: c,
            trace: Vec::new(),
            guard: c["guard"].as_bool().unwrap(),
            failure: None,
        };
        let result = reset_regiment(&mut p);
        assert_eq!(result.is_err(), p.failure.is_some());
        assert_eq!(
            json!({"trace":p.trace,"guard":p.guard,"current_command":0,"failure":p.failure}),
            row["output"],
            "{c}"
        );
    }
}
#[test]
fn fresh_application_runtime_latch_is_retained_in_the_same_owner() {
    let mut runtime = mind_world::Runtime::from_fresh_application_interval();
    assert!(!runtime.regiment_resetting());
    runtime.set_regiment_resetting(true);
    //The runtime is moved, not reinitialized per actor or callback.
    let mut same = runtime;
    assert!(same.regiment_resetting());
    same.set_regiment_resetting(false);
    assert!(!same.regiment_resetting());
}

impl mind_world::ElementResetWorld for Probe<'_> {
    fn resetting(&mut self) -> bool {
        self.trace.push(json!(["guard_read"]));
        self.guard
    }
    fn store_resetting(&mut self, value: bool) {
        self.trace.push(json!(["guard_store", value]));
        self.guard = value;
    }
    fn resume(&mut self) -> aa_mind::Result<bool> {
        self.child("resume", vec![])?;
        Ok(self.case["resume"].as_bool().unwrap())
    }
    fn activate_sleep(&mut self) -> aa_mind::Result<()> {
        self.child("activate", vec![json!(0x68), json!(2)])
    }
    fn clear_current(&mut self) -> aa_mind::Result<()> {
        self.child("clear", vec![])
    }
}
#[test]
fn native_element_reset_keeps_guard_on_child_error_and_clears_before_final_command() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-stored-mind-init/native-element-reset.json"
    ))
    .unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 96);
    for row in rows {
        let c = &row["input"];
        let mut p = Probe {
            case: c,
            trace: Vec::new(),
            guard: c["guard"].as_bool().unwrap(),
            failure: None,
        };
        let result = mind_world::reset_element(&mut p);
        assert_eq!(result.is_err(), p.failure.is_some());
        assert_eq!(
            json!({"trace":p.trace,"guard":p.guard,"current_command":0,"failure":p.failure}),
            row["output"],
            "{c}"
        );
    }
}
