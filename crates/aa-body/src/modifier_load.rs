//! Numeric definition conversion after the native TGI reader and attribute lookup.
//! This does not infer attribute IDs or parse files. The caller supplies their
//! resolved, source-bound values. Diagnostics become explicit construction errors.
use crate::{
    Error, F32, Result,
    property_effects::{NumericKind, NumericModifier},
};
use aa_numeric::Precision;
use std::cmp::Ordering;

/// Numeric parts of4FEC70/4FED70 after the amount's binary32 field store.
/// The returned amount is ready for `property_effects` aggregation. A failed
/// construction returns no record; it does not emulate a partially loaded object.
pub fn numeric(
    kind: NumericKind,
    attribute: i32,
    authored_amount: F32,
    repair: bool,
    p: Precision,
) -> Result<NumericModifier> {
    authored_amount.number()?;
    let valid = match kind {
        NumericKind::Add => (0..=12).contains(&attribute),
        NumericKind::Multiply => (13..=24).contains(&attribute),
    };
    if !valid {
        return Err(Error::Callback("unknown numeric modifier attribute".into()));
    }
    let amount = if kind == NumericKind::Multiply && [18, 23].contains(&attribute) {
        F32(authored_amount
            .number()?
            .mul(&F32(0xbf80_0000).number()?, p)?
            .store_f32()?)
    } else {
        authored_amount
    };
    if kind == NumericKind::Multiply
        && (amount.compare(F32(0xbf80_0000))? == Ordering::Less
            || amount.compare(F32(0x40a0_0000))? == Ordering::Greater)
    {
        return Err(Error::Callback(
            "multiply modifier amount outside [-1, 5]".into(),
        ));
    }
    // Multiply has no repair field/read. Reject a caller trying to attach one.
    if repair && (kind != NumericKind::Add || attribute != 4) {
        return Err(Error::Callback(
            "repair modifier requires native Add index4".into(),
        ));
    }
    Ok(NumericModifier {
        kind,
        index: attribute as u32,
        amount,
        repair,
    })
}
