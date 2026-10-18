use aa_mind::{
    Arena, Error, MindId, Result, StateId, World, activate, clear, push, replace_all, replace_top,
};
use serde_json::{Value, json};

struct Host {
    events: Vec<Value>,
    mutations: Vec<Value>,
    can: bool,
    fail: Option<&'static str>,
}
impl Host {
    fn event(
        &mut self,
        event: &str,
        identity: u32,
        extra: &[u32],
        arena: &mut Arena,
    ) -> Result<()> {
        let mut values = vec![json!(event), json!(identity)];
        values.extend(extra.iter().map(|n| json!(n)));
        values.push(json!(arena));
        self.events.push(Value::Array(values));
        if self.fail == Some(event) {
            return Err(Error::World(format!("fixture {event} failure")));
        }
        if self.mutations.first().is_some_and(|m| m["event"] == event) {
            let m = self.mutations.remove(0);
            let mut value = serde_json::to_value(&*arena).unwrap();
            if let Some(states) = m["states"].as_object() {
                for (id, changes) in states {
                    for (key, v) in changes.as_object().unwrap() {
                        value["states"][id][key] = v.clone();
                    }
                }
            }
            if let Some(stacks) = m["stacks"].as_object() {
                for (id, stack) in stacks {
                    value["minds"][id]["stack"] = stack.clone();
                }
            }
            *arena = serde_json::from_value(value).unwrap();
        }
        Ok(())
    }
}
impl World for Host {
    fn can_activate(&mut self, s: StateId, a: &mut Arena) -> Result<bool> {
        self.event("can", s.0, &[], a)?;
        Ok(self.can)
    }
    fn enter(&mut self, s: StateId, a: &mut Arena) -> Result<()> {
        self.event("enter", s.0, &[], a)
    }
    fn leave(&mut self, s: StateId, a: &mut Arena) -> Result<()> {
        self.event("leave", s.0, &[], a)
    }
    fn schedule(&mut self, m: MindId, d: u32, v: u32, a: &mut Arena) -> Result<()> {
        self.event("schedule", m.0, &[d, v], a)
    }
    fn reset_default(&mut self, m: MindId, a: &mut Arena) -> Result<()> {
        self.event("default", m.0, &[], a)
    }
    fn state_changed(&mut self, m: MindId, a: &mut Arena) -> Result<()> {
        self.event("changed", m.0, &[], a)
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-mind/native-lifecycle.json"
    ))
    .unwrap()
}
#[test]
fn original_stack_transitions_and_mutable_callback_order() {
    let doc = fixture();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 74);
    for row in cases {
        let c = &row["input"];
        let mut arena: Arena = serde_json::from_value(c["arena"].clone()).unwrap();
        let mut host = Host {
            events: vec![],
            mutations: serde_json::from_value(c["mutations"].clone()).unwrap(),
            can: c["can_activate"].as_bool().unwrap(),
            fail: None,
        };
        let accepted = match c["operation"].as_str().unwrap() {
            "activate" => Some(
                activate(
                    StateId(4),
                    c["mode"].as_u64().unwrap() as u32,
                    &mut arena,
                    &mut host,
                )
                .unwrap(),
            ),
            op => {
                match op {
                    "push" => push(MindId(1), StateId(4), &mut arena, &mut host),
                    "replace_top" => replace_top(MindId(1), StateId(4), &mut arena, &mut host),
                    "replace_all" => replace_all(MindId(1), StateId(4), &mut arena, &mut host),
                    "clear" => clear(MindId(1), &mut arena, &mut host),
                    _ => unreachable!(),
                }
                .unwrap();
                None
            }
        };
        assert_eq!(
            json!({"accepted":accepted,"arena":arena,"events":host.events}),
            row["output"],
            "{}",
            c["name"]
        );
        assert!(host.mutations.is_empty());
    }
}
#[test]
fn failed_callbacks_preserve_committed_transitions_without_fake_completion() {
    let doc = fixture();
    let mut arena: Arena =
        serde_json::from_value(doc["cases"][0]["input"]["arena"].clone()).unwrap();
    let mut host = Host {
        events: vec![],
        mutations: vec![],
        can: true,
        fail: Some("schedule"),
    };
    assert!(push(MindId(1), StateId(4), &mut arena, &mut host).is_err());
    assert_eq!(
        arena.minds[&MindId(1)].stack.front(),
        Some(&Some(StateId(4)))
    );
    assert_eq!(arena.states[&StateId(4)].status, 14);
    assert!(
        host.events
            .iter()
            .all(|e| e[0] != "enter" && e[0] != "changed")
    );
    host.fail = None;
    host.mutations = vec![json!({"event":"schedule","stacks":{"1":[]}})];
    assert!(matches!(
        push(MindId(1), StateId(4), &mut arena, &mut host),
        Err(Error::MissingActiveState)
    ));
}
