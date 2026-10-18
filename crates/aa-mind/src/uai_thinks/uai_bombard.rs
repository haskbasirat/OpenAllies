//! Original `KKC_UAIStateBombard::Think` (`5353E0`, element state
//! `uai_bombard` at Mind+10C, vtable A35480 slot +C): bombard the object
//! whose GTID is state+10, or the point state+18/+1C.
//!
//! Order (diagnostic log lines omitted):
//! 1. `529E60` ([`crate::think_base`]). The target (state+10 GTID) is
//!    resolved. A performer (actor+74, null is a native null read) that is
//!    the actor's current performer (`4DB0F0`) and not finished (+10 != 4)
//!    ends the Think ("waiting for ability").
//! 2. With a target whose virtual +5C(owner, 1) (owner = actor virtual +108)
//!    fails: a target with a +6C word reports (when the actor has no
//!    organization: `42D470([B39D8C], 0x26, owner, &target+20, actor name)`)
//!    and goes to Mind virtual +140(target GTID); otherwise the target is
//!    dropped (`bl` false). A target passing +5C keeps `bl` true.
//! 3. When state+14 (a CString) is not empty and `bl` is false: the actor
//!    owner is read; without a target, `54F3C0(owner)` on the world zone
//!    `42A530(state+18/+1C)` (world+30) true goes to Mind virtual +144.
//! 4. `bl` true: `4D4FF0(target, 3, 1)` on the actor; 1 approaches, 0
//!    attacks, other values go to Mind virtual +140(target GTID).
//!    `bl` false: `4D5180(&state+18, 1)` on the actor; 1 approaches, 0
//!    attacks, other values go to Mind virtual +144.
//! 5. Approach: no locomotion (actor+6C) goes to Mind virtual +124. A
//!    performer gets `57DD90(3, object(state+10), &state+18)`; an inactive
//!    move state is ordered to state+18/+1C (`52DF60(0, point, 1, 0,
//!    [B279C0], [B279C4])`, movement-owned); the move state Thinks.
//! 6. Attack: an active move state is left (`529C20`). With `bl`: the target
//!    without a component (+7C) is chosen itself; otherwise the nearest
//!    member of its component (`4364C0` member iterator, flag 0, null slots
//!    skipped; binary32 squared distance to the actor, unstored candidate
//!    against the stored best, x87 at the control-word precision); no member
//!    goes to Mind virtual +140(target+14). The choice runs `5340C0(object,
//!    null, &record, 1)` ([`crate::uai_kill::choose`]) and `5307A0(&record,
//!    0)` ([`crate::ability_decision::apply`]). Without `bl`:
//!    `5340C0(null, &state+18, &record, 1)` and `5307A0(&record, &state+18)`.
use super::{ThinkWorld, element, leave_if_active, null, status_at};
use crate::{
    Arena, MindId, Result, StateId, think_base,
    uai_kill::{KillDecision, PerformerState},
    x87::{n, num, store},
};
use aa_navigation::Activation;
use aa_numeric::Precision;
use std::cmp::Ordering;

/// `42D470` notification kind of the "target lost" report.
pub const REPORT_KIND: u32 = 0x26;

