//! Replays research/phase8-orders/native-actor-command.json: original
//! KKC_GActorCommand::Process 0x4D9B60 on synthetic actor graphs (every
//! virtual call in native order, the returned byte and the null-actor log).
use aa_orders::commands::{deliver_actor_command, CommandRouting};
use serde_json::{json, Value};

struct Graph<'a> {
    actors: &'a [Value],
    trace: Vec<Value>,
}
impl CommandRouting for Graph<'_> {
    type Actor = usize;
    type Handler = usize;
    fn handler(&mut self, actor: usize, slot: u32) -> Option<usize> {
        self.actors[actor]["handlers"].get(slot.to_string()).map(|_| actor)
    }
    fn accepts(&mut self, actor: usize) -> bool {
        self.trace.push(json!({"call": "accepts", "actor": actor, "type_ok": true}));
        self.actors[actor]["accepts"].as_bool().unwrap()
    }
    fn handle(&mut self, handler: usize) -> u8 {
        self.trace.push(json!({"call": "handle", "actor": handler, "command_ok": true}));
        self.actors[handler]["result"].as_u64().unwrap() as u8
    }
    fn redirect(&mut self, actor: usize) -> Option<Option<usize>> {
        self.trace.push(json!({"call": "has_redirect", "actor": actor}));
        let r = &self.actors[actor]["redirect"];
        if r == &Value::Bool(false) {
            return None;
        }
        self.trace.push(json!({"call": "redirect", "actor": actor}));
        Some(r.as_u64().map(|x| x as usize))
    }
    fn container_member(&mut self, actor: usize) -> Option<usize> {
        match &self.actors[actor]["container"] {
            Value::Null => None,
            Value::String(s) if s == "null" => None,
            v => Some(v.as_u64().unwrap() as usize),
        }
    }
}

#[test]
fn native_actor_command_delivery() {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-orders/native-actor-command.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let cases = v["cases"].as_array().unwrap();
    let (mut handled, mut logged) = (0, 0);
    for c in cases {
        let actors = c["actors"].as_array().unwrap();
        let mut g = Graph { actors, trace: Vec::new() };
        let start = c["start"].as_u64().map(|x| x as usize);
        let d = deliver_actor_command(&mut g, c["slot"].as_u64().unwrap() as u32, start).unwrap();
        if d.null_actor_logged {
            g.trace.push(json!({"call": "log", "line": 0xf7}));
            logged += 1;
        }
        assert_eq!(Value::Array(g.trace), c["trace"], "{c}");
        assert_eq!(u64::from(d.result), c["result"].as_u64().unwrap(), "{c}");
        handled += usize::from(c["trace"].as_array().unwrap().iter().any(|t| t["call"] == "handle"));
    }
    assert_eq!(cases.len(), 300);
    assert!(handled > 50 && logged > 30, "{handled} {logged}");
}

#[test]
fn cycle_is_a_typed_error() {
    let actors = vec![
        json!({"handlers": {}, "accepts": true, "result": 1, "redirect": 1, "container": null}),
        json!({"handlers": {}, "accepts": true, "result": 1, "redirect": false, "container": 0}),
    ];
    let mut g = Graph { actors: &actors, trace: Vec::new() };
    assert!(deliver_actor_command(&mut g, 0, Some(0)).is_err());
}
