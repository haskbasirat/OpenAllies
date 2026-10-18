//! Static original water reconstruction. Native saved/regular geometry, raw
//! heightfields, texture UV and authored depth curves are bound explicitly.
//! Runtime geometry rebuild, texture flip time and scrolling remain separate.
use crate::gpu::{CpuDraw, CpuMaterial, Instance, Vertex};
use aa_assets::PreparedScene;
use aa_numeric::{Binary, Precision};
use aa_resources::{MemoryEstimate, MemoryPolicy, count_bytes, preflight};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::collections::BTreeMap;

const REGULAR_INDICES: [u32; 6] = [0, 3, 1, 2, 1, 3];
fn regular_positions(cell: [usize; 2], cell_size: f32) -> [[f32; 2]; 4] {
    let [x, y] = cell.map(|c| c as f32 * cell_size);
    [
        [x, y],
        [x, y + cell_size],
        [x + cell_size, y + cell_size],
        [x + cell_size, y],
    ]
}

/// Original 4B6E10, with explicit caller precision. Intermediate interpolation
/// weights remain extended; only the four output channels store binary32.
fn depth_color(points: &[[f32; 5]], depth: f32, precision: Precision) -> Result<[f32; 4]> {
    ensure!(
        !points.is_empty()
            && depth.is_finite()
            && points.iter().flatten().all(|n| n.is_finite())
            && points.windows(2).all(|p| p[0][0] <= p[1][0]),
        "Invalid native water depth curve"
    );
    let rgba = |p: &[f32; 5]| [p[1], p[2], p[3], p[4]];
    if depth < points[0][0] {
        return Ok(rgba(&points[0]));
    }
    let last = points.last().context("Empty depth curve")?;
    if depth >= last[0] {
        return Ok(rgba(last));
    }
    let index = points
        .iter()
        .position(|p| depth < p[0])
        .context("Water interval")?;
    let previous = &points[index - 1];
    let current = &points[index];
    let b = |n: f32| Binary::from_f32_bits(n.to_bits());
    let fraction = b(depth)?
        .sub(&b(previous[0])?, precision)?
        .div(&b(current[0])?.sub(&b(previous[0])?, precision)?, precision)?;
    let inverse = Binary::from_u32(1).sub(&fraction, precision)?;
    let mut result = [0.0; 4];
    for (channel, value) in result.iter_mut().enumerate() {
        *value = f32::from_bits(
            inverse
                .mul(&b(previous[channel + 1])?, precision)?
                .add(
                    &fraction.mul(&b(current[channel + 1])?, precision)?,
                    precision,
                )?
                .store_f32()?,
        );
    }
    Ok(result)
}

#[derive(Deserialize)]
struct WaterTerrain {
    first_frame: String,
}
#[derive(Deserialize)]
struct Metadata {
    schema: u32,
    block_size: u32,
    terrains: BTreeMap<String, WaterTerrain>,
    variants: BTreeMap<String, String>,
    biomes: BTreeMap<String, BTreeMap<String, Vec<[f32; 5]>>>,
}

#[path = "water_geometry.rs"]
mod water_geometry;
#[path = "water_surface.rs"]
mod water_surface;

//738320 first uses the cell's own water terrain, then the native clockwise ring.
fn selected_water_cell(
    cell: [usize; 2],
    size: [usize; 2],
    mut water: impl FnMut([usize; 2]) -> bool,
) -> Option<[usize; 2]> {
    if water(cell) {
        return Some(cell);
    }
    for [dx, dy] in [
        [-1, -1],
        [-1, 0],
        [-1, 1],
        [0, 1],
        [1, 1],
        [1, 0],
        [1, -1],
        [0, -1],
    ] {
        let x = cell[0] as i64 + dx;
        let y = cell[1] as i64 + dy;
        if x >= 0 && y >= 0 && x < size[0] as i64 && y < size[1] as i64 {
            let neighbor = [x as usize, y as usize];
            if water(neighbor) {
                return Some(neighbor);
            }
        }
    }
    None
}

