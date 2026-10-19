//! Computer opponent: plays through the same commands as a human, following
//! the original strategic AI (aa_mind::sai) behaviour. Owner: opponent.
use crate::world::{Command, World};

pub fn think(world: &mut World) -> Vec<Command> {
    let _ = world;
    Vec::new()
}
