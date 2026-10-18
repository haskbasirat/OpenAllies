use aa_mind::{
    Arena, Error, MindId, Result, StateId,
    command::{self, Context, Definition, World},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

struct Host {
    commands: BTreeMap<u32, Value>,
    refs: BTreeMap<u32, u32>,
    events: Vec<Value>,
    mutations: Vec<Value>,
    input: Value,
    handled: usize,
    next_id: u32,
    fail: Option<&'static str>,
}
impl Host {
    fn new(input: &Value) -> Self {
        let commands: BTreeMap<u32, Value> =
            serde_json::from_value(input["commands"].clone()).unwrap();
        let mut refs = BTreeMap::new();
        let mind = &input["arena"]["minds"]["1"];
        for (id, command) in &commands {
            let count = command["external"].as_u64().unwrap() as u32
                + u32::from(mind["current_command"] == *id)
                + mind["pending_commands"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|v| **v == *id)
                    .count() as u32;
            refs.insert(*id, count);
        }
        Self {
            commands,
            refs,
            events: vec![],
            mutations: input["mutations"].as_array().unwrap().clone(),
            input: input.clone(),
            handled: 0,
            next_id: 100,
            fail: None,
        }
    }
    fn state(&self, arena: &Arena, depth: u32) -> Value {
        let mind = &arena.minds[&MindId(1)];
        json!({"current":mind.current_command,"pending":mind.pending_commands,"stack":mind.stack,"refs":self.refs,"depth":depth})
    }
    fn event(&mut self, name: &str, args: &[u32], arena: &mut Arena, depth: u32) -> Result<()> {
        let mut event = vec![json!(name)];
        event.extend(args.iter().map(|v| json!(v)));
        event.push(self.state(arena, depth));
        self.events.push(json!(event));
        if self.fail == Some(name) {
            return Err(Error::World(format!("fixture {name} failed")));
        }
        if self.mutations.first().is_some_and(|m| m["event"] == name) {
            let change = self.mutations.remove(0);
            if let Some(current) = change.get("current") {
                let id = current.as_u64().map(|v| v as u32);
                command::set_current(MindId(1), id, arena, self)?;
            }
            if let Some(stack) = change.get("stack") {
                arena.mind_mut(MindId(1))?.stack = serde_json::from_value(stack.clone()).unwrap();
            }
            if let Some(append) = change["append"].as_array() {
                for id in append {
                    let id = id.as_u64().unwrap() as u32;
                    self.retain(id)?;
                    arena.mind_mut(MindId(1))?.pending_commands.push_back(id);
                }
            }
            if let Some(kind) = change.get("original_type") {
                self.commands.get_mut(&1).unwrap()["mind_type"] = kind.clone();
            }
        }
        Ok(())
    }
}
impl World for Host {
    fn definition(&self, id: u32) -> Result<Definition> {
        let c = &self.commands[&id];
        Ok(Definition {
            component_slot: c["component_slot"].as_u64().unwrap() as u32,
            mind_type: c["mind_type"].as_u64().unwrap() as u32,
            behavior_flags: c["behavior_flags"].as_u64().unwrap() as u32,
        })
    }
    fn trailing(&self, id: u32) -> Result<bool> {
        Ok(self.commands[&id]["trailing"].as_bool().unwrap())
    }
    fn clone_owned(&mut self, id: u32) -> Result<u32> {
        let copied = self.next_id;
        self.next_id += 1;
        self.commands.insert(copied, self.commands[&id].clone());
        self.refs.insert(copied, 1);
        self.events.push(json!(["clone", id, copied]));
        Ok(copied)
    }
    fn retain(&mut self, id: u32) -> Result<()> {
        *self.refs.get_mut(&id).unwrap() += 1;
        Ok(())
    }
    fn release(&mut self, id: u32) -> Result<()> {
        if self.fail == Some("release") {
            return Err(Error::World(format!("release {id} failed")));
        }
        let count = self.refs.get_mut(&id).unwrap();
        assert!(*count > 0);
        *count -= 1;
        if *count == 0 {
            self.events.push(json!(["destroy", id]));
        }
        Ok(())
    }
    fn invoke(
        &mut self,
        _mind: MindId,
        slot: u32,
        id: u32,
        arena: &mut Arena,
        context: &mut Context,
    ) -> Result<bool> {
        self.event("handler", &[slot, id], arena, context.depth)?;
        let answers = self.input["accepted"].as_array().unwrap();
        let accepted = answers[self.handled.min(answers.len() - 1)]
            .as_bool()
            .unwrap();
        self.handled += 1;
        Ok(accepted)
    }
    fn schedule(&mut self, _mind: MindId, delay: u32, data: u32, arena: &mut Arena) -> Result<()> {
        self.event(
            "schedule",
            &[delay, data],
            arena,
            self.input["depth"].as_u64().unwrap() as u32,
        )
    }
    fn defers_rejected_command(&mut self, state: StateId, arena: &mut Arena) -> Result<bool> {
        self.event(
            "defers",
            &[state.0],
            arena,
            self.input["depth"].as_u64().unwrap() as u32,
        )?;
        Ok(self.input["defers"].as_bool().unwrap())
    }
    fn deliver_stop(&mut self, actor: u32, arena: &mut Arena) -> Result<bool> {
        self.event(
            "stop",
            &[actor],
            arena,
            self.input["depth"].as_u64().unwrap() as u32,
        )?;
        Ok(self.input["stop_result"].as_bool().unwrap())
    }
    fn actor_flags(&self, _actor: u32) -> Result<u32> {
        Ok(self.input["actor_flags"].as_u64().unwrap() as u32)
    }
    fn has_live_group(&self, _actor: u32) -> Result<bool> {
        Ok(self.input["group"].as_bool().unwrap())
    }
    fn finish_group_commands(&mut self, actor: u32, arena: &mut Arena) -> Result<()> {
        self.event(
            "group",
            &[actor],
            arena,
            self.input["depth"].as_u64().unwrap() as u32,
        )
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-mind/native-commands.json"
    ))
    .unwrap()
}

