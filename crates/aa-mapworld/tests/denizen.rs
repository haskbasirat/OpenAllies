use aa_actors::{ActorId, denizen_state::DenizenState};
use aa_mapworld::saved_components::SavedComponent;
#[test]
fn original_empty_inventory_denizen_constructor_and_saved_load_match() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-denizen.json"
    ))
    .unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 90);
    for c in cases {
        // Empty ActorData+35C list: 56B720 builds no Inventory slot.
        let mut state = DenizenState::with_inventory(ActorId::from_raw(19), &[]);
        let bytes: Vec<_> = c["payload_hex"]
            .as_str()
            .unwrap()
            .as_bytes()
            .chunks_exact(2)
            .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
            .collect();
        let (payload, n) = SavedComponent::decode_prefix(10, &bytes).unwrap();
        assert_eq!(n, bytes.len());
        let SavedComponent::Denizen(payload) = payload else {
            panic!()
        };
        for step in 0..3 {
            let expected = if step == 0 {
                &c["before"]
            } else {
                payload
                    .apply(
                        &mut state,
                        |_| unreachable!("no Inventory slot exists"),
                        aa_numeric::Precision::Double53,
                    )
                    .unwrap();
                &c["after"][step - 1]
            };
            assert!(expected["owner_matches"].as_bool().unwrap());
            assert!(expected["group_owner_matches"].as_bool().unwrap());
            assert!(expected["groups_zero"].as_bool().unwrap());
            assert!(expected["occupant_list_empty"].as_bool().unwrap());
            assert!(expected["command_list_empty"].as_bool().unwrap());
            assert_eq!(state.header.owner, ActorId::from_raw(19));
            assert_eq!(state.header.scheduled_head, None);
            assert!(state.groups_48.iter().all(Vec::is_empty));
            for (o, v) in &state.words {
                assert_eq!(
                    *v,
                    expected["words"][format!("{o:#x}")].as_u64().unwrap() as u32
                );
            }
            assert_eq!(
                serde_json::to_value([state.byte_20, state.byte_21]).unwrap(),
                expected["bytes"]
            );
            assert_eq!(
                serde_json::to_value(&state.handles_2c).unwrap(),
                expected["handles"]
            );
        }
    }
}
