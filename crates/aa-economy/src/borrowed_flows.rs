//! Native4E48C0/4E4930/4E3940/4E1F60 on the caller's actual flow storage.
//! No flow allocation, resizing, state clone or rollback is performed. Every read
//! is required only when reached, so failed later dependencies retain writes.
//! Finite binary32 operands/results and nearest-even PC24/53/64 are supported;
//! unsupported numeric values fail at their first reached arithmetic boundary.
use crate::{EconomyError, F32Bits, ResourceKind, X87Precision, binary::Binary};
use std::cmp::Ordering;

pub type Result<T> = std::result::Result<T, EconomyError>;

/// Implement against authoritative vectors, not a detached Economy snapshot.
/// Counts are distinct native fields, and getters must not create missing
/// entries. Setters preserve their own prefix if they return an error.
pub trait World {
    /// Resource manager B3A660+28, captured by4E48C0.
    fn consumption_count(&self) -> Result<i32>;
    /// Resource catalog B36530+38, captured by4E4930.
    fn resource_count(&self) -> Result<i32>;
    /// Live Kingdom+1A4, reread by shortage loops.
    fn effective_count(&self) -> Result<i32>;
    fn effective_provided(&self, index: usize) -> Result<F32Bits>;
    fn raw_provided(&self, index: usize) -> Result<F32Bits>;
    fn consumed(&self, index: usize) -> Result<F32Bits>;
    fn set_effective_provided(&mut self, index: usize, value: F32Bits) -> Result<()>;
    fn set_raw_provided(&mut self, index: usize, value: F32Bits) -> Result<()>;
    fn set_consumed(&mut self, index: usize, value: F32Bits) -> Result<()>;
    fn resource_kind(&self, index: usize) -> Result<ResourceKind>;
    fn purchase_cost(&self, source: usize, destination: usize) -> Result<F32Bits>;
    fn limited_provider(&self, index: usize) -> Result<bool>;
    /// SVar cache B39AB0+1B4. Reached only for a limited provider, AFTER the
    /// raw-provided store; a missing setting must not roll back that store.
    fn provider_maximum(&mut self) -> Result<i32>;
}

fn finite(value: F32Bits) -> Result<Binary> {
    if !value.is_finite() {
        return Err(EconomyError::NonFiniteInput);
    }
    Ok(Binary::from_bits(value))
}

///4E1F60. Compare the extended difference before using its binary32 spill
/// in multiplication; preserve ascending source/destination and store order.
pub fn shortage(world: &mut impl World, add: bool, pc: X87Precision) -> Result<()> {
    let mut count = world.effective_count()?;
    let mut source = 0i32;
    while source < count {
        let index = source as usize;
        let provided = finite(world.effective_provided(index)?)?;
        let consumed = finite(world.consumed(index)?)?;
        let difference = provided.sub(&consumed, pc);
        let spilled = difference.store_f32()?;
        if difference.is_negative_nonzero() && world.resource_kind(index)? == ResourceKind::Upkeep {
            let mut destination = 0i32;
            while destination < count {
                let target = destination as usize;
                if world.resource_kind(target)? == ResourceKind::Stockpiled {
                    let cost = finite(world.purchase_cost(index, target)?)?;
                    let charge = Binary::from_bits(spilled).neg().mul(&cost, pc);
                    let old = finite(world.consumed(target)?)?;
                    let value = if add {
                        charge.add(&old, pc)
                    } else {
                        old.sub(&charge, pc)
                    };
                    world.set_consumed(target, value.store_f32()?)?;
                }
                count = world.effective_count()?;
                destination += 1;
            }
        }
        count = world.effective_count()?;
        source += 1;
    }
    Ok(())
}

///4E3940. Old shortage removal, raw store, conditional cap read, effective
/// store, then new shortage application. No upfront vector/index validation.
pub fn adjust_provider(
    world: &mut impl World,
    index: usize,
    delta: F32Bits,
    pc: X87Precision,
) -> Result<()> {
    shortage(world, false, pc)?;
    let delta = finite(delta)?;
    let old = finite(world.raw_provided(index)?)?;
    world.set_raw_provided(index, delta.add(&old, pc).store_f32()?)?;
    if world.limited_provider(index)? {
        let cap = Binary::from_i32(world.provider_maximum()?);
        let raw = world.raw_provided(index)?;
        let comparison = finite(raw)?;
        let value = if cap.cmp(&comparison) == Ordering::Less {
            cap.store_f32()?
        } else {
            raw
        };
        world.set_effective_provided(index, value)?;
    } else {
        // Native uses a raw32 copy in this branch, not an extra arithmetic op.
        let raw = world.raw_provided(index)?;
        world.set_effective_provided(index, raw)?;
    }
    shortage(world, true, pc)
}

fn input(values: &[F32Bits], index: usize, add: bool) -> Result<F32Bits> {
    let value = *values.get(index).ok_or(EconomyError::Shape)?;
    finite(value)?; // Native FLD/FCHS/FSTP precedes the first shortage call.
    Ok(if add {
        value
    } else {
        F32Bits(value.0 ^ 0x8000_0000)
    })
}

///4E4930; values must be the distinct actor-component source vector. Zero
/// deltas still perform complete old/new shortage passes for each resource.
pub fn adjust_providers(
    world: &mut impl World,
    values: &[F32Bits],
    add: bool,
    pc: X87Precision,
) -> Result<()> {
    let count = world.resource_count()?;
    for index in 0..count {
        let index = index as usize;
        adjust_provider(world, index, input(values, index, add)?, pc)?;
    }
    Ok(())
}

///4E48C0; the manager count is captured once. Input, remove, mutate, apply
/// is repeated per ascending index, including zero deltas.
pub fn adjust_consumption(
    world: &mut impl World,
    values: &[F32Bits],
    add: bool,
    pc: X87Precision,
) -> Result<()> {
    let count = world.consumption_count()?;
    for index in 0..count {
        let index = index as usize;
        let delta = Binary::from_bits(input(values, index, add)?);
        shortage(world, false, pc)?;
        let old = finite(world.consumed(index)?)?;
        world.set_consumed(index, delta.add(&old, pc).store_f32()?)?;
        shortage(world, true, pc)?;
    }
    Ok(())
}
