//! Native4D6F10 InitCreate orchestration and fallible4D21A0 initial assembly.
//! The World owns the single live actor/component registry. Required callbacks
//! must perform the real operation or return an explicit unsupported error.
use crate::{
    ActorId,
    components::{ALL_SLOTS, ComponentSlot, StateBits},
};
use aa_body::F32;
use aa_numeric::{Binary, Precision};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("actor creation world service failed: {0}")]
    World(String),
    #[error("actor creation requires unsupported service: {0}")]
    Unsupported(&'static str),
    #[error("component {slot:?} requires missing {required:?}")]
    MissingComponent {
        slot: ComponentSlot,
        required: ComponentSlot,
    },
    #[error("Construction allocation returned no component")]
    ConstructionAllocation,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactoryKind {
    Standard(ComponentSlot),
    OrganizationMind,
    AircraftMind,
    ElementMind,
}
/// Original4CF480 Mind subclass precedence; all other31-slot entries map to
/// their own constructor. Mind's null branch is native, not a placeholder.
pub fn factory_kind(slot: ComponentSlot, definition_mask: u32) -> Option<FactoryKind> {
    if slot != ComponentSlot::Mind {
        return Some(FactoryKind::Standard(slot));
    }
    if definition_mask & (1 << 7) != 0 {
        Some(FactoryKind::OrganizationMind)
    } else if definition_mask & (1 << 23) != 0 {
        Some(FactoryKind::AircraftMind)
    } else if definition_mask & (1 << 8) != 0 {
        Some(FactoryKind::ElementMind)
    } else {
        None
    }
}
#[derive(Clone, Debug)]
pub struct LinkedDefinition<D> {
    pub definition: D,
    pub anchor: [F32; 3],
}
#[derive(Clone, Debug)]
pub struct Definition<D> {
    pub identity: D,
    pub component_mask: u32,
    /// Native +1AC registration definition; absent takes the auto-number branch.
    pub number_registration: Option<D>,
    pub automatic_number: bool,
    pub anchor: [F32; 3],
    pub linked: Option<LinkedDefinition<D>>,
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Native creation-record+38; value4 suppresses number registration.
    pub reason: u32,
    /// Native creation-record+51 and+52.
    pub start_character: bool,
    pub nonblocking: bool,
}
#[derive(Clone, Debug)]
pub struct LinkedRequest<A, D> {
    pub definition: D,
    pub position: [F32; 3],
    pub facing_xy: [F32; 2],
    pub owner: Option<A>,
    pub parent_id: ActorId,
    pub nonblocking: bool,
}

pub trait World {
    /// Portable object identity may precede its generation-bearing GTID during
    /// initialize_object. Do not fabricate a temporary GTID for the constructor.
    type Actor: Copy;
    type Definition: Clone;
    type Record;
    /// Definition storage is immutable during initialization, as in the typed
    /// registry. The original caches this definition pointer before base init.
    fn definition(&self, actor: Self::Actor) -> Result<Definition<Self::Definition>>;
    fn initialize_object(
        &mut self,
        actor: Self::Actor,
        record: &Self::Record,
        precision: Precision,
    ) -> Result<bool>;
    fn state_group(&self, actor: Self::Actor, group: u32) -> Result<Option<u32>>;
    fn set_state(&mut self, actor: Self::Actor, state: u32, enabled: bool) -> Result<()>;
    fn state_bits(&self, actor: Self::Actor) -> Result<StateBits>;
    fn has_component(&self, actor: Self::Actor, slot: ComponentSlot) -> Result<bool>;
    fn construct_component(
        &mut self,
        actor: Self::Actor,
        slot: ComponentSlot,
        kind: FactoryKind,
        precision: Precision,
    ) -> Result<()>;
    fn component_requirement(
        &mut self,
        actor: Self::Actor,
        slot: ComponentSlot,
    ) -> Result<Option<ComponentSlot>>;
    fn city_allowed(&mut self, actor: Self::Actor) -> Result<bool>;
    fn construction_ready(&mut self, actor: Self::Actor) -> Result<()>;
    fn register_number(&mut self, actor: Self::Actor, definition: &Self::Definition) -> Result<()>;
    /// Direct actor+E8 read for automatic numbering (no virtual callback).
    fn stored_owner(&self, actor: Self::Actor) -> Result<Option<Self::Actor>>;
    /// Actual virtual+108 owner dispatch; callbacks can mutate live actor fields.
    fn owner(&mut self, actor: Self::Actor) -> Result<Option<Self::Actor>>;
    /// Native4D30BA owner/UI notification after a nonempty new-component list.
    /// Runtime must evaluate the original UI-owner gates, not assume absence.
    fn assembly_completed(&mut self, actor: Self::Actor, created: &[ComponentSlot]) -> Result<()>;
    fn assign_owner_number(&mut self, actor: Self::Actor, owner: Self::Actor) -> Result<()>;
    /// True only when resolved Character view+4C is nonzero and its first+48
    /// animation reference is nonnull. No preview mesh heuristic is permitted.
    fn character_has_start(&self, actor: Self::Actor) -> Result<bool>;
    fn start_character(&mut self, actor: Self::Actor) -> Result<()>;
    fn default_stance(&mut self, actor: Self::Actor) -> Result<()>;
    fn position(&self, actor: Self::Actor) -> Result<[F32; 3]>;
    fn facing_xy(&self, actor: Self::Actor) -> Result<[F32; 2]>;
    fn actor_id(&self, actor: Self::Actor) -> Result<ActorId>;
    fn create_linked(
        &mut self,
        request: LinkedRequest<Self::Actor, Self::Definition>,
        precision: Precision,
    ) -> Result<Option<Self::Actor>>;
    fn set_linked_id(&mut self, actor: Self::Actor, child: ActorId) -> Result<()>;
    fn resolve_id(&self, id: ActorId) -> Result<Option<Self::Actor>>;
    fn set_parent_id(&mut self, child: Self::Actor, parent: ActorId) -> Result<()>;
    /// Native562200 on the child, with the parent's current Body health/maximum.
    fn copy_linked_health(
        &mut self,
        parent: Self::Actor,
        child: Self::Actor,
        precision: Precision,
    ) -> Result<()>;
    fn initialize_component(
        &mut self,
        actor: Self::Actor,
        slot: ComponentSlot,
        record: &Self::Record,
        precision: Precision,
    ) -> Result<bool>;
    fn register_construction(
        &mut self,
        actor: Self::Actor,
        owner: Option<Self::Actor>,
        definition: &Self::Definition,
    ) -> Result<()>;
}

/// Narrow concrete factory composition surface. Native assembly is shared by
/// full InitCreate and the staged runtime; callers need not invent unrelated
/// linked/Character/owner services merely to construct its component table.
pub trait AssemblyWorld {
    type Actor: Copy;
    fn state_bits(&self, actor: Self::Actor) -> Result<StateBits>;
    fn has_component(&self, actor: Self::Actor, slot: ComponentSlot) -> Result<bool>;
    fn construct_component(
        &mut self,
        actor: Self::Actor,
        slot: ComponentSlot,
        kind: FactoryKind,
        precision: Precision,
    ) -> Result<()>;
    fn component_requirement(
        &mut self,
        actor: Self::Actor,
        slot: ComponentSlot,
    ) -> Result<Option<ComponentSlot>>;
    fn city_allowed(&mut self, actor: Self::Actor) -> Result<bool>;
    fn construction_ready(&mut self, actor: Self::Actor) -> Result<()>;
    fn assembly_completed(&mut self, actor: Self::Actor, created: &[ComponentSlot]) -> Result<()>;
}
struct AssemblyAdapter<'a, W>(&'a mut W);
impl<W: World> AssemblyWorld for AssemblyAdapter<'_, W> {
    type Actor = W::Actor;
    fn state_bits(&self, a: Self::Actor) -> Result<StateBits> {
        self.0.state_bits(a)
    }
    fn has_component(&self, a: Self::Actor, s: ComponentSlot) -> Result<bool> {
        self.0.has_component(a, s)
    }
    fn construct_component(
        &mut self,
        a: Self::Actor,
        s: ComponentSlot,
        k: FactoryKind,
        p: Precision,
    ) -> Result<()> {
        self.0.construct_component(a, s, k, p)
    }
    fn component_requirement(
        &mut self,
        a: Self::Actor,
        s: ComponentSlot,
    ) -> Result<Option<ComponentSlot>> {
        self.0.component_requirement(a, s)
    }
    fn city_allowed(&mut self, a: Self::Actor) -> Result<bool> {
        self.0.city_allowed(a)
    }
    fn construction_ready(&mut self, a: Self::Actor) -> Result<()> {
        self.0.construction_ready(a)
    }
    fn assembly_completed(&mut self, a: Self::Actor, s: &[ComponentSlot]) -> Result<()> {
        self.0.assembly_completed(a, s)
    }
}
pub fn assemble<W: World>(
    actor: W::Actor,
    definition_mask: u32,
    world: &mut W,
    precision: Precision,
) -> Result<()> {
    assemble_with(
        actor,
        definition_mask,
        &mut AssemblyAdapter(world),
        precision,
    )
}
pub fn assemble_with<W: AssemblyWorld>(
    actor: W::Actor,
    definition_mask: u32,
    world: &mut W,
    precision: Precision,
) -> Result<()> {
    let mut created = Vec::new();
    for index in [
        15, 0, 2, 1, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 4,
    ] {
        let slot = ALL_SLOTS[index];
        if world.has_component(actor, slot)? {
            continue;
        }
        let eligible = if slot == ComponentSlot::Mind {
            !world.state_bits(actor)?.construction()
        } else if definition_mask & (1 << index) == 0 {
            false
        } else {
            match slot {
                ComponentSlot::Construction
                | ComponentSlot::Body
                | ComponentSlot::Properties
                | ComponentSlot::Wall
                | ComponentSlot::Structure
                | ComponentSlot::Detection => true,
                ComponentSlot::Element
                | ComponentSlot::Economy
                | ComponentSlot::MobileDeployment => {
                    let state = world.state_bits(actor)?;
                    !state.construction() || state.selection_special()
                }
                ComponentSlot::City => world.city_allowed(actor)?,
                _ => {
                    let state = world.state_bits(actor)?;
                    !state.construction() && !state.selection_special()
                }
            }
        };
        if eligible {
            if let Some(kind) = factory_kind(slot, definition_mask) {
                world.construct_component(actor, slot, kind, precision)?;
                if world.has_component(actor, slot)? {
                    created.push(slot);
                }
            }
            if slot == ComponentSlot::Construction && !world.has_component(actor, slot)? {
                return Err(Error::ConstructionAllocation);
            }
        }
    }
    for slot in ALL_SLOTS {
        if world.has_component(actor, slot)?
            && let Some(required) = world.component_requirement(actor, slot)?
            && !world.has_component(actor, required)?
        {
            return Err(Error::MissingComponent { slot, required });
        }
    }
    if world.has_component(actor, ComponentSlot::Construction)? {
        world.construction_ready(actor)?;
    }
    if !created.is_empty() {
        world.assembly_completed(actor, &created)?;
    }
    Ok(())
}

