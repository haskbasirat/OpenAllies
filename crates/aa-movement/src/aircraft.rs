//! Original AIRCRAFT component movement (`AirCraft.cpp`, component slot 23 at
//! actor+0xBC, vtable `0xA3AF78`, constructor `0x55E8E0`).
//!
//! The Mover Tick `0x577E50` calls aircraft virtual +0x38 = MoveTick
//! `0x5612F0` (string "KKC_AircraftComponent::MoveTick") when actor flag bit12
//! is set (`crate::mover::TickWorld::aircraft_step`). Ported routines:
//!
//! - `0x5612F0` [`move_tick`]: pending-attack flush, pose build, formation
//!   follow/leave, holding circle, path advance, extrapolation, Mover pose
//!   commit and the taxi scale written to actor+0x3C;
//! - `0x55E2F0` [`advance_path`], `0x55DCA0` [`arc_length`] and `0x55DA10`
//!   [`evaluate_curve`] (cubic segments with the path's 4x4 basis), using the
//!   normalization `0x4453A0` (`crate::falling::normalize`);
//! - `0x560050` [`plan_holding_circle`] with the leg target `0x55FE60`;
//! - `0x561080` [`follow_leader`] with the slot offset `0x5685E0`
//!   [`formation_offset`] and the inline handle getter `0x4CF1B0`;
//! - `0x55EBE0` [`reset_member_paths`], `0x55E1B0` [`clear_path`], `0x5685B0`
//!   [`group_leader`], `0x567FF0` [`group_leader_is`], `0x516B10`
//!   [`following_path`] and the Mover requested-speed clamp `0x576D20`
//!   [`set_mover_speed`].
//!
//! Explicit boundaries ([`AircraftWorld`], [`FormationWorld`]): Mind virtuals,
//! actor speed virtuals, the attack descriptor/ability/combat services
//! `0x4C8300`/`0x4C8F20`/`0x57E030`, diagnostics `0x404F60`, the formation
//! teardown `0x55EDC0` (refcounted group release with virtual deleting
//! destructors and recursion over followers), the follow planner `0x560A70`,
//! the leg builder `0x55F230`, the Mover pose commit `0x576BB0`, the terrain
//! height `0x4F0E50` and group virtual +0x14. The segment free `0x6F7430` is the
//! allocator: popping a segment here drops it.
//!
//! All arithmetic is `aa_numeric::Binary` at an explicit x87 precision with the
//! native binary32 store points. Nonfinite operands are outside that domain
//! and stop with an error, except the two native division-by-zero/overflow
//! shapes that the original resolves to finite state (a zero-length segment's
//! infinite step and the zero altitude band), which are modelled exactly.
//! Nothing is transactional: an error leaves earlier writes and callback
//! effects applied, and callers must not automatically retry.
use crate::falling::normalize;
use crate::mover::{MoverLimits, bin, store};
use crate::{ActorMotion, MoverState};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::VecDeque;

/// actor+0xBC = component array actor+0x60 + 23 * 4.
pub const AIRCRAFT_COMPONENT_SLOT: usize = 23;
/// AircraftComponent primary vtable; +0x38 is MoveTick `0x5612F0`.
pub const AIRCRAFT_VTABLE: u32 = 0x00a3_af78;
/// Bezier basis at `0xB281D8`, copied into every path (path+0x18) by the
/// constructor `0x55E8E0` through `0x55D9A0`. Rows are cubic polynomial
/// coefficients (t^3, t^2, t, 1) of the weights of control points 0..3.
#[rustfmt::skip]
pub const BEZIER_BASIS_BITS: [u32; 16] = [
    0xbf80_0000, 0x4040_0000, 0xc040_0000, 0x3f80_0000, 0x4040_0000, 0xc0c0_0000, 0x4040_0000, 0,
    0xc040_0000, 0x4040_0000, 0, 0, 0x3f80_0000, 0, 0, 0,
];
/// First argument MoveTick passes to (Mind+8) virtual +4 (binary32 2.0).
pub const STOPPED_THINKING_DELAY_BITS: u32 = 0x4000_0000;
/// `0x404F60` source line argument of the "flying in circles" diagnostic.
pub const STOPPED_THINKING_LINE: u32 = 0x56b;
/// Flags MoveTick passes to the terrain height query `0x4F0E50`.
pub const GROUND_HEIGHT_FLAGS: u32 = 0x123;

const ZERO: u32 = 0;
const ONE: u32 = 0x3f80_0000;
const HALF: u32 = 0x3f00_0000;
const THIRD: u32 = 0x3eaa_aaab; // 0xA3AF2C
const TENTH: u32 = 0x3dcc_cccd; // 0x9FE0C0
const HUNDREDTH: u32 = 0x3c23_d70a; // 0xA03078
const INDEFINITE: u32 = 0xffc0_0000;
const POSITIVE_INFINITY: u32 = 0x7f80_0000;
const NEGATIVE_INFINITY: u32 = 0xff80_0000;
/// `0xB2814C`..`0xB28154` vector returned by `0x5685E0` when no slot matches
/// (all-zero in the image; the oracle reads the image value).
const ZERO_VECTOR: [f32; 3] = [0.0; 3];

