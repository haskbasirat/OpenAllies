use aa_assets::*;
use std::collections::BTreeMap;

fn terrain() -> TerrainMap {
    TerrainMap {
        id: "Campaign/shared".into(),
        name: "Test".into(),
        description: String::new(),
        source: "Maps/Campaign/shared.RMP".into(),
        width: 2,
        height: 1,
        cell_size: 2.0,
        biomes: vec![],
        tiles: vec![0, 1],
        heights: vec![0.0, 2.0, 4.0, 0.0, 2.0, 4.0],
        mountain_heights: vec![10.0; 6],
        palette: BTreeMap::from([
            (
                0,
                TerrainPaletteEntry {
                    id: "a".into(),
                    texture: None,
                    color: [1.0, 0.0, 0.0],
                    width: Some(8.0),
                    height: Some(8.0),
                },
            ),
            (
                1,
                TerrainPaletteEntry {
                    id: "b".into(),
                    texture: None,
                    color: [0.0, 1.0, 0.0],
                    width: Some(8.0),
                    height: Some(8.0),
                },
            ),
        ]),
        limitations: vec![],
    }
}
fn mesh() -> RawMesh {
    RawMesh {
        name: "triangle".into(),
        hierarchy: vec![],
        positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0],
        normals: vec![],
        uvs: vec![],
        indices: vec![0, 1, 2],
        material: Material::default(),
        skinned_pose: false,
    }
}
#[test]
fn coordinates_winding_heights_and_normals() {
    assert_eq!(native_to_render([3.0, 4.0, 5.0]), [3.0, 5.0, -4.0]);
    let t = terrain();
    let p = t.prepare(HeightMode::BaseOnly).unwrap();
    assert_eq!(p.vertices[2].position, [0.0, 0.0, -2.0]);
    assert_eq!(p.batches[0].indices, [0, 1, 2, 1, 3, 2]);
    let n = p.vertices[0].normal;
    assert!((n[0] + std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    assert!((n[1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    assert_eq!(
        t.height_at_native(1.0, 1.0, HeightMode::BaseOnly),
        Some(1.0)
    );
    assert_eq!(
        t.height_at_native(1.0, 1.0, HeightMode::BasePlusMountain),
        Some(11.0)
    );
    assert_eq!(t.height_at_native(-1.0, 0.0, HeightMode::BaseOnly), None);
}
#[test]
fn invalid_terrain_rejected() {
    let mut t = terrain();
    t.heights.pop();
    assert!(t.prepare(HeightMode::BaseOnly).is_err());
    let mut t = terrain();
    t.tiles[0] = 999;
    assert!(t.validate().is_err());
    let mut t = terrain();
    t.heights[0] = f32::NAN;
    assert!(t.validate().is_err());
}

#[test]
fn textured_terrain_uses_native_v_direction_and_no_minimap_tint() {
    let mut t = terrain();
    t.palette.get_mut(&0).unwrap().texture = Some("/assets/ground.png".into());
    let p = t.prepare(HeightMode::BaseOnly).unwrap();
    assert_eq!(p.vertices[0].uv, [0.0, 1.0]);
    assert_eq!(p.vertices[1].uv, [0.25, 1.0]);
    assert_eq!(p.vertices[2].uv, [0.0, 0.75]);
    assert_eq!(p.vertices[3].uv, [0.25, 0.75]);
    assert!(p.vertices[..4].iter().all(|v| v.color == [1.0; 4]));
    assert!(p.vertices[4..].iter().all(|v| v.color == [0.0, 1.0, 0.0, 1.0]));
    // Texture repeat coordinates stay continuous across a shared tile edge,
    // including different batches that use the same authored repeat sizes.
    assert_eq!(p.vertices[1].uv, p.vertices[4].uv);
    assert_eq!(p.vertices[3].uv, p.vertices[6].uv);
}
#[test]
fn model_basis_uv_and_normal_preserved() {
    let m = mesh();
    let (p, warnings) = m.prepare().unwrap();
    assert_eq!(p.vertices[2].position, [0.0, 0.0, -1.0]);
    assert_eq!(p.vertices[0].normal, [0.0, 1.0, 0.0]);
    assert_eq!(warnings.len(), 2);
    let mut m = mesh();
    m.uvs = vec![0.0, 0.25, 1.0, 0.0, 0.0, 1.0];
    m.normals = vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0];
    assert_eq!(m.prepare().unwrap().0.vertices[0].uv, [0.0, 0.25]);
    m.indices[0] = 3;
    assert!(m.prepare().is_err());
}
#[test]
fn deterministic_parallel_terrain() {
    let t = terrain();
    let run = |count| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(count)
            .build()
            .unwrap()
            .install(|| t.prepare(HeightMode::BasePlusMountain).unwrap())
    };
    assert_eq!(run(1), run(4));
}

#[test]
fn conditional_height_matches_native_executed_fixtures() {
    // Matches engine-baseline/test_native_height.py, which executes actual
    // 0x4F0E50 + HeightMap samplers. Only the reviewed render0x122 subset.
    let mut t = terrain();
    t.width = 4;
    t.height = 4;
    t.tiles = vec![0; 16];
    t.heights = vec![10.0; 25];
    t.mountain_heights = vec![3.0; 25];
    for (key, name) in [(0, "land"), (1, "mountain"), (2, "water")] {
        t.palette.insert(
            key,
            TerrainPaletteEntry {
                id: name.into(),
                texture: None,
                color: [1.0; 3],
                width: None,
                height: None,
            },
        );
    }
    let mut meta = TerrainMetadata {
        schema: "aa-native-terrain-variation-evidence-v1".into(),
        variations: BTreeMap::new(),
    };
    for (native_type, name) in [(0, "land"), (1, "mountain"), (2, "water")] {
        meta.variations.insert(
            name.into(),
            TerrainVariation {
                terrain_id: name.into(),
                native_type,
                noise_min: Some(0.25),
                noise_max: Some(0.75),
                source: "fixture".into(),
                source_sha256: String::new(),
            },
        );
    }
    assert_eq!(t.native_height_at_grid(1, 1, &meta), Some((10.0, true)));
    t.tiles[5] = 2;
    assert_eq!(t.native_height_at_grid(1, 1, &meta), Some((13.0, true)));
    t.tiles[5] = 1;
    assert_eq!(
        t.native_height_at_grid(1, 1, &meta),
        Some((f32::from_bits(0x415b_999a), true))
    );
    let prepared = t
        .prepare_with_metadata(HeightMode::NativeConditional, Some(&meta))
        .unwrap();
    assert_eq!(
        prepared.vertices[5 * 4].position,
        [2.0, f32::from_bits(0x415b_999a), -2.0]
    );
    for neighbor in [0, 1, 2, 4, 6, 8, 9, 10] {
        t.tiles.fill(0);
        t.tiles[neighbor] = 1;
        assert_eq!(t.native_height_at_grid(1, 1, &meta), Some((13.0, true)));
    }
    t.tiles.fill(0);
    t.tiles[15] = 1;
    assert_eq!(t.native_height_at_grid(1, 1, &meta), Some((10.0, true)));
    t.tiles[15] = 2;
    assert_eq!(t.native_height_at_grid(4, 4, &meta), Some((13.0, true))); // edge clamps currenttile
    meta.variations.remove("water");
    assert_eq!(t.native_height_at_grid(4, 4, &meta), Some((10.0, false)));
    assert!(
        t.prepare_with_metadata(HeightMode::NativeConditional, Some(&meta))
            .unwrap()
            .warnings
            .iter()
            .any(|w| w.message.contains("base-only fallback"))
    );
}

#[test]
fn dimension_overflow_is_reported_without_panicking() {
    let mut t = terrain();
    t.width = u32::MAX;
    t.height = u32::MAX;
    assert!(t.validate().is_err());
}

#[test]
#[ignore = "requires recovered assets; set AA_ASSET_ROOT and run --ignored"]
fn recovered_assets_and_path_safety() {
    let store =
        AssetStore::open(std::env::var("AA_ASSET_ROOT").expect("AA_ASSET_ROOT is required"))
            .unwrap();
    assert_eq!(store.maps().len(), 80);
    let names: std::collections::BTreeSet<_> = store.maps().iter().map(|m| &m.id).collect();
    assert_eq!(names.len(), 80);
    let map = store.load_map("Campaign/tutorial01").unwrap();
    assert_eq!(map.placements.actors.len(), 2732);
    for bad in [
        "../Data.rwd",
        "Campaign/../tutorial01",
        "C:/Windows/system.ini",
        "/absolute",
        "Campaign//tutorial01",
    ] {
        assert!(store.load_map(bad).is_err());
    }
    assert!(store.resolve_url("/assets/../Data.rwd").is_err());
    assert!(store.resolve_url("https://example.com/a.png").is_err());
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap()
        .install(|| store.load_scene("Campaign/tutorial01", HeightMode::NativeConditional))
        .unwrap();
    let four = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| store.load_scene("Campaign/tutorial01", HeightMode::NativeConditional))
        .unwrap();
    assert_eq!(one, four);
    assert!(
        one.instances
            .iter()
            .filter(|i| i.model_key.is_some())
            .count()
            > 2000
    );
    assert!(
        !one.warnings
            .iter()
            .any(|w| w.message.contains("base-only fallback")
                || w.message.contains("Unresolved terrain variation"))
    );
}
