//! Settlement (wall) component.
//!
//! - `0x5853E0`: the random segment pick. Up to `+28` world RNG draws
//!   (`0x4F02B0` + `0x85D840`); each draw modulo the count picks a slot of
//!   the `+24` segment array, and the first filled slot whose owner
//!   ActorData `+47C` segment row (stride 0x14) is not kind 1 is returned.
//!
//! Proven by research/phase8-combat/native_random_segment.py
//! (tests/native_random_segment.rs).
use crate::{Error, Result};

/// Services of the segment pick.
pub trait SegmentWorld {
    /// `0x4F02B0` + `0x85D840`: one world RNG draw (with the two sync-log
    /// lines of `0x4F02B0`, which carry the seed before the draw).
    fn random(&mut self) -> Result<u32>;
    /// Owner ActorData `+47C` row `slot` (stride 0x14), word 0.
    fn segment_kind(&self, slot: usize) -> Result<u32>;
}

/// Native `0x5853E0`: `segments` is the `+24` array (`+28` entries, 0 for an
/// empty slot). None when the count is zero or no draw found a segment.
pub fn random_segment(segments: &[u32], world: &mut impl SegmentWorld) -> Result<Option<u32>> {
    let count = u32::try_from(segments.len())
        .ok()
        .filter(|&count| i32::try_from(count).is_ok())
        .ok_or(Error::Invalid("segment count outside the native int range"))?;
    for _ in 0..count {
        let slot = (world.random()? % count) as usize;
        let segment = segments[slot];
        if segment != 0 && world.segment_kind(slot)? != 1 {
            return Ok(Some(segment));
        }
    }
    Ok(None)
}
