//! Replays research/phase8-economy/native_transfers.py: original stock row
//! helpers, steal resources, team give-resources, faction ability
//! affordability and faction XP with multiplier, at PC24/PC53/PC64.
use aa_economy::tick::{CommanderExperience, Statistics};
use aa_economy::transfers::{
    AbilityCost, Counterparty, TributeWorld, add_kingdom_commander_experience,
    can_afford_commander_ability, give_primary_resource, steal_resources, tribute_actor,
};
use aa_economy::{Catalog, Economy, F32Bits, Limits, State, X87Precision};
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
#[derive(Deserialize)]
struct Case {
    catalog: Catalog,
    limits: Limits,
    faction_multiplier: F32Bits,
    stock: BTreeMap<String, Vec<F32Bits>>,
}
#[derive(Deserialize)]
struct Step {
    op: Value,
    expected: Expected,
}
#[derive(Deserialize)]
struct Expected {
    stock: [Vec<F32Bits>; 2],
    events: Vec<Value>,
    result: Value,
}

fn bits(value: &Value) -> F32Bits {
    F32Bits(value.as_u64().unwrap() as u32)
}
fn vector(value: &Value) -> Vec<F32Bits> {
    value.as_array().unwrap().iter().map(bits).collect()
}

fn economy(case: &Case, precision: X87Precision, which: &str) -> Economy {
    let n = case.catalog.resources.len();
    let state = State {
        raw_provided: vec![F32Bits::ZERO; n],
        effective_provided: vec![F32Bits::ZERO; n],
        consumed: vec![F32Bits::ZERO; n],
        stockpiled: case.stock[which].clone(),
    };
    Economy::new(case.catalog.clone(), state, case.limits, precision).unwrap()
}

/// Two kingdoms "K" and "K2"; returns (first, counterparty) honoring aliasing.
fn pair<'a>(
    kingdoms: &'a mut [Economy; 2],
    first: &str,
    second: &str,
) -> (&'a mut Economy, Counterparty<'a>) {
    let index = |name: &str| usize::from(name == "K2");
    let (a, b) = (index(first), index(second));
    if a == b {
        return (&mut kingdoms[a], Counterparty::Same);
    }
    let (left, right) = kingdoms.split_at_mut(1);
    if a == 0 {
        (&mut left[0], Counterparty::Other(&mut right[0]))
    } else {
        (&mut right[0], Counterparty::Other(&mut left[0]))
    }
}

struct Tribute<'a> {
    op: &'a Value,
    events: Vec<Value>,
}
impl TributeWorld for Tribute<'_> {
    fn resolve(&self, gtid: u32) -> Option<u32> {
        let slot = gtid & 0xffff;
        ((gtid >> 16) == 7 && (1..=8).contains(&slot)).then_some(slot - 1)
    }
    fn settlement_of(&mut self, actor: u32) -> Result<Option<u32>, aa_economy::EconomyError> {
        self.events
            .push(json!({"kind": "settlement_of", "actor": actor}));
        Ok((!self.op["members"].is_null()).then_some(100))
    }
    fn settlement_member_count(&mut self, _: u32) -> Result<usize, aa_economy::EconomyError> {
        Ok(self.op["members"].as_array().unwrap().len())
    }
    fn settlement_member(&mut self, _: u32, index: usize) -> Result<u32, aa_economy::EconomyError> {
        Ok(self.op["members"][index].as_u64().unwrap() as u32)
    }
    fn keeps_member(&mut self, member: u32) -> Result<bool, aa_economy::EconomyError> {
        self.events
            .push(json!({"kind": "keeps_member", "member": member}));
        Ok(self.op["kept"]
            .as_array()
            .unwrap()
            .iter()
            .any(|k| k.as_u64() == Some(u64::from(member))))
    }
    fn destroy_actor(&mut self, member: u32) -> Result<(), aa_economy::EconomyError> {
        self.events
            .push(json!({"kind": "destroy_actor", "member": member}));
        Ok(())
    }
    fn change_owner(&mut self, actor: u32, receiver: u32) -> Result<(), aa_economy::EconomyError> {
        let receiver = if receiver == 2 { "K2" } else { "K" };
        self.events
            .push(json!({"kind": "change_owner", "actor": actor, "receiver": receiver}));
        Ok(())
    }
    fn log_defeated_receiver(&mut self) -> Result<(), aa_economy::EconomyError> {
        self.events.push(json!({"kind": "log"}));
        Ok(())
    }
    fn tribute_event(&mut self, kind: u32, kingdom: u32) -> Result<(), aa_economy::EconomyError> {
        let name = if kind == 0x1f {
            "tribute_given"
        } else {
            "tribute_received"
        };
        let kingdom = if kingdom == 2 { "K2" } else { "K" };
        self.events.push(json!({"kind": name, "kingdom": kingdom}));
        Ok(())
    }
}

