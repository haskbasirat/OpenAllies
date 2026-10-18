//! Checked original RMP map inputs. Raw native values retain their source spans;
//! unsupported subtype data is never replaced with guessed gameplay defaults.
#![forbid(unsafe_code)]
pub mod actor_payload;
pub mod actor_finish;
pub mod commander_restore;
pub mod archive;
pub mod bootstrap;
pub mod class_conversion;
pub mod definition_counts;
pub mod dynamic_doodad;
pub mod fresh_creation;
pub mod header;
pub mod inventory;
pub mod kingdoms;
pub mod map_objects;
pub mod map_prepare;
pub mod performer_restore;
pub mod kingdom_ai;
pub mod organization_restore;
pub mod ownership;
pub mod recruitment;
pub mod restore;
pub mod restore_components;
pub mod special_state;
pub mod saved_components;
pub mod terrain;
pub mod world;
pub use archive::{Chunk, ChunkDirectory, MapArchive, SourceIdentity};
pub use inventory::{Inventory, Placement, ThingState};
use thiserror::Error;
pub use world::{IdManagerState, WorldInventory, WorldState};

#[derive(Debug, Error)]
pub enum Error {
    #[error("map input: {0}")]
    Format(String),
    #[error("map input requires {needed} bytes, caller decode budget is {budget}")]
    Budget { needed: u64, budget: u64 },
    #[error("map allocation failed")]
    Allocation,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
pub mod team_init;
