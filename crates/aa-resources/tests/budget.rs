use aa_resources::*;

fn snapshot() -> MemorySnapshot {
    MemorySnapshot {
        sampled_unix_ms: 0,
        physical_total_bytes: 10_000,
        physical_available_bytes: 8_000,
        system_commit_limit_bytes: 20_000,
        system_committed_bytes: 10_000,
        process_available_commit_bytes: 10_000,
        process_virtual_total_bytes: 100_000,
        process_virtual_available_bytes: 90_000,
        page_size_bytes: 4096,
    }
}
fn estimate(bytes: u64) -> MemoryEstimate {
    MemoryEstimate {
        resident_cpu_bytes: bytes,
        ..Default::default()
    }
}
fn no_reserves() -> MemoryPolicy {
    MemoryPolicy {
        physical_headroom_bps: 0,
        commit_headroom_bps: 0,
        estimation_overhead_bps: 0,
    }
}

#[test]
fn reserves_derive_from_observed_hardware_and_checked_peak() {
    let report = assess(
        &snapshot(),
        MemoryPolicy::default(),
        MemoryEstimate {
            resident_cpu_bytes: 1000,
            temporary_cpu_bytes: 100,
            upload_staging_bytes: 200,
            gpu_bytes: 9000,
        },
    )
    .unwrap();
    assert_eq!(report.estimated_cpu_peak_bytes, 1300);
    assert_eq!(report.adjusted_cpu_peak_bytes, 1560);
    assert_eq!(report.physical_reserved_bytes, 1000);
    assert_eq!(report.commit_reserved_bytes, 2000);
    assert_eq!(report.physical_budget_bytes, 7000);
    assert_eq!(report.system_commit_budget_bytes, 8000);
    assert_eq!(report.effective_cpu_budget_bytes, 7000);
    assert!(report.fits());
    assert_eq!(report.estimate.gpu_bytes, 9000); // DedicatedVRAM is not charged against CPU estimate.
    let mut larger = snapshot();
    larger.physical_total_bytes *= 2;
    larger.physical_available_bytes *= 2;
    larger.system_commit_limit_bytes *= 2;
    larger.system_committed_bytes *= 2;
    larger.process_available_commit_bytes *= 2;
    assert_eq!(
        assess(&larger, MemoryPolicy::default(), estimate(0))
            .unwrap()
            .effective_cpu_budget_bytes,
        14000
    );
}
#[test]
fn exact_fit_succeeds_one_extra_byte_fails() {
    let report = assess(&snapshot(), no_reserves(), estimate(8000)).unwrap();
    assert!(report.ensure_fits().is_ok());
    let report = assess(&snapshot(), no_reserves(), estimate(8001)).unwrap();
    assert_eq!(report.exceeded, [MemoryConstraint::PhysicalHeadroom]);
    assert!(report.ensure_fits().is_err());
}
#[test]
fn process_commit_and_address_space_are_independent_constraints() {
    let mut sample = snapshot();
    sample.process_available_commit_bytes = 500;
    sample.process_virtual_available_bytes = 400;
    let report = assess(&sample, no_reserves(), estimate(501)).unwrap();
    assert_eq!(report.effective_cpu_budget_bytes, 400);
    assert_eq!(
        report.exceeded,
        [
            MemoryConstraint::ProcessCommit,
            MemoryConstraint::ProcessAddressSpace
        ]
    );
}
#[test]
fn system_commit_and_physical_pressure_saturate_without_underflow() {
    let mut sample = snapshot();
    sample.system_committed_bytes = 20_001;
    sample.physical_available_bytes = 5;
    let report = assess(&sample, MemoryPolicy::default(), estimate(1)).unwrap();
    assert_eq!(report.system_commit_budget_bytes, 0);
    assert_eq!(report.physical_budget_bytes, 0);
    assert_eq!(
        report.exceeded,
        [
            MemoryConstraint::PhysicalHeadroom,
            MemoryConstraint::SystemCommitHeadroom
        ]
    );
    assert!(
        assess(&sample, MemoryPolicy::default(), estimate(0))
            .unwrap()
            .fits()
    );
}
#[test]
fn invalid_policy_and_observations_fail_explicitly() {
    let mut p = no_reserves();
    p.physical_headroom_bps = 10_001;
    assert!(assess(&snapshot(), p, estimate(1)).is_err());
    let mut s = snapshot();
    s.physical_available_bytes = s.physical_total_bytes + 1;
    assert!(assess(&s, no_reserves(), estimate(1)).is_err());
    let mut s = snapshot();
    s.page_size_bytes = 0;
    assert!(s.validate().is_err());
}
#[test]
fn all_estimation_overflows_are_errors() {
    assert!(count_bytes(u64::MAX, 2).is_err());
    assert!(
        MemoryEstimate {
            resident_cpu_bytes: u64::MAX,
            temporary_cpu_bytes: 1,
            ..Default::default()
        }
        .peak_cpu_bytes()
        .is_err()
    );
    assert!(assess(&snapshot(), MemoryPolicy::default(), estimate(u64::MAX)).is_err());
    assert!(rgba8_mip_chain_bytes(u32::MAX, u32::MAX).is_err());
}
#[test]
fn integer_fraction_rounds_up_at_tiny_and_large_values() {
    let mut policy = no_reserves();
    policy.estimation_overhead_bps = 1;
    assert_eq!(
        assess(&snapshot(), policy, estimate(1))
            .unwrap()
            .adjusted_cpu_peak_bytes,
        2
    );
    let mut s = snapshot();
    s.physical_total_bytes = u64::MAX;
    s.physical_available_bytes = u64::MAX;
    policy.physical_headroom_bps = 10_000;
    policy.estimation_overhead_bps = 0;
    assert_eq!(
        assess(&s, policy, estimate(0))
            .unwrap()
            .physical_reserved_bytes,
        u64::MAX
    );
}
#[test]
fn mip_estimate_matches_actual_non_square_and_odd_level_dimensions() {
    assert_eq!(rgba8_mip_chain_bytes(1, 1).unwrap(), 4);
    assert_eq!(rgba8_mip_chain_bytes(4, 2).unwrap(), 32 + 8 + 4);
    assert_eq!(rgba8_mip_chain_bytes(3, 7).unwrap(), 84 + 12 + 4);
    assert!(rgba8_mip_chain_bytes(0, 1).is_err());
}
