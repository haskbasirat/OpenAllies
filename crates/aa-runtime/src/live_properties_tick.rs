//! PropertiesComponent periodic Update on the live retained World.
//!
//! Properties+8 is its Updatable (vtable A3E3B4; queue1 via 50AC60). Periodic
//! 50B330 calls +14 = 5818D0 (elapsed and data unused). Native order:
//! 1. for every effect handle Properties+10[i] (count re-read each pass) call
//!    instance virtual +0: plain 5030F0 instances (vtable A29678) use 6A71F0
//!    = false; enchantment 503140 instances (A2EBE0) use 5031B0, comparing
//!    World+E0 - created with EnchantmentData+60. An expired effect is removed
//!    through 5031E0/5034F0/581690/4CEB00/+C/502C60/6F7430;
//! 2. without Actor+60 Body nothing else runs;
//! 3. 4CED40(actor) (Settlement+98 -> actor, else virtual +100 = 4CED60) and,
//!    when present, 4CEDA0 gate healing on +100 bits 0x100000/0x40000000;
//!    the actor's own +100 bits gate it again;
//! 4. heal 562BA0 for each ModifierState+A4 entry (501710), then damage
//!    5625C0 for each +98 record (501690/5016B0/5016D0/5016F0).
//! Steps reached with non-empty native data that is not yet bound (expiring
//! enchantments, Structure+10/Wall parents, heals, periodic damage) return
//! explicit errors at that call; earlier effects (none on the bound path)
//! remain committed. The aa_combat::periodic kernel models the same order.
use crate::actor_world::{ActorStore, ObjectKey, RuntimeComponent};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_world::queues::{Delivery, Error as QueueError};

fn fail(error: impl std::fmt::Display) -> QueueError {
    QueueError::Callback(format!("live Properties 5818D0: {error}"))
}
fn required(leaf: impl std::fmt::Display) -> QueueError {
    fail(format!("required native {leaf} is not bound; prior effects remain committed"))
}

/// Actor+100 low DWORD (state bits 0..31).
fn state_low(actors: &ActorStore, key: ObjectKey) -> Result<u32, QueueError> {
    Ok(actors
        .object(key)
        .map_err(fail)?
        .components()
        .map_err(fail)?
        .state
        .low)
}

/// 4CED40 on `key`: Settlement (+98) returns the actor itself; otherwise its
/// virtual +100 (4CED60 for GActor/Kingdom/Group): a non-null Structure+10, or
/// the Wall+90 58CB30 chain. Returns the outer object, or None for null.
fn outer_4ced40(actors: &ActorStore, key: ObjectKey) -> Result<Option<ObjectKey>, QueueError> {
    let object = actors.object(key).map_err(fail)?;
    let components = object.components().map_err(fail)?;
    if components.present(ComponentSlot::Settlement) {
        return Err(required("4CED40 Settlement+98 owner (Settlement is not a runtime component)"));
    }
    if object.definition.native_class != 3 {
        return Err(required(format!(
            "virtual +100 for native class {}",
            object.definition.native_class
        )));
    }
    match components.get(ComponentSlot::Structure) {
        Some(RuntimeComponent::Structure(v)) if v.object_handle_10 != 0 => {
            return Err(required("4CED60 Structure+10 parent object binding"));
        }
        Some(RuntimeComponent::Structure(_)) | None => {}
        Some(_) => return Err(fail("Structure slot payload mismatch")),
    }
    if components.present(ComponentSlot::Wall) {
        return Err(required("4CED60 Wall+90 58CB30 parent chain"));
    }
    Ok(None)
}

pub(crate) struct Context<'a> {
    pub actors: &'a mut ActorStore,
}

/// One queue1 periodic delivery to Properties+8 (virtual +14 = 5818D0).
pub(crate) fn run(key: ObjectKey, event: Delivery, cx: &mut Context<'_>) -> aa_world::queues::Result<()> {
    if event.elapsed_bits.is_none() {
        return Err(fail("Properties Updatable +18/+1C are 943800 RET4; deferred delivery is invalid here"));
    }
    let id: ActorId = cx
        .actors
        .id(key)
        .ok_or_else(|| fail("scheduled Properties owner has no live GTID"))?;
    let properties = match cx
        .actors
        .component(id, ComponentSlot::Properties)
        .map_err(fail)?
    {
        Some(RuntimeComponent::Properties(v)) => v,
        Some(_) => return Err(fail("Properties slot contains another component class")),
        None => return Err(fail("scheduled Properties allocation is absent")),
    };
    if properties.owner.0 != id.raw() {
        return Err(fail("Properties owner differs from its scheduled Actor"));
    }
    //Step 1: count is re-read each iteration; no removal occurs on this path.
    let mut index = 0;
    while index < properties.effects.len() {
        let handle = properties.effects[index];
        let instance = cx
            .actors
            .property_allocations
            .instances()
            .get(&handle)
            .ok_or_else(|| fail("effect handle is absent from the same PropertyState instances"))?;
        if instance.enchantment_allegiance.is_some() {
            //5031B0 reads World+E0, instance+4 and EnchantmentData+60; its
            //removal leaves are not bound.
            return Err(required("5031B0 enchantment expiry (EnchantmentData+60) and 5034F0 removal"));
        }
        //Plain A29678 virtual +0 is 6A71F0: XOR AL,AL; RET.
        index += 1;
    }
    let modifier = &properties.modifier;
    let heals = modifier.periodic_heals.len();
    let damage = modifier.periodic_damage.len();
    //Step 2: Actor+60 Body.
    let object = cx.actors.object(key).map_err(fail)?;
    if !object.components().map_err(fail)?.present(ComponentSlot::Body) {
        return Ok(());
    }
    //Step 3: 4CED40 and its flags; then the actor's own +100 bits.
    let mut heal = true;
    if let Some(outer) = outer_4ced40(cx.actors, key)? {
        let _ = outer;
        return Err(required("4CEDA0 ancestor of a non-null 4CED40 outer object"));
    }
    let flags = state_low(cx.actors, key)?;
    if flags & 0x0010_0000 != 0 || flags & 0x4000_0000 != 0 {
        heal = false;
    }
    //Step 4. Each 562BA0 call (request source null) first compares Body+10
    //health with +14 maximum: an ordered health >= maximum returns at once.
    if heal {
        for _ in 0..heals {
            let body = match object.components().map_err(fail)?.get(ComponentSlot::Body) {
                Some(RuntimeComponent::Body(v)) => &v.body,
                _ => return Err(fail("Body disappeared during 5818D0")),
            };
            let health = f32::from_bits(body.health.0);
            let maximum = f32::from_bits(body.maximum.0);
            if !(health >= maximum) {
                return Err(required(
                    "562BA0 healing below maximum (+168 linked Body, 50C380, 561E50, Body+20)",
                ));
            }
        }
    }
    if damage > 0 {
        return Err(required(format!("5625C0 Body damage for {damage} periodic damage record(s)")));
    }
    Ok(())
}
