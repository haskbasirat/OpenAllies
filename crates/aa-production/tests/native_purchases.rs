//! Replays research/phase8-economy/native_purchases.py: original denizen, HQ
//! attach and transport refund economics at PC24/PC53/PC64.
use aa_economy::{Catalog, Economy, F32Bits, Limits, State, X87Precision};
use aa_production::Result;
use aa_production::purchases::*;
use serde_json::{Value, json};

struct World {
    economy: Economy,
    precision: X87Precision,
    world: Value,
    case: Value,
    op: Value,
    events: Vec<Value>,
}
impl World {
    fn words(values: &[u32]) -> Vec<F32Bits> {
        values.iter().copied().map(F32Bits).collect()
    }
    fn vector(v: &Value) -> Vec<u32> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as u32)
            .collect()
    }
    fn event(&mut self, value: Value) {
        self.events.push(value);
    }
}
impl Purchase for World {
    fn purchase_owner(&mut self) -> Result<Option<u32>> {
        self.event(json!({"kind": "owner"}));
        Ok(self.world["kingdom"].as_bool().unwrap().then_some(1))
    }
    fn purchase_can_afford(&mut self, _: u32, cost: &[u32]) -> Result<bool> {
        Ok(self.economy.can_afford(&Self::words(cost))?)
    }
    fn purchase_debit(&mut self, _: u32, cost: &[u32]) -> Result<()> {
        self.event(json!({"kind": "debit", "values": cost}));
        Ok(self.economy.debit_stockpiles(&Self::words(cost))?)
    }
    fn purchase_credit(&mut self, _: u32, cost: &[u32]) -> Result<()> {
        self.event(json!({"kind": "credit", "values": cost}));
        Ok(self.economy.credit_stockpiles(&Self::words(cost))?)
    }
}
impl Denizen for World {
    fn denizen_cost(&mut self, purchase: DenizenPurchase) -> Result<Vec<u32>> {
        let index = usize::from(purchase == DenizenPurchase::AirRecon);
        Ok(Self::vector(&self.case["denizen_cost"][index]))
    }
    fn denizen_unaffordable(&mut self, _: DenizenPurchase, _: &[u32], _: u32) -> Result<()> {
        self.event(json!({"kind": "telemetry"}));
        self.event(json!({"kind": "denizen_unaffordable", "kingdom": true}));
        Ok(())
    }
    fn create_denizen(&mut self, _: DenizenPurchase, mode: u32) -> Result<Option<u32>> {
        self.event(json!({"kind": "create_denizen", "mode": mode}));
        Ok(self.op["created"].as_bool().unwrap_or(true).then_some(99))
    }
    fn use_denizen(&mut self, _: DenizenPurchase, created: u32, mode: u32) -> Result<()> {
        assert_eq!(created, 99);
        self.event(json!({"kind": "use_denizen", "mode": mode}));
        Ok(())
    }
}
impl HqAttach for World {
    fn organization_cost(&mut self, _: u32) -> Result<Vec<u32>> {
        self.event(json!({"kind": "organization_cost"}));
        Ok(Self::vector(&self.case["organization_cost"]))
    }
    fn attach_cost_factor(&self) -> u32 {
        self.case["attach_factor"].as_u64().unwrap() as u32
    }
    fn registry_count(&self) -> usize {
        self.economy.catalog().resources.len()
    }
    fn precision(&self) -> X87Precision {
        self.precision
    }
    fn attach_unaffordable(&mut self, _: u32, _: &[u32], _: u32) -> Result<()> {
        self.event(json!({"kind": "attach_unaffordable", "kingdom": true}));
        Ok(())
    }
    fn attach_allowed(&mut self, _: u32, notify: bool) -> Result<bool> {
        self.event(json!({"kind": "attach_allowed", "notify": notify}));
        Ok(self.world["attach_allowed"].as_bool().unwrap())
    }
    fn create_attachment(&mut self, _: u32, slot: u32) -> Result<()> {
        self.event(json!({"kind": "create_attachment", "slot": slot}));
        Ok(())
    }
    fn attach_to_organization(&mut self, _: u32) -> Result<()> {
        self.event(json!({"kind": "attach_to_organization"}));
        Ok(())
    }
    fn attach_event(&mut self, _: u32) -> Result<()> {
        self.event(json!({"kind": "attach_event"}));
        Ok(())
    }
}
impl TransportRefund for World {
    fn unloading_failed_message(&mut self, kingdom: Option<u32>) -> Result<()> {
        self.event(json!({"kind": "unloading_failed", "kingdom": kingdom.is_some()}));
        Ok(())
    }
    fn refresh_economy(&mut self) -> Result<()> {
        self.event(json!({"kind": "refresh_economy"}));
        Ok(())
    }
}

#[test]
fn purchases_match_original_x86_at_all_precisions() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-purchase-fixtures.json"
    ))
    .unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let precision: X87Precision = serde_json::from_value(fixture["precision"].clone()).unwrap();
        let catalog: Catalog = serde_json::from_value(case["catalog"].clone()).unwrap();
        let limits: Limits = serde_json::from_value(case["limits"].clone()).unwrap();
        let n = catalog.resources.len();
        let state = State {
            raw_provided: vec![F32Bits::ZERO; n],
            effective_provided: vec![F32Bits::ZERO; n],
            consumed: vec![F32Bits::ZERO; n],
            stockpiled: World::words(&World::vector(&case["stock"])),
        };
        let mut world = World {
            economy: Economy::new(catalog, state, limits, precision).unwrap(),
            precision,
            world: case["world"].clone(),
            case: case.clone(),
            op: Value::Null,
            events: vec![],
        };
        for step in fixture["steps"].as_array().unwrap() {
            let op = step["op"].clone();
            let label = format!("{} {:?} {op}", fixture["name"], precision);
            if let Some(update) = op.get("world") {
                for (k, v) in update.as_object().unwrap() {
                    world.world[k] = v.clone();
                }
            }
            world.op = op.clone();
            world.events.clear();
            let flag = |key: &str| op[key].as_bool().unwrap_or(true);
            let organization = (flag("organization") && flag("component")).then_some(5);
            let result = match op["kind"].as_str().unwrap() {
                "denizen" => {
                    let purchase: DenizenPurchase =
                        serde_json::from_value(op["purchase"].clone()).unwrap();
                    json!(purchase_denizen(purchase, &mut world).expect(&label))
                }
                "attach_gate" => json!(
                    hq_attach_affordable(
                        organization,
                        op["check_cost"].as_bool().unwrap(),
                        op["notify"].as_bool().unwrap(),
                        &mut world
                    )
                    .expect(&label)
                ),
                "attach" => json!(
                    hq_attach(
                        organization,
                        op["pay"].as_bool().unwrap(),
                        op["slot"].as_u64().unwrap() as u32,
                        &mut world
                    )
                    .expect(&label)
                ),
                "transport_refund" => {
                    let mut transport = TransportPayment {
                        no_refund: op["no_refund"].as_bool().unwrap(),
                        cost: World::vector(&op["cost"]),
                        upkeep: World::vector(&op["upkeep"]),
                    };
                    transport_unload_refund(&mut transport, n, &mut world).expect(&label);
                    json!({"cost": transport.cost, "upkeep": transport.upkeep})
                }
                other => panic!("{other}"),
            };
            let actual = json!({"stock": world.economy.state().stockpiled, "events": world.events,
                                "result": result});
            assert_eq!(actual, step["expected"], "{label}");
            executions += 1;
        }
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