pub fn build(
    store: &aa_assets::AssetStore,
    scene: &PreparedScene,
    policy: MemoryPolicy,
) -> Result<(Vec<CpuDraw>, Vec<String>)> {
    let meta: Metadata = serde_json::from_str(include_str!(
        "../../../research/phase6-water/water-metadata.json"
    ))?;
    ensure!(
        meta.schema == 1 && meta.block_size > 0,
        "Water metadata schema"
    );
    let map = &scene.map.terrain;
    let source = store.resolve_url(&format!("/assets/original/Data/{}", map.source))?;
    let source_bytes = std::fs::metadata(&source)?.len();
    let estimate = count_bytes(map.tiles.len() as u64, 512)?
        .checked_add(count_bytes(source_bytes, 12)?)
        .context("Water memory estimate overflow")?;
    let assessment = preflight(
        MemoryEstimate {
            temporary_cpu_bytes: estimate,
            ..Default::default()
        },
        policy,
    )?;
    let archive = aa_mapworld::MapArchive::from_bytes(std::fs::read(source)?)?;
    let budget = assessment.effective_cpu_budget_bytes / 12;
    let terrain =
        aa_mapworld::terrain::TerrainChunk::decode(&archive.decode_chunk("TERRAIN", budget)?)?;
    ensure!(
        terrain.tiles.len() == map.tiles.len(),
        "Original terrain grid differs from preview map"
    );
    let geometry =
        water_geometry::decode_suffix(&terrain.pending_auxiliary_bytes, map.width, map.height)?;
    let surface = water_surface::Surface::load(store, &archive, map, &terrain, budget)?;
    let terrain_at = |x: usize, y: usize| -> Option<&str> {
        meta.variants
            .get(
                surface
                    .effective_variations
                    .get(y * map.width as usize + x)?,
            )
            .map(String::as_str)
    };
    let source_cells = surface
        .effective_variations
        .iter()
        .filter(|id| meta.variants.contains_key(*id))
        .count();
    if source_cells == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    preflight(
        MemoryEstimate {
            temporary_cpu_bytes: estimate
                .checked_add(count_bytes(
                    geometry.indices.len() as u64,
                    std::mem::size_of::<Vertex>() as u64,
                )?)
                .context("Water geometry extent overflow")?,
            ..Default::default()
        },
        policy,
    )?;
    let block_size = meta.block_size as f32;
    let cells_per_block = block_size / map.cell_size;
    ensure!(
        cells_per_block >= 1.0 && cells_per_block.fract() == 0.0,
        "Water block/cell layout is unsupported"
    );
    let cells_per_block = cells_per_block as usize;
    let mut batches = BTreeMap::<(usize, usize, String), CpuDraw>::new();
    //Shared corners get identical native sampling and avoid repeating expensive
    //extended arithmetic. Material-specific WaterInfo colors stay per vertex.
    let mut samples = BTreeMap::<[u32; 2], (f32, f32)>::new();
    let mut included = 0;
    let mut unresolved = 0;
    let mut cached = 0;
    let mut empty = 0;
    for y in 0..map.height as usize {
        for x in 0..map.width as usize {
            let Some([tx, ty]) = selected_water_cell(
                [x, y],
                [map.width as usize, map.height as usize],
                |[nx, ny]| terrain_at(nx, ny).is_some(),
            ) else {
                continue;
            };
            let terrain_id = terrain_at(tx, ty).context("Selected water definition disappeared")?;
            let Some(terrain) = meta.terrains.get(terrain_id) else {
                unresolved += 1;
                continue;
            };
            let matching: Vec<_> = std::iter::once(&surface.biome)
                .filter_map(|biome| meta.biomes.get(biome)?.get(terrain_id))
                .collect();
            let Some(curve) = matching.first() else {
                unresolved += 1;
                continue;
            };
            if matching.iter().any(|other| *other != *curve) {
                unresolved += 1;
                continue;
            }
            let (points, indices) = if let Some(range) = geometry.cells.get(&[x as u32, y as u32]) {
                cached += 1;
                if range.is_empty() {
                    empty += 1;
                    continue;
                }
                let points = geometry.indices[range.clone()]
                    .iter()
                    .map(|&i| geometry.vertices[i as usize])
                    .collect::<Vec<_>>();
                let indices = (0..u32::try_from(points.len())?).collect::<Vec<_>>();
                (points, indices)
            } else {
                (
                    regular_positions([x, y], map.cell_size).to_vec(),
                    REGULAR_INDICES.to_vec(),
                )
            };
            let mut vertices = Vec::with_capacity(points.len());
            for [wx, wy] in points.iter().copied() {
                let key = [wx.to_bits(), wy.to_bits()];
                let (height, depth) = if let Some(&value) = samples.get(&key) {
                    value
                } else {
                    let Ok(value) = surface.sample([wx, wy], Precision::Single24) else {
                        break;
                    };
                    samples.insert(key, value);
                    value
                };
                vertices.push(Vertex {
                    position: aa_assets::native_to_render([wx, wy, height]),
                    normal: [0.0, 1.0, 0.0],
                    uv: [
                        (wx - (x / cells_per_block) as f32 * block_size) / block_size,
                        (wy - (y / cells_per_block) as f32 * block_size) / block_size,
                    ],
                    color: depth_color(curve, depth, Precision::Single24)?,
                    terrain_uv: [0.0; 2],
                });
            }
            if vertices.len() != points.len() {
                unresolved += 1;
                continue;
            }
            let key = (
                y / cells_per_block,
                x / cells_per_block,
                terrain_id.to_owned(),
            );
            let draw = batches.entry(key).or_insert_with(|| CpuDraw {
                vertices: Vec::new(),
                indices: Vec::new(),
                instances: vec![Instance::identity()],
                source_order: None,
                sort_center: None,
                material: CpuMaterial {
                    texture: Some(terrain.first_frame.clone()),
                    color: [1.0; 4],
                    emissive: [0.0; 3],
                    cutoff: 0.0,
                    blend: true,
                    double_sided: false,
                    team_color_mask: false,
                    terrain_layer: false,
                    terrain_mask: None,
                    native_render_state: Some(aa_assets::NativeRenderState {
                        alpha_flags: 0xed,
                        alpha_threshold: 0,
                        zbuffer_flags: 1,
                        zbuffer_function: 3,
                    }),
                },
            });
            let start = u32::try_from(draw.vertices.len())?;
            draw.indices.extend(indices.into_iter().map(|i| start + i));
            draw.vertices.extend(vertices);
            included += 1;
        }
    }
    Ok((
        batches.into_values().collect(),
        vec![format!(
            "Static native water: {included} selected water/shore cells from {source_cells} authored water cells; {cached} saved-cache cells ({empty} empty), {unresolved} unresolved cells skipped. Original TERRAIN cache offset {} and raw HEIGHT_MAP/MOUNTAIN_HEIGHT_MAP are bound; {} missing original variation cells retain native previous/default terrain. Native regular/cached geometry, first frame, block UV, surface offset, depth RGB/alpha and blend/depth state are used. Runtime geometry-cache rebuild, flip animation/scroll, live FPU state, native sphere bounds and lighting/gamma remain unfinished; preview arithmetic is explicitly binary32.",
            terrain.suffix_offset + geometry.cache_offset,
            surface.carried_cells
        )],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Deserialize)]
    struct Vectors {
        curve: Vec<Case>,
        regular_quads: Vec<Quad>,
        selection: Vec<Selection>,
    }
    #[derive(Deserialize)]
    struct Selection {
        mask: u32,
        cell: [usize; 2],
        selected: Option<usize>,
    }
    #[derive(Deserialize)]
    struct Quad {
        cell: [usize; 2],
        cell_size_bits: u32,
        vertices_bits: [[u32; 2]; 4],
        indices: [u32; 6],
    }
    #[derive(Deserialize)]
    struct Case {
        precision: Precision,
        points_bits: Vec<[u32; 5]>,
        depth_bits: u32,
        rgba_bits: [u32; 4],
    }
    #[test]
    fn original_water_depth_interpolation_all_precisions() {
        let vectors: Vectors = serde_json::from_str(include_str!(
            "../../../research/phase6-water/native-water-vectors.json"
        ))
        .unwrap();
        for case in vectors.curve {
            let points: Vec<_> = case
                .points_bits
                .iter()
                .map(|p| p.map(f32::from_bits))
                .collect();
            let actual =
                depth_color(&points, f32::from_bits(case.depth_bits), case.precision).unwrap();
            assert_eq!(actual.map(f32::to_bits), case.rgba_bits);
        }
        assert!(depth_color(&[], 0.0, Precision::Single24).is_err());
        assert!(depth_color(&[[0.0; 5]], f32::NAN, Precision::Single24).is_err());
    }
    #[test]
    fn original_water_regular_cell_topology() {
        let vectors: Vectors = serde_json::from_str(include_str!(
            "../../../research/phase6-water/native-water-vectors.json"
        ))
        .unwrap();
        for case in vectors.regular_quads {
            assert_eq!(
                regular_positions(case.cell, f32::from_bits(case.cell_size_bits))
                    .map(|p| p.map(f32::to_bits)),
                case.vertices_bits
            );
            assert_eq!(REGULAR_INDICES, case.indices);
        }
    }
    #[test]
    fn original_water_current_cell_and_neighbor_selection() {
        let vectors: Vectors = serde_json::from_str(include_str!(
            "../../../research/phase6-water/native-water-vectors.json"
        ))
        .unwrap();
        for case in vectors.selection {
            let got = selected_water_cell(case.cell, [3, 3], |[x, y]| {
                case.mask & (1 << (y * 3 + x)) != 0
            });
            assert_eq!(got.map(|[x, y]| y * 3 + x), case.selected);
        }
    }
    #[test]
    #[ignore = "requires original recovered assets; run explicitly"]
    fn original_crete_water_pass_preserves_source_height_and_depth_state() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../reconstruction");
        let store = aa_assets::AssetStore::open(root).unwrap();
        let scene = store
            .load_scene(
                "Campaign/axis01_crete",
                aa_assets::HeightMode::NativeConditional,
            )
            .unwrap();
        let (draws, warnings) = build(&store, &scene, MemoryPolicy::default()).unwrap();
        assert!(!draws.is_empty());
        assert_eq!(
            draws.iter().map(|d| d.indices.len() / 6).sum::<usize>(),
            14_169
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("0 unresolved cells skipped"))
        );
        for draw in draws {
            let state = draw.material.native_render_state.unwrap();
            assert_eq!(
                (
                    state.alpha_flags,
                    state.zbuffer_flags,
                    state.zbuffer_function
                ),
                (0xed, 1, 3)
            );
            assert!(
                draw.material
                    .texture
                    .as_ref()
                    .unwrap()
                    .contains("WaterFrames0000")
            );
            for v in draw.vertices {
                let x = (v.position[0] / scene.map.terrain.cell_size) as usize;
                let y = (-v.position[2] / scene.map.terrain.cell_size) as usize;
                let i = y * (scene.map.terrain.width as usize + 1) + x;
                // Original raw float height preserves more digits than the
                // independent recovered JSON. Shore vertices may lie below
                // terrain: the native depth test hides these portions.
                assert!((v.position[1] - (scene.map.terrain.heights[i] - 0.5)).abs() < 0.00001);
                assert!(v.color.iter().all(|v| (0.0..=1.0).contains(v)));
                assert!(v.uv.iter().all(|v| (0.0..=1.0).contains(v)));
            }
        }
    }
}
