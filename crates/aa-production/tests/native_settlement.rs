//! Replays research/phase8-economy/native_settlement.py: original
//! SettlementComponent structure building (0x587FD0 BuildStructure, 0x587B60
//! CanBuildStructure, 0x5879B0 affordability, 0x5655C0 paid-cost store) at
//! PC24/PC53/PC64.
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
    created: Vec<Option<ConstructionPayments>>,
    precision: X87Precision,
    events: Vec<Value>,
}
impl TestWorld {
    fn flag(&self, name: &str) -> bool {
        self.world[name].as_bool().unwrap()
    }
    fn words(values: &[u32]) -> Vec<F32Bits> {
        values.iter().copied().map(F32Bits).collect()
    }
    fn list(&self, name: &str) -> Vec<u32> {
        self.world[name]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect()
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
        self.events.push(json!({"kind": "debit", "values": cost}));
        Ok(self.economy.debit_stockpiles(&Self::words(cost))?)
    }
    fn construction_credit(&mut self, _: u32, _: &[u32]) -> Result<()> {
        unreachable!("settlement building never credits")
    }
    fn notify_construct_unaffordable(&mut self, _: u32, _: &[u32], _: u32) -> Result<()> {
        unreachable!()
    }
    fn notify_upgrade_unaffordable(&mut self, _: u32, _: &[u32], _: Option<u32>) -> Result<()> {
        unreachable!()
    }
    fn has_structure(&mut self) -> Result<bool> {
        unreachable!()
    }
    fn state_active(&mut self, state: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "state_active", "state": state}));
        Ok(self.flag("state_active"))
    }
    fn actor_flags(&mut self) -> Result<u64> {
        unreachable!()
    }
    fn definition_flags(&mut self, definition: u32) -> Result<u32> {
        Ok(self.world["definition_flags"][definition.to_string()]
            .as_u64()
            .unwrap() as u32)
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
impl FpuPrecision for TestWorld {
    fn precision(&self) -> X87Precision {
        self.precision
    }
}
impl SettlementBuild for TestWorld {
    fn structure_limit(&mut self) -> Result<(i32, i32)> {
        let level = self.world["level"].as_u64().unwrap() as usize;
        Ok((
            self.world["count"].as_i64().unwrap() as i32,
            self.world["limits"][level].as_i64().unwrap() as i32,
        ))
    }
    fn settlement_log(&mut self, log: SettlementLog, _: u32) -> Result<()> {
        self.events
            .push(json!({"kind": "settlement_log", "log": log}));
        Ok(())
    }
    fn enemies_nearby(&mut self, _: u32, _: u32, _: u32, owner: Option<u32>) -> Result<bool> {
        assert_eq!(owner, Some(KINGDOM));
        self.events.push(json!({"kind": "enemies_nearby"}));
        Ok(self.op["tiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t.as_bool().unwrap()))
    }
    fn zone_is_own(&mut self, _: u32, _: u32, _: u32) -> Result<bool> {
        self.events.push(json!({"kind": "zone_is_own"}));
        Ok(self.op["own_zone"].as_bool().unwrap())
    }
    fn notify_within_zone(&mut self, _: u32, kingdom: Option<u32>) -> Result<()> {
        self.events
            .push(json!({"kind": "notify_within_zone", "kingdom": kingdom.is_some()}));
        Ok(())
    }
    fn buildable(&mut self, definition: u32) -> Result<bool> {
        Ok(self.list("buildable").contains(&definition))
    }
    fn settlement_has_structure(&mut self, definition: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "settlement_has_structure", "definition": definition}));
        Ok(self.list("has").contains(&definition))
    }
    fn notify_settlement_unaffordable(
        &mut self,
        _: u32,
        _: &[u32],
        kingdom: Option<u32>,
    ) -> Result<()> {
        self.events
            .push(json!({"kind": "notify_settlement_unaffordable", "kingdom": kingdom.is_some()}));
        Ok(())
    }
    fn create_structure(
        &mut self,
        definition: u32,
        x: u32,
        y: u32,
        owner: Option<u32>,
    ) -> Result<Option<u32>> {
        self.events
            .push(json!({"kind": "create_structure", "definition": definition,
            "x": x, "y": y, "owner": owner.is_some()}));
        if !self.op["created"].as_bool().unwrap() {
            return Ok(None);
        }
        let n = self.economy.catalog().resources.len();
        // The created ConstructionComponent starts with a zeroed paid vector of
        // the registry length.
        let component = self.op["component"].as_bool().unwrap();
        self.created.push(component.then(|| ConstructionPayments {
            paid: vec![0; n],
            ..ConstructionPayments::default()
        }));
        Ok(Some(self.created.len() as u32 - 1))
    }
    fn construction_payments(&mut self, created: u32) -> Result<Option<&mut ConstructionPayments>> {
        Ok(self.created[created as usize].as_mut())
    }
    fn settlement_add_structure(&mut self, created: u32) -> Result<()> {
        self.events
            .push(json!({"kind": "settlement_add_structure", "created": created}));
        Ok(())
    }
}

fn document() -> Document {
    serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-settlement-fixtures.json"
    ))
    .unwrap()
}

fn test_world(case: &Case, precision: X87Precision) -> TestWorld {
    let mut state = State::zeroed(case.catalog.resources.len());
    state.stockpiled = case.stock.clone();
    TestWorld {
        economy: Economy::new(case.catalog.clone(), state, case.limits, precision).unwrap(),
        world: case.world.clone(),
        op: Value::Null,
        quotes: case
            .quotes
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), v.clone()))
            .collect(),
        created: vec![],
        precision,
        events: vec![],
    }
}

#[test]
fn settlement_structure_building_matches_original_x86_at_all_precisions() {
    let document = document();
    let mut executions = 0;
    for fixture in document.fixtures {
        let mut world = test_world(&fixture.case, fixture.precision);
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
            let definition = op["definition"].as_u64().unwrap() as u32;
            let flag = |name: &str| op[name].as_bool().unwrap();
            let x = op["x"].as_u64().unwrap() as u32;
            let y = op["y"].as_u64().unwrap() as u32;
            let result = match op["kind"].as_str().unwrap() {
                "can_build" => json!(
                    can_build_structure(
                        definition,
                        x,
                        y,
                        flag("check_cost"),
                        flag("check_location"),
                        flag("notify"),
                        &mut world
                    )
                    .expect(&label)
                ),
                "affordable" => json!(
                    settlement_affordable(definition, flag("notify"), &mut world).expect(&label)
                ),
                "build" => match build_structure(definition, x, y, &mut world).expect(&label) {
                    Some(created) => {
                        let paid = world.created[created as usize]
                            .as_ref()
                            .map(|p| p.paid.clone());
                        json!({"created": created, "paid": paid})
                    }
                    None => Value::Null,
                },
                other => panic!("{other}"),
            };
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

#[test]
fn can_build_without_owner_is_outside_the_native_domain() {
    let document = document();
    let fixture = &document.fixtures[0];
    let mut world = test_world(&fixture.case, fixture.precision);
    world.world["kingdom"] = json!(false);
    world.world["state_active"] = json!(true);
    world.world["count"] = json!(0);
    world.world["limits"] = json!([5, 5, 5]);
    world.world["definition_flags"]["1"] = json!(1 << 13);
    assert!(can_build_structure(1, 0, 0, false, false, false, &mut world).is_err());
}
