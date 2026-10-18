//! Original TERRAIN/GMap v6/v7 tile input and static passability binding.
//!
//! These are simulation tiles, not the renderer's composited terrain texture.
//! Region/auxiliary suffixes and dynamic object membership remain explicit.
use crate::{Chunk, Error, Result, SourceIdentity, inventory::Reader};
use aa_numeric::Binary;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainGridLayout {
    pub extent_i32: [i32; 2],
    pub width: i32,
    pub height: i32,
    pub coordinate_shift: u32,
    pub cell_size_bits: u32,
}
impl TerrainGridLayout {
    /// Original5565E1..556665. Supply the evaluated MapGTileResolutionBits;
    /// this is independent of HeightmapResolutionBits and has no final corner.
    pub fn from_world_extents(bits: [u32; 2], resolution: u32) -> Result<Self> {
        if resolution > 30 {
            return Err(Error::Format("unsupported signed terrain cell size".into()));
        }
        let ceil = |bits| -> Result<i32> {
            let value = Binary::from_f32_bits(bits).map_err(|e| Error::Format(e.to_string()))?;
            if value.is_negative_nonzero() {
                return Err(Error::Format("negative terrain extent".into()));
            }
            let n = value
                .truncate_i32()
                .map_err(|e| Error::Format(e.to_string()))?;
            if value.compare(&Binary::from_i32(n)) == Ordering::Greater {
                n.checked_add(1)
                    .ok_or_else(|| Error::Format("terrain extent overflow".into()))
            } else {
                Ok(n)
            }
        };
        let extent_i32 = [ceil(bits[0])?, ceil(bits[1])?];
        let out = Self {
            extent_i32,
            width: extent_i32[0] >> resolution,
            height: extent_i32[1] >> resolution,
            coordinate_shift: resolution,
            cell_size_bits: (127 + resolution) << 23,
        };
        out.tile_count()?;
        Ok(out)
    }
    pub fn tile_count(self) -> Result<usize> {
        if self.coordinate_shift > 30
            || self.extent_i32.iter().any(|&v| v < 0)
            || self.width != self.extent_i32[0] >> self.coordinate_shift
            || self.height != self.extent_i32[1] >> self.coordinate_shift
            || self.cell_size_bits != (127 + self.coordinate_shift) << 23
            || self.width <= 0
            || self.height <= 0
        {
            return Err(Error::Format("invalid terrain grid layout".into()));
        }
        let count = self
            .width
            .checked_mul(self.height)
            .filter(|&n| n > 0 && (n as u64) * 48 <= u64::from(u32::MAX))
            .ok_or_else(|| Error::Format("original terrain allocation extent overflows".into()))?;
        Ok(count as usize)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializedTile {
    /// Native v6 six-byte records are expanded to the v7 eight-byte structure.
    /// Native conversion clears unused v6 bytes. The source bytes remain in
    /// TerrainChunk.serialized_tile_bytes; no ownership meaning is inferred.
    pub expanded: [u8; 8],
}
impl SerializedTile {
    pub fn variation_index(&self) -> u16 {
        u16::from_le_bytes([self.expanded[0], self.expanded[1]])
    }
    pub fn explored_mask(&self) -> u16 {
        u16::from_le_bytes([self.expanded[2], self.expanded[3]])
    }
    pub fn byte_2c(&self) -> u8 {
        self.expanded[4]
    }
    pub fn saved_flags(&self) -> u8 {
        self.expanded[6]
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerrainChunk {
    pub map_source: SourceIdentity,
    pub source: SourceIdentity,
    pub version: u32,
    pub resolution_bits: u32,
    pub biome_utf16: Vec<u16>,
    pub palette_utf16: Vec<Vec<u16>>,
    pub serialized_stride: u32,
    pub tile_data_offset: usize,
    pub serialized_tile_bytes: Vec<u8>,
    pub tiles: Vec<SerializedTile>,
    pub suffix_offset: usize,
    /// Original7AA9B0 and the GMap+5C subtype load follow these tile records.
    /// Their bytes are retained, not treated as decoded runtime regions.
    pub pending_auxiliary_bytes: Vec<u8>,
}
impl TerrainChunk {
    pub fn decode(chunk: &Chunk) -> Result<Self> {
        chunk.verify()?;
        if chunk.directory.name != "TERRAIN" {
            return Err(Error::Format("incorrect TERRAIN chunk identity".into()));
        }
        let mut r = Reader::new(&chunk.bytes);
        let version = r.version(7)?;
        if !matches!(version, 6 | 7) {
            return Err(Error::Format(
                "older terrain load requires its legacy world side effects".into(),
            ));
        }
        let resolution_bits = r.u32()?;
        let biome_utf16 = r.string()?;
        let palette_count = r.u32()? as usize;
        if palette_count > i32::MAX as usize || palette_count > (r.bytes.len() - r.at) / 2 {
            return Err(Error::Format(
                "terrain palette exceeds source extent".into(),
            ));
        }
        let mut palette_utf16 = Vec::new();
        palette_utf16
            .try_reserve_exact(palette_count)
            .map_err(|_| Error::Allocation)?;
        for _ in 0..palette_count {
            palette_utf16.push(r.string()?);
        }
        let tile_count = r.u32()? as usize;
        let serialized_stride = r.u32()?;
        if serialized_stride != if version == 6 { 6 } else { 8 } {
            return Err(Error::Format(
                "terrain record stride disagrees with version".into(),
            ));
        }
        let tile_data_offset = r.at;
        let bytes = tile_count
            .checked_mul(serialized_stride as usize)
            .ok_or_else(|| Error::Format("terrain record extent overflows".into()))?;
        let raw = r.take(bytes)?;
        let mut tiles = Vec::new();
        tiles
            .try_reserve_exact(tile_count)
            .map_err(|_| Error::Allocation)?;
        for bytes in raw.chunks_exact(serialized_stride as usize) {
            let expanded = if version == 7 {
                bytes.try_into().unwrap()
            } else {
                // Original557C67..557C93: ID byte, flag byte, region u16,
                // byte2C and one unused byte -> zero-initialized native8.
                [bytes[0], 0, bytes[2], bytes[3], bytes[4], 0, bytes[1], 0]
            };
            let tile = SerializedTile { expanded };
            if usize::from(tile.variation_index()) >= palette_count {
                return Err(Error::Format(
                    "terrain variation index exceeds palette".into(),
                ));
            }
            tiles.push(tile);
        }
        Ok(Self {
            map_source: chunk.map_source.clone(),
            source: chunk.payload_source.clone(),
            version,
            resolution_bits,
            biome_utf16,
            palette_utf16,
            serialized_stride,
            tile_data_offset,
            serialized_tile_bytes: raw.to_vec(),
            tiles,
            suffix_offset: r.at,
            pending_auxiliary_bytes: r.bytes[r.at..].to_vec(),
        })
    }
    /// Bind a source-ordered palette. None retains a failed registry lookup and
    /// is allowed only for unused entries. Native missing-ID fallback for a
    /// referenced entry depends on prior cell/biome state; that case fails here
    /// rather than silently selecting a terrain or clearing obstacles.
    pub fn bind(
        &self,
        layout: TerrainGridLayout,
        palette: &[Option<ResolvedVariation>],
    ) -> Result<BoundTerrain> {
        self.bind_inner(layout, palette, None)
    }
    /// Original557D4B..557E37 carries the last resolved variation across the
    /// source-order tile loop when a palette lookup is null. The first fallback
    /// starts at biome+78->terrain+30, supplied explicitly by the caller.
    /// Missing-variation diagnostics are reported as source ordinals here.
    pub fn bind_with_native_fallback(
        &self,
        layout: TerrainGridLayout,
        palette: &[Option<ResolvedVariation>],
        initial: ResolvedVariation,
    ) -> Result<BoundTerrain> {
        self.bind_inner(layout, palette, Some(initial))
    }
    fn bind_inner(
        &self,
        layout: TerrainGridLayout,
        palette: &[Option<ResolvedVariation>],
        initial: Option<ResolvedVariation>,
    ) -> Result<BoundTerrain> {
        if layout.tile_count()? != self.tiles.len()
            || layout.coordinate_shift != self.resolution_bits
        {
            return Err(Error::Format(
                "TERRAIN differs from constructed map grid".into(),
            ));
        }
        if palette.len() != self.palette_utf16.len() {
            return Err(Error::Format(
                "resolved terrain palette length differs".into(),
            ));
        }
        for (id, definition) in self.palette_utf16.iter().zip(palette) {
            let Some(definition) = definition else {
                continue;
            };
            if &definition.id_utf16 != id
                || definition.native_type > 2
                || definition.passability_bits & !3 != 0
            {
                return Err(Error::Format(
                    "invalid or mismatched terrain definition".into(),
                ));
            }
        }
        if initial
            .as_ref()
            .is_some_and(|d| d.native_type > 2 || d.passability_bits & !3 != 0)
        {
            return Err(Error::Format("invalid initial terrain definition".into()));
        }
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(self.tiles.len())
            .map_err(|_| Error::Allocation)?;
        let mut previous = initial.as_ref().map(|d| (d, None));
        let mut missing_variation_ordinals = Vec::new();
        for (ordinal, tile) in self.tiles.iter().enumerate() {
            let entry = palette
                .get(usize::from(tile.variation_index()))
                .ok_or_else(|| Error::Format("terrain variation index exceeds palette".into()))?;
            let (definition, resolved_palette_index) = if let Some(d) = entry {
                (d, Some(tile.variation_index()))
            } else if initial.is_some() {
                missing_variation_ordinals
                    .try_reserve(1)
                    .map_err(|_| Error::Allocation)?;
                missing_variation_ordinals.push(ordinal);
                previous
                    .ok_or_else(|| Error::Format("missing initial terrain definition".into()))?
            } else {
                return Err(Error::Format(
                    "referenced terrain variation is unresolved".into(),
                ));
            };
            previous = Some((definition, resolved_palette_index));
            let flags = (u32::from(tile.saved_flags()) & !0x70) | definition.native_type << 4;
            cells.push(TerrainCell {
                variation_index: tile.variation_index(),
                resolved_palette_index,
                flags20: recompute_passability(flags, definition.passability_bits),
                byte_2c: tile.byte_2c(),
                saved_explored_mask: tile.explored_mask(),
            });
        }
        Ok(BoundTerrain {
            layout,
            source: self.source.clone(),
            cells,
            initial_variation: initial,
            missing_variation_ordinals,
            palette_sources: palette
                .iter()
                .map(|p| p.as_ref().map(|p| p.source.clone()))
                .collect(),
        })
    }
}

/// Typed inputs from original terrain definitions, not inferred from model,
/// texture or type names. `land` is bit0 and `water` is bit1 (native4B87A0).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedVariation {
    pub id_utf16: Vec<u16>,
    pub native_type: u32,
    pub passability_bits: u8,
    pub source: SourceIdentity,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainCell {
    /// Source palette ID, even when the original missing-ID fallback is used.
    pub variation_index: u16,
    /// Effective palette entry. None selects BoundTerrain.initial_variation.
    pub resolved_palette_index: Option<u16>,
    pub flags20: u32,
    pub byte_2c: u8,
    /// Saved low16 of tile+14. Native557E6E..8C applies it only for version>=4
    /// and stream59E520 true;54EA50 preserves the high16 visibility bits. This
    /// is an explored mask, never a tile+10 topology rectangle/region identity.
    #[serde(alias = "saved_region_reference")]
    pub saved_explored_mask: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundTerrain {
    pub layout: TerrainGridLayout,
    pub source: SourceIdentity,
    pub palette_sources: Vec<Option<SourceIdentity>>,
    pub cells: Vec<TerrainCell>,
    pub initial_variation: Option<ResolvedVariation>,
    pub missing_variation_ordinals: Vec<usize>,
}
impl BoundTerrain {
    pub fn cell(&self, x: i32, y: i32) -> Option<&TerrainCell> {
        if x < 0 || y < 0 || x >= self.layout.width || y >= self.layout.height {
            return None;
        }
        self.cells
            .get(y as usize * self.layout.width as usize + x as usize)
    }
    /// A diagnostic required by the native pathfinder, not a repair that changes
    /// saved terrain. Original map construction set this ring with5524B0.
    pub fn has_blocked_outer_ring(&self) -> bool {
        let blocked = |x, y| self.cell(x, y).is_some_and(|c| c.flags20 & 0x301 == 0x301);
        (0..self.layout.width).all(|x| blocked(x, 0) && blocked(x, self.layout.height - 1))
            && (0..self.layout.height).all(|y| blocked(0, y) && blocked(self.layout.width - 1, y))
    }
}

/// Native54EE20. This preserves all other bits and is also usable after an
/// explicitly authorized terrain change; object membership is independent.
pub fn recompute_passability(flags: u32, passability_bits: u8) -> u32 {
    if flags & 1 != 0 {
        return flags | 0x300;
    }
    (flags & !0x300)
        | if passability_bits & 1 == 0 { 0x100 } else { 0 }
        | if passability_bits & 2 == 0 { 0x200 } else { 0 }
}
