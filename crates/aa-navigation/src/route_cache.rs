//! Native SAI route cache, the cached-route mode (flag 1) of region search
//! 0x53F320/0x53EDE0, and region path cost 0x53E5B0.
//!
//! The cache is the object at [[0xB3652C]+0x40]: `{count, rows}`, where row `i`
//! is `{size, bytes}` and covers the pairs `(i, j)`, `j > i`, at byte `j-i-1`
//! (native address `[rows + i*8 + 4] - i + j - 1`). Each byte is the index, in
//! region `i`'s ordered adjacency, of the next hop from the smaller region `i`
//! towards the larger region `j`. 0xFF means "unknown" (0x53F0A0 fill and the
//! 0x53E610 miss test); 0xFE is only special in 0x53E920's first pair check.
//!
//! Lifecycle (the routines below are executed by the oracle
//! `research/phase8-movement/native_route_cache.py`; their callers are static
//! evidence recorded in `native-route-cache-findings.json`):
//! - create: 0x7ABC30 (manager region finalisation, called by 0x7A0D40 just
//!   before the SAI readiness byte [0xB36528]+0x20 is set) destroys any old
//!   object, allocates a new `{0,0}` object (0x79E130) and builds it with
//!   0x53F0A0 from the manager's region count [+0x18] => [`RouteCache::build`];
//! - reset: 0x5D8560, the `ResetPathPlanner` console command, frees the rows
//!   and rebuilds them from the CURRENT region count => [`RouteCache::reset`];
//! - destroy: the tail of manager clear 0x7AC9A0 (0x7AAC90 + free + null),
//!   reached from the destructor 0x7ACE50 and from topology load 0x7ACB00
//!   => drop the cache (the caller's `Option<RouteCache>` becomes `None`).
//!
//! Entries are written only by 0x53F120 during flag-1 searches; nothing else
//! invalidates them (no epoch, topology or region-count check), so a cached
//! route can be a stale tree path, exactly as in the original.
//!
//! Native undefined behaviour (out-of-range hop bytes, pairs outside the cache,
//! a null cache, non-terminating walks) is an explicit [`CacheError`], never a
//! guessed route. As with flag 0, an error leaves the earlier search effects
//! (epoch, marks, busy, output link, cache writes) in place: no retry is safe.
use crate::region_search::{self, Mode, Outcome, RegionGraph, Request, Result, State};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 0x53F0A0 fill byte; 0x53E610 treats every other byte as a cached route.
pub const UNKNOWN_HOP: u8 = 0xFF;
/// 0x53E920 returns "no route" (empty list) when the start/goal pair holds it.
/// It is also what 0x7B90C0 returns for a missing adjacency.
pub const NO_ROUTE: u8 = 0xFE;
/// Binary32 [0x9FDC48] = -1.0, returned by 0x53E5B0 for an empty list.
pub const EMPTY_PATH_COST_BITS: u32 = 0xbf80_0000;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CacheError {
    #[error("the SAI manager has no route cache ([0xB3652C]+0x40 is null)")]
    Missing,
    #[error("region pair ({0},{1}) lies outside the route cache rows")]
    PairOutOfRange(u32, u32),
    #[error("cached hop byte {index:#04x} of region {region} is not one of its {count} neighbours")]
    InvalidHop {
        region: u32,
        index: u8,
        count: usize,
    },
    #[error("0x53EAC0 called with equal regions {0} reads the region array at index -1")]
    EqualHopPair(u32),
    #[error("0x53F120 would write the diagonal byte of region {0} (outside its row)")]
    Diagonal(u32),
    #[error("cached route walk from ({0},{1}) never terminates in the original")]
    Cycle(u32, u32),
    #[error("0x53F120 for region {0} reads a pair it is rewriting")]
    SelfOverlap(u32),
}
fn err(e: CacheError) -> region_search::Error {
    region_search::Error::RouteCache(e)
}

