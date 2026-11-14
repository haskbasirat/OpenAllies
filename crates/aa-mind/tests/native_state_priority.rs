//! Replays research/phase8-ai/native-state-priority.json (original 529F80
//! and 529FC0 under Unicorn) against aa_mind::state_priority.
use aa_mind::{
    Arena, Mind, MindId, State, StateId,
    state_priority::{admits, can_activate},
};
use serde_json::{Value, json};

const STATES: [&str; 11] = [
    "unload", "deploy", "sleep", "move", "kill", "bombard", "engage", "repair", "join", "reattach",
    "unpack",
];

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-state-priority.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn state_id(name: &str) -> StateId {
    StateId(STATES.iter().position(|n| *n == name).unwrap() as u32 + 1)
}

#[test]
fn native_state_priority_matches_original() {
    let f = fixture();
    assert_eq!(f["coverage"]["529F80"], json!([21, 21]));
    assert_eq!(f["coverage"]["529FC0"], json!([22, 22]));
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    for row in cases {
        let c = &row["input"];
        let name = &c["name"];
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        for key in c["stack"].as_array().unwrap() {
            mind.stack.push_back(key.as_str().map(state_id));
        }
        arena.minds.insert(MindId(1), mind);
        for i in 0..STATES.len() {
            arena.states.insert(
                StateId(i as u32 + 1),
                State {
                    parent: MindId(1),
                    discard_on_suspend: false,
                    status: 0,
                },
            );
        }
        let priority_of = |s: StateId| -> aa_mind::Result<i32> {
            Ok(c["priorities"][STATES[s.0 as usize - 1]].as_i64().unwrap() as i32)
        };
        let r = if c["entry"] == "529F80" {
            let p = c["priority"].as_i64().unwrap() as i32;
            admits(MindId(1), p, &arena, priority_of).unwrap()
        } else {
            can_activate(state_id(c["state"].as_str().unwrap()), &arena, priority_of).unwrap()
        };
        assert_eq!(json!(u8::from(r)), row["output"]["result"], "{name}");
    }
}
