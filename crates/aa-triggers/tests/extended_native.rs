//! Replays research/phase8-triggers/native-extended.json: the ORIGINAL 546AF0
//! cases of the element types no campaign mission uses, executed under Unicorn
//! with recorded seams (native_extended.py). Same comparison as
//! native-elements.json; native null-dereference faults must be Rust errors.
mod fixture;
use fixture::{FixtureWorld, run_case};

#[test]
fn native_extended_replay() {
    let doc: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-triggers/native-extended.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    let mut failures = vec![];
    let mut opcodes = std::collections::BTreeSet::new();
    for case in cases {
        opcodes.insert(case["input"]["element"]["opcode"].as_u64().unwrap());
        if let Err(message) = run_case::<FixtureWorld>(case) {
            failures.push(message);
        }
    }
    println!("{} cases over opcodes {opcodes:?}", cases.len());
    assert!(
        failures.is_empty(),
        "{} of {} native cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.iter().take(60).cloned().collect::<Vec<_>>().join("\n")
    );
}
