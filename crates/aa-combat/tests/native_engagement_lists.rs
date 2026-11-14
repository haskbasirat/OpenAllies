//! Replays research/phase8-combat/native-engagement-lists-fixtures.json:
//! original 4D3990, 4D3A20, 4D0D70 (4F1430), 4D4EF0, 4D4E70 and 4D4F50 with
//! the real 4D0900 node insert under Unicorn. After every op the +118, +120
//! and caller lists (head to tail) and the recorded seam calls must match.
use aa_combat::{Result, engagement_lists::*};
use serde_json::{Value, json};

struct Host<'a> {
    world: &'a Value,
    events: Vec<Value>,
}

impl Host<'_> {
    fn thing(&self, handle: u32) -> &Value {
        &self.world["things"][(handle - 1) as usize]
    }
}

fn handle(v: &Value) -> Option<u32> {
    v.as_u64().map(|i| i as u32 + 1)
}

fn index(handle: Option<u32>) -> Value {
    handle.map_or(Value::Null, |h| json!(h - 1))
}

impl World for Host<'_> {
    fn gtid(&mut self, thing: u32) -> Result<u32> {
        Ok(self.thing(thing)["gtid"].as_u64().unwrap() as u32)
    }
    fn notices_enabled(&mut self) -> Result<bool> {
        Ok(self.world["notices"].as_bool() == Some(true))
    }
    fn kingdom_object(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok(self.thing(actor)["kingdom_object"].as_bool().unwrap().then_some(actor))
    }
    fn notice_target(&mut self, kingdom_object: Option<u32>) -> Result<Option<u32>> {
        self.events.push(json!(["notice_target", index(kingdom_object)]));
        Ok(self.world["notice_target"].as_bool().unwrap().then_some(0x77))
    }
    fn notice_attacked(&mut self, notice: u32, actor: u32) -> Result<()> {
        assert_eq!(notice, 0x77);
        self.events.push(json!(["notice_attacked", actor - 1]));
        Ok(())
    }
    fn organization(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok((!self.thing(actor)["org"].is_null()).then_some(actor))
    }
    fn organization_attacked(&mut self, organization: u32, attacker: Option<u32>) -> Result<()> {
        self.events.push(json!(["organization_attacked", organization - 1, index(attacker)]));
        Ok(())
    }
    fn member_count(&mut self, organization: u32) -> Result<i32> {
        Ok(self.thing(organization)["org"]["count"].as_i64().unwrap() as i32)
    }
    fn member(&mut self, organization: u32, i: i32) -> Result<Option<u32>> {
        Ok(handle(&self.thing(organization)["org"]["slots"][i as usize]))
    }
    fn component98(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok((!self.thing(actor)["c98"].is_null()).then_some(actor))
    }
    fn component98_count(&mut self, component: u32) -> Result<i32> {
        Ok(self.thing(component)["c98"]["count"].as_i64().unwrap() as i32)
    }
    fn component98_entry(&mut self, component: u32, i: i32) -> Result<Option<u32>> {
        Ok(handle(&self.thing(component)["c98"]["entries"][i as usize]))
    }
    fn component88(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok((!self.thing(actor)["c88"].is_null()).then_some(actor))
    }
    fn component88_nodes(&mut self, component: u32) -> Result<Vec<Option<u32>>> {
        // Holder objects are numbered (component << 8) | (node + 1).
        Ok(self.thing(component)["c88"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, n)| (!n.is_null()).then_some((component << 8) | (i as u32 + 1)))
            .collect())
    }
    fn gtid_nodes(&mut self, object: u32) -> Result<Vec<u32>> {
        let node = &self.thing(object >> 8)["c88"][((object & 0xFF) - 1) as usize];
        Ok(node.as_array().unwrap().iter().map(|g| g.as_u64().unwrap() as u32).collect())
    }
    fn registry_generation(&mut self, i: u32) -> Result<u16> {
        Ok(self.world["registry"][i.to_string()]["generation"].as_u64().unwrap_or(0) as u16)
    }
    fn registry_object(&mut self, i: u32) -> Result<Option<u32>> {
        Ok(handle(&self.world["registry"][i.to_string()]["object"]))
    }
}

fn check(list: &List, expected: &Value, what: &str) {
    assert_eq!(expected["tail_is_last"], true, "{what}");
    let values: Vec<u32> = list.values.iter().copied().collect();
    assert_eq!(json!(values), expected["values"], "{what}");
}

#[test]
fn original_engagement_lists_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-engagement-lists-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 980);
    let (mut list118, mut list120, mut caller) = (List::default(), List::default(), List::default());
    for (i, case) in cases.iter().enumerate() {
        if case["step"] == 0 {
            (list118, list120, caller) = (List::default(), List::default(), List::default());
        }
        let world = &doc["worlds"][case["world"].as_u64().unwrap() as usize];
        let mut host = Host { world, events: Vec::new() };
        let actor = case["actor"].as_u64().unwrap() as u32 + 1;
        let thing = handle(&case["thing"]);
        match case["op"].as_str().unwrap() {
            "attacker" => add_attacker(&mut host, actor, thing, &mut list118).unwrap(),
            "add120" => add_to_120(&mut host, thing, &mut list120).unwrap(),
            "remove120" => {
                let removed = remove_from_120(&mut host, thing, &mut list120).unwrap();
                assert_eq!(json!(removed as u8), case["result"], "case {i} result");
            }
            "collect98" => collect_component98(&mut host, &mut caller, thing.unwrap()).unwrap(),
            "collect_members" => collect_members(&mut host, &mut caller, thing.unwrap()).unwrap(),
            "collect88" => {
                let flag = case["flag"].as_u64().unwrap() & 0xFF != 0;
                collect_component88(&mut host, &mut caller, thing.unwrap(), flag).unwrap()
            }
            other => panic!("{other}"),
        }
        assert_eq!(Value::Array(host.events), case["events"], "case {i} events");
        check(&list118, &case["list118"], &format!("case {i} +118"));
        check(&list120, &case["list120"], &format!("case {i} +120"));
        check(&caller, &case["caller"], &format!("case {i} caller"));
    }
}
