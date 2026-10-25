//! ZoneSolver flood-fill paths (zones that are not fast-built: types 0 ZOS,
//! 3, 4 ZOC, 5 ZOG, 7, 8, and any zone while SVar
//! `ZoneBuilderFastDetectionZones` is clear).
//!
//! * Add / incremental move: `0x540860` slow branch, then the ZoneSolver
//!   (GWorld+0x248, vtable `0xA36954`) run `0x5402E0` with functor
//!   `0xA36910` and the visit `0x53F9F0`: a Dijkstra flood from the zone tile
//!   over a binary-heap open list (`0x53CDF0` class, push `0x53D2A0` ->
//!   `0x53D090`, pop `0x53D150`), step cost `0x53C5C0` (tile size, times
//!   `0xA363AC` = binary32 sqrt 2 diagonally). A tile receives the zone when
//!   its cost is within the radius and the zone passes its blocker; it is
//!   expanded while `tile/2 + cost < radius`. With `moved`, tiles that had the
//!   zone are flooded through and the zone removed where it no longer reaches.
//! * Remove: `0x540590` slow branch, flood solver (GWorld+0x244, vtable
//!   `0xA36048`, FIFO through tile +0x24 links) run `0x4F4050` with functor
//!   `0xA368C0`: breadth-first over 8-connected tiles that hold the zone,
//!   removing it (`0x53F880` -> `0x54FA80`).
//! * Kingdom pass: `0x5406D0` slow branch, functor `0xA36870` (`0x53F610`):
//!   the same breadth-first walk revealing (`0x54F670`) or hiding
//!   (`0x54F690`) the zone's tiles for one kingdom.
//! * Effects pass: `0x540690` (from `0x5147C0`), functor `0xA36820`
//!   (`0x53F600` -> `0x54F7F0`).
//!
//! Neighbour iteration `0x554ED0` / `0x4F2000` / `0x4F2760` is ported with
//! the functor flags of these classes (slots +0x34/+0x38/+0x3C/+0x40, byte
//! +8 and the blocked test +0x1C).
//!
//! Shared GMap tile scratch (other solvers use the same words): visit stamp
//! +0x28 and arrival direction +0x2D are only read for tiles reached in the
//! same solve, so they are kept per solve; the stamp counter GMap+0x48
//! (`0x552440`) and the process-wide busy byte `0xB3D43D` are in
//! [`SolverState`]. Tile +0x24 is a float cost for the ZoneSolver and a FIFO
//! link (tile pointer) for the flood solver, and the ZoneSolver reads its
//! start tile's +0x24 without resetting it: [`Scratch`] keeps it per tile.
//! A link read back as a float is the native tile ADDRESS reinterpreted; the
//! port assumes the tile array lies below 0x3F800000 (the value is then a
//! positive binary32 < 1.0, which never passes the `cost + 1e-4 < scratch`
//! test with costs >= one tile). This is recorded in findings.json.
//!
//! Removed engine cap: the open list refuses pushes once it holds 0x1F9C
//! (8092) entries (`0x53CDF0` +0x10); [`SolverState::queue_limit`] keeps it as
//! an explicit option (`None` in production, `Some(0x1F9C)` to replay native).
use crate::fpu;
use crate::solver::{tile_add, tile_has_zone_entry, tile_remove};
use crate::tile::{ZoneMap, hide, reveal};
use crate::tile::ZoneWorld;
use crate::zone::{Zone, tile_actor_effects};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Native open-list limit of the ZoneSolver queue (`0x53CDF0`: 0x1F9C).
pub const NATIVE_QUEUE_LIMIT: usize = 0x1F9C;
/// Binary32 sqrt(2) at `0xA363AC` (diagonal step factor).
pub const DIAGONAL_BITS: u32 = 0x3FB5_04F3;
/// Binary32 1e-4 at `0x9FC0BC`.
pub const EPSILON_BITS: u32 = 0x38D1_B717;
/// FLT_MAX at `0xA29608` (unreachable cost).
pub const UNREACHED_BITS: u32 = 0x7F7F_FFFF;

