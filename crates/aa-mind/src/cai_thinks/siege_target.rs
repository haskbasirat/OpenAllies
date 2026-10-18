//! Siege state (regiment Mind+F8, vtable A32CC8) target helpers in the Tai
//! range, ported because the kill and siege Thinks call them:
//! - `51D130(object)`: the object's siege site, or null.
//! - `51DA20(mode, object)`: activate the siege state for a target.
use super::RegimentWorld;
use crate::{Arena, MindId, Result, StateId, activate};

/// Services in native order.
pub trait SiegeTargetWorld: RegimentWorld {
    /// Object+90 word.
    fn object_word90(&mut self, object: u32) -> Result<u32>;
    /// `4D11D0` on the object (its site object), `None` when null.
    fn object_4d11d0(&mut self, object: u32, arena: &mut Arena) -> Result<Option<u32>>;
    /// Organization (`529C30`, Mind+4) virtual +16C(site).
    fn organization_virtual_16c(
        &mut self,
        mind: MindId,
        site: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// `57A500(object, a, b)` on the organization component (organization
    /// +7C, passed as read: the call itself is the service).
    fn component_57a500(
        &mut self,
        mind: MindId,
        object: u32,
        a: bool,
        b: bool,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Siege state+10 (target GTID) and +14 (site GTID) words.
    fn set_siege_words(&mut self, state: StateId, target: u32, site: u32) -> Result<()>;
}

/// `51D130` on the siege state: a null object or one with a nonzero +90 word
/// has no site; otherwise its `4D11D0` site counts when the organization
/// accepts it (+16C) and the organization component accepts the object
/// (`57A500(object, 0, 0)`).
pub fn site(
    state: StateId,
    object: Option<u32>,
    arena: &mut Arena,
    world: &mut impl SiegeTargetWorld,
) -> Result<Option<u32>> {
    let Some(object) = object else {
        return Ok(None);
    };
    if world.object_word90(object)? != 0 {
        return Ok(None);
    }
    let Some(site) = world.object_4d11d0(object, arena)? else {
        return Ok(None);
    };
    let mind = arena.state(state)?.parent;
    if !world.organization_virtual_16c(mind, site, arena)? {
        return Ok(None);
    }
    let accepted = world.component_57a500(mind, object, false, false, arena)?;
    Ok(accepted.then_some(site))
}

/// `51DA20(mode, object)` on the siege state: without a site, false; then the
/// `529C00` eligibility (modes other than 0 and 1 ask state virtual +4),
/// state+10 = the object's GTID (0 for null), state+14 = the site's GTID and
/// the `52A810(mode)` activation result.
pub fn activate_for(
    state: StateId,
    mode: u32,
    object: Option<u32>,
    arena: &mut Arena,
    world: &mut impl SiegeTargetWorld,
) -> Result<bool> {
    let Some(site) = site(state, object, arena, world)? else {
        return Ok(false);
    };
    if mode != 0 && mode != 1 && !world.can_activate(state, arena)? {
        return Ok(false);
    }
    let target = match object {
        Some(object) => world.object_gtid(object)?,
        None => 0,
    };
    let site = world.object_gtid(site)?;
    world.set_siege_words(state, target, site)?;
    activate(state, mode, arena, world)
}
