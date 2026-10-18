//! Native terrain texture pass decisions, using the original transition masks.
//! Layers are evaluated directly on the ground geometry. Native cached-target
//! quantization/filtering and final terrain lighting remain separate work.
use crate::gpu::{CpuDraw, CpuMaterial, Instance, Vertex};
use aa_assets::{PreparedScene, TerrainLayerInput, TerrainLayerPlanner, TerrainTextureLayer};
use aa_resources::{MemoryEstimate, MemoryPolicy, count_bytes, preflight};
use anyhow::{Context, Result, ensure};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct TerrainMaskMaterial {
    pub texture: String,
    pub rows: [[f32; 4]; 2],
    pub cell_size: f32,
}

#[derive(Deserialize)]
struct Variation {
    canonical: String,
    priority: i32,
    texture: String,
    width: f32,
    height: f32,
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
struct Metadata {
    schema: u32,
    variants: BTreeMap<String, Variation>,
    biomes: BTreeMap<String, Vec<String>>,
    transition_sets: BTreeMap<String, String>,
    texels_per_meter: u32,
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct BatchKey {
    priority: i32,
    overlay: bool,
    palette: u32,
    mask: u8,
    transform: u32,
    transition_set: usize,
}

pub fn build(
    scene: &PreparedScene,
    memory_policy: MemoryPolicy,
) -> Result<(Vec<CpuDraw>, Vec<String>)> {
    build_parts(&scene.map.terrain,&scene.terrain,memory_policy)
}

///The actual battle backend supplies current World samples. Only immutable
///rendering material metadata is shared with the inspection path.
pub(crate) fn build_parts(
    map:&aa_assets::TerrainMap,
    terrain:&aa_assets::PreparedTerrain,
    memory_policy:MemoryPolicy,
)->Result<(Vec<CpuDraw>,Vec<String>)>{
    let metadata: Metadata = serde_json::from_str(include_str!(
        "../../../research/phase6-terrain/terrain-render-metadata.json"
    ))?;
    anyhow::ensure!(metadata.schema == 1, "Terrain metadata schema mismatch");
    let planner = TerrainLayerPlanner::bundled()?;
    // Native translation aliases resolve to the same variation pointer. Keep
    // a single palette representative instead of inventing equal-priority ties.
    let mut canonical = BTreeMap::new();
    let mut inputs = BTreeMap::new();
    for (&id, palette) in &map.palette {
        if let Some(v) = metadata.variants.get(&palette.id) {
            let representative = *canonical.entry(v.canonical.clone()).or_insert(id);
            inputs.insert(
                id,
                TerrainLayerInput {
                    palette_index: representative,
                    priority: v.priority,
                },
            );
        }
    }
    // All shipped biomes currently use the same single authored Global01 set.
    // Preserve ordered selections when one biome is present. Multiple biomes
    // must agree, otherwise no global-biome choice is guessed.
    let set_names = map.biomes.first().and_then(|b| metadata.biomes.get(b));
    let sets_agree = set_names.is_some_and(|sets| {
        !sets.is_empty()
            && map
                .biomes
                .iter()
                .all(|b| metadata.biomes.get(b) == Some(sets))
    });
    let prefixes: Vec<_> = if sets_agree {
        set_names
            .unwrap()
            .iter()
            .map(|name| {
                metadata
                    .transition_sets
                    .get(name)
                    .context("Missing TransitionSet")
            })
            .collect::<Result<_>>()?
    } else {
        Vec::new()
    };
    let planned: Vec<_> = map
        .tiles
        .par_iter()
        .enumerate()
        .map(|(index, &id)| {
            let x = index % map.width as usize;
            let y = index / map.width as usize;
            // Border expansion remains a distinct native subsystem. Keep the old
            // outermost cells instead of guessing ghost-cell material identities.
            if prefixes.is_empty()
                || x == 0
                || y == 0
                || x + 1 == map.width as usize
                || y + 1 == map.height as usize
            {
                return None;
            }
            let mut grid = [*inputs.get(&id)?; 9];
            for (i, entry) in grid.iter_mut().enumerate() {
                let neighbor = map.tiles[(y + i / 3 - 1) * map.width as usize + x + i % 3 - 1];
                *entry = *inputs.get(&neighbor)?;
            }
            planner.plan(grid).ok()
        })
        .collect();
    let layer_count: usize = planned
        .iter()
        .map(|layers| layers.as_ref().map_or(1, Vec::len))
        .sum();
    preflight(
        MemoryEstimate {
            temporary_cpu_bytes: count_bytes(
                layer_count as u64,
                (4 * std::mem::size_of::<Vertex>() + 6 * 4 + 96) as u64,
            )?,
            ..Default::default()
        },
        memory_policy,
    )?;
    let mut batches = BTreeMap::<BatchKey, CpuDraw>::new();
    let mut fallback = 0;
    let mut blended = 0;
    for (index, plan) in planned.into_iter().enumerate() {
        let layers = plan.unwrap_or_else(|| {
            fallback += 1;
            vec![TerrainTextureLayer {
                palette_index: map.tiles[index],
                priority: i32::MIN,
                neighbor_mask: 0,
                transition: None,
            }]
        });
        blended += usize::from(layers.len() > 1);
        let x = (index % map.width as usize) as i32;
        let y = (index / map.width as usize) as i32;
        let set_index = if prefixes.is_empty() {
            0
        } else {
            aa_assets::terrain_transition_set_index([x, y], map.width as i32, prefixes.len())?
        };
        for layer in layers {
            let palette = &map.palette[&layer.palette_index];
            let variation = metadata.variants.get(&palette.id);
            let texture = variation
                .map(|v| v.texture.clone())
                .or_else(|| palette.texture.clone());
            let dims = variation
                .map(|v| [v.width, v.height])
                .unwrap_or([palette.width.unwrap_or(8.0), palette.height.unwrap_or(8.0)]);
            let mask = layer
                .transition
                .as_ref()
                .map(|m| -> Result<_> {
                    let prefix = prefixes[set_index];
                    let original = format!("{prefix}{}", m.filename_suffix());
                    let texture = format!(
                        "/assets/converted/textures/{}",
                        original
                            .strip_suffix(".tga")
                            .context("Transition suffix")?
                            .to_owned()
                            + ".png"
                    );
                    Ok(TerrainMaskMaterial {
                        texture,
                        rows: m.compositor_rows(
                            (metadata.texels_per_meter as f32 * map.cell_size) as u32,
                        )?,
                        cell_size: map.cell_size,
                    })
                })
                .transpose()?;
            let key = BatchKey {
                priority: layer.priority,
                overlay: mask.is_some(),
                palette: layer.palette_index,
                mask: layer.transition.as_ref().map_or(0, |m| m.base_mask),
                transform: layer.transition.as_ref().map_or(0, |m| m.transform),
                transition_set: set_index,
            };
            let draw = batches.entry(key).or_insert_with(|| CpuDraw {
                vertices: Vec::new(),
                indices: Vec::new(),
                instances: vec![Instance::identity()],
                source_order: None,
                sort_center: None,
                material: CpuMaterial {
                    texture: texture.clone(),
                    color: [1.0; 4],
                    emissive: [0.0; 3],
                    cutoff: 0.0,
                    blend: mask.is_some(),
                    double_sided: false,
                    team_color_mask: false,
                    native_render_state: Some(aa_assets::NativeRenderState {
                        alpha_flags: 0xec | u16::from(mask.is_some()),
                        alpha_threshold: 0,
                        zbuffer_flags: 3,
                        zbuffer_function: 3,
                    }),
                    terrain_layer: true,
                    terrain_mask: mask,
                },
            });
            let first = u32::try_from(draw.vertices.len())?;
            for (corner, v) in terrain.vertices[index * 4..index * 4 + 4]
                .iter()
                .enumerate()
            {
                draw.vertices.push(Vertex {
                    terrain_uv: [(corner % 2) as f32, 1.0 - (corner / 2) as f32],
                    position: v.position,
                    normal: v.normal,
                    uv: [v.position[0] / dims[0], 1.0 + v.position[2] / dims[1]],
                    color: if texture.is_some() { [1.0; 4] } else { v.color },
                });
            }
            draw.indices
                .extend([first, first + 1, first + 2, first + 1, first + 3, first + 2]);
        }
    }
    let warnings = vec![format!(
        "Original terrain transition masks blend {blended} cells; {fallback} border/unresolved cells retain the single-material fallback. Native offscreen-target quantization/filtering, water and final terrain lighting remain incomplete."
    )];
    Ok((batches.into_values().collect(), warnings))
}

///A converted material is usable by a battle only while its recorded source
///file is in the same live catalog and its canonical VariationData is present.
///A viewer fallback color or unresolved alias never satisfies this check.
pub(crate) fn require_current_materials(map:&aa_assets::TerrainMap,catalog:&aa_runtime::catalog::Catalog<'_>)->Result<()>{
    catalog.require_source_owner().map_err(anyhow::Error::msg)?;
    let metadata:Metadata=serde_json::from_str(include_str!("../../../research/phase6-terrain/terrain-render-metadata.json"))?;
    ensure!(metadata.schema==1,"terrain metadata schema differs");
    for palette in map.palette.values(){
        let v=metadata.variants.get(&palette.id).with_context(||format!("current terrain material {} is unresolved",palette.id))?;
        let entry=catalog.actors.native_data_find("Variation",&v.canonical)
            .with_context(||format!("canonical VariationData {} is absent",v.canonical))?;
        ensure!(entry.source.path.eq_ignore_ascii_case(&v.path)&&entry.source.sha256==v.sha256,
            "terrain material {} source differs from current VariationData",palette.id);
        ensure!(catalog.actors.sources.iter().any(|p|p.path.eq_ignore_ascii_case(&v.path)&&p.sha256==v.sha256),
            "terrain material {} source is absent from current catalog",palette.id);
        ensure!(v.width.is_finite()&&v.width>0.0&&v.height.is_finite()&&v.height>0.0,
            "terrain material dimensions are invalid");
    }
    ensure!(!map.biomes.is_empty(),"current terrain has no retained biome material selection");
    let sets=metadata.biomes.get(&map.biomes[0]).context("current biome transition selection is unavailable")?;
    ensure!(!sets.is_empty()&&sets.iter().all(|s|metadata.transition_sets.contains_key(s))
        &&map.biomes.iter().all(|b|metadata.biomes.get(b)==Some(sets)),
        "current biome transition sets are unresolved or disagree");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_transition_selection_and_mask_texel_center_fixtures() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase6-terrain/compositor-fixtures.json"
        ))
        .unwrap();
        for row in fixture["selector_cases"].as_array().unwrap() {
            let cell = std::array::from_fn(|i| row["cell"][i].as_i64().unwrap() as i32);
            assert_eq!(
                aa_assets::terrain_transition_set_index(
                    cell,
                    row["width"].as_i64().unwrap() as i32,
                    row["count"].as_u64().unwrap() as usize
                )
                .unwrap(),
                row["index"].as_u64().unwrap() as usize
            );
        }
        let masks: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase5-terrain/mask-lookup.json"
        ))
        .unwrap();
        for row in fixture["matrix_cases"].as_array().unwrap() {
            let value = masks["table"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| !m.is_null() && m["transform"] == row["transform"])
                .unwrap();
            let mask: aa_assets::TerrainTransitionMask =
                serde_json::from_value(value.clone()).unwrap();
            let actual = mask
                .compositor_rows(row["pixels"].as_u64().unwrap() as u32)
                .unwrap();
            for (axis, indices) in [[0, 4, 8], [1, 5, 9]].iter().enumerate() {
                for (column, &index) in indices.iter().enumerate() {
                    assert_eq!(
                        f64::from(actual[axis][column]),
                        row["matrix"][index].as_f64().unwrap()
                    );
                }
            }
        }
        let meta: Metadata = serde_json::from_str(include_str!(
            "../../../research/phase6-terrain/terrain-render-metadata.json"
        ))
        .unwrap();
        assert_eq!(meta.variants["chaparral_dirt01"].priority, 5000);
        assert_eq!(meta.variants["chaparral_dirt05"].priority, 5004);
        assert_eq!(meta.biomes["chaparral"], ["transition_set_global_01"]);
        assert_eq!(meta.biomes["arctic_forest"],meta.biomes["boreal_forest"]);
        for row in fixture["render_state_cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["alpha_flags"] == 236)
        {
            let expected = if row["layer"] == 0 {
                serde_json::json!([[27, 0], [137, 0]])
            } else {
                serde_json::json!([[27, 1], [19, 5], [20, 6], [137, 0]])
            };
            assert_eq!(row["states"], expected);
        }
    }

    #[test]
    #[ignore = "requires a native graphics adapter; run explicitly"]
    fn terrain_gpu_mask_orientation_and_source_alpha_composition() {
        use crate::gpu::{CpuScene, CpuTexture, SceneRenderer};
        use eframe::egui;
        use std::sync::Arc;
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .unwrap();
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let egui_renderer = egui_wgpu::Renderer::new(&device, format, Default::default());
        let mut renderer = SceneRenderer::new(egui_wgpu::RenderState {
            adapter,
            available_adapters: Vec::new(),
            device,
            queue,
            target_format: format,
            renderer: Arc::new(egui::mutex::RwLock::new(egui_renderer)),
        })
        .unwrap();
        let camera = crate::camera::Camera::for_model(&aa_assets::Bounds {
            min: [0.0, 0.0, -2.0],
            max: [2.0, 0.0, 0.0],
            size: [2.0, 0.0, 2.0],
        });
        let draw = |color: [f32; 4], mask: Option<TerrainMaskMaterial>| CpuDraw {
            vertices: [
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 0.0, -2.0],
                [2.0, 0.0, -2.0],
            ]
            .map(|position| Vertex {
                terrain_uv: [position[0] / 2.0, 1.0 + position[2] / 2.0],
                position,
                normal: [-0.5, 1.0, 0.35],
                uv: [0.0; 2],
                color,
            })
            .to_vec(),
            indices: vec![0, 1, 2, 1, 3, 2],
            instances: vec![Instance::identity()],
            source_order: None,
            sort_center: None,
            material: CpuMaterial {
                texture: None,
                color: [1.0; 4],
                emissive: [0.0; 3],
                cutoff: 0.0,
                blend: mask.is_some(),
                double_sided: false,
                team_color_mask: false,
                native_render_state: Some(aa_assets::NativeRenderState {
                    alpha_flags: 0xec | u16::from(mask.is_some()),
                    alpha_threshold: 0,
                    zbuffer_flags: 3,
                    zbuffer_function: 3,
                }),
                terrain_layer: true,
                terrain_mask: mask,
            },
        };
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase5-terrain/mask-lookup.json"
        ))
        .unwrap();
        let output = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../qa-output/phase6/terrain-gpu-probes")
            .join(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .to_string(),
            );
        std::fs::create_dir_all(&output).unwrap();
        for transform in [0, 1, 2, 4, 8, 9, 10, 12] {
            let record = catalog["table"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| !m.is_null() && m["transform"] == transform)
                .unwrap();
            let mask: aa_assets::TerrainTransitionMask =
                serde_json::from_value(record.clone()).unwrap();
            let rows = mask.compositor_rows(32).unwrap();
            let mut texture = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 255, 255, 255]));
            texture.put_pixel(0, 0, image::Rgba([255, 255, 255, 0]));
            let scene = CpuScene {
                live: None,
                objects: None,
                name: "terrain-mask-probe".into(),
                map_id: "synthetic".into(),
                size: [2.0; 2],
                draws: vec![
                    draw([0.0, 0.0, 1.0, 1.0], None),
                    draw(
                        [1.0, 0.0, 0.0, 1.0],
                        Some(TerrainMaskMaterial {
                            texture: "mask".into(),
                            rows,
                            cell_size: 2.0,
                        }),
                    ),
                ],
                textures: BTreeMap::from([(
                    "mask".into(),
                    CpuTexture {
                        mips: vec![texture],
                    },
                )]),
                placements: 0,
                rendered_instances: 0,
                warnings: Vec::new(),
                load_seconds: 0.0,
            };
            renderer.upload(&scene).unwrap();
            renderer.render(&camera, [256, 256]).unwrap();
            let path = output.join(format!("rotation-{transform}.png"));
            renderer.save_view(&path).unwrap();
            let image = image::open(path).unwrap().to_rgba8();
            for x in [0.5, 1.5] {
                for y in [0.5, 1.5] {
                    let uv = [x / 2.0, 1.0 - y / 2.0, 1.0];
                    let transformed =
                        rows.map(|row| (row[0] * uv[0] + row[1] * uv[1] + row[2]).rem_euclid(1.0));
                    let alpha = !(transformed[0] < 0.5 && transformed[1] < 0.5);
                    let clip = camera.view_projection(1.0) * glam::Vec4::new(x, 0.0, -y, 1.0);
                    let p = clip / clip.w;
                    let pixel =
                        image.get_pixel(((p.x + 1.0) * 128.0) as u32, ((1.0 - p.y) * 128.0) as u32);
                    assert!(
                        if alpha {
                            pixel[0] > 245 && pixel[2] < 5
                        } else {
                            pixel[2] > 245 && pixel[0] < 5
                        },
                        "transform{transform} at{x},{y}: {pixel:?}"
                    );
                }
            }
        }
        // Mid-alpha must mix both colors, not merely cut out or replace a cell.
        let scene = CpuScene {
                live: None,
            objects: None,
            name: "terrain-alpha-probe".into(),
            map_id: "synthetic".into(),
            size: [2.0; 2],
            draws: vec![
                draw([0.0, 0.0, 1.0, 1.0], None),
                draw(
                    [1.0, 0.0, 0.0, 1.0],
                    Some(TerrainMaskMaterial {
                        texture: "mask".into(),
                        rows: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]],
                        cell_size: 2.0,
                    }),
                ),
            ],
            textures: BTreeMap::from([(
                "mask".into(),
                CpuTexture {
                    mips: vec![image::RgbaImage::from_pixel(
                        1,
                        1,
                        image::Rgba([255, 255, 255, 128]),
                    )],
                },
            )]),
            placements: 0,
            rendered_instances: 0,
            warnings: Vec::new(),
            load_seconds: 0.0,
        };
        renderer.upload(&scene).unwrap();
        renderer.render(&camera, [256, 256]).unwrap();
        let path = output.join("mid-alpha.png");
        renderer.save_view(&path).unwrap();
        let image = image::open(path).unwrap().to_rgba8();
        let pixel = image.get_pixel(128, 128);
        // Scene target is currently sRGB: linear half intensity encodes near188.
        assert!(
            (i32::from(pixel[0]) - 188).abs() <= 2
                && pixel[1] < 3
                && (i32::from(pixel[2]) - 187).abs() <= 2
                && pixel[3] == 255,
            "mid-alpha {pixel:?}"
        );
    }
}
