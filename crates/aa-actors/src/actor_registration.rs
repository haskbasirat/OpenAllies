//! Native4D62F0: registered-state reentry, then grid registration4CFEA0.
//! The original count is captured as signed once; actor bits are reread for
//! each index because an earlier state entry can change later active states.
use crate::{actor_creation::Result, components::StateBits};
pub trait World {
    fn registered_state_count(&mut self) -> Result<i32>;
    fn state_bits(&mut self) -> Result<StateBits>;
    /// Native4D4A90(actual descriptor at this index,0).
    fn state_reenter(&mut self, index: u32) -> Result<()>;
    fn grid_register(&mut self) -> Result<()>;
}
pub fn register(world: &mut impl World) -> Result<()> {
    let count = world.registered_state_count()?;
    for index in 0..count {
        let state = world.state_bits()?;
        let bits = u64::from(state.low) | (u64::from(state.high) << 32);
        // CRT96D710 examines CL, then returns zero for shifts64..255.
        let shift = index as u8;
        let mask = if shift < 64 { 1u64 << shift } else { 0 };
        if bits & mask != 0 {
            world.state_reenter(index as u32)?;
        }
    }
    world.grid_register()
}
