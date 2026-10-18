//! Read-only original water/ground sampling over bound RMP heightfield chunks.
use aa_assets::{TerrainMap, TerrainMetadata};
use aa_heightfield::{
    Heightfield,
    binding::{GridLayout, LoadedHeightfield},
};
use aa_mapworld::{MapArchive, header::MapHeader};
use aa_numeric::{Binary, Precision};
use aa_spatial::{
    Actor, Cell, HeightLayer, HeightMethod, ShapeKind, ShapePlacement, TerrainGrid, World,
};
use anyhow::{Context, Result, ensure};

pub struct Surface {
    base: Heightfield,
    overlay: Heightfield,
    grid: TerrainGrid,
    pub effective_variations: Vec<String>,
    pub biome: String,
    pub carried_cells: usize,
}
impl Surface {
    pub fn load(
        store: &aa_assets::AssetStore,
        archive: &MapArchive,
        map: &TerrainMap,
        terrain: &aa_mapworld::terrain::TerrainChunk,
        decode_budget: u64,
    ) -> Result<Self> {
        use aa_mapworld::terrain::{ResolvedVariation, TerrainGridLayout};
        use std::collections::BTreeMap;
        #[derive(serde::Deserialize)]
        struct Definition {
            canonical: String,
            terrain_template: String,
            land: bool,
            water: bool,
            path: String,
            sha256: String,
        }
        #[derive(serde::Deserialize)]
        struct DefaultVariation {
            variation_id: String,
        }
        #[derive(serde::Deserialize)]
        struct Definitions {
            variants: BTreeMap<String, Definition>,
            biome_defaults: BTreeMap<String, DefaultVariation>,
        }
        let definitions: Definitions = serde_json::from_str(include_str!(
            "../../../research/phase6-terrain/terrain-render-metadata.json"
        ))?;
        let header = MapHeader::decode(&archive.decode_chunk("MAP_HEADER", decode_budget)?)?;
        let mut layers = Vec::new();
        for name in ["HEIGHT_MAP", "MOUNTAIN_HEIGHT_MAP"] {
            let chunk = archive.decode_chunk(name, decode_budget)?;
            let resolution = u32::from_le_bytes(
                chunk
                    .bytes
                    .get(16..20)
                    .context("Height resolution missing")?
                    .try_into()?,
            );
            let layout =
                GridLayout::from_world_extents(header.creator.world_extents_bits, resolution)?;
            let field = LoadedHeightfield::decode(layout, &chunk.bytes)?.state.field;
            ensure!(
                field.width == map.width as i32 + 1
                    && field.height == map.height as i32 + 1
                    && field.cell_size_bits == map.cell_size.to_bits(),
                "Water source height layout differs from preview map"
            );
            layers.push(field);
        }
        let overlay = layers.pop().context("Overlay missing")?;
        let base = layers.pop().context("Base missing")?;
        let metadata = TerrainMetadata::bundled()?;
        let mut sources = BTreeMap::new();
        let mut resolve = |id: &[u16]| -> Result<Option<ResolvedVariation>> {
            let name = String::from_utf16(id)?;
            let Some(v) = definitions.variants.get(&name) else {
                return Ok(None);
            };
            let canonical = metadata
                .variations
                .get(&v.canonical)
                .context("Native terrain canonical definition missing")?;
            let kind = match v.terrain_template.as_str() {
                "terrain" => 0,
                "mountainterrain" => 1,
                "waterterrain" => 2,
                _ => anyhow::bail!("Unresolved native terrain template"),
            };
            ensure!(
                canonical.native_type as u32 == kind,
                "Terrain metadata native type mismatch"
            );
            if !sources.contains_key(&v.path) {
                let path = store.resolve_url(&format!("/assets/original/Data/{}", v.path))?;
                let identity = aa_mapworld::SourceIdentity::of(&std::fs::read(path)?);
                ensure!(
                    identity.sha256 == v.sha256,
                    "Water terrain source changed: {}",
                    v.path
                );
                sources.insert(v.path.clone(), identity);
            }
            Ok(Some(ResolvedVariation {
                id_utf16: id.to_vec(),
                native_type: kind,
                passability_bits: u8::from(v.land) | (u8::from(v.water) << 1),
                source: sources[&v.path].clone(),
            }))
        };
        let biome = String::from_utf16(&terrain.biome_utf16)?;
        let initial_name = &definitions
            .biome_defaults
            .get(&biome)
            .context("Native biome initial terrain missing")?
            .variation_id;
        let initial = resolve(&initial_name.encode_utf16().collect::<Vec<_>>())?
            .context("Initial terrain variation unresolved")?;
        let palette = terrain
            .palette_utf16
            .iter()
            .map(|id| resolve(id))
            .collect::<Result<Vec<_>>>()?;
        let layout = TerrainGridLayout::from_world_extents(
            header.creator.world_extents_bits,
            terrain.resolution_bits,
        )?;
        ensure!(
            layout.width == map.width as i32
                && layout.height == map.height as i32
                && layout.cell_size_bits == map.cell_size.to_bits(),
            "Original water tile layout differs from preview"
        );
        let bound = terrain.bind_with_native_fallback(layout, &palette, initial)?;
        let effective_variations = bound
            .cells
            .iter()
            .map(|cell| {
                cell.resolved_palette_index.map_or_else(
                    || Ok(initial_name.clone()),
                    |index| String::from_utf16(&terrain.palette_utf16[index as usize]),
                )
            })
            .collect::<std::result::Result<Vec<_>, std::string::FromUtf16Error>>()?;
        let cells = effective_variations
            .iter()
            .map(|id| -> Result<Cell> {
                let resolved = &definitions.variants[id];
                let native = metadata
                    .variations
                    .get(&resolved.canonical)
                    .context("Native terrain noise metadata missing")?;
                let (min, max) = if native.native_type == 1 {
                    (
                        native.noise_min.context("Native noise minimum missing")?,
                        native.noise_max.context("Native noise maximum missing")?,
                    )
                } else {
                    (0.0, 0.0)
                };
                Ok(Cell {
                    overlay_kind: native.native_type.into(),
                    offset_min: min.to_bits(),
                    offset_max: max.to_bits(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let grid = TerrainGrid {
            width: layout.width,
            height: layout.height,
            coordinate_shift: layout.coordinate_shift,
            cell_size: layout.cell_size_bits,
            cells,
        };
        grid.validate()?;
        Ok(Self {
            base,
            overlay,
            grid,
            effective_variations,
            biome,
            carried_cells: bound.missing_variation_ordinals.len(),
        })
    }

    /// Original736AB0 uses the unstored550210 return minus0.5, then stores Z;
    /// it subtracts the complete4F0E50(flags0x122) result before storing depth.
    pub fn sample(&self, xy: [f32; 2], precision: Precision) -> Result<(f32, f32)> {
        let bits = xy.map(f32::to_bits);
        let height = self
            .base
            .sample_interpolated(bits, precision)?
            .sub(&Binary::from_f32_bits(0x3f000000)?, precision)?
            .store_f32()?;
        let layers = aa_spatial::height_layers::HeightLayers::new(&self.base, &self.overlay)?;
        let mut world = HeightOnly(layers);
        let mut actor = Actor {
            flags: 0,
            position: [bits[0], bits[1], 0],
            facing: [0, 0],
        };
        let ground =
            aa_spatial::query_actor_height(&mut actor, 0x122, &self.grid, precision, &mut world)?;
        let depth = Binary::from_f32_bits(height)?
            .sub(&ground, precision)?
            .store_f32()?;
        Ok((f32::from_bits(height), f32::from_bits(depth)))
    }
}
struct HeightOnly<'a>(aa_spatial::height_layers::HeightLayers<'a>);
impl World for HeightOnly<'_> {
    fn sample_height(
        &mut self,
        _: &mut Actor,
        layer: HeightLayer,
        method: HeightMethod,
        xy: [u32; 2],
        precision: Precision,
    ) -> aa_spatial::Result<Binary> {
        self.0.sample(layer, method, xy, precision)
    }
    fn shape(&mut self, _: &mut Actor, _: ShapeKind) -> aa_spatial::Result<u32> {
        Err(aa_spatial::Error::World(
            "Water height reader cannot query actor shapes".into(),
        ))
    }
    fn update_tiles(
        &mut self,
        _: &mut Actor,
        _: ShapePlacement,
        _: ShapePlacement,
        _: ShapePlacement,
        _: ShapePlacement,
    ) -> aa_spatial::Result<()> {
        Err(aa_spatial::Error::World(
            "Water height reader cannot mutate tiles".into(),
        ))
    }
    fn position_changed(&mut self, _: &mut Actor) -> aa_spatial::Result<()> {
        Err(aa_spatial::Error::World(
            "Water height reader cannot mutate actors".into(),
        ))
    }
    fn world_phase(&self) -> u32 {
        0
    }
    fn presentation_available(&self) -> bool {
        false
    }
    fn presentation_changed(&mut self, _: &mut Actor) -> aa_spatial::Result<()> {
        Err(aa_spatial::Error::World(
            "Water height reader cannot notify actors".into(),
        ))
    }
}
