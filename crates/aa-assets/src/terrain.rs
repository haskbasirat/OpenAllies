use crate::{AssetError, AssetWarning, Bounds, Result, TerrainMap};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The recovered JSON has two grids, but lacks native tile-type/noise metadata.
/// Neither choice claims the native conditional mountain/water height algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeightMode {
    BaseOnly,
    BasePlusMountain,
    NativeConditional,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TerrainVariation {
    pub terrain_id: String,
    pub native_type: u8,
    #[serde(default)]
    pub noise_min: Option<f32>,
    #[serde(default)]
    pub noise_max: Option<f32>,
    pub source: String,
    pub source_sha256: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TerrainMetadata {
    pub schema: String,
    pub variations: BTreeMap<String, TerrainVariation>,
}
impl TerrainMetadata {
    /// Reviewed original-TGI evidence embedded in this build; no installed game data is changed.
    pub fn bundled() -> Result<Self> {
        let metadata: Self = serde_json::from_str(include_str!(
            "../../../research/engine-baseline/terrain-variation-metadata.json"
        ))
        .map_err(|e| AssetError::Invalid(format!("bundled terrain metadata: {e}")))?;
        if metadata.schema != "aa-native-terrain-variation-evidence-v1"
            || metadata.variations.values().any(|v| {
                v.native_type > 2
                    || (v.native_type == 1 && (v.noise_min.is_none() || v.noise_max.is_none()))
                    || v.noise_min
                        .into_iter()
                        .chain(v.noise_max)
                        .any(|n| !n.is_finite())
            })
        {
            return Err(AssetError::Invalid(
                "invalid native terrain metadata".into(),
            ));
        }
        Ok(metadata)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[repr(C)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TerrainBatch {
    pub palette_index: u32,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreparedTerrain {
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    pub height_mode: HeightMode,
    pub vertices: Vec<TerrainVertex>,
    pub batches: Vec<TerrainBatch>,
    pub bounds: Bounds,
    pub warnings: Vec<AssetWarning>,
}

/// One original terrain-variation identity and its authored rendering priority.
/// Priority is terrain `base_priority` plus the variation's append ordinal
/// (0x4B87A0); a map palette index alone is not a rendering priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainLayerInput {
    pub palette_index: u32,
    pub priority: i32,
}

/// Authored alpha mask and the native centered texture matrix selected for it.
/// The matrix is the raw 4x4 row-major value from 0x729180. The compositor also
/// applies target-resolution scale/offset; this is not a complete GPU UV matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainTransitionMask {
    pub base_mask: u8,
    pub transform: u32,
    pub matrix: [f32; 16],
}

impl TerrainTransitionMask {
    /// Native 0x72A3B4 compositor transform for a tile's texture target width.
    /// Shrinks to the texel centers before the original wrap/rotate/reflection.
    pub fn compositor_rows(&self, pixels_per_cell: u32) -> Result<[[f32; 4]; 2]> {
        if pixels_per_cell == 0 {
            return Err(AssetError::Invalid("zero terrain texel density".into()));
        }
        let scale = (pixels_per_cell - 1) as f32 / pixels_per_cell as f32;
        let half = 0.5 / pixels_per_cell as f32;
        let m = &self.matrix;
        Ok([
            [m[0] * scale, m[4] * scale, (m[0] + m[4]) * half + m[8], 0.0],
            [m[1] * scale, m[5] * scale, (m[1] + m[5]) * half + m[9], 0.0],
        ])
    }
    /// Filename within an authored TransitionSet's path_prefix. The center (5)
    /// is always present; eight surrounding bits use the other keypad positions.
    pub fn filename_suffix(&self) -> String {
        let mut name = String::new();
        for position in 0..9 {
            let enabled =
                position == 4 || self.base_mask & (1 << (position - u32::from(position > 4))) != 0;
            if enabled {
                name.push(char::from_digit(position + 1, 10).expect("grid digit"));
            }
        }
        name.push_str(".tga");
        name
    }
}

/// Coordinate-hashed TransitionSet selection from 0x7296B0. The runtime uses
/// signed wrapping 32-bit arithmetic and IDIV; invalid negative indices are
/// rejected rather than reproducing an out-of-bounds native array access.
pub fn terrain_transition_set_index(cell: [i32; 2], width: i32, count: usize) -> Result<usize> {
    let count = i32::try_from(count)
        .ok()
        .filter(|&n| n > 0)
        .ok_or_else(|| AssetError::Invalid("invalid terrain TransitionSet count".into()))?;
    if width <= 0 || cell[0] < 0 || cell[1] < 0 || cell[0] >= width {
        return Err(AssetError::Invalid(
            "invalid terrain transition coordinates".into(),
        ));
    }
    let hash = cell[1]
        .wrapping_mul(width)
        .wrapping_add(cell[0])
        .wrapping_mul(4723)
        .wrapping_add(6323);
    usize::try_from((hash % 6997) % count)
        .map_err(|_| AssetError::Invalid("native terrain hash produced a negative index".into()))
}

/// CPU portion of a native terrain cell's ordered texture passes. The first
/// pass covers the cell; subsequent passes use original transition masks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TerrainTextureLayer {
    pub palette_index: u32,
    pub priority: i32,
    pub neighbor_mask: u8,
    pub transition: Option<TerrainTransitionMask>,
}

/// Exact native neighborhood/layer/mask decisions, kept separate from the
/// current single-texture terrain preview. GPU alpha composition, block borders
/// and target texel alignment have not yet been reproduced.
#[derive(Debug, Clone)]
pub struct TerrainLayerPlanner {
    masks: Vec<Option<TerrainTransitionMask>>,
}

impl TerrainLayerPlanner {
    pub fn bundled() -> Result<Self> {
        #[derive(Deserialize)]
        struct Catalog {
            schema: u32,
            table: Vec<Option<TerrainTransitionMask>>,
        }
        let catalog: Catalog = serde_json::from_str(include_str!(
            "../../../research/phase5-terrain/mask-lookup.json"
        ))
        .map_err(|e| AssetError::Invalid(format!("terrain mask catalog: {e}")))?;
        if catalog.schema != 1
            || catalog.table.len() != 256
            || catalog.table[0].is_none()
            || catalog
                .table
                .iter()
                .flatten()
                .any(|m| m.matrix.iter().any(|v| !v.is_finite()))
        {
            return Err(AssetError::Invalid("invalid terrain mask catalog".into()));
        }
        Ok(Self {
            masks: catalog.table,
        })
    }

    /// Original 0x729470 / 0x729760 for a fully resolved row-major 3x3 grid.
    /// Native CRT-qsort ordering of distinct variations with equal priority is
    /// not established; those ambiguous inputs are rejected instead of guessed.
    pub fn plan(&self, neighbors: [TerrainLayerInput; 9]) -> Result<Vec<TerrainTextureLayer>> {
        let mut identities = BTreeMap::new();
        let mut priorities = BTreeMap::new();
        for entry in neighbors {
            if identities
                .insert(entry.palette_index, entry.priority)
                .is_some_and(|priority| priority != entry.priority)
            {
                return Err(AssetError::Invalid(
                    "one terrain variation has conflicting priorities".into(),
                ));
            }
            if priorities
                .insert(entry.priority, entry.palette_index)
                .is_some_and(|id| id != entry.palette_index)
            {
                return Err(AssetError::Invalid(
                    "equal terrain priorities require native qsort tie recovery".into(),
                ));
            }
        }
        // Center, touching edge(s), then corner. A minimum-priority material at
        // any touching cell occupies that sample of the cell's transition mask.
        const TOUCHING: [&[usize]; 9] = [
            &[4, 3, 0, 1],
            &[4, 1],
            &[4, 5, 2, 1],
            &[4, 3],
            &[4],
            &[4, 5],
            &[4, 3, 6, 7],
            &[4, 7],
            &[4, 7, 8, 5],
        ];
        let mut layers = BTreeMap::<i32, TerrainTextureLayer>::new();
        for (sample, touching) in TOUCHING.iter().enumerate() {
            let entry = touching
                .iter()
                .map(|&i| neighbors[i])
                .min_by_key(|v| v.priority)
                .expect("nonempty native sample");
            let layer = layers.entry(entry.priority).or_insert(TerrainTextureLayer {
                palette_index: entry.palette_index,
                priority: entry.priority,
                neighbor_mask: 0,
                transition: None,
            });
            if sample != 4 {
                layer.neighbor_mask |= 1 << (sample - usize::from(sample > 4));
            }
        }
        let mut layers: Vec<_> = layers.into_values().collect();
        for (index, layer) in layers.iter_mut().enumerate() {
            let mask = &mut layer.neighbor_mask;
            // 0x729929..0x72996C fills an edge between two occupied corners.
            for (corners, edge) in [(0x05, 0x02), (0x21, 0x08), (0x84, 0x10), (0xA0, 0x40)] {
                if *mask & corners == corners {
                    *mask |= edge;
                }
            }
            if index > 0 {
                layer.transition = self.masks[usize::from(*mask)]
                    .as_ref()
                    .or(self.masks[0].as_ref())
                    .cloned();
            }
        }
        Ok(layers)
    }
}

pub(crate) fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len.is_finite() && len > 1e-12 {
        v.map(|n| n / len)
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Whole-map equivalent of the original terrain tile compositor's source UV.
/// 0x72B2B3 initializes the quad with decreasing V as map Y increases, and
/// 0x72A473 scales by authored texture_width/height and wraps the block origin.
/// The whole-map form requires a repeat sampler; it avoids a seam at each cell.
/// This is not the normalized UV of a native terrain blend-mask texture.
fn source_uv(world: [f32; 2], repeat: [f32; 2]) -> [f32; 2] {
    [world[0] / repeat[0], 1.0 - world[1] / repeat[1]]
}

impl TerrainMap {
    pub fn validate(&self) -> Result<()> {
        let cells = u64::from(self.width) * u64::from(self.height);
        let corners = (u64::from(self.width) + 1)
            .checked_mul(u64::from(self.height) + 1)
            .ok_or_else(|| AssetError::Invalid("terrain corner count overflow".into()))?;
        let last_vertex = cells.checked_mul(4).and_then(|v| v.checked_sub(1));
        if cells == 0
            || last_vertex.is_none_or(|v| u32::try_from(v).is_err())
            || !self.cell_size.is_finite()
            || self.cell_size <= 0.0
        {
            return Err(AssetError::Invalid(format!(
                "{}: invalid terrain dimensions/spacing",
                self.id
            )));
        }
        if self.tiles.len() as u64 != cells
            || self.heights.len() as u64 != corners
            || self.mountain_heights.len() as u64 != corners
        {
            return Err(AssetError::Invalid(format!(
                "{}: terrain array lengths disagree with dimensions",
                self.id
            )));
        }
        if self
            .heights
            .iter()
            .chain(&self.mountain_heights)
            .any(|v| !v.is_finite())
        {
            return Err(AssetError::Invalid(format!(
                "{}: non-finite height",
                self.id
            )));
        }
        if self
            .palette
            .values()
            .any(|p| p.color.iter().any(|v| !v.is_finite()))
            || self.tiles.iter().any(|id| !self.palette.contains_key(id))
        {
            return Err(AssetError::Invalid(format!(
                "{}: missing/invalid terrain palette entry",
                self.id
            )));
        }
        Ok(())
    }

    pub fn height_at_grid(&self, x: u32, y: u32, mode: HeightMode) -> Option<f32> {
        if mode == HeightMode::NativeConditional {
            return self
                .native_height_at_grid(x, y, &TerrainMetadata::bundled().ok()?)
                .map(|p| p.0);
        }
        if x > self.width || y > self.height {
            return None;
        }
        let i = y as usize * (self.width as usize + 1) + x as usize;
        let base = *self.heights.get(i)?;
        Some(
            base + if mode == HeightMode::BasePlusMountain {
                *self.mountain_heights.get(i)?
            } else {
                0.0
            },
        )
    }

    /// Native render query 0x122 at lattice corners: conditional mountain and
    /// current-tile noise, without bit-1 low clamping. This does not claim x87
    /// bit equivalence or arbitrary-position interpolation parity.
    /// Returns None only for invalid grid coordinates; boolean marks complete metadata.
    pub fn native_height_at_grid(
        &self,
        x: u32,
        y: u32,
        metadata: &TerrainMetadata,
    ) -> Option<(f32, bool)> {
        if x > self.width || y > self.height || self.width == 0 || self.height == 0 {
            return None;
        }
        let base = *self
            .heights
            .get(y as usize * (self.width as usize + 1) + x as usize)?;
        let tx = x.min(self.width - 1);
        let ty = y.min(self.height - 1);
        let info = |cx: u32, cy: u32| {
            let id = *self
                .tiles
                .get(cy as usize * self.width as usize + cx as usize)?;
            metadata.variations.get(&self.palette.get(&id)?.id)
        };
        let Some(current) = info(tx, ty) else {
            return Some((base, false));
        };
        let mut add_mountain = matches!(current.native_type, 1 | 2);
        let mut complete = true;
        for cy in ty.saturating_sub(1)..=(ty + 1).min(self.height - 1) {
            for cx in tx.saturating_sub(1)..=(tx + 1).min(self.width - 1) {
                match info(cx, cy) {
                    Some(v) => add_mountain |= matches!(v.native_type, 1 | 2),
                    None => complete = false,
                }
            }
        }
        // Unknown neighbors only matter if no known neighbor already enables it.
        if !complete && !add_mountain {
            return Some((base, false));
        }
        let mut result = f64::from(base);
        if add_mountain {
            let mountain = *self
                .mountain_heights
                .get(y as usize * (self.width as usize + 1) + x as usize)?;
            let noise = if current.native_type == 1 {
                let index = ty * self.width + tx;
                // Actual native arithmetic is wrapping signed 32-bit before IDIV.
                let hash = (index as i32).wrapping_mul(4723).wrapping_add(6323);
                let factor = (hash % 6997) % 1000;
                let min = f64::from(current.noise_min?);
                let max = f64::from(current.noise_max?);
                (max - min) * f64::from(factor) * f64::from(0.001_f32) + min
            } else {
                0.0
            };
            // Native x87 accumulates then stores float32; f64 is an approximation
            // of the extended intermediate, avoiding premature f32 rounding.
            result += f64::from(mountain) + noise;
        }
        Some((result as f32, true))
    }

    /// Bilinear inspection height, not native collision/ground queries.
    pub fn height_at_native(&self, x: f32, y: f32, mode: HeightMode) -> Option<f32> {
        if !x.is_finite() || !y.is_finite() || self.cell_size <= 0.0 {
            return None;
        }
        let gx = x / self.cell_size;
        let gy = y / self.cell_size;
        if gx < 0.0 || gy < 0.0 || gx > self.width as f32 || gy > self.height as f32 {
            return None;
        }
        let x0 = gx.floor() as u32;
        let y0 = gy.floor() as u32;
        let x1 = (x0 + 1).min(self.width);
        let y1 = (y0 + 1).min(self.height);
        let tx = gx - x0 as f32;
        let ty = gy - y0 as f32;
        let h = |a, b| self.height_at_grid(a, b, mode);
        Some(
            (h(x0, y0)? * (1.0 - tx) + h(x1, y0)? * tx) * (1.0 - ty)
                + (h(x0, y1)? * (1.0 - tx) + h(x1, y1)? * tx) * ty,
        )
    }

    pub fn prepare(&self, mode: HeightMode) -> Result<PreparedTerrain> {
        let metadata = if mode == HeightMode::NativeConditional {
            Some(TerrainMetadata::bundled()?)
        } else {
            None
        };
        self.prepare_with_metadata(mode, metadata.as_ref())
    }

    pub fn prepare_with_metadata(
        &self,
        mode: HeightMode,
        metadata: Option<&TerrainMetadata>,
    ) -> Result<PreparedTerrain> {
        self.validate()?;
        let mut warnings: Vec<_> = self
            .limitations
            .iter()
            .map(|message| AssetWarning {
                source: self.source.clone(),
                message: message.clone(),
            })
            .collect();
        let resolved_heights = if mode == HeightMode::NativeConditional {
            let metadata = metadata.ok_or_else(|| {
                AssetError::Invalid("NativeConditional requires terrain metadata".into())
            })?;
            let mut resolved = Vec::new();
            resolved
                .try_reserve_exact(self.heights.len())
                .map_err(|e| AssetError::Invalid(format!("terrain allocation: {e}")))?;
            let mut incomplete = 0;
            for y in 0..=self.height {
                for x in 0..=self.width {
                    let (height, complete) =
                        self.native_height_at_grid(x, y, metadata).ok_or_else(|| {
                            AssetError::Invalid(format!(
                                "{}: invalid native terrain metadata",
                                self.id
                            ))
                        })?;
                    resolved.push(height);
                    if !complete {
                        incomplete += 1;
                    }
                }
            }
            if incomplete > 0 {
                warnings.push(AssetWarning{source:self.source.clone(),message:format!("{incomplete} terrain corners use base-only fallback because active terrain-type metadata is unresolved; disabled legacy aliases are not revived.")});
            }
            let unresolved: Vec<_> = self
                .palette
                .values()
                .filter(|p| !metadata.variations.contains_key(&p.id))
                .map(|p| p.id.as_str())
                .collect();
            if !unresolved.is_empty() {
                warnings.push(AssetWarning {
                    source: self.source.clone(),
                    message: format!(
                        "Unresolved terrain variation IDs: {}",
                        unresolved.join(", ")
                    ),
                });
            }
            warnings.push(AssetWarning{source:self.source.clone(),message:"Uses native conditional mountain/type-1 noise control flow at grid vertices (render query0x122); rounded recovered floats and f64 intermediates are not bit-exact x87. Native arbitrary-position interpolation/edge mesh behavior remains unverified.".into()});
            resolved
        } else {
            warnings.push(AssetWarning { source: self.source.clone(), message: format!("Height mode {mode:?} is an inspection approximation: native mountain contribution is conditional on tile types/neighbors and adds type-1 noise.") });
            self.heights
                .iter()
                .zip(&self.mountain_heights)
                .map(|(&base, &mountain)| {
                    base + if mode == HeightMode::BasePlusMountain {
                        mountain
                    } else {
                        0.0
                    }
                })
                .collect()
        };
        let h =
            |x: u32, y: u32| resolved_heights[y as usize * (self.width as usize + 1) + x as usize];
        let normal = |x: u32, y: u32| {
            let xa = x.saturating_sub(1);
            let xb = (x + 1).min(self.width);
            let ya = y.saturating_sub(1);
            let yb = (y + 1).min(self.height);
            normalize([
                -(h(xb, y) - h(xa, y)) / ((xb - xa) as f32 * self.cell_size),
                1.0,
                (h(x, yb) - h(x, ya)) / ((yb - ya) as f32 * self.cell_size),
            ])
        };
        // Fallible reservation reports address/memory exhaustion, with no arbitrary map-size cap.
        let vertex_count = self
            .tiles
            .len()
            .checked_mul(4)
            .ok_or_else(|| AssetError::Invalid("terrain vertex count overflow".into()))?;
        let mut vertices = Vec::new();
        vertices
            .try_reserve_exact(vertex_count)
            .map_err(|e| AssetError::Invalid(format!("terrain vertex allocation: {e}")))?;
        vertices.resize(
            vertex_count,
            TerrainVertex {
                position: [0.0; 3],
                normal: [0.0; 3],
                uv: [0.0; 2],
                color: [0.0; 4],
            },
        );
        // Indexed disjoint chunks preserve cell order across thread counts.
        vertices.par_chunks_mut(4).enumerate().for_each(|(i, out)| {
            let palette_id = &self.tiles[i];
            let x = i as u32 % self.width;
            let y = i as u32 / self.width;
            let p = &self.palette[palette_id];
            let tw = p.width.filter(|v| v.is_finite() && *v > 0.0).unwrap_or(8.0);
            let th = p
                .height
                .filter(|v| v.is_finite() && *v > 0.0)
                .unwrap_or(8.0);
            let cell = [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)].map(|(cx, cy)| {
                let wx = cx as f32 * self.cell_size;
                let wy = cy as f32 * self.cell_size;
                TerrainVertex {
                    position: [wx, h(cx, cy), -wy],
                    normal: normal(cx, cy),
                    uv: source_uv([wx, wy], [tw, th]),
                    // Native normal-mode compositor 0x729F30 writes white.
                    // terrain_color belongs to the minimap, not these textures.
                    color: if p.texture.is_some() {
                        [1.0; 4]
                    } else {
                        [p.color[0], p.color[1], p.color[2], 1.0]
                    },
                }
            });
            out.copy_from_slice(&cell);
        });
        if vertices
            .iter()
            .any(|v| v.position.iter().any(|x| !x.is_finite()))
        {
            return Err(AssetError::Invalid(format!(
                "{}: combined height overflow",
                self.id
            )));
        }
        let mut counts = BTreeMap::<u32, usize>::new();
        for id in &self.tiles {
            *counts.entry(*id).or_default() += 1;
        }
        let mut batches = BTreeMap::<u32, Vec<u32>>::new();
        for (id, count) in counts {
            let mut indices = Vec::new();
            indices
                .try_reserve_exact(
                    count.checked_mul(6).ok_or_else(|| {
                        AssetError::Invalid("terrain index count overflow".into())
                    })?,
                )
                .map_err(|e| AssetError::Invalid(format!("terrain index allocation: {e}")))?;
            batches.insert(id, indices);
        }
        for (i, palette_id) in self.tiles.iter().enumerate() {
            let v = i as u32 * 4;
            batches
                .entry(*palette_id)
                .or_default()
                .extend([v, v + 1, v + 2, v + 1, v + 3, v + 2]);
        }
        let bounds = Bounds::from_positions(vertices.iter().map(|v| &v.position))
            .expect("nonempty validated terrain");
        Ok(PreparedTerrain {
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
            height_mode: mode,
            vertices,
            batches: batches
                .into_iter()
                .map(|(palette_index, indices)| TerrainBatch {
                    palette_index,
                    indices,
                })
                .collect(),
            bounds,
            warnings,
        })
    }
}

#[cfg(test)]
mod source_uv_tests {
    use super::source_uv;

    #[test]
    fn original_x86_quad_matrix_fixtures_are_repeat_equivalent() {
        let report: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase3-terrain/terrain-uv-native-fixtures.json"
        ))
        .unwrap();
        assert_eq!(report["passed"], true);
        let cases = report["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 34);
        for case in cases {
            let dims =
                std::array::from_fn(|i| case["texture_dimensions"][i].as_f64().unwrap() as f32);
            for i in 0..4 {
                let xy = std::array::from_fn(|j| case["world_xy"][i][j].as_f64().unwrap() as f32);
                let actual = source_uv(xy, dims);
                for (j, value) in actual.into_iter().enumerate() {
                    let native = case["native_uv"][i][j].as_f64().unwrap();
                    let difference = f64::from(value) - native;
                    assert!(
                        (difference - difference.round()).abs() < 2e-5,
                        "world={xy:?}, repeat={dims:?}, axis={j}, Rust={value}, native={native}"
                    );
                }
                assert_eq!(
                    case["modulation"][i],
                    serde_json::json!([1.0, 1.0, 1.0, 1.0])
                );
            }
        }
    }
}
