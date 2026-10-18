//! UAIStateMove collision results that end in the low-path continuation
//! 0x531520: `KKC_UAIStateMove::ResultAvoidStatic` 0x534A40 and the
//! collision-with-a-thing-at-a-distance result 0x536AD0.
//!
//! Callers in the original executable: ResultCollision 0x536F40 calls
//! 0x534A40 with a tile (0x52F3E0) or actor (0x4CF210) descriptor and a null
//! thing, and 0x536AD0 with the resolved thing and the clearance distance;
//! 0x536AD0 calls 0x534A40 with the thing's 0x4CF210 descriptor and the thing.
//! aa-mind's `element_move_continuation::World::collision_plan` is 0x534A40
//! ([`avoid_static`]) and `World::stop_distance` is 0x536AD0 ([`avoid_thing`]).
//! `World::avoid_actor` is 0x536840 (ResultCollisionThing, the engage/kill
//! decision through 0x5358A0/0x5351C0); it does not reach 0x536AD0.
//!
//! Ported here, with their native helpers: 0x534A40, the option attempt
//! 0x533140, the option distance 0x52E210, descriptor alignment/extents
//! 0x46DAB0/0x46DC50/0x46DCB0, closest point 0x46E4F0/0x46E150, the Manhattan
//! test 0x51D510, the leader test 0x4CEFC0, 0x536AD0, the swept shape
//! constructor 0x46EB30/0x46E0E0 and the descriptor builders 0x4CF210,
//! 0x4EA0F0 and 0x52F3E0. 0x531520 is [`crate::low_path::continue_current`].
//!
//! Everything else is a native leaf service of [`UaiCollisionWorld`]. The
//! diagnostic log 0x404F90 (UAI.cpp lines 0x86C, 0x873, 0x87E) only reads
//! state and is omitted. All floating point arithmetic follows the x87
//! instruction sequence (extended intermediates, the native binary32 store
//! points) at the precision of `Environment::options`. Effects made before an
//! error remain (non-transactional); callers must not retry automatically.
use crate::MoveState;
use crate::low_path::{
    self, ActorPose, Continuation, ContinuationInput, ContinuationWorld, EPSILON, Environment,
    FLT_MAX, HALF, LowPathSolver, NEGATIVE_ONE, ONE, ZERO, f, node, normalize, pop_front_kind0,
    push_front, st,
};
use crate::result_plan::Object;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// 0x9FD47C (-0.5), used by the shape constructor 0x46E0E0.
const NEGATIVE_HALF: u32 = 0xbf00_0000;
/// 0xA32B58 (0.0002): 0x51D510 strict Manhattan threshold.
const MANHATTAN: u32 = 0x3951_b717;
/// 0xA028C4 (cos 45 degrees): an avoid option nearly perpendicular to the
/// collision vector is checked for skipping the waypoint.
const SKIP_COSINE: u32 = 0x3f35_04f3;
/// 0x9FDD44 (2.0): seconds a blocking thing stays ignored (0x4D6940 entry).
const IGNORE_SECONDS: u32 = 0x4000_0000;
/// 0xA033BC (0.25): speed margin of 0x536AD0.
const QUARTER: u32 = 0x3e80_0000;
/// 0.3 pushed by 0x536AD0 to the owner timer after a stop.
const STOP_DELAY: u32 = 0x3e99_999a;
/// 0x52F3E0 tile facing (0, -1).
const TILE_FACING: [u32; 2] = [ZERO, NEGATIVE_ONE];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    LowPath(#[from] low_path::Error),
    /// The original instruction at `address` dereferences a null pointer
    /// (an access violation in the game). Effects before it remain.
    #[error("native null dereference at {address:#x}")]
    NullDereference { address: u32 },
}
pub type Result<T> = std::result::Result<T, Error>;

fn null(address: u32) -> Error {
    Error::NullDereference { address }
}

/// Native shape object (0x46E060 family): word 0 kind (1 circle, 2 square,
/// 3 rectangle; other kinds have no extents), word 1 bounding radius, words
/// 2..5 local minimum X/Y and maximum X/Y. Same layout as
/// `aa_spatial::collision::Shape`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shape {
    pub kind: u32,
    pub radius_bits: u32,
    /// +8 min X, +0xC min Y, +0x10 max X, +0x14 max Y.
    pub bounds_bits: [u32; 4],
}
impl Shape {
    pub fn words(&self) -> [u32; 6] {
        let b = self.bounds_bits;
        [self.kind, self.radius_bits, b[0], b[1], b[2], b[3]]
    }
}

/// Native 6-word placed-shape descriptor (0x4CF210, 0x4EA0F0, 0x52F3E0,
/// 0x536AD0 locals). `shape` is the pointer in word 0 (None is a null
/// pointer, which the native code dereferences at its first use).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Descriptor {
    pub shape: Option<Shape>,
    pub facing_bits: [u32; 2],
    pub position_bits: [u32; 2],
    /// Word 5: alignment cache of 0x46DAB0 (0 = not computed yet).
    pub alignment: u32,
}

/// Mind virtual +0x44 followed by the caller's read of result word +4.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MindTarget {
    /// The virtual returned null; reading its word +4 faults.
    Null,
    /// Result word +4 is null.
    NoPoint,
    /// Result word +4 points at these two binary32 words.
    Point([u32; 2]),
}

/// The entry 0x534A40 appends with 0x4D6940 to the actor's +0x110 list:
/// the blocking thing's GTID (0x4CF1B0) and GWorld+0xE0 + 2.0 (binary32).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoredThing {
    pub gtid: u32,
    pub until_bits: u32,
}

