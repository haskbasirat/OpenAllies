//! Original `KKC_UAIStateSleep::Think` (`52F950`, element state `uai_sleep`
//! at Mind+68, vtable A34798 slot +C): an idle unit follows its pending
//! position, its organization and its leader's pack/unpack posture.
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]); schedule 2.0 s.
//! 2. While current (`52A000`): an actor with a pending list (+110) and
//!    without flag 0x1000000 in +100 whose `4D1820(&point)` succeeds orders
//!    the move state (Mind+7C, `52DF60`, movement-owned) to that point with
//!    movement mode 1 and the null point; done.
//! 3. A membership (actor+80) with an organization (+10) and a clear +1C
//!    byte calls Mind virtual +FC.
//! 4. With actor+AC, an organization, its leader (organization component +44)
//!    and leader+AC, and unless the world time is below state+10 (unordered
//!    counts as below): state+10 = time + 3.0 (`[A32690]`, x87 at the
//!    control-word precision, stored binary32). Then:
//!    - leader `4D1080` and actor `4D10A0`: the unit's formation offset
//!      (component +78 table at the membership +18 index) is kept within
//!      `s = leader virtual +0 + definition(+534)+54 + 1e-4` (stored): a
//!      shorter offset is normalised (`409230`) and scaled by `s`. The
//!      unpack stack action (`51E800`, ordinal 0xB) at leader +20/+24 plus
//!      the offset, facing (0, -1), is delivered to the actor (`4D9B60`),
//!      done.
//!    - leader `4D10A0` or `4D0980`: the pack stack action (`51E850`, ordinal
//!      0xC) is delivered to the actor, done.
//! 5. While current, a pending command is resumed (`52BF30`).
use super::ThinkWorld;
use crate::{
    Arena, MindId, Result, StateId,
    cai_thinks::{
        unit_order::{PACK_ACTION, StackAction, UNPACK_ACTION},
        x87_copy,
    },
    is_current, think_base,
    x87::{n, num, store},
};
use aa_navigation::Activation;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// binary32 2.0: the sleep re-think delay.
pub const SLEEP_DELAY_BITS: u32 = 0x4000_0000;
/// `[A32690]` binary32 3.0: the leader check interval.
pub const CHECK_INTERVAL_BITS: u32 = 0x4040_0000;
/// `[A34770]` binary32 1e-4.
const MARGIN_BITS: u32 = 0x38d1_b717;
/// binary32 -1.0: the unpack facing y.
const FACING_Y_BITS: u32 = 0xbf80_0000;

