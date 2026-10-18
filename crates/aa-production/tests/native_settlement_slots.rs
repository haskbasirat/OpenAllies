//! Replays research/phase8-economy/native_settlement_slots.py: original
//! SettlementComponent find structure 0x585500 (0x5890C0 match) and
//! AddStructure 0x586B40.
use aa_production::Result;
use aa_production::construction::*;
use serde_json::{Value, json};

struct Slots {
    events: Vec<Value>,
}
impl SettlementSlots for Slots {
    fn attach_structure(&mut self, actor: u32) -> Result<()> {
        self.events
            .push(json!({"kind": "attach_structure", "actor": actor}));
        Ok(())
    }
    fn log_add_failed(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"kind": "log_add_failed"}));
        Ok(())
    }
}

fn ids(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect()
}

#[test]
fn settlement_slots_match_original_x86() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-settlement-slot-fixtures.json"
    ))
    .unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let actors = case["actors"].as_array().unwrap();
        let facts = |actor: u32| -> Result<StructureMatchFacts> {
            let a = &actors[actor as usize];
            let definition = a["definition"].as_u64().unwrap() as u32;
            let lists = &case["definitions"][definition.to_string()];
            Ok(StructureMatchFacts {
                structure_component: a["structure_component"].as_bool().unwrap(),
                state_25: a["state_25"].as_bool().unwrap(),
                transient: a["state_26"].as_bool().unwrap() || a["state_36"].as_bool().unwrap(),
                definition,
                constructing: a["constructing"].as_bool().unwrap(),
                upgrades: ids(&lists["upgrades"]),
                list_4d0: ids(&lists["list_4d0"]),
            })
        };
        let level = case["level"].as_u64().unwrap() as u32;
        let mut settlement = SettlementStructures {
            level,
            center: case["center"].as_u64().map(|c| c as u32),
            structures: ids(&case["structures"]),
        };
        let limit = case["limits"][level as usize].as_i64().unwrap() as i32;
        for step in fixture["steps"].as_array().unwrap() {
            let op = &step["op"];
            let mut slots = Slots { events: vec![] };
            let result = if op["kind"] == "find" {
                json!(
                    find_settlement_structure(
                        &settlement,
                        op["definition"].as_u64().unwrap() as u32,
                        op["require_complete"].as_bool().unwrap(),
                        op["include_upgrades"].as_bool().unwrap(),
                        facts,
                    )
                    .unwrap()
                )
            } else {
                let actor = op["actor"].as_u64().map(|a| a as u32);
                let component = actor.is_some_and(|a| facts(a).unwrap().structure_component);
                json!(
                    add_settlement_structure(&mut settlement, actor, component, limit, &mut slots)
                        .unwrap()
                )
            };
            let actual = json!({"result": result, "structures": settlement.structures,
                "events": slots.events});
            assert_eq!(actual, step["expected"], "{} {op}", fixture["name"]);
            executions += 1;
        }
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
