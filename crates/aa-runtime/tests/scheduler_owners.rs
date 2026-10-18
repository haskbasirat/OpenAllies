use aa_numeric::Precision;
use aa_runtime::scheduler_owners::{Error, Owners, Target};
use aa_world::{
    initialization::ClockPlan,
    queues::{Callbacks, Delivery, Queues},
};

struct Receive(Vec<Delivery>);
impl Callbacks for Receive {
    fn deliver(&mut self, _: &mut Queues, event: Delivery) -> aa_world::queues::Result<()> {
        self.0.push(event);
        Ok(())
    }
}
#[test]
fn replacing_an_updatable_cancels_every_old_queue_without_reusing_its_identity() {
    let plan = ClockPlan::from_tick_rate(16.0f32.to_bits(), Precision::Double53).unwrap();
    let mut q = plan
        .initialize_queues(plan.new_clock(), Precision::Double53)
        .unwrap();
    let mut bank = Owners::new();
    // Equal numeric command/controller identities are distinct actual instances.
    let target = Target::Command(9);
    let old = bank.register(target).unwrap();
    let unrelated = bank.register(Target::Controller(9)).unwrap();
    assert_ne!(old, unrelated);
    assert!(matches!(bank.register(target), Err(Error::Duplicate)));
    q.add_at(3, old, 21, 0).unwrap();
    q.add_at(4, old, 22, 0).unwrap();
    q.add_at(3, unrelated, 31, 0).unwrap();
    assert!(q.has_owner(old));
    bank.retire(target, &mut q).unwrap();
    assert!(!q.has_owner(old));
    assert!(q.has_owner(unrelated));
    assert!(bank.target(old).is_err());
    let replacement = bank.register(target).unwrap();
    assert_ne!(old, replacement);
    q.add_at(3, replacement, 41, 0).unwrap();
    q.clock.end_bits = 1.0f32.to_bits();
    let mut receive = Receive(Vec::new());
    q.update(3, &mut receive).unwrap();
    q.update(4, &mut receive).unwrap();
    assert_eq!(
        receive
            .0
            .iter()
            .map(|e| (e.owner, e.data))
            .collect::<Vec<_>>(),
        vec![(replacement, 41), (unrelated, 31)]
    );
    assert!(!q.has_owner(replacement));
    assert!(!q.has_owner(unrelated));
    assert_eq!(bank.target(replacement).unwrap(), target);
}
