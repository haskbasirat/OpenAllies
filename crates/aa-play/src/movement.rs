//! Orders to regiments and actors, path following, formation slot
//! following, terrain speed limits, turning. Owner: movement.
use crate::world::World;
use crate::world::Command;

pub fn tick(world: &mut World) {
    let _ = world;
}

/// Apply a player or computer command; Err is the refusal reason.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    let _ = world;
    Err(format!("{command:?} is not implemented yet"))
}