/// Services of `5353E0` in native order.
pub trait BombardThinkWorld: ThinkWorld {
    /// Bombard state+10 (target GTID), +14 CString length and +18/+1C point.
    fn bombard_target(&mut self, state: StateId) -> Result<u32>;
    fn area_length(&mut self, state: StateId) -> Result<u32>;
    fn bombard_point(&mut self, state: StateId) -> Result<[u32; 2]>;
    /// Actor +74 performer state (`None` when the performer is null).
    fn performer(&mut self, mind: MindId) -> Result<Option<PerformerState>>;
    /// Actor virtual +108 (owner).
    fn actor_owner(&mut self, mind: MindId, arena: &mut Arena) -> Result<u32>;
    /// Target virtual +5C(owner, flag).
    fn target_virtual_5c(&mut self, target: u32, owner: u32, flag: u32) -> Result<bool>;
    /// Object +6C word.
    fn object_word6c(&mut self, object: u32) -> Result<u32>;
    /// Actor +80 -> +10 (organization), `None` when either is null.
    fn organization(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// Actor virtual +24 (its name).
    fn actor_name(&mut self, mind: MindId, arena: &mut Arena) -> Result<String>;
    /// Object +20/+24.
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    /// `42D470([B39D8C], kind, owner, &point, name)`.
    fn report(&mut self, kind: u32, owner: u32, point: [u32; 2], name: &str) -> Result<()>;
    /// `42A530(point)` on world+30 (zone), then `54F3C0(owner)` on it.
    fn zone_at(&mut self, point: [u32; 2]) -> Result<u32>;
    fn zone_check(&mut self, zone: u32, owner: u32) -> Result<bool>;
    /// Mind virtuals +144 and +124.
    fn mind_virtual_144(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    fn mind_virtual_124(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// `4D4FF0(target, a, b)` and `4D5180(&point, flag)` on the actor.
    fn relation(&mut self, mind: MindId, target: u32, a: u32, b: u32) -> Result<i32>;
    fn point_relation(&mut self, mind: MindId, point: [u32; 2], flag: u32) -> Result<i32>;
    /// `57DD90(mode, object, &point)` on the performer (actor+74).
    fn performer_approach(
        &mut self,
        mind: MindId,
        mode: u32,
        object: Option<u32>,
        point: [u32; 2],
    ) -> Result<()>;
    /// `52DF60` on the move state (movement-owned); result unused here.
    fn move_activate(
        &mut self,
        mind: MindId,
        request: Activation,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Object +7C (component), `None` when null, and the component's member
    /// slots (+28 array, +2C count) in order, `None` for null slots.
    fn object_component(&mut self, object: u32) -> Result<Option<u32>>;
    fn component_members(&mut self, component: u32) -> Result<Vec<Option<u32>>>;
    /// Mind actor +20/+24.
    fn actor_position(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// `5340C0(target, point, &record, 1)` on the Mind.
    fn choose(
        &mut self,
        mind: MindId,
        target: Option<u32>,
        point: Option<[u32; 2]>,
        record: &mut KillDecision,
    ) -> Result<u32>;
    /// `5307A0(&record, point)` on the Mind.
    fn apply(
        &mut self,
        mind: MindId,
        record: &mut KillDecision,
        point: Option<[u32; 2]>,
        arena: &mut Arena,
    ) -> Result<()>;
}

/// The member of `component` nearest to the actor (step 6).
fn nearest_member(
    mind: MindId,
    component: u32,
    world: &mut impl BombardThinkWorld,
    p: Precision,
) -> Result<Option<u32>> {
    let mut best: Option<(u32, u32)> = None;
    for member in world.component_members(component)?.into_iter().flatten() {
        let [mx, my] = world.object_position(member)?;
        let [ax, ay] = world.actor_position(mind)?;
        let dy = num(n(my)?.sub(&n(ay)?, p))?;
        let dx = num(n(mx)?.sub(&n(ax)?, p))?;
        let d2 = num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))?;
        match best {
            Some((_, stored)) if d2.compare(&n(stored)?) != Ordering::Less => {}
            _ => best = Some((member, store(d2)?)),
        }
    }
    Ok(best.map(|(member, _)| member))
}

fn lost(
    mind: MindId,
    target: u32,
    arena: &mut Arena,
    world: &mut impl BombardThinkWorld,
) -> Result<()> {
    let gtid = world.object_gtid(target)?;
    world.mind_virtual_140(mind, gtid, arena)
}

/// Complete `5353E0` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl BombardThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.bombard_target(state)?;
    let target = world.resolve(gtid)?;
    let performer = world
        .performer(mind)?
        .ok_or_else(|| null("4DB0F0 on a null performer"))?;
    if performer.current && performer.stage != 4 {
        return Ok(wrong);
    }
    let mut keep = target.is_some();
    if let Some(t) = target {
        let owner = world.actor_owner(mind, arena)?;
        if !world.target_virtual_5c(t, owner, 1)? {
            if world.object_word6c(t)? == 0 {
                keep = false;
            } else {
                if world.organization(mind)?.is_none() {
                    let name = world.actor_name(mind, arena)?;
                    let point = world.object_position(t)?;
                    let owner = world.actor_owner(mind, arena)?;
                    world.report(REPORT_KIND, owner, point, &name)?;
                }
                lost(mind, t, arena, world)?;
                return Ok(wrong);
            }
        }
    }
    if world.area_length(state)? != 0 && !keep {
        let owner = world.actor_owner(mind, arena)?;
        if target.is_none() {
            let point = world.bombard_point(state)?;
            let zone = world.zone_at(point)?;
            if world.zone_check(zone, owner)? {
                world.mind_virtual_144(mind, arena)?;
                return Ok(wrong);
            }
        }
    }
    let decision = if keep {
        let t = target.ok_or_else(|| null("kept target"))?;
        match world.relation(mind, t, 3, 1)? {
            r @ (0 | 1) => r,
            _ => {
                lost(mind, t, arena, world)?;
                return Ok(wrong);
            }
        }
    } else {
        let point = world.bombard_point(state)?;
        match world.point_relation(mind, point, 1)? {
            r @ (0 | 1) => r,
            _ => {
                world.mind_virtual_144(mind, arena)?;
                return Ok(wrong);
            }
        }
    };
    if decision == 1 {
        if world.locomotion(mind)?.is_none() {
            world.mind_virtual_124(mind, arena)?;
            return Ok(wrong);
        }
        if world.performer(mind)?.is_some() {
            let gtid = world.bombard_target(state)?;
            let object = world.resolve(gtid)?;
            let point = world.bombard_point(state)?;
            world.performer_approach(mind, 3, object, point)?;
        }
        if status_at(world, mind, element::MOVE, arena)? != 2 {
            let request = Activation {
                activation_mode: 0,
                destination_bits: world.bombard_point(state)?,
                movement_mode: 1,
                requested_speed_bits: 0,
                auxiliary_bits: world.null_point()?,
            };
            world.move_activate(mind, request, arena)?;
        }
        world.embedded_think(mind, element::MOVE, arena)?;
        return Ok(wrong);
    }
    leave_if_active(world, mind, element::MOVE, arena)?;
    let mut record = KillDecision::initial();
    if keep {
        let t = target.ok_or_else(|| null("kept target"))?;
        let object = match world.object_component(t)? {
            None => t,
            Some(component) => match nearest_member(mind, component, world, p)? {
                Some(member) => member,
                None => {
                    let gtid = world.object_gtid(t)?;
                    world.mind_virtual_140(mind, gtid, arena)?;
                    return Ok(wrong);
                }
            },
        };
        world.choose(mind, Some(object), None, &mut record)?;
        world.apply(mind, &mut record, None, arena)?;
    } else {
        let point = world.bombard_point(state)?;
        world.choose(mind, None, Some(point), &mut record)?;
        world.apply(mind, &mut record, Some(point), arena)?;
    }
    Ok(wrong)
}
