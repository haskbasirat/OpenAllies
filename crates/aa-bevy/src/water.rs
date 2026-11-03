//! The water of a battle's terrain, as the original draws it (VTerrain's water blocks):
//!
//! * A terrain cell carries water when its own terrain or the first of its eight
//!   neighbours in the native ring is a WaterTerrain (0x738280, 0x738320).
//! * Its geometry is the saved cell of GMap+5C when the map holds one (0x55C540; an
//!   existing empty cell draws nothing), else a regular quad with the indices 0,3,1 / 2,1,3
//!   (0x7376B0).
//! * The surface of a vertex is the base map minus 0.5 (0x736AB0: 0x550210 on GWorld+0x34
//!   minus [9FBDB8]); its depth is that minus the ground query 0x4F0E50(0x122), and the
//!   WaterInfo of the Biome gives its colour and alpha (0x4B7490, 0x4B6E10). The surface
//!   hides under the ground where it stands below it: only the depth test does that.
//! * 0x7377F0 makes the mesh: upward normals, UVs block relative (TerrainBlockSize, 8 m,
//!   0x723E00), NiVertexColorProperty source 2 lighting 1 (the vertex colour is the ambient
//!   and the diffuse), NiAlphaProperty 0xED, the texture wrapped, filtered and MODULATE,
//!   z test on, no z write, and a NiFlipController over the terrain's frames with a frame
//!   time of 1 / anim_fps, looping.
//!
//! The arithmetic of 0x4B6E10 and of the geometry is proven against the original in
//! `research/phase6-water` (the fixtures of the earlier client).
//!
//! The texture translation 0x736D50 is the fraction of the terrain's cloud scroll VTerrain
//! +0x50 / +0x54 (0x71E020): the water's texture moves with the clouds' map.
use crate::look::{self, OriginalMaterial};
use crate::textures::{self, Kind};
use aa_assets::{AssetStore, TerrainMap};
use aa_numeric::{Binary, Precision};
use aa_play::map::NativeTerrain;
use anyhow::{Context, Result, bail, ensure};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

/// Terrain cells a batch of the mesh covers on each side. The original's batches are its
/// terrain blocks (8 m); a batch here only has to be small enough to be culled.
const BATCH_CELLS: usize = 32;
const REGULAR_INDICES: [u32; 6] = [0, 3, 1, 2, 1, 3];

#[derive(Deserialize)]
struct WaterTerrain {
    first_frame: String,
    frames_per_second: f32,
}
#[derive(Deserialize)]
struct Metadata {
    schema: u32,
    block_size: u32,
    terrains: BTreeMap<String, WaterTerrain>,
    variants: BTreeMap<String, String>,
    biomes: BTreeMap<String, BTreeMap<String, Vec<[f32; 5]>>>,
}

/// The cells of GMap+5C (0x55C540), from the TERRAIN chunk's auxiliary bytes.
#[derive(Default)]
pub struct SavedCells {
    vertices: Vec<[f32; 2]>,
    indices: Vec<u32>,
    /// A missing cell takes the regular quad (0x7376B0), an existing empty one draws nothing.
    cells: BTreeMap<[u32; 2], Range<usize>>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8]> {
        let end = self.at.checked_add(count).context("water extent overflow")?;
        let out = self.bytes.get(self.at..end).context("truncated water geometry")?;
        self.at = end;
        Ok(out)
    }
    fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn version(&mut self) -> Result<()> {
        ensure!(self.word()? == 0, "unverified water/region serialization version");
        Ok(())
    }
    fn count(&mut self, minimum: usize) -> Result<usize> {
        let count = self.word()? as usize;
        ensure!(count <= i32::MAX as usize && count <= (self.bytes.len() - self.at) / minimum, "water count exceeds source extent");
        Ok(count)
    }
}