/// Native leaf services. Objects are host keys ([`Object`]); `state` is the
/// UAIStateMove. Every method is a required boundary: no default invents a
/// collision, mind, organization or timer result. The movement services
/// already declared by [`ContinuationWorld`] are reused: `actor_pose`
/// (actor +0x20/+0x24, +0x2C/+0x30, reread at every native read point),
/// `action_go` (0x5300A0 with speed 0), `stop_action` (0x52D660) and
/// `schedule` (owner timer (+8) virtual +4, second argument 0).
pub trait UaiCollisionWorld: ContinuationWorld {
    /// 0x529C30: the state's owner (UAI, state +4) word +4, the moving actor.
    fn state_actor(&mut self, state: &mut MoveState) -> low_path::Result<Object>;
    /// Actor word +8 (flags; 0x534A40 tests bit 21).
    fn actor_flags(&mut self, actor: Object) -> low_path::Result<u32>;
    /// Actor word +0x14, the GTID 0x4CF1B0 copies.
    fn actor_gtid(&mut self, actor: Object) -> low_path::Result<u32>;
    /// `[[actor+0x80]+0x10]`: the organization thing (None when the +0x80
    /// component or its +0x10 word is null).
    fn organization_thing(&mut self, actor: Object) -> low_path::Result<Option<Object>>;
    /// Organization thing word +0x7C (organization component).
    fn organization_component(&mut self, thing: Object) -> low_path::Result<Option<Object>>;
    /// Organization component word +0x40 (leader actor).
    fn organization_leader(&mut self, component: Object) -> low_path::Result<Option<Object>>;
    /// Thing/actor word +0x70 (its Mind).
    fn thing_mind(&mut self, thing: Object) -> low_path::Result<Option<Object>>;
    /// Mind virtual +0x44, one call per native call.
    fn mind_virtual_44(&mut self, mind: Object) -> low_path::Result<MindTarget>;
    /// Mind virtual +0x68 on a thing's +0x70 Mind.
    fn mind_virtual_68(&mut self, mind: Object) -> low_path::Result<bool>;
    /// Owner (UAI, state +4) virtual +0x68.
    fn owner_virtual_68(&mut self, state: &mut MoveState) -> low_path::Result<bool>;
    /// 0x4CEDE0 (`aa_spatial::moving_state::excluded`).
    fn actor_moving(&mut self, actor: Object) -> low_path::Result<bool>;
    /// Actor virtual +0 (reach), the extended st(0) result.
    fn actor_virtual_0(&mut self, actor: Object) -> low_path::Result<Binary>;
    /// Actor virtual +4, the extended st(0) result (0x536AD0 doubles it as
    /// the swept-shape width).
    fn actor_virtual_4(&mut self, actor: Object) -> low_path::Result<Binary>;
    /// Actor virtual +0xC: the whole collision shape (None when the pointer
    /// is null). [`crate::low_path::LowPathWorld::actor_shape`] is the same
    /// virtual reduced to the three words 0x4325A0 reads.
    fn actor_collision_shape(&mut self, actor: Object) -> low_path::Result<Option<Shape>>;
    /// Actor virtual +0x10: soft shape pointer used by 0x4EA0F0.
    fn actor_soft_shape(&mut self, actor: Object) -> low_path::Result<Option<Shape>>;
    /// Actor virtual +0x18, the extended st(0) result.
    fn actor_virtual_18(&mut self, actor: Object) -> low_path::Result<Binary>;
    /// Actor virtual +0x20 with its native argument (1 here).
    fn actor_virtual_20(&mut self, actor: Object, argument: u32) -> low_path::Result<bool>;
    /// Actor virtual +0x16C(other).
    fn actor_virtual_16c(&mut self, actor: Object, other: Object) -> low_path::Result<bool>;
    /// 0x4D6940: append `entry` to the actor's +0x110/+0x114 list.
    fn ignore_thing(&mut self, actor: Object, entry: IgnoredThing) -> low_path::Result<()>;
    /// 0x531170(direction, distance, result): 0x554350 descriptor of the
    /// state's actor, 0x552250 at the actor, 0x5522B0 facing, generation
    /// checked target through 0x4CE6E0, 0x553CE0 to actor + direction *
    /// distance. Returns the four result words (the caller zeroed them); the
    /// native boolean is `result[0] != 0`.
    fn forward_probe(
        &mut self,
        state: &mut MoveState,
        direction: [u32; 2],
        distance_bits: u32,
    ) -> low_path::Result<[u32; 4]>;
    /// 0x5312C0(origin, direction, distance, result), the explicit-origin
    /// version of 0x531170; the native boolean is `result[0] != 0`.
    fn origin_probe(
        &mut self,
        state: &mut MoveState,
        origin: [u32; 2],
        direction: [u32; 2],
        distance_bits: u32,
    ) -> low_path::Result<[u32; 4]>;
    /// 0x46EF30(this = a, b): placed shapes overlap
    /// (`aa_spatial::shape_overlap::overlaps`).
    fn shapes_overlap(&mut self, a: &Descriptor, b: &Descriptor) -> low_path::Result<bool>;
    /// 0x46F100(this = a, range, b, inverted)
    /// (`aa_spatial::range::shapes_in_range`).
    fn shapes_in_range(
        &mut self,
        a: &Descriptor,
        range_bits: u32,
        b: &Descriptor,
        inverted: bool,
    ) -> low_path::Result<bool>;
}

