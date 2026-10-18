use aa_mapworld::{MapArchive, WorldInventory};
use std::path::{Path, PathBuf};
fn maps(path: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            maps(&path, out)
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rmp"))
        {
            out.push(path)
        }
    }
}
#[test]
#[ignore = "requires recovered original RMP corpus, read-only"]
fn all_original_maps_decode_exact_chunks_and_identity_generations() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../reconstruction");
    let original = root.join("assets/original/Data/Maps");
    let mut paths = Vec::new();
    maps(&original, &mut paths);
    assert_eq!(paths.len(), 80);
    let mut total = 0;
    for path in paths {
        let map = MapArchive::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let world = WorldInventory::decode(&map, 128 * 1024 * 1024).unwrap();
        assert!(
            world.header_error.is_none(),
            "{} {:?}",
            path.display(),
            world.header_error
        );
        assert!(world.header.is_some());
        assert!(
            world.dynamic_things.unsupported.is_empty(),
            "{}",
            path.display()
        );
        assert!(world.generation_mismatches.is_empty(), "{}", path.display());
        let rel = path.strip_prefix(&original).unwrap().with_extension("json");
        let json: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("game/data/map-actors").join(rel)).unwrap(),
        )
        .unwrap();
        let expected = json["actors"].as_array().unwrap();
        assert_eq!(expected.len(), world.dynamic_things.placements.len());
        total += expected.len();
        for (placement, old) in world.dynamic_things.placements.iter().zip(expected) {
            assert_eq!(
                placement.serialized_id,
                old["instanceId"].as_u64().unwrap() as u32
            );
            assert_eq!(
                placement.thing_id_utf16,
                old["thingId"]
                    .as_str()
                    .unwrap()
                    .encode_utf16()
                    .collect::<Vec<_>>()
            );
            for (axis, key) in ["x", "y", "z"].into_iter().enumerate() {
                assert_eq!(
                    placement.thing.position_bits[axis],
                    (old[key].as_f64().unwrap() as f32).to_bits()
                );
            }
            assert_eq!(
                placement.source_span.offset,
                old["recordOffset"].as_u64().unwrap() as usize
            );
            assert_eq!(
                placement.source_span.length,
                old["recordLength"].as_u64().unwrap() as usize
            );
            assert_eq!(
                placement
                    .object
                    .as_ref()
                    .and_then(|o| o.nested_count)
                    .unwrap_or(0),
                0
            );
        }
    }
    assert_eq!(total, 225851);
}
