use aa_simulation::{
    Batch, Command, CommandResult, Core, FloatBits, NativeClock, NativeRng, ProviderState, Snapshot,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Vectors {
    rng: Vec<RngCase>,
    clock: Vec<ClockCase>,
    providers: Vec<ProviderCase>,
}
#[derive(Deserialize)]
struct RngCase {
    seed: u32,
    state: u32,
    output: u32,
}
#[derive(Deserialize)]
struct ClockCase {
    tick_before: u32,
    step_bits: u32,
    old_end_bits: u32,
    tick_after: u32,
    begin_bits: u32,
    end_bits: u32,
}
#[derive(Deserialize)]
struct ProviderCase {
    old_raw_bits: u32,
    delta_bits: u32,
    maximum: i32,
    limited: bool,
    raw_bits: u32,
    effective_bits: u32,
}
fn vectors() -> Vectors {
    serde_json::from_str(include_str!("fixtures/native-rule-vectors.json")).unwrap()
}
fn f(value: f32) -> FloatBits {
    FloatBits::from_f32(value)
}
fn rows() -> Vec<ProviderState> {
    vec![
        ProviderState::new(f(0.0), true, 20).unwrap(),
        ProviderState::new(f(100.0), true, 100).unwrap(),
        ProviderState::new(FloatBits(0x8000_0000), false, 20).unwrap(),
        ProviderState::new(f(4.0), false, 20).unwrap(),
    ]
}

#[test]
fn original_x86_rng_vectors_match_state_and_full_output() {
    let cases = vectors().rng;
    assert_eq!(cases.len(), 74);
    for case in cases {
        let mut rng = NativeRng::new(case.seed);
        assert_eq!(rng.next_u32(), case.output, "seed {}", case.seed);
        assert_eq!(rng.state, case.state);
    }
}

#[test]
fn original_x86_clock_fragment_matches_ieee_bits_and_signed_wrap() {
    let cases = vectors().clock;
    assert_eq!(cases.len(), 84);
    for case in cases {
        let mut clock = NativeClock::with_step(FloatBits(case.step_bits)).unwrap();
        clock.tick = case.tick_before;
        clock.end = FloatBits(case.old_end_bits);
        clock.advance_fragment();
        assert_eq!(
            (clock.tick, clock.begin.0, clock.end.0),
            (case.tick_after, case.begin_bits, case.end_bits),
            "tick {} step {:08x}",
            case.tick_before,
            case.step_bits
        );
    }
}

#[test]
fn original_x86_provider_projection_matches_all_finite_fixture_bits() {
    let cases = vectors().providers;
    assert_eq!(cases.len(), 59);
    for case in cases {
        let state = ProviderState::new(FloatBits(case.old_raw_bits), case.limited, case.maximum)
            .unwrap()
            .accumulated(FloatBits(case.delta_bits))
            .unwrap();
        assert_eq!(
            (state.raw.0, state.effective.0),
            (case.raw_bits, case.effective_bits),
            "old {:08x} delta {:08x}",
            case.old_raw_bits,
            case.delta_bits
        );
    }
}

fn workload(sequence: u64) -> Batch {
    let mut commands = Vec::new();
    for index in 0..97 {
        if index % 7 == 0 {
            commands.push(Command::DrawRandom);
        }
        let slot = (index * 3 + sequence as usize) % 4;
        let delta = if index % 3 == 0 { -0.125 } else { 0.0625 };
        commands.push(Command::AccumulateProvider {
            slot,
            delta: f(delta),
        });
    }
    Batch { sequence, commands }
}

#[test]
fn snapshots_and_receipt_order_are_identical_for_one_two_four_and_eight_workers() {
    let mut reference = Core::new(0x1234_abcd, rows(), 1).unwrap();
    let mut expected = Vec::new();
    for sequence in 0..128 {
        let receipt = reference.apply_batch(workload(sequence)).unwrap();
        reference.advance_clock_fragment();
        expected.push((receipt, reference.snapshot()));
    }
    for workers in [1, 2, 4, 8] {
        let mut candidate = Core::new(0x1234_abcd, rows(), workers).unwrap();
        for (sequence, (receipt, snapshot)) in expected.iter().enumerate() {
            assert_eq!(
                &candidate.apply_batch(workload(sequence as u64)).unwrap(),
                receipt,
                "workers {workers} batch {sequence}"
            );
            candidate.advance_clock_fragment();
            assert_eq!(&candidate.snapshot(), snapshot);
            assert_eq!(
                candidate.snapshot().canonical_bytes(),
                snapshot.canonical_bytes()
            );
            assert_eq!(candidate.snapshot().sha256(), snapshot.sha256());
        }
    }
}

#[test]
fn nonassociative_updates_and_rng_results_stay_in_authoritative_command_order() {
    let mut core = Core::new(1, rows(), 4).unwrap();
    let mut rng = NativeRng::new(1);
    let receipt = core
        .apply_batch(Batch {
            sequence: 0,
            commands: vec![
                Command::AccumulateProvider {
                    slot: 0,
                    delta: f(16777216.0),
                },
                Command::DrawRandom,
                Command::AccumulateProvider {
                    slot: 1,
                    delta: f(1.0),
                },
                Command::AccumulateProvider {
                    slot: 0,
                    delta: f(1.0),
                },
                Command::DrawRandom,
                Command::AccumulateProvider {
                    slot: 0,
                    delta: f(-16777216.0),
                },
            ],
        })
        .unwrap();
    assert_eq!(receipt.results[1], CommandResult::Random(rng.next_u32()));
    assert_eq!(receipt.results[4], CommandResult::Random(rng.next_u32()));
    assert_eq!(core.snapshot().providers[0].raw, f(0.0)); // regrouping arithmetic would incorrectly produce one
    assert_eq!(core.snapshot().providers[1].raw, f(101.0));
    assert_eq!(core.snapshot().providers[1].effective, f(100.0));
}

#[test]
fn invalid_sequence_slot_or_nonfinite_job_does_not_partially_mutate_state() {
    let mut core = Core::new(42, rows(), 4).unwrap();
    let before = core.snapshot();
    for batch in [
        Batch {
            sequence: 2,
            commands: vec![Command::DrawRandom],
        },
        Batch {
            sequence: 0,
            commands: vec![
                Command::DrawRandom,
                Command::AccumulateProvider {
                    slot: 999,
                    delta: f(1.0),
                },
            ],
        },
        Batch {
            sequence: 0,
            commands: vec![
                Command::AccumulateProvider {
                    slot: 0,
                    delta: f(1.0),
                },
                Command::DrawRandom,
                Command::AccumulateProvider {
                    slot: 1,
                    delta: FloatBits(0x7fc0_0001),
                },
            ],
        },
    ] {
        assert!(core.apply_batch(batch).is_err());
        assert_eq!(core.snapshot(), before);
    }
    core.apply_batch(Batch {
        sequence: 0,
        commands: vec![],
    })
    .unwrap();
    let advanced = core.snapshot();
    assert!(
        core.apply_batch(Batch {
            sequence: 0,
            commands: vec![]
        })
        .is_err()
    );
    assert_eq!(core.snapshot(), advanced);
}

#[test]
fn restore_preserves_bits_and_continuation_across_worker_counts() {
    let mut first = Core::new(77, rows(), 2).unwrap();
    for sequence in 0..11 {
        first.apply_batch(workload(sequence)).unwrap();
        first.advance_clock_fragment();
    }
    let saved = first.snapshot();
    let json = serde_json::to_string(&saved).unwrap();
    let decoded: Snapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, saved);
    let mut restored = Core::restore(decoded, 8).unwrap();
    for sequence in 11..30 {
        assert_eq!(
            first.apply_batch(workload(sequence)).unwrap(),
            restored.apply_batch(workload(sequence)).unwrap()
        );
        first.advance_clock_fragment();
        restored.advance_clock_fragment();
        assert_eq!(first.snapshot(), restored.snapshot());
    }
    let mut invalid = saved;
    invalid.providers[0].effective = f(12345.0);
    assert!(Core::restore(invalid, 1).is_err());
    assert!(Core::new(0, vec![], 0).is_err());
}

#[test]
fn canonical_snapshots_retain_signed_zero_and_exclude_worker_identity() {
    let positive = Core::new(0, vec![ProviderState::new(f(0.0), false, 20).unwrap()], 1)
        .unwrap()
        .snapshot();
    let negative = Core::new(
        0,
        vec![ProviderState::new(FloatBits(0x8000_0000), false, 20).unwrap()],
        1,
    )
    .unwrap()
    .snapshot();
    assert_ne!(positive.canonical_bytes(), negative.canonical_bytes());
    assert_ne!(positive.sha256(), negative.sha256());
    assert_eq!(
        positive,
        Core::restore(positive.clone(), 8).unwrap().snapshot()
    );
    assert!(
        positive
            .canonical_bytes()
            .starts_with(b"AA-RUST-LAB\0\x01\0\0\0")
    );
}