/// `0xB322E0`: (dx, dy) per direction 0..=8.
const DELTA: [[i32; 2]; 9] = [[0, 0], [-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];
/// `0xAB7804` next, `0xAB7828` previous, `0xAB784C` next-but-one,
/// `0xAB7870` previous-but-one, `0xAB7894` opposite.
const NEXT: [u32; 9] = [0, 2, 3, 4, 5, 6, 7, 8, 1];
const PREVIOUS: [u32; 9] = [0, 8, 1, 2, 3, 4, 5, 6, 7];
const NEXT2: [u32; 9] = [0, 3, 4, 5, 6, 7, 8, 1, 2];
const PREVIOUS2: [u32; 9] = [0, 7, 8, 1, 2, 3, 4, 5, 6];
const OPPOSITE: [u32; 9] = [0, 5, 6, 7, 8, 1, 2, 3, 4];
/// `*0xB322D8` / `*0xB322DC` (indexed by dx/dy + 1) and `0xAB78D0`.
const DX_MASK: [u32; 3] = [1, 0, 4];
const DY_MASK: [u32; 3] = [2, 0, 8];
const MASK_DIRECTION: [u32; 16] = [0, 2, 8, 1, 6, 0, 7, 0, 4, 3, 0, 0, 5, 0, 0, 0];

/// Tile +0x24 scratch word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scratch {
    /// Binary32 bits (ZoneSolver costs; 0 after a FIFO reach).
    Bits(u32),
    /// FIFO link to another tile (a native tile pointer).
    Link(usize),
}

impl Default for Scratch {
    fn default() -> Self {
        Scratch::Bits(0)
    }
}

/// Shared solver state (see the module doc).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverState {
    /// `0xB3D43D`.
    pub busy: bool,
    /// GMap+0x48 visit stamp counter.
    pub stamp: u32,
    /// `0xB3D458`: the one-time result of the blocked-start assertion.
    pub warned: bool,
    /// Open-list limit (native `Some(0x1F9C)`).
    pub queue_limit: Option<usize>,
    /// Tile +0x24, per tile.
    pub scratch: Vec<Scratch>,
}

impl SolverState {
    pub fn new(tiles: usize, queue_limit: Option<usize>) -> Self {
        Self { busy: false, stamp: 0, warned: false, queue_limit, scratch: vec![Scratch::default(); tiles] }
    }

    /// `0x552440`: next stamp; at 0x7FFFFFFF every tile stamp is cleared and
    /// the counter restarts at 0 (tile stamps are solve-local here).
    fn advance(&mut self) {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp >= 0x7FFF_FFFF {
            self.stamp = 0;
        }
    }
}

/// Map reads of the flood paths.
pub trait FloodWorld: ZoneWorld {
    /// Tile +0x20 flags (bit 0 impassable, bit 8 terrain class tested by the
    /// ZoneSolver against the current tile).
    fn tile_flags(&mut self, tile: usize) -> Result<u32>;
    /// `0x45BC70`: the actor whose GTID is in tile +0x1C (0 when stale/none).
    fn tile_blocker(&mut self, tile: usize) -> Result<u32>;
    /// `0x855D30` assertion "!source_mapnode->IsSet(KKC_GTileFlags::E..."
    /// (ZoneSolver.cpp 0x26E) when a zone starts on an impassable tile; its
    /// result is kept in `0xB3D458` and stops later reports.
    fn assert_blocked_start(&mut self) -> Result<bool>;
}

/// ZoneSolver open-list entry `{x, y, parent(0), priority, cost}` (0x14 bytes).
#[derive(Clone, Copy, Debug)]
struct Entry {
    cell: [i32; 2],
    priority: u32,
    cost: u32,
}

fn f(bits: u32) -> Result<Binary> {
    fpu::load(bits)
}

/// The `fld dword` domain check of a value that is only compared.
fn finite(bits: u32) -> Result<()> {
    if f32::from_bits(bits).is_finite() { Ok(()) } else { Err(Error::NonFinite(bits)) }
}

/// The register arithmetic of the ZoneSolver visit `0x53F9F0` (PC53: exact
/// binary64, [`fpu::pc53`]; other precisions and the fallback: Binary).
struct Ops {
    p: Precision,
    tile_bits: u32,
    half: Binary,
}

impl Ops {
    fn pc53(&self) -> bool {
        self.p == Precision::Double53
    }

