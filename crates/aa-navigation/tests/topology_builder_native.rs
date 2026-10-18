//! Replays research/phase8-movement/native-topology-builder.json: the original
//! 7ACB00 static map analysis (and original 7BCA60/7B8FB0 on its result) run
//! under Unicorn, against `topology_builder::build` + `Topology::prepare_graph`.
use aa_mapworld::archive::sha256;
use aa_navigation::topology::{GridLayout, StaticTile};
use aa_navigation::topology_builder::{
    Error, StaticAnalysis, StaticGrid, StaticMapParameters, build, next_square_size, terrain_type,
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::path::PathBuf;
/// Read-only original game folder (the one holding AA.exe and `reconstruction/`).
fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"))
}

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-movement/native-topology-builder.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn tiles(case: &Value) -> Vec<StaticTile> {
    if let Some(types) = case.get("tile_definition_types") {
        return types
            .as_array()
            .unwrap()
            .iter()
            .map(|v| StaticTile {
                flags20: 0,
                definition_type: v.as_u64().unwrap() as u32,
            })
            .collect();
    }
    case["tiles"]
        .as_str()
        .unwrap()
        .chars()
        .map(|c| {
            let code = c.to_digit(32).unwrap();
            StaticTile {
                flags20: (code >> 3) << 8,
                definition_type: code & 7,
            }
        })
        .collect()
}

fn layout(case: &Value) -> GridLayout {
    let l = &case["layout"];
    GridLayout {
        width: l["width"].as_i64().unwrap() as i32,
        height: l["height"].as_i64().unwrap() as i32,
        coordinate_shift: l["coordinate_shift"].as_u64().unwrap() as u32,
        extent_bits: serde_json::from_value(l["extent_bits"].clone()).unwrap(),
    }
}

fn params(case: &Value) -> StaticMapParameters {
    serde_json::from_value(case["params"].clone()).unwrap()
}

fn run(case: &Value) -> Result<StaticAnalysis, Error> {
    let tiles = tiles(case);
    let grid = StaticGrid {
        layout: layout(case),
        cell_size_bits: case["cell_size_bits"].as_u64().unwrap() as u32,
        tiles: &tiles,
    };
    build(
        &grid,
        &params(case),
        serde_json::from_value(case["precision"].clone()).unwrap(),
    )
}

fn compare(case: &Value) {
    let name = case["name"].as_str().unwrap();
    let native = &case["native"];
    assert_eq!(native["terminated"], true, "{name}");
    assert_eq!(
        native["max_rectangle_width_7aa180"], case["params"]["max_rectangle_width"],
        "{name}"
    );
    // Native side effects owned by the caller (SAI map +4, B36518 +101).
    assert_eq!(native["analyzed"], 1, "{name}");
    assert_eq!(native["world_flag_101"], 1, "{name}");
    let out = run(case).unwrap();
    let topo = &out.topology;
    assert!(topo.enabled);
    let rects = topo
        .rectangles
        .iter()
        .map(|r| {
            json!({"region": r.region, "island": r.island, "bounds": r.bounds,
            "sides": r.sides.iter().map(|list| list.iter().map(|s| json!([s.bounds, s.rectangle])).collect::<Vec<_>>()).collect::<Vec<_>>()})
        })
        .collect::<Vec<_>>();
    assert_eq!(json!(rects), native["rectangles"], "{name} rectangles");
    let regions = native["regions"].as_array().unwrap();
    assert_eq!(topo.regions.len(), regions.len(), "{name} region count");
    for (i, (r, n)) in topo.regions.iter().zip(regions).enumerate() {
        assert_eq!(
            json!(topo.region_rectangles[i]),
            n["rectangles"],
            "{name} region {i}"
        );
        assert_eq!(
            json!(r.position_bits),
            n["position_bits"],
            "{name} center {i}"
        );
        // Native hash bucket order, including rehash growth (606750).
        assert_eq!(
            json!(r.distance_pairs),
            n["distance_pairs"],
            "{name} distances {i}"
        );
    }
    let islands = out.islands.iter().map(|i| &i.regions).collect::<Vec<_>>();
    assert_eq!(json!(islands), native["islands"], "{name} islands");
    assert_eq!(
        json!(out.post_load.primary_island),
        native["primary_island"],
        "{name} primary island"
    );
    assert_eq!(
        json!(out.post_load.diagonal_bits),
        native["diagonal_bits"],
        "{name} diagonal"
    );
    let packed = topo
        .tile_rectangles
        .iter()
        .flat_map(|v| v.unwrap_or(u32::MAX).to_le_bytes())
        .collect::<Vec<_>>();
    assert_eq!(
        sha256(&packed),
        native["tile_rectangles_sha256"],
        "{name} tiles"
    );
    if let Some(list) = native.get("tile_rectangles") {
        assert_eq!(json!(topo.tile_rectangles), *list, "{name} tiles");
    }
    assert_eq!(
        out.rectangle_count_warning,
        !native["warnings"].as_array().unwrap().is_empty(),
        "{name} 0x1000 warning"
    );
    // The existing routing preparation consumes the built topology exactly as
    // original 7BCA60/7B8FB0 consume the built native objects.
    let graph = topo.prepare_graph().unwrap();
    let got = graph
        .nodes
        .iter()
        .map(|n| json!({"category": n.category, "neighbors": n.neighbors}))
        .collect::<Vec<_>>();
    assert_eq!(json!(got), native["graph"], "{name} graph");
    for (node, region) in graph.nodes.iter().zip(&topo.regions) {
        assert_eq!(node.position_bits, region.position_bits);
        assert_eq!(
            node.distance_cache.len(),
            region.distance_pairs.len(),
            "{name} unique keys"
        );
    }
}

