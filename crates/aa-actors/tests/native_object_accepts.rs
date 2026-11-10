//! 0x4D1EE0 against research/phase8-combat/native-object-accepts-fixtures.json
//! (every object class x actor class x owner-word combination).
use aa_actors::object_accepts::{object_accepts, owner, owner_getter, owner_test};
use serde_json::Value;

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

#[test]
fn object_accepts_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-object-accepts-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 300);
    let mut accepted = 0;
    for (index, case) in cases.iter().enumerate() {
        let test = owner_test(word(&case["object_vtable"])).unwrap();
        let getter = owner_getter(word(&case["actor_vtable"])).unwrap();
        let actor_owner = owner(getter, word(&case["actor_e8"]));
        let got = object_accepts(test, word(&case["object"]), word(&case["object_e8"]), actor_owner);
        assert_eq!(u32::from(got), word(&case["result"]), "case {index}");
        accepted += u32::from(got);
    }
    assert!(accepted > 0);
}