/// Caller-owned native route cache (the manager's +0x40 object).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RouteCache {
    /// Native rows array; `rows.len()` is the object's count word.
    rows: Vec<Vec<u8>>,
}

impl RouteCache {
    /// 0x7ABC30 creation tail: a new `{0,0}` object filled by 0x53F0A0 from
    /// the manager's region count (signed: counts below 2 build no rows).
    pub fn build(region_count: usize) -> Result<Self> {
        let mut cache = Self::default();
        cache.fill(region_count)?;
        Ok(cache)
    }
    /// 0x5D8560 `ResetPathPlanner`: rows are released, then rebuilt by 0x53F0A0
    /// from the manager's current region count, which may differ from the
    /// count the cache was first built with.
    pub fn reset(&mut self, region_count: usize) -> Result<()> {
        self.rows = Vec::new();
        self.fill(region_count)
    }
    /// 0x53F0A0 (+0x53ED50): n-1 rows for n regions, row i of n-1-i bytes, all 0xFF.
    /// No native size cap exists beyond the signed 32-bit region count.
    fn fill(&mut self, region_count: usize) -> Result<()> {
        if region_count <= 1 {
            return Ok(());
        }
        let count = region_count - 1;
        let mut rows = Vec::new();
        rows.try_reserve_exact(count)?;
        for i in 0..count {
            let mut row = Vec::new();
            row.try_reserve_exact(count - i)?;
            row.resize(count - i, UNKNOWN_HOP);
            rows.push(row);
        }
        self.rows = rows;
        Ok(())
    }
    /// Native count word: region count - 1 when built for more than one region.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
    /// Native row bytes, row 0 first (fixture order).
    pub fn rows(&self) -> &[Vec<u8>] {
        &self.rows
    }
    /// All row bytes concatenated in row order.
    pub fn bytes(&self) -> Vec<u8> {
        self.rows.concat()
    }
    fn slot(&self, low: u32, high: u32) -> std::result::Result<(usize, usize), CacheError> {
        let (i, j) = (low as usize, high as usize);
        if low < high && i < self.rows.len() && j - i - 1 < self.rows[i].len() {
            Ok((i, j - i - 1))
        } else {
            Err(CacheError::PairOutOfRange(low, high))
        }
    }
    /// Byte `[rows + low*8 + 4] - low + high - 1` for `low < high`.
    fn ordered(&self, low: u32, high: u32) -> Result<u8> {
        let (i, k) = self.slot(low, high).map_err(err)?;
        Ok(self.rows[i][k])
    }
    fn set(&mut self, low: u32, high: u32, value: u8) -> Result<()> {
        if low == high {
            return Err(err(CacheError::Diagonal(low)));
        }
        let (i, k) = self.slot(low, high).map_err(err)?;
        self.rows[i][k] = value;
        Ok(())
    }
    /// The byte for an unordered region pair (min, max), as 0x53E610 reads it.
    pub fn entry(&self, a: u32, b: u32) -> Result<u8> {
        self.ordered(a.min(b), a.max(b))
    }
}

/// `**([region+0x74] + index*4)`: the region id of an adjacency slot.
fn neighbour(graph: &RegionGraph, region: u32, index: u8) -> Result<u32> {
    let n = graph.node(region)?;
    n.neighbors.get(index as usize).copied().ok_or_else(|| {
        err(CacheError::InvalidHop {
            region,
            index,
            count: n.neighbors.len(),
        })
    })
}
/// 0x7B90C0: first adjacency slot of `region` holding `target`, else 0xFE.
fn adjacency_index(graph: &RegionGraph, region: u32, target: u32) -> Result<u8> {
    Ok(graph
        .node(region)?
        .neighbors
        .iter()
        .position(|&n| n == target)
        .map(|i| i as u8)
        .unwrap_or(NO_ROUTE))
}