/// Original linked-child XY anchor difference/rotation through its first f32 stores.
pub fn linked_offset(
    parent_anchor: [F32; 3],
    child_anchor: [F32; 3],
    facing: [F32; 2],
    precision: Precision,
) -> Result<[F32; 2]> {
    let n = |v: F32| Binary::from_f32_bits(v.0);
    let dx = n(parent_anchor[0])?.sub(&n(child_anchor[0])?, precision)?;
    let dy = n(parent_anchor[1])?.sub(&n(child_anchor[1])?, precision)?;
    Ok([
        F32(dy
            .mul(&n(facing[0])?, precision)?
            .add(&dx.mul(&n(facing[1])?, precision)?, precision)?
            .store_f32()?),
        F32(dy
            .mul(&n(facing[1])?, precision)?
            .sub(&dx.mul(&n(facing[0])?, precision)?, precision)?
            .store_f32()?),
    ])
}
/// Native ignores anchor Z and copies the parent actor's actual Z unchanged.
pub fn linked_position(
    parent_anchor: [F32; 3],
    child_anchor: [F32; 3],
    position: [F32; 3],
    facing: [F32; 2],
    precision: Precision,
) -> Result<[F32; 3]> {
    offset_position(
        linked_offset(parent_anchor, child_anchor, facing, precision)?,
        position,
        precision,
    )
}
fn offset_position(offset: [F32; 2], position: [F32; 3], precision: Precision) -> Result<[F32; 3]> {
    Ok([
        F32(Binary::from_f32_bits(offset[0].0)?
            .add(&Binary::from_f32_bits(position[0].0)?, precision)?
            .store_f32()?),
        F32(Binary::from_f32_bits(offset[1].0)?
            .add(&Binary::from_f32_bits(position[1].0)?, precision)?
            .store_f32()?),
        position[2],
    ])
}

