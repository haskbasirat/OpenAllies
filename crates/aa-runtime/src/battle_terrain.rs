//! Owned original map ground surface for live placement/initialization.
//!
//! Height layouts come from authored SVars and MAP_HEADER world extents;
//! serialized resolution/count are checked witnesses. Terrain definitions and
//! retained biome defaults come from the native-order source registries.
//! Rendering metadata and converted viewer maps are never simulation inputs.
use crate::settings::{Declaration, Settings};
use aa_heightfield::binding::{GridLayout, LoadedHeightfield};
use aa_mapworld::{
    MapArchive, SourceIdentity,
    header::{MapHeader, WorldRules},
    terrain::{BoundTerrain, ResolvedVariation, TerrainChunk, TerrainGridLayout},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::{ActorCatalog, ResolvedNode},
    native::TypedValue,
    resources::Provenance,
    sources::V14Sources,
};
use aa_spatial::{Cell, TerrainGrid, height_layers::HeightLayers, index_world::GroundSurface};
use serde::Serialize;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "battle_terrain_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;
fn error(value: impl std::fmt::Display) -> String {
    value.to_string()
}
fn field(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<TypedValue> {
    node.field(name, pc)
        .map_err(error)?
        .ok_or_else(|| format!("native Terrain source field {name} is missing"))
}
fn string(value: TypedValue) -> Result<String> {
    match value {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err("native Terrain source requires a string field".into()),
    }
}
fn integer(settings: &Settings, name: &str) -> Result<(u32, Provenance)> {
    let entries = settings
        .declarations
        .get(&name.to_ascii_lowercase())
        .ok_or_else(|| format!("required authored SVar {name} is absent"))?;
    let [
        Declaration {
            field_type,
            raw,
            source,
        },
    ] = entries.as_slice()
    else {
        return Err(format!("{name} needs proven duplicate SVar override order"));
    };
    if !field_type
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("int"))
    {
        return Err(format!("{name} requires the native int declaration"));
    }
    let digits = raw.trim().strip_prefix('+').unwrap_or(raw.trim());
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "{name} requires a supported nonnegative native integer literal"
        ));
    }
    let value = digits.parse::<i32>().map_err(error)?;
    Ok((value as u32, source.clone()))
}

