//! Native actor combat value `4D0180` (the "current" value the City tick
//! `5647A0` and the strategic AI weigh) with the Performer value `57DC40`.
//!
//! `4D0180` keeps its sum in ST0 and returns it unstored: the City tick adds
//! that extended value to a binary32 accumulator before the store. Only one
//! intermediate is rounded to binary32 (the `+98` sum stored to the local the
//! Performer term is added to); every other step is an x87 operation at the
//! caller's precision control. The `0.0` start is `[9F32DC]` and the `1.0`
//! health ratio of a Performer without a Body is `[9F32E0]` (both read from
//! AA.exe).
use crate::Result;
use aa_body::F32;
use aa_numeric::{Binary, Precision};

/// The fields `4D0180` reads, each behind its component pointer.
pub trait World {
    /// `+7C` OrganizationComponent present: its `+68` (current value).
    fn organization_current(&mut self, actor: u32) -> Result<Option<F32>>;
    /// `+98` component present: its `+30`.
    fn component98_current(&mut self, actor: u32) -> Result<Option<F32>>;
    /// `+74` Performer present: its `+60` value and, when the Performer's
    /// `+4` owner has a `+60` Body, that Body's `+10` (health) and `+14`
    /// (maximum health).
    fn performer(&mut self, actor: u32) -> Result<Option<(F32, Option<[F32; 2]>)>>;
    /// `+88` Denizen present: its `+10` (current value).
    fn denizen_current(&mut self, actor: u32) -> Result<Option<F32>>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `57DC40`: `health / maximum * value` (`fld`, `fdiv`, `fmul`), or
/// `1.0 * value` without a Body. Returned unstored (ST0).
pub fn performer_value(value: F32, body: Option<[F32; 2]>, p: Precision) -> Result<Binary> {
    let ratio = match body {
        Some([health, maximum]) => n(health)?.div(&n(maximum)?, p)?,
        None => Binary::from_u32(1),
    };
    Ok(ratio.mul(&n(value)?, p)?)
}

/// Native `4D0180` on `actor`: the value in ST0 at the return.
pub fn value(actor: u32, world: &mut impl World, p: Precision) -> Result<Binary> {
    // 4D0181: fld [9F32DC] (0.0), fst local
    let mut st = Binary::from_u32(0);
    let mut local = F32::ZERO;
    if let Some(current) = world.organization_current(actor)? {
        st = n(current)?;
        local = current;
    }
    if let Some(current) = world.component98_current(actor)? {
        st = st.add(&n(current)?, p)?;
        local = F32(st.store_f32()?);
    }
    if let Some((performer, body)) = world.performer(actor)? {
        // 4D01B6: the running sum is popped; the Performer value is added to
        // the binary32 local.
        st = performer_value(performer, body, p)?.add(&n(local)?, p)?;
    }
    if let Some(current) = world.denizen_current(actor)? {
        st = st.add(&n(current)?, p)?;
    }
    Ok(st)
}
