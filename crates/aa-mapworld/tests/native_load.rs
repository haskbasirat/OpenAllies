use aa_mapworld::{
    Chunk, ChunkDirectory, IdManagerState, SourceIdentity, ThingState, WorldState,
    archive::{Encoding, sha256},
};
use serde_json::Value;
fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase6-mapworld/native-load-fixtures.json"
    ))
    .unwrap()
}
fn chunk(name: &str, bytes: Vec<u8>) -> Chunk {
    Chunk {
        map_source: SourceIdentity::of(b"native oracle synthetic bounded stream"),
        directory: ChunkDirectory {
            name: name.into(),
            directory_ordinal: 0,
            packed_offset: 12,
            packed_bytes: bytes.len() as u64,
            source_timestamp: 0,
            decoded_bytes: bytes.len() as u64,
        },
        payload_source: SourceIdentity::of(&bytes),
        encoding: Encoding::Raw,
        bytes,
    }
}

#[test]
fn thing_load_exact_bytes_flags_and_version_one_skip_match_original_x86() {
    let f = fixture();
    let cases = f["thing_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 48);
    for row in cases {
        let bytes = unhex(row["input_hex"].as_str().unwrap());
        let (state, consumed) = ThingState::decode_prefix(&bytes).unwrap();
        let expected: ThingState = serde_json::from_value(row["fields"].clone()).unwrap();
        assert_eq!(state, expected);
        assert_eq!(consumed, bytes.len());
        assert_eq!(
            state.runtime_flags(
                row["existing_flags"].as_u64().unwrap() as u32,
                row["definition_flags"].as_u64().unwrap() as u32
            ),
            row["native_flags"].as_u64().unwrap() as u32
        );
        assert_eq!(
            state.position_bits,
            serde_json::from_value::<[u32; 3]>(row["native_position_bits"].clone()).unwrap()
        );
        assert_eq!(
            state.facing_bits,
            serde_json::from_value::<[u32; 3]>(row["native_facing_bits"].clone()).unwrap()
        );
        for length in 0..bytes.len() {
            assert!(
                ThingState::decode_prefix(&bytes[..length]).is_err(),
                "truncated length {length}"
            );
        }
    }
}

#[test]
fn id_manager_exact_generation_prefix_and_zero_tail_match_original_x86() {
    let f = fixture();
    let cases = f["id_manager_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 8);
    for row in cases {
        let input = chunk("IDMGR", unhex(row["input_hex"].as_str().unwrap()));
        let result = IdManagerState::decode(&input);
        if row.get("error").is_some() {
            assert!(result.is_err());
            continue;
        }
        let state = result.unwrap();
        assert_eq!(state.next_slot, row["next_slot"].as_u64().unwrap() as u32);
        assert_eq!(
            state.highest_slot,
            row["highest_slot"].as_u64().unwrap() as u32
        );
        let mut bytes = vec![0u8; 65536 * 2];
        for (slot, generation) in state.serialized_generations.iter().enumerate() {
            bytes[slot * 2..slot * 2 + 2].copy_from_slice(&generation.to_le_bytes());
            assert!(state.generation_matches((u32::from(*generation) << 16) | slot as u32));
            assert!(
                !state.generation_matches(
                    (u32::from(generation.wrapping_add(1)) << 16) | slot as u32
                )
            );
        }
        assert_eq!(sha256(&bytes), row["generations_sha256"].as_str().unwrap());
    }
}

#[test]
fn world_prefix_version_gates_match_original_x86_read_segment() {
    let f = fixture();
    let cases = f["world_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 29);
    for row in cases {
        let state = WorldState::decode(&chunk("GWORLD", unhex(row["input_hex"].as_str().unwrap())))
            .unwrap();
        let mut actual = std::collections::BTreeMap::from([
            ("0xd4", state.rng_state),
            ("0xdc", state.word_dc),
            ("0xe0", state.begin_time_bits),
            ("0xe4", state.word_e4),
            ("0xe8", state.delta_bits),
            ("0x140", state.word_140),
        ]);
        for (key, value) in [
            ("0x184", state.word_184),
            ("0x194", state.word_194),
            ("0x198", state.word_198),
            ("0x44", state.word_44),
            ("0x48", state.word_48),
        ] {
            if let Some(value) = value {
                actual.insert(key, value);
            }
        }
        assert_eq!(serde_json::to_value(actual).unwrap(), row["native_fields"]);
        assert!(state.trailing.is_empty());
    }
}

#[test]
fn actor_owner_gate_and_serialized_state_prefix_match_original_x86() {
    let f = fixture();
    let cases = f["actor_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    for row in cases {
        let bytes = unhex(row["input_hex"].as_str().unwrap());
        let (prefix, used) = aa_mapworld::inventory::ActorPrefix::decode_prefix(&bytes).unwrap();
        assert_eq!(used, bytes.len());
        assert_eq!(
            prefix.owner_handle,
            row["owner_handle"].as_u64().unwrap() as u32
        );
        assert_eq!(
            prefix.state_after_read(serde_json::from_value(row["prior_state"].clone()).unwrap()),
            serde_json::from_value::<[u32; 2]>(row["native_state"].clone()).unwrap()
        );
        assert_eq!(
            prefix
                .owner_reference(row["thing_flags"].as_u64().unwrap() as u32)
                .is_some_and(|id| id != 0),
            row["native_owner_present"].as_bool().unwrap()
        );
        assert_eq!(
            prefix.upgraded_world_version(14),
            row["native_world_version"].as_u64().unwrap() as u32
        );
    }
}