#[derive(Debug, thiserror::Error)]
pub enum AircraftError {
    #[error(transparent)]
    Movement(#[from] crate::Error),
    #[error("native null dereference: {0}")]
    NullDereference(&'static str),
    #[error("nonfinite value outside the supported arithmetic domain: {0}")]
    NonFinite(&'static str),
    #[error("arc-length subdivision repeats its own interval (native recursion never ends)")]
    UnboundedRecursion,
    #[error("formation slot {0} is beyond the layout offsets (native out-of-bounds read)")]
    SlotOutOfBounds(usize),
    #[error("aircraft world boundary failed: {0}")]
    World(String),
}
impl From<aa_numeric::Error> for AircraftError {
    fn from(error: aa_numeric::Error) -> Self {
        Self::Movement(crate::Error::Numeric(error))
    }
}
pub type AircraftResult<T> = std::result::Result<T, AircraftError>;

/// Identity of a native actor object (the pointer the original compares).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorKey(pub u32);
/// Identity of a native formation group object (aircraft+0xA8/+0xAC/+0xB0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupKey(pub u32);
/// Identity of the pending ability object at aircraft+0xA4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilityKey(pub u32);

/// One entry of the generation-checked object registry at `[0xB364C0]`:
/// generation u16 at +4+index*2, object pointer at +0x20004+index*4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleSlot {
    pub generation: u16,
    pub actor: Option<ActorKey>,
}

/// One 0x5C-byte path segment (vtable `0xA3AF40`, built by `0x55E6E0`).
/// Segment+4 points at the owning path's basis (path+0x18); it is therefore
/// evaluated with [`AircraftPath::basis`]. +0x58 is the list link.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PathSegment {
    /// +0x08, +0x14, +0x20, +0x2C control points (+0x2C is the end point).
    pub control: [[f32; 3]; 4],
    /// +0x38 arc length (`0x55DCA0(0, 1)` at construction).
    pub length: f32,
    /// +0x3C (`0x55DEF0(0, 1)` at construction); not read by the step.
    pub field_3c: f32,
    /// +0x40 direction at the segment end.
    pub end_direction: [f32; 3],
    /// +0x4C speed at t = 1.
    pub end_speed: f32,
    /// +0x50 speed at t = 0.
    pub start_speed: f32,
    /// +0x54 arc-length subdivision tolerance.
    pub tolerance: f32,
}

/// Path sub-object at aircraft+0x28 (vtable `0xA3AF48`). Offsets in brackets
/// are relative to the aircraft component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AircraftPath {
    /// path+4 [+0x2C]: definition speed at construction; read by `0x55F230`.
    pub planning_speed: f32,
    /// path+8 [+0x30]: definition turn radius (+0x234); holding-circle radius.
    pub turn_radius: f32,
    /// path+0xC/+0x10 [+0x34/+0x38]: segment list head and tail.
    pub segments: VecDeque<PathSegment>,
    /// path+0x14 [+0x3C]: parameter within the head segment.
    pub t: f32,
    /// path+0x18..+0x54 [+0x40..+0x7C]: 4x4 polynomial basis.
    pub basis: [f32; 16],
    /// path+0x58 [+0x80]: target altitude of planned legs.
    pub altitude: f32,
    /// path+0x5C [+0x84]: leg speed used by the planners.
    pub leg_speed: f32,
    /// path+0x60 [+0x88]: accumulated planned length.
    pub total_length: f32,
    /// path+0x64 [+0x8C]: speed interpolated by `0x55E2F0`.
    pub speed: f32,
}

/// AircraftComponent fields used by the movement step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AircraftState {
    /// +0x10 byte: taxiing on the ground (the constructor stores 1).
    pub on_ground: u8,
    /// +0x14: cruise speed; `0x55EDC0` stores definition speed (+0x228).
    pub cruise_speed: f32,
    /// +0x28 path sub-object.
    pub path: AircraftPath,
    /// +0x90 byte: the current path is a MoveTick holding circle.
    pub circling: u8,
    /// +0x94: pending attack target handle.
    pub attack_target: u32,
    /// +0x98..+0xA0: pending attack point (used when the handle is stale).
    pub attack_point: [f32; 3],
    /// +0xA4: pending ability object.
    pub pending_ability: Option<AbilityKey>,
    /// +0xA8: formation group this aircraft belongs to.
    pub group_a: Option<GroupKey>,
    /// +0xAC: group whose first member is the leader to follow.
    pub group_b: Option<GroupKey>,
    /// +0xB0: follower group.
    pub group_c: Option<GroupKey>,
}

/// ActorDefinition (actor+4) AircraftComponent values (loader `0x464BD0`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AircraftDefinition {
    /// +0x408 `taxi_scale`.
    pub taxi_scale: f32,
    /// +0x40C `taxi_height`.
    pub taxi_height: f32,
    /// +0x414 `operational_height`.
    pub operational_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AircraftActor {
    pub key: ActorKey,
    /// actor+0x14 generation-bearing handle.
    pub handle: u32,
    /// actor+0x3C, written by MoveTick (taxi scale).
    pub scale: f32,
    pub definition: AircraftDefinition,
}

/// Live snapshots MoveTick reads and writes; callbacks receive it mutably.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AircraftUnit {
    pub actor: AircraftActor,
    /// actor+0x20 position, +0x2C facing, +8 flags, +0x70 Mind presence.
    pub motion: ActorMotion,
    /// Mover (actor+0x6C) +0x24 requested speed and +0x50 speed factor.
    pub mover: MoverState,
    /// Mover +0x28 maximum speed.
    pub mover_limits: MoverLimits,
    pub aircraft: AircraftState,
}

/// The 7-float pose MoveTick builds on its stack (position, direction, speed).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub speed: f32,
}

