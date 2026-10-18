//! Replays research/phase8-economy/native_build_actor.py: original
//! BuilderComponent::BuildActor 0x563160 (placement 0x6939B0, affordability
//! 0x562FD0, restrictions, enemy footprint, site lookup, creation, quote and
//! debit after creation) at PC24/PC53/PC64.
use aa_economy::{Catalog, Economy, F32Bits, Limits, State, X87Precision};
use aa_production::Result;
use aa_production::construction::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Document {
    executions: usize,
    fixtures: Vec<Fixture>,
}
#[derive(Deserialize)]
struct Fixture {
    name: String,
    precision: X87Precision,
    case: Case,
    steps: Vec<Step>,
}
#[derive(Deserialize, Clone)]
struct Case {
    catalog: Catalog,
    limits: Limits,
    stock: Vec<F32Bits>,
    quotes: BTreeMap<String, Vec<Vec<u32>>>,
    world: Value,
}
#[derive(Deserialize)]
struct Step {
    op: Value,
    expected: Value,
}

const KINGDOM: u32 = 1;

struct TestWorld {
    economy: Economy,
    world: Value,
    op: Value,
    quotes: BTreeMap<u32, Vec<Vec<u32>>>,
    created: u32,
    events: Vec<Value>,
}
impl TestWorld {
    fn flag(&self, name: &str) -> bool {
        self.world[name].as_bool().unwrap()
    }
    fn words(values: &[u32]) -> Vec<F32Bits> {
        values.iter().copied().map(F32Bits).collect()
    }
    fn pair(value: &Value) -> [u32; 2] {
        [
            value[0].as_u64().unwrap() as u32,
            value[1].as_u64().unwrap() as u32,
        ]
    }
}
impl Construction for TestWorld {
    fn construction_kingdom(&mut self) -> Result<Option<u32>> {
        self.events.push(json!({"kind": "kingdom"}));
        Ok(self.flag("kingdom").then_some(KINGDOM))
    }
    fn construction_cost(&mut self, definition: u32) -> Result<Vec<u32>> {
        let queue = self.quotes.get_mut(&definition).expect("quote sequence");
        let values = if queue.len() > 1 {
            queue.remove(0)
        } else {
            queue[0].clone()
        };
        self.events
            .push(json!({"kind": "quote", "definition": definition, "values": values}));
        Ok(values)
    }
    fn construction_can_afford(&mut self, kingdom: u32, cost: &[u32]) -> Result<bool> {
        assert_eq!(kingdom, KINGDOM);
        Ok(self.economy.can_afford(&Self::words(cost))?)
    }
    fn construction_debit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()> {
        assert_eq!(kingdom, KINGDOM);
        Ok(self.economy.debit_stockpiles(&Self::words(cost))?)
    }
    fn construction_credit(&mut self, _: u32, _: &[u32]) -> Result<()> {
        unreachable!("BuildActor never credits")
    }
    fn notify_construct_unaffordable(&mut self, _: u32, _: &[u32], kingdom: u32) -> Result<()> {
        assert_eq!(kingdom, KINGDOM);
        self.events
            .push(json!({"kind": "notify_construct_unaffordable", "kingdom": true}));
        Ok(())
    }
    fn notify_upgrade_unaffordable(&mut self, _: u32, _: &[u32], _: Option<u32>) -> Result<()> {
        unreachable!()
    }
    fn has_structure(&mut self) -> Result<bool> {
        unreachable!()
    }
    fn state_active(&mut self, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn actor_flags(&mut self) -> Result<u64> {
        unreachable!()
    }
    fn definition_flags(&mut self, _: u32) -> Result<u32> {
        unreachable!()
    }
    fn kingdom_allows(&mut self, kingdom: u32, definition: u32) -> Result<bool> {
        assert_eq!(kingdom, KINGDOM);
        self.events
            .push(json!({"kind": "kingdom_allows", "definition": definition}));
        Ok(self.flag("kingdom_allows"))
    }
    fn upgrade_list(&mut self) -> Result<Vec<u32>> {
        unreachable!()
    }
    fn construction_begun(&mut self, _: Option<u32>, _: u32) -> Result<()> {
        unreachable!()
    }
    fn apply_upgrade_modifiers(&mut self, _: u32) -> Result<()> {
        unreachable!()
    }
    fn construction_actor_state(&mut self, _: u32, _: bool) -> Result<()> {
        unreachable!()
    }
    fn destroyed(&mut self) -> Result<bool> {
        unreachable!()
    }
    fn destroy_actor(&mut self) -> Result<()> {
        unreachable!()
    }
}
impl PlacementMessages for TestWorld {
    fn notify_enemies_nearby(&mut self, kingdom: Option<u32>) -> Result<()> {
        self.events
            .push(json!({"kind": "notify_enemies_nearby", "kingdom": kingdom.is_some()}));
        Ok(())
    }
    fn notify_creation_failed(&mut self, _: u32, kingdom: Option<u32>) -> Result<()> {
        self.events
            .push(json!({"kind": "notify_creation_failed", "kingdom": kingdom.is_some()}));
        Ok(())
    }
}
impl BuildActorWorld for TestWorld {
    fn build_rules(&mut self, definition: u32) -> Result<BuildRules> {
        let d = definition.to_string();
        Ok(BuildRules {
            placement_flags: self.world["placement_flags"][&d].as_u64().unwrap() as u32,
            check_enemies: self.world["check_enemies"][&d].as_bool().unwrap(),
            site_class: self.world["site_class"][&d].as_u64().unwrap() as u32,
        })
    }
    fn snaps_to_grid(&mut self, definition: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "snaps_to_grid", "definition": definition}));
        Ok(self.op["snaps"].as_bool().unwrap())
    }
    fn snap_location(&mut self, definition: u32, location: [u32; 2]) -> Result<[u32; 2]> {
        self.events
            .push(json!({"kind": "snap_location", "definition": definition, "location": location}));
        Ok(Self::pair(&self.op["snapped"]))
    }
    fn build_enemies_nearby(
        &mut self,
        definition: u32,
        _: [u32; 2],
        _: [u32; 2],
        owner: Option<u32>,
    ) -> Result<bool> {
        assert_eq!(owner.is_some(), self.flag("kingdom"));
        self.events
            .push(json!({"kind": "enemies_nearby", "definition": definition}));
        Ok(self.op["tiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t.as_bool().unwrap()))
    }
    fn find_site(&mut self, definition: u32, location: [u32; 2]) -> Result<Option<u32>> {
        self.events
            .push(json!({"kind": "find_site", "definition": definition, "location": location}));
        Ok(self.op["site"].as_u64().map(|s| s as u32))
    }
    fn log_missing_site(&mut self, _: u32, _: [u32; 2]) -> Result<()> {
        self.events.push(json!({"kind": "log_missing_site"}));
        Ok(())
    }
    fn builder_field_28(&mut self) -> Result<u32> {
        Ok(self.world["builder_field_28"].as_u64().unwrap() as u32)
    }
    fn create_actor(&mut self, params: &BuildParams) -> Result<Option<u32>> {
        let mut recorded = serde_json::to_value(params).unwrap();
        recorded["owner"] = json!(params.owner.is_some());
        self.events
            .push(json!({"kind": "create_actor", "params": recorded}));
        if !self.op["created"].as_bool().unwrap() {
            return Ok(None);
        }
        self.created += 1;
        Ok(Some(self.created))
    }
}

#[test]
fn build_actor_matches_original_x86_at_all_precisions() {
    let document: Document = serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-build-actor-fixtures.json"
    ))
    .unwrap();
    let mut executions = 0;
    for fixture in document.fixtures {
        let case = &fixture.case;
        let mut state = State::zeroed(case.catalog.resources.len());
        state.stockpiled = case.stock.clone();
        let mut world = TestWorld {
            economy: Economy::new(case.catalog.clone(), state, case.limits, fixture.precision)
                .unwrap(),
            world: case.world.clone(),
            op: Value::Null,
            quotes: case
                .quotes
                .iter()
                .map(|(k, v)| (k.parse().unwrap(), v.clone()))
                .collect(),
            created: 0,
            events: vec![],
        };
        for (index, step) in fixture.steps.iter().enumerate() {
            let op = &step.op;
            let label = format!("{} {:?} step {index} {op}", fixture.name, fixture.precision);
            if let Some(update) = op.get("world") {
                for (key, value) in update.as_object().unwrap() {
                    world.world[key] = value.clone();
                }
            }
            world.op = op.clone();
            world.events.clear();
            let result = build_actor(
                op["definition"].as_u64().unwrap() as u32,
                TestWorld::pair(&op["location"]),
                TestWorld::pair(&op["facing"]),
                op["tag"].as_u64().unwrap() as u32,
                &mut world,
            )
            .expect(&label);
            let actual = json!({
                "stock": world.economy.state().stockpiled,
                "events": world.events,
                "result": result,
            });
            assert_eq!(actual, step.expected, "{label}");
            executions += 1;
        }
    }
    assert_eq!(executions, document.executions);
}
