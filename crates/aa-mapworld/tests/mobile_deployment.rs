//! Replays research/phase8-restoration/native-mobile-deployment.json.
use aa_actors::{ActorId, mobile_deployment_state::MobileDeploymentState};
use aa_mapworld::saved_components::SavedComponent;
use serde_json::Value;

const FIXTURE: &str =
    include_str!("../../../research/phase8-restoration/native-mobile-deployment.json");

#[test]
fn original_mobile_deployment_constructor_and_load_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 18);
    for case in cases {
        let text = case["payload"].as_str().unwrap();
        let payload: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect();
        let native = &case["native"];
        let (SavedComponent::MobileDeployment { word_10, word_14, .. }, consumed) =
            SavedComponent::decode_prefix(19, &payload).unwrap()
        else {
            panic!()
        };
        assert_eq!(consumed, native["consumed"].as_u64().unwrap() as usize);
        let mut state = MobileDeploymentState::new(ActorId::from_raw(4));
        let words = |s: &MobileDeploymentState| Value::from(vec![s.word_10.0, s.word_14.0]);
        assert_eq!(words(&state), native["after_ctor_10_14"]);
        state.load(word_10, word_14);
        assert_eq!(words(&state), native["words_10_14"], "{case}");
    }
}
