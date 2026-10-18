//! Original `KKC_LowPathSolver` (`.\Source\Sim\Solver\LowPathSolver.cpp`), the
//! moving-obstacle avoidance solver that runs inside a 64x64 sub-cell window
//! around the actor, and its UAIStateMove continuation 0x531520.
//!
//! Native objects and their Rust owners:
//!
//! | native | Rust |
//! |---|---|
//! | solver, 0xD0 bytes at GWorld+0x23C, constructor 0x53BB90 | [`LowPathSolver`] |
//! | solver vtable 0xA36460: [0] 0x53B0C0 expand, [1] 0x7C7840 examine (always 3), [2] 0x4F0550 mark start, [3] 0x53A790 queue exhausted | `search.rs` |
//! | window grid at solver+0x44, constructor 0x53A490 | [`Window`] |
//! | goal at solver+0x7C, vtable 0xA3634C (13 entries, 0xA36348 is a null RTTI word) | [`Goal`] |
//! | queue at solver+0xB4, shared class 0x53CDF0 / vtable 0xA3668C | [`crate::search::NativeQueue`] |
//! | timer at solver+0xC8 (0x59AA70 / 0x59AB60) | none: write-only QueryPerformanceCounter, see below |
//! | LowPathStats at GWorld+4 (0x539F30 / 0x539F60) | [`LowPathStats`] |
//!
//! Goal vtable 0xA3634C: +00 0x53A5D0 IsGoal(cell); +04 0x53A5F0 GetStart;
//! +08 0x53B720 OnSolveStarted; +0C 0x539F00 OnFailed (clears path/context
//! links); +10 0x53B960 OnSolved(cell); +14 0x53A610 Mark(from,to,cell);
//! +18 0x539EE0 IsVisited(cell); +1C 0x539F10 IsBlocked(cell) = 0x54F370 on the
//! cell's world tile with the solve context; +20 0x4C9FA0 start depth (0);
//! +24 0x4DB6B0 start cost (+0.0); +28 0x6A71D0 (returns 1, not called by this
//! solver); +2C 0x53A5C0 direct-line flag (+0x30); +30 0x6A71F0 (returns 0:
//! the neighbour fan is always chosen from the predecessor direction).
//!
//! Wall-clock time: 0x53C170 calls 0x41C0E0/0x41C110 with slot 0x17 (both are
//! no-ops for any slot outside 1..=4) and 0x59AB60, which stores
//! QueryPerformanceCounter at solver+0xC8 and discards the computed
//! milliseconds. Nothing in the solve reads solver+0xC8 and 0x59ABA0 (elapsed)
//! is not reachable from it, so no search limit depends on wall-clock time and
//! the timer is not modelled. The examined-node and depth limits are counts.
//!
//! Diagnostics: the PathLog flag 0xB3D43C guards 0x404F60 / 0x52D440 string
//! logging, and OnSolveStarted logs "fixed destination too close to source"
//! through 0x404F60. They read state only and are omitted.
mod continuation;
mod goal;
mod search;
mod window;

pub use continuation::{
    Continuation, ContinuationInput, ContinuationOutcome, ContinuationWorld, continue_current,
};
pub use goal::Goal;
pub use search::SearchState;
pub use window::{Window, WindowCell, WindowConfig};
///Exact53A3A0 finite segment parameter, also reached by terrain ray clipping.
pub use window::segment as segment_parameter_53a3a0;

