//! The doubly linked buckets (native 50ABC0 / 50B250 unlink): any mix of
//! head insertions, saved-list restoration and cancellations (head, middle,
//! tail, whole owners) leaves every bucket in the order a plain list model
//! gives.
use aa_numeric::Precision;
use aa_simulation::NativeRng;
use aa_world::queues::{Clock, EventId, Kind, Queues, restoration::SavedQueue};

fn queues() -> Queues {
    let mut q = Queues::new(Clock { begin_bits: 0, end_bits: 0.0625f32.to_bits(), step_bits: 0.0625f32.to_bits() }, Precision::Double53).unwrap();
    q.initialize(0, Kind::Periodic, 1).unwrap();
    q.initialize(1, Kind::Periodic, 4).unwrap();
    q
}

/// The model: per queue, per bucket, (event, owner, data) head first.
type Model = Vec<Vec<Vec<(EventId, u64, u32)>>>;

fn check(q: &Queues, model: &Model) {
    let snapshot = q.snapshot();
    for (id, buckets) in model.iter().enumerate() {
        let s = snapshot.iter().find(|s| s.id == id as u8).unwrap();
        let expected: Vec<Vec<(u64, u32)>> = buckets.iter().map(|b| b.iter().map(|&(_, o, d)| (o, d)).collect()).collect();
        assert_eq!(s.buckets, expected, "queue {id}");
    }
}

#[test]
fn unlink_anywhere_keeps_every_bucket_in_order() {
    let mut q = queues();
    let mut model: Model = vec![vec![Vec::new()], vec![Vec::new(); 4]];
    let mut rng = NativeRng { state: 0x1234_5678 };
    let mut pick = |n: usize| (rng.next_u32() as usize) % n.max(1);
    let mut world_rng = NativeRng { state: 0x0BAD_F00D };
    for step in 0..4000u32 {
        let live: Vec<(usize, usize, usize)> =
            model.iter().enumerate().flat_map(|(qi, bs)| bs.iter().enumerate().flat_map(move |(bi, b)| (0..b.len()).map(move |i| (qi, bi, i)))).collect();
        match pick(5) {
            0 | 1 | 2 => {
                let qi = pick(2);
                let owner = pick(40) as u64;
                let mut probe = NativeRng { state: world_rng.state };
                let bucket = probe.next_u32() as usize % model[qi].len();
                let id = q.add_periodic(qi as u8, owner, step, &mut world_rng).unwrap();
                model[qi][bucket].insert(0, (id, owner, step));
            }
            3 if !live.is_empty() => {
                let (qi, bi, i) = live[pick(live.len())];
                let (id, _, _) = model[qi][bi].remove(i);
                q.cancel(id).unwrap();
            }
            _ => {
                let owner = pick(40) as u64;
                q.cancel_owner(owner).unwrap();
                for bs in model.iter_mut() {
                    for b in bs.iter_mut() {
                        b.retain(|e| e.1 != owner);
                    }
                }
            }
        }
        check(&q, &model);
    }
}

#[test]
fn restored_lists_unlink_in_place() {
    let mut q = queues();
    q.restore_saved(SavedQueue { id: 1, kind: Kind::Periodic, current: 2, buckets: vec![vec![(1, 10), (2, 20), (3, 30)], vec![], vec![(4, 40)], vec![(5, 50), (6, 60)]] }).unwrap();
    // A head insertion before the saved entries, then unlinks of the middle,
    // the tail and the head.
    let mut rng = NativeRng { state: 0 };
    let bucket = NativeRng { state: 0 }.next_u32() as usize % 4;
    q.add_periodic(1, 7, 70, &mut rng).unwrap();
    q.cancel_owner(2).unwrap();
    q.cancel_owner(6).unwrap();
    q.cancel_owner(1).unwrap();
    let mut expected = vec![vec![(3, 30)], vec![], vec![(4, 40)], vec![(5, 50)]];
    expected[bucket].insert(0, (7, 70));
    let s = q.snapshot().into_iter().find(|s| s.id == 1).unwrap();
    assert_eq!(s.buckets, expected);
    assert_eq!(s.current, 2);
}
