use super::*;

fn word(out: &mut Vec<u8>, value: u32) {
    out.extend(value.to_le_bytes());
}
fn suffix() -> Vec<u8> {
    let mut out = vec![0; 5]; // disabled version-0 region graph
    word(&mut out, 0);
    word(&mut out, 3);
    for value in [0_f32, 0., 2., 0., 0., 2.] {
        word(&mut out, value.to_bits());
    }
    word(&mut out, 3);
    for value in [0, 1, 2] {
        word(&mut out, value);
    }
    word(&mut out, 2);
    for value in [0, 0, 0, 2, 1, 0, 4, 3] {
        word(&mut out, value);
    }
    out
}

#[test]
fn cached_empty_and_missing_cells_keep_the_native_distinction() {
    let got = decode_suffix(&suffix(), 2, 1).unwrap();
    assert_eq!(got.vertices, [[0, 0], [0x4000_0000, 0], [0, 0x4000_0000]]);
    assert_eq!(got.indices, [0, 1, 2]);
    assert_eq!(got.cell_points([0, 0]).unwrap().unwrap(), got.vertices);
    assert_eq!(got.cell_points([1, 0]).unwrap(), Some(Vec::new()));
    assert_eq!(got.cell_points([0, 1]).unwrap(), None);
}

#[test]
fn malformed_auxiliary_data_is_rejected_before_use() {
    let bytes = suffix();
    for end in 0..bytes.len() {
        assert!(decode_suffix(&bytes[..end], 2, 1).is_err());
    }
    let mut changed = bytes.clone();
    changed[9..13].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_suffix(&changed, 2, 1).is_err());
    let mut changed = bytes.clone();
    changed[41..45].copy_from_slice(&8_u32.to_le_bytes());
    assert!(decode_suffix(&changed, 2, 1).is_err());
    let mut changed = bytes;
    changed.push(0);
    assert!(decode_suffix(&changed, 2, 1).is_err());
}

/// The phase-6 native reader proof (original 0x7AA9B0 + 0x55C540 + 0x55B9E0 on
/// all 80 original maps): offsets, vertex/index byte hashes and the ordered
/// cell entries.
#[test]
#[ignore = "requires the original maps (AA_GAME_ROOT)"]
fn all_original_caches_match_the_native_reader() {
    use aa_mapworld::{terrain::TerrainChunk, MapArchive};
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let proof = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase6-water-geometry/native-water-geometry.json");
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(proof).unwrap()).unwrap();
    let maps = fixture["maps"].as_object().unwrap();
    assert_eq!(maps.len(), 80);
    for (path, row) in maps {
        let archive = MapArchive::from_bytes(std::fs::read(root.join("reconstruction/assets/original/Data").join(path)).unwrap()).unwrap();
        let chunk = archive.decode_chunk("TERRAIN", u64::MAX).unwrap();
        assert_eq!(chunk.payload_source.sha256, row["terrain_sha256"]);
        let terrain = TerrainChunk::decode(&chunk).unwrap();
        let width = row["width"].as_u64().unwrap() as u32;
        let height = row["height"].as_u64().unwrap() as u32;
        let got = decode_suffix(&terrain.pending_auxiliary_bytes, width, height).unwrap();
        assert_eq!(got.vertices.len() as u64, row["vertex_count"], "{path}");
        assert_eq!(got.indices.len() as u64, row["index_count"], "{path}");
        assert_eq!(got.cells.len() as u64, row["cell_count"], "{path}");
        let vertices: Vec<u8> = got.vertices.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
        let indices: Vec<u8> = got.indices.iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(aa_mapworld::archive::sha256(&vertices), row["vertices_sha256"], "{path}");
        assert_eq!(aa_mapworld::archive::sha256(&indices), row["indices_sha256"], "{path}");
        let mut entries: Vec<[u32; 5]> = got
            .cells
            .iter()
            .map(|(cell, range)| {
                let (first, last) = if range.is_empty() { (1, 0) } else { (range.start as u32, (range.end - 1) as u32) };
                [cell[0] + cell[1] * width, cell[0], cell[1], first, last]
            })
            .collect();
        entries.sort_unstable();
        let encoded = serde_json::to_vec(&entries).unwrap();
        // Empty ranges are not distinguishable after decoding; only nonempty caches carry entries.
        if entries.iter().all(|e| e[3] <= e[4]) {
            assert_eq!(aa_mapworld::archive::sha256(&encoded), row["entries_sha256"], "{path}");
        }
        for lookup in row["lookups"].as_array().unwrap() {
            let cell = [lookup["cell"][0].as_i64().unwrap() as i32, lookup["cell"][1].as_i64().unwrap() as i32];
            assert_eq!(got.cell_points(cell).unwrap().is_some(), lookup["exists"].as_bool().unwrap(), "{path} {cell:?}");
        }
    }
}