/// 0x7AA9B0 (the region graph, passed over) and 0x55C540.
pub fn saved_cells(bytes: &[u8], width: u32, height: u32) -> Result<SavedCells> {
    ensure!(width > 0 && height > 0 && width.checked_mul(height).is_some(), "invalid water grid");
    let mut r = Reader { bytes, at: 0 };
    r.version()?;
    if r.take(1)?[0] != 0 {
        for _ in 0..r.count(16)? {
            // 0x7BBAF0: version, two words, pair vector
            r.version()?;
            r.take(8)?;
            let pairs = r.count(8)?;
            r.take(pairs * 8)?;
        }
        for _ in 0..r.count(36)? {
            // 0x7BACF0: version, two words, four side lists
            r.version()?;
            r.take(8)?;
            for _ in 0..4 {
                r.take(2)?;
                for _ in 0..r.count(16)? {
                    // 0x7B94A0: four shorts and a word
                    r.version()?;
                    r.take(12)?;
                }
            }
        }
    }
    r.version()?;
    let vertex_count = r.count(8)?;
    let mut vertices = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        let vertex = [f32::from_bits(r.word()?), f32::from_bits(r.word()?)];
        ensure!(vertex.iter().all(|v| v.is_finite()), "nonfinite cached water vertex");
        vertices.push(vertex);
    }
    let index_count = r.count(4)?;
    let mut indices = Vec::with_capacity(index_count);
    for _ in 0..index_count {
        let index = r.word()?;
        ensure!((index as usize) < vertex_count, "cached water vertex index outside array");
        indices.push(index);
    }
    let mut cells = BTreeMap::new();
    for _ in 0..r.count(16)? {
        let cell = [r.word()?, r.word()?];
        let first = r.word()? as usize;
        let last = r.word()? as usize;
        ensure!(cell[0] < width && cell[1] < height, "cached water cell outside grid");
        let range = if first > last {
            // 0x7377F0 compares these as unsigned and skips this existing cache entry.
            0..0
        } else {
            ensure!(last < index_count && (last - first + 1).is_multiple_of(3), "cached water triangle range outside array or incomplete");
            first..last + 1
        };
        if cells.insert(cell, range).is_some() {
            bail!("duplicate cached water cell");
        }
    }
    Ok(SavedCells { vertices, indices, cells })
}

/// 0x4B6E10: the colour and alpha of a depth on the WaterInfo's curve, on the x87 of the
/// thread that runs it: the interpolation weights stay extended, only the four outputs are
/// stored binary32. A curve 0x4B8530 would not have loaded gives nothing.
fn depth_color(points: &[[f32; 5]], depth: f32, precision: Precision) -> Option<[f32; 4]> {
    let rgba = |p: &[f32; 5]| [p[1], p[2], p[3], p[4]];
    let (first, last) = (points.first()?, points.last()?);
    if !depth.is_finite() {
        return None;
    }
    if depth < first[0] {
        return Some(rgba(first));
    }
    if depth >= last[0] {
        return Some(rgba(last));
    }
    let index = points.iter().position(|p| depth < p[0])?;
    let (previous, current) = (&points[index - 1], &points[index]);
    let b = |n: f32| Binary::from_f32_bits(n.to_bits()).ok();
    let fraction = b(depth)?.sub(&b(previous[0])?, precision).ok()?.div(&b(current[0])?.sub(&b(previous[0])?, precision).ok()?, precision).ok()?;
    let inverse = Binary::from_u32(1).sub(&fraction, precision).ok()?;
    let mut out = [0.0; 4];
    for (channel, value) in out.iter_mut().enumerate() {
        let sum = inverse.mul(&b(previous[channel + 1])?, precision).ok()?.add(&fraction.mul(&b(current[channel + 1])?, precision).ok()?, precision).ok()?;
        *value = f32::from_bits(sum.store_f32().ok()?);
    }
    Some(out)
}

/// 0x7376B0, the ground positions of a regular cell.
fn regular_cell(x: usize, y: usize, cell: f32) -> [[f32; 2]; 4] {
    let (fx, fy) = (x as f32 * cell, y as f32 * cell);
    [[fx, fy], [fx, fy + cell], [fx + cell, fy + cell], [fx + cell, fy]]
}

