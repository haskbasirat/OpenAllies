//! Original element (unit) target choice, Source/Sim/Tai/UAI.cpp:
//!
//! * `534700(point, radius, out, minimum, filter, preferred)` on the element
//!   Mind ([`choose`]): the best target among the candidates of a spatial
//!   query, the organization's range (`558150(organization+7C)`) for a
//!   member of an organization, otherwise the circle (`5580F0(point,
//!   radius)`), walked in `558750` order.
//! * `52F010(best, candidate, bypass)` ([`rank`]): which of two decision
//!   records is better.
//! * `535910(target)` on the element engage state ([`engage_choice`]): the
//!   engage state's choice around the organization or around the unit.
//!
//! The decision record is the native 7-word record written by `5340C0`
//! (aa_mind `uai_kill::KillDecision` has the same layout).
use crate::Result;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// `[A34770]` (binary32 1e-4): scores this close rank by distance.
pub const SCORE_EPSILON_BITS: u32 = 0x38d1_b717;
/// `[A02790]` (binary32 1e-4): the `455000` near-equal threshold.
pub const NEAR_EQUAL_BITS: u32 = 0x38d1_b717;
/// `535910` asks `534700` for priority 2 or more.
pub const ENGAGE_MINIMUM_PRIORITY: i32 = 2;

/// The native 7-word decision record (`+0` ability, `+4` kind, `+8` status,
/// `+C` score, `+10` target, `+14` squared distance, `+18` target priority).
/// Handles are opaque; 0 means null.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub ability: u32,
    pub kind: u32,
    pub status: u32,
    pub score: u32,
    pub target: u32,
    pub distance2: u32,
    pub priority: i32,
}
impl Decision {
    /// Initial record written by `534700` for its best and its candidate
    /// record: kind 1, status 3, score -1, squared distance FLT_MAX.
    pub fn initial() -> Self {
        Self {
            ability: 0,
            kind: 1,
            status: 3,
            score: 0xbf80_0000,
            target: 0,
            distance2: 0x7f7f_ffff,
            priority: 0,
        }
    }
}

/// The spatial query `534700` walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Query {
    /// `558150(organization+7C)`: the organization's range.
    Organization(u32),
    /// `5580F0(point, radius)`: a circle (binary32 words).
    Circle { point: [u32; 2], radius: u32 },
}

/// Services of `534700`, called in native order. The host is bound to one
/// element Mind (its actor is the Mind +4 actor).
pub trait MemberChoiceWorld {
    /// Actor +74 (performer) is not null.
    fn has_performer(&mut self) -> Result<bool>;
    /// Actor +80 (membership) -> +10 (organization), `None` when either is
    /// null.
    fn organization(&mut self) -> Result<Option<u32>>;
    /// `558720` then the query's begin call.
    fn query_begin(&mut self, query: Query) -> Result<()>;
    /// `558750`: the next candidate, `None` at the end.
    fn query_next(&mut self) -> Result<Option<u32>>;
    /// `557FD0`: the query is released.
    fn query_end(&mut self) -> Result<()>;
    /// Candidate virtual +1C(filter).
    fn candidate_matches(&mut self, candidate: u32, filter: u32) -> Result<bool>;
    /// Mind virtual +A0(candidate).
    fn mind_considers(&mut self, candidate: u32) -> Result<bool>;
    /// `530E00(candidate)` on the Mind (the candidate is remembered as
    /// unreachable).
    fn remembered_unreachable(&mut self, candidate: u32) -> Result<bool>;
    /// Actor virtual +16C(candidate).
    fn actor_accepts(&mut self, candidate: u32) -> Result<bool>;
    /// Mind virtual +1AC(candidate): the target priority.
    fn priority(&mut self, candidate: u32) -> Result<i32>;
    /// `5340C0(candidate, null, &record, 0)` on the Mind; it writes the
    /// record in place (a failed decision leaves earlier words standing).
    fn decide(&mut self, candidate: u32, record: &mut Decision) -> Result<()>;
}

/// True when the binary32 difference `a - b`, taken at precision `p` and
/// made absolute, is below the binary32 `epsilon` (an unordered or infinite
/// difference is not).
fn within(a: u32, b: u32, epsilon: u32, p: Precision) -> Result<bool> {
    let (fa, fb) = (f32::from_bits(a), f32::from_bits(b));
    if !fa.is_finite() || !fb.is_finite() {
        return Ok(false);
    }
    let mut d = Binary::from_f32_bits(a)?.sub(&Binary::from_f32_bits(b)?, p)?;
    if d.is_negative_nonzero() {
        d = d.negated();
    }
    Ok(d.compare(&Binary::from_f32_bits(epsilon)?) == Ordering::Less)
}

/// `455000(a, b)`: `|a - b| < [A02790]` at precision `p`.
pub fn near_equal(a: u32, b: u32, p: Precision) -> Result<bool> {
    within(a, b, NEAR_EQUAL_BITS, p)
}

