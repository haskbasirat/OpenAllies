//! Replays research/phase8-ai/native-rout-think.json (original 521240 with
//! native 529E60/52A000/52A1F0/529F50 under Unicorn) against
//! aa_mind::cai_rout::think.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId,
    cai_rout::{RoutThinkWorld, think},
};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-rout-think.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}

struct Host {
    c: Value,
    command: Option<(u32, [u32; 2])>,
    events: Vec<Value>,
}
impl Host {
    fn snap(&self, arena: &Arena) -> Value {
        json!({
            "status": arena.states[&StateId(1)].status,
            "command": self.command.map(|(flags, point)| json!({"flags": flags, "point": point})),
        })
    }
    fn event(&mut self, name: &str, extra: Value, arena: &Arena) {
        let mut e = json!({"event": name});
        for (k, v) in extra.as_object().unwrap() {
            e[k] = v.clone();
        }
        for (k, v) in self.snap(arena).as_object().unwrap() {
            e[k] = v.clone();
        }
        self.events.push(e);
    }
}
impl RoutThinkWorld for Host {
    fn world_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["world_time"]))
    }
    fn routing_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["routing_time"]))
    }
    fn entered_time(&mut self, state: StateId) -> Result<u32> {
        assert_eq!(state, StateId(1));
        Ok(u(&self.c["entered"]))
    }
    fn end_rout(&mut self, arena: &mut Arena) -> Result<()> {
        self.event("end_rout", json!({}), arena);
        Ok(())
    }
    fn move_status(&mut self, _arena: &Arena) -> Result<u32> {
        Ok(u(&self.c["move_status"]))
    }
    fn select_destination(&mut self, arena: &mut Arena) -> Result<Option<[u32; 2]>> {
        let result = self.c["selected"].clone();
        self.event("select", json!({"result": result}), arena);
        Ok((!result.is_null()).then(|| pair(&result)))
    }
    fn activate_move(&mut self, destination: [u32; 2], arena: &mut Arena) -> Result<()> {
        let zero = pair(&self.c["zero_vector"]);
        self.event(
            "activate_move",
            json!({"args": [0, destination[0], destination[1], 0, zero[0], zero[1]]}),
            arena,
        );
        Ok(())
    }
    fn schedule(&mut self, delay_bits: u32, arena: &mut Arena) -> Result<()> {
        self.event("schedule", json!({"delay": delay_bits, "data": 0}), arena);
        Ok(())
    }
    fn has_current_command(&mut self, _arena: &Arena) -> Result<bool> {
        Ok(self.command.is_some())
    }
    fn mark_command_destination(&mut self, _arena: &mut Arena) -> Result<()> {
        let destination = pair(&self.c["move_destination"]);
        let command = self.command.as_mut().unwrap();
        command.0 |= 4;
        command.1 = destination;
        Ok(())
    }
    fn move_destination(&mut self, _arena: &Arena) -> Result<[u32; 2]> {
        Ok(pair(&self.c["move_destination"]))
    }
    fn actor_position(&mut self, _arena: &Arena) -> Result<[u32; 2]> {
        Ok(pair(&self.c["actor_position"]))
    }
    fn reached(&mut self, arena: &mut Arena) -> Result<()> {
        self.event("reached", json!({}), arena);
        Ok(())
    }
    fn move_think(&mut self, arena: &mut Arena) -> Result<()> {
        self.event("move_think", json!({}), arena);
        Ok(())
    }
}

#[test]
fn native_rout_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 228);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        match c["current"].as_bool() {
            None => {}
            Some(false) => mind.stack.push_front(Some(StateId(2))),
            Some(true) => mind.stack.push_front(Some(StateId(1))),
        }
        arena.minds.insert(MindId(1), mind);
        for (id, status) in [(1, u(&c["status"])), (2, 2)] {
            arena.states.insert(
                StateId(id),
                State {
                    parent: MindId(1),
                    discard_on_suspend: false,
                    status,
                },
            );
        }
        let mut host = Host {
            c: c.clone(),
            command: (!c["command"].is_null())
                .then(|| (u(&c["command"]["flags"]), [0x1111_1111, 0x2222_2222])),
            events: vec![],
        };
        let wrong = think(StateId(1), &mut arena, &mut host, p).unwrap();
        let native: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "diagnostic_log")
            .cloned()
            .collect();
        let logged = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["event"] == "diagnostic_log");
        assert_eq!(wrong, logged, "{name}");
        assert_eq!(host.events, native, "{name}");
        assert_eq!(
            json!(arena.states[&StateId(1)].status),
            o["status"],
            "{name}"
        );
        assert_eq!(host.snap(&arena)["command"], o["command"], "{name}");
    }
}
