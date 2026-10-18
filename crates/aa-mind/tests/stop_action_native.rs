use aa_mind::{
    Arena, Result, StateId,
    action::{self, Actor, World},
};
use serde_json::{Value, json};
struct Host {
    input: Value,
    events: Vec<Value>,
}
impl Host {
    fn mutate(&mut self, event: &str, arena: &mut Arena) {
        if self.input["mutations"]
            .as_array()
            .unwrap()
            .first()
            .is_some_and(|m| m["event"] == event)
        {
            let change = self.input["mutations"].as_array_mut().unwrap().remove(0);
            if let Some(actors) = change["actors"].as_object() {
                for (key, values) in actors {
                    for (k, v) in values.as_object().unwrap() {
                        self.input["actors"][key][k] = v.clone();
                    }
                }
            }
            if let Some(states) = change["states"].as_object() {
                let mut value = serde_json::to_value(&*arena).unwrap();
                for (key, changes) in states {
                    for (k, v) in changes.as_object().unwrap() {
                        value["states"][key][k] = v.clone();
                    }
                }
                *arena = serde_json::from_value(value).unwrap();
            }
        }
    }
}
impl World for Host {
    fn actor(&self, id: u32) -> Result<Actor> {
        Ok(serde_json::from_value(self.input["actors"][id.to_string()].clone()).unwrap())
    }
    fn component_owner(&self, id: u32) -> Result<u32> {
        Ok(self.input["owners"][id.to_string()].as_u64().unwrap() as u32)
    }
    fn stop_mover(&mut self, id: u32, a: bool, b: bool, arena: &mut Arena) -> Result<()> {
        self.events.push(json!(["stop", id, a, b]));
        assert!(!a && !b);
        self.events.push(json!(["cancel", id]));
        self.mutate("cancel", arena);
        Ok(())
    }
    fn default_stance(&mut self, id: u32, arena: &mut Arena) -> Result<()> {
        self.events.push(json!(["default", id]));
        self.mutate("default", arena);
        Ok(())
    }
}
#[test]
fn original_stop_action_rereads_actor_mover_and_owner_after_cancel() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-mind/native-stop-action.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 11);
    for row in rows {
        let mut host = Host {
            input: row["input"].clone(),
            events: vec![],
        };
        let mut arena: Arena = serde_json::from_value(host.input["arena"].clone()).unwrap();
        action::stop(StateId(4), &mut arena, &mut host).unwrap();
        assert_eq!(
            json!({"events":host.events,"arena":arena,"actors":host.input["actors"]}),
            row["output"],
            "{}",
            row["input"]["name"]
        );
    }
}
