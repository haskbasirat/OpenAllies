use aa_mapworld::{MapArchive, kingdoms::Teams};
#[test]
#[ignore = "requires original RMP corpus; comparison with complete original x86 team reader"]
fn all_original_team_records_match_original_load_and_stream_boundaries() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase6-kingdoms/native-teams.json"
    ))
    .unwrap();
    let maps = fixture["maps"].as_array().unwrap();
    assert_eq!(maps.len(), 80);
    let mut team_count = 0;
    for row in maps {
        let map =
            MapArchive::from_bytes(std::fs::read(row["map"].as_str().unwrap()).unwrap()).unwrap();
        let chunk = map.decode_chunk("TEAMS", 128 * 1024 * 1024).unwrap();
        let teams = Teams::decode(&chunk).unwrap();
        assert_eq!(teams.source.sha256, row["sha256"].as_str().unwrap());
        assert_eq!(
            chunk.bytes.len(),
            row["decoded_bytes"].as_u64().unwrap() as usize
        );
        for (slot, expected) in teams.slots.iter().zip(row["entries"].as_array().unwrap()) {
            assert_eq!(
                slot.source_presence_byte,
                expected["present_byte"].as_u64().unwrap() as u8
            );
            let Some(team) = &slot.record else {
                continue;
            };
            team_count += 1;
            assert_eq!(
                team.source_span.offset,
                expected["offset"].as_u64().unwrap() as usize
            );
            assert_eq!(
                team.source_span.offset + team.source_span.length,
                expected["end"].as_u64().unwrap() as usize
            );
            assert_eq!(
                team.serialized_id,
                expected["serialized_id"].as_u64().unwrap() as u32
            );
            assert_eq!(
                team.slot_index,
                expected["slot_index"].as_u64().unwrap() as u32
            );
            assert_eq!(
                team.definition_id_utf16,
                expected["definition"]
                    .as_str()
                    .unwrap()
                    .encode_utf16()
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                team.display_key_utf16,
                expected["name_key"]
                    .as_str()
                    .unwrap()
                    .encode_utf16()
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                team.words_4c,
                serde_json::from_value::<Vec<u32>>(expected["words_4c"].clone()).unwrap()
            );
            assert_eq!(
                team.words_58,
                serde_json::from_value::<Vec<u32>>(expected["words_58"].clone()).unwrap()
            );
            assert_eq!(
                team.string_64_utf16.as_ref().unwrap(),
                &expected["string_64"]
                    .as_str()
                    .unwrap()
                    .encode_utf16()
                    .collect::<Vec<_>>()
            );
        }
    }
    assert_eq!(team_count, 21);
}
