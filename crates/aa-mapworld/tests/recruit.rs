//! Replays research/phase8-restoration/native-recruit.json: the original
//! Recruit constructor 581E40 followed by saved Load 583C80.
use aa_actors::{ActorId, recruit_state::RecruitState};
use aa_mapworld::saved_components::SavedComponent;
use serde_json::Value;

const FIXTURE: &str = include_str!("../../../research/phase8-restoration/native-recruit.json");

#[test]
fn original_recruit_constructor_and_load_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 28);
    for case in cases {
        let text = case["payload"].as_str().unwrap();
        let payload: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect();
        let native = &case["native"];
        let (SavedComponent::Recruit(saved), consumed) =
            SavedComponent::decode_prefix(16, &payload).unwrap()
        else {
            panic!()
        };
        assert_eq!(consumed, native["consumed"].as_u64().unwrap() as usize);
        let mut state = RecruitState::new(ActorId::from_raw(9));
        let bytes = |s: &RecruitState| {
            let mut out = vec![s.byte_10, s.byte_11, 0xcd, 0xcd];
            out.extend(s.words_14[0].to_le_bytes());
            out.extend(s.words_14[1].to_le_bytes());
            out.extend([s.byte_1c, 0xcd, 0xcd, 0xcd]);
            Value::from(out)
        };
        assert_eq!(bytes(&state), native["after_ctor_bytes_10_1f"]);
        state.load(saved.byte_1c, saved.words_14);
        assert_eq!(bytes(&state), native["bytes_10_1f"], "{case}");
    }
}