/// 0x53EAC0: hop byte at `x` towards `y`. For `x > y` it walks from `y`
/// towards `x` along (y', x) entries; overshooting returns the (x, y') entry,
/// landing on `x` returns `x`'s adjacency slot of the last region walked.
/// `watch` is the region whose pairs the calling 0x53F120 is rewriting.
fn hop(cache: &RouteCache, graph: &RegionGraph, x: u32, y: u32, watch: Option<u32>) -> Result<u8> {
    let read = |low: u32, high: u32| -> Result<u8> {
        if watch.is_some_and(|w| w == low || w == high) {
            return Err(err(CacheError::SelfOverlap(x)));
        }
        cache.ordered(low, high)
    };
    if x < y {
        return read(x, y);
    }
    let mut last = None;
    let mut y = y;
    if y < x {
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(y) {
                return Err(err(CacheError::Cycle(y, x)));
            }
            last = Some(y);
            let index = read(y, x)?;
            y = neighbour(graph, y, index)?;
            if y >= x {
                break;
            }
        }
    }
    if y != x {
        return read(x, y);
    }
    let last = last.ok_or_else(|| err(CacheError::EqualHopPair(x)))?;
    adjacency_index(graph, x, last)
}

/// 0x5D8560 `ResetPathPlanner` on the manager's cache pointer. The original
/// dereferences it without a null check, so `None` is an explicit error.
pub fn reset_path_planner(cache: Option<&mut RouteCache>, region_count: usize) -> Result<()> {
    cache
        .ok_or_else(|| err(CacheError::Missing))?
        .reset(region_count)
}

/// 0x53E610 with strategy+0x0C set. A null cache is read only for start != goal.
pub(crate) fn early(cache: Option<&RouteCache>, start: u32, goal: u32) -> Result<bool> {
    if start == goal {
        return Ok(true);
    }
    let cache = cache.ok_or_else(|| err(CacheError::Missing))?;
    Ok(cache.entry(start, goal)? != UNKNOWN_HOP)
}

/// 0x53F120, called by mark 0x53F230 when flag 1 is set and `from != to`.
/// Every pair (x, to) for x on the cached chain start..from receives the hop
/// towards `to`, then the (from, to) pair itself.
pub(crate) fn record(
    cache: Option<&mut RouteCache>,
    graph: &RegionGraph,
    start: u32,
    from: u32,
    to: u32,
) -> Result<()> {
    let cache = cache.ok_or_else(|| err(CacheError::Missing))?;
    // Index, in `to`'s adjacency, of `from` (0xFE when absent).
    let back = adjacency_index(graph, to, from)?;
    graph.node(from)?;
    let mut cur = start;
    let mut seen = HashSet::new();
    while cur != from {
        if !seen.insert(cur) {
            return Err(err(CacheError::Cycle(cur, from)));
        }
        if cur < to {
            let value = hop(cache, graph, cur, from, Some(to))?;
            cache.set(cur, to, value)?;
        } else {
            cache.set(to, cur, back)?;
        }
        let index = hop(cache, graph, cur, from, Some(to))?;
        cur = neighbour(graph, cur, index)?;
    }
    if from < to {
        let value = adjacency_index(graph, from, to)?;
        cache.set(from, to, value)
    } else {
        cache.set(to, from, back)
    }
}

/// 0x53E920: the output list rebuilt from the cache. It is cleared first; the
/// smaller end advances each step, start-side regions in order at the front,
/// goal-side regions in reverse order at the back. Returns false (empty list)
/// when the start/goal pair holds [`NO_ROUTE`].
pub(crate) fn build_path(
    cache: Option<&RouteCache>,
    graph: &RegionGraph,
    start: u32,
    goal: u32,
    path: &mut Vec<u32>,
) -> Result<bool> {
    path.clear();
    if start == goal {
        path.try_reserve(1)?;
        path.push(start);
        return Ok(true);
    }
    let cache = cache.ok_or_else(|| err(CacheError::Missing))?;
    if cache.entry(start, goal)? == NO_ROUTE {
        return Ok(false);
    }
    let (mut a, mut b) = (start, goal);
    let (mut front, mut back) = (Vec::new(), Vec::new());
    let mut seen = HashSet::new();
    while a != b {
        if !seen.insert((a, b)) {
            return Err(err(CacheError::Cycle(a, b)));
        }
        if a < b {
            front.try_reserve(1)?;
            front.push(a);
            a = neighbour(graph, a, cache.ordered(a, b)?)?;
        } else {
            back.try_reserve(1)?;
            back.push(b);
            b = neighbour(graph, b, cache.ordered(b, a)?)?;
        }
    }
    path.try_reserve_exact(front.len() + back.len() + 1)?;
    path.extend_from_slice(&front);
    path.push(a);
    path.extend(back.iter().rev());
    Ok(true)
}

