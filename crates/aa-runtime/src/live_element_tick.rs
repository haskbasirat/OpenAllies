//! ElementComponent periodic Update on the live retained World.
//!
//! Element+8 is its Updatable (vtable A3C6C8; queue1 via 50AC60 at 5704D0).
//! Periodic 50B330 calls +14 = 570B90(elapsed, data); RET 8. Native order:
//! 1. first call binds the SVar MoverDetachedSecondsUntilTeleport (4B2E60);
//! 2. 5709D0: the formation byte Element+1C = (Element+10 organization's
//!    Organization(+7C)+40 is this actor) or (not Actor+100 bit 24 and the
//!    organization's zone 4 (+148[4] when +14C > 4) contains this actor by
//!    virtual +130); a change runs 4D4130, 578ED0/578EF0, 4EA200/54EF10;
//! 3. with Actor+70 Mind whose virtual +8C (535EA0: current state is Sleep at
//!    Mind+68) or +A4 (52D780: Mind+184 == 2) holds, Actor+100 bit 24 clear,
//!    Element+10 non-null and Element+1C clear: an ordered negative +24 timer
//!    starts at World+E0 + the SVar (x87, stored binary32); a due timer
//!    teleports the detached element (4CF290, 556070, 404F60, 4EEB40/4EEBE0,
//!    5709D0). Every other case stores +24 = -1.0;
//! 4. without Actor+88 Denizen: 4D6660; 58EA80 (Element present, Hq+D8 absent,
//!    Experience+A4 present) adds the elapsed seconds to the owner Kingdom's
//!    +340 relation statistic 0x3A for Actor+4 (58D8C0).
//! The unbound formation-change, zone-4 containment and teleport leaves return
//! explicit errors at their call.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent},
    definitions::Bank,
    restoration::KingdomEconomyLoad,
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_mind::construct::Kind;
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeClock;
use aa_world::queues::{Delivery, Error as QueueError};
use std::collections::BTreeMap;

fn fail(e: impl std::fmt::Display) -> QueueError {
    QueueError::Callback(format!("live Element 570B90: {e}"))
}
fn required(leaf: &str) -> QueueError {
    fail(format!("required native {leaf} is not bound; prior effects remain committed"))
}
const MINUS_ONE: u32 = 0xbf80_0000;

pub(crate) struct Context<'a> {
    pub actors: &'a mut ActorStore,
    pub bank: &'a Bank,
    pub economy: &'a mut BTreeMap<u32, KingdomEconomyLoad>,
    pub spatial: &'a SpatialWorld,
    pub variables: &'a crate::load_view_variables::State,
    pub clock: &'a NativeClock,
    pub precision: Precision,
}

fn element(actors: &ActorStore, id: ActorId) -> Result<&aa_actors::ground_states::ElementState<String>, QueueError> {
    match actors.component(id, Slot::Element).map_err(fail)? {
        Some(RuntimeComponent::Element(e)) if e.header.owner == id => Ok(e),
        _ => Err(fail("scheduled Element allocation/owner differs")),
    }
}
fn store_time(actors: &mut ActorStore, id: ActorId, bits: u32) -> Result<(), QueueError> {
    match actors.component_mut(id, Slot::Element).map_err(fail)? {
        Some(RuntimeComponent::Element(e)) => {
            e.time_24 = aa_body::F32(bits);
            Ok(())
        }
        _ => Err(fail("Element disappeared before its +24 store")),
    }
}
fn svar_fixed(variables: &crate::load_view_variables::State, name: &str) -> Result<u32, QueueError> {
    let ordinal = variables
        .registrations()
        .find(|r| r.name() == name)
        .map(|r| r.ordinal())
        .ok_or_else(|| fail(format!("SVar {name} is not registered")))?;
    match variables.current(ordinal as usize).map_err(fail)? {
        crate::load_view_variables::CurrentValue::Fixed(v) => Ok(*v),
        _ => Err(fail(format!("SVar {name} is not the float kind"))),
    }
}

/// 5709D0 without a byte change.
fn formation(actors: &ActorStore, id: ActorId, key: ObjectKey) -> Result<(), QueueError> {
    let e = element(actors, id)?;
    let mut flag = false;
    if let Some(org) = e.organization {
        let leader = match actors.component(org, Slot::Organization).map_err(fail)? {
            Some(RuntimeComponent::Organization(o)) => o
                .words
                .get(&0x40)
                .copied()
                .ok_or_else(|| fail("Organization+40 is uninitialized"))?,
            Some(_) => return Err(fail("Organization slot payload mismatch")),
            None => return Err(fail("5709DD null Element+10 Organization(+7C) dereference")),
        };
        if leader == id.raw() {
            flag = true;
        } else {
            let state = actors.object(key).map_err(fail)?.components().map_err(fail)?.state.low;
            if state & 0x0100_0000 == 0 {
                let org_key = actors.key(org).ok_or_else(|| fail("Element+10 has no live GTID"))?;
                let zones = &actors
                    .object(org_key)
                    .map_err(fail)?
                    .actor
                    .as_ref()
                    .ok_or_else(|| fail("Organization Actor shell absent"))?
                    .zones_148
                    .zones()
                    .slots;
                if zones.len() > 4 && zones[4].is_some() {
                    return Err(required("5709D0 virtual +130 zone-4 containment"));
                }
            }
        }
    }
    if e.flag_1c != flag {
        return Err(required("5709D0 formation byte change (4D4130, 578ED0/578EF0, 4EA200/54EF10)"));
    }
    Ok(())
}