/// 0x738280, the current cell's water terrain, then the native clockwise ring.
fn selected_cell(cell: [usize; 2], size: [usize; 2], mut water: impl FnMut([usize; 2]) -> bool) -> Option<[usize; 2]> {
    if water(cell) {
        return Some(cell);
    }
    for [dx, dy] in [[-1i64, -1i64], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]] {
        let (x, y) = (cell[0] as i64 + dx, cell[1] as i64 + dy);
        if x >= 0 && y >= 0 && x < size[0] as i64 && y < size[1] as i64 {
            let neighbour = [x as usize, y as usize];
            if water(neighbour) {
                return Some(neighbour);
            }
        }
    }
    None
}

/// The frames and the speed of a water terrain (0x4B87A0).
struct Material {
    frames: Vec<String>,
    fps: f32,
    curve: Vec<[f32; 5]>,
}

/// Vertices of a batch that came from one cell, for when the ground changes.
#[derive(Clone)]
struct CellSpan {
    cell: [u32; 2],
    first: u32,
    count: u32,
}

/// One batch of the water mesh.
struct Batch {
    material: usize,
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    spans: Vec<CellSpan>,
}

impl Batch {
    fn mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; self.positions.len()]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors.clone());
        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh
    }
}

/// A battle's water, from the map and the world's ground.
#[derive(Default)]
pub struct Water {
    materials: Vec<Material>,
    batches: Vec<Batch>,
}

/// (surface height, colour on the shader's linear scale) of a point on the native ground.
fn vertex(native: &NativeTerrain, curve: &[[f32; 5]], x: f32, y: f32) -> (f32, [f32; 4]) {
    // 0x736AB0: the unstored 0x550210 minus 0.5, stored; depth = that minus 0x4F0E50(0x122).
    let height = native.base_height(x, y) - 0.5;
    let depth = height - native.ground(x, y);
    // The view's thread runs at PC24 (research/phase8-tick/fpu-precision.md).
    let [r, g, b, a] = depth_color(curve, depth, Precision::Single24).unwrap_or([0.0; 4]);
    (height, [r, g, b, a])
}

