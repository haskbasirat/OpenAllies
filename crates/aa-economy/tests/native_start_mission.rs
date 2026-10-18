//! Replays research/phase8-economy/native_start_mission.py: original GKingdom
//! start-mission setup 0x4E6DB0 (with 0x4E39D0, 0x4730D0, 0x4CDF10) at
//! PC24/PC53/PC64.
use aa_economy::kingdom_start::*;
use aa_economy::tick::CommanderExperience;
use aa_economy::{
    Catalog, Economy, EconomyError, F32Bits, Limits, ResourceDefinition, ResourceKind, State,
    X87Precision,
};
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
    case: Value,
    expected: Value,
}

struct World {
    owned: Vec<u32>,
    events: Vec<Value>,
}
impl StartMissionWorld for World {
    fn has_property(&mut self, definition: u32) -> Result<bool, EconomyError> {
        self.events
            .push(json!({"kind": "has_property", "definition": definition}));
        Ok(self.owned.contains(&definition))
    }
    fn add_property(&mut self, definition: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "add_property", "definition": definition}));
        self.owned.push(definition);
        Ok(())
    }
    fn invalid_faction(&mut self) -> Result<(), EconomyError> {
        self.events.push(json!({"kind": "invalid_faction"}));
        Ok(())
    }
}

fn words(v: &Value) -> Vec<F32Bits> {
    serde_json::from_value(v.clone()).unwrap()
}
fn ids(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_u64().unwrap() as u32)
        .collect()
}
const COMMANDER_DEFINITION: u32 = 0xC0;

#[test]
fn start_mission_matches_original_x86_at_all_precisions() {
    let document: Document =
        serde_json::from_str(include_str!("fixtures/native-start-mission-fixtures.json")).unwrap();
    let mut executions = 0;
    for fixture in document.fixtures {
        let c = &fixture.case;
        let label = format!("{} {:?}", fixture.name, fixture.precision);
        let stock = words(&c["stock"]);
        let n = stock.len();
        let catalog = Catalog {
            resources: (0..n)
                .map(|i| ResourceDefinition {
                    id: format!("r{i}"),
                    kind: ResourceKind::Stockpiled,
                    limited_provider: false,
                    purchase_cost: vec![F32Bits::ZERO; n],
                })
                .collect(),
        };
        let mut state = State::zeroed(n);
        state.stockpiled = stock;
        let limits = Limits {
            provided_maximum: 100,
            stockpiled_maximum: F32Bits(c["maximum"].as_u64().unwrap() as u32),
        };
        let mut economy = Economy::new(catalog, state, limits, fixture.precision).unwrap();
        let commander = &c["commander"];
        let mut experience = (!commander.is_null()).then(|| CommanderExperience {
            balance: F32Bits(commander["balance"].as_u64().unwrap() as u32),
            maximum: F32Bits(commander["maximum"].as_u64().unwrap() as u32),
        });
        let input = StartMission {
            reset_stockpiles: c["reset"].as_bool().unwrap(),
            starting_stockpile: F32Bits(c["starting_stockpile"].as_u64().unwrap() as u32),
            starting_resources: words(&c["starting_resources"]),
            profile: (!c["profile"].is_null()).then(|| StartProfile {
                resources: words(&c["profile"]["resources"]),
                technology: Some(c["profile"]["technology"].as_u64().unwrap() as u32)
                    .filter(|&t| t != 0),
            }),
            start_technologies: ids(&c["start_technologies"])
                .into_iter()
                .map(|d| (d != 0).then_some(d))
                .collect(),
            commander_definition: (!commander.is_null()).then_some(COMMANDER_DEFINITION),
            commander_technologies: if commander.is_null() {
                vec![]
            } else {
                ids(&commander["technologies"])
            },
            nation_factions: (!c["nation"].is_null()).then(|| {
                c["nation"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|d| match d.as_str() {
                        Some("commander") => COMMANDER_DEFINITION,
                        _ => d.as_u64().unwrap() as u32,
                    })
                    .collect()
            }),
            commander_start_experience: F32Bits(c["commander_start"].as_u64().unwrap() as u32),
        };
        let mut world = World {
            owned: ids(&c["owned"]),
            events: vec![],
        };
        let started =
            start_mission(&mut economy, &input, experience.as_mut(), &mut world).expect(&label);
        let actual = json!({
            "stock": economy.state().stockpiled,
            "commander_balance": experience.map(|e| e.balance.0),
            "status": started.status,
            "started": started.started,
            "events": world.events,
        });
        assert_eq!(actual, fixture.expected, "{label}");
        executions += 1;
    }
    assert_eq!(executions, document.executions);
}
