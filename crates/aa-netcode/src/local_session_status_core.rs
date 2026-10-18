//!Storage-independent773CE0 body after the actual12C lock is held.
use aa_numeric::{Binary, Precision};
pub(super) trait Fields {
    type Error: From<aa_numeric::Error>;
    fn display_store_148(&mut self) -> Result<(), Self::Error>;
    fn milliseconds(&mut self) -> Result<u32, Self::Error>;
    fn diagnostic(&mut self, seconds: u32) -> Result<(), Self::Error>;
    fn store_14c(&mut self, word: u32) -> Result<(), Self::Error>;
}
pub(super) fn update<F: Fields>(fields: &mut F, word: u32, pc: Precision) -> Result<(), F::Error> {
    fields.display_store_148()?;
    let millis = fields.milliseconds()?;
    let mut value = Binary::from_i32(millis as i32);
    if (millis as i32) < 0 {
        value = value.add(&Binary::from_f32_bits(0x4f800000)?, pc)?;
    }
    let spilled = value.store_f32()?;
    let seconds = Binary::from_f32_bits(spilled)?
        .div(&Binary::from_i32(1000), pc)?
        .store_f32()?;
    fields.diagnostic(seconds)?;
    fields.store_14c(word)
}