/// Shared GWorld inputs and the 0x531520 binding.
pub struct Context<'a, 'g> {
    /// GWorld+0xE8 (multiplied by UAIStateMove +0x24 for the minimum option
    /// distance and the 0x536AD0 close range).
    pub world_step_bits: u32,
    /// GWorld+0xE0 simulation time (0x4D6940 entry, 0x531520 rate limit).
    pub world_time_bits: u32,
    /// `LowPathingWaitMinimum` SVar, looked up by 0x531520.
    pub wait_minimum_bits: u32,
    /// UAIStateMove +0x78, the last low-path solve time.
    pub last_solve_time_bits: &'a mut u32,
    /// GWorld+0x23C solver.
    pub solver: &'a mut LowPathSolver,
    /// Shared solve environment; its options carry the x87 precision.
    pub environment: &'a mut Environment<'g>,
}
impl Context<'_, '_> {
    fn precision(&self) -> Precision {
        self.environment.options.precision
    }
}

/// Native exits of 0x534A40.
#[derive(Clone, Debug)]
pub enum StaticOutcome {
    /// 0x52CF40 then 0x531520(0): empty route or zero collision vector at
    /// entry, or a blocked fallback probe without an ignorable thing.
    Continued(Continuation),
    /// 0x533140 accepted the (sorted) option `index`; it inserted kind-0
    /// nodes, cleared +0x54 and issued ActionGo(0).
    Avoided { index: usize },
    /// Both options failed and the forward probe was clear: ActionGo(0).
    Proceeded,
    /// Both options failed, the probe hit and the thing was appended to the
    /// actor's ignore list (0x4D6940): ActionGo(0).
    IgnoredThing(IgnoredThing),
}

/// Native exits of 0x536AD0.
#[derive(Clone, Debug)]
pub enum ThingOutcome {
    /// The thing is not moving (0x4CEDE0): 0x534A40 with its descriptor.
    Static(StaticOutcome),
    /// Same heading, overlapping and within close range: 0x52D660 and the
    /// owner timer with 0.3.
    Stopped,
    /// Soft shapes within 1.0: 0x52CF40 then 0x531520(0).
    Continued(Continuation),
    /// ActionGo(0).
    Proceeded,
}

fn less(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Less
}
fn greater(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Greater
}
fn mul(a: &Binary, bits: u32, p: Precision) -> Result<Binary> {
    Ok(a.mul(&f(bits)?, p)?)
}
/// `fld m32; fmul [-1.0]; fstp m32`.
fn negate(bits: u32, p: Precision) -> Result<u32> {
    Ok(st(&mul(&f(bits)?, NEGATIVE_ONE, p)?)?)
}
/// `fcom 0; test ah, 1; je; fchs`.
fn abs(value: Binary) -> Binary {
    if value.is_negative_nonzero() {
        value.negated()
    } else {
        value
    }
}
fn head(state: &MoveState, address: u32) -> Result<crate::PathNode> {
    state.path.nodes.first().copied().ok_or(null(address))
}
fn pose<W: UaiCollisionWorld>(world: &mut W, actor: Object) -> Result<ActorPose> {
    Ok(world.actor_pose(actor)?)
}

/// 0x4CF210: actor position and facing are read, then virtual +0xC.
pub fn actor_descriptor<W: UaiCollisionWorld>(actor: Object, world: &mut W) -> Result<Descriptor> {
    let pose = pose(world, actor)?;
    Ok(Descriptor {
        shape: world.actor_collision_shape(actor)?,
        facing_bits: pose.facing_bits,
        position_bits: pose.position_bits,
        alignment: 0,
    })
}

/// 0x4EA0F0: as 0x4CF210 with the soft shape (virtual +0x10).
pub fn soft_descriptor<W: UaiCollisionWorld>(actor: Object, world: &mut W) -> Result<Descriptor> {
    let pose = pose(world, actor)?;
    Ok(Descriptor {
        shape: world.actor_soft_shape(actor)?,
        facing_bits: pose.facing_bits,
        position_bits: pose.position_bits,
        alignment: 0,
    })
}

/// 0x52F3E0 on the GWorld+0x30 grid: `tile_shape` is the grid's embedded
/// +0x30 shape, `shift`/`cell_size_bits` its words +4/+8. The center is
/// `(tile << shift) + cell * 0.5` (x86 `shl` masks the count by 31).
pub fn tile_descriptor(
    shift: u32,
    cell_size_bits: u32,
    tile_shape: Shape,
    tile: [i32; 2],
    p: Precision,
) -> Result<Descriptor> {
    let half = f(st(&mul(&f(cell_size_bits)?, HALF, p)?)?)?;
    let center = |v: i32| -> Result<u32> {
        Ok(st(&Binary::from_i32(v.wrapping_shl(shift & 31)).add(&half, p)?)?)
    };
    Ok(Descriptor {
        shape: Some(tile_shape),
        facing_bits: TILE_FACING,
        position_bits: [center(tile[0])?, center(tile[1])?],
        alignment: 0,
    })
}

/// 0x46EB60 -> 0x46E0E0(size, size): the square shape. KKC_GMap's
/// constructor 0x556570 builds the GMap+0x30 tile shape that 0x52F3E0 uses
/// as `square_shape(GMap+8 cell size)` at the precision in force then.
pub fn square_shape(size_bits: u32, p: Precision) -> Result<Shape> {
    swept_shape(size_bits, size_bits, p)
}