/// Full4D6F10 call sequence. False is a native initialization refusal; unsupported
/// services are errors. Prior registration/RNG/component effects are retained.
pub fn initialize<W: World>(
    actor: W::Actor,
    record: &W::Record,
    options: Options,
    world: &mut W,
    precision: Precision,
) -> Result<bool> {
    let definition = world.definition(actor)?;
    if !world.initialize_object(actor, record, precision)? {
        return Ok(false);
    }
    if world.state_group(actor, 13)?.is_none() {
        world.set_state(actor, 27, true)?;
    }
    assemble(actor, definition.component_mask, world, precision)?;
    if options.reason != 4 {
        if let Some(number) = definition.number_registration.as_ref() {
            world.register_number(actor, number)?;
        } else if let Some(owner) = world.stored_owner(actor)?
            && definition.automatic_number
        {
            world.assign_owner_number(actor, owner)?;
        }
    }
    if options.start_character
        && world.has_component(actor, ComponentSlot::Character)?
        && world.character_has_start(actor)?
    {
        world.start_character(actor)?;
    } else {
        world.default_stance(actor)?;
    }
    if let Some(linked) = definition.linked {
        let offset = linked_offset(
            definition.anchor,
            linked.anchor,
            world.facing_xy(actor)?,
            precision,
        )?;
        let owner = world.owner(actor)?;
        let position = offset_position(offset, world.position(actor)?, precision)?;
        let request = LinkedRequest {
            definition: linked.definition,
            position,
            facing_xy: world.facing_xy(actor)?,
            owner,
            parent_id: world.actor_id(actor)?,
            nonblocking: options.nonblocking,
        };
        let child = world.create_linked(request, precision)?;
        let id = match child {
            Some(child) => world.actor_id(child)?,
            None => ActorId::NULL,
        };
        world.set_linked_id(actor, id)?;
        if let Some(child) = world.resolve_id(id)? {
            let parent_id = world.actor_id(actor)?;
            world.set_parent_id(child, parent_id)?;
            if world.has_component(actor, ComponentSlot::Body)?
                && world.has_component(child, ComponentSlot::Body)?
            {
                world.copy_linked_health(actor, child, precision)?;
            }
        }
    }
    for slot in ALL_SLOTS {
        if world.has_component(actor, slot)?
            && !world.initialize_component(actor, slot, record, precision)?
        {
            return Ok(false);
        }
    }
    if world.state_bits(actor)?.construction() {
        let owner = world.owner(actor)?;
        world.register_construction(actor, owner, &definition.identity)?;
    }
    Ok(true)
}
