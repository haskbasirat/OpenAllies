//! Small test compilation of these exact production source files. This avoids
//! collecting unrelated runtime lib tests; it does not duplicate the kernels
//! or make their private constructors public in the production library.
//! The MapResource companion now reaches private Creator owners and is tested
//! by its original runtime lib tests plus resource_creator::tests instead.
pub use aa_runtime::{actor_world,catalog,combat_source_completion,combat_source_receipt,definitions,load_view_variables,load_world_rules,settings,kingdom_mission_start};
#[path="../src/kingdom_start_source.rs"]
mod kingdom_start_source;
#[path="../src/map_resource_streams.rs"]
mod map_resource_streams;
// Kingdom mission tests now exercise private ActorStore field loans. They
// remain in the actual runtime lib test cohort, rather than embedding a
// second module whose external crate cannot access that private owner.