/// 0x46EB30 -> 0x46E0E0(width, length): kind 2 when width == length
/// (ordered), else 3; bounds +/-0.5 * width and length; radius
/// sqrt(maxY^2 + minX^2) from the extended products.
pub fn swept_shape(width_bits: u32, length_bits: u32, p: Precision) -> Result<Shape> {
    let width = f(width_bits)?;
    let length = f(length_bits)?;
    let kind = if width.compare(&length) == Ordering::Equal {
        2
    } else {
        3
    };
    let min_x = mul(&width, NEGATIVE_HALF, p)?;
    let max_y = mul(&length, HALF, p)?;
    let radius = max_y
        .mul(&max_y, p)?
        .add(&min_x.mul(&min_x, p)?, p)?
        .sqrt(p)?;
    Ok(Shape {
        kind,
        radius_bits: st(&radius)?,
        bounds_bits: [
            st(&min_x)?,
            st(&mul(&length, NEGATIVE_HALF, p)?)?,
            st(&mul(&width, HALF, p)?)?,
            st(&max_y)?,
        ],
    })
}

/// 0x46DAB0: cached word 5, else 3 when |facing.x| < 1e-4, 2 when
/// |facing.y| < 1e-4, otherwise 1 (stored in word 5).
fn alignment(d: &mut Descriptor) -> Result<u32> {
    if d.alignment == 0 {
        let epsilon = f(EPSILON)?;
        d.alignment = if less(&abs(f(d.facing_bits[0])?), &epsilon) {
            3
        } else if less(&abs(f(d.facing_bits[1])?), &epsilon) {
            2
        } else {
            1
        };
    }
    Ok(d.alignment)
}

/// 0x46DC50 (`x_small` false) / 0x46DCB0 (`x_small` true) for a shape of
/// kind 2/3: alignment 2 selects max X (0x46DCB0) or max Y (0x46DC50),
/// alignment 3 the other one; otherwise the radius.
fn extent(d: &mut Descriptor, shape: Shape, x_small: bool) -> Result<Binary> {
    if shape.kind != 2 && shape.kind != 3 {
        return f(shape.radius_bits).map_err(Into::into);
    }
    let [_, _, max_x, max_y] = shape.bounds_bits;
    Ok(f(match (alignment(d)?, x_small) {
        (2, true) | (3, false) => max_x,
        (2, false) | (3, true) => max_y,
        _ => shape.radius_bits,
    })?)
}

/// 0x46E4F0 / 0x46E150: the point of the placed shape closest to `point`.
/// The local delta is stored to binary32; kind 1 normalizes it (0x409230)
/// and scales by the radius, kinds 2/3 clamp it in the facing frame (X
/// stored, Y extended unless clamped), other kinds use the static zero
/// vector 0xB279C0.
pub fn closest_point(d: &Descriptor, point: [u32; 2], p: Precision) -> Result<[u32; 2]> {
    let shape = d.shape.ok_or(null(0x46e156))?;
    let dy = st(&f(point[1])?.sub(&f(d.position_bits[1])?, p)?)?;
    let dx = st(&f(point[0])?.sub(&f(d.position_bits[0])?, p)?)?;
    let local = match shape.kind {
        1 => {
            let mut v = [dx, dy];
            normalize(&mut v, p)?;
            let r = f(shape.radius_bits)?;
            [
                st(&f(v[0])?.mul(&r, p)?)?,
                st(&f(v[1])?.mul(&r, p)?)?,
            ]
        }
        2 | 3 => {
            let [fx, fy] = [f(d.facing_bits[0])?, f(d.facing_bits[1])?];
            let [dx, dy] = [f(dx)?, f(dy)?];
            let [min_x, min_y, max_x, max_y] = shape.bounds_bits;
            let mut lx = st(&fy.mul(&dx, p)?.sub(&dy.mul(&fx, p)?, p)?)?;
            let mut ly = fy.mul(&dy, p)?.add(&fx.mul(&dx, p)?, p)?;
            if less(&f(lx)?, &f(min_x)?) {
                lx = min_x;
            } else if greater(&f(lx)?, &f(max_x)?) {
                lx = max_x;
            }
            if less(&ly, &f(min_y)?) {
                ly = f(min_y)?;
            } else if greater(&ly, &f(max_y)?) {
                ly = f(max_y)?;
            }
            let lx = f(lx)?;
            [
                st(&fx.mul(&ly, p)?.add(&fy.mul(&lx, p)?, p)?)?,
                st(&ly.mul(&fy, p)?.sub(&fx.mul(&lx, p)?, p)?)?,
            ]
        }
        // 0xB279C0/0xB279C4: initialized static zero vector.
        _ => [ZERO, ZERO],
    };
    Ok([
        st(&f(d.position_bits[0])?.add(&f(local[0])?, p)?)?,
        st(&f(local[1])?.add(&f(d.position_bits[1])?, p)?)?,
    ])
}

/// 0x51D510(a, b): |b.x - a.x| + |b.y - a.y| < 0.0002.
fn manhattan_near(a: [u32; 2], b: [u32; 2], p: Precision) -> Result<bool> {
    let x = abs(f(b[0])?.sub(&f(a[0])?, p)?);
    let y = abs(f(b[1])?.sub(&f(a[1])?, p)?);
    Ok(less(&y.add(&x, p)?, &f(MANHATTAN)?))
}

