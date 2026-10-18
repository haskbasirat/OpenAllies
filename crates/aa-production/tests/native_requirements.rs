//! Replays research/phase8-economy/native_requirements.py: original research
//! requirement evaluation 0x5842F0 / 0x4C5630 / 0x4C5250.
use aa_production::Result;
use aa_production::research::*;
use serde_json::{Value, json};

const ACTOR: u32 = 1;
const SETTLEMENT: u32 = 2;
const ACTOR_PROPS: u32 = 11;
const SETTLEMENT_PROPS: u32 = 12;
const SETTLEMENT_COMPONENT: u32 = 13;

struct World {
    w: Value,
    events: Vec<Value>,
}
impl World {
    fn set(&self, key: &str) -> Vec<u32> {
        self.w[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect()
    }
    fn flag(&self, key: &str) -> bool {
        self.w[key].as_bool().unwrap()
    }
}
impl RequirementWorld for World {
    fn properties_of(&mut self, owner: u32) -> Result<Option<u32>> {
        Ok(match owner {
            ACTOR => self.flag("actor_properties").then_some(ACTOR_PROPS),
            SETTLEMENT => self
                .flag("settlement_properties")
                .then_some(SETTLEMENT_PROPS),
            _ => unreachable!(),
        })
    }
    fn properties_contain(&mut self, properties: u32, definition: u32) -> Result<bool> {
        let owner = if properties == ACTOR_PROPS {
            "actor"
        } else {
            "settlement"
        };
        self.events
            .push(json!({"kind": "properties_contain", "owner": owner, "definition": definition}));
        Ok(self.set(&format!("{owner}_known")).contains(&definition))
    }
    fn is_technology(&mut self, definition: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "is_technology", "definition": definition}));
        Ok(self.set("technologies").contains(&definition))
    }
    fn kingdom_knows(&mut self, _: u32, definition: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "kingdom_knows", "definition": definition}));
        Ok(self.set("kingdom_known").contains(&definition))
    }
    fn settlement_component(&mut self, settlement: u32) -> Result<Option<u32>> {
        assert_eq!(settlement, SETTLEMENT);
        Ok(self
            .flag("settlement_component")
            .then_some(SETTLEMENT_COMPONENT))
    }
    fn settlement_has_actor(&mut self, component: u32, definition: u32) -> Result<bool> {
        assert_eq!(component, SETTLEMENT_COMPONENT);
        self.events
            .push(json!({"kind": "settlement_has_actor", "definition": definition}));
        Ok(self.set("settlement_actors").contains(&definition))
    }
    fn kingdom_has_actor(&mut self, _: u32, definition: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "kingdom_has_actor", "definition": definition}));
        Ok(self.set("kingdom_actors").contains(&definition))
    }
    fn technology_allowed(&mut self, _: u32, definition: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "technology_allowed", "definition": definition}));
        Ok(self.set("allowed").contains(&definition))
    }
    fn describe_missing(&mut self, _: bool, list: &[u32]) -> Result<()> {
        self.events
            .push(json!({"kind": "describe_missing", "list": list}));
        Ok(())
    }
    fn missing_requirements_message(&mut self, item: &ResearchItem) -> Result<()> {
        self.events
            .push(json!({"kind": "missing_requirements_message", "definition": item.definition}));
        Ok(())
    }
}

#[test]
fn research_requirements_match_original_x86() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-requirements-fixtures.json"
    ))
    .unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let item: ResearchItem = serde_json::from_value(case["item"].clone()).unwrap();
        let mut world = World {
            w: case["world"].clone(),
            events: vec![],
        };
        for step in fixture["steps"].as_array().unwrap() {
            let op = &step["op"];
            world.events.clear();
            let result = research_requirements(
                &item,
                op["kingdom"].as_bool().unwrap().then_some(7),
                op["settlement"].as_bool().unwrap().then_some(SETTLEMENT),
                op["actor"].as_bool().unwrap().then_some(ACTOR),
                op["message"].as_bool().unwrap(),
                &mut world,
            )
            .unwrap();
            let actual = json!({"events": world.events, "result": result});
            assert_eq!(actual, step["expected"], "{} {op}", fixture["name"]);
            executions += 1;
        }
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
