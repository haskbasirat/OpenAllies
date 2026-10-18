use aa_mapworld::{performer_restore, saved_components::SavedComponent};
use aa_numeric::Precision;
use std::cell::RefCell;

fn bytes(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn original_performer_saved_load_preserves_owned_ability_instances() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-performer-load.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 180);
    for case in cases {
        let pc = match case["input"]["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let mut state: aa_combat::constructors::OwnedPerformer =
            serde_json::from_value(case["before"]["owned"].clone()).unwrap();
        let abilities: Vec<aa_combat::ability::Ability> =
            serde_json::from_value(case["before"]["abilities"].clone()).unwrap();
        let arena = RefCell::new(abilities);
        let mut diagnostics = 0;
        for (payload_key, output_key) in [
            ("payload_hex", "after"),
            ("second_payload_hex", "second_after"),
        ] {
            let input = bytes(case[payload_key].as_str().unwrap());
            let (payload, n) = SavedComponent::decode_prefix(5, &input).unwrap();
            assert_eq!(n, input.len());
            let SavedComponent::Performer(payload) = payload else {
                panic!()
            };
            let out = performer_restore::apply(&mut state, &payload, pc, &|id, saved| {
                aa_combat::commander::apply_ability(&mut arena.borrow_mut()[id as usize - 1], saved)
                    .map_err(|e| e.to_string())
            })
            .unwrap();
            assert_eq!(
                serde_json::to_value(&state).unwrap(),
                case[output_key]["owned"]
            );
            assert_eq!(
                serde_json::to_value(&*arena.borrow()).unwrap(),
                case[output_key]["abilities"]
            );
            diagnostics +=
                u32::from(out.turret_count_mismatch) + u32::from(out.ability_count_mismatch);
            assert_eq!(
                diagnostics,
                case[output_key]["diagnostics"].as_u64().unwrap() as u32
            );
        }
    }
}
#[test]
fn invalid_active_index_stops_before_later_fields_and_actions() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-performer-load.json"
    ))
    .unwrap();
    let case = &fixture["cases"][0];
    let mut state: aa_combat::constructors::OwnedPerformer =
        serde_json::from_value(case["before"]["owned"].clone()).unwrap();
    let (SavedComponent::Performer(mut input), _) =
        SavedComponent::decode_prefix(5, &bytes(case["payload_hex"].as_str().unwrap())).unwrap()
    else {
        panic!()
    };
    input.ability_index = 0;
    assert!(
        performer_restore::apply(&mut state, &input, Precision::Single24, &|_, _| panic!(
            "too late"
        ))
        .is_err()
    );
    assert_eq!(state.state.active_index, 0);
    assert_eq!(state.state.started_at, aa_body::F32::ZERO);
    assert_eq!(state.state.event_kind, 0);
    assert_eq!(state.state.target_id, input.word_28);
}