use crate::result_plan::{CollisionContext, Object};
use crate::search::SearchGrid;
use crate::{OwnedPath, PathNode};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Queue(#[from] crate::search::SearchError),
    #[error(transparent)]
    Navigation(#[from] crate::Error),
    #[error("low-path allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error("invalid low-path window configuration")]
    InvalidWindow,
    #[error("window cell index {0} is outside the native window storage")]
    WindowCell(i64),
    #[error("world tile index {0} is outside the native world grid")]
    WorldTile(i64),
    #[error("invalid native predecessor direction {0}")]
    InvalidDirection(u32),
    #[error("cell delta {0:?} has no native direction code")]
    InvalidDelta([i32; 2]),
    /// 0x531520 reads the tail node of an empty path after a nonzero Solve
    /// (native null dereference at 0x531711); no node is appended.
    #[error("a nonzero low-path solve left the previously empty path empty")]
    EmptySolvedPath,
    #[error("low-path world boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) const ZERO: u32 = 0;
pub(crate) const ONE: u32 = 0x3f80_0000; // 0x9F32E0
pub(crate) const HALF: u32 = 0x3f00_0000; // 0x9FBDB8
pub(crate) const SQRT2: u32 = 0x3fb5_04f3; // 0xA363AC
pub(crate) const EPSILON: u32 = 0x38d1_b717; // 0xA360A8 and 0x9F3228
pub(crate) const GOAL_EPSILON: u32 = 0x3951_b717; // 0xA32B58
pub(crate) const NEGATIVE_ONE: u32 = 0xbf80_0000; // 0x9FDC48
pub(crate) const FLT_MAX: u32 = 0x7f7f_ffff;
pub(crate) const TWO_POW_32: u32 = 0x4f80_0000; // 0x9F240C

/// Search depth rule of 0x53A660 (`cmp [entry+8], 0x20`). This is a native
/// search rule that shapes routes, not a storage cap, and is kept.
pub const DEPTH_LIMIT: i32 = 0x20;

/// The low-path queue is the same class as the tile search queue (0x53CDF0
/// stores limit 0x1F9C at +0x10). Pass this as `Options::queue_limit` for the
/// exact native bounded-queue policy; storage itself always grows.
pub const NATIVE_QUEUE_LIMIT: usize = crate::search::NATIVE_QUEUE_LIMIT;

/// Named SVars read through 0x4B2E60. The executable registers no code default
/// for them; a missing Var reports "Requested non-existent Var" and the
/// caller dereferences null. Values are therefore always explicit inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowPathSettings {
    /// `LowPathingExaminedMax` (int, type 3), 0x53A660: stop when the number
    /// of marked cells reaches it.
    pub examined_max: i32,
    /// `LowPathingIgnoreMovingDepth` (int, type 3), 0x53BC80: neighbour tests
    /// at depth >= this set descriptor byte +0x31 (ignore moving actors).
    pub ignore_moving_depth: i32,
    /// `LowPathingAccuracyDepth` (int, type 3), 0x53BC80: neighbour tests at
    /// depth < this use a full sweep instead of an end-point query.
    pub accuracy_depth: i32,
}
impl LowPathSettings {
    /// Values shipped in the original `Data/Game/SVars.tgi` (lines 96-98).
    /// They are data, not executable defaults; callers choose them explicitly.
    pub const ORIGINAL_DATA: Self = Self {
        examined_max: 512,
        ignore_moving_depth: 3,
        accuracy_depth: 5,
    };
}
/// `LowPathingWaitMinimum` (fixed, type 2) as shipped in `SVars.tgi` line 95
/// ("2"), binary32 2.0. Data value only; 0x531520 takes it explicitly.
pub const ORIGINAL_DATA_WAIT_MINIMUM_BITS: u32 = 0x4000_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Options {
    /// x87 precision control in force for the solve. The game runs at PC53;
    /// the oracles prove PC24, PC53 and PC64. The family uses only
    /// FADD/FSUB/FMUL/FDIV/FSQRT/FCOM/FILD/FIST (no transcendental opcodes).
    pub precision: Precision,
    /// `None` is unbounded growth; `Some(NATIVE_QUEUE_LIMIT)` is native policy.
    pub queue_limit: Option<usize>,
}

/// LowPathStats (0x34 bytes at GWorld+4, constructor 0x539F30 zeroes it).
/// Word 0 is never written by 0x539F60. Updated after every Solve when the
/// GWorld+4 object exists; read only by PrintStats 0x539FD0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowPathStats {
    /// [1] solves, [2] status 1, [3] status 2, [4] status 0, [5]/[6] queue sum
    /// and signed maximum, [7]/[8] examined sum and maximum, [9]..[12] the
    /// same four for status 1 only.
    pub words: [u32; 13],
}
impl LowPathStats {
    /// 0x539F60(status, queue count, examined count).
    pub fn record(&mut self, status: u32, queue: i32, examined: i32) {
        let w = &mut self.words;
        w[1] = w[1].wrapping_add(1);
        w[5] = w[5].wrapping_add(queue as u32);
        if queue > w[6] as i32 {
            w[6] = queue as u32;
        }
        w[7] = w[7].wrapping_add(examined as u32);
        if examined > w[8] as i32 {
            w[8] = examined as u32;
        }
        match status {
            0 => w[4] = w[4].wrapping_add(1),
            1 => {
                w[2] = w[2].wrapping_add(1);
                w[9] = w[9].wrapping_add(queue as u32);
                if queue > w[10] as i32 {
                    w[10] = queue as u32;
                }
                w[11] = w[11].wrapping_add(examined as u32);
                if examined > w[12] as i32 {
                    w[12] = examined as u32;
                }
            }
            2 => w[3] = w[3].wrapping_add(1),
            _ => {}
        }
    }
}

