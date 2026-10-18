use aa_body::F32;
use aa_combat::{commander, constructors::LocalAbility};
use aa_mapworld::{
    actor_payload::CountedList,
    commander_restore,
    kingdoms::{AbilityState, Commander, CommanderState},
};
fn definition() -> commander::Definition {
    commander::Definition {
        id: 91,
        abilities: vec![LocalAbility {
            definition: 7,
            local_definition: 8,
            main_power: F32::ONE,
        }],
    }
}
fn source(count: u32) -> Commander {
    Commander {
        presence: Some(1),
        reference_utf16: Some("source_profile".encode_utf16().collect()),
        state: Some(CommanderState {
            version: 0,
            word_0c: 17f32.to_bits(),
            abilities: CountedList {
                serialized_count: count,
                values: (0..count)
                    .map(|_| AbilityState {
                        version: 1,
                        word_0c: 0,
                        word_10: 9f32.to_bits(),
                        word_14: 0x1234,
                        byte_18: 255,
                        words: Some(CountedList {
                            serialized_count: 3,
                            values: vec![10, 20, 30],
                        }),
                    })
                    .collect(),
            },
        }),
    }
}
#[test]
fn source_wire_adapter_preserves_native_ability_reset_and_head_order() {
    let mut slot = None;
    let outcome =
        commander_restore::apply(&mut slot, &source(1), 123, Some((&definition(), F32(0x447a_0000))))
            .unwrap();
    assert_eq!(
        outcome.unwrap(),
        commander::LoadOutcome {
            count_mismatch: false,
            loaded: 1,
            discarded: 0
        }
    );
    let state = slot.as_ref().unwrap();
    assert_eq!(state.definition, 91);
    assert_eq!(state.kingdom, 123);
    assert_eq!(state.experience.balance.0, 17f32.to_bits());
    assert_eq!(state.experience.maximum.0, 1000f32.to_bits());
    let ability = &state.abilities[0];
    assert_eq!(ability.definition, 7);
    assert_eq!(ability.local_definition, Some(8));
    assert_eq!(ability.power, F32::ONE);
    assert!(ability.recharge_bypass);
    assert_eq!(ability.last_use, F32(0xff7fffff));
    assert_eq!(ability.word14, 0x1234);
    assert_eq!(ability.summoned_ids, [30, 20, 10]);
    let prior = state.clone();
    let no_replacement = Commander {
        presence: Some(0),
        reference_utf16: None,
        state: None,
    };
    assert!(
        commander_restore::apply(&mut slot, &no_replacement, 555, None)
            .unwrap()
            .is_none()
    );
    assert_eq!(slot.as_ref(), Some(&prior));
}
#[test]
fn source_wire_count_mismatch_reports_discarded_state_without_replacing_ctor_defaults() {
    let mut slot = None;
    let outcome = commander_restore::apply(&mut slot, &source(2), 123, Some((&definition(), F32(0xbf80_0000))))
        .unwrap()
        .unwrap();
    assert_eq!(
        outcome,
        commander::LoadOutcome {
            count_mismatch: true,
            loaded: 0,
            discarded: 2
        }
    );
    let state = slot.unwrap();
    assert_eq!(state.experience.balance.0, 17f32.to_bits());
    assert_eq!(state.abilities[0].word14, 0);
    assert!(state.abilities[0].summoned_ids.is_empty());
}
