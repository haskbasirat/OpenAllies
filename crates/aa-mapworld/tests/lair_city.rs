//! Replays research/phase8-restoration/native-lair-city.json.
use aa_actors::{
    ActorId,
    lair_city_state::{CityState, LairState},
};
use aa_mapworld::saved_components::SavedComponent;
use serde_json::Value;

const FIXTURE: &str = include_str!("../../../research/phase8-restoration/native-lair-city.json");

#[test]
fn original_lair_and_city_constructor_and_load_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 24);
    for case in cases {
        let text = case["payload"].as_str().unwrap();
        let payload: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect();
        let native = &case["native"];
        let list = native["list"].clone();
        match case["component"].as_str().unwrap() {
            "Lair" => {
                let (SavedComponent::Lair { words, word_1c, word_20, .. }, consumed) =
                    SavedComponent::decode_prefix(25, &payload).unwrap()
                else {
                    panic!()
                };
                assert_eq!(consumed, payload.len());
                let mut state = LairState::new(ActorId::from_raw(1));
                assert_eq!(u64::from(state.word_20.0), native["after_ctor_10_20"][4].as_u64().unwrap());
                state.load(&words.values, word_1c, word_20);
                assert_eq!(Value::from(state.words_10.clone()), list);
                assert_eq!(u64::from(state.word_1c), native["words_18_20"][1].as_u64().unwrap());
                assert_eq!(u64::from(state.word_20.0), native["words_18_20"][2].as_u64().unwrap());
            }
            _ => {
                let (SavedComponent::City { words, .. }, consumed) =
                    SavedComponent::decode_prefix(29, &payload).unwrap()
                else {
                    panic!()
                };
                assert_eq!(consumed, payload.len());
                let mut state = CityState::new(ActorId::from_raw(1));
                let result = state.load(
                    words.as_ref().map(|w| w.values.as_slice()),
                    case["radius"].as_u64().unwrap() as u32,
                );
                assert_eq!(result.is_err(), !native["cancel"].is_null(), "{case}");
                assert_eq!(Value::from(state.values.clone()), list);
                assert_eq!(
                    state.values.last().map_or(Value::Null, |v| Value::from(*v)),
                    native["tail"]
                );
                assert_eq!(state.byte_20, (native["words_18_20"][2].as_u64().unwrap() & 0xff) as u8);
            }
        }
    }
}
