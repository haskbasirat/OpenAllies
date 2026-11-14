//! Replays research/phase8-combat/native-relation-fixtures.json: original
//! 4D4FF0 and 4D4420 (with the real 4364C0 / 437940 member iterators) under
//! Unicorn over random organizations. Return codes and the order of every
//! virtual and 57E920 call must match bit for bit.
use aa_combat::{Result, relation::*};
use serde_json::{Value, json};

struct Host<'a> {
    world: &'a Value,
    events: Vec<Value>,
}

impl Host<'_> {
    fn actor(&self, handle: u32) -> &Value {
        &self.world["actors"][(handle - 1) as usize]
    }
    fn index(handle: u32) -> u64 {
        (handle - 1) as u64
    }
    fn handle(v: &Value) -> Option<u32> {
        v.as_u64().map(|i| i as u32 + 1)
    }
}

fn al(v: &Value) -> bool {
    v.as_u64().unwrap() & 0xFF != 0
}

impl World for Host<'_> {
    fn targetable(&mut self, target: u32) -> Result<bool> {
        self.events.push(json!(["targetable", Self::index(target)]));
        Ok(al(&self.actor(target)["targetable"]))
    }
    fn kingdom(&mut self, actor: u32) -> Result<u32> {
        self.events.push(json!(["kingdom", Self::index(actor)]));
        Ok(self.actor(actor)["kingdom"].as_u64().unwrap() as u32)
    }
    fn accepts(&mut self, target: u32, kingdom: u32) -> Result<bool> {
        self.events.push(json!(["accepts", Self::index(target), kingdom]));
        Ok(al(&self.actor(target)["accepts"][kingdom.to_string()]))
    }
    fn organization(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok((!self.actor(actor)["organization"].is_null()).then_some(actor))
    }
    fn member_count(&mut self, organization: u32) -> Result<i32> {
        Ok(self.actor(organization)["organization"]["count"].as_i64().unwrap() as i32)
    }
    fn member(&mut self, organization: u32, index: i32) -> Result<Option<u32>> {
        Ok(Self::handle(&self.actor(organization)["organization"]["slots"][index as usize]))
    }
    fn performer(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok(self.actor(actor)["performer"].as_bool().unwrap().then_some(actor))
    }
    fn performer_relation(&mut self, performer: u32, target: u32, a: u32, b: u32) -> Result<i32> {
        self.events.push(json!(["perform", Self::index(performer), Self::index(target), a, b]));
        Ok(self.world["perform"][Self::index(performer) as usize][Self::index(target) as usize]
            .as_i64()
            .unwrap() as i32)
    }
    fn membership_organization(&mut self, actor: u32) -> Result<Option<u32>> {
        let m = &self.actor(actor)["membership"];
        Ok(if m.is_null() { None } else { Self::handle(&m["organization"]) })
    }
    fn mind(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok((!self.actor(actor)["mind"].is_null()).then_some(actor))
    }
    fn mind_target(&mut self, mind: u32) -> Result<Option<u32>> {
        self.events.push(json!(["mind_target", Self::index(mind)]));
        Ok(Self::handle(&self.actor(mind)["mind"]["target"]))
    }
}

#[test]
fn original_relation_and_attacking_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-relation-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 900);
    let (mut relation_count, mut attacking_count) = (0, 0);
    for (i, case) in cases.iter().enumerate() {
        let world = &doc["worlds"][case["world"].as_u64().unwrap() as usize];
        let mut host = Host { world, events: Vec::new() };
        let actor = case["actor"].as_u64().unwrap() as u32 + 1;
        let result = match case["kind"].as_str().unwrap() {
            "relation" => {
                relation_count += 1;
                let target = Host::handle(&case["target"]);
                let a = case["a"].as_u64().unwrap() as u32;
                let b = case["b"].as_u64().unwrap() as u32;
                json!(relation(&mut host, actor, target, a, b).unwrap())
            }
            "attacking" => {
                attacking_count += 1;
                let target = case["target"].as_u64().unwrap() as u32 + 1;
                json!(attacking(&mut host, actor, target).unwrap() as u8)
            }
            other => panic!("unknown kind {other}"),
        };
        assert_eq!(result, case["result"], "case {i} result");
        assert_eq!(Value::Array(host.events), case["events"], "case {i} events");
    }
    assert_eq!((relation_count, attacking_count), (450, 450));
}
