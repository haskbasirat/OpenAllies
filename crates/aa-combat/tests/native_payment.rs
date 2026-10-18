use aa_body::F32;
use aa_combat::{
    Result,
    ability::Ability,
    payment::{self, Faction, State, World},
};
use aa_economy::{
    Catalog, Economy, F32Bits, Limits, ResourceDefinition, ResourceKind, X87Precision,
    tick::CommanderExperience,
};
use serde_json::{Value, json};
struct Host {
    input: Value,
    events: Vec<Value>,
}
impl World for Host {
    fn faction_xp_cost(&self, _: u32) -> Result<i32> {
        Ok(self.input["xp_cost"].as_i64().unwrap() as i32)
    }
    fn resource_costs(&self, _: u32) -> Result<Vec<F32>> {
        Ok(serde_json::from_value(self.input["costs"].clone()).unwrap())
    }
    fn add_statistic(
        &mut self,
        state: &mut State<'_>,
        statistics: u32,
        index: u32,
        amount: F32,
    ) -> Result<()> {
        self.events
            .push(json!({"statistics":statistics,"index":index,"amount":amount}));
        if self.input["mutate"] == true && index == 45 {
            let f = state.faction.as_mut().unwrap();
            f.experience.balance = F32Bits(3.17f32.to_bits());
            f.statistics = None;
            let count = self.input["costs"].as_array().unwrap().len();
            self.input["costs"] = json!(
                (0..count)
                    .map(|i| ((i + 1) as f64 * 1.717) as f32)
                    .map(f32::to_bits)
                    .collect::<Vec<_>>()
            );
        }
        if self.input["fail_stat"] == true {
            return Err(aa_combat::Error::World("stat failure".into()));
        }
        Ok(())
    }
}
/// Owned stores behind the borrowed payment State.
struct Stores {
    economy: Economy,
    experience: Option<CommanderExperience>,
    statistics: Option<u32>,
}
impl Stores {
    fn state(&mut self) -> State<'_> {
        State {
            economy: &mut self.economy,
            faction: self.experience.as_mut().map(|experience| Faction {
                experience,
                statistics: self.statistics,
            }),
        }
    }
}
fn stores(input: &Value) -> Stores {
    let stocks: Vec<F32Bits> = serde_json::from_value(input["stocks"].clone()).unwrap();
    let count = stocks.len();
    let catalog = Catalog {
        resources: (0..count)
            .map(|i| ResourceDefinition {
                id: i.to_string(),
                kind: ResourceKind::Stockpiled,
                limited_provider: false,
                purchase_cost: vec![F32Bits::ZERO; count],
            })
            .collect(),
    };
    let mut balances = aa_economy::State::zeroed(count);
    balances.stockpiled = stocks;
    let precision: X87Precision = serde_json::from_value(input["precision"].clone()).unwrap();
    Stores {
        economy: Economy::new(
            catalog,
            balances,
            Limits {
                provided_maximum: 100,
                stockpiled_maximum: F32Bits(1e30f32.to_bits()),
            },
            precision,
        )
        .unwrap(),
        experience: (input["faction"] == true).then(|| CommanderExperience {
            balance: serde_json::from_value(input["experience"].clone()).unwrap(),
            maximum: F32Bits(0xbf80_0000),
        }),
        statistics: (input["faction"] == true && input["statistics"] == true).then_some(9),
    }
}
#[test]
fn original_ability_payment_mutates_real_economy_and_faction_in_native_order() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-payment/native-payment-fixtures.json"
    ))
    .unwrap();
    for row in f["cases"].as_array().unwrap() {
        let i = &row["input"];
        let mut stores = stores(i);
        let mut s = stores.state();
        let mut w = Host {
            input: i.clone(),
            events: vec![],
        };
        let a = Ability::new(10, (i["local"] == true).then_some(11), F32::ONE);
        payment::pay(&mut s, &a, &mut w).unwrap();
        let actual = json!({"experience":s.faction.as_ref().map(|f|f.experience.balance),"stocks":s.economy.state().stockpiled,"events":w.events,"statistics":s.faction.as_ref().and_then(|f|f.statistics)});
        assert_eq!(actual, row["output"], "{}", i["name"]);
    }
}
#[test]
fn failed_stat_keeps_xp_debit_but_stops_before_stockpile_payment() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-payment/native-payment-fixtures.json"
    ))
    .unwrap();
    let mut i = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["input"]["faction"] == true
                && r["input"]["statistics"] == true
                && r["input"]["local"] == true
                && r["input"]["xp_cost"] == 1
        })
        .unwrap()["input"]
        .clone();
    i["fail_stat"] = json!(true);
    let mut stores = stores(&i);
    let mut s = stores.state();
    let stocks = s.economy.state().stockpiled.clone();
    let xp = s.faction.as_ref().unwrap().experience.balance;
    let mut w = Host {
        input: i,
        events: vec![],
    };
    assert!(payment::pay(&mut s, &Ability::new(10, Some(11), F32::ONE), &mut w).is_err());
    assert_eq!(w.events.len(), 1);
    assert_ne!(s.faction.as_ref().unwrap().experience.balance, xp);
    assert_eq!(s.economy.state().stockpiled, stocks);
}
