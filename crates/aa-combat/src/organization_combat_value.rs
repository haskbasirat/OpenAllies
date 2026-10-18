//! Borrowed native Organization `5788B0`, including Performer `57DC40`.
//!
//! The caller owns the actual Organization. Every read and store is a required
//! service; no projected Organization or success-only copyback is used. Handles
//! preserve native pointer identity across intervening reads/stores. In
//! particular the member-array pointer is recaptured at `5788E8`, before the
//! current-value store, and the member/Performer are read again afterwards.
use crate::{Error, Result};
use aa_body::F32;
use aa_numeric::{Binary, Precision};

pub trait World {
    type Members: Copy;
    type Member: Copy;
    type Performer: Copy;
    type Slot: Copy;

    /// Organization `+2C`, a signed count; read before resets and after each slot.
    fn member_count(&mut self) -> Result<i32>;
    /// Capture Organization `+28` pointer identity, not a copy of its contents.
    fn member_array(&mut self) -> Result<Self::Members>;
    fn member(&mut self, array: Self::Members, index: i32) -> Result<Option<Self::Member>>;
    /// Member `+74`; the second call must not read the Performer's Body.
    fn performer(&mut self, member: Self::Member) -> Result<Option<Self::Performer>>;
    /// `57DC40`: Performer `+4` owner -> owner `+60` Body -> `+10/+14`.
    fn performer_health(&mut self, performer: Self::Performer) -> Result<Option<[F32; 2]>>;
    /// Performer `+60`, independently read for the current and maximum values.
    fn performer_value(&mut self, performer: Self::Performer) -> Result<F32>;
    /// Organization layout `+18/+28[index]` selects current `+1C` parameters.
    fn slot(&mut self, index: i32) -> Result<Option<Self::Slot>>;
    fn slot_flags(&mut self, slot: Self::Slot) -> Result<u32>;
    /// ActorData `+1E4`. Called only after flags `+174` bit5 was observed set.
    fn slot_value(&mut self, slot: Self::Slot) -> Result<F32>;
    fn current(&mut self) -> Result<F32>;
    fn maximum(&mut self) -> Result<F32>;
    /// Store the actual Organization `+68` immediately, including resets.
    fn store_current(&mut self, value: F32) -> Result<()>;
    /// Store the actual Organization `+6C` immediately, including resets.
    fn store_maximum(&mut self, value: F32) -> Result<()>;
}

pub(crate) fn health_ratio(health: Option<[F32; 2]>, p: Precision) -> Result<Binary> {
    match health {
        Some([health, maximum]) => {
            Ok(Binary::from_f32_bits(health.0)?.div(&Binary::from_f32_bits(maximum.0)?, p)?)
        }
        None => Ok(Binary::from_u32(1)),
    }
}

pub(crate) fn add(value: &Binary, accumulated: F32, p: Precision) -> Result<Binary> {
    Ok(value.add(&Binary::from_f32_bits(accumulated.0)?, p)?)
}

/// Execute the exact read/arithmetic/store order of `5788B0` on borrowed state.
///
/// A missing dependency returns its error with all earlier stores retained.
/// A member or Performer disappearing at the unchecked second dereference is
/// an explicit invalid-input error, not an invented zero contribution. Numeric
/// inputs are bounded by `aa_numeric::Binary`'s supported finite arithmetic.
pub fn recompute(world: &mut impl World, p: Precision) -> Result<()> {
    let count = world.member_count()?; // 5788B3, before either reset
    world.store_current(F32::ZERO)?;
    world.store_maximum(F32::ZERO)?;
    if count <= 0 {
        return Ok(());
    }
    let mut array = world.member_array()?;
    let mut index = 0i32;
    loop {
        if let Some(member) = world.member(array, index)? {
            if let Some(performer) = world.performer(member)? {
                let ratio = health_ratio(world.performer_health(performer)?, p)?;
                let value = Binary::from_f32_bits(world.performer_value(performer)?.0)?;
                let weighted = ratio.mul(&value, p)?;
                let current = add(&weighted, world.current()?, p)?;
                array = world.member_array()?; // 5788E8, before fstp +68
                world.store_current(F32(current.store_f32()?))?;
                let member = world.member(array, index)?.ok_or(Error::Invalid(
                    "member vanished during combat-value refresh",
                ))?;
                let performer = world
                    .performer(member)?
                    .ok_or(Error::Invalid("member combat component vanished"))?;
                let value = Binary::from_f32_bits(world.performer_value(performer)?.0)?;
                let maximum = add(&value, world.maximum()?, p)?;
                world.store_maximum(F32(maximum.store_f32()?))?;
            }
        } else if let Some(slot) = world.slot(index)?
            && world.slot_flags(slot)? & 0x20 != 0
        {
            let value = Binary::from_f32_bits(world.slot_value(slot)?.0)?;
            let maximum = add(&value, world.maximum()?, p)?;
            world.store_maximum(F32(maximum.store_f32()?))?;
        }
        let count = world.member_count()?; // 578926, before inc/cmp
        index = index.wrapping_add(1);
        if index >= count {
            return Ok(());
        }
    }
}
