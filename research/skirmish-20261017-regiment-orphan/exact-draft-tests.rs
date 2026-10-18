//Standalone type shim only: the unpublished coordinator uses no Arena or
//MindId fields. This avoids a runtime link during the release source freeze.
#[derive(Clone, Copy)]
pub struct MindId(pub u32);
#[derive(Debug)]
pub enum Error {
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;
#[path = "drafts/regiment_orphan.rs"]
mod native;
use serde_json::{Value, json};

fn required<T>(name: &str) -> Result<T> {
    Err(Error::World(name.into()))
}
struct Probe<'a> {
    input: &'a Value,
    reasons: Vec<u32>,
    current: usize,
    org_fields: Vec<usize>,
    vectors: Vec<Vec<Option<usize>>>,
    counts: Vec<usize>,
    events: Vec<Value>,
    writes: Vec<Value>,
    calls: u64,
}
impl<'a> Probe<'a> {
    fn new(input: &'a Value) -> Self {
        let list = |name: &str| {
            input[name]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().map(|v| v as usize))
                .collect::<Vec<_>>()
        };
        let a = list("members");
        let b = list("alternate");
        Self {
            input,
            reasons: input["reasons"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect(),
            current: input["initial_actor"].as_u64().unwrap() as usize,
            org_fields: vec![0, 0, 0, 0, 1, 1],
            counts: vec![a.len(), b.len()],
            vectors: vec![a, b.clone(), b],
            events: vec![],
            writes: vec![],
            calls: 0,
        }
    }
}
impl native::World for Probe<'_> {
    type Actor = usize;
    type Organization = usize;
    fn current_actor(&mut self, _: MindId) -> Result<usize> {
        Ok(self.current)
    }
    fn request_reason(&mut self, actor: usize, reason: u32) -> Result<()> {
        self.calls += 1;
        self.events.push(json!(["reason", actor, reason]));
        if self.input["fail_at"].as_u64() == Some(self.calls) {
            return required("reason");
        }
        if let Some(store) = native::ordinary_reason(self.reasons[actor], reason) {
            self.reasons[actor] = store;
            self.writes.push(json!([actor, store]));
        }
        if self.calls == 1 {
            if let Some(actor) = self.input["after_first_actor"].as_u64() {
                self.current = actor as usize;
            }
        }
        if self.calls == 2 {
            match self.input["mutation"].as_str().unwrap() {
                "change_org_field_only" => self.org_fields[self.current] = 1,
                "replace_vector" => self.vectors[0] = self.vectors[2].clone(),
                "shrink" => self.counts[0] = 1,
                "grow" => {
                    self.vectors[0] = self.vectors[2].clone();
                    self.counts[0] = self.vectors[0].len();
                }
                _ => (),
            }
            if let Some(actor) = self.input["after_member_actor"].as_u64() {
                self.current = actor as usize;
            }
        }
        Ok(())
    }
    fn organization(&mut self, actor: usize) -> Result<usize> {
        Ok(self.org_fields[actor])
    }
    fn member_count(&mut self, org: usize) -> Result<usize> {
        Ok(self.counts[org])
    }
    fn member(&mut self, org: usize, index: usize) -> Result<Option<usize>> {
        Ok(self.vectors[org][index])
    }
    fn death_4d7960(&mut self, actor: usize) -> Result<()> {
        self.events.push(json!(["death", actor]));
        if self.input["death_removed"] == true {
            return Ok(());
        }
        self.events.push(json!(["required_credit", actor]));
        required("4D57C0")
    }
}
#[test]
fn actual_522270_reason_writes_live_members_and_current_death_actor() {
    let fixture: Value =
        serde_json::from_str(include_str!("native-regiment-orphan-v4.json")).unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 123);
    for row in fixture["rows"].as_array().unwrap() {
        let mut p = Probe::new(&row["input"]);
        let result = native::run(&mut p, MindId(1));
        let actual = json!({"events":p.events,"writes":p.writes,"error":result.err().map(|e|match e{Error::World(s)=>s}),
        "reasons":p.reasons,"current_actor":p.current});
        assert_eq!(actual, row["output"], "{row}");
    }
}
struct Death<'a> {
    input: &'a Value,
    events: Vec<Value>,
}
impl native::DeathPrefixWorld for Death<'_> {
    type Actor = ();
    type Credit = u32;
    fn flags(&mut self, _: ()) -> Result<u32> {
        Ok(self.input["flags"].as_u64().unwrap() as u32)
    }
    fn credit_4d57c0(&mut self, _: ()) -> Result<u32> {
        self.events.push(json!("credit"));
        assert!(self.input["flags"].as_u64().unwrap() & 16 != 0 || self.input["session"] == 3);
        Ok(0)
    }
    fn remaining_4d7993(&mut self, _: (), credit: u32) -> Result<()> {
        self.events.push(json!(["after_credit", credit]));
        required("remaining_death")
    }
}
#[test]
fn actual_death_initial_removed_guard_and_proved_credit_returns() {
    let fixture: Value =
        serde_json::from_str(include_str!("native-regiment-orphan-v4.json")).unwrap();
    for row in fixture["death_prefix"].as_array().unwrap() {
        let mut p = Death {
            input: &row["input"],
            events: vec![],
        };
        let result = native::death_prefix(&mut p, ());
        let actual = json!({"events":p.events,"error":result.err().map(|e|match e{Error::World(s)=>s}),"flags":row["input"]["flags"]});
        assert_eq!(actual, row["output"], "{row}");
    }
}