/// Services of `52F950` in native order.
pub trait SleepThinkWorld: ThinkWorld {
    /// Actor +110 (pending list head) and +100 flags.
    fn actor_word110(&mut self, mind: MindId) -> Result<u32>;
    fn actor_flags100(&mut self, mind: MindId) -> Result<u32>;
    /// `4D1820(&point)` on the actor: the point when it succeeds.
    fn actor_pending_point(&mut self, mind: MindId) -> Result<Option<[u32; 2]>>;
    /// `52DF60` on the move state (movement-owned); result unused here.
    fn move_activate(
        &mut self,
        mind: MindId,
        request: Activation,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Actor +80 (membership), `None` when null; its +10 organization, +1C
    /// byte and +18 formation index.
    fn membership(&mut self, mind: MindId) -> Result<Option<u32>>;
    fn membership_organization(&mut self, membership: u32) -> Result<Option<u32>>;
    fn membership_detached(&mut self, membership: u32) -> Result<bool>;
    fn membership_index(&mut self, membership: u32) -> Result<u32>;
    /// Mind virtual +FC.
    fn mind_virtual_fc(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Actor +AC and an object's +AC word.
    fn actor_word_ac(&mut self, mind: MindId) -> Result<u32>;
    fn object_word_ac(&mut self, object: u32) -> Result<u32>;
    /// Organization component (+7C) +44 (leader), `None` when null.
    fn organization_leader(&mut self, organization: u32) -> Result<Option<u32>>;
    /// Organization component (+7C) +78 table entry `index` (binary32 x, y).
    fn formation_offset(&mut self, organization: u32, index: u32) -> Result<[u32; 2]>;
    /// `[B36534]+E0` world time; sleep state+10 (next leader check) and its
    /// write.
    fn world_time(&mut self) -> Result<u32>;
    fn sleep_time(&mut self, state: StateId) -> Result<u32>;
    fn set_sleep_time(&mut self, state: StateId, bits: u32) -> Result<()>;
    /// Flag predicates `4D1080`, `4D10A0` and `4D0980` on the leader, and
    /// `4D10A0` on the actor.
    fn leader_4d1080(&mut self, leader: u32) -> Result<bool>;
    fn leader_4d10a0(&mut self, leader: u32) -> Result<bool>;
    fn leader_4d0980(&mut self, leader: u32) -> Result<bool>;
    fn actor_4d10a0(&mut self, mind: MindId) -> Result<bool>;
    /// Actor definition +534 -> +54 (binary32).
    fn definition_534_54(&mut self, mind: MindId) -> Result<u32>;
    /// Leader virtual +0, unstored ST0.
    fn leader_virtual_0(&mut self, leader: u32) -> Result<Binary>;
    /// `409230` on a binary32 vector (normalise in place; the returned length
    /// is discarded by the caller).
    fn normalize(&mut self, vector: [u32; 2]) -> Result<[u32; 2]>;
    /// Object +20/+24.
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    /// Builds `action` (`4D9E20` + vtable A3CC38), delivers it to the actor
    /// (`4D9B60(actor)`) and destroys it (`4CA2D0`); the result is unused.
    fn issue_action(
        &mut self,
        mind: MindId,
        action: &StackAction,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// `52BF30` on the Mind (`command::resume_pending`); result unused.
    fn resume_pending(&mut self, mind: MindId, arena: &mut Arena) -> Result<bool>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Step 4 when the leader check is due; returns true when an action was
/// delivered (the Think then ends).
fn follow_leader(
    state: StateId,
    mind: MindId,
    leader: u32,
    organization: u32,
    arena: &mut Arena,
    world: &mut impl SleepThinkWorld,
    p: Precision,
) -> Result<bool> {
    let time = world.world_time()?;
    world.set_sleep_time(
        state,
        store(num(n(time)?.add(&n(CHECK_INTERVAL_BITS)?, p))?)?,
    )?;
    if world.leader_4d1080(leader)? && world.actor_4d10a0(mind)? {
        let membership = world
            .membership(mind)?
            .ok_or_else(|| super::null("membership +18 read"))?;
        let index = world.membership_index(membership)?;
        let [mut x, mut y] = world.formation_offset(organization, index)?;
        let reach = world.definition_534_54(mind)?;
        let v = world.leader_virtual_0(leader)?;
        let s = store(num(num(v.add(&n(reach)?, p))?.add(&n(MARGIN_BITS)?, p))?)?;
        let yy = num(n(y)?.mul(&n(y)?, p))?;
        let xx = num(n(x)?.mul(&n(x)?, p))?;
        let length = num(num(yy.add(&xx, p))?.sqrt(p))?;
        if length.compare(&n(s)?) == Ordering::Less {
            [x, y] = world.normalize([x, y])?;
            let sy = num(n(y)?.mul(&n(s)?, p))?;
            x = store(num(n(x)?.mul(&n(s)?, p))?)?;
            y = store(sy)?;
        }
        let [lx, ly] = world.object_position(leader)?;
        let ty = num(n(y)?.add(&n(ly)?, p))?;
        let tx = store(num(n(x)?.add(&n(lx)?, p))?)?;
        let ty = store(ty)?;
        let action = StackAction {
            ordinal: UNPACK_ACTION,
            words18_24: Some([tx, ty, 0, FACING_Y_BITS].map(x87_copy)),
        };
        world.issue_action(mind, &action, arena)?;
        return Ok(true);
    }
    if world.leader_4d10a0(leader)? || world.leader_4d0980(leader)? {
        let action = StackAction {
            ordinal: PACK_ACTION,
            words18_24: None,
        };
        world.issue_action(mind, &action, arena)?;
        return Ok(true);
    }
    Ok(false)
}

/// Complete `52F950` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl SleepThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    world.schedule(mind, SLEEP_DELAY_BITS, 0, arena)?;
    if is_current(state, arena)?
        && world.actor_word110(mind)? != 0
        && world.actor_flags100(mind)? & 0x0100_0000 == 0
        && let Some(point) = world.actor_pending_point(mind)?
    {
        let request = Activation {
            activation_mode: 2,
            destination_bits: point,
            movement_mode: 1,
            requested_speed_bits: 0,
            auxiliary_bits: world.null_point()?,
        };
        world.move_activate(mind, request, arena)?;
        return Ok(wrong);
    }
    if let Some(m) = world.membership(mind)?
        && world.membership_organization(m)?.is_some()
        && !world.membership_detached(m)?
    {
        world.mind_virtual_fc(mind, arena)?;
    }
    let organization = match world.membership(mind)? {
        Some(m) => world.membership_organization(m)?,
        None => None,
    };
    if world.actor_word_ac(mind)? != 0
        && let Some(organization) = organization
        && let Some(leader) = world.organization_leader(organization)?
        && world.object_word_ac(leader)? != 0
    {
        let time = world.world_time()?;
        let due = world.sleep_time(state)?;
        // `fcomp; test ah, 1`: below or unordered skips the check.
        if !(host(time) < host(due) || host(time).is_nan() || host(due).is_nan())
            && follow_leader(state, mind, leader, organization, arena, world, p)?
        {
            return Ok(wrong);
        }
    }
    if is_current(state, arena)? {
        world.resume_pending(mind, arena)?;
    }
    Ok(wrong)
}
