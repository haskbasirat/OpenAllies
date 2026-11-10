//! Native City candidate test `5640A0(object)`, the City registration
//! `564300(object)` and the actor maximum value `4D01E0(0)` it weighs (with
//! the Organization maximum `578C00(0)`).
//!
//! `5640A0` admits an object as a City `+18` candidate when it is not null,
//! not the City actor (City `+4`), not being removed (`+8` bit 0), owned
//! (virtual `+108`) by a kingdom whose `+190` word is zero, not an
//! organization member (`4549C0`: `+80` component `+10`), not flying (`+8`
//! bit 12), of a maximum value above 0.0, without a `+60` Body or of a
//! definition (`+4`) with byte `+290` or `+291` set, and not one of the
//! City's blocks (City `+10`, `517F30` over its GTID).
//!
//! `564300` puts an admitted object's GTID at the head of City `+18`
//! (`4D0900`; `+1C` takes the node when it was null) unless it is listed
//! already, and registers the City's Updatable (`+8`, `50AC60(0)`) when its
//! `+C` word is zero.
//!
//! `4D01E0(0)` sums in ST0 from `[9F32DC]` (0.0): `+7C` present loads the
//! Organization `+6C` (`578C00` with a zero argument returns it unchanged,
//! replacing the 0.0), then adds the `+98` component `+34`, the Performer
//! (`+74`) `+60` and the Denizen (`+88`) `+14` at the caller's precision.
use crate::Result;
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// The fields `4D01E0(0)` reads, each behind its component pointer.
pub trait MaximumWorld {
    /// `+7C` OrganizationComponent present: its `+6C` (maximum value).
    fn organization_maximum(&mut self, actor: u32) -> Result<Option<F32>>;
    /// `+98` component present: its `+34`.
    fn component98_maximum(&mut self, actor: u32) -> Result<Option<F32>>;
    /// `+74` Performer present: its `+60`.
    fn performer_value(&mut self, actor: u32) -> Result<Option<F32>>;
    /// `+88` Denizen present: its `+14`.
    fn denizen_maximum(&mut self, actor: u32) -> Result<Option<F32>>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `4D01E0(0)` on `actor`: the value in ST0 at the return.
pub fn maximum_value(actor: u32, world: &mut impl MaximumWorld, p: Precision) -> Result<Binary> {
    let mut st = Binary::from_u32(0);
    if let Some(maximum) = world.organization_maximum(actor)? {
        st = n(maximum)?;
    }
    if let Some(v) = world.component98_maximum(actor)? {
        st = st.add(&n(v)?, p)?;
    }
    if let Some(v) = world.performer_value(actor)? {
        st = st.add(&n(v)?, p)?;
    }
    if let Some(v) = world.denizen_maximum(actor)? {
        st = st.add(&n(v)?, p)?;
    }
    Ok(st)
}

/// The services of `5640A0` and `564300`.
pub trait AdmissionWorld: MaximumWorld {
    /// City `+4`.
    fn city_actor(&mut self) -> Result<u32>;
    /// Object `+8`.
    fn flags8(&mut self, object: u32) -> Result<u32>;
    /// Object virtual `+108` (owner).
    fn owner(&mut self, object: u32) -> Result<Option<u32>>;
    /// Kingdom `+190`.
    fn kingdom_word190(&mut self, kingdom: u32) -> Result<u32>;
    /// `4549C0`: object `+80` component present and its `+10` (organization),
    /// else zero.
    fn membership_organization(&mut self, object: u32) -> Result<u32>;
    /// Object `+60` (Body) word.
    fn body_word(&mut self, object: u32) -> Result<u32>;
    /// Object definition (`+4`) bytes `+290` and `+291`.
    fn definition_bytes(&mut self, object: u32) -> Result<[u8; 2]>;
    /// Object `+14` (its GTID).
    fn gtid(&mut self, object: u32) -> Result<u32>;
    /// City `+10` (blocks) and `+18` (candidates) GTID words in list order.
    fn blocks(&mut self) -> Result<Vec<u32>>;
    fn candidates(&mut self) -> Result<Vec<u32>>;
    /// `4D0900`: a new City `+18` head node holding `gtid` (City `+1C`
    /// takes it when null).
    fn push_candidate(&mut self, gtid: u32) -> Result<()>;
    /// City Updatable (`+8`) `+C` word, and its registration `50AC60(0)`.
    fn updatable_word(&mut self) -> Result<u32>;
    fn register_update(&mut self) -> Result<()>;
}

/// Native `5640A0(object)` on a City component.
pub fn admits(world: &mut impl AdmissionWorld, object: Option<u32>, p: Precision) -> Result<bool> {
    let Some(object) = object else {
        return Ok(false);
    };
    if object == world.city_actor()? || world.flags8(object)? & 1 != 0 {
        return Ok(false);
    }
    if world.owner(object)?.is_none() {
        return Ok(false);
    }
    // 5640D5: the owner is read again (no null check) for its +190 word.
    let owner = world
        .owner(object)?
        .ok_or(crate::Error::Invalid("5640A0 owner vanished between its two reads"))?;
    if world.kingdom_word190(owner)? != 0 || world.membership_organization(object)? != 0 {
        return Ok(false);
    }
    if (world.flags8(object)? >> 12) & 1 != 0 {
        return Ok(false);
    }
    // 56410B: a maximum value below or equal to 0.0 refuses.
    if maximum_value(object, world, p)?.compare(&Binary::from_u32(0)) != Ordering::Greater {
        return Ok(false);
    }
    if world.body_word(object)? != 0 && world.definition_bytes(object)? == [0, 0] {
        return Ok(false);
    }
    let gtid = world.gtid(object)?;
    Ok(!world.blocks()?.contains(&gtid))
}

/// Native `564300(object)` on a City component.
pub fn register(world: &mut impl AdmissionWorld, object: Option<u32>, p: Precision) -> Result<()> {
    if !admits(world, object, p)? {
        return Ok(());
    }
    let gtid = match object {
        Some(object) => world.gtid(object)?,
        None => 0,
    };
    if world.candidates()?.contains(&gtid) {
        return Ok(());
    }
    world.push_candidate(gtid)?;
    if world.updatable_word()? == 0 {
        world.register_update()?;
    }
    Ok(())
}
