//! Original CAI state behaviour query `51D410` (a CAI state virtual; no
//! direct callers): whether the organization the state belongs to engages.
//!
//! The state's `+4` owner is asked for its state-group-4 descriptor with
//! `4D49B0(owner, 4)`. A descriptor whose `+24` word is exactly 1 answers 1,
//! exactly 0 answers 0; any other word, or no descriptor, answers the byte at
//! owner `+7C` (organization) `+8D`. The answer is the AL byte the callers
//! test; the `+8D` byte is returned unchanged, not reduced to 0/1.

/// The two reads `51D410` makes outside the state.
pub trait World {
    type Error;
    /// `4D49B0(owner, 4)` on the state's `+4` owner: the `+24` word of the
    /// state-group-4 descriptor, `None` when the lookup returns null.
    fn group4_word_24(&mut self) -> Result<Option<u32>, Self::Error>;
    /// Owner `+7C` organization `+8D` byte. Read only when the descriptor
    /// does not decide.
    fn organization_byte_8d(&mut self) -> Result<u8, Self::Error>;
}

/// `51D410`.
pub fn behavior<W: World + ?Sized>(world: &mut W) -> Result<u8, W::Error> {
    match world.group4_word_24()? {
        Some(1) => Ok(1),
        Some(0) => Ok(0),
        _ => world.organization_byte_8d(),
    }
}