/// First four result words of 0x553BA0 / 0x553CE0; the solver reads only
/// the kind (0 none, 1 static tile, 2 actor GTID, 3 outside) and reference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialHit {
    pub kind: u32,
    pub reference: u32,
}

/// Raw actor words +0x20/+0x24 (position) and +0x2C/+0x30 (facing).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorPose {
    pub position_bits: [u32; 2],
    pub facing_bits: [u32; 2],
}

/// Shape returned by actor virtual +0xC; 0x4325A0 selects word +0x14 for
/// kinds 2 and 3 and word +4 otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorShape {
    pub kind: u32,
    pub word1_bits: u32,
    pub word5_bits: u32,
}
impl ActorShape {
    /// 0x4325A0.
    pub fn radius_bits(&self) -> u32 {
        if self.kind == 3 || self.kind == 2 {
            self.word5_bits
        } else {
            self.word1_bits
        }
    }
}

/// External world services used by the low-path solver. The collision
/// descriptor is the caller's 0x58-byte context: pointer-bearing words hold
/// host keys (word 7, +0x1C, is the source actor; 0 means none). Positions,
/// facing (0x5522B0) and byte +0x31 are written by this module; every other
/// effect belongs to the world. No callback has a permissive default.
pub trait LowPathWorld {
    /// 0x54F370 for the world tile under a window cell, with the solve context.
    fn tile_blocked(&mut self, tile: [i32; 2], context: &CollisionContext) -> Result<bool>;
    /// 0x552250: stores the position (+0x4C/+0x50), snapping through the
    /// source definition when its ObjectData flag requests it.
    fn set_context_position(
        &mut self,
        context: &mut CollisionContext,
        position: [u32; 2],
    ) -> Result<()>;
    /// 0x553BA0 on GWorld+0x30 at the context's current placement.
    fn first_hit(&mut self, context: &CollisionContext) -> Result<SpatialHit>;
    /// 0x553CE0 on GWorld+0x30 from the context placement to `goal`.
    fn sweep(&mut self, context: &CollisionContext, goal: [u32; 2]) -> Result<SpatialHit>;
    /// 0x4F3C40(GWorld, source actor (context +0x1C), context): 0 means clear.
    fn placement_result(
        &mut self,
        source: Option<Object>,
        context: &CollisionContext,
    ) -> Result<u32>;
    /// Generation-checked GTID lookup in the registry at 0xB364C0.
    fn resolve_actor(&mut self, reference: u32) -> Option<Object>;
    /// Actor virtual +0xB4.
    fn actor_virtual_b4(&mut self, actor: Object) -> Result<bool>;
    /// Actor word +0x90 is nonnull.
    fn actor_word_90_present(&mut self, actor: Object) -> Result<bool>;
    /// Actor virtual +0x100 (its nonzero result gates goal displacement).
    fn actor_virtual_100(&mut self, actor: Object) -> Result<u32>;
    /// 0x552340(context, actor): true excludes the actor.
    fn context_excludes(&mut self, context: &CollisionContext, actor: Object) -> Result<bool>;
    /// Raw actor pose words, read at each native read point.
    fn actor_pose(&mut self, actor: Object) -> Result<ActorPose>;
    /// Source actor virtual +0, stored to binary32 by 0x53AB90.
    fn source_virtual_0(&mut self, source: Object) -> Result<u32>;
    /// Actor virtual +0xC (shape pointer) read by 0x4325A0.
    fn actor_shape(&mut self, actor: Object) -> Result<ActorShape>;
    /// 0x4D67F0 on the source actor (overlapping-thing list refresh).
    fn refresh_overlaps(&mut self, source: Object) -> Result<()>;
}

