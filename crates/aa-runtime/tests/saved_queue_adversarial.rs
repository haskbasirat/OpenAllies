//! Independent reviewer coverage for checked saved queue/table boundaries.
use aa_numeric::Precision;
use aa_runtime::{saved_queues, updatables::Table};
use aa_world::initialization::ClockPlan;
fn words(values: &[i32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn payload() -> Vec<u8> {
    words(&[
        2, //bound table count
        0, 0, 2, //deferred3 version/current/bucket count
        4, -1, 10, 0, 11, 1, 12, 0, 13, //null ID, boundID0, nullslot1, boundID0
        -7, //native signed list loop skips a negative count
        0, 1, 2, 0, 0, //deferred4 with two empty buckets
    ])
}
#[test]
fn negative_list_count_and_null_references_do_not_reorder_valid_events() {
    let mut table = Table::new(2).unwrap();
    table.register(0, 0_u64).unwrap();
    let decoded = saved_queues::decode(&payload(), 21, &table, |&v| v).unwrap();
    assert_eq!(decoded.discarded_updates, 2);
    assert_eq!(
        decoded.queues[0].buckets,
        vec![vec![(0, 11), (0, 13)], vec![]]
    );
    let plan = ClockPlan::from_tick_rate(16f32.to_bits(), Precision::Double53).unwrap();
    let mut queues = plan
        .initialize_queues(plan.new_clock(), Precision::Double53)
        .unwrap();
    decoded.install(&mut queues).unwrap();
    assert_eq!(
        queues.snapshot()[3].buckets,
        vec![vec![(0, 11), (0, 13)], vec![]]
    );
    assert_eq!(queues.snapshot()[4].current, 1);
}
#[test]
fn every_truncated_prefix_and_invalid_identity_or_extent_is_rejected() {
    let bytes = payload();
    let mut table = Table::new(2).unwrap();
    table.register(0, 5_u64).unwrap();
    for end in 0..bytes.len() {
        assert!(
            saved_queues::decode(&bytes[..end], 21, &table, |&v| v).is_err(),
            "accepted truncation at{end}"
        );
    }
    let change = |offset: usize, value: i32| {
        let mut b = bytes.clone();
        b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        b
    };
    for b in [
        change(0, -1),
        change(0, 1),
        change(4, 1),
        change(8, -1),
        change(8, 2),
        change(12, 0),
        change(12, i32::MAX),
        change(16, i32::MAX),
        change(20, -2),
        change(20, 2),
        change(20, i32::MAX),
    ] {
        assert!(saved_queues::decode(&b, 21, &table, |&v| v).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(saved_queues::decode(&trailing, 21, &table, |&v| v).is_err());
    for version in [0, 10, 29, u32::MAX] {
        assert!(saved_queues::decode(&bytes, version, &table, |&v| v).is_err());
    }
    assert_eq!(table.slots(), [Some(5), None]);
}
#[test]
fn aliases_remove_first_slot_only_and_rejected_registration_keeps_prior_entries() {
    let mut t = Table::new(4).unwrap();
    t.register(3, 7).unwrap();
    t.register(1, 7).unwrap();
    t.register(2, 8).unwrap();
    assert!(t.register(3, 9).is_err());
    assert_eq!(t.slots(), [None, Some(7), Some(8), Some(7)]);
    assert!(t.remove(&7));
    assert_eq!(t.slots(), [None, None, Some(8), Some(7)]);
    assert!(t.remove(&7));
    assert!(!t.remove(&7));
    assert_eq!(t.slots(), [None, None, Some(8), None]);
    assert!(!t.register(i32::MIN, 99).unwrap());
    assert!(t.resolve(4).is_err());
    assert!(t.resolve(i32::MIN).is_err());
}