/// 0x52E210(descriptor, axis): signed distance the actor travels along
/// `axis` to clear the obstacle (extended st(0) result).
fn option_distance<W: UaiCollisionWorld>(
    d: &mut Descriptor,
    axis: [u32; 2],
    actor: Object,
    p: Precision,
    world: &mut W,
) -> Result<Binary> {
    let at = pose(world, actor)?.position_bits;
    let dx = f(st(&f(d.position_bits[0])?.sub(&f(at[0])?, p)?)?)?;
    let dy = f(d.position_bits[1])?.sub(&f(at[1])?, p)?;
    let projection = st(&f(axis[1])?
        .mul(&dy, p)?
        .add(&dx.mul(&f(axis[0])?, p)?, p)?)?;
    let shape = d.shape.ok_or(null(0x52e24f))?;
    let reach = st(&world
        .actor_virtual_0(actor)?
        .add(&f(shape.radius_bits)?, p)?)?;
    let mut clearance = f(reach)?;
    let aligned = alignment(d)?;
    if aligned == 3 || aligned == 2 {
        // |axis.x| <= 1e-4 selects 0x46DCB0, otherwise 0x46DC50.
        let x_small = !greater(&abs(f(axis[0])?), &f(EPSILON)?);
        let ext = extent(d, shape, x_small)?;
        clearance = f(reach)?.sub(&f(shape.radius_bits)?.sub(&ext, p)?, p)?;
    }
    let projection = f(projection)?;
    let epsilon = f(EPSILON)?;
    Ok(if less(&clearance, &projection.sub(&epsilon, p)?) {
        projection.sub(&clearance, p)?.sub(&epsilon, p)?
    } else {
        clearance.add(&projection, p)?.add(&epsilon, p)?
    })
}

struct OptionInput {
    collision: [u32; 2],
    direction: [u32; 2],
    axis: [u32; 2],
    distance: u32,
    minimum: u32,
    actor: Object,
}

/// 0x533140: try the avoid option `axis` at `distance`. On success two
/// (or one) kind-0 nodes are pushed to the front, +0x54 is cleared and
/// ActionGo(0) is issued; a failed attempt changes nothing.
fn try_option<W: UaiCollisionWorld>(
    d: &Descriptor,
    o: &OptionInput,
    state: &mut MoveState,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    let [ax, ay] = o.axis;
    let mut perpendicular = [ay ^ 0x8000_0000, ax];
    let side = f(o.direction[1])?
        .mul(&f(ax)?, p)?
        .add(&f(perpendicular[0])?.mul(&f(o.direction[0])?, p)?, p)?;
    if side.is_negative_nonzero() {
        perpendicular = [negate(perpendicular[0], p)?, negate(ax, p)?];
    }
    let step_y = st(&f(ay)?.mul(&f(o.distance)?, p)?)?;
    let step_x = st(&f(ax)?.mul(&f(o.distance)?, p)?)?;
    let at = pose(world, o.actor)?.position_bits;
    let y0 = st(&f(step_y)?.add(&f(at[1])?, p)?)?;
    let x0 = st(&f(step_x)?.add(&f(at[0])?, p)?)?;
    let mut closest = closest_point(d, [x0, y0], p)?;
    let shape = d.shape.ok_or(null(0x5331ff))?;
    let radius = f(shape.radius_bits)?;
    let epsilon = f(EPSILON)?;
    let dx = f(st(&f(d.position_bits[0])?.sub(&f(closest[0])?, p)?)?)?;
    let dy = f(d.position_bits[1])?.sub(&f(closest[1])?, p)?;
    let separation = dy.mul(&dy, p)?.add(&dx.mul(&dx, p)?, p)?;
    let inner = radius.sub(&epsilon, p)?;
    let inner = inner.mul(&inner, p)?;
    let mut origin = [x0, y0];
    let mut heading = o.axis;
    // Push the closest point out from the obstacle center by the actor's
    // reach (virtual +0) relative to the obstacle radius.
    let push_out = |world: &mut W, closest: [u32; 2]| -> Result<[u32; 2]> {
        let k = world
            .actor_virtual_0(o.actor)?
            .add(&epsilon, p)?
            .div(&radius, p)?;
        let ey = f(closest[1])?
            .sub(&f(d.position_bits[1])?, p)?
            .mul(&k, p)?;
        let dqx = st(&f(closest[0])?.sub(&f(d.position_bits[0])?, p)?)?;
        let ex = st(&f(dqx)?.mul(&k, p)?)?;
        Ok([
            st(&f(ex)?.add(&f(closest[0])?, p)?)?,
            st(&f(closest[1])?.add(&ey, p)?)?,
        ])
    };
    if !greater(&inner, &separation) {
        let pushed = push_out(world, closest)?;
        let at = pose(world, o.actor)?.position_bits;
        let dy = f(pushed[1])?.sub(&f(at[1])?, p)?;
        let dx = f(pushed[0])?.sub(&f(at[0])?, p)?;
        let length = st(&dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?.sqrt(p)?)?;
        let inverse = f(st(&f(ONE)?.div(&f(length)?, p)?)?)?;
        let unit = [st(&dx.mul(&inverse, p)?)?, st(&dy.mul(&inverse, p)?)?];
        let mut direct = false;
        if !less(&f(length)?, &f(o.minimum)?) {
            let toward = f(unit[0])?
                .mul(&f(o.collision[0])?, p)?
                .add(&f(o.collision[1])?.mul(&f(unit[1])?, p)?, p)?;
            if !toward.is_negative_nonzero()
                && world.forward_probe(state, unit, length)?[0] == 0
            {
                heading = unit;
                origin = pushed;
                direct = true;
            }
        }
        if !direct && world.forward_probe(state, o.axis, o.distance)?[0] != 0 {
            return Ok(false);
        }
    } else {
        // The option point lies inside the obstacle: step twice the radius
        // along the perpendicular, then push out.
        let ry = f(perpendicular[1])?.mul(&radius, p)?;
        let rx = st(&f(perpendicular[0])?.mul(&radius, p)?)?;
        let ry = ry.add(&ry, p)?;
        let rx = f(rx)?;
        let rx = st(&rx.add(&rx, p)?)?;
        let y2 = st(&ry.add(&f(y0)?, p)?)?;
        let x2 = st(&f(rx)?.add(&f(x0)?, p)?)?;
        closest = closest_point(d, [x2, y2], p)?;
        origin = push_out(world, closest)?;
        let at = pose(world, o.actor)?.position_bits;
        let dy = st(&f(origin[1])?.sub(&f(at[1])?, p)?)?;
        let dx = f(origin[0])?.sub(&f(at[0])?, p)?;
        let dy = f(dy)?;
        let length = st(&dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?.sqrt(p)?)?;
        let inverse = f(st(&f(ONE)?.div(&f(length)?, p)?)?)?;
        heading = [st(&dx.mul(&inverse, p)?)?, st(&dy.mul(&inverse, p)?)?];
        if world.forward_probe(state, heading, length)?[0] != 0 {
            return Ok(false);
        }
    }
    let qy = f(closest[1])?.sub(&f(origin[1])?, p)?;
    let qx = f(closest[0])?.sub(&f(origin[0])?, p)?;
    let mut depth = st(&f(perpendicular[1])?
        .mul(&qy, p)?
        .add(&qx.mul(&f(perpendicular[0])?, p)?, p)?)?;
    if greater(&f(depth)?, &f(ZERO)?) {
        if less(&f(depth)?, &f(o.minimum)?) {
            depth = o.minimum;
        }
        let toward = f(o.collision[0])?
            .mul(&f(heading[0])?, p)?
            .add(&f(o.collision[1])?.mul(&f(heading[1])?, p)?, p)?;
        if less(&toward, &epsilon) {
            if world.origin_probe(state, origin, perpendicular, depth)?[0] != 0 {
                return Ok(false);
            }
            let y = f(perpendicular[1])?.mul(&f(depth)?, p)?;
            let x = st(&f(perpendicular[0])?.mul(&f(depth)?, p)?)?;
            let position = [
                st(&f(x)?.add(&f(origin[0])?, p)?)?,
                st(&y.add(&f(origin[1])?, p)?)?,
            ];
            push_front(&mut state.path, node(position, perpendicular, 0))?;
        }
    }
    push_front(&mut state.path, node(origin, heading, 0))?;
    state.waypoint_offset_bits = 0;
    world.action_go(0, state)?;
    Ok(true)
}

