//! GoalBase (aa-navigation, constructor 7A66E0) <-> engine goal views.
use aa_mind::sai::{
    assign::ActorList,
    bridge::{goal_actors, goal_state, priority_goal, write_goal_state, write_priority_goal},
};
use aa_navigation::sai_state::GoalBase;

#[test]
fn goal_base_round_trips_through_engine_views() {
    let mut base = GoalBase::new(aa_navigation::startup::Object(7));
    base.costs = vec![0x4120_0000, 0x4040_0000];
    base.inertia = 0x3f00_0000;
    base.byte2c = true;
    let mut state = goal_state(&base);
    assert_eq!(state.status, 1);
    assert_eq!(state.current, 0xbf80_0000);
    assert_eq!(state.base, 0xff7f_ffff);
    assert_eq!(state.travel, 0xbf80_0000);
    let list = goal_actors(&base).unwrap();
    assert_eq!(list.count, 0);
    state.status = 2;
    state.current = 0x4100_0000;
    state.base = 0x4110_0000;
    state.travel = 0x4000_0000;
    write_goal_state(&state, Some(&list), &mut base).unwrap();
    assert_eq!(
        (
            base.status,
            base.current_priority,
            base.priority,
            base.travel
        ),
        (2, 0x4100_0000, 0x4110_0000, 0x4000_0000)
    );
    // Fields the view does not carry are preserved.
    assert_eq!(
        (
            base.fuzzy,
            base.inertia,
            base.byte2c,
            base.cost_capacity_word
        ),
        (0xbf80_0000, 0x3f00_0000, true, 1)
    );
    let mut goal = priority_goal(&base);
    goal.fuzzy = 0x3e80_0000;
    write_priority_goal(&goal, &mut base);
    assert_eq!(base.fuzzy, 0x3e80_0000);
    // A linked native list cannot be converted or written.
    base.assignment_words = [0x1234, 0x1234, 1];
    assert!(goal_actors(&base).is_err());
    let mut linked = ActorList::from_actors([Some(7)], 1);
    linked.count = 1;
    assert!(write_goal_state(&state, Some(&linked), &mut base).is_err());
}
