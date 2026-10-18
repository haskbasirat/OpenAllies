//! Original `KKC_CAIStateEngage::Think` (`526A70`, regiment state `cai_engage`
//! at Mind+1CC, vtable A33140 slot +C).
//!
//! The engage state keeps the sorted target list (`cai_engage::Targets`;
//! the tail node is the highest priority target), the last seen attacker
//! GTID (+20) and the "every member engaged" byte (+1C).
//!
//! Order (diagnostic calls excluded):
//! 1. `529E60`; an organization in status 1 (`578140`) with a nonzero +118
//!    takes the wavering reaction (`51F920`) and ends the Think.
//! 2. Without a resolvable tail target, Mind virtual +140(tail GTID or 0)
//!    ends the Think.
//! 3. When Mind virtual +4C (current attacker) differs from the resolved +20,
//!    +20 becomes that attacker's GTID; with the SVar
//!    `KillTargetIsHighestPriority` an attacker in range (`578470`) is added
//!    (`524C10`; a true result ends the Think without rescheduling) and every
//!    member Mind gets virtual +17C.
//! 4. The chasing state (Mind+120, `520D90`) for the current tail GTID may
//!    hand the target to Mind virtual +F8 (true ends without rescheduling).
//! 5. Members not busy (+6C) and without flag 0x200 at +100 get +F0(target)
//!    for the target resolved in step 2.
//! 6. +1C = every member Mind has a +E4 object in status 2 (+18) whose
//!    member target (+4C) the organization follows (+164); a member without a
//!    Mind is skipped.
//! 7. Schedule the next Think in 1.0 s.
//!
//! GTIDs resolve through `EngageWorld::resolve` (generation match and a
//! nonnull registry entry).
//!
//! Member iteration is `4364C0` (flag 0): component (+7C) array +28 with the
//! count +2C reread at every step, null slots skipped, `0x7FFFFFFF` ends.
use crate::cai_engage::{EngageWorld, Targets, mind_add_target};
use crate::{Arena, Error, MindId, Result, StateId, World, think_base};

/// binary32 1.0: engage re-think delay.
pub const ENGAGE_DELAY_BITS: u32 = 0x3f80_0000;
const END: i32 = 0x7fff_ffff;

