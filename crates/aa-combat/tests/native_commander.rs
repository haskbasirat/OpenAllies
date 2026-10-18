use aa_combat::commander::{self, Definition, SavedState, State};
use serde_json::{Value, json};

fn definitions(value: &Value) -> Vec<Definition> {
    serde_json::from_value(value.clone()).unwrap()
}
const CAP: aa_body::F32 = aa_body::F32(0x4479_c000);
/// Native output words: Commander+0xC is the balance of the single store.
fn native_view(state: &Option<State>) -> Value {
    let mut v = json!(state);
    if let Some(o) = v.as_object_mut() {
        let balance = o["experience"]["balance"].clone();
        assert_eq!(o["experience"]["maximum"], json!(CAP.0));
        o.insert("experience".into(), balance);
    }
    v
}
#[test]
fn original_commander_replacement_and_saved_state_sequences_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-commander/native-commander-fixtures.json"
    ))
    .unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let input = &row["input"];
        let definitions = definitions(&input["definitions"]);
        let mut state: Option<State> = None;
        let mut diagnostics = 0;
        let mut output = vec![];
        for step in input["steps"].as_array().unwrap() {
            if step["kind"] == "replace" {
                let definition = step["definition"]
                    .as_u64()
                    .and_then(|id| definitions.iter().find(|d| u64::from(d.id) == id));
                commander::replace_with_maximum(&mut state, definition.map(|d| (d, CAP)), 1)
                    .unwrap();
            } else {
                let saved: SavedState = serde_json::from_value(step["saved"].clone()).unwrap();
                let outcome = commander::apply_saved(state.as_mut().unwrap(), &saved).unwrap();
                diagnostics += u32::from(outcome.count_mismatch);
            }
            output.push(json!({"state":native_view(&state),"diagnostics":diagnostics}));
        }
        assert_eq!(json!(output), row["output"], "{}", input["name"]);
    }
}

#[test]
fn invalid_later_save_record_preserves_earlier_native_writes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-commander/native-commander-fixtures.json"
    ))
    .unwrap();
    let c = &fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["input"]["definitions"][0]["abilities"]
                .as_array()
                .unwrap()
                .len()
                == 2
        })
        .unwrap()["input"];
    let definition = definitions(&c["definitions"]).remove(0);
    let mut state = State::with_maximum(&definition, CAP, 1).unwrap();
    assert_eq!(
        State::new(&definition, 1).unwrap().experience.maximum.0,
        commander::MAXIMUM_EXPERIENCE_UNBOUND.0
    );
    let mut saved: SavedState = serde_json::from_value(c["steps"][2]["saved"].clone()).unwrap();
    saved.abilities[1].version = 2;
    let original_second = state.abilities[1].clone();
    assert!(commander::apply_saved(&mut state, &saved).is_err());
    assert_eq!(state.experience.balance.0, saved.experience.0);
    assert_eq!(state.abilities[0].use_count, saved.abilities[0].use_count);
    assert_eq!(state.abilities[1], original_second);
}
