//! Replays research/phase8-restoration/native-construction-actions.json:
//! the original 565140/565090 Construction contributor under Unicorn.
use aa_actors::{
    ActorId,
    ability_flags::{ActorFields, Result},
    construction_actions::{Definition, World, contribute},
};
use serde_json::Value;
use std::collections::BTreeMap;

const FIXTURE: &str =
    include_str!("../../../research/phase8-restoration/native-construction-actions.json");

struct Fixture {
    ids: BTreeMap<String, ActorId>,
    specs: BTreeMap<ActorId, Value>,
    actions: BTreeMap<ActorId, u64>,
    allow: bool,
    ready: bool,
    events: Vec<Value>,
}
impl Fixture {
    fn spec(&self, a: ActorId) -> &Value {
        &self.specs[&a]
    }
    fn reference(&self, a: ActorId, key: &str) -> Option<Option<ActorId>> {
        let spec = self.spec(a);
        spec.get(key)
            .map(|v| v.as_str().map(|name| self.ids[name]))
    }
}
impl World for Fixture {
    fn fields(&self, a: ActorId) -> Result<ActorFields> {
        Ok(ActorFields {
            thing_flags: 0,
            state_bits: self.spec(a)["state"].as_u64().unwrap(),
            action_bits: self.actions[&a],
        })
    }
    fn set_action_bits(&mut self, a: ActorId, bits: u64) -> Result<()> {
        self.actions.insert(a, bits);
        Ok(())
    }
    fn definition(&self, a: ActorId) -> Result<Definition> {
        let spec = self.spec(a);
        Ok(Definition {
            can_sell: spec.get("can_sell").and_then(Value::as_bool).unwrap_or(false),
            can_raze: spec.get("can_raze").and_then(Value::as_bool).unwrap_or(false),
            upgrade_count: spec.get("upgrade_count").and_then(Value::as_i64).unwrap_or(0) as i32,
        })
    }
    fn structure_parent(&self, a: ActorId) -> Result<Option<Option<ActorId>>> {
        Ok(self.reference(a, "structure"))
    }
    fn wall_reference(&self, a: ActorId) -> Result<Option<Option<ActorId>>> {
        Ok(self.reference(a, "wall"))
    }
    fn settlement_reference(&self, a: ActorId) -> Result<Option<Option<ActorId>>> {
        Ok(self.reference(a, "settlement"))
    }
    fn properties_flags(&self, a: ActorId) -> Result<Option<u32>> {
        Ok(self
            .spec(a)
            .get("properties")
            .and_then(Value::as_u64)
            .map(|v| v as u32))
    }
    fn allow_sell_under_siege(&mut self) -> Result<bool> {
        self.events
            .push(serde_json::json!(["svar", "AllowSellUnderSiege", 1]));
        Ok(self.allow)
    }
    fn settlement_upgrades_ready(&mut self, parent: ActorId) -> Result<bool> {
        let name = self
            .ids
            .iter()
            .find(|(_, v)| **v == parent)
            .unwrap()
            .0
            .clone();
        self.events.push(serde_json::json!(["upgrades_ready", name]));
        Ok(self.ready)
    }
}

#[test]
fn original_construction_contributor_fixtures_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 640);
    for case in cases {
        let input = &case["input"];
        let mut fixture = Fixture {
            ids: BTreeMap::new(),
            specs: BTreeMap::new(),
            actions: BTreeMap::new(),
            allow: input["allow_sell_under_siege"].as_bool().unwrap(),
            ready: input["upgrades_ready"].as_bool().unwrap(),
            events: Vec::new(),
        };
        for (i, (name, spec)) in input["actors"].as_object().unwrap().iter().enumerate() {
            let id = ActorId::from_raw(0x1000 + i as u32);
            fixture.ids.insert(name.clone(), id);
            fixture.specs.insert(id, spec.clone());
            fixture
                .actions
                .insert(id, spec.get("actions").and_then(Value::as_u64).unwrap_or(0));
        }
        let me = fixture.ids["self"];
        contribute(&mut fixture, me).unwrap();
        assert_eq!(
            fixture.actions[&me],
            case["output"]["actions"].as_u64().unwrap(),
            "{case}"
        );
        assert_eq!(
            Value::Array(fixture.events.clone()),
            case["output"]["events"],
            "{case}"
        );
    }
}
