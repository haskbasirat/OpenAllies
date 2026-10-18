//! Actor BeginDeath `0x4D7960` (called by Body CheckDeath `0x561C20`).
//!
//! Orchestration only: every native callee is a world service at the point
//! the original calls it, in the original order. Kill credit is aa-body
//! `credit::credit_kill` (`0x4D57C0`), the siege refresh is
//! [`crate::siege::refresh_under_siege`] (`0x4D6660`), start dying is aa-body
//! `dying::start_dying` (`0x562380`) and removal is aa-world's `0x4F6760`.
//! Proven by research/phase8-combat/native_begin_death.py.
use crate::Result;

/// Actor component slots (`actor + 0x60 + 4 * index`, 31 slots).
pub const COMPONENT_BASE: u32 = 0x60;
pub const COMPONENT_COUNT: u32 = 31;

pub trait World {
    /// Actor `+8` flags.
    fn flags8(&self, actor: u32) -> Result<u32>;
    fn set_flags8(&mut self, actor: u32, flags: u32) -> Result<()>;
    /// Actor `+100` low dword (state bits 0..31).
    fn flags100(&self, actor: u32) -> Result<u32>;
    /// `0x4D57C0` kill crediting: the credited source passed to the
    /// components' death notification (`None` = null).
    fn credit_kill(&mut self, actor: u32) -> Result<Option<u32>>;
    /// `+148[i]` for `i < +14C`; `None` for a null entry.
    fn zone_entries(&self, actor: u32) -> Result<Vec<Option<u32>>>;
    /// Zone entry `+14` word.
    fn zone_word(&self, entry: u32) -> Result<u16>;
    /// `0x514CE0` on a zone entry.
    fn zone_release(&mut self, entry: u32) -> Result<()>;
    /// Actor component pointer at `offset` (`None` = null).
    fn component(&self, actor: u32, offset: u32) -> Result<Option<u32>>;
    /// `+80` component `+10` word nonzero.
    fn component_80_word10(&self, component: u32) -> Result<bool>;
    /// `+C0` component `+38` byte nonzero.
    fn component_c0_byte38(&self, component: u32) -> Result<bool>;
    /// `0x58CB30` on the `+90` component (its `+10`), `None` = null.
    fn wall_owner(&self, component_90: u32) -> Result<Option<u32>>;
    /// `0x5853B0` on that object's `+98` component.
    fn wall_owner_check(&mut self, owner: u32) -> Result<bool>;
    /// Actor virtual `+100`: `None` = null, else its `+8` bit 0.
    fn virtual_100(&mut self, actor: u32) -> Result<Option<bool>>;
    /// Actor virtual `+20(1)`.
    fn virtual_20(&mut self, actor: u32) -> Result<bool>;
    /// `0x42D490([B39D8C], message, actor virtual +108(...))`: message 0x18
    /// passes the actor only; 0x12/0x13 also pass virtual `+24`'s name.
    fn announce(&mut self, actor: u32, message: u32, with_name: bool) -> Result<()>;
    /// `0x4D64F0(state, enabled)`.
    fn set_state(&mut self, actor: u32, state: u32, enabled: bool) -> Result<()>;
    /// `0x4D6660`.
    fn refresh_under_siege(&mut self, actor: u32) -> Result<()>;
    /// Component virtual `+34(credited)`.
    fn component_death(&mut self, component: u32, credited: Option<u32>) -> Result<()>;
    /// `+70` component `+28` GTID resolved in the registry: that object's
    /// `+88` component, if all exist.
    fn linked_component_88(&self, actor: u32) -> Result<Option<u32>>;
    /// `0x568B70(component88; actor)`.
    fn component_88_death(&mut self, component: u32, actor: u32) -> Result<()>;
    /// Actor virtual `+158`.
    fn is_wall_member(&mut self, actor: u32) -> Result<bool>;
    /// Actor virtual `+15C`, then `0x5859C0` and `0x584D50(0)` on its `+98`.
    fn wall_member_died(&mut self, actor: u32) -> Result<()>;
    /// `0x4EE8E0(0)`.
    fn stop(&mut self, actor: u32) -> Result<()>;
    /// `0x4D1350`.
    fn notify(&mut self, actor: u32) -> Result<()>;
    /// `0x4D37E0`.
    fn refresh_37e0(&mut self, actor: u32) -> Result<()>;
    /// Actor definition `+1B2` byte.
    fn definition_1b2(&self, actor: u32) -> Result<bool>;
    /// Actor virtual `+50(0)`.
    fn virtual_50(&mut self, actor: u32) -> Result<()>;
    /// `0x562380` on the `+60` body.
    fn start_dying(&mut self, actor: u32) -> Result<()>;
    /// `world.0x4F6760(actor, 0)`; true when the actor was removed.
    fn request_removal(&mut self, actor: u32) -> Result<bool>;
}