#[test]
fn synthetic_grids_match_original_static_map_analysis() {
    let doc = fixture();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(doc["case_count"].as_u64().unwrap() as usize, cases.len());
    let mut seen = [0usize; 3];
    let mut compared = 0;
    for case in cases {
        if case["native"]["terminated"] == false {
            continue;
        }
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        seen[p.significand_bits() as usize / 30] += 1;
        compare(case);
        compared += 1;
    }
    assert!(compared >= 40, "{compared}");
    assert!(seen.iter().all(|&n| n > 0), "{seen:?}");
    // Coverage guards: the 0x1000 warning, uncovered tiles, hash rehash.
    let native = cases.iter().map(|c| &c["native"]);
    assert!(
        native
            .clone()
            .any(|n| n["warnings"].as_array().is_some_and(|w| !w.is_empty()))
    );
    assert!(native.clone().any(|n| {
        n["tile_rectangles"]
            .as_array()
            .is_some_and(|t| t.iter().any(Value::is_null))
    }));
    assert!(native.clone().any(|n| {
        n["regions"]
            .as_array()
            .is_some_and(|r| r.iter().any(|r| r["bucket_count"] != 11))
    }));
}

#[test]
fn real_map_grids_match_original_static_map_analysis() {
    let doc = fixture();
    let real = doc["real_maps"].as_array().unwrap();
    assert_eq!(doc["real_map_count"].as_u64().unwrap() as usize, real.len());
    assert!(!real.is_empty());
    for case in real {
        compare(case);
    }
}