/// The water of this map, and the texture frames it needs.
pub fn build(store: &AssetStore, map: &TerrainMap, native: Option<&NativeTerrain>, biome: &str, saved: Option<&SavedCells>, requests: &mut textures::Requests) -> Result<Water> {
    let Some(native) = native else { return Ok(Water::default()) };
    let meta: Metadata = serde_json::from_str(include_str!("../../../research/phase6-water/water-metadata.json"))?;
    ensure!(meta.schema == 1 && meta.block_size > 0, "water metadata schema");
    let (width, height, cell) = (map.width as usize, map.height as usize, map.cell_size);
    let block = meta.block_size as f32;
    let cells_per_block = (block / cell).max(1.0) as usize;
    let terrain_at = |x: usize, y: usize| -> Option<&String> {
        let tile = *map.tiles.get(y * width + x)?;
        meta.variants.get(&map.palette.get(&tile)?.id)
    };
    let mut materials: Vec<Material> = Vec::new();
    let mut material_of: HashMap<String, Option<usize>> = HashMap::new();
    let mut batches: BTreeMap<(usize, usize, usize), Batch> = BTreeMap::new();
    let mut corners: HashMap<(u32, u32, usize), (f32, [f32; 4])> = HashMap::new();
    for y in 0..height {
        for x in 0..width {
            let Some([tx, ty]) = selected_cell([x, y], [width, height], |[nx, ny]| terrain_at(nx, ny).is_some()) else { continue };
            let Some(terrain_id) = terrain_at(tx, ty) else { continue };
            let material = *material_of.entry(terrain_id.clone()).or_insert_with(|| {
                let terrain = meta.terrains.get(terrain_id)?;
                let curve = meta.biomes.get(biome)?.get(terrain_id)?.clone();
                // 0x4B87A0: anim_textures_path_prefix, frames from 0 until one is missing.
                let (head, tail) = terrain.first_frame.split_once("0000")?;
                let frames: Vec<String> = (0..10_000).map(|i| format!("{head}{i:04}{tail}")).take_while(|url| store.resolve_url(url).is_ok_and(|p| p.is_file())).collect();
                if frames.is_empty() {
                    return None;
                }
                for frame in &frames {
                    requests.add(frame, Kind::Color);
                }
                materials.push(Material { frames, fps: terrain.frames_per_second, curve });
                Some(materials.len() - 1)
            });
            let Some(material) = material else { continue };
            let saved_range = saved.and_then(|s| s.cells.get(&[x as u32, y as u32]).map(|r| (s, r)));
            let (points, order): (Vec<[f32; 2]>, Vec<u32>) = if let Some((s, range)) = saved_range {
                if range.is_empty() {
                    continue;
                }
                let points: Vec<[f32; 2]> = s.indices[range.clone()].iter().map(|&i| s.vertices[i as usize]).collect();
                let order = (0..points.len() as u32).collect();
                (points, order)
            } else {
                (regular_cell(x, y, cell).to_vec(), REGULAR_INDICES.to_vec())
            };
            let batch = batches.entry((y / BATCH_CELLS, x / BATCH_CELLS, material)).or_insert_with(|| Batch { material, positions: Vec::new(), colors: Vec::new(), uvs: Vec::new(), indices: Vec::new(), spans: Vec::new() });
            let start = batch.positions.len() as u32;
            let curve = &materials[material].curve;
            for &[wx, wy] in &points {
                // Shared corners: the extended arithmetic once.
                let (z, color) = *corners.entry((wx.to_bits(), wy.to_bits(), material)).or_insert_with(|| vertex(native, curve, wx, wy));
                batch.positions.push(aa_assets::native_to_render([wx, wy, z]));
                batch.colors.push(color);
                batch.uvs.push([(wx - (x / cells_per_block) as f32 * block) / block, (wy - (y / cells_per_block) as f32 * block) / block]);
            }
            batch.indices.extend(order.into_iter().map(|i| start + i));
            batch.spans.push(CellSpan { cell: [x as u32, y as u32], first: start, count: points.len() as u32 });
        }
    }
    Ok(Water { materials, batches: batches.into_values().collect() })
}

/// The models of the doodads a water's edge carries (the WaterTerrain's water_doodad_IDS,
/// placed along the shore by 0x55B4E0): animated, blended quads.
pub const SHORE_MODELS: [&str; 7] = [
    "doodads/shared/waves/waves01.nif",
    "doodads/shared/waves/waves02.nif",
    "doodads/shared/waves/waves03.nif",
    "doodads/temperate/temperatelakeripples/temperatelakeripples01.nif",
    "doodads/tropical/tropicalriverripples/tropicalriverripples01.nif",
    "doodads/tropical/tropicalriverripples/tropicalriverripples02.nif",
    "doodads/tropical/tropicalriverripples/tropicalriverripples03.nif",
];

/// The effect table's key when a placed model (a scene model key) is one of [`SHORE_MODELS`].
pub fn shore_model(model_key: &str) -> Option<String> {
    let lower = model_key.replace('\\', "/").to_ascii_lowercase();
    let at = lower.find("doodads/")?;
    let key = format!("{}.nif", lower[at..].trim_end_matches(".json").trim_end_matches(".nif"));
    SHORE_MODELS.contains(&key.as_str()).then_some(key)
}

/// The map's doodads at the water's edge, for the effects to play.
#[derive(Resource)]
pub struct ShoreDoodads {
    pub battle: u64,
    pub list: Vec<(String, Mat4)>,
}

impl ShoreDoodads {
    pub fn new(list: Vec<(String, Mat4)>) -> Self {
        static BATTLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self { battle: BATTLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed), list }
    }
}

