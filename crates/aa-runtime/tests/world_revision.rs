//! Small direct replay of the production constructor field kernel.
//The runtime witness tests also borrow the actual Sim alias. Include the same
//three source modules together, preserving their one crate/type identity.
pub use aa_runtime::{catalog, load_host_settings, simulation};
#[path = "../src/live_sim.rs"]
mod live_sim;
#[path = "../src/load_sim_world.rs"]
mod load_sim_world;
#[path = "../src/world_revision.rs"]
mod world_revision;
