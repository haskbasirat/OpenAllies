//! Direct-link replay of the unchanged public API test. Uses the exact warm
//! aa-runtime library; avoids an unrelated changing aa-netcode dev dependency.
pub use aa_runtime::*;
use aa_runtime::{catalog::Catalog,live_object_init::Record,battle_terrain::BattleTerrain};
use aa_runtime::load_organization_terrain_refresh::refresh;
use aa_actors::{ActorId,components::ComponentSlot};
use aa_mapworld::restore::{RestoredRecord,RestoredComponent};
use aa_numeric::Precision;
#[path="../../crates/aa-runtime/src/load_organization_terrain_refresh_tests.rs"]
mod tests;
