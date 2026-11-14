//! Original element kill-state hand-over, reached from the move state's
//! collision with an enemy thing (`536840` ResultCollisionThing) and from the
//! engage Think (`536E10`):
//!
//! * `5358A0(object)` on any element state (only its Mind is read): hand an
//!   object to the kill state ([`hand_to_kill`]).
//! * `5351C0(mode, object)` on the kill state (Mind+F8): activate the kill
//!   state on an attackable object ([`kill_select`]).
//! * `533D90(object)` on the kill state: replace its target GTID (state+10,
//!   Mind+108), releasing the old target and ordering the new one
//!   ([`set_kill_target`]).
//!
//! Element Mind embedded states used: move +7C (status Mind+88) and kill +F8
//! (status Mind+104, target GTID Mind+108).
use super::{ThinkWorld, element, leave_if_active, status_at};
use crate::{Arena, MindId, Result, activate};

/// Services of `5358A0`, `5351C0` and `533D90`, in native order.
pub trait KillSelectWorld: ThinkWorld {
    /// Kill state +10 (Mind+108, target GTID) and its write.
    fn kill_state_target(&mut self, mind: MindId) -> Result<u32>;
    fn set_kill_state_target(&mut self, mind: MindId, gtid: u32) -> Result<()>;
    /// Locomotion +56 byte (the actor+6C locomotion is
    /// [`ThinkWorld::locomotion`]).
    fn locomotion_byte56(&mut self, locomotion: u32) -> Result<bool>;
    /// `4D4FF0(object, a, b)` on the actor (0 = allowed).
    fn actor_relation(&mut self, mind: MindId, object: u32, a: u32, b: u32) -> Result<i32>;
    /// `4D0D70(actor)` on the former kill target.
    fn target_released(&mut self, target: u32, mind: MindId) -> Result<()>;
    /// Actor +74 (performer), `None` when null.
    fn actor_performer(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// `57DEA0` on a performer.
    fn performer_halt(&mut self, performer: u32) -> Result<()>;
    /// `532930(mode, object, a3, a4, point)` on the move state (Mind+7C),
    /// movement-owned; the result is unused here.
    #[allow(clippy::too_many_arguments)]
    fn kill_move_to_object(
        &mut self,
        mind: MindId,
        mode: u32,
        object: u32,
        a3: u32,
        a4: u32,
        point: [u32; 2],
        arena: &mut Arena,
    ) -> Result<()>;
    /// The object is the Mind's own actor (`529C30`).
    fn object_is_actor(&mut self, mind: MindId, object: u32) -> Result<bool>;
    /// `4D3A20(actor)` on the new kill target.
    fn target_acquired(&mut self, object: u32, mind: MindId) -> Result<()>;
    /// Actor virtual +108(&scratch) with a zeroed two-word scratch, then
    /// object virtual +88 on the pointer it returns.
    fn kill_in_reach(&mut self, mind: MindId, object: u32) -> Result<bool>;
    /// `57DD90(1, object, &scratch)` on the actor +74 performer (not
    /// null-checked by the original), with the scratch left by
    /// [`Self::kill_in_reach`].
    fn kill_approach(&mut self, mind: MindId, object: u32) -> Result<()>;
}

/// `5358A0(object)` on an element state of `mind`. Returns true when the
/// object was handed to the kill state.
///
/// Null object: an active kill state (status 2) is left (`529C20`); false.
/// Otherwise, unless the kill state is active (2) on the object's GTID
/// already, an active move state (2) is left and `5351C0(0, object)` runs on
/// the kill state (its result unused); true.
pub fn hand_to_kill(
    mind: MindId,
    object: Option<u32>,
    arena: &mut Arena,
    world: &mut impl KillSelectWorld,
) -> Result<bool> {
    let Some(object) = object else {
        if status_at(world, mind, element::KILL, arena)? == 2 {
            leave_if_active(world, mind, element::KILL, arena)?;
        }
        return Ok(false);
    };
    if status_at(world, mind, element::KILL, arena)? == 2
        && world.object_gtid(object)? == world.kill_state_target(mind)?
    {
        return Ok(false);
    }
    if status_at(world, mind, element::MOVE, arena)? == 2 {
        leave_if_active(world, mind, element::MOVE, arena)?;
    }
    kill_select(mind, 0, Some(object), arena, world)?;
    Ok(true)
}

/// `5351C0(mode, object)` on the kill state of `mind`.
///
/// Order: `529C00(mode)` (modes other than 0 and 1 ask the kill state's
/// virtual +4); the actor +6C locomotion must exist with a clear +56 byte;
/// a null object fails; `4D4FF0(object, 1, 0)` must be 0. Then, when the
/// kill target GTID differs from the object's, an inactive kill state
/// (status != 2) is activated in `mode` (`52A810`) and `533D90(object)`
/// sets the target. Returns true from the relation test on.
pub fn kill_select(
    mind: MindId,
    mode: u32,
    object: Option<u32>,
    arena: &mut Arena,
    world: &mut impl KillSelectWorld,
) -> Result<bool> {
    let kill = world.embedded_state(mind, element::KILL)?;
    if mode != 0 && mode != 1 && !world.can_activate(kill, arena)? {
        return Ok(false);
    }
    match world.locomotion(mind)? {
        Some(l) if !world.locomotion_byte56(l)? => {}
        _ => return Ok(false),
    }
    let Some(object) = object else {
        return Ok(false);
    };
    if world.actor_relation(mind, object, 1, 0)? != 0 {
        return Ok(false);
    }
    if world.kill_state_target(mind)? != world.object_gtid(object)? {
        if arena.state(kill)?.status != 2 {
            activate(kill, mode, arena, world)?;
        }
        set_kill_target(mind, Some(object), arena, world)?;
    }
    Ok(true)
}

/// `533D90(object)` on the kill state of `mind`.
///
/// Nothing happens when the object is already the target (a null object:
/// the target GTID is 0; otherwise the GTID resolves to the object). A
/// nonzero former target that still resolves gets `4D0D70(actor)`; the GTID
/// is cleared, the actor +74 performer (when present) halts (`57DEA0`) and
/// the move state is left when active (`529C20`). A new object: its GTID is
/// stored; unless it is the unit's own actor, `532930(0, object, 2, 0,
/// [B279C0])` orders the move state; `4D3A20(actor)` on the object; when it
/// is in reach (actor +108, object +88) the performer approaches it
/// (`57DD90(1, object, &scratch)`).
pub fn set_kill_target(
    mind: MindId,
    object: Option<u32>,
    arena: &mut Arena,
    world: &mut impl KillSelectWorld,
) -> Result<()> {
    let gtid = world.kill_state_target(mind)?;
    let same = match object {
        None => gtid == 0,
        Some(o) => world.resolve(gtid)? == Some(o),
    };
    if same {
        return Ok(());
    }
    if gtid != 0 {
        if let Some(old) = world.resolve(gtid)? {
            world.target_released(old, mind)?;
        }
        world.set_kill_state_target(mind, 0)?;
        if let Some(performer) = world.actor_performer(mind)? {
            world.performer_halt(performer)?;
        }
        leave_if_active(world, mind, element::MOVE, arena)?;
    }
    let Some(object) = object else {
        return Ok(());
    };
    let new = world.object_gtid(object)?;
    world.set_kill_state_target(mind, new)?;
    if !world.object_is_actor(mind, object)? {
        let point = world.null_point()?;
        world.kill_move_to_object(mind, 0, object, 2, 0, point, arena)?;
    }
    world.target_acquired(object, mind)?;
    if world.kill_in_reach(mind, object)? {
        world.kill_approach(mind, object)?;
    }
    Ok(())
}
