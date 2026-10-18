//! Native resource-value Modifier getters501730/501A00/5019D0.
//! Resource entries are numeric values in the actual registered order. Opaque
//! saved handles are not silently reinterpreted as resource adjustments.
use crate::{Error, F32, Result};
use aa_numeric::{Binary, Precision};

///501730. `recruitable` is the exact45E5A0 result; word17C is source ActorData.
pub fn cost(
    base: F32,
    factor60: F32,
    recruitable: bool,
    word17c: u32,
    p: Precision,
) -> Result<Binary> {
    let base = base.number()?;
    if recruitable || word17c != 0 {
        Ok(base.mul(&factor60.number()?, p)?)
    } else {
        Ok(base)
    }
}
///501A00, selected by45F4D0 when definition flags174 bit15 is set.
pub fn special_cost(
    base: F32,
    factor4c: F32,
    definition_flags: u32,
    p: Precision,
) -> Result<Binary> {
    let base = base.number()?;
    if definition_flags & (1 << 13) != 0 {
        Ok(base.mul(&factor4c.number()?, p)?)
    } else {
        Ok(base)
    }
}
///5019D0. A table-size mismatch returns the original zero. A matching table uses
/// the requested resource slot; an invalid index is explicit instead of OOB.
pub fn production_delta(values: &[F32], registered_count: usize, index: usize) -> Result<Binary> {
    if values.len() != registered_count {
        return Ok(Binary::from_i32(0));
    }
    values
        .get(index)
        .ok_or_else(|| Error::Callback("production modifier resource index outside table".into()))?
        .number()
}
