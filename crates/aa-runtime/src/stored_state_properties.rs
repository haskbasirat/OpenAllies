//! Property children of stored Actor state entry, on the canonical allocation
//! owner while its caller holds the same Mind arena. No shell or State moves.
use crate::{
    actor_world::{ActorAccess, RuntimeComponent},
    catalog::Catalog,
    definitions::Bank,
    load_property_instances as property,
    property_fields::Fields,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_body::{credit::Object, property_effects::Selection};
use aa_numeric::Precision;
use aa_rules::native::layout_data::PropertyRef;
type Result<T> = std::result::Result<T, String>;
fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn present(actors: &ActorAccess<'_>, id: ActorId, slot: Slot) -> Result<bool> {
    Ok(actors.component(id, slot).map_err(text)?.is_some())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn add(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    arena: &mut aa_mind::Arena,
    bank: &mut Bank,
    spatial: &mut crate::spatial_world::SpatialWorld,
    variables: &mut crate::load_view_variables::State,
    clock: &aa_simulation::NativeClock,
    inventory: &aa_mapworld::WorldInventory,
    owner: ActorId,
    reference: PropertyRef,
    kind: u32,
    source: Option<ActorId>,
    pc: Precision,
) -> Result<()> {
    let key = actors
        .key(owner)
        .ok_or("state5817A0 actual target generation absent")?;
    let tail;
    {
        let (state, focus) = actors.property_focus(key).map_err(text)?;
        state.bind_in_bank(catalog, bank)?;
        let definition = state.definition_for_reference(reference)?;
        let instance =
            property::construct_instance(state, definition, kind, source, || Ok(clock.begin.0))?;
        if focus.id != owner {
            return Err("state5817A0 current target identity changed".into());
        }
        let object = focus
            .entity
            .runtime
            .as_mut()
            .ok_or("state5817A0 actual runtime absent")?;
        let actor = object
            .actor
            .as_mut()
            .ok_or("state5817A0 actual Actor absent")?;
        let (properties, components) = actor.components.split_slot(Slot::Properties);
        let Some(RuntimeComponent::Properties(properties)) = properties else {
            return Err("state5817A0 actual Properties allocation absent".into());
        };
        tail = property::insert_preconstructed(
            catalog,
            Fields::stored(
                &object.definition,
                pc,
                owner,
                &mut actor.words,
                components,
                focus.identities,
                focus.others,
                bank,
                focus.abilities,
            ),
            properties,
            state,
            instance,
            pc,
            &mut crate::staged_property_instances::Numeric { variables },
        )?;
    }
    let Some(instance) = tail else { return Ok(()) };
    if present(actors, owner, Slot::Organization)? {
        let mut index = 0usize;
        loop {
            let member = match actors.component(owner, Slot::Organization).map_err(text)? {
                Some(RuntimeComponent::Organization(o)) if o.header.owner == owner => {
                    o.members.get(index).copied()
                }
                _ => return Err("5787F0 actual Organization allocation/owner changed".into()),
            };
            let Some(member) = member else { break };
            if !member.is_null() && present(actors, member, Slot::Properties)? {
                let current = actors
                    .property_allocations
                    .instances()
                    .get(&instance)
                    .ok_or("5787F0 current instance+C absent")?;
                let facts = actors.property_allocations.facts(current.definition)?;
                if facts.pass_down_members()
                    && !(present(actors, member, Slot::Experience)? && current.kind == 7)
                    && !current.disabled
                {
                    let reference = facts.reference();
                    let kind = current.kind;
                    //5787F0 ->4CEF10: member eligibility is a real child, not unconditional
                    //copying. The read-only shared admission kernel sees the SAME instances.
                    if crate::staged_property_admission::applicable_on_split_owners(
                        catalog, actors, bank, member, reference,
                    )? {
                        add(
                            catalog,
                            actors,
                            arena,
                            bank,
                            spatial,
                            variables,
                            clock,
                            inventory,
                            member,
                            reference,
                            kind,
                            Some(owner),
                            pc,
                        )?;
                    }
                }
            }
            index = index.checked_add(1).ok_or("5787F0 member index overflow")?;
        }
    }
    if present(actors, owner, Slot::Settlement)? {
        return Err(
            "584F80 state Property actual Settlement allocation and propagation required".into(),
        );
    }
    if present(actors, owner, Slot::Performer)? {
        crate::stored_performer_refresh::refresh_on_split_owners(
            catalog,
            actors,
            arena,
            bank,
            Some(spatial),
            key,
            variables,
            inventory,
            catalog.runtime_environment.unwrap_or(required_environment),
            pc,
        )?;
    }
    Ok(())
}
fn required_environment() -> Result<aa_numeric::crt_pow::CrtEnvironment> {
    Err("state Property requires current-thread CRT sampler".into())
}

///581810's actual empty-list guard precedes runtime instance/definition arena
///reads. The immutable source PropertyRef is already owned by its source list.
#[allow(clippy::too_many_arguments)]
pub(crate) fn remove_reference(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    bank: &mut Bank,
    variables: &mut crate::load_view_variables::State,
    owner: ActorId,
    reference: Option<PropertyRef>,
    source: ActorId,
    pc: Precision,
) -> Result<()> {
    match actors.component(owner, Slot::Properties).map_err(text)? {
        Some(RuntimeComponent::Properties(p)) if p.owner.0 == owner.raw() => {
            if p.effects.is_empty() {
                return Ok(());
            }
        }
        _ => return Err("state581810 actual Properties allocation/owner differs".into()),
    }
    let definition = if let Some(reference) = reference {
        actors.property_allocations.bind_in_bank(catalog, bank)?;
        actors
            .property_allocations
            .definition_for_reference(reference)?
    } else {
        Object(0)
    };
    remove(
        catalog, actors, bank, variables, owner, definition, source, pc,
    )
}

pub(crate) fn remove(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    bank: &mut Bank,
    variables: &mut crate::load_view_variables::State,
    owner: ActorId,
    definition: Object,
    source: ActorId,
    pc: Precision,
) -> Result<()> {
    let key = actors
        .key(owner)
        .ok_or("state581810 actual target generation absent")?;
    let (state, focus) = actors.property_focus(key).map_err(text)?;
    state.require_in_bank(catalog, bank)?;
    let object = focus
        .entity
        .runtime
        .as_mut()
        .ok_or("state581810 actual runtime absent")?;
    let actor = object
        .actor
        .as_mut()
        .ok_or("state581810 actual Actor absent")?;
    let (properties, components) = actor.components.split_slot(Slot::Properties);
    let Some(RuntimeComponent::Properties(properties)) = properties else {
        return Err("state581810 actual Properties allocation absent".into());
    };
    property::remove_preconstructed(
        catalog,
        Fields::stored(
            &object.definition,
            pc,
            owner,
            &mut actor.words,
            components,
            focus.identities,
            focus.others,
            bank,
            focus.abilities,
        ),
        properties,
        state,
        Selection {
            definition,
            source: Some(Object(source.raw())),
        },
        pc,
        &mut crate::staged_property_instances::Numeric { variables },
    )?;
    Ok(())
}