/// Rust builder on the analysis classes recovered from each shipped map versus
/// the TERRAIN topology the original map ships, decoded by the existing port of
/// the original loader. Load normalization only: 7AA9B0 rebuilds each region's
/// rectangle list by head insertion in rectangle order (descending ids).
#[test]
#[ignore = "requires original local maps; run with AA_GAME_ROOT and --include-ignored"]
fn rust_builder_reproduces_every_shipped_map_topology() {
    use aa_mapworld::{MapArchive, terrain::TerrainChunk};
    use aa_navigation::topology::Topology;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc: Value = serde_json::from_slice(
        &std::fs::read(root.join("research/phase8-movement/native-topology-builder-shipped.json"))
            .unwrap(),
    )
    .unwrap();
    let phase7: Value = serde_json::from_slice(
        &std::fs::read(root.join("research/phase7-navigation/native-topology.json")).unwrap(),
    )
    .unwrap();
    let svars = &doc["svars"];
    let mut checked = 0;
    for (path, row) in doc["maps"].as_object().unwrap() {
        let width = match &row["matched_width"] {
            Value::String(s) if s == "current" => row["current_formula_width"].as_i64().unwrap(),
            Value::Number(n) => n.as_i64().unwrap(),
            other => panic!("{path}: no native reproduction recorded: {other}"),
        } as i32;
        let mut tiles = Vec::new();
        for run in row["class_rle"].as_array().unwrap() {
            let t = StaticTile {
                flags20: 0,
                definition_type: run[0].as_u64().unwrap() as u32,
            };
            tiles.extend(std::iter::repeat_n(t, run[1].as_u64().unwrap() as usize));
        }
        let case = json!({"layout": row["layout"]});
        let grid = StaticGrid {
            layout: layout(&case),
            cell_size_bits: row["cell_size_bits"].as_u64().unwrap() as u32,
            tiles: &tiles,
        };
        let p = StaticMapParameters {
            starting_square_size: svars["starting_square_size"].as_i64().unwrap() as i32,
            use_fast_square_reduction: svars["use_fast_square_reduction"].as_i64().unwrap() != 0,
            fast_square_reduction_mult: svars["fast_square_reduction_mult"].as_i64().unwrap()
                as i32,
            fast_square_reduction_div: svars["fast_square_reduction_div"].as_i64().unwrap() as i32,
            min_region_size: svars["min_region_size"].as_i64().unwrap() as i32,
            min_region_width: svars["min_region_width"].as_i64().unwrap() as i32,
            max_region_size_for_expansion: svars["max_region_size_for_expansion"].as_i64().unwrap()
                as i32,
            max_rectangle_width: width,
        };
        let mut built = build(&grid, &p, Precision::Double53).unwrap().topology;
        let bytes = std::fs::read(
            game_root().join("reconstruction/assets/original/Data")
                .join(path),
        )
        .unwrap();
        let archive = MapArchive::from_bytes(bytes).unwrap();
        let chunk = archive.decode_chunk("TERRAIN", u64::MAX).unwrap();
        assert_eq!(chunk.payload_source.sha256, row["terrain_sha256"], "{path}");
        let terrain = TerrainChunk::decode(&chunk).unwrap();
        let version = phase7["maps"][path]["world_version"].as_u64().unwrap() as u32;
        let (shipped, _) =
            Topology::decode_prefix(&terrain.pending_auxiliary_bytes, grid.layout, version)
                .unwrap();
        let rect = |t: &Topology| {
            json!(
                t.rectangles
                    .iter()
                    .map(|r| json!([
                        r.region,
                        r.island,
                        r.bounds,
                        r.sides
                            .iter()
                            .map(|l| l
                                .iter()
                                .map(|s| json!([s.bounds, s.rectangle]))
                                .collect::<Vec<_>>())
                            .collect::<Vec<_>>()
                    ]))
                    .collect::<Vec<_>>()
            )
        };
        assert_eq!(rect(&built), rect(&shipped), "{path} rectangles");
        assert_eq!(
            built.tile_rectangles, shipped.tile_rectangles,
            "{path} tiles"
        );
        for list in &mut built.region_rectangles {
            list.sort_unstable_by(|a, b| b.cmp(a));
        }
        assert_eq!(built.region_rectangles, shipped.region_rectangles, "{path}");
        let (a, b) = (
            built.prepare_graph().unwrap(),
            shipped.prepare_graph().unwrap(),
        );
        assert_eq!(a.nodes.len(), b.nodes.len(), "{path}");
        for (i, (x, y)) in a.nodes.iter().zip(&b.nodes).enumerate() {
            assert_eq!(x.position_bits, y.position_bits, "{path} center {i}");
            assert_eq!(x.category, y.category, "{path} category {i}");
            assert_eq!(x.neighbors, y.neighbors, "{path} neighbors {i}");
            assert_eq!(x.distance_cache, y.distance_cache, "{path} distances {i}");
        }
        checked += 1;
    }
    assert_eq!(checked, doc["map_count"].as_u64().unwrap() as usize);
}

#[test]
fn native_nonterminating_configuration_is_an_explicit_error() {
    let doc = fixture();
    let case = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["native"]["terminated"] == false)
        .unwrap();
    // The original did not reach its return within the recorded budget; the
    // 7A9B00 reduction maps the size onto itself, so it never can.
    assert!(case["native"]["instruction_limit"].as_u64().unwrap() > 0);
    let p = params(case);
    let s = p.starting_square_size;
    assert_eq!(next_square_size(s, &p).unwrap(), s);
    assert!(matches!(run(case), Err(Error::NonTerminating(_))));
}

#[test]
fn native_faults_are_errors_not_results() {
    let doc = fixture();
    let case = &doc["cases"][0];
    let tiles = tiles(case);
    let grid = StaticGrid {
        layout: layout(case),
        cell_size_bits: case["cell_size_bits"].as_u64().unwrap() as u32,
        tiles: &tiles,
    };
    let base = params(case);
    let p = Precision::Double53;
    let mut zero_width = base;
    zero_width.max_rectangle_width = 0;
    assert!(matches!(
        build(&grid, &zero_width, p),
        Err(Error::DivideFault(_))
    ));
    let mut zero_div = base;
    zero_div.fast_square_reduction_div = 0;
    assert!(matches!(
        build(&grid, &zero_div, p),
        Err(Error::DivideFault(_))
    ));
    let short = StaticGrid {
        tiles: &tiles[1..],
        ..grid
    };
    assert!(matches!(build(&short, &base, p), Err(Error::Input(_))));
    let mut bad = grid;
    bad.layout.extent_bits[0] = 3f32.to_bits();
    assert!(matches!(build(&bad, &base, p), Err(Error::Input(_))));
    // Inline terrain type (7A9AC0): bit 9 selects bit 8, bit 8 alone is 2.
    let t = |flags20, definition_type| {
        terrain_type(&StaticTile {
            flags20,
            definition_type,
        })
    };
    assert_eq!(
        [t(0x200, 5), t(0x300, 5), t(0x100, 5), t(0, 5)],
        [0, 1, 2, 5]
    );
}