    /// `0x53C5C0`: `fstp(factor * tile + cost)`.
    fn step(&self, factor: u32, cost: u32) -> Result<u32> {
        use fpu::pc53;
        if self.pc53()
            && let Some(v) = (|| pc53::store(pc53::add(pc53::mul(pc53::load(factor)?, pc53::load(self.tile_bits)?)?, pc53::load(cost)?)?))()
        {
            return Ok(v);
        }
        fpu::f32_store(&fpu::add(&fpu::mul(&f(factor)?, &f(self.tile_bits)?, self.p)?, &f(cost)?, self.p)?)
    }

    /// `d + 1e-4 < old`.
    fn eps_less(&self, d: u32, old: u32) -> Result<bool> {
        use fpu::pc53;
        if self.pc53()
            && let Some(v) = (|| Some(pc53::add(pc53::load(d)?, pc53::load(EPSILON_BITS)?)? < pc53::load(old)?))()
        {
            return Ok(v);
        }
        Ok(fpu::less(&fpu::add(&f(d)?, &f(EPSILON_BITS)?, self.p)?, &f(old)?))
    }

    /// `tile / 2 + d < radius`.
    fn half_less(&self, d: u32, radius: u32) -> Result<bool> {
        use fpu::pc53;
        if self.pc53()
            && let Some(v) = (|| Some(pc53::add(pc53::half_tile(self.tile_bits)?, pc53::load(d)?)? < pc53::load(radius)?))()
        {
            return Ok(v);
        }
        Ok(fpu::less(&fpu::add(&self.half, &f(d)?, self.p)?, &f(radius)?))
    }

    /// `fstp(a + b)`.
    fn sum(&self, a: u32, b: u32) -> Result<u32> {
        use fpu::pc53;
        if self.pc53()
            && let Some(v) = (|| pc53::store(pc53::add(pc53::load(a)?, pc53::load(b)?)?))()
        {
            return Ok(v);
        }
        fpu::f32_store(&fpu::add(&f(a)?, &f(b)?, self.p)?)
    }
}

/// `0x53CDF0` binary heap (min-priority, native tie rules).
struct Queue {
    entries: Vec<Entry>,
    limit: Option<usize>,
}

impl Queue {
    /// `0x53D090`: refuse when full; sift up while `p <= parent` (ties move up).
    fn push(&mut self, e: Entry) -> Result<bool> {
        if self.limit.is_some_and(|l| l > 0 && self.entries.len() == l) {
            return Ok(false);
        }
        finite(e.priority)?;
        let mut i = self.entries.len();
        self.entries.push(e);
        while i > 0 {
            let parent = (i - 1) / 2;
            if fpu::compare_bits(e.priority, self.entries[parent].priority)? == std::cmp::Ordering::Greater {
                break;
            }
            self.entries[i] = self.entries[parent];
            i = parent;
        }
        self.entries[i] = e;
        Ok(true)
    }

    /// `0x53D150`: take the root; sift the last entry down choosing the left
    /// child only when strictly smaller than the right, stopping when the last
    /// entry is `<=` the chosen child.
    fn pop(&mut self) -> Result<Option<Entry>> {
        if self.entries.is_empty() {
            return Ok(None);
        }
        let out = self.entries[0];
        let n = self.entries.len() - 1;
        let last = self.entries[n];
        finite(last.priority)?;
        let mut hole = 0usize;
        let mut child = 1usize;
        if n > 1 {
            loop {
                let right = 2 * hole + 2;
                let pick = if right < n && fpu::less_bits(self.entries[child].priority, self.entries[right].priority)? {
                    child
                } else if right < n {
                    right
                } else {
                    child
                };
                if fpu::compare_bits(last.priority, self.entries[pick].priority)? != std::cmp::Ordering::Greater {
                    break;
                }
                self.entries[hole] = self.entries[pick];
                child = 2 * pick + 1;
                hole = pick;
                if child >= n {
                    break;
                }
            }
        }
        self.entries[hole] = last;
        self.entries.truncate(n);
        Ok(Some(out))
    }
}

fn linear(map: &ZoneMap, cell: [i32; 2]) -> Result<usize> {
    // Native pointer arithmetic without a bounds check: a row overflow lands
    // in the neighbouring row; outside the tile array the original reads
    // foreign memory (rejected here).
    let w = i64::from(map.geometry.width);
    let i = i64::from(cell[1]) * w + i64::from(cell[0]);
    if i < 0 || i >= map.tiles.len() as i64 {
        return Err(Error::TileOutside(i));
    }
    Ok(i as usize)
}