/// What the battle holds of its water.
#[derive(Resource, Default)]
pub struct WaterView {
    /// Per material: its handle, its frames, seconds per frame, the frame shown.
    flips: Vec<(Handle<OriginalMaterial>, Vec<Handle<Image>>, f32, usize)>,
    batches: Vec<(Handle<Mesh>, Batch)>,
    /// Per material, the WaterInfo's curve.
    curves: Vec<Vec<[f32; 5]>>,
    clock: f32,
}

/// The meshes and materials of a battle's water. `order` is the layer after the terrain's.
pub fn spawn(commands: &mut Commands, water: Water, handles: &HashMap<(String, Kind), Handle<Image>>, order: usize, meshes: &mut Assets<Mesh>, materials: &mut Assets<OriginalMaterial>, marker: impl Bundle + Clone) {
    let mut view = WaterView::default();
    let mut made = Vec::with_capacity(water.materials.len());
    for m in &water.materials {
        let frames: Vec<Handle<Image>> = m.frames.iter().filter_map(|f| handles.get(&(f.clone(), Kind::Color)).cloned()).collect();
        let handle = materials.add(look::water_material(frames.first().cloned(), order));
        view.flips.push((handle.clone(), frames, 1.0 / m.fps.max(0.001), 0));
        view.curves.push(m.curve.clone());
        made.push(handle);
    }
    for batch in water.batches {
        let mesh = meshes.add(batch.mesh());
        commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(made[batch.material].clone()), bevy::light::NotShadowCaster, marker.clone()));
        view.batches.push((mesh, batch));
    }
    commands.insert_resource(view);
}

/// The cloud map's motion, repeats per second (`look::GroundMaps::cloud_motion`).
#[derive(Resource, Default, Clone, Copy)]
pub struct WaterScroll(pub Vec2);

/// The NiFlipController of 0x7377F0, looping, a frame per 1 / anim_fps, and the texture
/// translation 0x736D50. The translation changes the materials' uniform each frame; there are
/// as few of them as the map has water terrains.
pub fn flip(time: Res<Time>, view: Option<ResMut<WaterView>>, scroll: Option<Res<WaterScroll>>, mut materials: ResMut<Assets<OriginalMaterial>>) {
    let Some(mut view) = view else { return };
    view.clock += time.delta_secs();
    let clock = view.clock;
    let motion = scroll.map_or(Vec2::ZERO, |s| s.0);
    let offset = (motion * clock).fract_gl();
    for (handle, frames, seconds, shown) in &mut view.flips {
        let Some(mut m) = materials.get_mut(handle.id()) else { continue };
        if motion != Vec2::ZERO {
            m.base.uv_transform = bevy::math::Affine2::from_translation(offset);
        }
        if frames.is_empty() {
            continue;
        }
        let frame = (clock / *seconds) as usize % frames.len();
        if frame != *shown {
            *shown = frame;
            m.base.base_color_texture = Some(frames[frame].clone());
        }
    }
}

/// The ground changed under these terrain cells (`[x0, y0, x1, y1]`, from
/// `scene::follow_ground`): the surface and the depth of their vertices are the height
/// queries again.
pub fn follow_ground(view: &mut WaterView, native: &NativeTerrain, rects: &[[i32; 4]], meshes: &mut Assets<Mesh>) {
    let WaterView { batches, curves, .. } = view;
    for (mesh, batch) in batches {
        let mut touched = false;
        for span in &batch.spans {
            if !rects.iter().any(|r| (span.cell[0] as i32) >= r[0] && (span.cell[0] as i32) <= r[2] && (span.cell[1] as i32) >= r[1] && (span.cell[1] as i32) <= r[3]) {
                continue;
            }
            for i in span.first..span.first + span.count {
                let p = batch.positions[i as usize];
                let (z, color) = vertex(native, &curves[batch.material], p[0], -p[2]);
                batch.positions[i as usize][1] = z;
                batch.colors[i as usize] = color;
            }
            touched = true;
        }
        if touched {
            let _ = meshes.insert(mesh.id(), batch.mesh());
        }
    }
}