#[test]
fn transfers_match_original_x86_at_all_precisions() {
    let document: Document =
        serde_json::from_str(include_str!("fixtures/native-transfer-fixtures.json")).unwrap();
    let mut executions = 0;
    for fixture in document.fixtures {
        let case = &fixture.case;
        let mut kingdoms = [
            economy(case, fixture.precision, "0"),
            economy(case, fixture.precision, "1"),
        ];
        for (index, step) in fixture.steps.iter().enumerate() {
            let label = format!(
                "{} {:?} step {index} {}",
                fixture.name, fixture.precision, step.op
            );
            let op = &step.op;
            let mut events = Vec::new();
            let mut result = Value::Null;
            match op["kind"].as_str().unwrap() {
                "credit_row" | "debit_row" => {
                    let k = usize::from(op["kingdom"] == "K2");
                    let row = op["index"].as_u64().unwrap() as usize;
                    if op["kind"] == "credit_row" {
                        kingdoms[k]
                            .add_stockpile_row(row, bits(&op["amount"]))
                            .expect(&label);
                    } else {
                        kingdoms[k]
                            .subtract_stockpile_row(row, bits(&op["amount"]))
                            .expect(&label);
                    }
                }
                "steal" => {
                    let resolved = op["target"].as_bool().unwrap_or(true)
                        && op["target_alive"].as_bool().unwrap_or(true)
                        && !op["victim"].is_null();
                    if resolved {
                        let victim = op["victim"].as_str().unwrap();
                        let thief = op["thief"].as_str().unwrap();
                        let (victim, thief) = pair(&mut kingdoms, victim, thief);
                        steal_resources(&vector(&op["amounts"]), victim, thief).expect(&label);
                        result = json!(true);
                    } else {
                        events.push(json!({"kind": "log"}));
                        result = json!(false);
                    }
                }
                "give" => {
                    let giver = op["giver"].as_str().unwrap();
                    let receiver = op["receiver"].as_str().unwrap();
                    let defeated = op["defeated"].as_bool().unwrap_or(false);
                    let amount = bits(&op["amount"]);
                    let (giver_economy, other) = pair(&mut kingdoms, giver, receiver);
                    match give_primary_resource(giver_economy, other, amount, defeated)
                        .expect(&label)
                    {
                        None => {
                            events.push(json!({"kind": "log"}));
                            result = json!(amount.0);
                        }
                        Some(outcome) => {
                            events.push(json!({"kind": "resources_given", "kingdom": giver,
                                               "display": outcome.display}));
                            events.push(json!({"kind": "resources_received", "kingdom": receiver,
                                               "display": outcome.display}));
                            result = json!(outcome.amount.0);
                        }
                    }
                }
                "tribute" => {
                    let id = |name: &Value| if name == "K2" { 2 } else { 1 };
                    let mut world = Tribute { op, events: vec![] };
                    tribute_actor(
                        (7 << 16) | (op["actor"].as_u64().unwrap() as u32 + 1),
                        id(&op["giver"]),
                        id(&op["receiver"]),
                        op["defeated"].as_bool().unwrap_or(false),
                        &mut world,
                    )
                    .expect(&label);
                    events = world.events;
                }
                "ability_afford" => {
                    let ability = (!op["ability"].is_null()).then(|| AbilityCost {
                        experience: op["ability"]["experience"].as_i64().unwrap() as i32,
                        resources: vector(&op["ability"]["resources"]),
                    });
                    let faction = (!op["faction"].is_null()).then(|| CommanderExperience {
                        balance: bits(&op["faction"]["balance"]),
                        maximum: F32Bits::ZERO,
                    });
                    result = json!(
                        can_afford_commander_ability(
                            &kingdoms[0],
                            faction.as_ref(),
                            ability.as_ref()
                        )
                        .expect(&label)
                    );
                }
                "faction_xp" => {
                    let mut faction = (!op["faction"].is_null()).then(|| CommanderExperience {
                        balance: bits(&op["faction"]["balance"]),
                        maximum: bits(&op["faction"]["maximum"]),
                    });
                    let mut statistics = (!op["statistics"].is_null()).then(|| {
                        let mut scores = vec![F32Bits::ZERO; 62];
                        scores[44] = bits(&op["statistics"]);
                        Statistics {
                            production: vec![],
                            scores,
                        }
                    });
                    add_kingdom_commander_experience(
                        faction.as_mut(),
                        statistics.as_mut(),
                        bits(&op["amount"]),
                        case.faction_multiplier,
                        fixture.precision,
                    )
                    .expect(&label);
                    if let Some(faction) = faction {
                        result = json!({"balance": faction.balance.0,
                                        "score44": statistics.map(|s| s.scores[44].0)});
                    }
                }
                other => panic!("{other}"),
            }
            assert_eq!(
                kingdoms[0].state().stockpiled,
                step.expected.stock[0],
                "{label} K"
            );
            assert_eq!(
                kingdoms[1].state().stockpiled,
                step.expected.stock[1],
                "{label} K2"
            );
            assert_eq!(events, step.expected.events, "{label} events");
            assert_eq!(result, step.expected.result, "{label} result");
            executions += 1;
        }
    }
    assert_eq!(executions, document.executions);
}