#[derive(Clone, Debug, Serialize)]
pub struct TerrainDefinition {
    pub id: String,
    pub native_type: u32,
    pub passability_bits: u8,
    /// Native+68/+6C are read only for MOUNTAIN; that branch requires both.
    pub noise_bits: Option<[u32; 2]>,
    ///4B7E10 ctor0;4B8966/4B8BAD load only MOUNTAIN/WATER into+5C.
    ///Kept outside the historical renderer/source metadata projection.
    #[serde(skip_serializing)]
    pub max_slope_bits: u32,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct VariationDefinition {
    pub id: String,
    pub terrain: usize,
    pub source: SourceIdentity,
    pub provenance: Provenance,
}

/// Once-per-source-bank terrain bindings. The immutable native registry owner
/// remains borrowed, so palette Find cannot silently use a different catalog.
pub struct Source<'a> {
    actors: &'a ActorCatalog,
    source_precision: Precision,
    pub height_resolution: u32,
    pub terrain_resolution: u32,
    pub resolution_sources: [Provenance; 2],
    pub files: BTreeMap<String, SourceIdentity>,
    pub terrains: Vec<TerrainDefinition>,
    pub variations: Vec<VariationDefinition>,
    variation_paths: BTreeMap<Vec<usize>, usize>,
    biome_initial: BTreeMap<Vec<usize>, usize>,
}
impl<'a> Source<'a> {
    pub fn bind(
        sources: &mut V14Sources,
        actors: &'a ActorCatalog,
        settings: &Settings,
        pc: Precision,
    ) -> Result<Self> {
        let terrain_registry = actors
            .native_data_registry("Terrain")
            .ok_or("native Terrain registry is unavailable")?;
        let variation_registry = actors
            .native_data_registry("Variation")
            .ok_or("native Variation registry is unavailable")?;
        let biome_registry = actors
            .native_data_registry("Biome")
            .ok_or("native Biome registry is unavailable")?;
        let (height_resolution, height_source) = integer(settings, "HeightmapResolutionBits")?;
        let (terrain_resolution, terrain_source) = integer(settings, "MapGTileResolutionBits")?;
        let mut files = BTreeMap::<String, SourceIdentity>::new();
        let mut pin = |provenance: &Provenance| -> Result<SourceIdentity> {
            let key = provenance.path.to_ascii_lowercase();
            if !files.contains_key(&key) {
                let file = sources.read(&provenance.path).map_err(error)?;
                if file.sha256 != provenance.sha256 {
                    return Err(format!("Terrain source owner changed: {}", provenance.path));
                }
                files.insert(key.clone(), SourceIdentity::of(&file.bytes));
            }
            let identity = &files[&key];
            if identity.sha256 != provenance.sha256 {
                return Err(format!("Terrain source hash differs: {}", provenance.path));
            }
            Ok(identity.clone())
        };
        pin(&height_source)?;
        pin(&terrain_source)?;
        for translation in actors.resolved_definitions("Translate").map_err(error)? {
            let manager = string(field(&translation, "node", pc)?)?;
            if ["Variation", "Terrain", "Biome"]
                .iter()
                .any(|name| manager.eq_ignore_ascii_case(name))
            {
                for item in translation.sources() {
                    pin(&item)?;
                }
            }
        }
        let mut terrains = vec![];
        let mut terrain_paths = BTreeMap::new();
        for entry in &terrain_registry.entries {
            let node = actors.native_data_node(entry).map_err(error)?;
            let kind = string(field(&node, "type", pc)?)?;
            let native_type = match kind.to_ascii_uppercase().as_str() {
                "LAND" => 0,
                "MOUNTAIN" => 1,
                "WATER" => 2,
                _ => return Err(format!("unknown native Terrain type {kind}")),
            };
            let land = field(&node, "land", pc)?.boolean().map_err(error)?;
            let water = field(&node, "water", pc)?.boolean().map_err(error)?;
            let noise_bits = if native_type == 1 {
                Some([
                    field(&node, "noise_min", pc)?.fixed_bits().map_err(error)?,
                    field(&node, "noise_max", pc)?.fixed_bits().map_err(error)?,
                ])
            } else {
                None
            };
            let provenance = node.sources();
            for item in &provenance {
                pin(item)?;
            }
            terrain_paths.insert(entry.path.clone(), terrains.len());
            terrains.push(TerrainDefinition {
                id: entry.ids.clone(),
                native_type,
                passability_bits: u8::from(land) | (u8::from(water) << 1),
                noise_bits,
                max_slope_bits: if native_type==0 {0}else{field(&node,"max_slope",pc)?.fixed_bits().map_err(error)?},
                sources: provenance,
            });
        }
        let mut variations = vec![];
        let mut variation_paths = BTreeMap::new();
        for entry in &variation_registry.entries {
            actors.native_data_node(entry).map_err(error)?;
            let [root, _child] = entry.path.as_slice() else {
                return Err(format!(
                    "Variation {} has no actual Terrain parent",
                    entry.ids
                ));
            };
            let terrain = *terrain_paths
                .get(&vec![*root])
                .ok_or_else(|| format!("Variation {} parent Terrain is unavailable", entry.ids))?;
            let source = pin(&entry.source)?;
            variation_paths.insert(entry.path.clone(), variations.len());
            variations.push(VariationDefinition {
                id: entry.ids.clone(),
                terrain,
                source,
                provenance: entry.source.clone(),
            });
        }
        let mut biome_initial = BTreeMap::new();
        for biome in &biome_registry.entries {
            let node = actors.native_data_node(biome).map_err(error)?;
            for item in node.sources() {
                pin(&item)?;
            }
            let children = node.children("Terrain").map_err(error)?;
            let first = children
                .first()
                .ok_or_else(|| format!("Biome {} has no native initial Terrain", biome.ids))?;
            let id = string(field(first, "terrain_IDS", pc)?)?;
            //Biome+78 and Terrain+30 retain pointers created during source
            //loading. Do not translate those retained objects a second time.
            let terrain=terrain_registry.entries.iter().find(|entry|entry.ids.eq_ignore_ascii_case(&id)).ok_or_else(||format!("Biome {} initial Terrain reference {id} requires unavailable source-order alias resolution",biome.ids))?;
            if terrain.path.first() >= biome.path.first() {
                return Err(format!(
                    "Biome {} initial Terrain was not registered before its load",
                    biome.ids
                ));
            }
            let first_variation = variation_registry
                .entries
                .iter()
                .filter(|variation| {
                    variation.path.len() == 2 && variation.path[0] == terrain.path[0]
                })
                .min_by_key(|variation| variation.path[1])
                .ok_or_else(|| format!("Terrain {} has no initial Variation", terrain.ids))?;
            biome_initial.insert(biome.path.clone(), variation_paths[&first_variation.path]);
        }
        drop(pin);
        Ok(Self {
            actors,
            source_precision: pc,
            height_resolution,
            terrain_resolution,
            resolution_sources: [height_source, terrain_source],
            files,
            terrains,
            variations,
            variation_paths,
            biome_initial,
        })
    }
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
    fn resolve(&self, id: &[u16]) -> Result<Option<(usize, ResolvedVariation)>> {
        let id_string = String::from_utf16(id).map_err(error)?;
        let Some(entry) = self.actors.native_data_find("Variation", &id_string) else {
            return Ok(None);
        };
        let index = *self
            .variation_paths
            .get(&entry.path)
            .ok_or("resolved Variation has no retained source object")?;
        Ok(Some((index, self.resolved(index, id.to_vec()))))
    }
    fn resolved(&self, index: usize, id_utf16: Vec<u16>) -> ResolvedVariation {
        let variation = &self.variations[index];
        let terrain = &self.terrains[variation.terrain];
        ResolvedVariation {
            id_utf16,
            native_type: terrain.native_type,
            passability_bits: terrain.passability_bits,
            source: variation.source.clone(),
        }
    }
    fn initial(&self, biome: &[u16]) -> Result<(usize, ResolvedVariation)> {
        let name = String::from_utf16(biome).map_err(error)?;
        let entry = self
            .actors
            .native_data_find("Biome", &name)
            .ok_or_else(|| format!("unknown native Biome {name}"))?;
        let index = *self
            .biome_initial
            .get(&entry.path)
            .ok_or("Biome retained initial Variation is unavailable")?;
        Ok((
            index,
            self.resolved(index, self.variations[index].id.encode_utf16().collect()),
        ))
    }
}

