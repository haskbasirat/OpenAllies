use aa_actors::{ActorId, structure_state::StructureState};
use aa_mapworld::saved_components::SavedComponent;
#[test]
fn original_structure_constructor_and_empty_queue_load_match() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-structure.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 112);
    for case in cases {
        let mut state = StructureState::new(ActorId::from_raw(77));
        for (phase, apply) in [("before", false), ("after", true)] {
            if apply {
                let bytes: Vec<_> = case["payload_hex"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
                    .collect();
                let (SavedComponent::Structure(payload), n) =
                    SavedComponent::decode_prefix(13, &bytes).unwrap()
                else {
                    panic!()
                };
                assert_eq!(n, bytes.len());
                payload.apply(&mut state).unwrap();
            }
            assert!(case[phase]["owner_matches"].as_bool().unwrap());
            assert_eq!(state.header.owner, ActorId::from_raw(77));
            assert_eq!(state.header.scheduled_head, None);
            assert_eq!(case[phase]["head"], 0);
            assert_eq!(case[phase]["queue_count"], state.queue_14.len());
            assert_eq!(case[phase]["capacity"], state.capacity_hint);
            assert_eq!(case[phase]["byte30"], state.byte_30);
            for (offset, value) in [
                (0x10, state.object_handle_10),
                (0x20, state.word_20),
                (0x24, state.word_24),
                (0x28, state.word_28),
                (0x2c, state.word_2c),
                (0x34, state.word_34),
            ] {
                assert_eq!(case[phase]["words"][format!("{offset:#x}")], value);
            }
        }
    }
}