#[test]
fn original_command_dispatch_ownership_queue_and_drain() {
    let cases = fixture();
    assert_eq!(cases["cases"].as_array().unwrap().len(), 130);
    for case in cases["cases"].as_array().unwrap() {
        let input = &case["input"];
        let mut host = Host::new(input);
        let mut arena: Arena = serde_json::from_value(input["arena"].clone()).unwrap();
        let mut context = Context {
            depth: input["depth"].as_u64().unwrap() as u32,
        };
        let mind = MindId(1);
        let result = match input["operation"].as_str().unwrap() {
            "dispatch" => {
                Some(command::dispatch(mind, 1, &mut arena, &mut host, &mut context).unwrap())
            }
            "enqueue" => {
                command::enqueue(mind, 1, &mut arena, &mut host).unwrap();
                None
            }
            "clear" => {
                command::clear_pending(mind, &mut arena, &mut host).unwrap();
                None
            }
            "discard" => Some(command::discard_suspended(mind, &mut arena, &mut host).unwrap()),
            "process" => {
                Some(command::process(mind, 1, &mut arena, &mut host, &mut context).unwrap())
            }
            "resume" => {
                Some(command::resume_pending(mind, &mut arena, &mut host, &mut context).unwrap())
            }
            _ => unreachable!(),
        };
        assert_eq!(
            json!({"accepted":result,"state":host.state(&arena,context.depth),"events":host.events}),
            case["output"],
            "{}",
            input["name"]
        );
        assert!(host.mutations.is_empty());
    }
}

#[test]
fn callback_failures_preserve_owned_commands_and_release_temporary_holds() {
    let f = fixture();
    let input = &f["cases"][0]["input"];
    let mut arena: Arena = serde_json::from_value(input["arena"].clone()).unwrap();
    let mut host = Host::new(input);
    host.fail = Some("handler");
    let mut context = Context { depth: 5 };
    assert!(command::dispatch(MindId(1), 1, &mut arena, &mut host, &mut context).is_err());
    assert_eq!(context.depth, 5);
    assert_eq!(arena.minds[&MindId(1)].current_command, Some(1));
    assert_eq!(host.refs[&1], 2);
    assert_eq!(host.refs[&2], 1);
    let mut arena: Arena = serde_json::from_value(input["arena"].clone()).unwrap();
    let mut host = Host::new(input);
    host.fail = Some("handler");
    assert!(command::process(MindId(1), 1, &mut arena, &mut host, &mut context).is_err());
    assert_eq!(arena.minds[&MindId(1)].current_command, Some(100));
    assert_eq!(host.refs[&100], 1);
    assert_eq!(host.refs[&2], 1);
    let mut arena: Arena = serde_json::from_value(input["arena"].clone()).unwrap();
    let mut host = Host::new(input);
    host.fail = Some("schedule");
    assert!(command::enqueue(MindId(1), 1, &mut arena, &mut host).is_err());
    assert_eq!(arena.minds[&MindId(1)].pending_commands.back(), Some(&1));
    assert_eq!(host.refs[&1], 2);
    let mut arena: Arena = serde_json::from_value(input["arena"].clone()).unwrap();
    arena.mind_mut(MindId(1)).unwrap().current_command = None;
    let mut host = Host::new(input);
    host.fail = Some("release");
    let error =
        command::resume_pending(MindId(1), &mut arena, &mut host, &mut context).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("temporary command Some(3) remains owned")
    );
    assert_eq!(arena.minds[&MindId(1)].pending_commands.front(), Some(&3));
}