/// Original 0x53F320 with flag 1 ("cached route"): the 0x53EDE0 engine of
/// [`region_search::search`] with the cached early test 0x53E610, cache-writing
/// mark 0x53F230/0x53F120 and cache finaliser 0x53EBD0/0x53E920.
///
/// Status is as in flag 0. A cached pair (any byte but 0xFF) returns status 1
/// without queue work; a pair holding [`NO_ROUTE`] returns status 1 with an
/// empty `path`. Mismatched categories, busy and not-ready return 0 before the
/// cache is read; start == goal never reads it. `cache` is `None` exactly
/// when the native manager pointer is null (other reads are then an error).
pub fn search_cached(
    graph: &mut RegionGraph,
    request: Request,
    state: &mut State,
    cache: Option<&mut RouteCache>,
    path: &mut Vec<u32>,
) -> Result<Outcome> {
    region_search::run(graph, request, state, path, Mode::Cached(cache))
}

/// Original 0x53E5B0 (fastcall list): binary32 bits of the summed region path
/// cost. Empty: [`EMPTY_PATH_COST_BITS`]. Otherwise, from +0.0, for each
/// consecutive pair the 0x7BB420 distance (node cache, else 0x7B9A60 geometry,
/// ECX = region of the earlier id, argument = the later id) is added at the
/// x87 precision `precision` and stored to binary32. A single region costs +0.0
/// and is not looked up. Non-finite inputs and binary32 overflow (native +inf
/// or NaN) are explicit numeric errors.
pub fn path_cost(graph: &RegionGraph, path: &[u32], precision: Precision) -> Result<u32> {
    if path.is_empty() {
        return Ok(EMPTY_PATH_COST_BITS);
    }
    let mut local = 0u32;
    for pair in path.windows(2) {
        let distance = graph.distance(pair[0], pair[1], precision)?;
        local = distance
            .add(&Binary::from_f32_bits(local)?, precision)?
            .store_f32()?;
    }
    Ok(local)
}

/// Result of [`route_cost`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteCost {
    pub outcome: Outcome,
    /// The released native list (0x5C2CD0), returned for inspection.
    pub path: Vec<u32>,
    /// 0x53E5B0 result after the 0x7A1BDF binary32 store (unchanged bits).
    pub cost_bits: u32,
}

/// The region-route part of 0x7A1B30 (0x7A1BA3..0x7A1BEC): a fresh empty list,
/// 0x53F320(list, start, goal, 1) on the persistent SAI solver
/// [[0xB36534]+0x234] (built by 0x53F290: queue limit 0x2000, i.e.
/// `queue_limit: Some(NATIVE_QUEUE_LIMIT)` for parity), 0x53E5B0 on the list,
/// then release. The search status is ignored by the original, as here.
pub fn route_cost(
    graph: &mut RegionGraph,
    request: Request,
    state: &mut State,
    cache: Option<&mut RouteCache>,
) -> Result<RouteCost> {
    let mut path = Vec::new();
    let outcome = search_cached(graph, request, state, cache, &mut path)?;
    let cost_bits = path_cost(graph, &path, request.precision)?;
    Ok(RouteCost {
        outcome,
        path,
        cost_bits,
    })
}
