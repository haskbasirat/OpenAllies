//! Playable Axis & Allies battle simulation.
//!
//! Behaviour-faithful, not byte-exact: every system follows the original
//! game's logic (as read from AA.exe and the exact ports in the other crates)
//! and every number comes from the original rules data, but the arithmetic is
//! plain `f32` and the data layout is our own. Nothing the original does not
//! have is added. A system can later be replaced by an exact port.
//!
//! Coordinates are native: X/Y on the ground, Z up, metres. Angles are radians
//! measured like the original facing (see `world::Facing`).
//!
//! Module ownership while the game is being built (one owner per file):
//! - data, map, setup, economy, production, deploy, supply: the core owner
//! - path, movement, formation: the movement owner
//! - combat, morale: the combat owner
//! - ai, victory: the opponent owner
//! - world, sim, lib: shared types; change only by agreement

pub mod ai;
pub mod books;
pub mod combat;
pub mod data;
pub mod deploy;
pub mod economy;
pub mod formation;
pub mod map;
pub mod morale;
pub mod movement;
pub mod path;
pub mod production;
pub mod setup;
pub mod sim;
pub mod supply;
pub mod victory;
pub mod world;

pub use data::{Def, DefClass, DefId, GameData};
pub use map::GameMap;
pub use setup::{PlayerSetup, SkirmishSetup};
pub use world::*;

/// The original simulation rate: 16 ticks per second (aa_simulation::NativeClock).
pub const TICKS_PER_SECOND: u32 = 16;
pub const DT: f32 = 1.0 / TICKS_PER_SECOND as f32;
pub mod mapgen;
/// Fog of war (appended by the UI/fog owner).
pub mod fog;
/// HUD player orders (appended by the UI owner).
pub mod player_orders;