fn continue_low_path<W: UaiCollisionWorld>(
    state: &mut MoveState,
    ctx: &mut Context<'_, '_>,
    world: &mut W,
) -> Result<Continuation> {
    pop_front_kind0(&mut state.path);
    Ok(low_path::continue_current(
        ContinuationInput {
            argument: 0,
            world_time_bits: ctx.world_time_bits,
            wait_minimum_bits: ctx.wait_minimum_bits,
        },
        state,
        ctx.last_solve_time_bits,
        ctx.solver,
        ctx.environment,
        world,
    )?)
}

/// x87 ordered equality with +0.0 (`fucompp`, `test ah, 0x44`).
fn is_zero(bits: u32) -> bool {
    bits & 0x7fff_ffff == 0
}

/// Complete 0x534A40 `KKC_UAIStateMove::ResultAvoidStatic(descriptor,
/// collision vector, thing)` without diagnostics. `obstacle` is the caller's
/// descriptor: its word 5 alignment cache is written as the native code does.
/// `thing` is null (None) from 0x536F40 and the moving thing from 0x536AD0.
pub fn avoid_static<W: UaiCollisionWorld>(
    obstacle: &mut Descriptor,
    collision: [u32; 2],
    thing: Option<Object>,
    state: &mut MoveState,
    ctx: &mut Context<'_, '_>,
    world: &mut W,
) -> Result<StaticOutcome> {
    let p = ctx.precision();
    let minimum = st(&f(ctx.world_step_bits)?.mul(&f(state.requested_speed_bits)?, p)?)?;
    let mut axes = [[ONE, ZERO], [ZERO, ONE]];
    let mut distances = [FLT_MAX, FLT_MAX];
    let actor = world.state_actor(state)?;
    let group = world.organization_thing(actor)?;
    let mut leader = None;
    if let Some(group) = group {
        let component = world
            .organization_component(group)?
            .ok_or(null(0x534b78))?;
        leader = world.organization_leader(component)?;
    }
    let mut follow = false;
    // 0x4CEFC0 rereads the same links: is the actor its own leader?
    if let Some(group) = group
        && leader != Some(actor)
        && state.mode == 3
        && head(state, 0x534af2)?.words[4] != 0
    {
        let mind = world.thing_mind(group)?.ok_or(null(0x534afc))?;
        match world.mind_virtual_44(mind)? {
            MindTarget::Null => return Err(null(0x534b01)),
            MindTarget::NoPoint => {}
            MindTarget::Point(_) => follow = true,
        }
    }
    pop_front_kind0(&mut state.path);
    if state.path.nodes.is_empty() || (is_zero(collision[0]) && is_zero(collision[1])) {
        return Ok(StaticOutcome::Continued(continue_low_path(
            state, ctx, world,
        )?));
    }
    let mut direction = collision;
    if follow {
        let leader = leader.ok_or(null(0x4cede0))?;
        let mut target = None;
        if world.actor_moving(leader)?
            && let Some(mind) = world.thing_mind(leader)?
        {
            match world.mind_virtual_44(mind)? {
                MindTarget::Null => return Err(null(0x534bb3)),
                MindTarget::NoPoint => {}
                MindTarget::Point(_) => match world.mind_virtual_44(mind)? {
                    MindTarget::Null => return Err(null(0x534bc2)),
                    MindTarget::NoPoint => return Err(null(0x534bc5)),
                    MindTarget::Point(point) => target = Some(point),
                },
            }
        }
        let from = pose(world, leader)?.position_bits;
        let delta = match target {
            Some(point) => Some([point, from]),
            None => {
                let at = pose(world, actor)?.position_bits;
                (!manhattan_near(at, from, p)?).then_some([from, at])
            }
        };
        if let Some([to, from]) = delta {
            direction = [
                st(&f(to[0])?.sub(&f(from[0])?, p)?)?,
                st(&f(to[1])?.sub(&f(from[1])?, p)?)?,
            ];
            normalize(&mut direction, p)?;
        }
    }
    let shape = obstacle.shape.ok_or(null(0x534c30))?;
    if shape.kind == 0 || shape.kind == 1 {
        let at = pose(world, actor)?.position_bits;
        let dy = st(&f(obstacle.position_bits[1])?.sub(&f(at[1])?, p)?)?;
        let dx = f(obstacle.position_bits[0])?.sub(&f(at[0])?, p)?;
        axes[0] = [dy ^ 0x8000_0000, st(&dx)?];
        normalize(&mut axes[0], p)?;
        distances[0] = st(&world
            .actor_virtual_0(actor)?
            .add(&f(shape.radius_bits)?, p)?
            .add(&f(EPSILON)?, p)?)?;
        let dot = f(direction[1])?
            .mul(&f(axes[0][1])?, p)?
            .add(&f(direction[0])?.mul(&f(axes[0][0])?, p)?, p)?;
        if dot.is_negative_nonzero() {
            axes[0] = [negate(axes[0][0], p)?, negate(axes[0][1], p)?];
        }
    } else {
        let [fx, fy] = obstacle.facing_bits;
        axes = [[fx, fy], [fy, fx ^ 0x8000_0000]];
        for axis in &mut axes {
            let dot = f(direction[1])?
                .mul(&f(axis[1])?, p)?
                .add(&f(direction[0])?.mul(&f(axis[0])?, p)?, p)?;
            let negative = if less(&abs(dot.clone()), &f(EPSILON)?) {
                let route = head(state, 0x534d35)?;
                f(route.words[3])?
                    .mul(&f(axis[1])?, p)?
                    .add(&f(axis[0])?.mul(&f(route.words[2])?, p)?, p)?
                    .is_negative_nonzero()
            } else {
                dot.is_negative_nonzero()
            };
            if negative {
                *axis = [negate(axis[0], p)?, negate(axis[1], p)?];
            }
        }
        for i in 0..2 {
            let [ax, ay] = axes[i];
            let distance = st(&option_distance(obstacle, axes[i], actor, p, world)?)?;
            distances[i] = distance;
            let along = abs(f(collision[1])?
                .mul(&f(ay)?, p)?
                .add(&f(collision[0])?.mul(&f(ax)?, p)?, p)?);
            if less(&along, &f(SKIP_COSINE)?) {
                // Does this option pass beyond the waypoint?
                let skip = |world: &mut W,
                            state: &MoveState,
                            axis: [u32; 2],
                            distance: u32|
                 -> Result<Binary> {
                    let ty = st(&f(axis[1])?.mul(&f(distance)?, p)?)?;
                    let tx = st(&f(distance)?.mul(&f(axis[0])?, p)?)?;
                    let at = pose(world, actor)?.position_bits;
                    let y = f(ty)?.add(&f(at[1])?, p)?;
                    let x = f(tx)?.add(&f(at[0])?, p)?;
                    let route = head(state, 0x534e02)?;
                    let wy = st(&f(route.words[1])?.sub(&y, p)?)?;
                    let wx = f(route.words[0])?.sub(&x, p)?;
                    Ok(wx
                        .mul(&f(axis[0])?, p)?
                        .add(&f(wy)?.mul(&f(axis[1])?, p)?, p)?)
                };
                if skip(world, state, axes[i], distance)?.is_negative_nonzero() {
                    let flipped = [negate(ax, p)?, negate(ay, p)?];
                    let other = st(&option_distance(obstacle, flipped, actor, p, world)?)?;
                    // 0x86C: the flipped option does not skip; 0x873: both
                    // skip, keep the shorter.
                    let replace = !skip(world, state, flipped, other)?.is_negative_nonzero()
                        || less(&f(other)?, &f(distance)?);
                    if replace {
                        axes[i] = flipped;
                        distances[i] = other;
                    }
                }
            }
            // 0x87E: an option that moves away from the obstacle is ignored.
            if !greater(&f(distances[i])?, &f(ZERO)?) {
                distances[i] = FLT_MAX;
            }
            if less(&f(distances[i])?, &f(minimum)?) {
                distances[i] = minimum;
            }
        }
    }
    if less(&f(distances[1])?, &f(distances[0])?) {
        axes.swap(0, 1);
        distances.swap(0, 1);
    }
    for index in 0..2 {
        if distances[index] == FLT_MAX {
            continue;
        }
        let option = OptionInput {
            collision,
            direction,
            axis: axes[index],
            distance: distances[index],
            minimum,
            actor,
        };
        if try_option(obstacle, &option, state, p, world)? {
            return Ok(StaticOutcome::Avoided { index });
        }
    }
    let doubled = f(minimum)?;
    let doubled = st(&doubled.add(&doubled, p)?)?;
    let route = head(state, 0x535083)?;
    if world.forward_probe(state, [route.words[2], route.words[3]], doubled)?[0] == 0 {
        world.action_go(0, state)?;
        return Ok(StaticOutcome::Proceeded);
    }
    let mut blocked = (world.actor_flags(actor)? >> 21) & 1 != 0
        && world.actor_virtual_20(actor, 1)?
        && match thing {
            None => true,
            Some(other) => {
                (world.actor_flags(other)? >> 21) & 1 != 0 && world.actor_virtual_20(other, 1)?
            }
        };
    if !blocked {
        blocked = match thing {
            None => true,
            Some(other) => match world.thing_mind(other)? {
                None => true,
                Some(mind) => {
                    world.actor_moving(other)?
                        || world.mind_virtual_68(mind)?
                        || world.actor_virtual_16c(actor, other)?
                        || world.owner_virtual_68(state)?
                }
            },
        };
    }
    match thing {
        Some(other) if !blocked => {
            let entry = IgnoredThing {
                gtid: world.actor_gtid(other)?,
                until_bits: st(&f(ctx.world_time_bits)?.add(&f(IGNORE_SECONDS)?, p)?)?,
            };
            world.ignore_thing(actor, entry)?;
            world.action_go(0, state)?;
            Ok(StaticOutcome::IgnoredThing(entry))
        }
        _ => Ok(StaticOutcome::Continued(continue_low_path(
            state, ctx, world,
        )?)),
    }
}

