use aa_mapworld::{MapArchive, archive::sha256, terrain::TerrainChunk};
use aa_navigation::{
    preparation,
    topology::{GridLayout, Topology},
};
use serde_json::{Value, json};
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
/// Read-only original game folder (the one holding AA.exe and `reconstruction/`).
fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"))
}
fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(root().join("research/phase7-readiness/native-preparation.json")).unwrap(),
    )
    .unwrap()
}
fn check(bytes: &[u8], row: &Value) {
    let v = &row["layout"];
    let layout = GridLayout {
        width: v["width"].as_i64().unwrap() as i32,
        height: v["height"].as_i64().unwrap() as i32,
        coordinate_shift: v["coordinate_shift"].as_u64().unwrap() as u32,
        extent_bits: std::array::from_fn(|i| {
            (v["extent_i32"][i].as_i64().unwrap() as f32).to_bits()
        }),
    };
    let (topo, consumed) =
        Topology::decode_prefix(bytes, layout, row["world_version"].as_u64().unwrap() as u32)
            .unwrap();
    let prepared = preparation::prepare(&topo, row["depth"].as_i64().unwrap() as i32).unwrap();
    assert!(!prepared.graph.ready);
    let actual = json!({"consumed":consumed,"prepared":prepared.prepared,"areas":prepared.areas.iter().map(|&v|v as u32).collect::<Vec<_>>(),"bottlenecks":prepared.bottlenecks,"neighbors":prepared.graph.nodes.iter().map(|n|&n.neighbors).collect::<Vec<_>>(),"cache_lengths":prepared.route_cache.iter().map(Vec::len).collect::<Vec<_>>(),"cache_sha256":sha256(&prepared.route_cache.into_iter().flatten().collect::<Vec<_>>())});
    assert_eq!(
        actual,
        row["native"],
        "{}",
        row.get("name").unwrap_or(&Value::Null)
    );
}
#[test]
fn complete_native_fresh_preparation_matches_bottlenecks_and_cache_rows() {
    let doc = fixture();
    let rows = doc["fixtures"].as_array().unwrap();
    assert_eq!(rows.len(), 42);
    for row in rows {
        let bytes = row["input_hex"]
            .as_str()
            .unwrap()
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        check(&bytes, row);
    }
}
#[test]
#[ignore = "requires original local maps; run with AA_GAME_ROOT and --include-ignored"]
fn native_source_map_preparation_matches_original() {
    let doc = fixture();
    let maps = doc["maps"].as_object().unwrap();
    assert_eq!(maps.len(), 3);
    for (path, row) in maps {
        let archive = MapArchive::from_bytes(
            std::fs::read(
                game_root()
                    .join("reconstruction/assets/original/Data")
                    .join(path),
            )
            .unwrap(),
        )
        .unwrap();
        let chunk = archive.decode_chunk("TERRAIN", u64::MAX).unwrap();
        assert_eq!(chunk.payload_source.sha256, row["terrain_sha256"]);
        let terrain = TerrainChunk::decode(&chunk).unwrap();
        assert_eq!(terrain.suffix_offset as u64, row["suffix_offset"]);
        check(&terrain.pending_auxiliary_bytes, row);
    }
}