fn direction(from: [i32; 2], to: [i32; 2]) -> Result<u32> {
    let d = [to[0].wrapping_sub(from[0]), to[1].wrapping_sub(from[1])];
    if d.iter().any(|v| !(-1..=1).contains(v)) {
        return Err(Error::World("solver direction outside the 3x3 tables".into()));
    }
    Ok(MASK_DIRECTION[(DX_MASK[(d[0] + 1) as usize] | DY_MASK[(d[1] + 1) as usize]) as usize])
}

/// Functor flags used by `0x554ED0` / `0x4F2000`.
#[derive(Clone, Copy)]
struct Functor {
    /// +0x34: iterate all eight directions regardless of the arrival.
    all_dirs: bool,
    /// +0x38: corner test for diagonal steps.
    corners: bool,
    /// +0x3C: three directions ahead after an orthogonal arrival (else five);
    /// also admits a diagonal with one blocked corner.
    three_ahead: bool,
    /// +0x40: bounds-checked neighbours, no impassable-origin test.
    bounded: bool,
    /// byte +8: skip the blocked tests.
    skip_blocked: bool,
}

/// Per-solve tile state (stamp == current, +0x2D arrival direction).
#[derive(Default)]
struct Visits {
    came: HashMap<usize, u32, crate::set::FxBuild>,
}

/// The blocked test (functor +0x1C) of each functor class.
enum Blocked<'z> {
    /// `0x53C590` (ZoneSolver functor, +4 = 0): tile +0x20 bit 0.
    Impassable,
    /// `0x53F5B0`: the tile does not hold the zone (`!0x54EF40`).
    LacksZone(&'z Zone),
}

fn blocked<W: FloodWorld + ?Sized>(map: &ZoneMap, w: &mut W, b: &Blocked<'_>, t: usize) -> Result<bool> {
    match b {
        Blocked::Impassable => Ok(w.tile_flags(t)? & 1 != 0),
        Blocked::LacksZone(z) => Ok(!tile_has_zone_entry(map, t, z)?),
    }
}

fn offset(c: [i32; 2], dir: u32) -> [i32; 2] {
    let d = DELTA[dir as usize];
    [c[0].wrapping_add(d[0]), c[1].wrapping_add(d[1])]
}

/// `0x4F2000`.
fn neighbour_valid<W: FloodWorld + ?Sized>(
    map: &ZoneMap,
    w: &mut W,
    fu: Functor,
    b: &Blocked<'_>,
    origin: [i32; 2],
    came: u32,
    dir: u32,
) -> Result<bool> {
    if OPPOSITE[came as usize] == dir {
        return Ok(false);
    }
    let n = offset(origin, dir);
    if fu.bounded {
        if n[0] < 0 || n[0] >= map.geometry.width || n[1] < 0 || n[1] >= map.geometry.height {
            return Ok(false);
        }
    } else if w.tile_flags(linear(map, n)?)? & 1 != 0 {
        return Ok(false);
    }
    if fu.skip_blocked {
        return Ok(true);
    }
    if blocked(map, w, b, linear(map, n)?)? {
        return Ok(false);
    }
    if !fu.corners || dir & 1 == 0 {
        return Ok(true);
    }
    let mut count = 0;
    if blocked(map, w, b, linear(map, offset(origin, PREVIOUS[dir as usize]))?)? {
        count += 1;
    }
    if blocked(map, w, b, linear(map, offset(origin, NEXT[dir as usize]))?)? {
        count += 1;
    }
    Ok(match count {
        0 => true,
        1 => fu.three_ahead,
        _ => false,
    })
}

