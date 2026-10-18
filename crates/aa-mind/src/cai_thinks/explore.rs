//! Original `KKC_CAIStateExplore::Think` (`525D70`, regiment state
//! `cai_explore` at Mind+220, vtable A32BEC slot +C) with the Tai arrival
//! test `51F200`.
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`). The regiment follows its group
//!    when the organization's group (+164 GTID, resolved non-null; `4D1900`)
//!    fails `4DCE60(organization)`.
//! 2. Following: the group's leader (+194 GTID, resolved non-null) Mind's
//!    (+70) explore target (+238..+248) becomes the best target unless its
//!    point equals the state's target point (state+18/+1C, ordered equal);
//!    then an active move state is left (`529C20`). Not following: a target
//!    point inside the map (`4282A0`) that is not yet reached (`51F200`:
//!    the live-precision distance from the organization above the SVar
//!    `[[B39AB0]+A0]` radius) keeps the move; otherwise an active move state
//!    is left.
//! 3. An active move state thinks and ends the Think.
//! 4. Not following: an explore point (state+10/+14) outside the map is
//!    clamped (`552640`). `[[B39AB0]+74]` candidates come from `524170` and
//!    pass the map bounds and `4F46E0(organization, &candidate, 4.0)`;
//!    `[[B39AB0]+78]` candidates come from `5242C0` and pass the map bounds.
//!    A candidate wins with a greater (signed) count, or an equal count and a
//!    greater score (ordered).
//! 5. A best target inside the map bounds (binary32 compare with 0 and the
//!    map +C/+10 size) is adopted: an active sleep state is left, state+18..
//!    +28 = best, a leading group shares it (`4DCF00(&state+18)`), and the
//!    move state is ordered (`520440(0, target, 0, null point)`) or, when
//!    active, retargeted (`51E9B0(x, y)`); then the move state thinks.
//!    Otherwise the sleep state is activated in mode 0 unless active and
//!    thinks.
use super::{MOVE_OFFSET, RegimentWorld, SLEEP_OFFSET, leave_active};
use crate::{
    Arena, Error, MindId, Result, StateId, activate, think_base,
    x87::{n, num},
};
use aa_numeric::Precision;
use std::cmp::Ordering;

/// binary32 -1.0 and -10000.0: the empty candidate.
const MINUS_ONE: u32 = 0xbf80_0000;
const NO_SCORE: u32 = 0xc61c_4000;
/// binary32 4.0 passed to `4F46E0`.
pub const CANDIDATE_RADIUS_BITS: u32 = 0x4080_0000;

/// An explore target or candidate: x, y, score (binary32 words), signed
/// count and a word (state+18..+28 layout).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub x: u32,
    pub y: u32,
    pub score: u32,
    pub count: u32,
    pub word: u32,
}
impl Candidate {
    pub fn words(self) -> [u32; 5] {
        [self.x, self.y, self.score, self.count, self.word]
    }
    pub fn from_words(w: [u32; 5]) -> Self {
        Self {
            x: w[0],
            y: w[1],
            score: w[2],
            count: w[3],
            word: w[4],
        }
    }
}

/// Services in native order.
pub trait ExploreThinkWorld: RegimentWorld {
    /// Organization +164 (group GTID).
    fn organization_word164(&mut self, mind: MindId) -> Result<u32>;
    /// `4DCE60(organization)` on the group.
    fn group_4dce60(&mut self, group: u32, mind: MindId, arena: &mut Arena) -> Result<bool>;
    /// `4DCF00(&state+18)` on the group.
    fn group_4dcf00(&mut self, group: u32, state: StateId, arena: &mut Arena) -> Result<()>;
    /// Group +194 (leader GTID).
    fn group_word194(&mut self, group: u32) -> Result<u32>;
    /// Object +70 (its Mind), `None` when null.
    fn object_mind(&mut self, object: u32) -> Result<Option<u32>>;
    /// Leader Mind +238..+248.
    fn mind_explore_target(&mut self, leader_mind: u32) -> Result<[u32; 5]>;
    /// Explore state +18..+28 and +10/+14.
    fn explore_target(&mut self, state: StateId) -> Result<[u32; 5]>;
    fn set_explore_target(&mut self, state: StateId, target: [u32; 5]) -> Result<()>;
    fn explore_point(&mut self, state: StateId) -> Result<[u32; 2]>;
    /// `4282A0(x, y)` on the region map `[B36534]+30`.
    fn region_contains(&mut self, point: [u32; 2], arena: &mut Arena) -> Result<bool>;
    /// `552640(&state+10)` on the region map.
    fn region_clamp(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
    /// Region map +C/+10 (binary32 size).
    fn region_size(&mut self) -> Result<[u32; 2]>;
    /// Organization +20/+24 (binary32).
    fn organization_position(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// SVar table `[B39AB0]` +A0 -> binary32 arrival radius; +74/+78 counts.
    fn explore_radius(&mut self) -> Result<u32>;
    fn candidate_counts(&mut self) -> Result<[u32; 2]>;
    /// `524170(&candidate, best count)` and `5242C0(&candidate, best count)`
    /// on the explore state: they fill the candidate.
    fn candidate_524170(
        &mut self,
        state: StateId,
        best_count: u32,
        candidate: &mut Candidate,
        arena: &mut Arena,
    ) -> Result<()>;
    fn candidate_5242c0(
        &mut self,
        state: StateId,
        best_count: u32,
        candidate: &mut Candidate,
        arena: &mut Arena,
    ) -> Result<()>;
    /// `4F46E0(organization, &candidate, radius)` on the world `[B36534]`.
    fn world_4f46e0(
        &mut self,
        mind: MindId,
        candidate: &Candidate,
        radius: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// `[B279C0]/[B279C4]`.
    fn null_point(&mut self) -> Result<[u32; 2]>;
    /// `520440` (six words) and `51E9B0(x, y)` on the move state.
    fn move_order(&mut self, mind: MindId, args: [u32; 6], arena: &mut Arena) -> Result<()>;
    fn move_retarget(&mut self, mind: MindId, point: [u32; 2], arena: &mut Arena) -> Result<()>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// The inline map bounds: `0 <= x < width` and `0 <= y < height` (ordered).
fn in_map(world: &mut impl ExploreThinkWorld, x: u32, y: u32) -> Result<bool> {
    let size = world.region_size()?;
    Ok(host(x) >= 0.0 && host(x) < host(size[0]) && host(y) >= 0.0 && host(y) < host(size[1]))
}

/// `51F200` on the explore state: the target point is within the arrival
/// radius of the organization.
pub fn arrived(
    state: StateId,
    arena: &Arena,
    world: &mut impl ExploreThinkWorld,
    p: Precision,
) -> Result<bool> {
    let mind = arena.state(state)?.parent;
    let org = world.organization_position(mind)?;
    let target = world.explore_target(state)?;
    let dy = num(n(org[1])?.sub(&n(target[1])?, p))?;
    let dx = num(n(org[0])?.sub(&n(target[0])?, p))?;
    let d2 = num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))?;
    let d = num(d2.sqrt(p))?;
    Ok(d.compare(&n(world.explore_radius()?)?) != Ordering::Greater)
}

/// Whether `candidate` beats the best (count, score).
fn better(candidate: &Candidate, best_count: u32, best_score: u32) -> bool {
    let count = candidate.count as i32;
    if count == best_count as i32 {
        host(candidate.score) > host(best_score)
    } else {
        count > best_count as i32
    }
}

/// The candidate loop count: `n` iterations when `n - 1` is not negative.
fn iterations(n: u32) -> u32 {
    if (n as i32).wrapping_sub(1) >= 0 {
        n
    } else {
        0
    }
}

/// Complete `525D70` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag. A null leader Mind is the native null dereference.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl ExploreThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let group_gtid = world.organization_word164(mind)?;
    let mut following = None;
    if let Some(group) = world.resolve(group_gtid)?
        && !world.group_4dce60(group, mind, arena)?
    {
        following = Some(group);
    }
    let mut best = Candidate {
        x: MINUS_ONE,
        y: MINUS_ONE,
        score: NO_SCORE,
        count: 0,
        word: 0,
    };
    let mut stop_moving = false;
    if let Some(group) = following {
        let leader_gtid = world.group_word194(group)?;
        if let Some(leader) = world.resolve(leader_gtid)? {
            let leader_mind = world
                .object_mind(leader)?
                .ok_or_else(|| Error::World("null leader Mind dereference".into()))?;
            let theirs = world.mind_explore_target(leader_mind)?;
            let ours = world.explore_target(state)?;
            if !(host(theirs[0]) == host(ours[0]) && host(theirs[1]) == host(ours[1])) {
                best = Candidate::from_words(theirs);
                stop_moving = true;
            }
        }
    } else {
        let target = world.explore_target(state)?;
        stop_moving = !world.region_contains([target[0], target[1]], arena)?
            || arrived(state, arena, world, p)?;
    }
    if stop_moving {
        let moving = world.embedded_state(mind, MOVE_OFFSET)?;
        if arena.state(moving)?.status == 2 {
            leave_active(world, mind, MOVE_OFFSET, arena)?;
        }
    }
    let moving = world.embedded_state(mind, MOVE_OFFSET)?;
    if arena.state(moving)?.status == 2 {
        world.embedded_think(mind, MOVE_OFFSET, arena)?;
        return Ok(wrong);
    }
    if following.is_none() {
        let point = world.explore_point(state)?;
        if !world.region_contains(point, arena)? {
            world.region_clamp(state, arena)?;
        }
        let [count_a, count_b] = world.candidate_counts()?;
        for _ in 0..iterations(count_a) {
            let mut candidate = Candidate {
                x: MINUS_ONE,
                y: MINUS_ONE,
                score: NO_SCORE,
                count: 0,
                word: 0,
            };
            world.candidate_524170(state, best.count, &mut candidate, arena)?;
            if in_map(world, candidate.x, candidate.y)?
                && world.world_4f46e0(mind, &candidate, CANDIDATE_RADIUS_BITS, arena)?
                && better(&candidate, best.count, best.score)
            {
                best = candidate;
            }
        }
        for _ in 0..iterations(count_b) {
            let mut candidate = Candidate {
                x: MINUS_ONE,
                y: MINUS_ONE,
                score: NO_SCORE,
                count: 0,
                word: 0,
            };
            world.candidate_5242c0(state, best.count, &mut candidate, arena)?;
            if in_map(world, candidate.x, candidate.y)?
                && better(&candidate, best.count, best.score)
            {
                best = candidate;
            }
        }
    }
    if !in_map(world, best.x, best.y)? {
        let sleep = world.embedded_state(mind, SLEEP_OFFSET)?;
        if arena.state(sleep)?.status != 2 {
            activate(sleep, 0, arena, world)?;
        }
        world.embedded_think(mind, SLEEP_OFFSET, arena)?;
        return Ok(wrong);
    }
    let sleep = world.embedded_state(mind, SLEEP_OFFSET)?;
    if arena.state(sleep)?.status == 2 {
        leave_active(world, mind, SLEEP_OFFSET, arena)?;
    }
    world.set_explore_target(state, best.words())?;
    let group_gtid = world.organization_word164(mind)?;
    if let Some(group) = world.resolve(group_gtid)?
        && world.group_4dce60(group, mind, arena)?
    {
        world.group_4dcf00(group, state, arena)?;
    }
    let target = world.explore_target(state)?;
    let moving = world.embedded_state(mind, MOVE_OFFSET)?;
    if arena.state(moving)?.status != 2 {
        let null = world.null_point()?;
        world.move_order(mind, [0, target[0], target[1], 0, null[0], null[1]], arena)?;
    } else {
        world.move_retarget(mind, [target[0], target[1]], arena)?;
    }
    world.embedded_think(mind, MOVE_OFFSET, arena)?;
    Ok(wrong)
}
