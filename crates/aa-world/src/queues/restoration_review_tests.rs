//! Independent reviewer tests; production restoration is owned by the root.
use super::*;

fn queues() -> Queues {
    let mut q = Queues::new(
        Clock {
            begin_bits: 1.0_f32.to_bits(),
            end_bits: 1.0625_f32.to_bits(),
            step_bits: 0.0625_f32.to_bits(),
        },
        Precision::Double53,
    )
    .unwrap();
    for (id, count) in [2, 3, 1, 4, 2].into_iter().enumerate() {
        q.initialize(
            id as u8,
            if id < 3 {
                Kind::Periodic
            } else {
                Kind::Deferred
            },
            count,
        )
        .unwrap();
    }
    q
}
fn owner_order(q: &Queues, owner: Owner) -> Vec<(u8, u32)> {
    q.owners
        .get(&owner)
        .into_iter()
        .flatten()
        .map(|id| {
            let n = &q.nodes[&id.0];
            (n.queue, n.data)
        })
        .collect()
}
#[test]
fn restored_bucket_tail_order_and_owner_head_order_survive_cross_queue_cancellation() {
    let mut q = queues();
    let mut rng = NativeRng::new(123);
    q.add_periodic(1, 7, 70, &mut rng).unwrap();
    q.add_after(3, 7, 900, 0).unwrap();
    q.add_after(3, 8, 901, 0).unwrap();
    q.add_after(4, 9, 902, 0).unwrap();
    q.queues.get_mut(&3).unwrap().last_end_bits = 19.0_f32.to_bits();
    let before_rng = rng.state;
    let buckets = vec![
        vec![(7, 10), (8, 11), (7, 12)],
        vec![],
        vec![(8, 20), (7, 21)],
        vec![],
        vec![],
        vec![(7, 50)],
    ];
    q.restore_saved(SavedQueue {
        id: 3,
        kind: Kind::Deferred,
        current: 5,
        buckets: buckets.clone(),
    })
    .unwrap();
    q.restore_saved(SavedQueue {
        id: 4,
        kind: Kind::Deferred,
        current: 0,
        buckets: vec![vec![(7, 60), (7, 61)]],
    })
    .unwrap();
    assert_eq!(rng.state, before_rng);
    assert_eq!(q.snapshot()[3].buckets, buckets);
    assert_eq!(q.snapshot()[3].current, 5);
    assert_eq!(q.snapshot()[3].last_end_bits, 0);
    assert_eq!(
        owner_order(&q, 7),
        [
            (4, 61),
            (4, 60),
            (3, 50),
            (3, 21),
            (3, 12),
            (3, 10),
            (1, 70)
        ]
    );
    assert_eq!(owner_order(&q, 8), [(3, 20), (3, 11)]);
    assert!(!q.owners.contains_key(&9));
    q.cancel_owner(7).unwrap();
    q.cancel_owner(7).unwrap();
    assert!(q.snapshot()[1].buckets.iter().all(Vec::is_empty));
    assert_eq!(
        q.snapshot()[3].buckets,
        vec![vec![(8, 11)], vec![], vec![(8, 20)], vec![], vec![], vec![]]
    );
    assert!(q.snapshot()[4].buckets[0].is_empty());
    assert_eq!(q.nodes.len(), 2);
    assert_eq!(owner_order(&q, 8), [(3, 20), (3, 11)]);
}
#[test]
fn invalid_restore_dimensions_kind_and_nested_dispatch_leave_existing_lists_untouched() {
    let mut q = queues();
    q.add_after(3, 4, 17, 0).unwrap();
    let baseline = q.snapshot();
    let owners = q.owners.clone();
    for saved in [
        SavedQueue {
            id: 99,
            kind: Kind::Deferred,
            current: 0,
            buckets: vec![vec![]],
        },
        SavedQueue {
            id: 3,
            kind: Kind::Periodic,
            current: 0,
            buckets: vec![vec![]],
        },
        SavedQueue {
            id: 3,
            kind: Kind::Deferred,
            current: 0,
            buckets: vec![],
        },
        SavedQueue {
            id: 3,
            kind: Kind::Deferred,
            current: 1,
            buckets: vec![vec![]],
        },
        SavedQueue {
            id: 0,
            kind: Kind::Periodic,
            current: 0,
            buckets: vec![vec![]],
        },
    ] {
        assert!(q.restore_saved(saved).is_err());
        assert_eq!(q.snapshot(), baseline);
        assert_eq!(q.owners, owners);
    }
    q.dispatching = true;
    assert!(matches!(
        q.restore_saved(SavedQueue {
            id: 3,
            kind: Kind::Deferred,
            current: 0,
            buckets: vec![vec![]]
        }),
        Err(Error::NestedDispatch)
    ));
    assert_eq!(q.snapshot(), baseline);
    assert_eq!(q.owners, owners);
}