/// `0x554ED0` + `0x4F2760`: the valid directions from `origin`, in order.
fn neighbours<W: FloodWorld + ?Sized>(
    map: &ZoneMap,
    w: &mut W,
    fu: Functor,
    b: &Blocked<'_>,
    visits: &Visits,
    origin: [i32; 2],
) -> Result<Vec<u32>> {
    let t = linear(map, origin)?;
    let came = visits.came.get(&t).copied().unwrap_or(0);
    let mut out = Vec::new();
    if !fu.bounded && w.tile_flags(t)? & 1 != 0 {
        return Ok(out);
    }
    let key = if fu.all_dirs { 0 } else { came };
    let (mut dir, end) = match key {
        0 => (1, 8),
        1 | 3 | 5 | 7 => (PREVIOUS2[key as usize], NEXT2[key as usize]),
        2 | 4 | 6 | 8 => {
            if fu.three_ahead {
                (PREVIOUS[key as usize], NEXT[key as usize])
            } else {
                (PREVIOUS2[key as usize], NEXT2[key as usize])
            }
        }
        _ => (0, 0),
    };
    if dir == 0 {
        return Ok(out);
    }
    let first = dir;
    if neighbour_valid(map, w, fu, b, origin, came, dir)? {
        out.push(dir);
    }
    // 0x4F2760 repeatedly: advance until valid, the end, or a full turn.
    let start_of_scan = first;
    loop {
        if dir == end {
            break;
        }
        dir = NEXT[dir as usize];
        if dir == start_of_scan {
            break;
        }
        if neighbour_valid(map, w, fu, b, origin, came, dir)? {
            out.push(dir);
        }
    }
    Ok(out)
}

/// `0x540860` slow branch + `0x5402E0`: flood the zone in from `(x, y)`.
#[allow(clippy::too_many_arguments)]
pub fn add<W: FloodWorld + ?Sized>(
    map: &mut ZoneMap,
    st: &mut SolverState,
    w: &mut W,
    zone: &mut Zone,
    x: i32,
    y: i32,
    radius_bits: u32,
    moved: bool,
    p: Precision,
) -> Result<u32> {
    zone.x = x;
    zone.y = y;
    zone.radius_bits = radius_bits;
    let skip_blocked = zone.fast_eligible();
    let t0 = linear(map, [x, y])?;
    let flags0 = w.tile_flags(t0)?;
    if !st.warned && flags0 & 1 != 0 {
        st.warned = w.assert_blocked_start()?;
    }
    if flags0 & 1 != 0 {
        return Ok(0);
    }
    if !tile_has_zone_entry(map, t0, zone)? {
        tile_add(map, w, zone, t0)?;
    }
    run_dijkstra(map, st, w, zone, moved, skip_blocked, p)
}

/// `0x5402E0` with functor `0xA36910` (returns the native EAX).
fn run_dijkstra<W: FloodWorld + ?Sized>(
    map: &mut ZoneMap,
    st: &mut SolverState,
    w: &mut W,
    zone: &mut Zone,
    moved: bool,
    skip_blocked: bool,
    p: Precision,
) -> Result<u32> {
    if st.busy {
        st.busy = false;
        return Ok(0);
    }
    reentry_guard(map, st)?;
    let mut queue = Queue { entries: Vec::new(), limit: st.queue_limit };
    st.advance();
    st.busy = true;
    let fu = Functor { all_dirs: false, corners: true, three_ahead: false, bounded: false, skip_blocked };
    let start = [zone.x, zone.y];
    let ts = linear(map, start)?;
    let mut visits = Visits::default();
    visits.came.insert(ts, 0);
    let zero = fpu::f32_store(&fpu::add(&f(0)?, &f(0)?, p)?)?;
    if !queue.push(Entry { cell: start, priority: zero, cost: 0 })? {
        st.busy = false;
        return Ok(0);
    }
    finite(zone.radius_bits)?;
    let tile = f(map.geometry.tile_size_bits)?;
    let ops = Ops { p, tile_bits: map.geometry.tile_size_bits, half: fpu::mul(&tile, &f(fpu::HALF_BITS)?, p)? };
    while let Some(node) = queue.pop()? {
        let cur = node.cell;
        let dirs = neighbours(map, w, fu, &Blocked::Impassable, &visits, cur)?;
        for dir in dirs {
            let n = offset(cur, dir);
            let nt = linear(map, n)?;
            visit_dijkstra(map, st, w, zone, &mut queue, &mut visits, node, cur, n, nt, moved, &ops)?;
        }
    }
    st.busy = false;
    Ok(1)
}

