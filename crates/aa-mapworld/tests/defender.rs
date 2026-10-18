//! Replays research/phase8-restoration/native-defender.json: the original
//! Defender constructor 567CD0 then saved Load 567020 on one object.
use aa_actors::{ActorId, defender_state::DefenderState};
use aa_body::F32;
use aa_combat::ability::Ability;
use aa_mapworld::saved_components::SavedComponent;
use serde_json::Value;

const FIXTURE: &str = include_str!("../../../research/phase8-restoration/native-defender.json");

#[test]
fn original_defender_constructor_and_load_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(doc["passed"], true);
    let powers: Vec<u32> = doc["shared_main_power"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 51);
    for case in cases {
        let input = &case["input"];
        let count = input["ability_count"].as_u64().unwrap() as usize;
        // Arena handles are 1-based indexes into `arena`.
        let mut arena: Vec<Ability> = (0..count)
            .map(|i| Ability::new(i as u32, Some(i as u32), F32(powers[i % powers.len()])))
            .collect();
        let mut state =
            DefenderState::new(ActorId::from_raw(3), (1..=count as u32).collect::<Vec<_>>());
        let text = input["payload"].as_str().unwrap();
        let payload: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect();
        let (SavedComponent::Defender(saved), consumed) =
            SavedComponent::decode_prefix(20, &payload).unwrap()
        else {
            panic!()
        };
        assert_eq!(consumed, case["native"]["consumed"].as_u64().unwrap() as usize);
        if let Some(records) = &saved.abilities {
            for (index, record) in records.values.iter().enumerate() {
                let handle = state.load_target(index).unwrap();
                aa_combat::commander::apply_ability(
                    &mut arena[handle as usize - 1],
                    &aa_mapworld::commander_restore::saved_ability(record),
                )
                .unwrap();
            }
        }
        state
            .load_entries(saved.pairs.as_ref().map(|p| p.values.as_slice()), |h| {
                Ok(arena[h as usize - 1].word14)
            })
            .unwrap();
        let native = &case["native"]["state"];
        for (ours, theirs) in arena.iter().zip(native["abilities"].as_array().unwrap()) {
            assert_eq!(u64::from(ours.definition), theirs["definition"].as_u64().unwrap());
            assert_eq!(u64::from(ours.use_count), theirs["use_count"].as_u64().unwrap(), "{case}");
            assert_eq!(u64::from(ours.last_use.0), theirs["last_use"].as_u64().unwrap(), "{case}");
            assert_eq!(u64::from(ours.word14), theirs["word14"].as_u64().unwrap());
            // aa-combat stores the enabled byte as bool (native tests it nonzero).
            assert_eq!(ours.enabled, theirs["enabled"].as_u64().unwrap() != 0);
            assert_eq!(u64::from(ours.recharge_bypass), theirs["recharge_bypass"].as_u64().unwrap());
            assert_eq!(u64::from(ours.power.0), theirs["power"].as_u64().unwrap());
            assert_eq!(
                Value::from(ours.summoned_ids.clone()),
                theirs["summoned_head_first"],
                "{case}"
            );
        }
        let entries: Vec<Value> = state.entries.iter().map(|e| Value::from(e.to_vec())).collect();
        assert_eq!(Value::Array(entries), native["entries_head_first"], "{case}");
        assert_eq!(
            state.entries.last().map(|e| Value::from(e.to_vec())).unwrap_or(Value::Null),
            native["tail"]
        );
    }
}
