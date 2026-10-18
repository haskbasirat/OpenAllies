//!566DB0 Defender InitExisting. Range is stored before zone allocation or any
//!zone/Performer child. Existing Zone4 skips source reads and the range store.
use crate::actor_creation::{Error, Result};
#[cfg(test)]
#[path = "defender_init_tests.rs"]
mod tests;
pub trait World {
    fn zone_present(&mut self) -> Result<bool>;
    fn definition_mask(&mut self) -> Result<u32>;
    ///45D4C0(NULL) for Organization;45D6C0(NULL) otherwise, including FSTP32.
    fn range(&mut self, organization: bool) -> Result<u32>;
    fn store_range(&mut self, bits: u32) -> Result<()>;
    fn create_zone(&mut self) -> Result<bool>;
    fn aircraft_present(&mut self) -> Result<bool>;
    fn aircraft_zone_flag(&mut self) -> Result<()>;
    fn position(&mut self) -> Result<[u32; 2]>;
    fn move_zone(&mut self, position: [u32; 2], range: u32) -> Result<()>;
    fn performer_present(&mut self) -> Result<bool>;
    fn refresh_performer(&mut self) -> Result<()>;
}
pub fn initialize(world: &mut impl World) -> Result<bool> {
    if !world.zone_present()? {
        let organization = world.definition_mask()? & (1 << 7) != 0;
        let range = world.range(organization)?;
        world.store_range(range)?;
        if world.create_zone()? {
            if world.aircraft_present()? {
                if !world.zone_present()? {
                    return Err(Error::NullDereference("515940 on a null zone"));
                }
                world.aircraft_zone_flag()?;
            }
            let position = world.position()?;
            world.move_zone(position, range)?;
        }
    }
    if world.performer_present()? {
        world.refresh_performer()?;
    }
    Ok(true)
}