///GMap+5C helper+90, initialized by55AAF0. The current ordinary loaded-map
///path has no552010 RMG/editor cache-build call; no generic dirty setter exists.
pub struct MapNotification { dirty: bool }
impl MapNotification { pub fn dirty(&self)->bool { self.dirty } }

pub struct BattleTerrain {
    source_precision: Precision,
    pub base: LoadedHeightfield,
    pub overlay: LoadedHeightfield,
    pub terrain: TerrainChunk,
    pub bound: BoundTerrain,
    pub grid: TerrainGrid,
    effective_variations: Vec<usize>,
    source_variations: Vec<VariationDefinition>,
    source_terrains: Vec<TerrainDefinition>,
    pub resolution_sources: [Provenance; 2],
    world_extents_bits: [u32;2],
    height_brush: aa_heightfield::brush::State,
    map_notification: MapNotification,
}
impl BattleTerrain {
    pub fn load(archive: &MapArchive, source: &Source<'_>) -> Result<Self> {
        let header = MapHeader::decode(
            &archive
                .decode_chunk("MAP_HEADER", u64::MAX)
                .map_err(error)?,
        )
        .map_err(error)?;
        let rules = WorldRules::decode_bytes(
            &archive
                .decode_chunk("RULES", u64::MAX)
                .map_err(error)?
                .bytes,
        )
        .map_err(error)?;
        for rule in rules
            .rules
            .iter()
            .chain(&header.rules.rules)
            .chain(&header.creator.rules.rules)
        {
            let key = String::from_utf16(&rule.key).map_err(error)?;
            if ["HeightmapResolutionBits", "MapGTileResolutionBits"]
                .iter()
                .any(|name| key.eq_ignore_ascii_case(name))
            {
                return Err(format!(
                    "map rule {key} requires the native SVar override policy"
                ));
            }
        }
        let layout = GridLayout::from_world_extents(
            header.creator.world_extents_bits,
            source.height_resolution,
        )
        .map_err(error)?;
        let base = LoadedHeightfield::decode(
            layout,
            &archive
                .decode_chunk("HEIGHT_MAP", u64::MAX)
                .map_err(error)?
                .bytes,
        )
        .map_err(error)?;
        let overlay = LoadedHeightfield::decode(
            layout,
            &archive
                .decode_chunk("MOUNTAIN_HEIGHT_MAP", u64::MAX)
                .map_err(error)?
                .bytes,
        )
        .map_err(error)?;
        let terrain =
            TerrainChunk::decode(&archive.decode_chunk("TERRAIN", u64::MAX).map_err(error)?)
                .map_err(error)?;
        if terrain.resolution_bits != source.terrain_resolution {
            return Err(
                "serialized terrain resolution differs from authored MapGTileResolutionBits".into(),
            );
        }
        let terrain_layout = TerrainGridLayout::from_world_extents(
            header.creator.world_extents_bits,
            source.terrain_resolution,
        )
        .map_err(error)?;
        let (initial_index, initial) = source.initial(&terrain.biome_utf16)?;
        let resolved = terrain
            .palette_utf16
            .iter()
            .map(|id| source.resolve(id))
            .collect::<Result<Vec<_>>>()?;
        let palette = resolved
            .iter()
            .map(|entry| entry.as_ref().map(|(_, definition)| definition.clone()))
            .collect::<Vec<_>>();
        let bound = terrain
            .bind_with_native_fallback(terrain_layout, &palette, initial)
            .map_err(error)?;
        let mut effective_variations = Vec::new();
        let mut cells = Vec::new();
        effective_variations
            .try_reserve_exact(bound.cells.len())
            .map_err(error)?;
        cells.try_reserve_exact(bound.cells.len()).map_err(error)?;
        for cell in &bound.cells {
            let index = match cell.resolved_palette_index {
                None => initial_index,
                Some(i) => resolved[i as usize]
                    .as_ref()
                    .map(|(index, _)| *index)
                    .ok_or("effective native Variation is missing")?,
            };
            let definition = &source.terrains[source.variations[index].terrain];
            //Non-mountain branches never read +68/+6C; these unused Cell
            //payload words are zero, not a guessed mountain-noise default.
            let noise = definition.noise_bits.unwrap_or([0, 0]);
            if definition.native_type == 1 && definition.noise_bits.is_none() {
                return Err("mountain noise source is unavailable".into());
            }
            cells.push(Cell {
                overlay_kind: definition.native_type as i32,
                offset_min: noise[0],
                offset_max: noise[1],
            });
            effective_variations.push(index);
        }
        let grid = TerrainGrid {
            width: terrain_layout.width,
            height: terrain_layout.height,
            coordinate_shift: terrain_layout.coordinate_shift,
            cell_size: terrain_layout.cell_size_bits,
            cells,
        };
        grid.validate().map_err(error)?;
        HeightLayers::new(&base.state.field, &overlay.state.field).map_err(error)?;
        Ok(Self {
            source_precision: source.source_precision,
            base,
            overlay,
            terrain,
            bound,
            grid,
            effective_variations,
            source_variations: source.variations.clone(),
            source_terrains: source.terrains.clone(),
            resolution_sources: source.resolution_sources.clone(),
            world_extents_bits: header.creator.world_extents_bits,
            height_brush: aa_heightfield::brush::State::from_height_map_constructor(),
            map_notification: MapNotification { dirty: false },
        })
    }
    pub fn load_with_sources(
        archive: &MapArchive,
        sources: &mut V14Sources,
        actors: &ActorCatalog,
        settings: &Settings,
        pc: Precision,
    ) -> Result<Self> {
        Self::load(archive, &Source::bind(sources, actors, settings, pc)?)
    }
    pub fn surface(&self) -> aa_spatial::Result<GroundSurface<'_>> {
        Ok(GroundSurface {
            terrain: &self.grid,
            heights: HeightLayers::new(&self.base.state.field, &self.overlay.state.field)?,
        })
    }
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
    ///Read the same resolved VariationData identity retained by the loaded
    ///tile. Rendering may snapshot this value; it must not reselect a biome
    ///default or restore a disabled serialized palette alias.
    pub fn variation_definition_at(&self, x:i32, y:i32)->Result<&VariationDefinition>{
        if x<0||y<0||x>=self.bound.layout.width||y>=self.bound.layout.height{
            return Err("native variation cell is outside the loaded map".into());
        }
        let offset=(y as usize).checked_mul(self.bound.layout.width as usize)
            .and_then(|n|n.checked_add(x as usize)).ok_or("native variation cell offset overflows")?;
        let index=*self.effective_variations.get(offset).ok_or("actual tile Variation identity is unavailable")?;
        self.source_variations.get(index).ok_or("actual Variation source owner is unavailable".into())
    }
    /// Actual loaded tile VariationData+1C -> TerrainData. This reads the
    /// retained source definition, independently of runtime tile flags20.
    /// Definition/variation identities are private owned bindings and cannot
    /// be swapped for a different source bank through this API.
    pub fn terrain_definition_at(&self, x: i32, y: i32) -> Result<&TerrainDefinition> {
        if x < 0 || y < 0 || x >= self.bound.layout.width || y >= self.bound.layout.height {
            return Err("native terrain definition cell is outside the loaded map".into());
        }
        let offset = (y as usize)
            .checked_mul(self.bound.layout.width as usize)
            .and_then(|n| n.checked_add(x as usize))
            .ok_or("native terrain cell offset overflows")?;
        let variation = *self
            .effective_variations
            .get(offset)
            .ok_or("actual tile Variation identity is unavailable")?;
        let definition = self
            .source_variations
            .get(variation)
            .ok_or("actual Variation source owner is unavailable")?;
        self.source_terrains
            .get(definition.terrain)
            .ok_or("actual TerrainData source owner is unavailable".into())
    }
    pub fn passability_bits_at(&self, x: i32, y: i32) -> Result<u8> {
        Ok(self.terrain_definition_at(x, y)?.passability_bits)
    }
    ///Disjoint synchronous loans of the SAME mutable HeightMap and actual
    ///TerrainData lookup owners. No height/sample or tile table is copied.
    pub(crate) fn height_brush_parts(&mut self)->(
        &mut aa_heightfield::binding::InitializedHeightfield,
        &mut aa_heightfield::brush::State,HeightBrushTerrain<'_>,
    ){
        (&mut self.base.state,&mut self.height_brush,HeightBrushTerrain{
            notification:&self.map_notification,extents:self.world_extents_bits,layout:&self.bound.layout,
            effective:&self.effective_variations,variations:&self.source_variations,terrains:&self.source_terrains})
    }
    pub fn height_dirty_bounds(&self)->[u32;4]{self.height_brush.dirty_bounds()}
    pub fn map_notification(&self)->&MapNotification { &self.map_notification }
}

