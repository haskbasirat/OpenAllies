//! Original Construction command-mask contributor 565140 (and its upgrade
//! predicate 565090), dispatched by 4D1350 when actor+9C (Construction) is set.
//! Writes actor+F0/F4 bits 19 (cancel/finish), 20 (sell), 21 (upgrade) and
//! 22 (raze) through read-modify-write, in native order.
//!
//! GActor virtuals are ported here: +34 is 4D1F90 (action bit test), +100 is
//! 4CED60 (Structure parent, else Wall reference), +158 is 4CE980/588990
//! (this actor is its Structure parent's Settlement center), +15C is 4CE9A0.
use crate::{
    ActorId,
    ability_flags::{ActorFields, Error, Result, available, set_action},
    state,
};

/// ActorData +4E4 can_sell, +4E5 can_raze (cleared by 464BD0 when can_sell),
/// +4C0 outgoing Upgrade count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Definition {
    pub can_sell: bool,
    pub can_raze: bool,
    pub upgrade_count: i32,
}
/// Live component reads. `None` is an absent component; the inner option is
/// the component's actor reference word (null allowed).
pub trait World {
    fn fields(&self, actor: ActorId) -> Result<ActorFields>;
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()>;
    fn definition(&self, actor: ActorId) -> Result<Definition>;
    /// Structure component (actor+94) parent word +10.
    fn structure_parent(&self, actor: ActorId) -> Result<Option<Option<ActorId>>>;
    /// Wall component (actor+90) word +10 (58CB30).
    fn wall_reference(&self, actor: ActorId) -> Result<Option<Option<ActorId>>>;
    /// Settlement component (actor+98) word +14.
    fn settlement_reference(&self, actor: ActorId) -> Result<Option<Option<ActorId>>>;
    /// Properties component (actor+78) flags word +1C (4CF110).
    fn properties_flags(&self, actor: ActorId) -> Result<Option<u32>>;
    /// SVar "AllowSellUnderSiege" (4B2E60, cached by native static B3D65C).
    fn allow_sell_under_siege(&mut self) -> Result<bool>;
    /// 585F00 on the Structure parent's Settlement; reached only when
    /// ActorData+4C0 is nonzero.
    fn settlement_upgrades_ready(&mut self, parent: ActorId) -> Result<bool>;
}

const NORMAL: u8 = 27;
const UPGRADING: u8 = 29;
const RAZING: u8 = 30;

/// 4CED60 (GActor virtual +100).
pub fn parent(world: &impl World, actor: ActorId) -> Result<Option<ActorId>> {
    if let Some(Some(parent)) = world.structure_parent(actor)? {
        return Ok(Some(parent));
    }
    if let Some(Some(reference)) = world.wall_reference(actor)? {
        return Ok(Some(reference));
    }
    Ok(None)
}
/// 4CE980 / 588990 (GActor virtual +158). 588990 dereferences the parent's
/// Settlement without a null check; an absent one is reported, not guessed.
pub fn settlement_center(world: &impl World, actor: ActorId) -> Result<bool> {
    let Some(Some(parent)) = world.structure_parent(actor)? else {
        return Ok(false);
    };
    let center = world.settlement_reference(parent)?.ok_or(Error::Missing(
        "588990 reads a null Structure-parent Settlement (native null dereference)",
    ))?;
    Ok(center == Some(actor))
}
/// 4CEDA0: first Settlement +14 along the virtual +100 parent chain. The
/// native loop has no cycle guard (it would not terminate); cycles error here.
pub fn settlement_target(world: &impl World, actor: ActorId) -> Result<Option<ActorId>> {
    let mut current = actor;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        if let Some(target) = world.settlement_reference(current)? {
            return Ok(target);
        }
        if !seen.insert(current) {
            return Err(Error::Missing("4CEDA0 parent chain cycle (native does not terminate)"));
        }
        match parent(world, current)? {
            Some(next) => current = next,
            None => return Ok(None),
        }
    }
}
fn group13(fields: &ActorFields) -> Result<i32> {
    // 4D14C0(13) with the registered masks; -1 when no group bit is set.
    Ok(state::group(fields.state_bits, 13)
        .map_err(|e| Error::World(e.to_string()))?
        .map_or(-1, |d| i32::from(d.index)))
}
fn update(world: &mut impl World, actor: ActorId, index: u32, enabled: bool) -> Result<()> {
    let mut bits = world.fields(actor)?.action_bits;
    set_action(&mut bits, index, enabled)?;
    world.set_action_bits(actor, bits)
}
/// 565090(check_parent).
pub fn can_upgrade(world: &mut impl World, actor: ActorId, check_parent: bool) -> Result<bool> {
    if world.wall_reference(actor)?.is_some() {
        return Ok(false);
    }
    let fields = world.fields(actor)?;
    if fields.state_bits & (1 << 20) != 0 || fields.state_bits & (1 << 21) != 0 {
        return Ok(false);
    }
    if group13(&fields)? != i32::from(NORMAL) || world.definition(actor)?.upgrade_count == 0 {
        return Ok(false);
    }
    if check_parent && settlement_center(world, actor)? {
        // 4CE9A0 rereads virtual +158, then 585F00 on parent+98.
        let parent = if settlement_center(world, actor)? {
            world.structure_parent(actor)?.flatten()
        } else {
            None
        }
        .ok_or(Error::Missing("4CE9A0 returned null before 585F00"))?;
        if !world.settlement_upgrades_ready(parent)? {
            return Ok(false);
        }
    }
    Ok(true)
}
/// 565140.
pub fn contribute(world: &mut impl World, actor: ActorId) -> Result<()> {
    let entry = world.fields(actor)?;
    let mut selected = group13(&entry)?;
    let mut normal = selected == i32::from(NORMAL);
    let under_siege = entry.state_bits & (1 << 20) != 0;
    if let Some(target) = settlement_target(world, actor)?
        && world.fields(target)?.state_bits & (1 << 30) != 0
    {
        selected = i32::from(RAZING);
        normal = false;
    }
    if world.fields(actor)?.state_bits & (1 << 21) != 0 {
        for (index, enabled) in [(21, false), (20, false), (22, false), (19, true)] {
            update(world, actor, index, enabled)?;
        }
        return Ok(());
    }
    let upgrade = can_upgrade(world, actor, true)?;
    update(world, actor, 21, upgrade)?;
    let definition = world.definition(actor)?;
    let mut raze = false;
    if definition.can_raze && !under_siege {
        let parent_allows =
            parent(world, actor)?.is_none() || settlement_center(world, actor)?;
        raze = parent_allows && selected != i32::from(UPGRADING) && normal;
    }
    let properties = world.properties_flags(actor)?;
    let blocked = properties.is_some_and(|flags| flags & 0x800 != 0);
    if blocked {
        raze = false;
    }
    update(world, actor, 22, raze)?;
    update(world, actor, 19, false)?;
    let finish = if selected == i32::from(UPGRADING) {
        world.wall_reference(actor)?.is_none()
    } else if selected == i32::from(RAZING) {
        parent(world, actor)?.is_none() || settlement_center(world, actor)?
    } else {
        false
    };
    if finish {
        update(world, actor, 19, true)?;
    }
    let definition = world.definition(actor)?;
    let mut sell = definition.can_sell && !available(world.fields(actor)?.action_bits, 19)?;
    let allow = world.allow_sell_under_siege()?;
    if sell && under_siege && !allow {
        sell = false;
    }
    if blocked {
        sell = false;
    }
    update(world, actor, 20, sell)
}
