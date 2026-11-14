//! Original city-block binding test `51D560` (fastcall on a block's `+94`
//! component), read by the Mind target admission `522CC0` (Mind virtual
//! `+A0`): a block whose `+34` city handle still resolves is refused.
//!
//! The handle (index in the low 16 bits, generation in the high 16) is
//! looked up in the GTID table `[B364C0]`: true when the `+4` generation word
//! at the index equals the handle's generation and the `+20004` slot at the
//! index is nonnull. Only the table entry is consulted, so a city that is
//! dead but not yet removed from the table still binds its blocks.
use crate::Result;

/// The two GTID table words `51D560` reads.
pub trait GtidTable {
    /// `[B364C0]+4+index*2`: the generation word.
    fn generation(&mut self, index: u16) -> Result<u16>;
    /// `[B364C0]+20004+index*4` is nonnull. Read only after a generation match.
    fn slot_live(&mut self, index: u16) -> Result<bool>;
}

/// `51D560`: `handle_34` is the component's `+34` word.
pub fn city_block_bound<T: GtidTable + ?Sized>(table: &mut T, handle_34: u32) -> Result<bool> {
    let index = (handle_34 & 0xffff) as u16;
    if (handle_34 >> 16) as u16 != table.generation(index)? {
        return Ok(false);
    }
    table.slot_live(index)
}