/// `0x53F9F0` for neighbour `n` (tile `nt`) of the popped entry `node`.
#[allow(clippy::too_many_arguments)]
fn visit_dijkstra<W: FloodWorld + ?Sized>(
    map: &mut ZoneMap,
    st: &mut SolverState,
    w: &mut W,
    zone: &mut Zone,
    queue: &mut Queue,
    visits: &mut Visits,
    node: Entry,
    cur: [i32; 2],
    n: [i32; 2],
    nt: usize,
    moved: bool,
    ops: &Ops,
) -> Result<()> {
    let radius = zone.radius_bits;
    // Functor +0x30 (0x53F970): FLT_MAX priority stays unreached; else
    // 0x53C5C0: (diagonal ? sqrt2 : 1) * tile + entry cost (+0x10).
    let mut d = if node.priority == UNREACHED_BITS {
        UNREACHED_BITS
    } else {
        let dir = direction(cur, n)?;
        let factor = if dir & 1 != 0 { DIAGONAL_BITS } else { fpu::ONE_BITS };
        ops.step(factor, node.cost)?
    };
    let mut had_zone = false; // [esp+0x12]
    let mut within_before = false; // [esp+0x13]
    let first_reach = direction(cur, n)?;
    if let std::collections::hash_map::Entry::Vacant(v) = visits.came.entry(nt) {
        v.insert(first_reach);
        if moved {
            had_zone = tile_has_zone_entry(map, nt, zone)?;
        }
    } else {
        if fpu::less_bits(radius, d)? {
            return Ok(());
        }
        let old = scratch_bits(st.scratch[nt]);
        if !ops.eps_less(d, old)? {
            return Ok(());
        }
        within_before = !fpu::less_bits(radius, old)?;
    }
    st.scratch[nt] = Scratch::Bits(d);
    let mut pass = true;
    if d == UNREACHED_BITS {
        pass = false;
    } else {
        if !zone.fast_eligible() {
            let ct = linear(map, cur)?;
            if ((w.tile_flags(nt)? >> 8) ^ (w.tile_flags(ct)? >> 8)) & 1 != 0 {
                pass = false;
            }
        }
        if pass {
            if fpu::less_bits(radius, d)? {
                pass = false;
            } else {
                let blocker = w.tile_blocker(nt)?;
                if !zone.passes(blocker) {
                    pass = false;
                }
            }
        }
    }
    let mut reached = pass;
    if pass && direction(cur, n)? & 1 != 0 {
        let dir = direction(cur, n)? as usize;
        let side = |c: [i32; 2]| -> Option<usize> {
            (c[0] >= 0 && c[0] < map.geometry.width && c[1] >= 0 && c[1] < map.geometry.height)
                .then(|| c[1] as usize * map.geometry.width as usize + c[0] as usize)
        };
        let t1 = side(offset(cur, NEXT[dir]));
        let t2 = side(offset(cur, PREVIOUS[dir]));
        if let (Some(t1), Some(t2)) = (t1, t2) {
            let b1 = w.tile_blocker(t1)?;
            if !zone.passes(b1) {
                let b2 = w.tile_blocker(t2)?;
                if !zone.passes(b2) {
                    reached = false;
                }
            }
        }
    }
    if reached {
        if !(had_zone || within_before) {
            tile_add(map, w, zone, nt)?;
        }
    } else {
        d = UNREACHED_BITS;
        st.scratch[nt] = Scratch::Bits(UNREACHED_BITS);
        if !had_zone {
            return Ok(());
        }
        tile_remove(map, w, zone, nt)?;
    }
    if !had_zone {
        if !fpu::less_bits(d, UNREACHED_BITS)? {
            return Ok(());
        }
        if !ops.half_less(d, radius)? {
            return Ok(());
        }
    }
    let priority = ops.sum(d, 0)?;
    queue.push(Entry { cell: n, priority, cost: d })?;
    Ok(())
}

fn reentry_guard(map: &ZoneMap, st: &SolverState) -> Result<()> {
    if st.scratch.len() != map.tiles.len() {
        return Err(Error::World("re-entrant zone solve while the solver state is in use".into()));
    }
    Ok(())
}

fn scratch_bits(s: Scratch) -> u32 {
    match s {
        Scratch::Bits(b) => b,
        // A tile address below 0x3F800000 read as binary32: positive, < 1.0.
        // Only `cost + 1e-4 < value` reads it, and costs here are >= one tile.
        Scratch::Link(_) => 0x0080_0000,
    }
}

/// Which FIFO functor walks the zone's tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FifoAction {
    /// `0xA368C0` / `0x53F880`: remove the zone (`0x54FA80`).
    Remove,
    /// `0xA36870` / `0x53F610`: reveal (`0x54F670`) or hide (`0x54F690`).
    Kingdom { kingdom: u32, set: bool },
    /// `0xA36820` / `0x53F600`: zone effects on the tile's actors (`0x54F7F0`).
    Effects,
}

