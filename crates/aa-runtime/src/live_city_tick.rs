//! CityComponent periodic Update on the live retained World.
//!
//! City+8 is its Updatable (vtable A3B6D0). Periodic 50B330 calls +14 =
//! 564C10 (arguments unused). Native order: zone 8 of the actor (+148[8] when
//! the zone count +14C > 8, else null); walk the City+18 candidate list,
//! resolving each GTID and keeping it when 5640A0 and virtual +130(zone8, 0)
//! accept it (50C510 unlinks the others); then an empty list clears City+20
//! (and runs 4D6660 when it was set) and cancels every event of this
//! Updatable (50B250); a non-empty list runs 5647A0 (capture progress).
//! The non-empty candidate path is not bound and returns an explicit error.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent},
    spatial_world::SpatialWorld,
};
use aa_actors::components::ComponentSlot;
use aa_numeric::Precision;
use aa_simulation::NativeClock;
use aa_world::queues::{Delivery, Error as QueueError, Queues};

fn fail(e: impl std::fmt::Display) -> QueueError {
    QueueError::Callback(format!("live City 564C10: {e}"))
}

pub(crate) struct Context<'a> {
    pub actors: &'a mut ActorStore,
    pub spatial: &'a SpatialWorld,
    pub variables: &'a crate::load_view_variables::State,
    pub queues: &'a mut Queues,
    pub clock: &'a NativeClock,
    pub precision: Precision,
}

pub(crate) fn run(key: ObjectKey, event: Delivery, cx: &mut Context<'_>) -> aa_world::queues::Result<()> {
    if event.elapsed_bits.is_none() {
        return Err(fail("City Updatable +18/+1C are 943800 RET4; deferred delivery is invalid"));
    }
    let id = cx.actors.id(key).ok_or_else(|| fail("City owner has no live GTID"))?;
    let (has_candidates, was_set) = match cx.actors.component(id, ComponentSlot::City).map_err(fail)? {
        Some(RuntimeComponent::City(c)) if c.header.owner == id => {
            (c.candidates.head().is_some(), c.byte_20 != 0)
        }
        _ => return Err(fail("scheduled City allocation/owner differs")),
    };
    if has_candidates {
        return Err(fail(
            "required native 5640A0/virtual +130 candidate test, 50C510 and 5647A0 capture progress are not bound",
        ));
    }
    if was_set {
        match cx.actors.component_mut(id, ComponentSlot::City).map_err(fail)? {
            Some(RuntimeComponent::City(c)) => c.byte_20 = 0,
            _ => return Err(fail("City disappeared before its +20 store")),
        }
        crate::live_updates::siege::refresh(
            cx.actors,
            cx.spatial,
            cx.variables,
            aa_body::F32(cx.clock.begin.0),
            id,
            cx.precision,
        )
        .map_err(fail)?;
    }
    //50B250 on City+8: every queued event of this same Updatable.
    cx.queues.cancel_owner(event.owner)
}
