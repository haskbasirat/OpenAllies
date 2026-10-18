//! Original TERRAIN auxiliary region footprint and saved water geometry.
//!
//! Native 7AA9B0 precedes GMap+5C's 55C540 reader. This module consumes their
//! serialized version0 layouts; it does not construct a runtime region graph.
use anyhow::{Context, Result, bail, ensure};
use std::{collections::BTreeMap, ops::Range};

#[derive(Debug)]
pub(crate) struct WaterGeometry {
    /// Relative to the supplied auxiliary suffix, after the region graph.
    pub cache_offset: usize,
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    /// A missing cell takes native7376B0's regular quad path. An existing empty
    /// range takes the cached path and emits no triangles.
    pub cells: BTreeMap<[u32; 2], Range<usize>>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8]> {
        let end = self
            .at
            .checked_add(count)
            .context("water extent overflow")?;
        let out = self
            .bytes
            .get(self.at..end)
            .context("truncated water geometry")?;
        self.at = end;
        Ok(out)
    }
    fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn version(&mut self) -> Result<()> {
        ensure!(
            self.word()? == 0,
            "unverified water/region serialization version"
        );
        Ok(())
    }
    fn count(&mut self, minimum: usize) -> Result<usize> {
        let count = self.word()? as usize;
        ensure!(
            count <= i32::MAX as usize && count <= (self.bytes.len() - self.at) / minimum,
            "water count exceeds source extent"
        );
        Ok(count)
    }
}

