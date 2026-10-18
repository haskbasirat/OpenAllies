//! Port of the original AA.exe zone, detection and fog-of-war systems
//! (`.\Source\Sim\Zones.cpp` KKC_Zone, `.\Source\Sim\Solver\ZoneSolver.cpp`,
//! the GMap tile zone/fog words, and the zone-type tile queries the AI uses:
//! ZOS supply, ZOC control, ZOG guard, ZOD detection), reconstructed from the
//! native code and proven with Unicorn oracle fixtures under
//! `research/phase8-zones/`.
//!
//! Module map (native addresses in each module):
//! - [`set`]: interned per-tile zone sets (leaf/union classes, add/remove).
//! - [`query`]: the zone-set virtual queries and the tile wrappers the AI and
//!   other systems call (`0x54EF40`..`0x54F8F0`).
//! - [`tile`]: tile fog word (explored/visible per kingdom), supply word, and
//!   the reveal/hide/occupy routines driven by zone types.
//! - [`zone`]: the KKC_Zone object (create, move, release) and its tile
//!   callbacks.
//! - [`solver`]: the ZoneSolver fast circle path and dispatch.
//! - [`flood`]: the ZoneSolver / flood-solver paths for the other zones.
//! - [`component`]: the detection component (ZOD/RZOD ranges and zones).
//! - [`detection`]: the actor detection word (who sees whom), its tile
//!   callbacks and the detection memory refresh.
#![forbid(unsafe_code)]

pub mod component;
pub mod detection;
pub mod flood;
pub mod fpu;
pub mod query;
pub mod set;
pub mod solver;
pub mod tile;
pub mod zone;

pub use set::{ActorKey, Entry, LiveSet, NodeId, SetRef, ZonePool};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    /// A union node id that is not alive in the pool (caller bug).
    #[error("zone-set node {0} is not alive")]
    StaleZoneSet(u32),
    /// A leaf value that is not interned (caller bug).
    #[error("zone-set leaf ({0:#x}, {1:#x}) is not interned")]
    StaleLeaf(u32, u16),
    /// The original passes the null leaf of `0x516360(actor, 0)` to `0x5161C0`
    /// and dereferences it (access violation).
    #[error("native null leaf dereference (new zone entry with flags 0)")]
    NativeNullLeaf,
    /// `0x54FA80` on a tile without a zone set dereferences null.
    #[error("native null zone-set dereference (remove on an empty tile)")]
    NativeNullSet,
    /// `0x516560` walks into the final leaf when no entry has the actor (the
    /// original corrupts the leaf's hash link and reads garbage).
    #[error("native undefined behaviour: removing an absent actor from a zone list")]
    NativeAbsentActor,
    /// A native field read through a null pointer (named).
    #[error("native null dereference: {0}")]
    NativeNull(&'static str),
    /// A binary32 input outside the finite domain of the port (NaN/Inf).
    #[error("non-finite binary32 {0:#010x} outside the ported domain")]
    NonFinite(u32),
    /// A tile index outside the GMap tile array.
    #[error("tile {0} outside the map")]
    TileOutside(i64),
    /// A required world service failed.
    #[error("world service failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;
