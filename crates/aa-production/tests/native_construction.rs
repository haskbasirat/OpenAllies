//! Replays research/phase8-economy/native_construction.py: original
//! construction/upgrade affordability, payment and refunds at PC24/PC53/PC64.
use aa_economy::{Catalog, Economy, F32Bits, Limits, State, X87Precision};
use aa_production::construction::*;
use aa_production::{Error, Result};
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
    payments: ConstructionPayments,
    world: Value,
}
#[derive(Deserialize)]
struct Step {
    op: Value,
    expected: Value,
}

struct TestWorld {
    economy: Economy,
    world: Value,
    quotes: BTreeMap<u32, Vec<Vec<u32>>>,
    events: Vec<Value>,
}
impl TestWorld {
    fn flag(&self, name: &str) -> bool {
        self.world[name].as_bool().unwrap()
    }
    fn words(values: &[u32]) -> Vec<F32Bits> {
        values.iter().copied().map(F32Bits).collect()
    }
}
impl Construction for TestWorld {
    fn construction_kingdom(&mut self) -> Result<Option<u32>> {
        self.events.push(json!({"kind": "kingdom"}));
        Ok(self.flag("kingdom").then_some(1))
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
        assert_eq!(kingdom, 1);
        Ok(self.economy.can_afford(&Self::words(cost))?)
    }
    fn construction_debit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()> {
        assert_eq!(kingdom, 1);
        Ok(self.economy.debit_stockpiles(&Self::words(cost))?)
    }
    fn construction_credit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()> {
        assert_eq!(kingdom, 1);
        Ok(self.economy.credit_stockpiles(&Self::words(cost))?)
    }
    fn notify_construct_unaffordable(&mut self, _: u32, _: &[u32], kingdom: u32) -> Result<()> {
        assert_eq!(kingdom, 1);
        self.events
            .push(json!({"kind": "notify_construct_unaffordable", "kingdom": true}));
        Ok(())
    }
    fn notify_upgrade_unaffordable(
        &mut self,
        _: u32,
        _: &[u32],
        kingdom: Option<u32>,
    ) -> Result<()> {
        self.events
            .push(json!({"kind": "notify_upgrade_unaffordable", "kingdom": kingdom.is_some()}));
        Ok(())
    }
    fn has_structure(&mut self) -> Result<bool> {
        Ok(self.flag("has_structure"))
    }
    fn state_active(&mut self, state: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "state_active", "state": state}));
        Ok(self.flag("state_active"))
    }
    fn actor_flags(&mut self) -> Result<u64> {
        Ok(self.world["actor_flags"].as_u64().unwrap())
    }
    fn definition_flags(&mut self, definition: u32) -> Result<u32> {
        Ok(self.world["definition_flags"][definition.to_string()]
            .as_u64()
            .unwrap() as u32)
    }
    fn kingdom_allows(&mut self, kingdom: u32, definition: u32) -> Result<bool> {
        assert_eq!(kingdom, 1);
        self.events
            .push(json!({"kind": "kingdom_allows", "definition": definition}));
        Ok(self.flag("kingdom_allows"))
    }
    fn upgrade_list(&mut self) -> Result<Vec<u32>> {
        Ok(self.world["upgrade_list"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect())
    }
    fn construction_begun(&mut self, kingdom: Option<u32>, definition: u32) -> Result<()> {
        self.events.push(
            json!({"kind": "construction_begun", "kingdom": kingdom.is_some(),
                                "definition": definition}),
        );
        Ok(())
    }
    fn apply_upgrade_modifiers(&mut self, definition: u32) -> Result<()> {
        self.events
            .push(json!({"kind": "apply_upgrade_modifiers", "definition": definition}));
        Ok(())
    }
    fn construction_actor_state(&mut self, state: u32, enabled: bool) -> Result<()> {
        self.events
            .push(json!({"kind": "actor_state", "state": state, "enabled": enabled}));
        Ok(())
    }
    fn destroyed(&mut self) -> Result<bool> {
        Ok(self.flag("destroyed"))
    }
    fn destroy_actor(&mut self) -> Result<()> {
        self.events.push(json!({"kind": "destroy_actor"}));
        Ok(())
    }
}

fn document() -> Document {
    serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-construction-fixtures.json"
    ))
    .unwrap()
}

fn world(case: &Case, precision: X87Precision) -> TestWorld {
    let n = case.catalog.resources.len();
    let state = State {
        raw_provided: vec![F32Bits::ZERO; n],
        effective_provided: vec![F32Bits::ZERO; n],
        consumed: vec![F32Bits::ZERO; n],
        stockpiled: case.stock.clone(),
    };
    TestWorld {
        economy: Economy::new(case.catalog.clone(), state, case.limits, precision).unwrap(),
        world: case.world.clone(),
        quotes: case
            .quotes
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), v.clone()))
            .collect(),
        events: vec![],
    }
}

#[test]
fn construction_payments_match_original_x86_at_all_precisions() {
    let document = document();
    let mut executions = 0;
    for fixture in document.fixtures {
        let mut world = world(&fixture.case, fixture.precision);
        let mut payments = fixture.case.payments.clone();
        for (index, step) in fixture.steps.iter().enumerate() {
            let op = &step.op;
            let label = format!("{} {:?} step {index} {op}", fixture.name, fixture.precision);
            if let Some(update) = op.get("world") {
                for (key, value) in update.as_object().unwrap() {
                    world.world[key] = value.clone();
                }
            }
            world.events.clear();
            let definition = op["definition"].as_u64().unwrap_or(0) as u32;
            let result = match op["kind"].as_str().unwrap() {
                "construct_afford" => {
                    json!(construct_affordable(definition, &mut world).expect(&label))
                }
                "upgrade_afford" => json!(
                    upgrade_affordable(
                        definition,
                        op["check_cost"].as_bool().unwrap(),
                        op["notify"].as_bool().unwrap(),
                        &mut world
                    )
                    .expect(&label)
                ),
                "upgrade_start" => {
                    start_upgrade(&mut payments, definition, &mut world).expect(&label);
                    Value::Null
                }
                "upgrade_cancel" => {
                    cancel_upgrade(&mut payments, &mut world).expect(&label);
                    Value::Null
                }
                "construction_cancel" => {
                    cancel_construction(&mut payments, &mut world).expect(&label);
                    Value::Null
                }
                other => panic!("{other}"),
            };
            let actual = json!({
                "stock": world.economy.state().stockpiled,
                "payments": payments,
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
fn upgrade_gate_without_owner_is_outside_the_native_domain() {
    let document = document();
    let fixture = &document.fixtures[0];
    let mut world = world(&fixture.case, fixture.precision);
    world.world["kingdom"] = json!(false);
    world.world["has_structure"] = json!(true);
    world.world["actor_flags"] = json!(ACTOR_UPGRADE_ENABLED);
    assert!(matches!(
        upgrade_affordable(1, true, true, &mut world),
        Err(Error::Invalid(_))
    ));
}
