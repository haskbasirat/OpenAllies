use aa_actors::{
    ActorId, ability_flags as flags,
    organization_recruitment::{self as recruitment, World},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::cell::RefCell;

#[derive(Clone, Deserialize)]
struct Input {
    kind: String,
    hqs: Vec<u32>,
    returns: Vec<u32>,
    reference: u32,
    valid_reference: bool,
    definition: u32,
    next_definition: u32,
    replace: Option<Vec<u32>>,
}
struct Fixture {
    input: Input,
    hqs: Vec<u32>,
    definition: u32,
    count: usize,
    calls: usize,
    events: RefCell<Vec<Value>>,
    fail_after: Option<usize>,
}
impl Fixture {
    fn new(input: Input) -> Self {
        Self {
            hqs: input.hqs.clone(),
            definition: input.definition,
            count: input.hqs.len(),
            input,
            calls: 0,
            events: RefCell::new(Vec::new()),
            fail_after: None,
        }
    }
    fn callback(&mut self, kind: &str, hq: ActorId, definition: &u32) -> flags::Result<bool> {
        self.events
            .get_mut()
            .push(json!([kind, hq.raw(), definition]));
        if self.fail_after == Some(self.calls) {
            return Err(flags::Error::Missing("fixture HQ leaf"));
        }
        self.calls += 1;
        if self.calls == 1 {
            if let Some(replacement) = &self.input.replace {
                self.hqs = replacement.clone();
                self.count = 0;
            }
            self.definition = self.input.next_definition;
        }
        Ok(self.input.returns[hq.raw() as usize - 1] & 255 != 0)
    }
}
impl World for Fixture {
    type Definition = u32;
    fn hq_count(&self, owner: ActorId) -> flags::Result<usize> {
        assert_eq!(owner.raw(), 100);
        Ok(self.count)
    }
    fn hq_at(&self, _: ActorId, index: usize) -> flags::Result<ActorId> {
        let value = self.hqs[index];
        self.events.borrow_mut().push(json!(["hq", value]));
        Ok(ActorId::from_raw(value))
    }
    fn actor_definition(&self, actor: ActorId) -> flags::Result<u32> {
        assert_eq!(actor.raw(), 200);
        self.events
            .borrow_mut()
            .push(json!(["definition", self.definition]));
        Ok(self.definition)
    }
    fn hq_can_recruit(&mut self, hq: ActorId, definition: &u32) -> flags::Result<bool> {
        self.callback("can", hq, definition)
    }
    fn hq_accepts(&mut self, hq: ActorId, definition: &u32) -> flags::Result<bool> {
        self.callback("accept", hq, definition)
    }
    fn organization_reference(&self, _: ActorId) -> flags::Result<Option<ActorId>> {
        let value = self.input.valid_reference.then_some(self.input.reference);
        self.events.borrow_mut().push(json!(["reference", value]));
        Ok(value.map(ActorId::from_raw))
    }
}

#[test]
fn original_and_v14_hq_scan_cases() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-owner-recruitment/native-owner.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 600);
    for (i, row) in rows.iter().enumerate() {
        let input: Input = serde_json::from_value(row["input"].clone()).unwrap();
        let mut world = Fixture::new(input);
        let owner = ActorId::from_raw(100);
        let value = if world.input.kind == "can" {
            let definition = world.input.definition;
            json!(recruitment::can_recruit(&mut world, owner, &definition).unwrap())
        } else {
            json!(recruitment::recruit_count(&mut world, owner, ActorId::from_raw(200)).unwrap())
        };
        assert_eq!(
            json!({"value":value,"events":world.events.into_inner()}),
            row["expected"],
            "row {i}"
        );
    }
}

#[test]
fn hq_callback_failure_keeps_completed_prefix_and_never_resolves_failed_entry() {
    let input = Input {
        kind: "count".into(),
        hqs: vec![1, 2, 3],
        returns: vec![1; 4],
        reference: 1,
        valid_reference: true,
        definition: 0,
        next_definition: 1,
        replace: None,
    };
    let mut world = Fixture::new(input);
    world.fail_after = Some(1);
    let error =
        recruitment::recruit_count(&mut world, ActorId::from_raw(100), ActorId::from_raw(200))
            .unwrap_err();
    assert!(error.to_string().contains("fixture HQ leaf"));
    assert_eq!(
        world.events.into_inner(),
        vec![
            json!(["hq", 1]),
            json!(["definition", 0]),
            json!(["accept", 1, 0]),
            json!(["reference", 1]),
            json!(["hq", 2]),
            json!(["definition", 1]),
            json!(["accept", 2, 1])
        ]
    );
}
