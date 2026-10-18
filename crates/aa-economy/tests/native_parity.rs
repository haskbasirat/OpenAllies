use aa_economy::{
    Catalog, Economy, EconomyError, F32Bits, FlowArithmetic, Limits, MutationTrace, State,
    X87Precision,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixtures {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    name: String,
    precision: X87Precision,
    catalog: Catalog,
    limits: Limits,
    initial: State,
    steps: Vec<Step>,
}
#[derive(Deserialize)]
struct Step {
    action: Action,
    expected: Expected,
}
#[derive(Deserialize)]
struct Expected {
    state: State,
    trace: MutationTrace,
    affordable: Option<bool>,
    flow: Option<FlowArithmetic>,
}
#[derive(Deserialize)]
#[serde(tag = "kind")]
enum Action {
    Provider {
        index: usize,
        delta: F32Bits,
    },
    Providers {
        values: Vec<F32Bits>,
        add: bool,
    },
    Consumption {
        values: Vec<F32Bits>,
        add: bool,
    },
    Credit {
        values: Vec<F32Bits>,
    },
    Debit {
        values: Vec<F32Bits>,
    },
    Afford {
        values: Vec<F32Bits>,
    },
    Flow {
        seconds: F32Bits,
        suppress_overdrawn: bool,
    },
}

fn fixtures() -> Fixtures {
    serde_json::from_str(include_str!("fixtures/economy-native-vectors.json")).unwrap()
}

#[test]
fn all_coupled_states_and_shortage_return_snapshots_match_original_x86() {
    let fixtures = fixtures();
    assert_eq!(fixtures.cases.len(), 150);
    let mut operations = 0;
    for case in fixtures.cases {
        let mut economy =
            Economy::new(case.catalog, case.initial, case.limits, case.precision).unwrap();
        for (index, step) in case.steps.into_iter().enumerate() {
            let label = format!("{} {:?} operation {index}", case.name, case.precision);
            let mut trace = MutationTrace::default();
            match step.action {
                Action::Provider { index, delta } => {
                    trace = economy.adjust_provider(index, delta).expect(&label)
                }
                Action::Providers { values, add } => {
                    trace = economy.adjust_providers(&values, add).expect(&label)
                }
                Action::Consumption { values, add } => {
                    trace = economy.adjust_consumption(&values, add).expect(&label)
                }
                Action::Credit { values } => economy.credit_stockpiles(&values).expect(&label),
                Action::Debit { values } => economy.debit_stockpiles(&values).expect(&label),
                Action::Afford { values } => assert_eq!(
                    Some(economy.can_afford(&values).expect(&label)),
                    step.expected.affordable,
                    "{label}"
                ),
                Action::Flow {
                    seconds,
                    suppress_overdrawn,
                } => assert_eq!(
                    Some(
                        economy
                            .flow_arithmetic(seconds, suppress_overdrawn)
                            .expect(&label)
                    ),
                    step.expected.flow,
                    "{label}"
                ),
            }
            assert_eq!(economy.state(), &step.expected.state, "{label}");
            assert_eq!(trace, step.expected.trace, "{label}");
            operations += 1;
        }
    }
    assert_eq!(operations, 960);
}

#[test]
fn native_shortage_example_removes_old_money_charge_before_reapplying() {
    let case = fixtures()
        .cases
        .into_iter()
        .find(|case| {
            case.name == "ammo_oil_shortages_and_expanded_caps"
                && case.precision == X87Precision::Extended64
        })
        .unwrap();
    assert_eq!(case.initial.consumed[0].as_f32(), 25.0); // base 3 + ammo 5*2 + oil 4*3
    let mut economy =
        Economy::new(case.catalog, case.initial, case.limits, case.precision).unwrap();
    let trace = economy.adjust_provider(1, F32Bits::from_f32(3.0)).unwrap();
    assert_eq!(trace.shortage_steps.len(), 2);
    assert!(!trace.shortage_steps[0].add);
    assert_eq!(trace.shortage_steps[0].consumed[0].as_f32(), 3.0);
    assert!(trace.shortage_steps[1].add);
    assert_eq!(economy.state().consumed[0].as_f32(), 19.0); // base 3 + ammo 2*2 + oil 4*3
    assert_eq!(economy.state().effective_provided[1].as_f32(), 13.0);
}

#[test]
fn selected_precision_is_observable_and_cannot_be_silently_collapsed() {
    let fixtures = fixtures();
    let differences = fixtures
        .cases
        .chunks_exact(3)
        .filter(|modes| {
            modes[0]
                .steps
                .iter()
                .zip(&modes[2].steps)
                .any(|(single, extended)| {
                    single.expected.state != extended.expected.state
                        || single.expected.flow != extended.expected.flow
                })
        })
        .count();
    assert!(
        differences > 10,
        "oracle must exercise actual x87 precision differences"
    );
    let boundary: Vec<_> = fixtures
        .cases
        .iter()
        .filter(|case| case.name == "double_rounding_boundary")
        .collect();
    assert_ne!(
        boundary[1].initial.consumed, boundary[2].initial.consumed,
        "oracle setup must distinguish actual 53-bit versus 64-bit multiply/add rounding"
    );
    assert_eq!(X87Precision::Single24.control_word(), 0x007f);
    assert_eq!(X87Precision::Double53.control_word(), 0x027f);
    assert_eq!(X87Precision::Extended64.control_word(), 0x037f);
}

#[test]
fn invalid_or_nonfinite_operations_are_rejected_without_partial_state_commit() {
    let case = fixtures().cases.remove(0);
    let mut economy =
        Economy::new(case.catalog, case.initial, case.limits, case.precision).unwrap();
    let before = economy.state().clone();
    assert_eq!(
        economy.adjust_provider(999, F32Bits::ZERO),
        Err(EconomyError::ResourceIndex)
    );
    assert_eq!(
        economy.adjust_provider(0, F32Bits(0x7fc0_0000)),
        Err(EconomyError::NonFiniteInput)
    );
    assert_eq!(
        economy.adjust_consumption(&[], true),
        Err(EconomyError::Shape)
    );
    assert_eq!(economy.state(), &before);
    let mut overflow = before.clone();
    overflow.raw_provided[0] = F32Bits(0x7f7f_ffff);
    overflow.effective_provided[0] = F32Bits(0x7f7f_ffff);
    let mut economy = Economy::new(
        economy.catalog().clone(),
        overflow.clone(),
        case.limits,
        case.precision,
    )
    .unwrap();
    assert_eq!(
        economy.adjust_provider(0, F32Bits(0x7f7f_ffff)),
        Err(EconomyError::NonFiniteResult)
    );
    assert_eq!(economy.state(), &overflow);
}
