//! Original `KKC_CAIStateBombard::Activate` (`527B00(mode, target)`,
//! regiment state `cai_bombard` at Mind+D4, Source/Sim/Tai/CAI.cpp) and its
//! target setter `527A60`.
//!
//! Order of `527B00`:
//! 1. A null target, or an organization component (organization +7C)
//!    failing `57A5E0`: false.
//! 2. The state is current (`52A000`) on the same target (state+10 equals
//!    the target's GTID): when Mind+C is 0 the next update is scheduled in
//!    0.6 s (Mind+8 virtual +4(0.6, 0)); true.
//! 3. `4D4FF0(target, 3, 4D1080())` on the organization (`4D1080`: actor
//!    +104 bit 2): 5 (in fog) and 0 go on, anything else is ignored (false).
//!    The diagnostic logs (CAI.cpp lines 0x73B and 0x741) are omitted.
//! 4. `527A60(target)`, the point state+18/+1C = target +20/+24, then
//!    `52A810(mode)` and its result.
use crate::{Arena, MindId, Result, StateId, World, activate, is_current};

/// binary32 0.6: the re-activation delay of a current bombard state.
pub const REACTIVATE_DELAY_BITS: u32 = 0x3f19_999a;
/// `4D4FF0` answer for a target in fog (bombarded anyway).
pub const RELATION_FOG: i32 = 5;

/// Services of `527B00` and `527A60`, in native order.
pub trait BombardActivateWorld: World {
    /// `57A5E0` on the organization component (organization +7C).
    fn component_57a5e0(&mut self, mind: MindId) -> Result<bool>;
    /// Bombard state +10 (target GTID).
    fn bombard_target_gtid(&mut self, state: StateId) -> Result<u32>;
    /// Object +14 (its GTID).
    fn object_gtid(&mut self, object: u32) -> Result<u32>;
    /// Mind+C (the Updatable word read before rescheduling).
    fn mind_word_c(&mut self, mind: MindId) -> Result<u32>;
    /// `4D1080` on the organization: actor +104 bit 2.
    fn organization_4d1080(&mut self, mind: MindId) -> Result<bool>;
    /// `4D4FF0(target, a, b)` on the organization.
    fn organization_relation(&mut self, mind: MindId, target: u32, a: u32, b: u32) -> Result<i32>;
    /// `527A60(target)` on the bombard state ([`set_target`]).
    fn bombard_set_target(&mut self, state: StateId, target: Option<u32>) -> Result<()>;
    /// Object +20/+24 (binary32).
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    /// Bombard state +18/+1C write.
    fn set_bombard_point(&mut self, state: StateId, point: [u32; 2]) -> Result<()>;
}

/// Complete `527B00(mode, target)`.
pub fn activate_bombard(
    state: StateId,
    mode: u32,
    target: Option<u32>,
    arena: &mut Arena,
    world: &mut impl BombardActivateWorld,
) -> Result<bool> {
    let Some(target) = target else {
        return Ok(false);
    };
    let mind = arena.state(state)?.parent;
    if !world.component_57a5e0(mind)? {
        return Ok(false);
    }
    if is_current(state, arena)?
        && world.bombard_target_gtid(state)? == world.object_gtid(target)?
    {
        if world.mind_word_c(mind)? == 0 {
            world.schedule(mind, REACTIVATE_DELAY_BITS, 0, arena)?;
        }
        return Ok(true);
    }
    let flag = u32::from(world.organization_4d1080(mind)?);
    let relation = world.organization_relation(mind, target, 3, flag)?;
    if relation != RELATION_FOG && relation != 0 {
        return Ok(false);
    }
    world.bombard_set_target(state, Some(target))?;
    let point = world.object_position(target)?;
    world.set_bombard_point(state, point)?;
    activate(state, mode, arena, world)
}

/// Services of `527A60`.
pub trait BombardTargetWorld {
    /// Bombard state +10 write (target GTID).
    fn set_bombard_target_gtid(&mut self, state: StateId, gtid: u32) -> Result<()>;
    /// Object +14 (its GTID).
    fn target_gtid(&mut self, object: u32) -> Result<u32>;
    /// Object virtual +24: its name.
    fn object_name(&mut self, object: u32) -> Result<String>;
    /// Bombard state +14 CString: assigned (`405B80`), or emptied (`401170`)
    /// with `None`.
    fn set_bombard_label(&mut self, state: StateId, label: Option<String>) -> Result<()>;
}

/// `527A60(target)` on the bombard state: a target stores its GTID (+10)
/// and its name (+14); null stores 0 and empties the name.
pub fn set_target(
    state: StateId,
    target: Option<u32>,
    world: &mut impl BombardTargetWorld,
) -> Result<()> {
    match target {
        None => {
            world.set_bombard_target_gtid(state, 0)?;
            world.set_bombard_label(state, None)
        }
        Some(t) => {
            let gtid = world.target_gtid(t)?;
            world.set_bombard_target_gtid(state, gtid)?;
            let name = world.object_name(t)?;
            world.set_bombard_label(state, Some(name))
        }
    }
}
