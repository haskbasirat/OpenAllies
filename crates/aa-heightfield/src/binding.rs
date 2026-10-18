//! Native map construction and HEIGHT_MAP/MOUNTAIN_HEIGHT_MAP chunk binding.
//! The original constructor takes actual world extents and an authored SVar;
//! neither orientation nor dimensions are inferred from a sample count.
use crate::{Error, Heightfield, Result};
use aa_numeric::Binary;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridLayout {
    pub extent_i32: [i32; 2],
    pub width: i32,
    pub height: i32,
    pub coordinate_shift: u32,
    pub cell_size_bits: u32,
    pub index_bias: i32,
}
impl GridLayout {
    /// Original5519A0: ceil the binary32 world extents, then arithmetic shift
    /// and add the last corner. The native assert requires positive resolution;
    /// resolutions1..30 keep its signed cell size positive and representable.
    /// No game-thread precision choice is needed for these exact powers of two.
    pub fn from_world_extents(extent_bits: [u32; 2], resolution_bits: u32) -> Result<Self> {
        if !(1..=30).contains(&resolution_bits) {
            return Err(Error::InvalidLayout);
        }
        let ceil_extent = |bits| -> Result<i32> {
            let value = Binary::from_f32_bits(bits)?;
            if value.is_negative_nonzero() {
                return Err(Error::InvalidLayout);
            }
            let truncated = value.truncate_i32()?;
            if value.compare(&Binary::from_i32(truncated)) == Ordering::Greater {
                truncated.checked_add(1).ok_or(Error::InvalidLayout)
            } else {
                Ok(truncated)
            }
        };
        let extent_i32 = [ceil_extent(extent_bits[0])?, ceil_extent(extent_bits[1])?];
        let layout = Self {
            extent_i32,
            width: (extent_i32[0] >> resolution_bits) + 1,
            height: (extent_i32[1] >> resolution_bits) + 1,
            coordinate_shift: resolution_bits,
            cell_size_bits: (127 + resolution_bits) << 23,
            index_bias: 1 << (resolution_bits - 1),
        };
        layout.sample_count()?;
        Ok(layout)
    }
    pub fn sample_count(self) -> Result<usize> {
        let canonical = !(1..=30).contains(&self.coordinate_shift)
            || self.extent_i32.iter().any(|v| *v < 0)
            || self.width != (self.extent_i32[0] >> self.coordinate_shift) + 1
            || self.height != (self.extent_i32[1] >> self.coordinate_shift) + 1
            || self.cell_size_bits != (127 + self.coordinate_shift) << 23
            || self.index_bias != 1 << (self.coordinate_shift - 1);
        if canonical {
            return Err(Error::InvalidLayout);
        }
        let count = self
            .width
            .checked_mul(self.height)
            .filter(|n| *n > 0)
            .ok_or(Error::InvalidGrid)? as usize;
        // Original allocation argument is a 32-bit count*4. Reject overflowing
        // original-format layouts rather than reproducing undersized allocation.
        if count > u32::MAX as usize / 4 {
            return Err(Error::InvalidGrid);
        }
        Ok(count)
    }
    fn field(self, height_bits: Vec<u32>) -> Heightfield {
        Heightfield {
            width: self.width,
            height: self.height,
            coordinate_shift: self.coordinate_shift,
            cell_size_bits: self.cell_size_bits,
            index_bias: self.index_bias,
            height_bits,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitializedHeightfield {
    pub layout: GridLayout,
    pub field: Heightfield,
    pub minimum_bits: u32,
    pub maximum_bits: u32,
    pub default_bits: u32,
    /// Original byte storage at+44: the per-vertex water-area id written by
    /// `0x550F20` / `0x5507C0` (0 = none, an area id, 0xFF = shared by
    /// several areas); see `aa_spatial::water_areas::VertexAreas`.
    pub sample_flags: Vec<u8>,
}
impl InitializedHeightfield {
    ///5519A0 fills every sample with the supplied default and clears byte flags.
    /// Limits are stored, not applied as a clamp during initialization or load.
    pub fn new(
        layout: GridLayout,
        minimum_bits: u32,
        maximum_bits: u32,
        default_bits: u32,
    ) -> Result<Self> {
        let count = layout.sample_count()?;
        let mut height_bits = Vec::new();
        height_bits
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        height_bits.resize(count, default_bits);
        let mut sample_flags = Vec::new();
        sample_flags
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        sample_flags.resize(count, 0);
        Ok(Self {
            layout,
            field: layout.field(height_bits),
            minimum_bits,
            maximum_bits,
            default_bits,
            sample_flags,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedHeightfield {
    pub state: InitializedHeightfield,
    pub version: u32,
    pub chunk_sha256: String,
    pub consumed_bytes: usize,
    /// The original reader stops after its byte flags. Preserve any unread tail;
    /// do not invent a meaning or claim the enclosing chunk format is exhausted.
    pub trailing_bytes: Vec<u8>,
}
impl LoadedHeightfield {
    ///551E60 reads limits/default, checks resolution/count, then samples/flags.
    /// Native keeps the previously constructed layout. Version0 instead rebuilds
    /// flags through world5507C0 and remains unsupported here. Rust reports
    /// mismatches before allocation/publication rather than native exceptions.
    pub fn decode(layout: GridLayout, chunk: &[u8]) -> Result<Self> {
        let count = layout.sample_count()?;
        let consumed_bytes = count
            .checked_mul(5)
            .and_then(|n| n.checked_add(24))
            .ok_or(Error::InvalidGrid)?;
        if chunk.len() < 24 {
            return Err(Error::TruncatedChunk);
        }
        let word = |at: usize| {
            u32::from_le_bytes(
                chunk[at..at + 4]
                    .try_into()
                    .expect("checked header/sample span"),
            )
        };
        let version = word(0);
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
        if word(16) != layout.coordinate_shift || word(20) as usize != count {
            return Err(Error::LayoutMismatch);
        }
        if chunk.len() < consumed_bytes {
            return Err(Error::TruncatedChunk);
        }
        let mut heights = Vec::new();
        heights
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        for at in (24..24 + count * 4).step_by(4) {
            heights.push(word(at));
        }
        let mut sample_flags = Vec::new();
        sample_flags
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        sample_flags.extend_from_slice(&chunk[24 + count * 4..consumed_bytes]);
        let mut trailing_bytes = Vec::new();
        trailing_bytes
            .try_reserve_exact(chunk.len() - consumed_bytes)
            .map_err(|_| Error::Allocation)?;
        trailing_bytes.extend_from_slice(&chunk[consumed_bytes..]);
        Ok(Self {
            state: InitializedHeightfield {
                layout,
                field: layout.field(heights),
                minimum_bits: word(4),
                maximum_bits: word(8),
                default_bits: word(12),
                sample_flags,
            },
            version,
            chunk_sha256: format!("{:x}", Sha256::digest(chunk)),
            consumed_bytes,
            trailing_bytes,
        })
    }
}