pub(crate) struct HeightBrushTerrain<'a>{pub(crate) notification:&'a MapNotification,extents:[u32;2],layout:&'a TerrainGridLayout,
    effective:&'a [usize],variations:&'a [VariationDefinition],terrains:&'a [TerrainDefinition]}
impl HeightBrushTerrain<'_>{
    ///558A70 actual4282A0 bounds, native truncation/shift then retained
    ///Variation+1C ->Terrain+5C. Outside-map queries retain the caller slope.
    pub(crate) fn slope(&self,xy:[u32;2])->Result<Option<u32>>{
        use std::cmp::Ordering;
        let values=xy.map(aa_numeric::Binary::from_f32_bits);
        let [x,y]=values;let(x,y)=(x.map_err(error)?,y.map_err(error)?);
        if x.is_negative_nonzero()||y.is_negative_nonzero()
            ||x.compare(&aa_numeric::Binary::from_f32_bits(self.extents[0]).map_err(error)?)!=Ordering::Less
            ||y.compare(&aa_numeric::Binary::from_f32_bits(self.extents[1]).map_err(error)?)!=Ordering::Less{return Ok(None)}
        let shift=self.layout.coordinate_shift&31;
        let cell=[x.truncate_i32().map_err(error)?>>shift,y.truncate_i32().map_err(error)?>>shift];
        if cell[0]<0||cell[1]<0||cell[0]>=self.layout.width||cell[1]>=self.layout.height{return Ok(None)}
        let index=(cell[1]as usize)*(self.layout.width as usize)+cell[0]as usize;
        let variation=*self.effective.get(index).ok_or("558A70 actual tile Variation absent")?;
        let v=self.variations.get(variation).ok_or("558A70 actual Variation source absent")?;
        Ok(Some(self.terrains.get(v.terrain).ok_or("558A70 actual Terrain source absent")?.max_slope_bits))
    }
}