/// The saved cells of a map file (the TERRAIN chunk's auxiliary bytes). A generated map
/// has no file: its cells are all regular.
pub fn saved_cells_of(store: &AssetStore, map: &TerrainMap) -> Option<SavedCells> {
    let path = store.resolve_url(&format!("/assets/original/Data/{}", map.source)).ok()?;
    let archive = aa_mapworld::MapArchive::from_bytes(std::fs::read(path).ok()?).ok()?;
    let chunk = archive.decode_chunk("TERRAIN", 1 << 30).ok()?;
    let terrain = aa_mapworld::terrain::TerrainChunk::decode(&chunk).ok()?;
    saved_cells(&terrain.pending_auxiliary_bytes, map.width, map.height).map_err(|e| warn!("water: {} saved cells: {e:#}", map.source)).ok()
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
    struct Case {
        precision: Precision,
        points_bits: Vec<[u32; 5]>,
        depth_bits: u32,
        rgba_bits: [u32; 4],
    }
    #[derive(Deserialize)]
    struct Quad {
        cell: [usize; 2],
        cell_size_bits: u32,
        vertices_bits: [[u32; 2]; 4],
        indices: [u32; 6],
    }
    #[derive(Deserialize)]
    struct Selection {
        mask: u32,
        cell: [usize; 2],
        selected: Option<usize>,
    }

    fn vectors() -> Vectors {
        serde_json::from_str(include_str!("../../../research/phase6-water/native-water-vectors.json")).unwrap()
    }

    /// The original's 0x4B6E10 on every authored curve, at PC24, PC53 and PC64.
    #[test]
    fn depth_colours_equal_the_original() {
        for case in vectors().curve {
            let points: Vec<_> = case.points_bits.iter().map(|p| p.map(f32::from_bits)).collect();
            let got = depth_color(&points, f32::from_bits(case.depth_bits), case.precision).unwrap();
            assert_eq!(got.map(f32::to_bits), case.rgba_bits);
        }
        assert!(depth_color(&[], 0.0, Precision::Single24).is_none());
        assert!(depth_color(&[[0.0; 5]], f32::NAN, Precision::Single24).is_none());
    }

    /// The original's 0x7376B0: the vertices and indices of a regular cell.
    #[test]
    fn a_regular_cell_equals_the_original() {
        for case in vectors().regular_quads {
            assert_eq!(regular_cell(case.cell[0], case.cell[1], f32::from_bits(case.cell_size_bits)).map(|p| p.map(f32::to_bits)), case.vertices_bits);
            assert_eq!(REGULAR_INDICES, case.indices);
        }
    }

    /// The original's 0x738280: the cell itself, then the ring, on every 3x3 pattern.
    #[test]
    fn the_selected_cell_equals_the_original() {
        for case in vectors().selection {
            let got = selected_cell(case.cell, [3, 3], |[x, y]| case.mask & (1 << (y * 3 + x)) != 0);
            assert_eq!(got.map(|[x, y]| y * 3 + x), case.selected);
        }
    }

    #[test]
    fn saved_cells_keep_a_missing_cell_apart_from_an_empty_one() {
        let word = |out: &mut Vec<u8>, value: u32| out.extend(value.to_le_bytes());
        let mut bytes = vec![0; 5];
        word(&mut bytes, 0);
        word(&mut bytes, 3);
        for value in [0_f32, 0., 2., 0., 0., 2.] {
            word(&mut bytes, value.to_bits());
        }
        word(&mut bytes, 3);
        for value in [0, 1, 2] {
            word(&mut bytes, value);
        }
        word(&mut bytes, 2);
        for value in [0, 0, 0, 2, 1, 0, 4, 3] {
            word(&mut bytes, value);
        }
        let got = saved_cells(&bytes, 2, 1).unwrap();
        assert_eq!(got.cells[&[0, 0]], 0..3);
        assert!(got.cells[&[1, 0]].is_empty());
        assert!(!got.cells.contains_key(&[0, 1]));
    }
}