/// Exact persistent solver state (GWorld+0x23C).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LowPathSolver {
    pub window: Window,
    pub goal: Goal,
    pub search: SearchState,
    /// Queue count (+0xBC) left by the last search that cleared the queue;
    /// 0x53C170 passes it to LowPathStats, including after a busy rejection.
    pub queue_remaining: usize,
}
impl LowPathSolver {
    /// 0x53BB90(world grid, sub shift, tiles wide, tiles high). Goal and
    /// search words not written by the constructor are never read before
    /// a search writes them; they start at zero here.
    pub fn new(grid: &SearchGrid, config: WindowConfig, precision: Precision) -> Result<Self> {
        Ok(Self {
            window: Window::new(grid, config, precision)?,
            goal: Goal::default(),
            search: SearchState::default(),
            queue_remaining: 0,
        })
    }
}

/// Shared GWorld state used by a solve.
pub struct Environment<'a> {
    /// GWorld+0x30 world tile grid: epoch +0x48 (shared with the tile
    /// search), tile flags +0x20 and visit stamps +0x28.
    pub grid: &'a mut SearchGrid,
    /// 0xB3D43D re-entry flag shared with the tile search 0x53D8D0.
    pub search_busy: &'a mut bool,
    /// GWorld+4, absent when the stats object was not created.
    pub stats: Option<&'a mut LowPathStats>,
    pub settings: LowPathSettings,
    pub options: Options,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub start_bits: [u32; 2],
    pub goal_bits: [u32; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// Native return: 0 failure, 1 goal or direct line, 2 partial route.
    pub status: u32,
    /// Every popped queue entry (solver +0x24) in native order, as the tile
    /// search reports its expansion (`search::SearchOutcome::expanded`).
    pub expanded: Vec<crate::search::QueueEntry>,
}