/// Services in native order; `EngageWorld` resolves GTIDs to actor handles.
pub trait EngageThinkWorld: EngageWorld + World {
    /// `578140` on the organization component (+7C).
    fn organization_status(&mut self, mind: MindId, arena: &Arena) -> Result<u32>;
    /// Organization +118 is nonzero.
    fn organization_word118(&mut self, mind: MindId, arena: &Arena) -> Result<bool>;
    /// `51F920` (`cai_reactions::wavering`); result unused.
    fn wavering_reaction(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Mind virtual +140(gtid).
    fn mind_virtual_140(&mut self, mind: MindId, gtid: u32, arena: &mut Arena) -> Result<()>;
    /// Mind virtual +4C (current attacker), `None` when null.
    fn mind_virtual_4c(&mut self, mind: MindId, arena: &mut Arena) -> Result<Option<u32>>;
    /// Engage state +20 (last attacker GTID).
    fn last_attacker(&mut self, state: StateId) -> Result<u32>;
    fn set_last_attacker(&mut self, state: StateId, gtid: u32) -> Result<()>;
    /// SVar `KillTargetIsHighestPriority` (bool, `[B39AA8]` registry).
    fn kill_target_is_highest_priority(&mut self) -> Result<bool>;
    /// `578470(actor)` on the organization component (squared distance below
    /// the organization's +304 range).
    fn within_engage_range(&mut self, mind: MindId, actor: u32, arena: &Arena) -> Result<bool>;
    /// Component +2C member count and +28 slot `index`.
    fn member_count(&mut self, mind: MindId, arena: &Arena) -> Result<i32>;
    fn member_slot(&mut self, mind: MindId, index: i32, arena: &Arena) -> Result<Option<u32>>;
    /// Member +70 (its Mind), `None` when null.
    fn member_mind(&mut self, member: u32) -> Result<Option<u32>>;
    /// Member Mind virtual +17C.
    fn member_mind_virtual_17c(&mut self, member_mind: u32) -> Result<()>;
    /// `520D90(gtid)` on the chasing state (Mind+120).
    fn chase_continue(&mut self, mind: MindId, gtid: u32, arena: &mut Arena) -> Result<bool>;
    /// Mind virtual +F8(actor or 0).
    fn mind_virtual_f8(&mut self, mind: MindId, actor: u32, arena: &mut Arena) -> Result<bool>;
    /// Member Mind virtual +6C (busy).
    fn member_mind_virtual_6c(&mut self, member_mind: u32) -> Result<bool>;
    /// Member +100 word.
    fn member_flags100(&mut self, member: u32) -> Result<u32>;
    /// Member Mind virtual +F0(target).
    fn member_mind_virtual_f0(&mut self, member_mind: u32, target: u32) -> Result<()>;
    /// Member +E4 object virtual +18 (`None` when the object is null).
    fn member_e4_status(&mut self, member: u32) -> Result<Option<u32>>;
    /// Member Mind virtual +4C, `None` when null.
    fn member_mind_virtual_4c(&mut self, member_mind: u32) -> Result<Option<u32>>;
    /// Organization virtual +164(actor).
    fn organization_follows(&mut self, mind: MindId, actor: u32, arena: &mut Arena)
    -> Result<bool>;
    /// Engage state +1C byte.
    fn set_all_engaged(&mut self, state: StateId, value: bool) -> Result<()>;
}

/// `4364C0` with flag 0: the next nonnull member slot after `index`.
fn next_member(
    world: &mut impl EngageThinkWorld,
    mind: MindId,
    index: i32,
    arena: &Arena,
) -> Result<i32> {
    let mut i = index.wrapping_add(1);
    while i < world.member_count(mind, arena)? {
        if world.member_slot(mind, i, arena)?.is_some() {
            return Ok(i);
        }
        i += 1;
    }
    Ok(END)
}
fn member(
    world: &mut impl EngageThinkWorld,
    mind: MindId,
    index: i32,
    arena: &Arena,
) -> Result<u32> {
    world
        .member_slot(mind, index, arena)?
        .ok_or_else(|| Error::World("null organization member slot".into()))
}
fn tail_gtid(targets: &Targets) -> u32 {
    targets.nodes.last().map_or(0, |n| n.gtid)
}

/// Complete `526A70`. `chasing` is the Mind+120 state used by `524C10`.
/// Returns the base Think's wrong-activity flag.
pub fn think(
    state: StateId,
    chasing: StateId,
    targets: &mut Targets,
    arena: &mut Arena,
    world: &mut impl EngageThinkWorld,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    if world.organization_status(mind, arena)? == 1 && world.organization_word118(mind, arena)? {
        world.wavering_reaction(mind, arena)?;
        return Ok(wrong);
    }
    let resolved = if targets.nodes.is_empty() {
        None
    } else {
        world.resolve(tail_gtid(targets))?
    };
    let Some(target) = resolved else {
        let gtid = tail_gtid(targets);
        world.mind_virtual_140(mind, gtid, arena)?;
        return Ok(wrong);
    };
    let attacker = world.mind_virtual_4c(mind, arena)?;
    let last = world.last_attacker(state)?;
    let known = world.resolve(last)?;
    if attacker != known {
        let gtid = match attacker {
            Some(a) => world.gtid(a)?,
            None => 0,
        };
        world.set_last_attacker(state, gtid)?;
        if world.kill_target_is_highest_priority()? {
            if let Some(a) = attacker
                && world.within_engage_range(mind, a, arena)?
                && mind_add_target(targets, state, chasing, Some(a), arena, world)?
            {
                return Ok(wrong);
            }
            let mut i = next_member(world, mind, -1, arena)?;
            while i != END {
                let m = member(world, mind, i, arena)?;
                if let Some(mm) = world.member_mind(m)? {
                    world.member_mind_virtual_17c(mm)?;
                }
                i = next_member(world, mind, i, arena)?;
            }
        }
    }
    let gtid = tail_gtid(targets);
    if world.chase_continue(mind, gtid, arena)? {
        let gtid = tail_gtid(targets);
        let actor = world.resolve(gtid)?.unwrap_or(0);
        if world.mind_virtual_f8(mind, actor, arena)? {
            return Ok(wrong);
        }
    }
    let mut i = next_member(world, mind, -1, arena)?;
    while i != END {
        let m = member(world, mind, i, arena)?;
        let mm = world
            .member_mind(m)?
            .ok_or_else(|| Error::World("null member Mind dereference".into()))?;
        if !world.member_mind_virtual_6c(mm)? && world.member_flags100(m)? & 0x200 == 0 {
            world.member_mind_virtual_f0(mm, target)?;
        }
        i = next_member(world, mind, i, arena)?;
    }
    world.set_all_engaged(state, true)?;
    let mut i = next_member(world, mind, -1, arena)?;
    while i != END {
        let m = member(world, mind, i, arena)?;
        if let Some(mm) = world.member_mind(m)? {
            let engaged = world.member_e4_status(m)? == Some(2)
                && match world.member_mind_virtual_4c(mm)? {
                    Some(t) => world.organization_follows(mind, t, arena)?,
                    None => false,
                };
            if !engaged {
                world.set_all_engaged(state, false)?;
                break;
            }
        }
        i = next_member(world, mind, i, arena)?;
    }
    world.schedule(mind, ENGAGE_DELAY_BITS, 0, arena)?;
    Ok(wrong)
}
