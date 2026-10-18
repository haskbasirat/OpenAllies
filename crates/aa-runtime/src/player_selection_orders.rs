//!5F4450/5F4490/5F44C0/5F44F0 Process bodies, after76A530 decoding.
//!Each child resolves the current Admin284/player50 owner when reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Add,
    Remove,
    Clear,
    Set,
}
pub trait World {
    type Actor: Copy;
    fn target(&mut self) -> Result<Option<Self::Actor>, String>;
    fn clear(&mut self) -> Result<(), String>;
    fn add(&mut self, actor: Self::Actor) -> Result<(), String>;
    fn remove(&mut self, actor: Self::Actor) -> Result<(), String>;
}
pub fn run(world: &mut impl World, operation: Operation) -> Result<(), String> {
    match operation {
        Operation::Add => {
            if let Some(actor) = world.target()? {
                world.add(actor)?
            }
        }
        Operation::Remove => {
            if let Some(actor) = world.target()? {
                world.remove(actor)?
            }
        }
        Operation::Clear => world.clear()?,
        Operation::Set => {
            world.clear()?;
            if let Some(actor) = world.target()? {
                world.add(actor)?
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "player_selection_orders_tests.rs"]
mod tests;