pub(crate) fn decode_suffix(bytes: &[u8], width: u32, height: u32) -> Result<WaterGeometry> {
    ensure!(
        width > 0 && height > 0 && width.checked_mul(height).is_some(),
        "invalid water grid"
    );
    let mut r = Reader { bytes, at: 0 };
    r.version()?; //7AA9B0
    if r.take(1)?[0] != 0 {
        for _ in 0..r.count(16)? {
            //7BBAF0: version, two words, pair vector
            r.version()?;
            r.take(8)?;
            let pairs = r.count(8)?;
            r.take(pairs * 8)?;
        }
        for _ in 0..r.count(36)? {
            //7BACF0: version, two words, four side lists
            r.version()?;
            r.take(8)?;
            for _ in 0..4 {
                r.take(2)?;
                for _ in 0..r.count(16)? {
                    r.version()?; //7B94A0: four shorts and a word
                    r.take(12)?;
                }
            }
        }
    }
    let cache_offset = r.at;
    r.version()?; //55C540
    let vertex_count = r.count(8)?;
    let mut vertices = Vec::new();
    vertices.try_reserve_exact(vertex_count)?;
    for _ in 0..vertex_count {
        let vertex = [f32::from_bits(r.word()?), f32::from_bits(r.word()?)];
        ensure!(
            vertex.iter().all(|v| v.is_finite()),
            "nonfinite cached water vertex"
        );
        vertices.push(vertex);
    }
    let index_count = r.count(4)?;
    let mut indices = Vec::new();
    indices.try_reserve_exact(index_count)?;
    for _ in 0..index_count {
        let index = r.word()?;
        ensure!(
            (index as usize) < vertex_count,
            "cached water vertex index outside array"
        );
        indices.push(index);
    }
    let mut cells = BTreeMap::new();
    for _ in 0..r.count(16)? {
        let cell = [r.word()?, r.word()?];
        let first = r.word()? as usize;
        let last = r.word()? as usize;
        ensure!(
            cell[0] < width && cell[1] < height,
            "cached water cell outside grid"
        );
        let range = if first > last {
            //7377F0 compares these as unsigned and skips this existing cache entry.
            0..0
        } else {
            ensure!(
                last < index_count && (last - first + 1).is_multiple_of(3),
                "cached water triangle range outside array or incomplete"
            );
            first..last + 1
        };
        if cells.insert(cell, range).is_some() {
            bail!("duplicate cached water cell requires native hash collision policy");
        }
    }
    ensure!(r.at == bytes.len(), "unconsumed terrain auxiliary suffix");
    Ok(WaterGeometry {
        cache_offset,
        vertices,
        indices,
        cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn word(out: &mut Vec<u8>, value: u32) {
        out.extend(value.to_le_bytes());
    }
    fn suffix() -> Vec<u8> {
        let mut out = vec![0; 5]; //disabled version0 region graph
        word(&mut out, 0);
        word(&mut out, 3);
        for value in [0_f32, 0., 2., 0., 0., 2.] {
            word(&mut out, value.to_bits());
        }
        word(&mut out, 3);
        for value in [0, 1, 2] {
            word(&mut out, value);
        }
        word(&mut out, 2);
        for value in [0, 0, 0, 2, 1, 0, 4, 3] {
            word(&mut out, value);
        }
        out
    }
    #[test]
    fn cached_empty_and_missing_cells_keep_native_distinction() {
        let got = decode_suffix(&suffix(), 2, 1).unwrap();
        assert_eq!(got.cache_offset, 5);
        assert_eq!(got.vertices, [[0., 0.], [2., 0.], [0., 2.]]);
        assert_eq!(got.indices, [0, 1, 2]);
        assert_eq!(got.cells[&[0, 0]], 0..3);
        assert!(got.cells[&[1, 0]].is_empty());
        assert!(!got.cells.contains_key(&[0, 1]));
    }
    #[test]
    fn malformed_auxiliary_data_is_rejected_before_use() {
        let bytes = suffix();
        for end in 0..bytes.len() {
            assert!(decode_suffix(&bytes[..end], 2, 1).is_err());
        }
        let mut changed = bytes.clone();
        changed[9..13].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_suffix(&changed, 2, 1).is_err());
        let mut changed = bytes.clone();
        changed[41..45].copy_from_slice(&8_u32.to_le_bytes());
        assert!(decode_suffix(&changed, 2, 1).is_err());
        let mut changed = bytes;
        changed.push(0);
        assert!(decode_suffix(&changed, 2, 1).is_err());
    }

    #[test]
    #[ignore = "requires original local maps and AA_GAME_ROOT"]
    fn all_original_caches_match_native_reader_arrays_and_cell_hashes() {
        use aa_mapworld::{MapArchive, terrain::TerrainChunk};
        use sha2::{Digest, Sha256};
        let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let proof = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/phase6-water-geometry/native-water-geometry.json");
        let fixture: serde_json::Value =
            serde_json::from_slice(&std::fs::read(proof).unwrap()).unwrap();
        let maps = fixture["maps"].as_object().unwrap();
        assert_eq!(maps.len(), 80);
        let hash = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
        for (path, row) in maps {
            let archive = MapArchive::from_bytes(
                std::fs::read(root.join("reconstruction/assets/original/Data").join(path)).unwrap(),
            )
            .unwrap();
            let chunk = archive.decode_chunk("TERRAIN", u64::MAX).unwrap();
            assert_eq!(chunk.payload_source.sha256, row["terrain_sha256"]);
            let terrain = TerrainChunk::decode(&chunk).unwrap();
            assert_eq!(terrain.suffix_offset as u64, row["suffix_offset"]);
            let width = row["width"].as_u64().unwrap() as u32;
            let height = row["height"].as_u64().unwrap() as u32;
            let got = decode_suffix(&terrain.pending_auxiliary_bytes, width, height).unwrap();
            assert_eq!(got.cache_offset as u64, row["cache_offset"]);
            assert_eq!(got.vertices.len() as u64, row["vertex_count"]);
            assert_eq!(got.indices.len() as u64, row["index_count"]);
            assert_eq!(got.cells.len() as u64, row["cell_count"]);
            let vertices: Vec<u8> = got
                .vertices
                .iter()
                .flatten()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let indices: Vec<u8> = got.indices.iter().flat_map(|v| v.to_le_bytes()).collect();
            assert_eq!(hash(&vertices), row["vertices_sha256"]);
            assert_eq!(hash(&indices), row["indices_sha256"]);
            let mut entries: Vec<[u32; 5]> = got
                .cells
                .iter()
                .map(|(cell, range)| {
                    [
                        cell[0] + cell[1] * width,
                        cell[0],
                        cell[1],
                        range.start as u32,
                        (range.end - 1) as u32,
                    ]
                })
                .collect();
            entries.sort_unstable();
            assert_eq!(
                hash(&serde_json::to_vec(&entries).unwrap()),
                row["entries_sha256"]
            );
        }
    }
}
