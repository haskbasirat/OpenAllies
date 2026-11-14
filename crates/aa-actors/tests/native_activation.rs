//! Replays research/phase8-combat/native-activation-fixtures.json: original
//! 4CFC10 under Unicorn with recording component virtuals +44/+40/+3C (some
//! +40 callees rewrite actor +E4). Return byte, final +E4 and call order
//! must match.
use aa_actors::activation::*;
use serde_json::{Value, json};

struct Host<'a> {
    case: &'a Value,
    active: Option<u32>,
    events: Vec<Value>,
}

fn handle(v: &Value) -> Option<u32> {
    v.as_u64().map(|i| i as u32 + 1)
}

impl World for Host<'_> {
    fn actor_flags(&mut self, _: u32) -> Result<u32> {
        Ok(self.case["flags"].as_u64().unwrap() as u32)
    }
    fn active_component(&mut self, _: u32) -> Result<Option<u32>> {
        Ok(self.active)
    }
    fn set_active_component(&mut self, _: u32, component: Option<u32>) -> Result<()> {
        self.active = component;
        Ok(())
    }
    fn accepts_activation(&mut self, component: u32) -> Result<bool> {
        self.events.push(json!(["accepts", component - 1]));
        Ok(self.case["components"][(component - 1) as usize]["accepts"].as_u64().unwrap() & 0xFF != 0)
    }
    fn deactivated(&mut self, component: u32, same: bool) -> Result<()> {
        self.events.push(json!(["deactivated", component - 1, same as u8]));
        let rewrites = &self.case["components"][(component - 1) as usize]["rewrites"];
        if rewrites == "null" {
            self.active = None;
        } else if let Some(c) = handle(rewrites) {
            self.active = Some(c);
        }
        Ok(())
    }
    fn activated(&mut self, component: u32, same: bool) -> Result<()> {
        self.events.push(json!(["activated", component - 1, same as u8]));
        Ok(())
    }
}

#[test]
fn original_activation_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-activation-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 300);
    for (i, case) in cases.iter().enumerate() {
        let mut host = Host { case, active: handle(&case["current"]), events: Vec::new() };
        let result = activate(&mut host, 0x100, handle(&case["component"])).unwrap();
        assert_eq!(json!(result as u8), case["result"], "case {i}");
        assert_eq!(json!(host.active.map(|c| c - 1)), case["final"], "case {i} final");
        assert_eq!(Value::Array(host.events), case["events"], "case {i} events");
    }
}
