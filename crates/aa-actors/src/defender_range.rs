//! Defender range getters and the range refresh after a modifier change.
//!
//! - `0x45D6C0` (definition +218) and `0x45D4C0` (definition +2FC): without
//!   a modifier the field itself; with one, `0x501520`: the field times the
//!   modifier's +54 factor, one x87 multiply at the running precision.
//! - `0x566F80`: the Defender component's +2C becomes that range stored to
//!   binary32 (the organization getter when definition +174 bit 7 is set),
//!   and its +30 byte is set. The definition is component +4 -> actor +4
//!   (`0x529C30`).
//!
//! Proven at PC24, PC53 and PC64 (the 80-bit getter results and the stored
//! word) by research/phase8-combat/native_defender_range.py (no seams).
use crate::actor_creation::Result;
use aa_body::F32;
use aa_numeric::{Fast, Precision};

/// `0x45D6C0` / `0x45D4C0`: `field`, or `field` times the modifier's +54
/// factor (`0x501520`), as the x87 register value.
pub fn modified_range(field: F32, modifier_factor: Option<F32>, p: Precision) -> Result<Fast> {
    let value = Fast::from_f32_bits(field.0)?;
    Ok(match modifier_factor {
        None => value,
        Some(factor) => value.mul(&Fast::from_f32_bits(factor.0)?, p)?,
    })
}

/// The definition words `0x566F80` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeDefinition {
    /// Definition +174 (bit 7: an organization).
    pub mask: u32,
    /// Definition +218 (`0x45D6C0`).
    pub defender_range: F32,
    /// Definition +2FC (`0x45D4C0`).
    pub organization_range: F32,
}

/// `0x566F80`: the new Defender +2C word (the caller also sets +30 to 1).
pub fn refreshed_range(definition: &RangeDefinition, modifier_factor: Option<F32>, p: Precision) -> Result<F32> {
    let field = if definition.mask >> 7 & 1 != 0 {
        definition.organization_range
    } else {
        definition.defender_range
    };
    Ok(F32(modified_range(field, modifier_factor, p)?.store_f32()?))
}
