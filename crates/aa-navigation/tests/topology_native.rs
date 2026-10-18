use aa_mapworld::{MapArchive, archive::sha256, terrain::TerrainChunk};
use aa_navigation::topology::{GridLayout, StaticTile, Topology};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
/// Read-only original game folder (the one holding AA.exe and `reconstruction/`).
fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"))
}
fn layout(row: &Value) -> GridLayout {
    let v = &row["layout"];
    GridLayout {
        width: v["width"].as_i64().unwrap() as i32,
        height: v["height"].as_i64().unwrap() as i32,
        coordinate_shift: v["coordinate_shift"].as_u64().unwrap() as u32,
        extent_bits: std::array::from_fn(|i| {
            (v["extent_i32"][i].as_i64().unwrap() as f32).to_bits()
        }),
    }
}
fn compare(topo: &Topology, native: &Value, p: Precision) {
    let graph = topo.prepare_graph().unwrap();
    assert!(
        !graph.ready,
        "topology does not fabricate full world readiness"
    );
    let regions=graph.nodes.iter().enumerate().map(|(i,n)|json!({"position_bits":n.position_bits,"category":n.category,"rectangles":topo.region_rectangles[i],"neighbors":n.neighbors,"area":topo.region_area(i).unwrap(),"distance_cache":n.distance_cache})).collect::<Vec<_>>();
    assert_eq!(json!(regions), native["regions"]);
    let bytes = topo
        .tile_rectangles
        .iter()
        .flat_map(|v| v.unwrap_or(u32::MAX).to_le_bytes())
        .collect::<Vec<_>>();
    assert_eq!(sha256(&bytes), native["tile_rectangles_sha256"]);
    if topo.enabled {
        assert_eq!(
            topo.diagonal_bits(p).unwrap(),
            native["diagonal_bits"].as_u64().unwrap() as u32
        );
    }
    for query in native["lookups"].as_array().unwrap() {
        let point = serde_json::from_value(query["point_bits"].clone()).unwrap();
        assert_eq!(json!(topo.region_at(point).unwrap()), query["region"]);
    }
}
#[test]
fn saved_regions_match_native_publication_neighbor_order_and_lookup() {
    let doc: Value = serde_json::from_slice(
        &std::fs::read(root().join("research/phase7-navigation/synthetic-topology.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(doc["fixtures"].as_array().unwrap().len(), 21);
    for row in doc["fixtures"].as_array().unwrap() {
        let input = row["input_hex"]
            .as_str()
            .unwrap()
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        let (topo, consumed) = Topology::decode_prefix(&input, layout(row), 28).unwrap();
        assert_eq!(consumed as u64, row["native"]["consumed"]);
        compare(
            &topo,
            &row["native"],
            serde_json::from_value(row["precision"].clone()).unwrap(),
        );
        let tiles = row["flags20"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let flags20 = v.as_u64().unwrap() as u32;
                StaticTile {
                    flags20,
                    definition_type: (flags20 >> 4) & 7,
                }
            })
            .collect::<Vec<_>>();
        if let Some(result) = topo
            .post_load(
                &tiles,
                serde_json::from_value(row["precision"].clone()).unwrap(),
            )
            .unwrap()
        {
            assert_eq!(
                json!(result.primary_island),
                row["native"]["primary_island"]
            );
        }
        for end in 0..consumed {
            assert!(
                Topology::decode_prefix(&input[..end], layout(row), 28).is_err(),
                "{} truncation{end}",
                row["name"]
            );
        }
        assert!(Topology::decode_prefix(&input, layout(row), 16).is_err());
    }
}
#[test]
#[ignore = "requires original local maps; run with AA_GAME_ROOT and --include-ignored"]
fn all_original_saved_topology_matches_original_load_and_adjacency() {
    let doc: Value = serde_json::from_slice(
        &std::fs::read(root().join("research/phase7-navigation/native-topology.json")).unwrap(),
    )
    .unwrap();
    let maps = doc["maps"].as_object().unwrap();
    assert_eq!(maps.len(), 80);
    let meta_bytes =
        std::fs::read(root().join("research/phase6-terrain/terrain-render-metadata.json")).unwrap();
    let types_bytes =
        std::fs::read(root().join("research/engine-baseline/terrain-variation-metadata.json"))
            .unwrap();
    assert_eq!(sha256(&meta_bytes), doc["render_metadata_sha256"]);
    assert_eq!(sha256(&types_bytes), doc["types_metadata_sha256"]);
    let meta: Value = serde_json::from_slice(&meta_bytes).unwrap();
    let types: Value = serde_json::from_slice(&types_bytes).unwrap();
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
        assert_eq!(
            sha256(&terrain.pending_auxiliary_bytes),
            row["suffix_sha256"]
        );
        let (topo, consumed) = Topology::decode_prefix(
            &terrain.pending_auxiliary_bytes,
            layout(row),
            row["world_version"].as_u64().unwrap() as u32,
        )
        .unwrap();
        assert_eq!(consumed as u64, row["consumed"], "{path}");
        compare(&topo, row, Precision::Double53);
        let resolve = |id: &str| {
            let d = meta["variants"].get(id)?;
            let t = &types["variations"][d["canonical"].as_str().unwrap()];
            let bytes = std::fs::read(
                game_root()
                    .join("reconstruction/assets/original/Data")
                    .join(d["path"].as_str().unwrap()),
            )
            .unwrap();
            let source = aa_mapworld::SourceIdentity::of(&bytes);
            assert_eq!(source.sha256, d["sha256"]);
            Some(aa_mapworld::terrain::ResolvedVariation {
                id_utf16: id.encode_utf16().collect(),
                native_type: t["native_type"].as_u64().unwrap() as u32,
                passability_bits: u8::from(d["land"].as_bool().unwrap())
                    | u8::from(d["water"].as_bool().unwrap()) << 1,
                source,
            })
        };
        let palette = terrain
            .palette_utf16
            .iter()
            .map(|v| resolve(&String::from_utf16(v).unwrap()))
            .collect::<Vec<_>>();
        let biome = String::from_utf16(&terrain.biome_utf16).unwrap();
        let initial = resolve(
            meta["biome_defaults"][biome]["variation_id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let terrain_layout = aa_mapworld::terrain::TerrainGridLayout::from_world_extents(
            topo.layout.extent_bits,
            topo.layout.coordinate_shift,
        )
        .unwrap();
        let bound = terrain
            .bind_with_native_fallback(terrain_layout, &palette, initial)
            .unwrap();
        let tiles = bound
            .cells
            .iter()
            .map(|c| StaticTile {
                flags20: c.flags20,
                definition_type: match c.resolved_palette_index {
                    Some(i) => palette[i as usize].as_ref().unwrap().native_type,
                    None => bound.initial_variation.as_ref().unwrap().native_type,
                },
            })
            .collect::<Vec<_>>();
        let post = topo
            .post_load(&tiles, Precision::Double53)
            .unwrap()
            .unwrap();
        assert_eq!(json!(post.primary_island), row["primary_island"], "{path}");
    }
}

#[test]
#[ignore = "requires original local maps; run with AA_GAME_ROOT and --include-ignored"]
fn source_topology_feeds_exact_native_region_routes() {
    use aa_navigation::region_search::{Request, State, search};
    let doc: Value = serde_json::from_slice(
        &std::fs::read(root().join("research/phase7-navigation/native-map-routes.json")).unwrap(),
    )
    .unwrap();
    let topo_bytes =
        std::fs::read(root().join("research/phase7-navigation/native-topology.json")).unwrap();
    assert_eq!(sha256(&topo_bytes), doc["topology_fixture_sha256"]);
    let topo_proof: Value = serde_json::from_slice(&topo_bytes).unwrap();
    assert_eq!(doc["fixtures"].as_array().unwrap().len(), 9);
    for case in doc["fixtures"].as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        let row = &topo_proof["maps"][path];
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
        let (topo, _) = Topology::decode_prefix(
            &terrain.pending_auxiliary_bytes,
            layout(row),
            row["world_version"].as_u64().unwrap() as u32,
        )
        .unwrap();
        let mut graph = topo.prepare_graph().unwrap();
        graph.ready = true; //Explicit native fixture startup boundary.
        let mut state = State {
            busy: false,
            output_linked: false,
        };
        for expected in case["routes"].as_array().unwrap() {
            let mut path = Vec::new();
            let request = Request {
                start: expected["start"].as_u64().unwrap() as u32,
                goal: expected["goal"].as_u64().unwrap() as u32,
                precision: serde_json::from_value(case["precision"].clone()).unwrap(),
                queue_limit: Some(8192),
            };
            let result = search(&mut graph, request, &mut state, &mut path).unwrap();
            let actual = json!({"start":request.start,"goal":request.goal,"status":result.status,"route":path,"epoch":graph.epoch,"node_states":graph.nodes.iter().map(|n|[n.visit_epoch,n.parent_neighbor]).collect::<Vec<_>>(),"busy":state.busy,"output_linked":state.output_linked});
            assert_eq!(actual, *expected, "{} {}", case["path"], case["precision"]);
        }
    }
}
