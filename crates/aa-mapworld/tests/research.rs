//! Replays research/phase8-restoration/native-research.json: the original
//! Research constructor 584210 then saved Load 5849E0 on one object.
use aa_actors::{ActorId, research_state::ResearchState};
use aa_mapworld::saved_components::SavedComponent;
use serde_json::Value;

const FIXTURE: &str = include_str!("../../../research/phase8-restoration/native-research.json");

#[test]
fn original_research_constructor_and_load_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 17);
    for case in cases {
        let input = &case["input"];
        let native = &case["native"];
        let text = input["payload"].as_str().unwrap();
        let payload: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect();
        let (SavedComponent::Research(saved), consumed) =
            SavedComponent::decode_prefix(21, &payload).unwrap()
        else {
            panic!()
        };
        assert_eq!(consumed, payload.len());
        let items: Vec<String> = input["research_list"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        let references: Vec<String> = saved
            .references
            .values
            .iter()
            .map(|v| String::from_utf16(v).unwrap())
            .collect();
        let mut state = ResearchState::new(ActorId::from_raw(2));
        let mut fx = Vec::new();
        let result = state.load(&references, &items, |item| fx.push(item.to_owned()));
        assert_eq!(result.is_err(), !native["error"].is_null(), "{case}");
        assert_eq!(Value::from(state.items.clone()), native["items_head_first"], "{case}");
        assert_eq!(
            state.items.last().map_or(Value::Null, |v| Value::from(v.clone())),
            native["tail"]
        );
        assert_eq!(Value::from(fx), native["fx"], "{case}");
    }
}
