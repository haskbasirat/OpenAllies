use aa_mapworld::{MapArchive, WorldInventory, kingdoms::Kingdoms, ownership::OwnerTarget};
use serde_json::Value;
use std::path::PathBuf;
fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/phase6-kingdoms/native-kingdoms.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
#[ignore = "requires original RMP corpus; all reads are read-only"]
fn all_kingdom_records_match_original_reader_boundaries_and_identity() {
    let fixture = fixture();
    let mut total = 0;
    for row in fixture["maps"].as_array().unwrap() {
        let path = row["map"].as_str().unwrap();
        let map = MapArchive::from_bytes(std::fs::read(path).unwrap()).unwrap();
        let chunk = map.decode_chunk("KINGDOMS", 128 << 20).unwrap();
        let kingdoms =
            Kingdoms::decode(&chunk, row["world_version"].as_u64().unwrap() as u32).unwrap();
        assert_eq!(kingdoms.source.sha256, row["sha256"].as_str().unwrap());
        assert_eq!(
            chunk.bytes.len(),
            row["decoded_bytes"].as_u64().unwrap() as usize
        );
        assert_eq!(
            kingdoms.major_count,
            row["major_count"].as_u64().unwrap() as u32
        );
        for (got, want) in kingdoms
            .slots
            .iter()
            .zip(row["entries"].as_array().unwrap())
        {
            assert_eq!(
                got.source_presence_byte,
                want["present_byte"].as_u64().unwrap() as u8
            );
            if let Some(got) = &got.record {
                total += 1;
                assert_eq!(
                    got.source_span.offset,
                    want["offset"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    got.source_span.offset + got.source_span.length,
                    want["end"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    got.serialized_id,
                    want["serialized_id"].as_u64().unwrap() as u32
                );
                assert_eq!(
                    got.payload.word_188,
                    want["word_188"].as_u64().unwrap() as u32
                );
                assert_eq!(
                    got.payload.word_18c,
                    want["word_18c"].as_u64().unwrap() as u32
                );
                assert_eq!(
                    got.payload.word_190,
                    want["word_190"].as_u64().unwrap() as u32
                );
                assert_eq!(
                    got.definition_id_utf16,
                    want["definition"]
                        .as_str()
                        .unwrap()
                        .encode_utf16()
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    assert_eq!(total, 328);
}
#[test]
#[ignore = "requires original RMP corpus; all reads are read-only"]
fn world_resolves_actual_owner_handles_including_minor_kingdoms() {
    let fixture = fixture();
    let mut references = 0;
    let mut bound = 0;
    let mut missing = Vec::new();
    for row in fixture["maps"].as_array().unwrap() {
        let path = row["map"].as_str().unwrap();
        let map = MapArchive::from_bytes(std::fs::read(path).unwrap()).unwrap();
        let world = WorldInventory::decode(&map, 128 << 20).unwrap();
        assert!(
            world.teams_error.is_none(),
            "{path}: {:?}",
            world.teams_error
        );
        assert!(
            world.kingdoms_error.is_none(),
            "{path}: {:?}",
            world.kingdoms_error
        );
        assert!(world.team_generation_mismatches.is_empty());
        assert!(world.kingdom_generation_mismatches.is_empty());
        assert!(
            world
                .opaque_initialization
                .iter()
                .all(|s| s.name != "TEAMS" && s.name != "KINGDOMS")
        );
        let owners = world.ownership.as_ref().unwrap();
        for reference in &owners.references {
            references += 1;
            assert_eq!(
                owners
                    .runtime_owner_slot(reference.dynamic_record_ordinal, 0)
                    .unwrap(),
                None
            );
            match &reference.target {
                OwnerTarget::Null => assert_eq!(
                    owners
                        .runtime_owner_slot(reference.dynamic_record_ordinal, 1 << 23)
                        .unwrap(),
                    None
                ),
                OwnerTarget::Kingdom {
                    source_slot,
                    serialized_id,
                } => {
                    bound += 1;
                    assert_eq!(reference.owner_handle, *serialized_id);
                    assert_eq!(
                        owners
                            .runtime_owner_slot(reference.dynamic_record_ordinal, 1 << 23)
                            .unwrap(),
                        Some(*source_slot)
                    );
                    assert_eq!(
                        world.kingdoms.as_ref().unwrap().slots[*source_slot]
                            .record
                            .as_ref()
                            .unwrap()
                            .serialized_id,
                        *serialized_id
                    );
                }
                other => missing.push((
                    path.to_string(),
                    reference.dynamic_record_ordinal,
                    format!("{other:?}"),
                )),
            }
        }
        if path.ends_with("Campaign\\axis01_crete.RMP")
            || path.ends_with("Campaign\\axis01_crete.rmp")
        {
            let minors = owners
                .references
                .iter()
                .filter(|r| r.owner_handle == 12344)
                .collect::<Vec<_>>();
            assert_eq!(minors.len(), 18);
            assert!(minors.iter().all(|r| r.target
                == OwnerTarget::Kingdom {
                    source_slot: 4,
                    serialized_id: 12344
                }));
        }
    }
    assert!(missing.is_empty(), "{missing:?}");
    assert_eq!(references, 11701);
    println!("{references} owner references, {bound} non-null exact Kingdom bindings");
}