/// Complete 0x536AD0(thing, distance): collision with `thing` at the
/// clearance `distance_bits` computed by 0x536F40. `collision` is
/// UAIStateMove +0x4C/+0x50 (the vector saved by 0x52FD50).
pub fn avoid_thing<W: UaiCollisionWorld>(
    thing: Object,
    distance_bits: u32,
    collision: [u32; 2],
    state: &mut MoveState,
    ctx: &mut Context<'_, '_>,
    world: &mut W,
) -> Result<ThingOutcome> {
    let p = ctx.precision();
    if !world.actor_moving(thing)? {
        let mut descriptor = actor_descriptor(thing, world)?;
        return Ok(ThingOutcome::Static(avoid_static(
            &mut descriptor,
            collision,
            Some(thing),
            state,
            ctx,
            world,
        )?));
    }
    let actor = world.state_actor(state)?;
    let own = pose(world, actor)?.facing_bits;
    let other = pose(world, thing)?.facing_bits;
    let heading = f(own[1])?
        .mul(&f(other[1])?, p)?
        .add(&f(own[0])?.mul(&f(other[0])?, p)?, p)?;
    let heading_bits = st(&heading)?;
    let mut soft = true;
    if greater(&heading, &f(ZERO)?) {
        let distance = f(distance_bits)?;
        // Each swept rectangle: width 2 * virtual +4, length `distance`,
        // centered half the distance ahead of the actor along its facing.
        let swept = |world: &mut W, who: Object| -> Result<Descriptor> {
            let width = world.actor_virtual_4(who)?;
            let shape = swept_shape(st(&width.add(&width, p)?)?, distance_bits, p)?;
            let at = pose(world, who)?;
            let half_x = st(&mul(&distance.mul(&f(at.facing_bits[0])?, p)?, HALF, p)?)?;
            let half_y = st(&mul(&distance.mul(&f(at.facing_bits[1])?, p)?, HALF, p)?)?;
            Ok(Descriptor {
                shape: Some(shape),
                facing_bits: at.facing_bits,
                position_bits: [
                    st(&f(half_x)?.add(&f(at.position_bits[0])?, p)?)?,
                    st(&f(half_y)?.add(&f(at.position_bits[1])?, p)?)?,
                ],
                alignment: 0,
            })
        };
        let ahead = swept(world, thing)?;
        let own_ahead = swept(world, actor)?;
        if !world.shapes_overlap(&ahead, &own_ahead)? {
            world.action_go(0, state)?;
            return Ok(ThingOutcome::Proceeded);
        }
        let closing = st(&world.actor_virtual_18(thing)?.mul(&f(heading_bits)?, p)?)?;
        if !greater(&world.actor_virtual_18(actor)?, &f(closing)?) {
            world.action_go(0, state)?;
            return Ok(ThingOutcome::Proceeded);
        }
        let speed = st(&world.actor_virtual_18(thing)?)?;
        let margin = world.actor_virtual_18(actor)?.sub(&f(QUARTER)?, p)?;
        soft = greater(&margin, &f(speed)?);
        if !soft {
            let range = st(&f(ctx.world_step_bits)?
                .mul(&f(state.requested_speed_bits)?, p)?
                .add(&f(EPSILON)?, p)?)?;
            let target = actor_descriptor(thing, world)?;
            let own = actor_descriptor(actor, world)?;
            if world.shapes_in_range(&own, range, &target, false)? {
                world.stop_action(state)?;
                world.schedule(STOP_DELAY, state)?;
                return Ok(ThingOutcome::Stopped);
            }
        }
    }
    if soft {
        let own = soft_descriptor(actor, world)?;
        let target = soft_descriptor(thing, world)?;
        if world.shapes_in_range(&target, ONE, &own, false)? {
            return Ok(ThingOutcome::Continued(continue_low_path(
                state, ctx, world,
            )?));
        }
    }
    world.action_go(0, state)?;
    Ok(ThingOutcome::Proceeded)
}