pub(crate) fn f(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
pub(crate) fn st(value: &Binary) -> Result<u32> {
    Ok(value.store_f32()?)
}
/// 0x96CC4C truncation. Results outside the signed 32-bit range (where the
/// native routine returns the low word of a 64-bit integer) are errors.
pub(crate) fn ftol(value: &Binary) -> Result<i32> {
    Ok(value.truncate_i32()?)
}
/// 0x409230: normalize in place; returns the extended length or +0.0 when
/// the length is <= 0.0001 (vector set to +0.0, +0.0).
pub(crate) fn normalize(v: &mut [u32; 2], p: Precision) -> Result<Binary> {
    let x = f(v[0])?;
    let y = f(v[1])?;
    let length = x.mul(&x, p)?.add(&y.mul(&y, p)?, p)?.sqrt(p)?;
    if length.compare(&f(EPSILON)?) != std::cmp::Ordering::Greater {
        *v = [0, 0];
        return f(ZERO);
    }
    let inverse = f(ONE)?.div(&length, p)?;
    v[0] = st(&inverse.mul(&x, p)?)?;
    v[1] = st(&inverse.mul(&y, p)?)?;
    Ok(length)
}
/// 0x5522B0: facing +0x44/+0x48, orientation cache +0x54 cleared.
pub(crate) fn set_facing(context: &mut CollisionContext, v: [u32; 2]) {
    context.words[17] = v[0];
    context.words[18] = v[1];
    context.words[21] = 0;
}
/// Descriptor byte +0x31 (word 12, bits 8..15).
pub(crate) fn set_byte_31(context: &mut CollisionContext, value: u8) {
    context.words[12] = (context.words[12] & !0xff00) | (u32::from(value) << 8);
}
pub(crate) fn source(context: &CollisionContext) -> Option<Object> {
    (context.words[7] != 0).then_some(Object(context.words[7]))
}

/// 0x52CF40: remove leading kind-0 nodes (the low-path waypoints of a previous
/// solve); each removal marks the path dirty. Counts are untouched.
pub fn pop_front_kind0(path: &mut OwnedPath) {
    while path.nodes.first().is_some_and(|node| node.words[4] == 0) {
        path.nodes.remove(0);
        path.dirty = true;
    }
}
/// 0x52D180 push_front with 0x52D000's copy and kind counting.
pub(crate) fn push_front(path: &mut OwnedPath, node: PathNode) -> Result<()> {
    path.nodes.try_reserve(1)?;
    path.nodes.insert(0, node);
    count(path, node);
    Ok(())
}
/// 0x52D1E0 push_back.
pub(crate) fn push_back(path: &mut OwnedPath, node: PathNode) -> Result<()> {
    path.nodes.try_reserve(1)?;
    path.nodes.push(node);
    count(path, node);
    Ok(())
}
fn count(path: &mut OwnedPath, node: PathNode) {
    match node.words[4] {
        1 => path.kind1_count = path.kind1_count.wrapping_add(1),
        2 => path.kind2_count = path.kind2_count.wrapping_add(1),
        _ => {}
    }
    path.dirty = true;
}
/// 0x52CA90 node constructor.
pub(crate) fn node(position: [u32; 2], direction: [u32; 2], kind: u32) -> PathNode {
    PathNode {
        words: [
            position[0],
            position[1],
            direction[0],
            direction[1],
            kind,
            0,
            position[0],
            position[1],
        ],
    }
}

/// Complete 0x53C170 `KKC_LowPathSolver::Solve(path, start, goal, context)`
/// without its PathLog diagnostics. Effects made before an error remain
/// (non-transactional); callers must not retry automatically.
pub fn solve<W: LowPathWorld>(
    solver: &mut LowPathSolver,
    env: &mut Environment<'_>,
    request: Request,
    path: &mut OwnedPath,
    context: &mut CollisionContext,
    world: &mut W,
) -> Result<Outcome> {
    let p = env.options.precision;
    solver.window.place(env.grid, request.start_bits, p)?;
    solver.goal.path_linked = true;
    let cell_size = f(env.grid.cell_size_bits)?;
    let cells = |window: &Window, bits: [u32; 2]| -> Result<[i32; 2]> {
        // tmp = (float)(1.0 / (1 << sub)); divisor = tmp * cell size.
        let unit = Binary::from_i32(1i32.wrapping_shl(window.sub_shift & 31));
        let inverse = f(st(&f(ONE)?.div(&unit, p)?)?)?;
        let y = f(bits[1])?
            .sub(&f(window.origin_bits[1])?, p)?
            .div(&inverse.mul(&cell_size, p)?, p)?;
        let y = ftol(&y)?;
        let x = f(bits[0])?
            .sub(&f(window.origin_bits[0])?, p)?
            .div(&inverse.mul(&cell_size, p)?, p)?;
        Ok([ftol(&x)?, y])
    };
    solver.goal.start_cell = cells(&solver.window, request.start_bits)?;
    solver.goal.goal_cell = cells(&solver.window, request.goal_bits)?;
    solver.goal.start_bits = request.start_bits;
    solver.goal.goal_bits = request.goal_bits;
    solver.goal.context_linked = true;
    let mut expanded = Vec::new();
    let status = search::run(solver, env, path, context, &mut expanded, world)?;
    if let Some(stats) = env.stats.as_deref_mut() {
        stats.record(status, solver.queue_remaining as i32, solver.goal.examined);
    }
    Ok(Outcome { status, expanded })
}
