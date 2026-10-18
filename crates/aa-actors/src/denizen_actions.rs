//! Original Denizen action contributor569C70. Reads the same owned component
//! at every native access, and stores action bits23..28 in native order.
//! There are no allocations, floating point operations, or RNG calls here.
use crate::{
    ActorId,
    ability_flags::{ActorFields, Result, set_action},
};

/// Concrete live storage reads. The outer option distinguishes an absent
/// component; the inner option is its nullable Actor reference.
pub trait World {
    fn denizen_owner(&self, component: ActorId) -> Result<ActorId>;
    fn fields(&self, actor: ActorId) -> Result<ActorFields>;
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()>;
    fn denizen_byte(&self, component: ActorId, offset: u32) -> Result<u8>;
    /// 4CED40 only tests component presence; no Settlement payload is read.
    fn settlement_present(&self, actor: ActorId) -> Result<bool>;
    /// Structure+10, or None when the component is absent.
    fn structure_parent(&self, actor: ActorId) -> Result<Option<Option<ActorId>>>;
    /// Wall58CB30 (+10), or None when the component is absent.
    fn wall_reference(&self, actor: ActorId) -> Result<Option<Option<ActorId>>>;
}

fn update(world: &mut impl World, component: ActorId, index: u32, value: bool) -> Result<()> {
    let actor = world.denizen_owner(component)?;
    let mut bits = world.fields(actor)?.action_bits;
    set_action(&mut bits, index, value)?;
    world.set_action_bits(actor, bits)
}

/// Native4CED40/4CED60: Settlement presence returns the actor itself;
/// otherwise Structure+10, then Wall+10. Native rereads a nonnull Wall+10.
pub fn settlement_or_parent(world: &impl World, actor: ActorId) -> Result<Option<ActorId>> {
    if world.settlement_present(actor)? {
        return Ok(Some(actor));
    }
    if let Some(Some(parent)) = world.structure_parent(actor)? {
        return Ok(Some(parent));
    }
    if let Some(Some(_)) = world.wall_reference(actor)? {
        return Ok(world.wall_reference(actor)?.flatten());
    }
    Ok(None)
}

/// Denizen569C70, dispatched by GActor4D1350 for actor+88. State30 clears
/// all six bits and returns before any parent or Denizen flag read. Otherwise
/// missing capability flags preserve bits23/24 and27/28, but always write25/26.
pub fn contribute(world: &mut impl World, component: ActorId) -> Result<()> {
    let owner = world.denizen_owner(component)?;
    if world.fields(owner)?.state_bits & (1 << 30) != 0 {
        for index in 23..=28 {
            update(world, component, index, false)?;
        }
        return Ok(());
    }
    if settlement_or_parent(world, owner)?.is_none() {
        update(world, component, 23, false)?;
        update(world, component, 24, false)?;
    } else if world.denizen_byte(component, 0x38)? & 8 != 0 {
        let disabled = world.denizen_byte(component, 0x20)? == 0;
        update(world, component, 23, disabled)?;
        let enabled = world.denizen_byte(component, 0x20)? != 0;
        update(world, component, 24, enabled)?;
    }
    if world.denizen_byte(component, 0x38)? & 0x80 != 0 {
        let disabled = world.denizen_byte(component, 0x21)? == 0;
        update(world, component, 27, disabled)?;
        let enabled = world.denizen_byte(component, 0x21)? != 0;
        update(world, component, 28, enabled)?;
    }
    let enabled = world.denizen_byte(component, 0x38)? & 0x10 != 0
        && world.denizen_byte(component, 0x3c)? & 0x10 != 0;
    update(world, component, 25, enabled)?;
    let enabled = world.denizen_byte(component, 0x38)? & 0x40 != 0
        && world.denizen_byte(component, 0x3c)? & 0x40 != 0;
    update(world, component, 26, enabled)
}
