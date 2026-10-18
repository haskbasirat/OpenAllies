//! Replays research/phase8-triggers/native-elements.json: every case executed
//! the ORIGINAL 546AF0 under Unicorn with the recorded seams.  The Rust port must
//! produce the same result, the same seam calls (receiver and arguments, in
//! order) and the same manager/Kingdom state.
mod fixture;
use fixture::{FixtureWorld, run_case};

#[test]
fn native_elements_replay() {
    let doc: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase8-triggers/native-elements.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    let mut failures = vec![];
    for case in cases {
        if let Err(message) = run_case::<FixtureWorld>(case) {
            failures.push(message);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} native cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.iter().take(60).cloned().collect::<Vec<_>>().join("\n")
    );
}