/// Group+8 layout definition: +0x10 slot offsets (12 bytes each), +0x14 word
/// that must be nonzero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormationDefinition {
    pub slot_word: u32,
    pub offsets: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupLayout {
    /// Group+8 (null when absent).
    pub definition: Option<FormationDefinition>,
    /// Group+0x14..+0x1C anchor subtracted by relative slot offsets.
    pub anchor: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AttackTarget {
    /// Descriptor +0x1C: the resolved target actor.
    Actor(ActorKey),
    /// Descriptor +0x20..+0x28: aircraft+0x98 point (stale or null handle).
    Point([f32; 3]),
}

/// Fields MoveTick writes into the descriptor built by `0x4C8300`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AttackDescriptor {
    /// `0x4C8300` argument: ((ability def +0x54 >> 3) & 1) << 1 | 1.
    pub flags: u32,
    /// Descriptor +8: the aircraft's actor.
    pub source: ActorKey,
    pub target: AttackTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveTickOutcome {
    /// Mind virtual +0x1B0 returned true; nothing else ran.
    Suspended,
    Moved,
}

/// Read-only formation data and the one formation virtual.
pub trait FormationWorld {
    /// Registry slot `index` of `[0xB364C0]`.
    fn handle_slot(&mut self, index: u16) -> AircraftResult<HandleSlot>;
    /// actor+0x14 of another actor.
    fn actor_handle(&mut self, actor: ActorKey) -> AircraftResult<u32>;
    /// actor+8 of another actor.
    fn actor_flags(&mut self, actor: ActorKey) -> AircraftResult<u32>;
    /// Group+0xC member handle list, head first (nodes {handle, next}).
    fn group_members(&mut self, group: GroupKey) -> AircraftResult<Vec<u32>>;
    /// Group+8 definition and +0x14 anchor.
    fn group_layout(&mut self, group: GroupKey) -> AircraftResult<GroupLayout>;
    /// AIRCRAFT component (actor+0xBC) of another actor; absent is a native
    /// null dereference and must be an error.
    fn member_aircraft(&mut self, actor: ActorKey) -> AircraftResult<&mut AircraftState>;
}

/// MoveTick boundaries, in native call order.
pub trait AircraftWorld: FormationWorld {
    /// Mind virtual +0x1B0 (no Mind null check natively).
    fn mind_movement_blocked(&mut self, unit: &mut AircraftUnit) -> AircraftResult<bool>;
    /// `[[aircraft+0xA4]+4]+0x54`.
    fn ability_definition_word(&mut self, ability: AbilityKey) -> AircraftResult<u32>;
    /// `0x4C8300(flags)` on the stack descriptor.
    fn attack_descriptor(&mut self, flags: u32, unit: &mut AircraftUnit) -> AircraftResult<()>;
    /// `0x4C8F20(&descriptor, 1)` on the ability; returns EAX.
    fn ability_check(
        &mut self,
        ability: AbilityKey,
        descriptor: &AttackDescriptor,
        unit: &mut AircraftUnit,
    ) -> AircraftResult<u32>;
    /// `0x57E030(ability, &descriptor)` on actor+0x74; the ability is reread
    /// from aircraft+0xA4 (null is passed through).
    fn fire_ability(
        &mut self,
        ability: Option<AbilityKey>,
        descriptor: &AttackDescriptor,
        unit: &mut AircraftUnit,
    ) -> AircraftResult<()>;
    /// Actor virtual +0x18 (current speed, x87 return).
    fn actor_speed(&mut self, unit: &mut AircraftUnit) -> AircraftResult<Binary>;
    /// Actor virtual +0x104 (formation speed, x87 return).
    fn formation_speed(&mut self, unit: &mut AircraftUnit) -> AircraftResult<Binary>;
    /// Group virtual +0x14.
    fn group_disbanded(&mut self, group: GroupKey, unit: &mut AircraftUnit)
    -> AircraftResult<bool>;
    /// `0x55EDC0` formation teardown on this aircraft.
    fn leave_formation(&mut self, unit: &mut AircraftUnit) -> AircraftResult<()>;
    /// `0x560A70(pose, leader path, offset, delta)` on this aircraft's path.
    fn plan_follow_path(
        &mut self,
        unit: &mut AircraftUnit,
        pose: &Pose,
        leader: ActorKey,
        offset: [f32; 3],
        delta: f32,
    ) -> AircraftResult<()>;
    /// `0x55F230(from, &out, target, &side, flag, 0)` on this aircraft's path;
    /// returns the 7-float pose it writes.
    fn build_leg(
        &mut self,
        unit: &mut AircraftUnit,
        from: &Pose,
        target: [f32; 3],
        side: [f32; 3],
        flag: u32,
    ) -> AircraftResult<Pose>;
    /// Mind+0xC (scheduled entry) is nonnull.
    fn mind_scheduled(&mut self, unit: &mut AircraftUnit) -> AircraftResult<bool>;
    /// `0x404F60` "aircraft %d flying in circles and stopped thinking".
    fn report_stopped_thinking(&mut self, line: u32, actor_handle: u32) -> AircraftResult<()>;
    /// (Mind+8) virtual +4 with (delay bits, argument).
    fn mind_reschedule(
        &mut self,
        delay_bits: u32,
        argument: u32,
        unit: &mut AircraftUnit,
    ) -> AircraftResult<()>;
    /// `0x576BB0` Mover pose commit (position, facing, speed).
    fn commit_pose(
        &mut self,
        position: [f32; 3],
        facing: [f32; 3],
        speed: f32,
        unit: &mut AircraftUnit,
    ) -> AircraftResult<()>;
    /// `0x4F0E50(&xy, flags)` on GWorld; the extended result.
    fn ground_height(
        &mut self,
        xy: [f32; 2],
        flags: u32,
        unit: &mut AircraftUnit,
    ) -> AircraftResult<Binary>;
    /// Mind virtual +0x144.
    fn mind_path_exhausted(&mut self, unit: &mut AircraftUnit) -> AircraftResult<()>;
}

fn b(value: f32) -> AircraftResult<Binary> {
    Ok(bin(value)?)
}
fn st(value: &Binary) -> AircraftResult<f32> {
    Ok(store(value)?)
}
fn k(bits: u32) -> AircraftResult<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn norm(vector: &mut [f32; 3], pc: Precision) -> AircraftResult<Binary> {
    Ok(normalize(vector, pc)?)
}
/// `fld; fchs; fstp` of a finite binary32 (exact negation, signed zeros).
fn negate(value: f32) -> AircraftResult<f32> {
    st(&b(value)?.negated())
}
fn sub3(p: [f32; 3], q: [f32; 3], pc: Precision) -> AircraftResult<[Binary; 3]> {
    Ok([
        b(p[0])?.sub(&b(q[0])?, pc)?,
        b(p[1])?.sub(&b(q[1])?, pc)?,
        b(p[2])?.sub(&b(q[2])?, pc)?,
    ])
}
/// `sqrt((z*z + y*y) + x*x)` in the native register order.
fn length_zyx(v: &[Binary; 3], pc: Precision) -> AircraftResult<Binary> {
    Ok(v[2]
        .mul(&v[2], pc)?
        .add(&v[1].mul(&v[1], pc)?, pc)?
        .add(&v[0].mul(&v[0], pc)?, pc)?
        .sqrt(pc)?)
}
fn sign_bit(value: &Binary) -> AircraftResult<bool> {
    Ok(value.store_f32()? >> 31 != 0)
}

/// `0x55DA10`: `sum_k poly_k(t) * P_k` with poly_k = ((t*M[4k] + M[4k+1])*t +
/// M[4k+2])*t + M[4k+3]. The native register allocation keeps the X sum
/// extended through term 2, Y through term 3 and Z only from term 2; the
/// other partials are stored to binary32 exactly as reproduced here.
pub fn evaluate_curve(
    basis: &[f32; 16],
    control: &[[f32; 3]; 4],
    t: f32,
    pc: Precision,
) -> AircraftResult<[f32; 3]> {
    let t = b(t)?;
    let poly = |row: usize| -> AircraftResult<Binary> {
        Ok(t.mul(&b(basis[4 * row])?, pc)?
            .add(&b(basis[4 * row + 1])?, pc)?
            .mul(&t, pc)?
            .add(&b(basis[4 * row + 2])?, pc)?
            .mul(&t, pc)?
            .add(&b(basis[4 * row + 3])?, pc)?)
    };
    let [p0, p1, p2, p3] = control;
    let w0 = poly(0)?;
    let a = [
        st(&w0.mul(&b(p0[0])?, pc)?)?,
        st(&w0.mul(&b(p0[1])?, pc)?)?,
        st(&w0.mul(&b(p0[2])?, pc)?)?,
    ];
    let w1 = poly(1)?;
    let x1 = w1.mul(&b(p1[0])?, pc)?.add(&b(a[0])?, pc)?;
    let y1 = b(st(&w1.mul(&b(p1[1])?, pc)?)?)?.add(&b(a[1])?, pc)?;
    let z1 = st(&b(st(&w1.mul(&b(p1[2])?, pc)?)?)?.add(&b(a[2])?, pc)?)?;
    let w2 = poly(2)?;
    let x2 = st(&x1.add(&w2.mul(&b(p2[0])?, pc)?, pc)?)?;
    let y2 = y1.add(&b(st(&w2.mul(&b(p2[1])?, pc)?)?)?, pc)?;
    let z2 = b(z1)?.add(&b(st(&w2.mul(&b(p2[2])?, pc)?)?)?, pc)?;
    let w3 = poly(3)?;
    Ok([
        st(&b(x2)?.add(&w3.mul(&b(p3[0])?, pc)?, pc)?)?,
        st(&y2.add(&b(st(&w3.mul(&b(p3[1])?, pc)?)?)?, pc)?)?,
        st(&z2.add(&b(st(&w3.mul(&b(p3[2])?, pc)?)?)?, pc)?)?,
    ])
}

/// `0x55DCA0` (thiscall segment, args t0, t1): adaptive arc length. Four
/// samples at t0, t0+h, t0+2h, t1 (h = (t1-t0)/3, stored); when the polyline
/// exceeds the chord by more than +0x54 it recurses on both halves and returns
/// `L(mid, t1) + binary32(L(t0, mid))`, else `(polyline + chord) / 2`. The
/// result is extended (st0). A subdivision whose midpoint equals an endpoint
/// would recurse on its own interval forever natively; that is an error.
pub fn arc_length(
    basis: &[f32; 16],
    segment: &PathSegment,
    t0: f32,
    t1: f32,
    pc: Precision,
) -> AircraftResult<Binary> {
    let half = k(HALF)?;
    let h = st(&b(t1)?.sub(&b(t0)?, pc)?.mul(&k(THIRD)?, pc)?)?;
    let a = evaluate_curve(basis, &segment.control, t0, pc)?;
    let b1 = evaluate_curve(basis, &segment.control, st(&b(h)?.add(&b(t0)?, pc)?)?, pc)?;
    let c = evaluate_curve(
        basis,
        &segment.control,
        st(&b(h)?.add(&b(h)?, pc)?.add(&b(t0)?, pc)?)?,
        pc,
    )?;
    let d = evaluate_curve(basis, &segment.control, t1, pc)?;
    let chord = st(&length_zyx(&sub3(d, a, pc)?, pc)?)?;
    let ab = sub3(b1, a, pc)?;
    let ab = [b(st(&ab[0])?)?, b(st(&ab[1])?)?, b(st(&ab[2])?)?];
    let bc = sub3(c, b1, pc)?;
    let cd = sub3(d, c, pc)?;
    let cd_length = st(&length_zyx(&cd, pc)?)?;
    let polyline = b(cd_length)?
        .add(&length_zyx(&bc, pc)?, pc)?
        .add(&length_zyx(&ab, pc)?, pc)?;
    let excess = polyline.sub(&b(chord)?, pc)?;
    if excess.compare(&b(segment.tolerance)?) == Ordering::Greater {
        let mid = st(&b(t0)?.add(&b(t1)?, pc)?.mul(&half, pc)?)?;
        let mid_value = b(mid)?;
        if mid_value.compare(&b(t0)?) == Ordering::Equal
            || mid_value.compare(&b(t1)?) == Ordering::Equal
        {
            return Err(AircraftError::UnboundedRecursion);
        }
        let first = st(&arc_length(basis, segment, t0, mid, pc)?)?;
        let second = arc_length(basis, segment, mid, t1, pc)?;
        return Ok(second.add(&b(first)?, pc)?);
    }
    Ok(polyline.add(&b(chord)?, pc)?.mul(&half, pc)?)
}

/// Binary32 step `dist / length` as stored by `0x55E2F0`: x87 masked
/// division by zero and binary32 overflow give infinities.
#[derive(Debug, Clone, Copy)]
enum Step {
    Finite(f32),
    Infinite { negative: bool },
}
impl Step {
    fn finite(self) -> AircraftResult<f32> {
        match self {
            Step::Finite(value) => Ok(value),
            Step::Infinite { .. } => Err(AircraftError::NonFinite("refined segment step")),
        }
    }
}
fn stored_quotient(
    numerator: &Binary,
    denominator: &Binary,
    pc: Precision,
) -> AircraftResult<Step> {
    if denominator.is_zero() {
        if numerator.is_zero() {
            return Err(AircraftError::NonFinite("0/0 segment step"));
        }
        return Ok(Step::Infinite {
            negative: numerator.is_negative_nonzero() != sign_bit(denominator)?,
        });
    }
    let quotient = numerator.div(denominator, pc)?;
    match quotient.store_f32() {
        Ok(bits) => Ok(Step::Finite(f32::from_bits(bits))),
        Err(aa_numeric::Error::Binary32Overflow) => Ok(Step::Infinite {
            negative: quotient.is_negative_nonzero(),
        }),
        Err(error) => Err(error.into()),
    }
}

/// `0x55E2F0` (thiscall path, args delta, &pose_in, &pose_out): consume
/// `delta` seconds of flight along the segment list. Returns the binary32
/// time left over (st0) and the output pose. The input pose is copied first;
/// the output speed changes only when a segment end is reached.
pub fn advance_path(
    path: &mut AircraftPath,
    delta: f32,
    pose: &Pose,
    pc: Precision,
) -> AircraftResult<(f32, Pose)> {
    let zero = k(ZERO)?;
    let one = k(ONE)?;
    let half = k(HALF)?;
    let tenth = k(TENTH)?;
    let mut local = *pose;
    let mut remaining = delta;
    while let Some(&segment) = path.segments.front() {
        if b(remaining)?.compare(&zero) != Ordering::Greater {
            break;
        }
        let t = b(path.t)?;
        let speed = one
            .sub(&t, pc)?
            .mul(&b(segment.start_speed)?, pc)?
            .add(&t.mul(&b(segment.end_speed)?, pc)?, pc)?;
        path.speed = st(&speed)?;
        if speed.compare(&tenth) == Ordering::Less {
            path.speed = f32::from_bits(TENTH);
        }
        let distance_ext = b(remaining)?.mul(&b(path.speed)?, pc)?;
        let distance = st(&distance_ext)?;
        let length = b(segment.length)?;
        let mut dt = stored_quotient(&distance_ext, &length, pc)?;
        let tolerance = st(&b(distance)?.mul(&tenth, pc)?)?;
        let mut step = dt;
        if length.compare(&k(HUNDREDTH)?) != Ordering::Less {
            let negative_tolerance = b(negate(tolerance)?)?;
            for _ in 0..4 {
                let current = dt.finite()?;
                let end = st(&b(current)?.add(&b(path.t)?, pc)?)?;
                let error =
                    arc_length(&path.basis, &segment, path.t, end, pc)?.sub(&b(distance)?, pc)?;
                if error.compare(&negative_tolerance) == Ordering::Less {
                    let halved = b(step.finite()?)?.mul(&half, pc)?;
                    step = Step::Finite(st(&halved)?);
                    dt = Step::Finite(st(&halved.add(&b(current)?, pc)?)?);
                } else if error.compare(&b(tolerance)?) != Ordering::Greater {
                    break;
                } else {
                    step = Step::Finite(st(&b(step.finite()?)?.mul(&half, pc)?)?);
                    dt = Step::Finite(st(&b(current)?.sub(&b(step.finite()?)?, pc)?)?);
                }
            }
        }
        // `fld dt; fadd t; fcomp 1.0; test ah, 1`: below 1 (or unordered)
        // stays inside the segment.
        let inside = match dt {
            Step::Infinite { negative: false } => None,
            Step::Infinite { negative: true } => {
                return Err(AircraftError::NonFinite(
                    "negative infinite segment parameter",
                ));
            }
            Step::Finite(step_time) => {
                let next = b(step_time)?.add(&b(path.t)?, pc)?;
                (next.compare(&one) == Ordering::Less).then_some((next, step_time))
            }
        };
        match inside {
            None => {
                let rest = arc_length(&path.basis, &segment, path.t, 1.0, pc)?;
                let time = rest.div(&b(path.speed)?, pc)?;
                local.position = segment.control[3];
                local.direction = segment.end_direction;
                local.speed = segment.end_speed;
                path.t = 0.0;
                remaining = st(&b(remaining)?.sub(&time, pc)?)?;
                path.segments.pop_front();
            }
            Some((next, step_time)) => {
                let next = st(&next)?;
                path.t = next;
                remaining = 0.0;
                local.position = evaluate_curve(&path.basis, &segment.control, next, pc)?;
                let ahead_t = b(next)?.add(&b(step_time)?, pc)?;
                if ahead_t.compare(&one) == Ordering::Greater {
                    local.direction = segment.end_direction;
                } else {
                    let ahead = evaluate_curve(&path.basis, &segment.control, st(&ahead_t)?, pc)?;
                    let toward = sub3(ahead, local.position, pc)?;
                    let mut toward = [st(&toward[0])?, st(&toward[1])?, st(&toward[2])?];
                    norm(&mut toward, pc)?;
                    let mut blended = [
                        st(&b(toward[0])?.add(&b(local.direction[0])?, pc)?)?,
                        st(&b(local.direction[1])?.add(&b(toward[1])?, pc)?)?,
                        st(&b(local.direction[2])?.add(&b(toward[2])?, pc)?)?,
                    ];
                    let length = norm(&mut blended, pc)?;
                    local.direction = if length.compare(&zero) == Ordering::Equal {
                        toward
                    } else {
                        blended
                    };
                }
            }
        }
    }
    Ok((remaining, local))
}

/// `0x55E1B0` (thiscall path): free every segment, then t = 0 and
/// total length = 0.
pub fn clear_path(path: &mut AircraftPath) {
    path.segments.clear();
    path.t = 0.0;
    path.total_length = 0.0;
}

/// `0x516B10`: not circling and the path is nonempty.
pub fn following_path(aircraft: &AircraftState) -> bool {
    aircraft.circling == 0 && !aircraft.path.segments.is_empty()
}

/// `0x576D20` (thiscall Mover, args speed, factor): +0x24 = speed, replaced
/// by +0x28 when the maximum is ordered below it; +0x50 = factor.
pub fn set_mover_speed(
    mover: &mut MoverState,
    limits: &MoverLimits,
    speed: f32,
    factor: f32,
) -> AircraftResult<()> {
    mover.requested_speed = speed;
    if b(limits.max_speed)?.compare(&b(speed)?) == Ordering::Less {
        mover.requested_speed = limits.max_speed;
    }
    mover.speed_factor = factor;
    Ok(())
}

/// Inline registry lookup: the slot object when the handle's generation
/// (high 16 bits) matches and the slot is nonnull.
pub fn resolve_handle<W: FormationWorld + ?Sized>(
    world: &mut W,
    handle: u32,
) -> AircraftResult<Option<ActorKey>> {
    let slot = world.handle_slot((handle & 0xffff) as u16)?;
    Ok(if (handle >> 16) as u16 == slot.generation {
        slot.actor
    } else {
        None
    })
}

/// `0x5685B0` (thiscall group): the first member's object when its handle
/// generation matches (the slot object may be null).
pub fn group_leader<W: FormationWorld + ?Sized>(
    world: &mut W,
    group: GroupKey,
) -> AircraftResult<Option<ActorKey>> {
    match world.group_members(group)?.first() {
        Some(&handle) => resolve_handle(world, handle),
        None => Ok(None),
    }
}

/// `0x567FF0` (thiscall group, arg handle): the first member handle resolves
/// to a live object and equals `handle`.
pub fn group_leader_is<W: FormationWorld + ?Sized>(
    world: &mut W,
    group: GroupKey,
    handle: u32,
) -> AircraftResult<bool> {
    let Some(&first) = world.group_members(group)?.first() else {
        return Ok(false);
    };
    Ok(resolve_handle(world, first)?.is_some() && first == handle)
}

/// `0x5685E0` (thiscall group, args out, handle, relative byte): the slot
/// offset of the member whose handle equals `handle` (exact match, no
/// generation check), minus the group anchor when `relative`; the zero vector
/// when the group has no layout, its +0x14 word is zero or no member matches.
pub fn formation_offset<W: FormationWorld + ?Sized>(
    world: &mut W,
    group: GroupKey,
    handle: u32,
    relative: bool,
    pc: Precision,
) -> AircraftResult<[f32; 3]> {
    let layout = world.group_layout(group)?;
    let Some(definition) = layout.definition.as_ref() else {
        return Ok(ZERO_VECTOR);
    };
    if definition.slot_word == 0 {
        return Ok(ZERO_VECTOR);
    }
    let Some(index) = world
        .group_members(group)?
        .iter()
        .position(|&member| member == handle)
    else {
        return Ok(ZERO_VECTOR);
    };
    let offset = *definition
        .offsets
        .get(index)
        .ok_or(AircraftError::SlotOutOfBounds(index))?;
    if !relative {
        return Ok(offset);
    }
    let difference = sub3(offset, layout.anchor, pc)?;
    Ok([
        st(&difference[0])?,
        st(&difference[1])?,
        st(&difference[2])?,
    ])
}

fn actor_flags<W: FormationWorld + ?Sized>(
    unit: &AircraftUnit,
    world: &mut W,
    actor: ActorKey,
) -> AircraftResult<u32> {
    if actor == unit.actor.key {
        Ok(unit.motion.flags)
    } else {
        world.actor_flags(actor)
    }
}

/// `0x55EBE0` (thiscall aircraft): when this aircraft has both +0xA8 and
/// +0xAC groups and leads +0xAC, clear the paths of every live (flag bit0
/// clear) airborne member of +0xA8 other than itself, then of every live
/// airborne member of +0xB0 (natively without a self check).
pub fn reset_member_paths<W: FormationWorld + ?Sized>(
    unit: &mut AircraftUnit,
    world: &mut W,
) -> AircraftResult<()> {
    let (Some(group_a), Some(group_b)) = (unit.aircraft.group_a, unit.aircraft.group_b) else {
        return Ok(());
    };
    if !group_leader_is(world, group_b, unit.actor.handle)? {
        return Ok(());
    }
    for handle in world.group_members(group_a)? {
        let Some(member) = resolve_handle(world, handle)? else {
            continue;
        };
        if actor_flags(unit, world, member)? & 1 != 0 {
            continue;
        }
        let own = member == unit.actor.key;
        let on_ground = if own {
            unit.aircraft.on_ground
        } else {
            world.member_aircraft(member)?.on_ground
        };
        if on_ground == 0 && !own {
            clear_path(&mut world.member_aircraft(member)?.path);
        }
    }
    let Some(group_c) = unit.aircraft.group_c else {
        return Ok(());
    };
    for handle in world.group_members(group_c)? {
        let Some(member) = resolve_handle(world, handle)? else {
            continue;
        };
        if actor_flags(unit, world, member)? & 1 != 0 {
            continue;
        }
        let aircraft = if member == unit.actor.key {
            &mut unit.aircraft
        } else {
            world.member_aircraft(member)?
        };
        if aircraft.on_ground == 0 {
            clear_path(&mut aircraft.path);
        }
    }
    Ok(())
}

/// `0x561080` (thiscall aircraft, args &pose, delta): follow the leader of
/// +0xAC. Returns whether the follow planner `0x560A70` ran.
pub fn follow_leader<W: AircraftWorld + ?Sized>(
    unit: &mut AircraftUnit,
    pose: &Pose,
    delta: f32,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<bool> {
    let group = unit.aircraft.group_b.ok_or(AircraftError::NullDereference(
        "aircraft+0xAC group virtual +0x14",
    ))?;
    if world.group_disbanded(group, unit)? {
        return Ok(false);
    }
    let group_b = unit.aircraft.group_b.ok_or(AircraftError::NullDereference(
        "aircraft+0xAC group (0x5685B0)",
    ))?;
    let leader_handle = match group_leader(world, group_b)? {
        None => 0,
        Some(actor) if actor == unit.actor.key => unit.actor.handle,
        Some(actor) => world.actor_handle(actor)?,
    };
    let Some(leader) = resolve_handle(world, leader_handle)? else {
        return Ok(false);
    };
    let leader_idle = if leader == unit.actor.key {
        unit.aircraft.path.segments.is_empty()
    } else {
        world.member_aircraft(leader)?.path.segments.is_empty()
    };
    if leader_idle {
        return Ok(false);
    }
    let shared = unit.aircraft.group_b == unit.aircraft.group_a;
    let group_b = unit.aircraft.group_b.ok_or(AircraftError::NullDereference(
        "aircraft+0xAC group (0x5685E0)",
    ))?;
    let leader_offset = formation_offset(world, group_b, leader_handle, true, pc)?;
    let negated = [
        negate(leader_offset[0])?,
        negate(leader_offset[1])?,
        negate(leader_offset[2])?,
    ];
    let group_a = unit.aircraft.group_a.ok_or(AircraftError::NullDereference(
        "aircraft+0xA8 group (0x5685E0)",
    ))?;
    let own = formation_offset(world, group_a, unit.actor.handle, shared, pc)?;
    let offset = [
        st(&b(negated[0])?.add(&b(own[0])?, pc)?)?,
        st(&b(negated[1])?.add(&b(own[1])?, pc)?)?,
        st(&b(negated[2])?.add(&b(own[2])?, pc)?)?,
    ];
    world.plan_follow_path(unit, pose, leader, offset, delta)?;
    reset_member_paths(unit, world)?;
    Ok(true)
}

/// `0x55FE60` (thiscall path, args from, out, side x/y/z, radius, flag): leg
/// target `((side + dir) * radius + position)` with the X product stored
/// first and Z = path+0x58, then `0x55F230(from, out, target, &side, flag, 0)`.
fn plan_leg<W: AircraftWorld + ?Sized>(
    unit: &mut AircraftUnit,
    from: &Pose,
    side: [f32; 3],
    radius: f32,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<Pose> {
    let radius = b(radius)?;
    let x = st(&b(side[0])?
        .add(&b(from.direction[0])?, pc)?
        .mul(&radius, pc)?)?;
    let target = [
        st(&b(x)?.add(&b(from.position[0])?, pc)?)?,
        st(&b(side[1])?
            .add(&b(from.direction[1])?, pc)?
            .mul(&radius, pc)?
            .add(&b(from.position[1])?, pc)?)?,
        unit.aircraft.path.altitude,
    ];
    world.build_leg(unit, from, target, side, 0)
}

/// `0x560050` (thiscall path, arg &pose): four legs around a holding circle.
/// The first side vector is normalize(dir.y, -dir.x, 0); later legs use the
/// previous leg's direction unnormalized with that first Z. The radius
/// (path+8) is reread before every leg.
pub fn plan_holding_circle<W: AircraftWorld + ?Sized>(
    unit: &mut AircraftUnit,
    pose: &Pose,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<()> {
    let mut side = [pose.direction[1], negate(pose.direction[0])?, 0.0];
    norm(&mut side, pc)?;
    let side_z = side[2];
    let radius = unit.aircraft.path.turn_radius;
    let mut leg = plan_leg(unit, pose, side, radius, pc, world)?;
    for _ in 0..3 {
        let from = leg;
        let side = [from.direction[1], negate(from.direction[0])?, side_z];
        let radius = unit.aircraft.path.turn_radius;
        leg = plan_leg(unit, &from, side, radius, pc, world)?;
    }
    Ok(())
}

/// Stored binary32 bits of `numerator / band` with x87 masked exceptions.
fn stored_fraction(numerator: &Binary, band: &Binary, pc: Precision) -> AircraftResult<u32> {
    if band.is_zero() {
        if numerator.is_zero() {
            return Ok(INDEFINITE);
        }
        let negative = numerator.is_negative_nonzero() != sign_bit(band)?;
        return Ok(if negative {
            NEGATIVE_INFINITY
        } else {
            POSITIVE_INFINITY
        });
    }
    let quotient = numerator.div(band, pc)?;
    match quotient.store_f32() {
        Ok(bits) => Ok(bits),
        Err(aa_numeric::Error::Binary32Overflow) => Ok(if quotient.is_negative_nonzero() {
            NEGATIVE_INFINITY
        } else {
            POSITIVE_INFINITY
        }),
        Err(error) => Err(error.into()),
    }
}

/// Complete MoveTick `0x5612F0` (thiscall aircraft, arg delta).
pub fn move_tick<W: AircraftWorld + ?Sized>(
    unit: &mut AircraftUnit,
    delta: f32,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<MoveTickOutcome> {
    if !unit.motion.has_mind {
        return Err(AircraftError::NullDereference(
            "actor+0x70 Mind virtual +0x1B0",
        ));
    }
    if world.mind_movement_blocked(unit)? {
        return Ok(MoveTickOutcome::Suspended);
    }
    if let Some(ability) = unit.aircraft.pending_ability {
        let word = world.ability_definition_word(ability)?;
        let flags = (((word >> 3) & 1) << 1) | 1;
        world.attack_descriptor(flags, unit)?;
        let target = match resolve_handle(world, unit.aircraft.attack_target)? {
            Some(actor) => AttackTarget::Actor(actor),
            None => AttackTarget::Point(unit.aircraft.attack_point),
        };
        let descriptor = AttackDescriptor {
            flags,
            source: unit.actor.key,
            target,
        };
        let ability = unit
            .aircraft
            .pending_ability
            .ok_or(AircraftError::NullDereference(
                "aircraft+0xA4 ability (0x4C8F20)",
            ))?;
        if world.ability_check(ability, &descriptor, unit)? == 0 {
            let ability = unit.aircraft.pending_ability;
            world.fire_ability(ability, &descriptor, unit)?;
        }
        unit.aircraft.attack_target = 0;
        unit.aircraft.pending_ability = None;
    }
    // Stack pose: position by bit copy, facing X/Y through fld/fstp, Z = 0.
    let position = unit.motion.position;
    let facing = [
        st(&b(unit.motion.facing[0])?)?,
        st(&b(unit.motion.facing[1])?)?,
    ];
    let speed = if unit.aircraft.path.segments.is_empty() {
        st(&world.actor_speed(unit)?)?
    } else {
        unit.aircraft.path.speed
    };
    let mut direction = [facing[0], facing[1], 0.0];
    norm(&mut direction, pc)?;
    let mut pose = Pose {
        position,
        direction,
        speed,
    };
    if unit.aircraft.group_a.is_some() && unit.aircraft.on_ground == 0 {
        let group = unit.aircraft.group_b.ok_or(AircraftError::NullDereference(
            "aircraft+0xAC group virtual +0x14",
        ))?;
        if world.group_disbanded(group, unit)? {
            world.leave_formation(unit)?;
        } else {
            let group = unit.aircraft.group_b.ok_or(AircraftError::NullDereference(
                "aircraft+0xAC group (0x5685B0)",
            ))?;
            let leader = group_leader(world, group)?;
            if leader != Some(unit.actor.key) && !following_path(&unit.aircraft) {
                clear_path(&mut unit.aircraft.path);
                // `fstp [esp+0x58]` (0x5614BE) stores the halved formation
                // speed into the stack pose's speed slot itself.
                pose.speed = st(&world.formation_speed(unit)?.mul(&k(HALF)?, pc)?)?;
                let limits = unit.mover_limits;
                set_mover_speed(&mut unit.mover, &limits, pose.speed, 1.0)?;
                follow_leader(unit, &pose, delta, pc, world)?;
                unit.aircraft.circling = 0;
            }
        }
    }
    if unit.aircraft.path.segments.is_empty() {
        plan_holding_circle(unit, &pose, pc, world)?;
        reset_member_paths(unit, world)?;
        unit.aircraft.circling = 1;
        if !unit.motion.has_mind {
            return Err(AircraftError::NullDereference("actor+0x70 Mind +0xC"));
        }
        if !world.mind_scheduled(unit)? {
            world.report_stopped_thinking(STOPPED_THINKING_LINE, unit.actor.handle)?;
            world.mind_reschedule(STOPPED_THINKING_DELAY_BITS, 0, unit)?;
        }
    }
    let (remaining, mut out) = advance_path(&mut unit.aircraft.path, delta, &pose, pc)?;
    let remaining = b(remaining)?;
    if remaining.compare(&k(ZERO)?) == Ordering::Greater {
        let cruise = b(unit.aircraft.cruise_speed)?;
        let z = st(&b(out.direction[2])?.mul(&remaining, pc)?)?;
        let x = st(&b(out.direction[0])?.mul(&remaining, pc)?.mul(&cruise, pc)?)?;
        let y = b(out.direction[1])?.mul(&remaining, pc)?.mul(&cruise, pc)?;
        let z = b(z)?.mul(&cruise, pc)?;
        out.position = [
            st(&b(x)?.add(&b(out.position[0])?, pc)?)?,
            st(&y.add(&b(out.position[1])?, pc)?)?,
            st(&z.add(&b(out.position[2])?, pc)?)?,
        ];
    }
    let travel = sub3(out.position, pose.position, pc)?;
    let mut facing = [st(&travel[0])?, st(&travel[1])?, st(&travel[2])?];
    norm(&mut facing, pc)?;
    world.commit_pose(out.position, facing, out.speed, unit)?;
    let height = world.ground_height(
        [out.position[0], out.position[1]],
        GROUND_HEIGHT_FLAGS,
        unit,
    )?;
    let definition = unit.actor.definition;
    let numerator = b(out.position[2])?
        .sub(&height, pc)?
        .sub(&b(definition.taxi_height)?, pc)?;
    let band = b(definition.operational_height)?.sub(&b(definition.taxi_height)?, pc)?;
    let mut fraction = stored_fraction(&numerator, &band, pc)?;
    // `fcomp 0.0; test ah, 5; jp` then `fcomp 1.0; test ah, 0x41; jne`:
    // NaN fails both tests and is kept.
    fraction = match fraction {
        INDEFINITE => INDEFINITE,
        NEGATIVE_INFINITY => ZERO,
        POSITIVE_INFINITY => ONE,
        bits => {
            let value = k(bits)?;
            if value.compare(&k(ZERO)?) == Ordering::Less {
                ZERO
            } else if value.compare(&k(ONE)?) == Ordering::Greater {
                ONE
            } else {
                bits
            }
        }
    };
    let scale = if unit.aircraft.on_ground == 0 {
        ONE
    } else if fraction == INDEFINITE {
        // 1.0 - NaN, * taxi_scale, + NaN all propagate the indefinite.
        INDEFINITE
    } else {
        let fraction = k(fraction)?;
        k(ONE)?
            .sub(&fraction, pc)?
            .mul(&b(definition.taxi_scale)?, pc)?
            .add(&fraction, pc)?
            .store_f32()?
    };
    unit.actor.scale = f32::from_bits(scale);
    if unit.aircraft.path.segments.is_empty() && unit.aircraft.circling == 0 && unit.motion.has_mind
    {
        world.mind_path_exhausted(unit)?;
    }
    Ok(MoveTickOutcome::Moved)
}
