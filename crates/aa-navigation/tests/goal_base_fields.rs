//! SAI goal base (0x7A66E0) named fields keep the original serialized layout.
use aa_navigation::sai_state::GoalBase;
use aa_navigation::startup::Object;
use serde_json::json;

#[test]
fn constructor_fields_and_original_layout_round_trip() {
    let goal = GoalBase::new(Object(7));
    assert_eq!(goal.status, 1);
    assert_eq!(goal.status_signed(), 1);
    assert_eq!(goal.fuzzy, 0xbf80_0000);
    assert!(goal.costs.is_empty());
    assert_eq!(goal.cost_capacity_word, 1);
    assert_eq!(goal.inertia, 0);
    assert_eq!(goal.priority, 0xff7f_ffff);
    assert_eq!(goal.travel, 0xbf80_0000);
    assert_eq!(goal.current_priority, 0xbf80_0000);
    assert_eq!(goal.actor_count(), 0);
    assert!(goal.actor_list_unlinked());
    // Exactly the phase7 0x7A66E0 fixture shape (research/phase7-sai/controller_oracle.py).
    assert_eq!(
        serde_json::to_value(&goal).unwrap(),
        json!({"controller":7,"category8":1,"words0c_14":[0,0,0],"scalar18":0xbf80_0000u32,
            "words1c_20":[0,0],"word24":1,"word28":0,"byte2c":false,
            "scalars30_38":[0xbf80_0000u32,0xbf80_0000u32,0xff7f_ffffu32]})
    );
    let mut live = goal.clone();
    live.status = 2;
    live.costs = vec![0x4120_0000, 0];
    live.inertia = 0x3f00_0000;
    live.priority = 0x42c8_0000;
    live.travel = 0x4040_0000;
    live.current_priority = 0x42ca_0000;
    live.assignment_words = [0x100, 0x120, 2];
    let wire = serde_json::to_value(&live).unwrap();
    assert_eq!(wire["words1c_20"], json!([0, 2]));
    // +0x30 travel, +0x34 current priority, +0x38 base priority.
    assert_eq!(wire["scalars30_38"], json!([0x4040_0000u32, 0x42ca_0000u32, 0x42c8_0000u32]));
    assert_eq!(wire["words0c_14"], json!([0x100, 0x120, 2]));
    assert_eq!(live.actor_count(), 2);
    assert!(!live.actor_list_unlinked());
    let back: GoalBase = serde_json::from_value(wire).unwrap();
    assert_eq!(back, live);
    let mut bad = serde_json::to_value(&live).unwrap();
    bad["words1c_20"] = json!([0, 3]);
    assert!(serde_json::from_value::<GoalBase>(bad).is_err());
}
