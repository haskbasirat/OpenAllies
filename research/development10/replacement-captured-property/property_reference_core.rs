//!Exact production transport helper against existing public Bank identities.
//!No Actor, Staged or successful gameplay allocation is constructed.
mod definitions{pub use aa_runtime::definitions::Bank;}
mod map_replacement_queue{pub use aa_runtime::map_replacement_queue::DefinitionRef;}
#[path="../../../crates/aa-runtime/src/replacement_property_reference.rs"]
mod captured;
