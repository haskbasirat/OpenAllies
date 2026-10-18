use super::*;
use aa_actors::{
    ActorId,
    loading::{LoadKind, construct_shell_with_components},
};
use aa_mapworld::restore::RestoredComponent;
use serde_json::Value;

#[test]
fn saved_state_matches_original_and_v14_complete_or_first_effect_prefix() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-saved-actor-state/native-saved-actor-state.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 4044);
    let mut completed = 0;
    let mut refused = 0;
    let mut reached_load_states = 0;
    for row in cases {
        let input = &row["input"];
        let expected = &row["output"];
        let mut base = construct_shell_with_components::<_, RestoredComponent>(LoadKind::Actor, ());
        let actor = base.actor.as_mut().unwrap();
        let bits = input["bits"].as_u64().unwrap();
        actor.components.state.low = bits as u32;
        actor.components.state.high = (bits >> 32) as u32;
        // These offsets are not the authoritative state fields. Keep hostile
        // duplicate words to detect accidental access or blanket shell reset.
        actor.words.insert(0x100, 0x12345678);
        actor.words.insert(0x104, 0x87654321);
        let words = actor.words.clone();
        base.thing.flags = input["flags"].as_u64().unwrap() as u32;
        let index = input["index"].as_u64().unwrap() as u8;
        let result = set(
            ActorStateContext {
                owner: ActorId::from_raw(4099),
                owner_actor: actor,
                owner_thing_flags: &mut base.thing.flags,
            },
            index,
            input["enabled"].as_bool().unwrap(),
        );
        let actual_bits =
            u64::from(actor.components.state.low) | (u64::from(actor.components.state.high) << 32);
        assert_eq!(actual_bits, expected["bits"].as_u64().unwrap(), "{row}");
        assert_eq!(
            u64::from(base.thing.flags),
            expected["flags"].as_u64().unwrap(),
            "{row}"
        );
        assert_eq!(actor.words, words, "{row}");
        assert_eq!(actor.components.iter().count(), 0);
        if expected["completed"].as_bool().unwrap() {
            completed += 1;
            assert!(result.is_ok(), "{row}: {result:?}");
            assert!(expected["first_required_effect"].is_null());
        } else {
            refused += 1;
            let error = result.unwrap_err();
            assert!(
                error.contains(expected["first_required_effect"].as_str().unwrap()),
                "{row}: {error}"
            );
            assert!(error.contains("earlier effects remain committed"));
        }
        if index == 34 || index == 37 {
            reached_load_states += 1;
        }
    }
    assert_eq!(completed, 2388);
    assert_eq!(refused, 1656);
    assert_eq!(reached_load_states, 204);
}

#[test]
fn invalid_state_is_portably_refused_without_touching_the_saved_actor() {
    let mut base = construct_shell_with_components::<_, RestoredComponent>(LoadKind::Actor, ());
    let actor = base.actor.as_mut().unwrap();
    actor.components.state.low = 0x12345678;
    actor.components.state.high = 0x87654321;
    base.thing.flags = 0x80000100;
    let before = actor.components.state;
    let words = actor.words.clone();
    let result = set(
        ActorStateContext {
            owner: ActorId::from_raw(4099),
            owner_actor: actor,
            owner_thing_flags: &mut base.thing.flags,
        },
        42,
        true,
    );
    assert!(
        result
            .unwrap_err()
            .contains("state index outside registered table")
    );
    assert_eq!(actor.components.state, before);
    assert_eq!(actor.words, words);
    assert_eq!(base.thing.flags, 0x80000100);
}