/// `52F010(best, candidate, bypass)`: positive when `candidate` is better.
///
/// A record without ability loses (both without: 0). Unless `bypass`, a
/// different priority decides (the candidate wins when the best's priority
/// is lower or equal, signed). Scores at least 1e-4 apart (or unordered):
/// the candidate wins unless the best's score is greater. Otherwise squared
/// distances within 1e-4 tie (0); else the candidate wins unless the best's
/// is smaller.
pub fn rank(best: &Decision, candidate: &Decision, bypass: bool, p: Precision) -> Result<i32> {
    if best.ability == 0 {
        return Ok(i32::from(candidate.ability != 0));
    }
    if candidate.ability == 0 {
        return Ok(-1);
    }
    if !bypass && best.priority != candidate.priority {
        return Ok(if best.priority <= candidate.priority { 1 } else { -1 });
    }
    if !within(best.score, candidate.score, SCORE_EPSILON_BITS, p)? {
        let greater = f32::from_bits(best.score) > f32::from_bits(candidate.score);
        return Ok(if greater { -1 } else { 1 });
    }
    if near_equal(best.distance2, candidate.distance2, p)? {
        return Ok(0);
    }
    let smaller = f32::from_bits(best.distance2) < f32::from_bits(candidate.distance2);
    Ok(if smaller { -1 } else { 1 })
}

/// Complete `534700(point, radius, out, minimum, filter, preferred)`.
/// Returns the best record's target (`None` for null); `out` receives the
/// best record (not written when the actor has no performer).
///
/// Per candidate, in query order: its record target is set first; a
/// `filter` must match; Mind +A0 must hold and `530E00` must not; the bypass
/// flag is "actor +16C refuses it, or the best so far was so taken"; a zero
/// priority is skipped; without bypass a priority below the best's (when the
/// best has a target) or below `minimum` (unless it is `preferred`) is
/// skipped. Then `5340C0` fills the (reused) candidate record and a
/// positive [`rank`] makes it the best, remembering its own refusal flag.
#[allow(clippy::too_many_arguments)]
pub fn choose(
    point: [u32; 2],
    radius: u32,
    out: Option<&mut Decision>,
    minimum: i32,
    filter: Option<u32>,
    preferred: Option<u32>,
    world: &mut impl MemberChoiceWorld,
    p: Precision,
) -> Result<Option<u32>> {
    if !world.has_performer()? {
        return Ok(None);
    }
    let mut best = Decision::initial();
    let mut best_refused = false;
    let query = match world.organization()? {
        Some(organization) => Query::Organization(organization),
        None => Query::Circle { point, radius },
    };
    world.query_begin(query)?;
    let mut candidate = Decision::initial();
    while let Some(object) = world.query_next()? {
        candidate.target = object;
        if let Some(f) = filter
            && !world.candidate_matches(object, f)?
        {
            continue;
        }
        if !world.mind_considers(object)? || world.remembered_unreachable(object)? {
            continue;
        }
        let refused = !world.actor_accepts(object)?;
        let bypass = refused || best_refused;
        let priority = world.priority(object)?;
        candidate.priority = priority;
        if priority == 0 {
            continue;
        }
        if !bypass {
            if best.target != 0 && priority < best.priority {
                continue;
            }
            if preferred != Some(object) && priority < minimum {
                continue;
            }
        }
        world.decide(object, &mut candidate)?;
        if rank(&best, &candidate, bypass, p)? > 0 {
            best = candidate;
            best_refused = refused;
        }
    }
    if let Some(o) = out {
        *o = best;
    }
    world.query_end()?;
    Ok((best.target != 0).then_some(best.target))
}

/// Services of `535910` beyond [`MemberChoiceWorld`].
pub trait EngageChoiceWorld: MemberChoiceWorld {
    /// Organization +20/+24 (binary32).
    fn organization_position(&mut self, organization: u32) -> Result<[u32; 2]>;
    /// `577F20` on organization +7C: the binary32 it loads.
    fn organization_radius(&mut self, organization: u32) -> Result<u32>;
    /// The element actor +20/+24 (binary32).
    fn actor_position(&mut self) -> Result<[u32; 2]>;
    /// Object +20/+24 (binary32).
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
}

/// The point and radius `535910(target)` passes to `534700`; `None` when
/// it returns null without a search.
///
/// A member of an organization searches the organization's range around
/// its position (+20/+24) with the `577F20` radius. Otherwise a null target
/// gives null, and the circle around the unit has radius twice the distance
/// to the target (x87 at `p`, stored to binary32).
pub fn engage_arguments(
    target: Option<u32>,
    world: &mut impl EngageChoiceWorld,
    p: Precision,
) -> Result<Option<([u32; 2], u32)>> {
    if let Some(organization) = world.organization()? {
        let point = world.organization_position(organization)?;
        return Ok(Some((point, world.organization_radius(organization)?)));
    }
    let point = world.actor_position()?;
    let Some(t) = target else {
        return Ok(None);
    };
    let [ax, ay] = world.actor_position()?;
    let [tx, ty] = world.object_position(t)?;
    let n = Binary::from_f32_bits;
    let dy = n(ty)?.sub(&n(ay)?, p)?;
    let dx = n(tx)?.sub(&n(ax)?, p)?;
    let sum = dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?;
    let root = sum.sqrt(p)?;
    Ok(Some((point, root.add(&root, p)?.store_f32()?)))
}

/// Complete `535910(target)` on the element engage state (its second
/// argument is unused): [`engage_arguments`], then `534700(point, radius,
/// null, 2, null, target)` ([`choose`]).
pub fn engage_choice(
    target: Option<u32>,
    world: &mut impl EngageChoiceWorld,
    p: Precision,
) -> Result<Option<u32>> {
    let Some((point, radius)) = engage_arguments(target, world, p)? else {
        return Ok(None);
    };
    choose(
        point,
        radius,
        None,
        ENGAGE_MINIMUM_PRIORITY,
        None,
        target,
        world,
        p,
    )
}
