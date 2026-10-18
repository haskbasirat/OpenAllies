//! Native actor state property/view/effect transitions4D4A90/4D16B0.
//! Required callbacks bind actual world effects; earlier mutations survive errors.
use crate::{ActorId, components::ComponentSlot, state};
use aa_body::{F32, credit::Object, properties::PropertiesState, property_effects::Instances};
use aa_numeric::{Binary, Precision};
use std::collections::BTreeSet;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("required actor state effect field unavailable: {0}")]
    Missing(&'static str),
    #[error("actor state effect lookup failed: {0}; earlier effects remain committed")]
    World(String),
    #[error("cyclic Organization membership during property applicability")]
    Cycle,
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug)]
pub struct ViewRule {
    pub health_limit: F32,
    pub required_low_states: u32,
    pub property: Option<Object>,
}
/// Original4B4590/45E620: reverse source order, extended health division.
pub fn select_view(
    rules: &[ViewRule],
    health: Option<(F32, F32)>,
    state_bits: u64,
    pc: Precision,
) -> Result<i32> {
    let Some((health, maximum)) = health else {
        return Ok(-1);
    };
    if rules.len() > i32::MAX as usize {
        return Err(Error::Missing("signed native view count"));
    }
    for (index, rule) in rules.iter().enumerate().rev() {
        let ratio = Binary::from_f32_bits(health.0)?.div(&Binary::from_f32_bits(maximum.0)?, pc)?;
        if ratio.compare(&Binary::from_f32_bits(rule.health_limit.0)?)
            != std::cmp::Ordering::Greater
            && (state_bits as u32 & rule.required_low_states) == rule.required_low_states
        {
            return Ok(index as i32);
        }
    }
    Ok(-1)
}
///581280/5032D0 with no source filter and disabled-filterfalse. Disabled
///duplicates still count as dependencies. Missing instance handles are errors.
pub fn property_present(
    state: &PropertiesState,
    instances: &Instances,
    definition: Object,
) -> Result<bool> {
    for id in &state.effects {
        let record = instances
            .get(id)
            .ok_or(Error::Missing("property instance handle"))?;
        if record.definition == definition {
            return Ok(true);
        }
    }
    Ok(false)
}
pub trait Applicability {
    fn has_properties(&self, actor: ActorId) -> Result<bool>;
    fn property_requirements(&self, property: Object) -> Result<Vec<Object>>;
    fn actor_has_property(&self, actor: ActorId, property: Object) -> Result<bool>;
    fn organization_count(&self, actor: ActorId) -> Result<Option<usize>>;
    fn organization_member(&self, actor: ActorId, index: usize) -> Result<Option<ActorId>>;
    /// Exact layout expanded group to parameter definition for a vacant slot.
    fn vacant_member_definition(&self, actor: ActorId, index: usize) -> Result<Option<Object>>;
    fn definition_component_mask(&self, definition: Object) -> Result<u32>;
    fn definition_properties(&self, definition: Object) -> Result<Vec<Object>>;
}
pub fn definition_applicable(
    world: &impl Applicability,
    definition: Object,
    property: Object,
) -> Result<bool> {
    if world.definition_component_mask(definition)? & (1 << 6) == 0 {
        return Ok(false);
    }
    for required in world.property_requirements(property)? {
        if !world.definition_properties(definition)?.contains(&required) {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Full5035E0 recursion and503390 definition fallback for empty member slots.
pub fn applicable(world: &impl Applicability, actor: ActorId, property: Object) -> Result<bool> {
    fn visit(
        world: &impl Applicability,
        actor: ActorId,
        property: Object,
        path: &mut BTreeSet<ActorId>,
    ) -> Result<bool> {
        if !path.insert(actor) {
            return Err(Error::Cycle);
        }
        let result = (|| {
            if let Some(count) = world.organization_count(actor)? {
                for index in 0..count {
                    let matched = if let Some(member) = world.organization_member(actor, index)? {
                        visit(world, member, property, path)?
                    } else if let Some(definition) = world.vacant_member_definition(actor, index)? {
                        definition_applicable(world, definition, property)?
                    } else {
                        false
                    };
                    if matched {
                        return Ok(true);
                    }
                }
            }
            if !world.has_properties(actor)? {
                return Ok(false);
            }
            for required in world.property_requirements(property)? {
                if !world.actor_has_property(actor, required)? {
                    return Ok(false);
                }
            }
            Ok(true)
        })();
        path.remove(&actor);
        result
    }
    visit(world, actor, property, &mut BTreeSet::new())
}
#[derive(Clone, Debug)]
pub struct Descriptor {
    pub index: u8,
    pub properties: Vec<Option<Object>>,
    pub entrenchable: bool,
    pub remove_from_group: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxList {
    Enter,
    Leave,
}
#[derive(Clone, Copy, Debug)]
pub struct StateFx {
    pub handle: Object,
    pub state_index: u32,
}
pub trait World: Applicability {
    fn descriptor(&self, index: u8) -> Result<Descriptor>;
    fn has_component(&self, actor: ActorId, slot: ComponentSlot) -> Result<bool>;
    fn state_bits(&self, actor: ActorId) -> Result<u64>;
    fn body_health(&self, actor: ActorId) -> Result<Option<(F32, F32)>>;
    fn view_rules(&self, actor: ActorId) -> Result<Vec<ViewRule>>;
    fn current_view(&self, actor: ActorId) -> Result<i32>;
    fn set_current_view(&mut self, actor: ActorId, index: i32) -> Result<()>;
    /// Actual Character5638B0 including property/effect transitions.
    fn character_view(&mut self, actor: ActorId, index: i32) -> Result<()>;
    /// Must allocate5030F0 andcall concrete581630, e.g.property_effects::insert.
    fn add_property(
        &mut self,
        actor: ActorId,
        property: Object,
        kind: u32,
        source: ActorId,
    ) -> Result<()>;
    /// Actual581810 source-filtered removal.
    fn remove_property(
        &mut self,
        actor: ActorId,
        property: Option<Object>,
        source: ActorId,
    ) -> Result<()>;
    /// Actual57C400; helperbelow supplies entrenchmenttime behavior.
    fn organization_transition(
        &mut self,
        actor: ActorId,
        new: &Descriptor,
        old: Option<&Descriptor>,
    ) -> Result<()>;
    /// Actual570020 required evenwhen supply_requiredfalse.
    fn refresh_economy(&mut self, actor: ActorId) -> Result<()>;
    fn state_fx(&self, actor: ActorId, list: FxList) -> Result<Vec<StateFx>>;
    fn active_fx_only(&self) -> Result<bool>;
    fn fx_active(&mut self, fx: Object) -> Result<bool>;
    ///4293C0 receives entirelist, currentposition andstateindex.
    fn emit_state_fx(
        &mut self,
        actor: ActorId,
        list: FxList,
        state_index: u8,
        active_only: bool,
    ) -> Result<()>;
    fn group_reference(&self, actor: ActorId) -> Result<ActorId>;
    fn resolve_reference(&self, id: ActorId) -> Result<Option<ActorId>>;
    fn remove_from_group(&mut self, group: ActorId, actor: ActorId) -> Result<()>;
    fn thing_flags(&self, actor: ActorId) -> Result<u32>;
    fn definition_flags(&self, actor: ActorId) -> Result<u32>;
    fn set_thing_flags(&mut self, actor: ActorId, flags: u32) -> Result<()>;
}
pub fn refresh_view(world: &mut impl World, actor: ActorId, pc: Precision) -> Result<()> {
    let rules = world.view_rules(actor)?;
    let next = select_view(
        &rules,
        world.body_health(actor)?,
        world.state_bits(actor)?,
        pc,
    )?;
    let old = world.current_view(actor)?;
    if next == old {
        return Ok(());
    }
    if world.has_component(actor, ComponentSlot::Character)? {
        return world.character_view(actor, next);
    }
    if old != -1 {
        let old = usize::try_from(old)
            .ok()
            .and_then(|i| rules.get(i))
            .ok_or(Error::Missing("valid previous view index"))?;
        if let Some(property) = old.property
            && world.has_properties(actor)?
        {
            world.remove_property(actor, Some(property), actor)?;
        }
    }
    if next != -1
        && let Some(property) = rules[next as usize].property
        && world.has_properties(actor)?
        && applicable(world, actor, property)?
    {
        world.add_property(actor, property, 9, actor)?;
    }
    world.set_current_view(actor, next)
}
/// Original4D0A00 first matching gate; emitter receives the whole list.
pub fn state_fx(
    world: &mut impl World,
    actor: ActorId,
    list: FxList,
    index: u8,
    active_only: bool,
) -> Result<bool> {
    for fx in world.state_fx(actor, list)? {
        if active_only && !world.fx_active(fx.handle)? {
            continue;
        }
        if fx.state_index == u32::from(index) {
            world.emit_state_fx(actor, list, index, active_only)?;
            return Ok(true);
        }
    }
    Ok(false)
}
pub fn enter(
    world: &mut impl World,
    actor: ActorId,
    index: u8,
    old: Option<u8>,
    pc: Precision,
) -> Result<()> {
    let new = world.descriptor(index)?;
    let old = old.map(|i| world.descriptor(i)).transpose()?;
    if world.has_component(actor, ComponentSlot::Organization)? {
        world.organization_transition(actor, &new, old.as_ref())?;
    }
    if world.has_component(actor, ComponentSlot::Economy)? {
        world.refresh_economy(actor)?;
    }
    for property in &new.properties {
        if let Some(property) = *property
            && world.has_properties(actor)?
            && applicable(world, actor, property)?
        {
            world.add_property(actor, property, 6, actor)?;
        }
    }
    refresh_view(world, actor, pc)?;
    let active = world.active_fx_only()?;
    state_fx(world, actor, FxList::Enter, index, active)?;
    if new.remove_from_group {
        let id = world.group_reference(actor)?;
        if world.resolve_reference(id)?.is_some() {
            let group = world
                .resolve_reference(id)?
                .ok_or(Error::Missing("second group lookup after first succeeded"))?;
            world.remove_from_group(group, actor)?;
        }
    }
    Ok(())
}
pub fn leave(world: &mut impl World, actor: ActorId, index: u8, pc: Precision) -> Result<()> {
    let descriptor = world.descriptor(index)?;
    for property in &descriptor.properties {
        if world.has_properties(actor)? {
            world.remove_property(actor, *property, actor)?;
        }
    }
    refresh_view(world, actor, pc)?;
    state_fx(world, actor, FxList::Leave, index, false)?;
    if index == 24 {
        let mut flags = world.thing_flags(actor)?;
        if world.definition_flags(actor)? & (1 << 12) != 0 {
            flags |= 1 << 12;
        } else {
            flags &= !(1 << 12);
        }
        world.set_thing_flags(actor, flags)?;
    }
    if world.has_component(actor, ComponentSlot::Economy)? {
        world.refresh_economy(actor)?;
    }
    Ok(())
}
/// Concrete57C400; queued Organization+70 application remains requiredonlyon
///the nonzero, Mindgroup4 branch. Entrenchmenttime is written before callback.
pub fn organization_transition<D>(
    organization: &mut crate::ground_states::OrganizationState<D>,
    new: &Descriptor,
    old: Option<&Descriptor>,
    world_time: F32,
    mut apply_pending: impl FnMut(u32) -> Result<()>,
) -> Result<()> {
    let registered = state::DESCRIPTORS
        .get(usize::from(new.index))
        .ok_or(Error::Missing("registered actor state"))?;
    if registered.group == Some(4)
        && let Some(old) = old
        && !old.entrenchable
        && new.entrenchable
    {
        organization.created_at = world_time;
    }
    let pending = *organization
        .words
        .get(&0x70)
        .ok_or(Error::Missing("Organization+70"))?;
    if pending != 0 && registered.group == Some(4) {
        apply_pending(pending)?;
    }
    Ok(())
}
