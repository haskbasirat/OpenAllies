//! Original saved static-region topology, tile publication and point lookup.
//! Native world versions below17 rebuild topology and are not accepted here.
use crate::region_search::{RegionGraph, RegionNode};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("invalid or unsupported saved topology: {0}")]
    Format(&'static str),
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Exact constructed GMap dimensions and stored integer extent converted to f32.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GridLayout {
    pub width: i32,
    pub height: i32,
    pub coordinate_shift: u32,
    pub extent_bits: [u32; 2],
}
impl GridLayout {
    fn count(self) -> Result<usize> {
        if self.width <= 0 || self.height <= 0 || self.coordinate_shift > 30 {
            return Err(Error::Format("invalid grid dimensions"));
        }
        for (extent, cells) in self.extent_bits.into_iter().zip([self.width, self.height]) {
            let b = Binary::from_f32_bits(extent)?;
            if b.is_negative_nonzero() || b.truncate_i32()? >> self.coordinate_shift != cells {
                return Err(Error::Format("grid dimensions differ from stored extent"));
            }
        }
        (self.width as usize)
            .checked_mul(self.height as usize)
            .ok_or(Error::Format("grid extent overflow"))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedRegion {
    pub position_bits: [u32; 2],
    /// Stream order is retained; the native hash lookup picks the last duplicate.
    pub distance_pairs: Vec<[u32; 2]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Side {
    pub bounds: [u16; 4],
    pub rectangle: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rectangle {
    pub region: i32,
    pub island: i32,
    /// Native+C,+E,+10,+12: minY,maxX,maxY,minX, inclusive signed shorts.
    pub bounds: [i16; 4],
    pub sides: [Vec<Side>; 4],
}
impl Rectangle {
    fn area(&self) -> i32 {
        let x = self.bounds[1].wrapping_sub(self.bounds[3]).wrapping_add(1);
        let y = self.bounds[2].wrapping_sub(self.bounds[0]).wrapping_add(1);
        i32::from(x).wrapping_mul(i32::from(y))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Topology {
    pub layout: GridLayout,
    pub enabled: bool,
    pub regions: Vec<SavedRegion>,
    pub rectangles: Vec<Rectangle>,
    /// Original7B99B0 inserts each loaded rectangle at the region's list head.
    pub region_rectangles: Vec<Vec<u32>>,
    pub tile_rectangles: Vec<Option<u32>>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StaticTile {
    pub flags20: u32,
    /// Actual effective definition type, required when neither blocking bit is set.
    pub definition_type: u32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PostLoad {
    pub primary_island: Option<u32>,
    pub diagonal_bits: u32,
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or(Error::Format("byte extent overflow"))?;
        let out = self
            .bytes
            .get(self.at..end)
            .ok_or(Error::Format("truncated region data"))?;
        self.at = end;
        Ok(out)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn version(&mut self) -> Result<()> {
        if self.u32()? != 0 {
            return Err(Error::Format("unknown region version"));
        }
        Ok(())
    }
    fn count(&mut self, minimum: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        if n > i32::MAX as usize || n > (self.bytes.len() - self.at) / minimum {
            return Err(Error::Format("count exceeds source extent"));
        }
        Ok(n)
    }
}
impl Topology {
    /// The TERRAIN suffix begins at7AA9B0. Return bytes consumed before water.
    /// World<17 executes static reanalysis instead; no stale topology shortcut.
    pub fn decode_prefix(
        bytes: &[u8],
        layout: GridLayout,
        world_version: u32,
    ) -> Result<(Self, usize)> {
        if world_version < 17 {
            return Err(Error::Format(
                "world version requires native static-map rebuild",
            ));
        }
        let count = layout.count()?;
        let mut r = Reader { bytes, at: 0 };
        r.version()?;
        let enabled = r.take(1)?[0] != 0;
        let mut regions = Vec::new();
        let mut rectangles = Vec::new();
        if enabled {
            let n = r.count(16)?;
            regions.try_reserve_exact(n)?;
            for _ in 0..n {
                r.version()?;
                let position_bits = [r.u32()?, r.u32()?];
                let n = r.count(8)?;
                let mut distance_pairs = Vec::new();
                distance_pairs.try_reserve_exact(n)?;
                for _ in 0..n {
                    distance_pairs.push([r.u32()?, r.u32()?]);
                }
                regions.push(SavedRegion {
                    position_bits,
                    distance_pairs,
                });
            }
            let n = r.count(36)?;
            rectangles.try_reserve_exact(n)?;
            for _ in 0..n {
                r.version()?;
                let region = r.u32()? as i32;
                let island = r.u32()? as i32;
                let mut bounds = [0; 4];
                let mut sides: [Vec<Side>; 4] = std::array::from_fn(|_| Vec::new());
                for i in 0..4 {
                    bounds[i] = r.u16()? as i16;
                    let n = r.count(16)?;
                    sides[i].try_reserve_exact(n)?;
                    for _ in 0..n {
                        r.version()?;
                        sides[i].push(Side {
                            bounds: [r.u16()?, r.u16()?, r.u16()?, r.u16()?],
                            rectangle: r.u32()?,
                        });
                    }
                }
                rectangles.push(Rectangle {
                    region,
                    island,
                    bounds,
                    sides,
                });
            }
        }
        let mut region_rectangles = Vec::new();
        region_rectangles.try_reserve_exact(regions.len())?;
        region_rectangles.resize_with(regions.len(), Vec::new);
        let mut tile_rectangles = Vec::new();
        tile_rectangles.try_reserve_exact(count)?;
        tile_rectangles.resize(count, None);
        for (id, rect) in rectangles.iter().enumerate() {
            let Some(list) = region_rectangles.get_mut(rect.region as usize) else {
                return Err(Error::Format("rectangle has invalid region"));
            };
            if rect.island < 0 {
                return Err(Error::Format("rectangle has invalid island"));
            }
            list.try_reserve(1)?;
            list.insert(0, id as u32);
            for side in rect.sides.iter().flatten() {
                if side.rectangle as usize >= rectangles.len() {
                    return Err(Error::Format("side has invalid rectangle"));
                }
            }
            let [y0, x1, y1, x0] = rect.bounds;
            if x0 < 0
                || y0 < 0
                || x1 < 0
                || y1 < 0
                || i32::from(x0) >= layout.width
                || i32::from(y0) >= layout.height
                || i32::from(x1) >= layout.width
                || i32::from(y1) >= layout.height
            {
                return Err(Error::Format("rectangle leaves grid"));
            }
            // Empty inverted ranges have no native tile writes. Overlaps retain
            // the last source rectangle, exactly as the original load loop.
            for x in x0..=x1 {
                for y in y0..=y1 {
                    tile_rectangles[y as usize * layout.width as usize + x as usize] =
                        Some(id as u32);
                }
            }
        }
        Ok((
            Self {
                layout,
                enabled,
                regions,
                rectangles,
                region_rectangles,
                tile_rectangles,
            },
            r.at,
        ))
    }
    ///7AA3D0→7B9300. Bounds use stored world extents before truncate/shift.
    pub fn region_at(&self, point_bits: [u32; 2]) -> Result<Option<u32>> {
        let mut xy = [0; 2];
        for i in 0..2 {
            let p = Binary::from_f32_bits(point_bits[i])?;
            if p.is_negative_nonzero()
                || p.compare(&Binary::from_f32_bits(self.layout.extent_bits[i])?) != Ordering::Less
            {
                return Ok(None);
            }
            xy[i] = p.truncate_i32()? >> self.layout.coordinate_shift;
        }
        if xy[0] >= self.layout.width || xy[1] >= self.layout.height {
            return Err(Error::Format("extent maps past allocated grid"));
        }
        let id = self.tile_rectangles[xy[1] as usize * self.layout.width as usize + xy[0] as usize]
            .ok_or(Error::Format("native point lookup has null tile rectangle"))?;
        let region = self.rectangles[id as usize].region;
        Ok((region >= 0).then_some(region as u32))
    }
    pub fn region_area(&self, id: usize) -> Result<i32> {
        let rectangles = self
            .region_rectangles
            .get(id)
            .ok_or(Error::Format("invalid region"))?;
        Ok(rectangles.iter().fold(0i32, |sum, &i| {
            sum.wrapping_add(self.rectangles[i as usize].area())
        }))
    }
    ///7BCA60: reverse region rectangle list, four ordered side lists, first
    /// occurrence deduplication including self, then reverse the emitted array.
    pub fn prepare_graph(&self) -> Result<RegionGraph> {
        let mut nodes = Vec::new();
        nodes.try_reserve_exact(self.regions.len())?;
        for (id, saved) in self.regions.iter().enumerate() {
            let mut neighbors = vec![id as u32];
            for &r in &self.region_rectangles[id] {
                for side in self.rectangles[r as usize].sides.iter().flatten() {
                    let other = self.rectangles[side.rectangle as usize].region as u32;
                    if !neighbors.contains(&other) {
                        neighbors.try_reserve(1)?;
                        neighbors.push(other);
                    }
                }
            }
            neighbors.remove(0);
            neighbors.reverse();
            if neighbors.len() > 255 {
                return Err(Error::Format(
                    "region neighbors exceed native byte iterator",
                ));
            }
            let category = self.region_rectangles[id]
                .first()
                .map_or(u32::MAX, |&r| self.rectangles[r as usize].island as u32);
            let distance_cache = saved
                .distance_pairs
                .iter()
                .copied()
                .map(|p| (p[0], p[1]))
                .collect::<BTreeMap<_, _>>();
            nodes.push(RegionNode {
                position_bits: saved.position_bits,
                category,
                neighbors,
                distance_cache,
                visit_epoch: 0,
                parent_neighbor: 255,
            });
        }
        Ok(RegionGraph {
            nodes,
            epoch: 0,
            //7A0D40 marks the actual SAI engine ready only after other startup
            //work. Ordered topology alone does not authorize that world state.
            ready: false,
        })
    }
    ///7AA2A0 and the final7AA9B0 diagonal store, with actual static tile inputs.
    /// Disabled source topology leaves existing manager fields unchanged, so
    /// there is no invented replacement PostLoad value in that case.
    pub fn post_load(&self, tiles: &[StaticTile], p: Precision) -> Result<Option<PostLoad>> {
        if !self.enabled {
            return Ok(None);
        }
        if tiles.len() != self.tile_rectangles.len() {
            return Err(Error::Format("static tile count differs"));
        }
        let mut islands: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for id in (0..self.regions.len()).rev() {
            let r = *self.region_rectangles[id]
                .first()
                .ok_or(Error::Format("native island lookup has empty region"))?;
            islands
                .entry(self.rectangles[r as usize].island as u32)
                .or_default()
                .push(id);
        }
        let mut best = Binary::from_u32(0);
        let mut primary_island = None;
        for (island, regions) in islands {
            let r = &self.rectangles[self.region_rectangles[regions[0]][0] as usize];
            let tile =
                tiles[r.bounds[0] as usize * self.layout.width as usize + r.bounds[3] as usize];
            let terrain_type = if tile.flags20 & 0x200 != 0 {
                u32::from(tile.flags20 & 0x100 != 0)
            } else if tile.flags20 & 0x100 != 0 {
                2
            } else {
                tile.definition_type
            };
            if terrain_type != 0 {
                continue;
            }
            let mut area = Binary::from_u32(0);
            for region in regions {
                for &rect in &self.region_rectangles[region] {
                    area = area.add(&Binary::from_i32(self.rectangles[rect as usize].area()), p)?;
                }
            }
            if area.compare(&best) == Ordering::Greater {
                primary_island = Some(island);
                best = Binary::from_f32_bits(area.store_f32()?)?;
            }
        }
        Ok(Some(PostLoad {
            primary_island,
            diagonal_bits: self.diagonal_bits(p)?,
        }))
    }
    /// Native post-load map diagonal at7AAC5A..71. Not a route length.
    pub fn diagonal_bits(&self, p: Precision) -> Result<u32> {
        let x = Binary::from_f32_bits(self.layout.extent_bits[0])?;
        let y = Binary::from_f32_bits(self.layout.extent_bits[1])?;
        Ok(y.mul(&y, p)?.add(&x.mul(&x, p)?, p)?.sqrt(p)?.store_f32()?)
    }
}
