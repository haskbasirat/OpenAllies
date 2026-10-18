//! Complete native 0x779F00 outcomes, not a wire serialize/deserialize proxy.
use aa_netcode::launch_assignment::CompleteSettings;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-net-integration/native-launch-assignment.json"
    ))
    .unwrap()
}

#[test]
fn original_launch_assignment_preserves_all_wire_and_retained_values() {
    let data = fixture();
    let rows = data["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 32);
    let mut self_assignments = 0;
    for row in rows {
        let source: CompleteSettings = serde_json::from_value(row["source"].clone()).unwrap();
        let mut target: CompleteSettings = serde_json::from_value(row["before"].clone()).unwrap();
        target.assign_from(&source).unwrap();
        assert_eq!(
            serde_json::to_value(&target).unwrap(),
            row["after"],
            "case {}",
            row["index"]
        );
        if row["self_assignment"].as_bool().unwrap() {
            self_assignments += 1;
        }
    }
    assert_eq!(self_assignments, 4);
}

#[test]
fn incomplete_retained_state_is_rejected_without_mutating_the_destination() {
    let data = fixture();
    let row = &data["cases"][1];
    let mut source: CompleteSettings = serde_json::from_value(row["source"].clone()).unwrap();
    let mut target: CompleteSettings = serde_json::from_value(row["before"].clone()).unwrap();
    let before = target.clone();
    source.retained.kingdoms.game.clear();
    assert!(target.assign_from(&source).is_err());
    assert_eq!(target, before);
}
