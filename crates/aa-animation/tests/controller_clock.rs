use aa_animation::controller_clock::{ClockCache, Controller, ControllerClock};
use aa_body::F32;
use aa_numeric::Precision;
#[test]
fn native_controller_clock_matches_rate_phase_wrap_clamp_and_reverse_sequences() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-live-animation/native-controller-clock.json"
    ))
    .unwrap();
    assert_eq!(f["cases"].as_array().unwrap().len(), 72);
    for row in f["cases"].as_array().unwrap() {
        let p: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let mut c = ControllerClock {
            accumulated: F32(7f32.to_bits()),
            ..Default::default()
        };
        let mut cache = ClockCache::default();
        for step in row["steps"].as_array().unwrap() {
            let controller: Controller =
                serde_json::from_value(step["controller"].clone()).unwrap();
            let got = c
                .update(
                    F32(step["input"].as_u64().unwrap() as u32),
                    controller,
                    &mut cache,
                    p,
                )
                .unwrap();
            assert_eq!(
                got.0,
                step["output"].as_u64().unwrap() as u32,
                "{step} at {p:?}"
            );
            assert_eq!(serde_json::to_value(&c).unwrap(), step["state"], "{step}");
        }
    }
}
#[test]
fn controller_rejection_and_reset_preserve_native_owned_state() {
    assert!(
        aa_animation::controller_clock::sequence_time(
            F32::ZERO,
            F32(0x7f7fffff),
            Precision::Double53
        )
        .is_err()
    );
    let mut state = ControllerClock::default();
    let mut cache = ClockCache::default();
    let c = Controller {
        flags: 44,
        frequency: F32::ONE,
        phase: F32::ZERO,
        start: F32::ZERO,
        stop: F32::ONE,
    };
    state
        .update(F32(0.2f32.to_bits()), c, &mut cache, Precision::Double53)
        .unwrap();
    let old = state.clone();
    assert!(
        state
            .update(
                F32::ZERO,
                Controller { flags: 45, ..c },
                &mut cache,
                Precision::Double53
            )
            .is_err()
    );
    assert_eq!(state, old);
    state.reset();
    assert_eq!(state.accumulated, old.accumulated);
    assert_eq!(
        state
            .update(F32::ZERO, c, &mut cache, Precision::Double53)
            .unwrap(),
        F32::ZERO
    );
}
