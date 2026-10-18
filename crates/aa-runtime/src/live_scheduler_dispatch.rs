//! Native queue deliveries over actual scheduler identities and actor storage.
//!
//! Periodic50B330 invokes Updatable+14, queue3 uses4F05C0 -> +18 and queue4
//! uses4F05B0 -> +1C. This is a bounded concrete service kernel; it does not
//! fabricate the other services required by clocked::Services or battle readiness.
use crate::{
    actor_world::{ActiveComponent, ActorStore, ObjectKey, RuntimeComponent, Spatial},
    scheduler_owners::{Owners, Target},
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_simulation::NativeRng;
use aa_world::queues::{Delivery, Error, Queues, Result};

pub struct Context<'a, S: Spatial> {
    pub actors: &'a mut ActorStore,
    pub owners: &'a Owners,
    pub queues: &'a mut Queues,
    /// The authoritative stream is retained even on paths that draw nothing.
    pub rng: &'a mut NativeRng,
    pub spatial: &'a S,
}

fn fail(message: impl std::fmt::Display) -> Error {
    Error::Callback(format!("live scheduled Update: {message}"))
}

fn required(leaf: &str) -> Error {
    fail(format!(
        "required native {leaf} is not bound; prior effects remain committed"
    ))
}

///One delivery-kind check shared with the actual frame's Mind interception.
pub(crate) fn validate_delivery(event:Delivery)->Result<()> {
    let periodic = event.elapsed_bits.is_some();
    if event.queue > 4 || periodic != (event.queue < 3) {
        return Err(fail(
            "delivery queue/kind is outside the native five-queue binding",
        ));
    }
    Ok(())
}
/// Delivery must come from the actual Queues update. The queue's deferred entry
/// has already been removed before this call, exactly as50B020 requires. Errors
/// are nontransactional; retrying an errored world without recovery is invalid.
pub fn dispatch<S: Spatial>(event: Delivery, context: &mut Context<'_, S>) -> Result<()> {
    validate_delivery(event)?;
    let periodic = event.elapsed_bits.is_some();
    let target = context.owners.target(event.owner).map_err(fail)?;
    let Target::Component { object, slot } = target else {
        // Sun is the retained World+4C allocation, not an Actor/GTID alias.
        return Err(required(&format!("Update for {target:?}")));
    };
    let slot = ComponentSlot::from_index(slot as usize)
        .ok_or_else(|| fail("scheduler component slot is invalid"))?;
    let id = context
        .actors
        .id(object)
        .ok_or_else(|| fail("scheduler Actor has no live GTID"))?;
    let component = context
        .actors
        .object(object)
        .map_err(fail)?
        .components()
        .map_err(fail)?
        .get(slot)
        .ok_or_else(|| fail("scheduled component allocation is absent"))?;
    if component.slot() != slot {
        return Err(fail(
            "actual component class does not match its scheduled slot",
        ));
    }
    if periodic {
        // Their actual +14 vtable entry is4D1E10 (RET8), with no state/RNG reads.
        let owner = match component {
            RuntimeComponent::Recruit(v) => Some(v.header.owner),
            RuntimeComponent::Builder(v) => Some(v.header.owner),
            RuntimeComponent::MobileDeployment(v) => Some(v.header.owner),
            RuntimeComponent::Research(v) => Some(v.header.owner),
            RuntimeComponent::Hq(v) => Some(v.header.owner),
            _ => None,
        };
        if let Some(owner) = owner {
            if owner != id {
                return Err(fail(
                    "actual component owner differs from scheduler allocation",
                ));
            }
            return Ok(());
        }
        return Err(required(&format!("{slot:?} periodic virtual+14")));
    }
    if event.queue == 4 {
        return Err(required(&format!("{slot:?} deferred virtual+1C")));
    }
    if slot != ComponentSlot::Performer {
        return Err(required(&format!("{slot:?} deferred virtual+18")));
    }
    let RuntimeComponent::Performer(performer) = component else {
        return Err(fail("Performer slot does not contain its native class"));
    };
    if performer.state.owner != id.raw() {
        return Err(fail(
            "actual Performer owner differs from scheduler allocation",
        ));
    }
    performer_timed_prefix(event, object, id, context)
}

fn performer_timed_prefix<S: Spatial>(
    event: Delivery,
    object: ObjectKey,
    id: ActorId,
    context: &mut Context<'_, S>,
) -> Result<()> {
    match context
        .actors
        .object(object)
        .map_err(fail)?
        .active_component
    {
        ActiveComponent::None => return Ok(()),
        ActiveComponent::Slot(slot) if slot != ComponentSlot::Performer => return Ok(()),
        ActiveComponent::Slot(_) => {}
        ActiveComponent::UnresolvedNativeWord(_) => {
            return Err(fail("active component reference is unresolved"));
        }
    }
    // Native4D03D0 is reached only after actual pointer identity. Thing+8 bit0
    // suppresses the Properties read; +1C is ModifierState.flags, not its later
    // modifier_flags field. Neither absence nor a foreign owner is fabricated.
    let flags = context
        .actors
        .placement(object, context.spatial)
        .map_err(fail)?
        .flags;
    let paused = if flags & 1 != 0 {
        false
    } else {
        match context
            .actors
            .component(id, ComponentSlot::Properties)
            .map_err(fail)?
        {
            None => false,
            Some(RuntimeComponent::Properties(v)) => {
                if v.owner != aa_body::credit::Object(id.raw()) {
                    return Err(fail("Properties owner differs from actual Actor"));
                }
                v.modifier.flags & 2 != 0
            }
            Some(_) => return Err(fail("Properties slot contains another component class")),
        }
    };
    if paused {
        // Performer Updatable+C is native6A71D0=true.50ACC0 uses World+1E8
        // (queue3), preserves unsigned payload bits and draws no RNG value.
        context
            .queues
            .add_after(3, event.owner, event.data, 0x38d1_b717)?;
        return Ok(());
    }
    let data = event.data as i32;
    if data >= 6 {
        return match context
            .actors
            .component(id, ComponentSlot::Character)
            .map_err(fail)?
        {
            None => Ok(()),
            Some(RuntimeComponent::Character(_)) => Err(required("Animator563AA0 cue")),
            Some(_) => Err(fail("Character slot contains another component class")),
        };
    }
    let state = match context
        .actors
        .component(id, ComponentSlot::Performer)
        .map_err(fail)?
    {
        Some(RuntimeComponent::Performer(v)) => &v.state,
        _ => {
            return Err(fail(
                "actual Performer disappeared without a native callback",
            ));
        }
    };
    if state.stage != data || !(0..=3).contains(&data) {
        return Ok(());
    }
    if data == 0 {
        match context
            .actors
            .component_mut(id, ComponentSlot::Performer)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Performer(v)) => v.state.turret_index = -1,
            _ => return Err(fail("actual Performer disappeared before turret store")),
        }
        // Native50B1C0's first argument is queue ID, second is payload.
        //57F0BD..CA removes queue0/data0, even though this delivery is queue3.
        context.queues.remove_matching(event.owner, 0, 0)?;
        return Err(required("Performer57ED20 begin"));
    }
    Err(required(match data {
        1 => "Performer57E8C0 use frame",
        2 => "Performer4C8300/4C8850 end ability context",
        _ => "Performer57DC90 actual Mind+138 notification",
    }))
}
