//! Native actor command-availability masks (4D1350), Element571640 and
//! Organization57B390/57ABE0. These are actor+F0/F4, distinct from state+100/+104.
use crate::{ActorId, components::ComponentSlot, state};
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("command bit outside proved64-bit domain: {0}")]
    Bit(u32),
    #[error("required actor flag field unavailable: {0}")]
    Missing(&'static str),
    #[error("actor flag world operation failed: {0}; earlier writes remain committed")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug)]
pub struct ActorFields {
    pub thing_flags: u32,
    pub state_bits: u64,
    pub action_bits: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct ElementView {
    pub owner: ActorId,
    pub definition_present: bool,
    pub organization: Option<ActorId>,
    pub ready: bool,
}
#[derive(Clone, Debug)]
pub struct AttackGroup {
    /// Layout group byte+D.
    pub locked: bool,
    /// Ordered ActorData+1EC local AbilityData entries' definition+54 flags.
    /// Historical field name: these are ability flags, not ActorClass data.
    pub class_flags: Vec<u32>,
}
/// The source/state methods must read the authoritative live arena. They are
/// typed field access, not success defaults; missing dependencies are errors.
pub trait World {
    fn actor_fields(&self, actor: ActorId) -> Result<ActorFields>;
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()>;
    fn has_component(&self, actor: ActorId, slot: ComponentSlot) -> Result<bool>;
    fn element(&self, actor: ActorId) -> Result<ElementView>;
    fn organization_member_count(&self, actor: ActorId) -> Result<usize>;
    fn organization_member(&self, actor: ActorId, index: usize) -> Result<Option<ActorId>>;
    fn organization_member_44(&self, actor: ActorId) -> Result<Option<ActorId>>;
    fn organization_reference_d0(&self, actor: ActorId) -> Result<ActorId>;
    fn valid_reference(&self, id: ActorId) -> Result<bool>;
    fn organization_group_count(&self, actor: ActorId) -> Result<usize>;
    fn organization_attack_group(&self, actor: ActorId, index: usize) -> Result<AttackGroup>;
    /// 578A30 tests group+D before dereferencing its parameter when requested.
    fn organization_group_locked(&self, actor: ActorId, group: usize) -> Result<bool> {
        Ok(self.organization_attack_group(actor, group)?.locked)
    }
    fn organization_group_ability_count(&self, actor: ActorId, group: usize) -> Result<usize> {
        Ok(self
            .organization_attack_group(actor, group)?
            .class_flags
            .len())
    }
    fn organization_group_ability_flags(
        &self,
        actor: ActorId,
        group: usize,
        ability: usize,
    ) -> Result<u32> {
        self.organization_attack_group(actor, group)?
            .class_flags
            .get(ability)
            .copied()
            .ok_or(Error::Missing("578A30 local AbilityData index"))
    }
    fn owner(&mut self, actor: ActorId) -> Result<Option<ActorId>>;
    /// Actual loaded Kingdom+218, reread after the second virtual owner dispatch.
    fn kingdom_word_218(&self, owner: ActorId) -> Result<u32>;
    /// Original ActorState descriptor+1C; caller binds authored resupply.
    fn state_resupply(&self, state_index: u8) -> Result<bool>;
    /// Native owner4E1430 with the actual Organization definition.
    fn owner_can_recruit(&mut self, owner: ActorId, actor: ActorId) -> Result<bool>;
    /// Native owner4E1E40(actor), called even after can_recruit returnedfalse.
    fn owner_recruit_count(&mut self, owner: ActorId, actor: ActorId) -> Result<i32>;
    /// Actual virtual+34; ordinaryGActor implementations can use `available`.
    fn action_available(&mut self, actor: ActorId, index: u32) -> Result<bool>;
    /// Nonground contributor virtuals retain exact native call order. Must bind
    /// concrete behavior or reject; they cannot be silently skipped.
    fn other_contributor(&mut self, actor: ActorId, slot: ComponentSlot) -> Result<()>;
    fn mind_contributor(&mut self, actor: ActorId) -> Result<()>;
    /// Original4CFCC0, called only when the completed mask differs from its entry.
    fn changed(&mut self, actor: ActorId) -> Result<()>;
}
pub fn available(bits: u64, index: u32) -> Result<bool> {
    if index >= 64 {
        return Err(Error::Bit(index));
    }
    Ok(bits & (1u64 << index) != 0)
}
pub fn set_action(bits: &mut u64, index: u32, enabled: bool) -> Result<()> {
    if index >= 64 {
        return Err(Error::Bit(index));
    }
    if enabled {
        *bits |= 1u64 << index;
    } else {
        *bits &= !(1u64 << index);
    }
    Ok(())
}
fn set(world: &mut impl World, actor: ActorId, index: u32, enabled: bool) -> Result<()> {
    let mut bits = world.actor_fields(actor)?.action_bits;
    set_action(&mut bits, index, enabled)?;
    world.set_action_bits(actor, bits)
}
fn nonnull_member_count(world: &impl World, actor: ActorId) -> Result<usize> {
    let mut count = 0;
    let mut i = 0;
    while i < world.organization_member_count(actor)? {
        if world.organization_member(actor, i)?.is_some() {
            count += 1;
        }
        i += 1;
    }
    Ok(count)
}
/// Original57B2E0's typed resupply refusal code. Zero alone permits resupply.
pub fn resupply_code(world: &mut impl World, actor: ActorId) -> Result<u32> {
    if world.owner(actor)?.is_some() {
        let owner = world
            .owner(actor)?
            .ok_or(Error::Missing("second owner after nonnull first owner"))?;
        if world.kingdom_word_218(owner)? == 0 {
            return Ok(7);
        }
    }
    let bits = world.actor_fields(actor)?.state_bits;
    if bits & (1 << 19) != 0 {
        return Ok(4);
    }
    let descriptor = state::group(bits, 4).map_err(|e| Error::World(e.to_string()))?;
    let Some(descriptor) = descriptor else {
        return Ok(5);
    };
    if !world.state_resupply(descriptor.index)? {
        return Ok(5);
    }
    Ok(if nonnull_member_count(world, actor)? > 1 {
        0
    } else {
        6
    })
}
/// Original571640. Rereadsready/ownerflags after an Organization query.
pub fn element(world: &mut impl World, actor: ActorId) -> Result<()> {
    let entry = world.element(actor)?;
    let mut enabled = false;
    if entry.definition_present {
        let refusal = if let Some(org) = entry.organization {
            if !world.has_component(org, ComponentSlot::Organization)? {
                return Err(Error::Missing("Organization for Element parent"));
            }
            resupply_code(world, org)?
        } else {
            0
        };
        if refusal == 0 && world.element(actor)?.ready {
            enabled = world.actor_fields(world.element(actor)?.owner)?.thing_flags & 1 == 0;
        }
    }
    set(world, world.element(actor)?.owner, 30, enabled)
}
fn any_member_action(world: &mut impl World, actor: ActorId, action: u32) -> Result<bool> {
    let mut i = 0;
    while i < world.organization_member_count(actor)? {
        if let Some(member) = world.organization_member(actor, i)?
            && world.action_available(member, action)?
        {
            return Ok(true);
        }
        i += 1;
    }
    Ok(false)
}
/// Original578A30, scanning resolved parameters' ordered local AbilityData lists.
pub fn has_attack_class(world: &impl World, actor: ActorId, locked_only: bool) -> Result<bool> {
    // 578A38 captures +20 in EBX; 578A97 restores that value for the loop.
    let count = world.organization_group_count(actor)?;
    for i in 0..count {
        if locked_only && !world.organization_group_locked(actor, i)? {
            continue;
        }
        let count = world.organization_group_ability_count(actor, i)?;
        for ability in 0..count {
            if world.organization_group_ability_flags(actor, i, ability)? & (1 << 3) != 0 {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
/// Original579580; ownercount is queried before the eligibility/state branch.
pub fn can_recruit(world: &mut impl World, actor: ActorId, require_count: bool) -> Result<bool> {
    let owner = world.owner(actor)?;
    let eligible = if let Some(owner) = owner {
        world.owner_can_recruit(owner, actor)?
    } else {
        false
    };
    let count = if let Some(owner) = owner {
        world.owner_recruit_count(owner, actor)?
    } else {
        0
    };
    if !eligible || world.actor_fields(actor)?.state_bits & (1 << 4) != 0 {
        return Ok(false);
    }
    Ok(!require_count || count > 0)
}
/// Original57B390. All aggregation writes read current masks between callbacks.
pub fn organization(world: &mut impl World, actor: ActorId) -> Result<()> {
    let bits = world.actor_fields(actor)?.state_bits;
    if bits & ((1 << 10) | (1 << 26) | (1u64 << 36) | (1 << 25)) != 0 {
        return Ok(());
    }
    for (first, last) in [(0, 12), (29, 33), (14, 17), (35, 35), (18, 18), (20, 20)] {
        for action in first..=last {
            let enabled = any_member_action(world, actor, action)?;
            set(world, actor, action, enabled)?;
        }
    }
    let enabled = resupply_code(world, actor)? == 0;
    set(world, actor, 29, enabled)?;
    set(world, actor, 33, true)?;
    for action in [12, 11] {
        let enabled = if let Some(member) = world.organization_member_44(actor)? {
            world.action_available(member, action)?
        } else {
            false
        };
        if !enabled {
            set(world, actor, action, false)?;
        }
    }
    let action0 = world.action_available(actor, 0)?;
    let attack = has_attack_class(world, actor, true)?;
    let action1 = if attack && action0 {
        false
    } else {
        any_member_action(world, actor, 1)?
    };
    set(world, actor, 1, action1)?;
    let _ = world.owner(actor)?;
    let valid = world.valid_reference(world.organization_reference_d0(actor)?)?;
    set(world, actor, 32, valid)?;
    let recruit = can_recruit(world, actor, true)?;
    set(world, actor, 31, recruit)?;
    let fields = world.actor_fields(actor)?;
    if fields.thing_flags & (1 << 29) != 0
        || fields.state_bits & ((1u64 << 34) | (1 << 26) | (1u64 << 36) | (1 << 25)) != 0
    {
        for action in [5, 9, 2] {
            set(world, actor, action, false)?;
        }
    }
    Ok(())
}
/// Complete4D1350 ordering. Ground Element/Organization are concrete functions;
/// actual Mind and other specialized contributors are required runtime dispatch.
pub fn rebuild(world: &mut impl World, actor: ActorId) -> Result<()> {
    let previous = world.actor_fields(actor)?.action_bits;
    world.set_action_bits(actor, 0)?;
    let fields = world.actor_fields(actor)?;
    if fields.thing_flags & 1 == 0
        && fields.state_bits & ((1 << 26) | (1u64 << 36) | (1 << 25)) == 0
        && !world.has_component(actor, ComponentSlot::Wall)?
    {
        if world.has_component(actor, ComponentSlot::Element)? {
            element(world, actor)?;
        }
        if world.has_component(actor, ComponentSlot::Organization)? {
            organization(world, actor)?;
        }
        for slot in [
            ComponentSlot::Flare,
            ComponentSlot::Recruit,
            ComponentSlot::Research,
            ComponentSlot::Settlement,
            ComponentSlot::Structure,
            ComponentSlot::Denizen,
            ComponentSlot::Construction,
            ComponentSlot::MobileDeployment,
        ] {
            if world.has_component(actor, slot)? {
                world.other_contributor(actor, slot)?;
            }
        }
        if world.has_component(actor, ComponentSlot::Mind)? {
            world.mind_contributor(actor)?;
        }
    }
    if world.actor_fields(actor)?.action_bits != previous {
        world.changed(actor)?;
    }
    Ok(())
}

/// Original578140. Mind words are actual core+28/+2C; the final virtual+5C
/// predicate is evaluated only on its native branch. Numeric fields are finite
/// when read; earlier Mind busy flags short-circuit the numeric reads.
pub fn organization_status<D>(
    organization: &crate::ground_states::OrganizationState<D>,
    mind_words: Option<[u32; 2]>,
    mut mind_5c: impl FnMut() -> Result<bool>,
) -> Result<u32> {
    use aa_numeric::Binary;
    use std::cmp::Ordering;
    if mind_words.is_some_and(|words| words[0] != 0 || words[1] != 0) {
        return Ok(2);
    }
    let number = |offset| {
        let bits = *organization
            .words
            .get(&offset)
            .ok_or(Error::Missing("initialized Organization status word"))?;
        Binary::from_f32_bits(bits).map_err(|e| Error::World(e.to_string()))
    };
    if number(0x60)?.compare(&Binary::from_i32(0)) != Ordering::Greater {
        return Ok(2);
    }
    let current = number(0x5c)?;
    let threshold = Binary::from_f32_bits(organization.definition_value_308.0)
        .map_err(|e| Error::World(e.to_string()))?;
    if current.compare(&threshold) == Ordering::Greater {
        return Ok(2);
    }
    if current.is_zero() {
        return Ok(0);
    }
    if mind_words.is_none() {
        return Err(Error::Missing(
            "Mind for nonzero Organization status after threshold checks",
        ));
    }
    Ok(u32::from(!mind_5c()?))
}