fn fifo_apply<W: FloodWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, zone: &Zone, action: FifoAction, t: usize) -> Result<()> {
    match action {
        FifoAction::Remove => tile_remove(map, w, zone, t),
        FifoAction::Kingdom { kingdom, set } => {
            if set {
                reveal(map, w, t, kingdom, false, true)
            } else {
                hide(map, w, t, kingdom)
            }
        }
        FifoAction::Effects => tile_actor_effects(map, w, zone, t),
    }
}

/// `0x4F4050` with the flood solver (GWorld+0x244) and a zone functor: from
/// the zone tile, breadth-first through 8-connected tiles that hold the zone
/// (functor +0x1C `0x53F5B0`), applying the action to each reached tile and
/// to the start tile when it is popped (`0x53F890`). Returns the native EAX.
pub fn walk<W: FloodWorld + ?Sized>(
    map: &mut ZoneMap,
    st: &mut SolverState,
    w: &mut W,
    zone: &Zone,
    action: FifoAction,
) -> Result<u32> {
    if st.busy {
        st.busy = false;
        return Ok(0);
    }
    reentry_guard(map, st)?;
    st.advance();
    st.busy = true;
    let fu = Functor { all_dirs: false, corners: false, three_ahead: true, bounded: true, skip_blocked: false };
    let start = [zone.x, zone.y];
    let ts = linear(map, start)?;
    let mut visits = Visits::default();
    visits.came.insert(ts, 0);
    st.scratch[ts] = Scratch::Bits(0);
    let (mut head, mut tail): (Option<usize>, Option<usize>) = (None, None);
    let push = |st: &mut SolverState, head: &mut Option<usize>, tail: &mut Option<usize>, t: usize| {
        if let Some(prev) = *tail {
            st.scratch[prev] = Scratch::Link(t);
        }
        *tail = Some(t);
        if head.is_none() {
            *head = Some(t);
        }
    };
    push(st, &mut head, &mut tail, ts);
    let blocked_rule = Blocked::LacksZone(zone);
    while let Some(cur_t) = head {
        // 0x53F5D0 pop: follow the +0x24 link.
        head = match st.scratch[cur_t] {
            Scratch::Link(n) => Some(n),
            Scratch::Bits(0) => None,
            Scratch::Bits(b) => return Err(Error::World(format!("FIFO link holds a cost {b:#x}"))),
        };
        if head.is_none() {
            tail = None;
        }
        let w_ = map.geometry.width;
        let cur = [(cur_t as i32) % w_, (cur_t as i32) / w_];
        if cur == start {
            fifo_apply(map, w, zone, action, cur_t)?;
        }
        let dirs = neighbours(map, w, fu, &blocked_rule, &visits, cur)?;
        for dir in dirs {
            let n = offset(cur, dir);
            let nt = linear(map, n)?;
            // 0x53F8D0
            if visits.came.contains_key(&nt) {
                continue;
            }
            visits.came.insert(nt, direction(cur, n)?);
            st.scratch[nt] = Scratch::Bits(0);
            if blocked(map, w, &blocked_rule, nt)? {
                continue;
            }
            fifo_apply(map, w, zone, action, nt)?;
            push(st, &mut head, &mut tail, nt);
        }
    }
    st.busy = false;
    Ok(1)
}

/// `0x540590` slow branch.
pub fn remove<W: FloodWorld + ?Sized>(map: &mut ZoneMap, st: &mut SolverState, w: &mut W, zone: &Zone) -> Result<u32> {
    walk(map, st, w, zone, FifoAction::Remove)
}

/// `0x5406D0` slow branch for one kingdom.
pub fn kingdom_pass<W: FloodWorld + ?Sized>(
    map: &mut ZoneMap,
    st: &mut SolverState,
    w: &mut W,
    zone: &Zone,
    kingdom: u32,
    set: bool,
) -> Result<u32> {
    walk(map, st, w, zone, FifoAction::Kingdom { kingdom, set })
}

/// `0x540690`: effects pass of an active zone.
pub fn effects_pass<W: FloodWorld + ?Sized>(map: &mut ZoneMap, st: &mut SolverState, w: &mut W, zone: &Zone) -> Result<u32> {
    if !zone.active() {
        return Ok(0);
    }
    walk(map, st, w, zone, FifoAction::Effects)
}

