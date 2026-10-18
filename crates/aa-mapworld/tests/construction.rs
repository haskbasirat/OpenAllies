use aa_actors::{
    ActorId,
    construction_state::{ConstructionState, Definition},
};
use aa_mapworld::saved_components::SavedComponent;
#[test]
fn original_construction_ctor_and_saved_application_match() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-construction.json"
    ))
    .unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 224);
    for c in cases {
        let input = Definition {
            base_cost_words: serde_json::from_value(c["costs"].clone()).unwrap(),
            empty_predecessor_chain: true,
        };
        let mut state = ConstructionState::new(ActorId::from_raw(9), &input).unwrap();
        for (phase, apply) in [("before", false), ("after", true)] {
            if apply {
                let bytes: Vec<_> = c["payload_hex"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
                    .collect();
                let (payload, n) = SavedComponent::decode_prefix(15, &bytes).unwrap();
                assert_eq!(n, bytes.len());
                let SavedComponent::Construction(payload) = payload else {
                    panic!()
                };
                payload
                    .apply(&mut state, &input, !c["editor"].as_bool().unwrap())
                    .unwrap();
            }
            assert!(c[phase]["owner_matches"].as_bool().unwrap());
            assert_eq!(state.header.owner, ActorId::from_raw(9));
            assert_eq!(c[phase]["head"], 0);
            assert_eq!(state.header.scheduled_head, None);
            assert_eq!(c[phase]["definition28"], 0);
            assert_eq!(state.definition_28, None);
            for (i, values) in [&state.words_10, &state.words_1c].into_iter().enumerate() {
                assert_eq!(
                    serde_json::to_value(values).unwrap(),
                    c[phase]["arrays"][i]["values"]
                );
                assert_eq!(
                    state.capacity_hints[i],
                    c[phase]["arrays"][i]["capacity"].as_u64().unwrap() as u32
                );
            }
        }
    }
}
#[test]
fn unbound_predecessor_costs_and_nonempty_upgrade_do_not_invent_state() {
    let definition = Definition {
        base_cost_words: Some(vec![1]),
        empty_predecessor_chain: false,
    };
    assert!(ConstructionState::new(ActorId::from_raw(1), &definition).is_err());
    let good = Definition {
        empty_predecessor_chain: true,
        ..definition
    };
    let mut state = ConstructionState::new(ActorId::from_raw(1), &good).unwrap();
    let payload = aa_mapworld::saved_components::ConstructionPayload {
        updatable: aa_mapworld::actor_payload::UpdatablePayload {
            version: 0,
            serialized_id: None,
        },
        version: 1,
        words_10: aa_mapworld::actor_payload::CountedList {
            serialized_count: 1,
            values: vec![7],
        },
        words_1c: None,
        reference_28_utf16: Some(vec![65]),
        legacy_upgrade_words: None,
    };
    assert!(payload.apply(&mut state, &good, true).is_err());
    assert_eq!(state.words_10, [1]);
    assert_eq!(state.definition_28, None);
}
