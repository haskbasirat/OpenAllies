//! Original aircraft Mind virtual +8 (`517C60`, vtable `A32710`): the
//! activation that also decides the dog-fighter byte Mind `+1D0` read by the
//! attack Think (`51B8C0`, [`crate::uai_thinks::aai_attack`]).
//!
//! Order:
//! 1. With an empty state stack (Mind `+10` null): the byte `+7C` of the
//!    object at Mind `+18C` (the `+4` word of the embedded state at `+188`)
//!    is cleared; `529C30` on that object's `+34` state yields an actor whose
//!    flight component (`+BC`) gets `55EB30(0)`; then `52A810(Mind+188, 2)`.
//! 2. The base activation `529EC0` (state `+1A0` when the stack is empty,
//!    else `+B8`). A zero AL is returned at once, leaving `+1D0` unchanged.
//! 3. `+1D0 = 0`. The owner's Performer (`+4` actor `+74`) ability array
//!    (`+14`, signed count `+18`) is scanned in order: the first ability whose
//!    definition (`+4`) has a null `+3C` object, or a `+3C` object whose
//!    `+2B4` word is zero, sets `+1D0 = 1` and ends the scan. Returns 1.

/// The native reads and calls of `517C60`, in its order.
pub trait World {
    type Error;
    /// Fact: Mind `+10` (state stack head) is null.
    fn stack_empty(&mut self) -> Result<bool, Self::Error>;
    /// `[Mind+18C]+7C = 0`.
    fn clear_188_owner_byte_7c(&mut self) -> Result<(), Self::Error>;
    /// `55EB30(mode)` on the flight component (`+BC`) of the actor that
    /// `529C30` returns for state `[Mind+18C]+34`.
    fn join_formation(&mut self, mode: u32) -> Result<(), Self::Error>;
    /// `52A810(Mind+188, mode)`.
    fn activate_188(&mut self, mode: u32) -> Result<(), Self::Error>;
    /// `529EC0` on the Mind: its AL.
    fn base_activate(&mut self) -> Result<u8, Self::Error>;
    /// Fact: Performer `+18` ability count (signed).
    fn ability_count(&mut self) -> Result<i32, Self::Error>;
    /// Fact: ability `index` definition `+3C` object, `None` when null, and
    /// otherwise that object's `+2B4` word.
    fn ability_3c_word_2b4(&mut self, index: i32) -> Result<Option<u32>, Self::Error>;
    /// Mind `+1D0` byte store.
    fn set_dogfighter(&mut self, value: u8) -> Result<(), Self::Error>;
}

/// `517C60`: the returned AL.
pub fn activate<W: World + ?Sized>(world: &mut W) -> Result<u8, W::Error> {
    if world.stack_empty()? {
        world.clear_188_owner_byte_7c()?;
        world.join_formation(0)?;
        world.activate_188(2)?;
    }
    let base = world.base_activate()?;
    if base == 0 {
        return Ok(base);
    }
    world.set_dogfighter(0)?;
    let count = world.ability_count()?;
    let mut index = 0;
    while index < count {
        match world.ability_3c_word_2b4(index)? {
            Some(word) if word != 0 => index += 1,
            _ => {
                world.set_dogfighter(1)?;
                break;
            }
        }
    }
    Ok(1)
}
