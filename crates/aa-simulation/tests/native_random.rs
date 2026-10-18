use aa_numeric::Precision;
use aa_simulation::{NativeRng, RandomError, native_unit_float};
use serde::Deserialize;

#[derive(Deserialize)]
struct Constructors {
    address: String,
    seed: u32,
    state: u32,
}
#[derive(Deserialize)]
struct Bounded {
    state_before: u32,
    bound: u32,
    value: u32,
    state_after: u32,
}
#[derive(Deserialize)]
struct Converted {
    precision: Precision,
    value: u32,
    result_bits: u32,
}
#[derive(Deserialize)]
struct Drawn {
    precision: Precision,
    state_before: u32,
    state_after: u32,
    result_bits: u32,
}
#[derive(Deserialize)]
struct Oracle {
    constructors: Vec<Constructors>,
    bounded: Vec<Bounded>,
    converted: Vec<Converted>,
    drawn: Vec<Drawn>,
}
fn oracle() -> Oracle {
    serde_json::from_str(include_str!(
        "../../../research/phase4-rng/native-rng-vectors.json"
    ))
    .unwrap()
}

#[test]
fn native_constructors_and_bounded_draws_match_original_instructions() {
    let oracle = oracle();
    assert_eq!(oracle.constructors.len(), 330);
    assert_eq!(oracle.bounded.len(), 1100);
    for row in oracle.constructors {
        let state = if row.address == "0x85d870" {
            NativeRng::default()
        } else {
            let mut state = NativeRng::new(0xdeadbeef);
            state.reseed(row.seed);
            assert_eq!(state, NativeRng::from_native_seed(row.seed));
            state
        };
        assert_eq!(
            state.state, row.state,
            "{} seed {:08x}",
            row.address, row.seed
        );
    }
    for row in oracle.bounded {
        let mut state = NativeRng::new(row.state_before);
        assert_eq!(state.next_bounded(row.bound).unwrap(), row.value);
        assert_eq!(state.state, row.state_after);
    }
}

#[test]
fn explicit_float_stores_match_native_across_all_precision_modes() {
    let oracle = oracle();
    assert_eq!(oracle.converted.len(), 9585);
    assert_eq!(oracle.drawn.len(), 330);
    for row in oracle.converted {
        assert_eq!(
            native_unit_float(row.value, row.precision).unwrap().0,
            row.result_bits,
            "{:?} input {:08x}",
            row.precision,
            row.value
        );
    }
    for row in oracle.drawn {
        let mut state = NativeRng::new(row.state_before);
        assert_eq!(state.next_f32(row.precision).unwrap().0, row.result_bits);
        assert_eq!(state.state, row.state_after);
    }
}

#[test]
fn invalid_bound_does_not_consume_rng_and_raw_restore_stays_distinct() {
    let mut state = NativeRng::new(0);
    assert!(matches!(state.next_bounded(0), Err(RandomError::ZeroBound)));
    assert_eq!(state.state, 0);
    assert_ne!(state, NativeRng::from_native_seed(0));
    for precision in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        assert_eq!(native_unit_float(0, precision).unwrap().0, 0);
        assert_eq!(
            native_unit_float(u32::MAX, precision).unwrap().as_f32(),
            1.0
        );
    }
}