/// Native `0x4D7960`.
pub fn begin_death(actor: u32, world: &mut impl World) -> Result<()> {
    if world.flags8(actor)? & 1 != 0 {
        return Ok(());
    }
    let credited = world.credit_kill(actor)?;
    for entry in world.zone_entries(actor)?.into_iter().flatten() {
        // 0x514CD0: a word with 0x42 set is skipped.
        if world.zone_word(entry)? & 0x42 == 0 {
            world.zone_release(entry)?;
        }
    }
    announce(actor, world)?;
    let flags = world.flags8(actor)? | 1;
    world.set_flags8(actor, flags)?;
    world.set_state(actor, 0x17, true)?;
    world.refresh_under_siege(actor)?;
    if let Some(component) = world.component(actor, 0xa4)? {
        world.component_death(component, credited)?;
    }
    for index in 0..COMPONENT_COUNT {
        if index == 0x11 || index == 8 {
            continue;
        }
        if let Some(component) = world.component(actor, COMPONENT_BASE + 4 * index)? {
            world.component_death(component, credited)?;
        }
    }
    if let Some(component) = world.component(actor, 0x80)? {
        world.component_death(component, credited)?;
    }
    if world.component(actor, 0x70)?.is_some()
        && let Some(component) = world.linked_component_88(actor)?
    {
        world.component_88_death(component, actor)?;
    }
    if world.is_wall_member(actor)? {
        world.wall_member_died(actor)?;
    }
    world.stop(actor)?;
    world.notify(actor)?;
    world.refresh_37e0(actor)?;
    if !world.definition_1b2(actor)? {
        world.virtual_50(actor)?;
    }
    if world.component(actor, 0x68)?.is_none()
        && world.component(actor, 0xbc)?.is_none()
        && world.flags100(actor)? & 0x0100_0000 == 0
    {
        if world.component(actor, 0x60)?.is_some() {
            world.start_dying(actor)?;
        } else if world.request_removal(actor)? {
            return Ok(());
        }
    }
    let flags = world.flags8(actor)? | 2;
    world.set_flags8(actor, flags)
}

/// 4D79E1..4D7B2D: death announcement selection.
fn announce(actor: u32, world: &mut impl World) -> Result<()> {
    if world.component(actor, 0x64)?.is_some() {
        return Ok(());
    }
    if let Some(component) = world.component(actor, 0x80)?
        && world.component_80_word10(component)?
    {
        return Ok(());
    }
    if world.flags100(actor)? & 0x2000_0000 != 0 {
        return Ok(());
    }
    if let Some(component) = world.component(actor, 0x90)?
        && let Some(owner) = world.wall_owner(component)?
        && !world.wall_owner_check(owner)?
    {
        return world.announce(actor, 0x18, false);
    }
    let flag = world.virtual_100(actor)?.unwrap_or(false);
    let structure =
        world.component(actor, 0x94)?.is_some() || world.component(actor, 0x88)?.is_some();
    if structure && flag {
        return Ok(());
    }
    let announced = (world.virtual_20(actor)? && world.component(actor, 0x90)?.is_none())
        || match world.component(actor, 0xc0)? {
            Some(component) => !world.component_c0_byte38(component)?,
            None => false,
        };
    if !announced {
        return Ok(());
    }
    let message = (!(world.flags8(actor)? >> 29) & 1) | 0x12;
    world.announce(actor, message, true)
}
