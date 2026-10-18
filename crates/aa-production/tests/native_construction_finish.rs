//! Replays research/phase8-economy/native_construction_finish.py: original
//! completion dispatch 0x566AC0, OnConstructionFinished 0x565B00 and
//! OnUpgradeFinished 0x566400 (with the 0x565700 refund on failure) at
//! PC24/PC53/PC64.
use aa_economy::{Catalog, Economy, F32Bits, Limits, State, X87Precision};
use aa_production::Result;
use aa_production::construction::*;
use serde::Deserialize;
use serde_json::{Value, json};

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
    payments: ConstructionPayments,
    world: Value,
}
#[derive(Deserialize)]
struct Step {
    op: Value,
    expected: Value,
}

const KINGDOM: u32 = 1;
const CREATED: u32 = 1000;
const GROUP: u32 = 77;
const CONTROLLER: u32 = 55;
const AI: u32 = 66;
const ACTOR_DEFINITION: u32 = 0xADA;

struct TestWorld {
    economy: Economy,
    world: Value,
    op: Value,
    created_definition: u32,
    events: Vec<Value>,
}
impl TestWorld {
    fn flag(&self, name: &str) -> bool {
        self.world[name].as_bool().unwrap()
    }
    fn op(&self, name: &str) -> bool {
        self.op[name].as_bool().unwrap()
    }
    fn push(&mut self, value: Value) {
        self.events.push(value);
    }
    fn words(values: &[u32]) -> Vec<F32Bits> {
        values.iter().copied().map(F32Bits).collect()
    }
}
impl Construction for TestWorld {
    fn construction_kingdom(&mut self) -> Result<Option<u32>> {
        self.push(json!({"kind": "kingdom"}));
        Ok(self.flag("kingdom").then_some(KINGDOM))
    }
    fn construction_cost(&mut self, _: u32) -> Result<Vec<u32>> {
        unreachable!()
    }
    fn construction_can_afford(&mut self, _: u32, _: &[u32]) -> Result<bool> {
        unreachable!()
    }
    fn construction_debit(&mut self, _: u32, _: &[u32]) -> Result<()> {
        unreachable!()
    }
    fn construction_credit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()> {
        assert_eq!(kingdom, KINGDOM);
        Ok(self.economy.credit_stockpiles(&Self::words(cost))?)
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
    fn state_active(&mut self, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn actor_flags(&mut self) -> Result<u64> {
        Ok(self.world["actor_flags"].as_u64().unwrap())
    }
    fn definition_flags(&mut self, _: u32) -> Result<u32> {
        unreachable!()
    }
    fn kingdom_allows(&mut self, _: u32, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn upgrade_list(&mut self) -> Result<Vec<u32>> {
        unreachable!()
    }
    fn construction_begun(&mut self, _: Option<u32>, _: u32) -> Result<()> {
        unreachable!()
    }
    fn apply_upgrade_modifiers(&mut self, definition: u32) -> Result<()> {
        self.push(json!({"kind": "apply_upgrade_modifiers", "definition": definition}));
        Ok(())
    }
    fn construction_actor_state(&mut self, state: u32, enabled: bool) -> Result<()> {
        self.push(json!({"kind": "actor_state", "state": state, "enabled": enabled}));
        Ok(())
    }
    fn destroyed(&mut self) -> Result<bool> {
        Ok(self.flag("destroyed"))
    }
    fn destroy_actor(&mut self) -> Result<()> {
        self.push(json!({"kind": "destroy_actor"}));
        Ok(())
    }
}
impl PlacementMessages for TestWorld {
    fn notify_enemies_nearby(&mut self, _: Option<u32>) -> Result<()> {
        unreachable!()
    }
    fn notify_creation_failed(&mut self, _: u32, kingdom: Option<u32>) -> Result<()> {
        self.push(json!({"kind": "notify_creation_failed", "kingdom": kingdom.is_some()}));
        Ok(())
    }
}
impl ConstructionFinish for TestWorld {
    fn settlement_definition(&mut self) -> Result<u32> {
        Ok(self.world["settlement_definition"].as_u64().unwrap() as u32)
    }
    fn actor_virtual_100(&mut self) -> Result<bool> {
        self.push(json!({"kind": "virtual_100"}));
        Ok(self.op("virtual_100"))
    }
    fn actor_definition(&mut self) -> Result<u32> {
        Ok(ACTOR_DEFINITION)
    }
    fn actor_placement(&mut self) -> Result<[u32; 5]> {
        let p = &self.world["placement"];
        Ok(std::array::from_fn(|i| p[i].as_u64().unwrap() as u32))
    }
    fn actor_gtid(&mut self) -> Result<u32> {
        Ok(0x50033)
    }
    fn components(&mut self) -> Result<ComponentPresence> {
        let c = &self.op["components"];
        Ok(ComponentPresence {
            c78: c["c78"].as_bool().unwrap(),
            c88: c["c88"].as_bool().unwrap(),
            c90: c["c90"].as_bool().unwrap(),
            c94: c["c94"].as_bool().unwrap(),
        })
    }
    fn replace_create(&mut self, params: &ReplaceParams) -> Result<Option<u32>> {
        let mut recorded = serde_json::to_value(params).unwrap();
        recorded["owner"] = json!(params.owner.is_some());
        self.push(json!({"kind": "replace_create", "params": recorded}));
        self.created_definition = params.definition;
        Ok(self.op("created").then_some(CREATED))
    }
    fn created_definition(&mut self, created: u32) -> Result<u32> {
        assert_eq!(created, CREATED);
        Ok(self.created_definition)
    }
    fn join_new_settlement(&mut self, created: u32) -> Result<bool> {
        assert_eq!(created, CREATED);
        if !self.op("created_settlement") {
            return Ok(false);
        }
        self.push(json!({"kind": "join_new_settlement"}));
        Ok(self.op("joins"))
    }
    fn create_components(&mut self) -> Result<bool> {
        self.push(json!({"kind": "create_components"}));
        Ok(self.op("components_created"))
    }
    fn log_finish(&mut self, log: FinishLog) -> Result<()> {
        self.push(json!({"kind": "log_finish", "log": log}));
        Ok(())
    }
    fn kingdom_actor_completed(&mut self, kingdom: u32) -> Result<()> {
        assert_eq!(kingdom, KINGDOM);
        self.push(json!({"kind": "kingdom_actor_completed"}));
        Ok(())
    }
    fn refresh_economy(&mut self) -> Result<()> {
        self.push(json!({"kind": "refresh_economy"}));
        Ok(())
    }
    fn ai_manager_present(&mut self) -> Result<bool> {
        Ok(self.op("ai_manager"))
    }
    fn ai_for(&mut self, owner: Option<u32>) -> Result<Option<u32>> {
        self.push(json!({"kind": "ai_for", "owner": owner.is_some()}));
        Ok(self.op("ai").then_some(AI))
    }
    fn ai_constructed(&mut self, ai: u32) -> Result<()> {
        assert_eq!(ai, AI);
        self.push(json!({"kind": "ai_constructed"}));
        Ok(())
    }
    fn in_settlement(&mut self) -> Result<bool> {
        self.push(json!({"kind": "in_settlement"}));
        Ok(self.op("in_settlement"))
    }
    fn refresh_settlement(&mut self) -> Result<()> {
        self.push(json!({"kind": "refresh_settlement"}));
        Ok(())
    }
    fn refresh_organization(&mut self, created: Option<u32>) -> Result<()> {
        self.push(json!({"kind": "refresh_organization", "created": created.is_some()}));
        Ok(())
    }
    fn completion_statistics(&mut self) -> Result<()> {
        self.push(json!({"kind": "completion_statistics"}));
        Ok(())
    }
    fn upgrade_statistics(&mut self) -> Result<()> {
        self.push(json!({"kind": "upgrade_statistics"}));
        Ok(())
    }
    fn completion_event(&mut self, kind: u32, owner: Option<u32>) -> Result<()> {
        self.push(json!({"kind": "completion_event", "event": kind, "owner": owner.is_some()}));
        Ok(())
    }
    fn construction_trigger(&mut self, owner: Option<u32>, definition: u32) -> Result<()> {
        self.push(
            json!({"kind": "construction_trigger", "owner": owner.is_some(),
            "definition": definition}),
        );
        Ok(())
    }
    fn complete_component(
        &mut self,
        component: CompletionComponent,
        created: Option<u32>,
    ) -> Result<()> {
        self.push(json!({"kind": "complete_component", "component": component,
            "created": created.map(|c| c == CREATED)}));
        Ok(())
    }
    fn builder_controller(&mut self, _: u32) -> Result<u32> {
        self.push(json!({"kind": "builder_controller"}));
        Ok(CONTROLLER)
    }
    fn controller_busy(&mut self, controller: u32) -> Result<bool> {
        assert_eq!(controller, CONTROLLER);
        self.push(json!({"kind": "controller_busy"}));
        Ok(self.op("controller_busy"))
    }
    fn notify_builder(&mut self, controller: u32) -> Result<()> {
        assert_eq!(controller, CONTROLLER);
        self.push(json!({"kind": "notify_builder"}));
        Ok(())
    }
    fn built_trigger(&mut self) -> Result<()> {
        self.push(json!({"kind": "built_trigger"}));
        Ok(())
    }
    fn destroy_replaced(&mut self) -> Result<()> {
        self.push(json!({"kind": "destroy_replaced"}));
        Ok(())
    }
    fn prepare_upgrade(&mut self) -> Result<()> {
        self.push(json!({"kind": "prepare_upgrade"}));
        Ok(())
    }
    fn actor_group(&mut self) -> Result<Option<u32>> {
        Ok(self.op("group").then_some(GROUP))
    }
    fn leave_group(&mut self) -> Result<()> {
        self.push(json!({"kind": "leave_group"}));
        Ok(())
    }
    fn rejoin_group(&mut self, group: u32) -> Result<()> {
        assert_eq!(group, GROUP);
        self.push(json!({"kind": "rejoin_group"}));
        Ok(())
    }
    fn restore_after_failed_upgrade(&mut self) -> Result<()> {
        self.push(json!({"kind": "restore_after_failed_upgrade"}));
        Ok(())
    }
    fn created_state(&mut self, created: u32, state: u32, enabled: bool) -> Result<()> {
        assert_eq!(created, CREATED);
        self.push(json!({"kind": "created_state", "state": state, "enabled": enabled}));
        Ok(())
    }
    fn transfer_state(&mut self, created: u32) -> Result<()> {
        assert_eq!(created, CREATED);
        self.push(json!({"kind": "transfer_state"}));
        Ok(())
    }
    fn has_default_name(&mut self) -> Result<bool> {
        self.push(json!({"kind": "has_default_name"}));
        Ok(self.op("default_name"))
    }
    fn copy_name(&mut self, created: u32) -> Result<()> {
        assert_eq!(created, CREATED);
        self.push(json!({"kind": "copy_name"}));
        Ok(())
    }
}

#[test]
fn construction_completion_matches_original_x86_at_all_precisions() {
    let document: Document = serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-construction-finish-fixtures.json"
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
            created_definition: 0,
            events: vec![],
        };
        let mut payments = case.payments.clone();
        assert_eq!(payments, fixture.initial);
        for (index, step) in fixture.steps.iter().enumerate() {
            let op = &step.op;
            let label = format!("{} {:?} step {index} {op}", fixture.name, fixture.precision);
            for (key, value) in op["world"].as_object().unwrap() {
                world.world[key] = value.clone();
            }
            world.op = op.clone();
            world.events.clear();
            let builder = (op["builder_ref"].as_bool().unwrap()
                && op["builder"].as_bool().unwrap())
            .then_some(9);
            construction_complete(builder, &mut payments, &mut world).expect(&label);
            let actual = json!({
                "stock": world.economy.state().stockpiled,
                "payments": payments,
                "events": world.events,
            });
            assert_eq!(actual, step.expected, "{label}");
            executions += 1;
        }
    }
    assert_eq!(executions, document.executions);
}

#[test]
fn upgrade_without_definition_is_outside_the_native_domain() {
    let document: Document = serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-construction-finish-fixtures.json"
    ))
    .unwrap();
    let case = &document.fixtures[0].case;
    let mut state = State::zeroed(case.catalog.resources.len());
    state.stockpiled = case.stock.clone();
    let mut world = TestWorld {
        economy: Economy::new(
            case.catalog.clone(),
            state,
            case.limits,
            X87Precision::Double53,
        )
        .unwrap(),
        world: case.world.clone(),
        op: Value::Null,
        created_definition: 0,
        events: vec![],
    };
    let mut payments = case.payments.clone();
    payments.upgrade = 0;
    assert!(matches!(
        upgrade_finished(&mut payments, &mut world),
        Err(aa_production::Error::Invalid(_))
    ));
    assert!(world.events.is_empty());
}
