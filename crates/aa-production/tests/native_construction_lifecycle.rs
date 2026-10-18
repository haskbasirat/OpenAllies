//! Replays research/phase8-economy/native_construction_lifecycle.py: original
//! ConstructionComponent salvage (0x565500), sell (0x565A20/0x5658C0), cancel
//! command (0x565860), command dispatch (0x5667F0) and Tick (0x566880 with the
//! 0x501570 auto-upgrade timer) at PC24/PC53/PC64.
use aa_economy::{Catalog, Economy, F32Bits, Limits, ResourceKind, State, X87Precision};
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
    initial: ConstructionPayments,
    steps: Vec<Step>,
}
#[derive(Deserialize, Clone)]
struct Case {
    catalog: Catalog,
    limits: Limits,
    stock: Vec<F32Bits>,
    quotes: BTreeMap<String, Vec<Vec<u32>>>,
    payments: ConstructionPayments,
    fraction: u32,
    world: Value,
}
#[derive(Deserialize)]
struct Step {
    op: Value,
    expected: Value,
}

const KINGDOM: u32 = 1;
const TOKEN: u32 = 7;

struct TestWorld {
    economy: Economy,
    world: Value,
    quotes: BTreeMap<u32, Vec<Vec<u32>>>,
    fraction: u32,
    precision: X87Precision,
    events: Vec<Value>,
}
impl TestWorld {
    fn flag(&self, name: &str) -> bool {
        self.world[name].as_bool().unwrap()
    }
    fn word(&self, name: &str) -> u32 {
        self.world[name].as_u64().unwrap() as u32
    }
    fn words(values: &[u32]) -> Vec<F32Bits> {
        values.iter().copied().map(F32Bits).collect()
    }
    fn next_quote(&mut self, definition: u32) -> Vec<u32> {
        let queue = self.quotes.get_mut(&definition).expect("quote sequence");
        if queue.len() > 1 {
            queue.remove(0)
        } else {
            queue[0].clone()
        }
    }
}
impl Construction for TestWorld {
    fn construction_kingdom(&mut self) -> Result<Option<u32>> {
        self.events.push(json!({"kind": "kingdom"}));
        Ok(self.flag("kingdom").then_some(KINGDOM))
    }
    fn construction_cost(&mut self, definition: u32) -> Result<Vec<u32>> {
        let values = self.next_quote(definition);
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
    fn construction_credit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()> {
        assert_eq!(kingdom, KINGDOM);
        Ok(self.economy.credit_stockpiles(&Self::words(cost))?)
    }
    fn notify_construct_unaffordable(&mut self, _: u32, _: &[u32], _: u32) -> Result<()> {
        unreachable!("0x562FD0 is not reached by the lifecycle")
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
        Ok(self.world["states"]
            .get(state.to_string())
            .map(|v| v.as_bool().unwrap())
            .unwrap_or_else(|| self.flag("state_active")))
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
        assert_eq!(kingdom, KINGDOM);
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
        self.events.push(json!({"kind": "construction_begun",
            "kingdom": kingdom.is_some(), "definition": definition}));
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
impl FpuPrecision for TestWorld {
    fn precision(&self) -> X87Precision {
        self.precision
    }
}
impl ConstructionLifecycle for TestWorld {
    fn salvage_fraction(&mut self) -> Result<u32> {
        Ok(self.fraction)
    }
    fn notify_sold(&mut self, refund: &[u32], kingdom: Option<u32>) -> Result<()> {
        self.events
            .push(json!({"kind": "notify_sold", "kingdom": kingdom.is_some(), "refund": refund}));
        Ok(())
    }
    fn enable_structure(&mut self) -> Result<()> {
        self.events.push(json!({"kind": "enable_structure"}));
        Ok(())
    }
    fn disable_structure(&mut self) -> Result<()> {
        self.events.push(json!({"kind": "disable_structure"}));
        Ok(())
    }
    fn kingdom_dissolving(&mut self, kingdom: u32) -> Result<bool> {
        assert_eq!(kingdom, KINGDOM);
        Ok(self.flag("dissolving"))
    }
    fn refresh_actor(&mut self) -> Result<()> {
        self.events.push(json!({"kind": "refresh_actor"}));
        Ok(())
    }
    fn auto_upgrade_token(&mut self) -> Result<Option<u32>> {
        Ok(self.flag("token").then_some(TOKEN))
    }
    fn auto_upgrade_interval(&mut self, token: u32) -> Result<u32> {
        assert_eq!(token, TOKEN);
        Ok(self.word("interval"))
    }
    fn actor_time(&mut self) -> Result<u32> {
        Ok(self.word("actor_time"))
    }
    fn world_time(&mut self) -> Result<u32> {
        Ok(self.word("world_time"))
    }
    fn base_cost(&mut self, definition: u32) -> Result<Vec<u32>> {
        let values = self.next_quote(definition);
        self.events
            .push(json!({"kind": "base_quote", "definition": definition, "values": values}));
        Ok(values)
    }
    fn stockpiled_resources(&mut self) -> Result<Vec<bool>> {
        Ok(self
            .economy
            .catalog()
            .resources
            .iter()
            .map(|r| r.kind == ResourceKind::Stockpiled)
            .collect())
    }
    fn log_auto_upgrade_cost(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"kind": "log_auto_upgrade_cost"}));
        Ok(())
    }
    fn construction_start_time(&mut self) -> Result<u32> {
        Ok(self.word("start_time"))
    }
    fn construction_window(&mut self) -> Result<u32> {
        Ok(self.word("window"))
    }
}

#[test]
fn construction_lifecycle_matches_original_x86_at_all_precisions() {
    let document: Document = serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-construction-lifecycle-fixtures.json"
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
            quotes: case
                .quotes
                .iter()
                .map(|(k, v)| (k.parse().unwrap(), v.clone()))
                .collect(),
            fraction: case.fraction,
            precision: fixture.precision,
            events: vec![],
        };
        let mut payments = case.payments.clone();
        assert_eq!(payments, fixture.initial, "{}", fixture.name);
        for (index, step) in fixture.steps.iter().enumerate() {
            let op = &step.op;
            let label = format!("{} {:?} step {index} {op}", fixture.name, fixture.precision);
            if let Some(update) = op.get("world") {
                for (key, value) in update.as_object().unwrap() {
                    world.world[key] = value.clone();
                }
            }
            world.events.clear();
            let result = match op["kind"].as_str().unwrap() {
                "salvage" => {
                    let mut out: Vec<u32> = op["out"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap() as u32)
                        .collect();
                    salvage(
                        &mut out,
                        &payments.paid,
                        world.fraction,
                        op["accumulate"].as_bool().unwrap(),
                        world.precision,
                    )
                    .expect(&label);
                    json!(out)
                }
                "sell" => {
                    sell_structure(&mut payments, &mut world).expect(&label);
                    Value::Null
                }
                "command" => json!(
                    construction_command(
                        op["type"].as_u64().unwrap() as u32,
                        op["definition"].as_u64().unwrap() as u32,
                        &mut payments,
                        &mut world
                    )
                    .expect(&label)
                ),
                "tick" => {
                    construction_tick(&mut payments, &mut world).expect(&label);
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
