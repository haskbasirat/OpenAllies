use super::*;
use serde_json::{Value, json};
struct Host<'a> {
    input: &'a Value,
    target: u32,
    player: u32,
    events: Vec<Value>,
}
impl Host<'_> {
    fn child(&mut self, name: &str, actor: Option<u32>) -> Result<(), String> {
        self.events.push(json!(["current", self.player]));
        self.events.push(if let Some(actor) = actor {
            json!([name, self.player, actor])
        } else {
            json!([name, self.player])
        });
        if self.input["stop"].as_str() == Some(name) {
            return Err(name.into());
        }
        if name == "clear" {
            if let Some(value) = self.input["after_target"].as_u64() {
                self.target = value as u32
            }
            if self.input["after_player"] == true {
                self.player = 2
            }
        }
        Ok(())
    }
}
impl World for Host<'_> {
    type Actor = u32;
    fn target(&mut self) -> Result<Option<u32>, String> {
        Ok((self.target != 0).then_some(self.target))
    }
    fn clear(&mut self) -> Result<(), String> {
        self.child("clear", None)
    }
    fn add(&mut self, actor: u32) -> Result<(), String> {
        self.child("add", Some(actor))
    }
    fn remove(&mut self, actor: u32) -> Result<(), String> {
        self.child("remove", Some(actor))
    }
}
#[test]
fn native_selection_process_order_null_paths_rereads_and_child_error_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-player-selection-frame/native-selection-orders.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 45);
    for row in rows {
        let c = &row["input"];
        let op = match c["operation"].as_str().unwrap() {
            "Add" => Operation::Add,
            "Remove" => Operation::Remove,
            "Clear" => Operation::Clear,
            "Set" => Operation::Set,
            _ => unreachable!(),
        };
        let mut h = Host {
            input: c,
            target: c["target"].as_u64().unwrap() as u32,
            player: 1,
            events: vec![],
        };
        let result = run(&mut h, op);
        let status = result.err().unwrap_or_else(|| "complete".into());
        assert_eq!(
            json!({"status":status,"events":h.events,"target":h.target,"player":h.player}),
            row["native"],
            "{c}"
        );
    }
}
