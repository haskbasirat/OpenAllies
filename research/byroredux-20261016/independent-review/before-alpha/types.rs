use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MapInfo {
    /// Full directory identity, e.g. `Campaign/tutorial01`, not a basename.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub source: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub biomes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TerrainPaletteEntry {
    pub id: String,
    #[serde(default)]
    pub texture: Option<String>,
    /// Original terrain_color, intended for the minimap. It is also retained as
    /// an inspection fallback when no texture resolves; it is not diffuse tint.
    pub color: [f32; 3],
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default)]
    pub height: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerrainMap {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    #[serde(default)]
    pub biomes: Vec<String>,
    /// Row-major cell IDs; there are `width * height` cells.
    pub tiles: Vec<u32>,
    /// Row-major corner samples; there are `(width+1) * (height+1)` samples.
    pub heights: Vec<f32>,
    pub mountain_heights: Vec<f32>,
    pub palette: BTreeMap<u32, TerrainPaletteEntry>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub instance_id: u32,
    pub thing_id: String,
    pub class_code: u32,
    pub thing_version: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Null components preserve the extractor's unresolved/non-finite warning.
    pub orientation: [Option<f32>; 3],
    #[serde(default)]
    pub yaw: Option<f32>,
    #[serde(default)]
    pub scale_multiplier: Option<f32>,
    #[serde(default)]
    pub scale: Option<f32>,
    #[serde(default)]
    pub variant_index: Option<i32>,
    pub base_field: u32,
    pub record_offset: u64,
    pub record_length: u64,
    #[serde(default)]
    pub object_version: Option<u32>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl Placement {
    pub fn native_position(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
    pub fn render_position(&self) -> [f32; 3] {
        crate::native_to_render(self.native_position())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelReference {
    #[serde(default)]
    pub aspect_id: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlacementType {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub models: Vec<ModelReference>,
    #[serde(default)]
    pub aspects_as_variations: bool,
    #[serde(default)]
    pub scale_min: Option<f32>,
    #[serde(default)]
    pub scale_max: Option<f32>,
    #[serde(default)]
    pub unknown: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlacementDecodeError {
    pub record_offset: u64,
    pub record_length: u64,
    pub error: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlacementMap {
    pub id: String,
    pub source: String,
    pub expected_records: usize,
    pub actors: Vec<Placement>,
    pub types: BTreeMap<String, PlacementType>,
    #[serde(default)]
    pub errors: Vec<PlacementDecodeError>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MapAsset {
    pub terrain: TerrainMap,
    pub placements: PlacementMap,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub size: [f32; 3],
}

impl Bounds {
    pub fn from_positions<'a>(positions: impl IntoIterator<Item = &'a [f32; 3]>) -> Option<Self> {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        let mut count = 0;
        for p in positions {
            count += 1;
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        (count > 0).then(|| Self {
            min,
            max,
            size: std::array::from_fn(|i| max[i] - min[i]),
        })
    }
}

pub(crate) fn white() -> [f32; 3] {
    [1.0; 3]
}
pub(crate) fn one() -> f32 {
    1.0
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    #[serde(default = "white")]
    pub color: [f32; 3],
    #[serde(default = "one")]
    pub opacity: f32,
    #[serde(default)]
    pub emissive: [f32; 3],
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub texture: Option<String>,
    #[serde(default)]
    pub source_texture: Option<String>,
    /// Raw authored base-map descriptor values. Retaining these does not select
    /// preview sampler behavior or infer defaults for older converted payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_texture_state: Option<BaseTextureState>,
    #[serde(default)]
    pub double_sided: bool,
    #[serde(default)]
    pub alpha_blend: bool,
    #[serde(default)]
    pub alpha_test: f32,
    #[serde(default)]
    pub team_color_mask: bool,
    /// Source-bound inherited NIF render properties, supplied by the audited
    /// sidecar. The legacy conversion did not preserve this state.
    #[serde(default)]
    pub native_render_state: Option<NativeRenderState>,
    /// Authored NiGeometry sphere center transformed into the existing static
    /// preview's model basis, supplied by the source-bound material sidecar.
    #[serde(default)]
    pub native_sort_center: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BaseTextureState {
    pub clamp_mode: u32,
    pub filter_mode: u32,
    pub uv_set: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub struct NativeRenderState {
    pub alpha_flags: u16,
    pub alpha_threshold: u8,
    pub zbuffer_flags: u16,
    pub zbuffer_function: u8,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            color: white(),
            opacity: 1.0,
            emissive: [0.0; 3],
            name: None,
            texture: None,
            source_texture: None,
            base_texture_state: None,
            double_sided: false,
            alpha_blend: false,
            alpha_test: 0.0,
            team_color_mask: false,
            native_render_state: None,
            native_sort_center: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RawMesh {
    pub name: String,
    #[serde(default)]
    pub hierarchy: Vec<String>,
    pub positions: Vec<f32>,
    #[serde(default)]
    pub normals: Vec<f32>,
    #[serde(default)]
    pub uvs: Vec<f32>,
    pub indices: Vec<u32>,
    #[serde(default)]
    pub material: Material,
    #[serde(default)]
    pub skinned_pose: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SkippedHelper {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelAsset {
    pub format: String,
    pub source: String,
    pub nif_version: String,
    pub up_axis: String,
    pub pose: String,
    pub bounds: Bounds,
    pub meshes: Vec<RawMesh>,
    #[serde(default)]
    pub skipped_helpers: Vec<SkippedHelper>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub source: String,
    pub url: String,
    pub name: String,
    pub mesh_count: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub bounds: Bounds,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelManifest {
    pub format: String,
    pub model_count: usize,
    pub models: Vec<ModelInfo>,
    pub by_path: BTreeMap<String, String>,
}
