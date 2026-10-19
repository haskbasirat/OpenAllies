//! Terrain layers for Bevy. Same layer planning as the wgpu client's
//! `terrain_render::build_parts` (original transition masks, native layer
//! priorities); only the output is shaped for Bevy meshes.
use aa_assets::{PreparedTerrain, TerrainLayerInput, TerrainLayerPlanner, TerrainMap, TerrainTextureLayer};
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Variation {
    canonical: String,
    priority: i32,
    texture: String,
    width: f32,
    height: f32,
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

pub struct Mask {
    pub texture: String,
    pub rows: [[f32; 4]; 2],
}
/// One terrain layer batch, drawn in list order (later layers on top).
pub struct Draw {
    pub texture: Option<String>,
    pub color: [f32; 4],
    pub mask: Option<Mask>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub cell_uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

pub fn build(map: &TerrainMap, terrain: &PreparedTerrain) -> Result<Vec<Draw>> {
    let metadata: Metadata = serde_json::from_str(include_str!(
        "../../../research/phase6-terrain/terrain-render-metadata.json"
    ))?;
    anyhow::ensure!(metadata.schema == 1, "terrain metadata schema mismatch");
    let planner = TerrainLayerPlanner::bundled()?;
    let mut canonical = BTreeMap::new();
    let mut inputs = BTreeMap::new();
    for (&id, palette) in &map.palette {
        if let Some(v) = metadata.variants.get(&palette.id) {
            let representative = *canonical.entry(v.canonical.clone()).or_insert(id);
            inputs.insert(id, TerrainLayerInput { palette_index: representative, priority: v.priority });
        }
    }
    let set_names = map.biomes.first().and_then(|b| metadata.biomes.get(b));
    let sets_agree = set_names.is_some_and(|sets| {
        !sets.is_empty() && map.biomes.iter().all(|b| metadata.biomes.get(b) == Some(sets))
    });
    let prefixes: Vec<&String> = if sets_agree {
        set_names
            .unwrap()
            .iter()
            .map(|name| metadata.transition_sets.get(name).context("missing TransitionSet"))
            .collect::<Result<_>>()?
    } else {
        Vec::new()
    };
    let width = map.width as usize;
    let planned: Vec<Option<Vec<TerrainTextureLayer>>> = map
        .tiles
        .par_iter()
        .enumerate()
        .map(|(index, &id)| {
            let (x, y) = (index % width, index / width);
            if prefixes.is_empty() || x == 0 || y == 0 || x + 1 == width || y + 1 == map.height as usize {
                return None;
            }
            let mut grid = [*inputs.get(&id)?; 9];
            for (i, entry) in grid.iter_mut().enumerate() {
                let neighbor = map.tiles[(y + i / 3 - 1) * width + x + i % 3 - 1];
                *entry = *inputs.get(&neighbor)?;
            }
            planner.plan(grid).ok()
        })
        .collect();
    let mut batches = BTreeMap::<BatchKey, Draw>::new();
    for (index, plan) in planned.into_iter().enumerate() {
        let layers = plan.unwrap_or_else(|| {
            vec![TerrainTextureLayer {
                palette_index: map.tiles[index],
                priority: i32::MIN,
                neighbor_mask: 0,
                transition: None,
            }]
        });
        let (x, y) = ((index % width) as i32, (index / width) as i32);
        let set_index = if prefixes.is_empty() {
            0
        } else {
            aa_assets::terrain_transition_set_index([x, y], map.width as i32, prefixes.len())?
        };
        for layer in layers {
            let palette = &map.palette[&layer.palette_index];
            let variation = metadata.variants.get(&palette.id);
            let texture = variation.map(|v| v.texture.clone()).or_else(|| palette.texture.clone());
            let dims = variation
                .map(|v| [v.width, v.height])
                .unwrap_or([palette.width.unwrap_or(8.0), palette.height.unwrap_or(8.0)]);
            let mask = layer
                .transition
                .as_ref()
                .map(|m| -> Result<Mask> {
                    let original = format!("{}{}", prefixes[set_index], m.filename_suffix());
                    let stem = original.strip_suffix(".tga").context("transition suffix")?;
                    Ok(Mask {
                        texture: format!("/assets/converted/textures/{stem}.png"),
                        rows: m.compositor_rows((metadata.texels_per_meter as f32 * map.cell_size) as u32)?,
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
            let color = if texture.is_some() {
                [1.0; 4]
            } else {
                let c = palette.color;
                [c[0], c[1], c[2], 1.0]
            };
            let draw = batches.entry(key).or_insert_with(|| Draw {
                texture: texture.clone(),
                color,
                mask,
                positions: Vec::new(),
                normals: Vec::new(),
                uvs: Vec::new(),
                cell_uvs: Vec::new(),
                indices: Vec::new(),
            });
            let first = u32::try_from(draw.positions.len())?;
            for (corner, v) in terrain.vertices[index * 4..index * 4 + 4].iter().enumerate() {
                draw.positions.push(v.position);
                draw.normals.push(v.normal);
                draw.uvs.push([v.position[0] / dims[0], 1.0 + v.position[2] / dims[1]]);
                draw.cell_uvs.push([(corner % 2) as f32, 1.0 - (corner / 2) as f32]);
            }
            draw.indices.extend([first, first + 1, first + 2, first + 1, first + 3, first + 2]);
        }
    }
    Ok(batches.into_values().collect())
}
