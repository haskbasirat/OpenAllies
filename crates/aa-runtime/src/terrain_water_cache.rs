//! GMap+5C saved water triangulation (AA.exe 0x55C540 reader, after the
//! 0x7AA9B0 region-graph records in the TERRAIN auxiliary suffix).
//!
//! The same serialized layout as aa-client's `water_geometry::decode_suffix`
//! (proven by research/phase6-water-geometry against the original reader and
//! 0x55B9E0 lookups on all 80 original maps). aa-runtime cannot depend on
//! aa-client, so the reader lives here. The aa-client copy can switch to this
//! one (owner decision). Words stay raw binary32 bits for the native
//! 0x7377F0 copies.
//!
//! Only 0x555F20 (the random map generator and an editor command) rebuilds
//! this cache; authored maps keep the saved one. A gtile without a cell entry
//! takes 0x7376B0's regular quad.
use std::{collections::BTreeMap, ops::Range};

type Result<T> = std::result::Result<T, String>;

/// GMap+5C: +1C vertex pairs, +10 dword indices, +28 cell hash
/// `(x, y) -> (first, last)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaterCache {
    pub vertices: Vec<[u32; 2]>,
    pub indices: Vec<u32>,
    /// `first..last+1`; an existing entry with `first > last` is empty (the
    /// 0x7378D9 unsigned `ja` skips it).
    pub cells: BTreeMap<[u32; 2], Range<usize>>,
}
impl WaterCache {
    /// 0x55B9E0 then 0x7378E0..: the cached points of a gtile in index order,
    /// or `None` for a missing cell.
    pub fn cell_points(&self, cell: [i32; 2]) -> Result<Option<Vec<[u32; 2]>>> {
        if cell[0] < 0 || cell[1] < 0 {
            return Ok(None);
        }
        let Some(range) = self.cells.get(&[cell[0] as u32, cell[1] as u32]) else { return Ok(None) };
        let mut out = Vec::new();
        out.try_reserve_exact(range.len()).map_err(|_| "water cache cell allocation failed")?;
        for k in range.clone() {
            let index = *self.indices.get(k).ok_or("0x7378E0 reads beyond the GMap+6C index array")? as usize;
            out.push(*self.vertices.get(index).ok_or("0x7378ED reads beyond the GMap+78 vertex array")?);
        }
        Ok(Some(out))
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8]> {
        let end = self.at.checked_add(count).ok_or("water extent overflow")?;
        let out = self.bytes.get(self.at..end).ok_or("truncated water geometry")?;
        self.at = end;
        Ok(out)
    }
    fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "water word")?))
    }
    fn version(&mut self) -> Result<()> {
        if self.word()? != 0 {
            return Err("unverified water/region serialization version".into());
        }
        Ok(())
    }
    fn count(&mut self, minimum: usize) -> Result<usize> {
        let count = self.word()? as usize;
        if count > i32::MAX as usize || count > (self.bytes.len() - self.at) / minimum {
            return Err("water count exceeds source extent".into());
        }
        Ok(count)
    }
}

/// The TERRAIN auxiliary suffix (`TerrainChunk::pending_auxiliary_bytes`):
/// 0x7AA9B0 region records skipped by their version-0 footprints, then the
/// 0x55C540 cache.
pub fn decode_suffix(bytes: &[u8], width: u32, height: u32) -> Result<WaterCache> {
    if width == 0 || height == 0 || width.checked_mul(height).is_none() {
        return Err("invalid water grid".into());
    }
    let mut r = Reader { bytes, at: 0 };
    r.version()?; // 0x7AA9B0
    if r.take(1)?[0] != 0 {
        for _ in 0..r.count(16)? {
            r.version()?; // 0x7BBAF0: two words, pair vector
            r.take(8)?;
            let pairs = r.count(8)?;
            r.take(pairs * 8)?;
        }
        for _ in 0..r.count(36)? {
            r.version()?; // 0x7BACF0: two words, four side lists
            r.take(8)?;
            for _ in 0..4 {
                r.take(2)?;
                for _ in 0..r.count(16)? {
                    r.version()?; // 0x7B94A0
                    r.take(12)?;
                }
            }
        }
    }
    r.version()?; // 0x55C540
    let vertex_count = r.count(8)?;
    let mut vertices = Vec::new();
    vertices.try_reserve_exact(vertex_count).map_err(|_| "water vertex allocation failed")?;
    for _ in 0..vertex_count {
        let v = [r.word()?, r.word()?];
        if v.iter().any(|w| (w >> 23) & 0xff == 0xff) {
            return Err("nonfinite cached water vertex".into());
        }
        vertices.push(v);
    }
    let index_count = r.count(4)?;
    let mut indices = Vec::new();
    indices.try_reserve_exact(index_count).map_err(|_| "water index allocation failed")?;
    for _ in 0..index_count {
        let index = r.word()?;
        if index as usize >= vertex_count {
            return Err("cached water vertex index outside array".into());
        }
        indices.push(index);
    }
    let mut cells = BTreeMap::new();
    for _ in 0..r.count(16)? {
        let cell = [r.word()?, r.word()?];
        let first = r.word()? as usize;
        let last = r.word()? as usize;
        if cell[0] >= width || cell[1] >= height {
            return Err("cached water cell outside grid".into());
        }
        let range = if first > last {
            0..0
        } else {
            if last >= index_count || !(last - first + 1).is_multiple_of(3) {
                return Err("cached water triangle range outside array or incomplete".into());
            }
            first..last + 1
        };
        if cells.insert(cell, range).is_some() {
            return Err("duplicate cached water cell requires the native hash collision policy".into());
        }
    }
    if r.at != bytes.len() {
        return Err("unconsumed terrain auxiliary suffix".into());
    }
    Ok(WaterCache { vertices, indices, cells })
}

#[cfg(test)]
#[path = "terrain_water_cache_tests.rs"]
mod tests;