pub(crate) fn run(key: ObjectKey, event: Delivery, cx: &mut Context<'_>) -> aa_world::queues::Result<()> {
    let elapsed = event
        .elapsed_bits
        .ok_or_else(|| fail("Element +18/+1C are 943800 RET4; deferred delivery is invalid"))?;
    let id = cx.actors.id(key).ok_or_else(|| fail("owner has no live GTID"))?;
    //Step 1 is a process-lifetime SVar pointer; reading the registered owner
    //at its use below is the same storage.
    formation(cx.actors, id, key)?;
    //Step 3.
    let components = cx.actors.object(key).map_err(fail)?.components().map_err(fail)?;
    let state_low = components.state.low;
    let mind_ok = match components.get(Slot::Mind) {
        None => false,
        Some(RuntimeComponent::Mind(m)) => {
            let sleeping = match m.kind {
                Kind::Element => {
                    let current = cx.actors.minds.mind(m.mind_id).map_err(fail)?.stack.front().copied().flatten();
                    current.is_some() && current == m.state_bindings.get(&0x68).copied()
                }
                _ => return Err(required("virtual +8C/+A4 for a non-Element Mind")),
            };
            sleeping
                || m
                    .words
                    .get(&0x184)
                    .copied()
                    .ok_or_else(|| fail("Mind+184 is uninitialized"))?
                    == 2
        }
        Some(_) => return Err(fail("Mind slot payload mismatch")),
    };
    let e = element(cx.actors, id)?;
    if mind_ok && state_low & 0x0100_0000 == 0 && e.organization.is_some() && !e.flag_1c {
        let t = e.time_24.0;
        let now = cx.clock.begin.0;
        if f32::from_bits(t).is_nan() {
            //570C4C: unordered skips the start; 570C83 unordered skips the teleport.
        } else if Binary::from_f32_bits(t).map_err(fail)?.compare(&Binary::from_i32(0)) == std::cmp::Ordering::Less {
            let svar = svar_fixed(cx.variables, "MoverDetachedSecondsUntilTeleport")?;
            let next = Binary::from_f32_bits(now)
                .and_then(|n| n.add(&Binary::from_f32_bits(svar)?, cx.precision))
                .and_then(|v| v.store_f32())
                .map_err(fail)?;
            store_time(cx.actors, id, next)?;
        } else if Binary::from_f32_bits(now).map_err(fail)?.compare(&Binary::from_f32_bits(t).map_err(fail)?)
            == std::cmp::Ordering::Less
        {
        } else {
            return Err(required("570C91 detached-element teleport (4CF290, 556070, 4EEB40/4EEBE0)"));
        }
    } else {
        store_time(cx.actors, id, MINUS_ONE)?;
    }
    //Step 4.
    let components = cx.actors.object(key).map_err(fail)?.components().map_err(fail)?;
    if !components.present(Slot::Denizen) {
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
    let components = cx.actors.object(key).map_err(fail)?.components().map_err(fail)?;
    if components.present(Slot::Element) && !components.present(Slot::Hq) && components.present(Slot::Experience) {
        let owner = cx
            .actors
            .object(key)
            .map_err(fail)?
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&0xe8))
            .copied()
            .ok_or_else(|| fail("Actor+E8 is uninitialized"))?;
        if owner != 0 {
            let relations = cx
                .economy
                .get_mut(&owner)
                .ok_or_else(|| fail("owner Kingdom statistics owner absent"))?
                .statistics
                .relations
                .as_mut();
            if let Some(relations) = relations {
                let definition = &cx.actors.object(key).map_err(fail)?.definition.proof.id;
                let token = cx
                    .bank
                    .token("ActorDefinition", definition)
                    .ok_or_else(|| fail("ActorDefinition token absent for 58D8C0 key"))?;
                relations
                    .add_by_definition(
                        0x3a,
                        Some(token),
                        aa_economy::F32Bits(elapsed),
                        match cx.precision {
                            Precision::Single24 => aa_economy::X87Precision::Single24,
                            Precision::Double53 => aa_economy::X87Precision::Double53,
                            Precision::Extended64 => aa_economy::X87Precision::Extended64,
                        },
                    )
                    .map_err(fail)?;
            }
        }
    }
    Ok(())
}
