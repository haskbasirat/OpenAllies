//! Replays research/phase8-economy/native_tick.py: the complete original
//! 0x4E9C40 GKingdom economy Tick executed under Unicorn at PC24/PC53/PC64.
use aa_economy::tick::{
    CommanderExperience, CommanderParts, OwnerStatistics, RecruitSingles, Statistics, TickInput,
    TickParts, TickState, TickWorld,
};
use aa_economy::{Catalog, Economy, EconomyError, F32Bits, Limits, State, X87Precision};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Fixtures {
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
struct Settings {
    commander_experience_rate: F32Bits,
}
#[derive(Deserialize, Clone)]
struct Faction {
    balance: F32Bits,
    maximum: F32Bits,
    owner: String,
}
#[derive(Deserialize)]
struct Case {
    catalog: Catalog,
    limits: Limits,
    settings: Settings,
    economy: State,
    gtid_table: BTreeMap<String, (u16, bool)>,
    tick: TickState,
    singles: RecruitSingles,
    faction: Option<Faction>,
    faction_owner_statistics: Option<Statistics>,
    statistics: Option<Statistics>,
}
#[derive(Deserialize)]
struct Input {
    seconds: F32Bits,
    world_time: F32Bits,
}
#[derive(Deserialize)]
struct Snapshot {
    economy: State,
    tick: TickState,
    singles: RecruitSingles,
    events: Vec<Value>,
    faction: Option<Faction>,
    statistics: Option<Statistics>,
    faction_owner_statistics: Option<Statistics>,
}
#[derive(Deserialize)]
struct Step {
    input: Input,
    expected: Snapshot,
}

struct World {
    gtids: BTreeMap<u32, (u16, bool)>,
    events: Vec<Value>,
    fail_on: Option<&'static str>,
}
impl World {
    fn event(&mut self, value: Value) -> Result<(), EconomyError> {
        let kind = value["kind"].as_str().unwrap_or_default().to_owned();
        self.events.push(value);
        if self.fail_on == Some(kind.as_str()) {
            return Err(EconomyError::World(kind));
        }
        Ok(())
    }
}
impl TickWorld for World {
    fn gtid_live(&mut self, gtid: u32) -> Result<bool, EconomyError> {
        let (generation, live) = self
            .gtids
            .get(&(gtid & 0xffff))
            .copied()
            .unwrap_or((0, false));
        Ok(live && generation == (gtid >> 16) as u16)
    }
    fn upkeep_shortage(&mut self, percent: i32) -> Result<(), EconomyError> {
        self.event(json!({"kind": "upkeep_shortage", "percent": percent}))
    }
    fn upkeep_repaired(&mut self) -> Result<(), EconomyError> {
        self.event(json!({"kind": "upkeep_repaired"}))
    }
    fn team_victory_check(&mut self) -> Result<(), EconomyError> {
        self.event(json!({"kind": "team_victory_check"}))
    }
    fn percentage_victory_check(&mut self) -> Result<(), EconomyError> {
        self.event(json!({"kind": "percentage_victory_check"}))
    }
    fn statistics_snapshot(&mut self) -> Result<(), EconomyError> {
        self.event(json!({"kind": "statistics_snapshot"}))
    }
}

fn fixtures() -> Fixtures {
    serde_json::from_str(include_str!("fixtures/native-tick-fixtures.json")).unwrap()
}

struct Kingdom {
    economy: Economy,
    tick: TickState,
    singles: RecruitSingles,
    faction: Option<(CommanderExperience, String)>,
    statistics: Option<Statistics>,
    owner_statistics: Option<Statistics>,
    world: World,
}
impl Kingdom {
    fn new(fixture_precision: X87Precision, case: Case) -> Self {
        let economy =
            Economy::new(case.catalog, case.economy, case.limits, fixture_precision).unwrap();
        Self {
            economy,
            tick: case.tick,
            singles: case.singles,
            faction: case.faction.map(|f| {
                (
                    CommanderExperience {
                        balance: f.balance,
                        maximum: f.maximum,
                    },
                    f.owner,
                )
            }),
            statistics: case.statistics,
            owner_statistics: case.faction_owner_statistics,
            world: World {
                gtids: case
                    .gtid_table
                    .into_iter()
                    .map(|(k, v)| (k.parse().unwrap(), v))
                    .collect(),
                events: Vec::new(),
                fail_on: None,
            },
        }
    }
    fn tick(&mut self, input: TickInput) -> Result<(), EconomyError> {
        let faction = self
            .faction
            .as_mut()
            .map(|(experience, owner)| CommanderParts {
                experience,
                owner_statistics: match owner.as_str() {
                    "Kingdom" => OwnerStatistics::Kingdom,
                    "Other" => match self.owner_statistics.as_mut() {
                        Some(stats) => OwnerStatistics::Other(stats),
                        None => OwnerStatistics::Absent,
                    },
                    _ => OwnerStatistics::Absent,
                },
            });
        let parts = TickParts {
            state: &mut self.tick,
            singles: &mut self.singles,
            commander: faction,
            statistics: self.statistics.as_mut(),
        };
        self.economy.tick(input, parts, &mut self.world).map(|_| ())
    }
}

#[test]
fn complete_economy_tick_matches_original_x86_at_all_precisions() {
    let fixtures = fixtures();
    let mut executions = 0;
    let mut events = 0;
    for fixture in fixtures.fixtures {
        let rate = fixture.case.settings.commander_experience_rate;
        let mut kingdom = Kingdom::new(fixture.precision, fixture.case);
        for (index, step) in fixture.steps.into_iter().enumerate() {
            let label = format!("{} {:?} step {index}", fixture.name, fixture.precision);
            kingdom.world.events.clear();
            kingdom
                .tick(TickInput {
                    seconds: step.input.seconds,
                    world_time: step.input.world_time,
                    commander_experience_rate: rate,
                })
                .expect(&label);
            let expected = step.expected;
            assert_eq!(
                kingdom.economy.state(),
                &expected.economy,
                "{label} economy"
            );
            assert_eq!(kingdom.tick, expected.tick, "{label} tick state");
            assert_eq!(kingdom.singles, expected.singles, "{label} recruit singles");
            assert_eq!(kingdom.world.events, expected.events, "{label} events");
            assert_eq!(
                kingdom.faction.as_ref().map(|f| f.0.balance),
                expected.faction.as_ref().map(|f| f.balance),
                "{label} faction balance"
            );
            assert_eq!(
                kingdom.statistics, expected.statistics,
                "{label} statistics"
            );
            assert_eq!(
                kingdom.owner_statistics, expected.faction_owner_statistics,
                "{label} owner statistics"
            );
            executions += 1;
            events += expected.events.len();
        }
    }
    assert_eq!(executions, fixtures.executions);
    assert!(
        executions >= 1000 && events >= 1500,
        "{executions} {events}"
    );
}

#[test]
fn ratio_stage_failure_does_not_commit_flow() {
    // Overdrawn stock with a zero consumption denominator: the native ratio is
    // infinite/NaN, which is outside the supported finite domain.
    let catalog: Catalog = serde_json::from_value(json!({"resources": [
        {"id": "money", "kind": "Stockpiled", "limited_provider": false, "purchase_cost": [0]}
    ]}))
    .unwrap();
    let state = State {
        raw_provided: vec![F32Bits::from_f32(-5.0)],
        effective_provided: vec![F32Bits::from_f32(-5.0)],
        consumed: vec![F32Bits::ZERO],
        stockpiled: vec![F32Bits::from_f32(0.01)],
    };
    let limits = Limits {
        provided_maximum: 100,
        stockpiled_maximum: F32Bits::from_f32(10000.0),
    };
    let mut economy = Economy::new(catalog, state.clone(), limits, X87Precision::Double53).unwrap();
    let mut tick = TickState {
        suppress_overdrawn: false,
        shortage_ratio: F32Bits::ZERO,
        announced_ratio: F32Bits::ZERO,
        snapshot_timer: F32Bits::ZERO,
    };
    let mut singles = RecruitSingles {
        charging: -1,
        items: vec![],
    };
    let mut world = World {
        gtids: BTreeMap::new(),
        events: vec![],
        fail_on: None,
    };
    let result = economy.tick(
        TickInput {
            seconds: F32Bits::from_f32(1.0),
            world_time: F32Bits::from_f32(2.0),
            commander_experience_rate: F32Bits::ZERO,
        },
        TickParts {
            state: &mut tick,
            singles: &mut singles,
            commander: None,
            statistics: None,
        },
        &mut world,
    );
    assert_eq!(result.unwrap_err(), EconomyError::DivisionByZero);
    assert_eq!(economy.state(), &state);
    assert!(world.events.is_empty());
}

#[test]
fn callback_failure_keeps_native_effects_already_applied() {
    let fixtures = fixtures();
    let fixture = fixtures
        .fixtures
        .into_iter()
        .find(|f| f.name == "repair-3ecccccd" && f.precision == X87Precision::Extended64)
        .unwrap();
    let rate = fixture.case.settings.commander_experience_rate;
    let step = &fixture.steps[0];
    let (seconds, world_time) = (step.input.seconds, step.input.world_time);
    let expected_economy = step.expected.economy.clone();
    let mut kingdom = Kingdom::new(fixture.precision, fixture.case);
    kingdom.world.fail_on = Some("upkeep_repaired");
    let error = kingdom
        .tick(TickInput {
            seconds,
            world_time,
            commander_experience_rate: rate,
        })
        .unwrap_err();
    assert_eq!(error, EconomyError::World("upkeep_repaired".into()));
    // Flow and clamp were applied before the event; the ratio fields were not
    // yet cleared and no later stage ran.
    assert_eq!(kingdom.economy.state(), &expected_economy);
    assert_ne!(kingdom.tick.shortage_ratio, F32Bits::ZERO);
    assert_eq!(kingdom.world.events.len(), 1);
}
