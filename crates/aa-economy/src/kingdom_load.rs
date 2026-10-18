//! Economy semantics of GKingdom::Load `0x4E8550` that the wire decoders
//! (aa-mapworld phase 6/7: prefix `0x4E85C1..0x4E86C3`, suffix
//! `0x4E8EF8..0x4E93A0`, raw Properties/Tactical/Strategic words) do not
//! apply. Statistics: [`crate::statistics::Statistics::load_saved`]
//! (`0x58EE80`, the "strategic" words), [`crate::statistics::RelationStatistics::
//! load_saved`] (`0x58DBE0`, the "tactical" tables) into the loaded slots
//! +0x330/+0x344, promoted at world start by
//! [`crate::statistics::KingdomStatistics::start_world`] (`0x4F1E10`:
//! `0x4E3690` + `0x58EA00`). Property records: [`loaded_property_records`].

use serde::{Deserialize, Serialize};

/// Disposition of one saved kingdom property record (kingdom Load v7+).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadedPropertyRecord {
    /// Appended to kingdom+0x318 in saved order.
    Kept,
    /// `0x503080` failed (record source not 1, 2 or 3): "kingdom load
    /// ignoring prop %s" (`0x404F60`), record dropped (and leaked natively).
    IgnoredSource,
    /// World version (world+0x2C) < 0x15: every valid record is discarded and
    /// freed; only the first one logs "Ignoring incorrectly saved tech: %s
    /// (%d total)" (`0x405D60`/`0x5A1040`; the counter is reset at loop start).
    LegacyDiscarded { logged: bool },
}

/// The v7+ property loop of `0x4E8550`. `sources` are the decoded record
/// source words (record+8); a version-0 record carries none and `0x503B10`
/// stores -1, which the source check rejects.
pub fn loaded_property_records(
    world_version: u32,
    sources: &[Option<u32>],
) -> Vec<LoadedPropertyRecord> {
    let mut legacy = 0u32;
    sources
        .iter()
        .map(|source| {
            let source = source.unwrap_or(u32::MAX);
            if !(1..=3).contains(&source) {
                LoadedPropertyRecord::IgnoredSource
            } else if world_version < 0x15 {
                legacy = legacy.wrapping_add(1);
                LoadedPropertyRecord::LegacyDiscarded {
                    logged: legacy == 1,
                }
            } else {
                LoadedPropertyRecord::Kept
            }
        })
        .collect()
}
