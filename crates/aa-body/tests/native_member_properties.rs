//! Replays research/phase8-combat/native-member-properties-fixtures.json:
//! original 4D0660 under Unicorn with 5035E0 and 5817A0 recorded.
use aa_body::{Result, member_properties::*};
use serde_json::{Value, json};

const ACTOR: u32 = 1;
const SOURCE: u32 = 2;

struct Host<'a> {
    case: &'a Value,
    events: Vec<Value>,
}

impl Host<'_> {
    fn definition(&self, handle: u32) -> &Value {
        &self.case["definitions"][(handle - 100) as usize]
    }
}

impl World for Host<'_> {
    fn properties(&mut self, actor: u32) -> Result<Option<u32>> {
        let key = if actor == ACTOR { "actor_properties" } else { "source_properties" };
        Ok(self.case[key].as_bool().unwrap().then_some(actor + 10))
    }
    fn instance_count(&mut self, list: u32) -> Result<i32> {
        assert_eq!(list, SOURCE + 10);
        Ok(self.case["count"].as_i64().unwrap() as i32)
    }
    fn instance_definition(&mut self, _: u32, index: i32) -> Result<Option<u32>> {
        Ok((!self.case["definitions"][index as usize].is_null()).then_some(100 + index as u32))
    }
    fn definition_bytes(&mut self, definition: u32) -> Result<[u8; 2]> {
        let d = self.definition(definition);
        Ok([d["b40"].as_u64().unwrap() as u8, d["b41"].as_u64().unwrap() as u8])
    }
    fn applicable(&mut self, actor: u32, definition: u32) -> Result<bool> {
        assert_eq!(actor, ACTOR);
        self.events.push(json!(["applicable", definition - 100]));
        Ok(self.definition(definition)["applicable"].as_u64().unwrap() & 0xFF != 0)
    }
    fn add(&mut self, properties: u32, definition: u32, kind: u32, source: Option<u32>) -> Result<()> {
        assert_eq!(properties, ACTOR + 10);
        let source = match source {
            Some(SOURCE) => json!("source"),
            Some(other) => panic!("unexpected source {other}"),
            None => json!(0),
        };
        self.events.push(json!(["add", definition - 100, kind, source]));
        Ok(())
    }
}

#[test]
fn original_member_properties_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-member-properties-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 300);
    for (i, case) in cases.iter().enumerate() {
        let mut host = Host { case, events: Vec::new() };
        copy_properties(
            &mut host,
            ACTOR,
            SOURCE,
            case["no_source"].as_u64().unwrap() & 0xFF != 0,
            case["need40"].as_bool().unwrap(),
            case["need41"].as_bool().unwrap(),
        )
        .unwrap();
        assert_eq!(Value::Array(host.events), case["events"], "case {i}");
    }
}
