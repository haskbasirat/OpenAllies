//! Trucks driving to a spot and unpacking into their building; packing
//! buildings back into trucks. Owner: core.
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
