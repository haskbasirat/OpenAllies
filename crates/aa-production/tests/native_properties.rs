//! Replays research/phase8-economy/native_properties.py: original
//! GKingdom::AddProperty 0x4E3AD0 with its property-state container.
use aa_production::Result;
use aa_production::research::*;
use serde_json::{Value, json};

struct World {
    w: Value,
    events: Vec<Value>,
}
impl World {
    fn has(&self, key: &str, d: u32) -> bool {
        self.w[key]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64() == Some(u64::from(d)))
    }
}
impl KingdomPropertyWorld for World {
    fn is_technology(&mut self, definition: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "is_technology", "definition": definition}));
        Ok(self.has("technologies", definition))
    }
    fn restricted(&mut self, definition: u32) -> Result<bool> {
        Ok(self.has("restricted", definition))
    }
    fn technology_allowed(&mut self, _: u32, definition: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "technology_allowed", "definition": definition}));
        Ok(self.has("allowed", definition))
    }
    fn log_not_allowed(&mut self, _: u32, _: u32) -> Result<()> {
        self.events.push(json!({"kind": "log_not_allowed"}));
        Ok(())
    }
    fn world_time(&mut self) -> Result<u32> {
        Ok(self.w["time"].as_u64().unwrap() as u32)
    }
    fn kingdom_actors(&mut self, _: u32) -> Result<Vec<u32>> {
        Ok(self.w["actors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect())
    }
    fn apply_property(
        &mut self,
        actor: u32,
        definition: u32,
        source: u32,
        origin: Option<u32>,
    ) -> Result<()> {
        self.events
            .push(json!({"kind": "apply_property", "actor": actor,
            "definition": definition, "source": source, "origin": origin}));
        Ok(())
    }
    fn resolve(&self, gtid: u32) -> Option<u32> {
        let slot = gtid & 0xffff;
        ((gtid >> 16) == 3 && (1..=4).contains(&slot)).then_some(slot)
    }
    fn technology_gained(&mut self, _: u32, definition: u32) -> Result<()> {
        self.events
            .push(json!({"kind": "technology_gained", "definition": definition}));
        Ok(())
    }
}

impl KingdomPropertyRemoval for World {
    fn log_missing_property(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"kind": "log_missing_property"}));
        Ok(())
    }
    fn remove_property_effect(
        &mut self,
        actor: u32,
        definition: u32,
        origin: Option<u32>,
    ) -> Result<()> {
        self.events
            .push(json!({"kind": "remove_property_effect", "actor": actor,
            "definition": definition, "origin": origin}));
        Ok(())
    }
    fn technology_lost(&mut self, _: u32, definition: u32) -> Result<()> {
        self.events
            .push(json!({"kind": "technology_lost", "definition": definition}));
        Ok(())
    }
}

#[test]
fn add_property_matches_original_x86() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-property-fixtures.json"
    ))
    .unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let mut properties = KingdomProperties {
            records: serde_json::from_value(fixture["initial"].clone()).unwrap(),
        };
        let mut world = World {
            w: case["world"].clone(),
            events: vec![],
        };
        for step in fixture["steps"].as_array().unwrap() {
            let op = &step["op"];
            world.events.clear();
            let origin = op["origin"].as_u64().unwrap() as u32;
            if op["remove"].as_bool().unwrap_or(false) {
                remove_kingdom_property(
                    &mut properties,
                    1,
                    op["definition"].as_u64().unwrap() as u32,
                    (origin != 0).then_some(origin),
                    &mut world,
                )
                .unwrap();
                let actual = json!({"records": properties.records, "events": world.events});
                assert_eq!(actual, step["expected"], "{} {op}", fixture["name"]);
                executions += 1;
                continue;
            }
            add_kingdom_property(
                &mut properties,
                1,
                op["definition"].as_u64().unwrap() as u32,
                op["source"].as_u64().unwrap() as u32,
                if origin == 0 { 0 } else { (3 << 16) | origin },
                op["notify"].as_bool().unwrap(),
                &mut world,
            )
            .unwrap();
            let actual = json!({"records": properties.records, "events": world.events});
            assert_eq!(actual, step["expected"], "{} {op}", fixture["name"]);
            executions += 1;
        }
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
