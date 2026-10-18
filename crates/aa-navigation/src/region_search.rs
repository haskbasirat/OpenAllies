//! Concrete native region graph search 0x53F320/0x53EDE0. [`search`] is the
//! uncached-route mode (flag 0); the cached-route mode (flag 1) shares this
//! engine and is exposed by [`crate::route_cache::search_cached`].
//! Topology, node categories and optional distance-cache values are caller data.
//!
//! [`State::busy`] is the process-wide solver byte 0xB3D43D. The original
//! shares it between every A*/flood core built on the same template (0x4F4050
//! water flood fill = `aa_spatial::water_areas::FloodSolver::busy`, 0x539650,
//! 0x53BE30, 0x53D8D0, this 0x53EDE0 in both modes, 0x5402E0, 0x960FB0). A
//! caller reproducing native interlocks must use one shared busy flag for all.
use crate::route_cache::{self, RouteCache};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashSet},
};

pub const NATIVE_QUEUE_LIMIT: usize = 0x2000;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("region-search allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error("invalid native region graph node {0}")]
    InvalidNode(u32),
    #[error("region adjacency exceeds its native byte count")]
    AdjacencyCount,
    #[error("native goal predecessor chain is missing or cyclic at region {0}")]
    InvalidPredecessor(u32),
    #[error(transparent)]
    RouteCache(#[from] route_cache::CacheError),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionNode {
    pub position_bits: [u32; 2],
    /// Native7B8FB0 result from the region category data, including -1 as bits.
    pub category: u32,
    /// Original adjacency order, not a sorted or deduplicated edge set.
    pub neighbors: Vec<u32>,
    /// Original node+5c hash-table values. Missing pairs use native geometry.
    pub distance_cache: BTreeMap<u32, u32>,
    pub visit_epoch: u8,
    pub parent_neighbor: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionGraph {
    pub nodes: Vec<RegionNode>,
    pub epoch: u8,
    pub ready: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct State {
    pub busy: bool,
    pub output_linked: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Request {
    pub start: u32,
    pub goal: u32,
    pub precision: Precision,
    pub queue_limit: Option<usize>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEntry {
    pub node: u32,
    pub depth: i32,
    pub priority_bits: u32,
    pub cost_bits: u32,
}
impl QueueEntry {
    pub fn words(self) -> [u32; 4] {
        [
            self.node,
            self.depth as u32,
            self.priority_bits,
            self.cost_bits,
        ]
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub status: i32,
    pub popped: Vec<QueueEntry>,
    pub discovered: Vec<[u32; 2]>,
    pub queue_remaining: usize,
}

/// Native16-byte region queue53F350/53F410. Its payload and comparisons were
/// independently recovered; equality rises on push and selects the right child
/// on pop. The tail stops moving when equal to the selected child.
struct Queue {
    entries: Vec<QueueEntry>,
    limit: Option<usize>,
}
impl Queue {
    fn push(&mut self, entry: QueueEntry) -> Result<bool> {
        Binary::from_f32_bits(entry.priority_bits)?;
        if self.limit.is_some_and(|n| n > 0 && self.entries.len() == n) {
            return Ok(false);
        }
        self.entries.try_reserve(1)?;
        let mut index = self.entries.len();
        self.entries.push(entry);
        while index > 0 {
            let parent = (index - 1) / 2;
            if f32::from_bits(entry.priority_bits)
                > f32::from_bits(self.entries[parent].priority_bits)
            {
                break;
            }
            self.entries[index] = self.entries[parent];
            index = parent;
        }
        self.entries[index] = entry;
        Ok(true)
    }
    fn pop(&mut self) -> Option<QueueEntry> {
        let first = *self.entries.first()?;
        let last = self.entries.pop()?;
        if self.entries.is_empty() {
            return Some(first);
        }
        let mut index = 0;
        while index * 2 + 1 < self.entries.len() {
            let left = index * 2 + 1;
            let right = left + 1;
            let child = if right < self.entries.len()
                && f32::from_bits(self.entries[left].priority_bits)
                    >= f32::from_bits(self.entries[right].priority_bits)
            {
                right
            } else {
                left
            };
            if f32::from_bits(last.priority_bits)
                <= f32::from_bits(self.entries[child].priority_bits)
            {
                break;
            }
            self.entries[index] = self.entries[child];
            index = child;
        }
        self.entries[index] = last;
        Some(first)
    }
}
fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
impl RegionGraph {
    pub(crate) fn node(&self, id: u32) -> Result<&RegionNode> {
        self.nodes.get(id as usize).ok_or(Error::InvalidNode(id))
    }
    fn validate(&self) -> Result<()> {
        for (id, node) in self.nodes.iter().enumerate() {
            if node.neighbors.len() > 255 {
                return Err(Error::AdjacencyCount);
            }
            for &next in &node.neighbors {
                self.node(next)?;
            }
            if id > u32::MAX as usize {
                return Err(Error::InvalidNode(u32::MAX));
            }
        }
        Ok(())
    }
    fn next_epoch(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            for node in &mut self.nodes {
                node.parent_neighbor = 255;
            }
        }
    }
    fn visited(&self, id: u32) -> Result<bool> {
        let n = self.node(id)?;
        Ok(n.visit_epoch == self.epoch && n.parent_neighbor != 255)
    }
    fn mark(&mut self, from: u32, to: u32) -> Result<()> {
        let index = self
            .node(to)?
            .neighbors
            .iter()
            .position(|&n| n == from)
            .map(|v| v as u8)
            .unwrap_or(254);
        let epoch = self.epoch;
        let n = &mut self.nodes[to as usize];
        n.visit_epoch = epoch;
        n.parent_neighbor = index;
        Ok(())
    }
    /// Original7BB420 cache lookup and7B9A60 fallback: X difference is stored to
    /// binary32, Y remains extended until the squared distance and square root.
    pub fn distance(&self, from: u32, to: u32, p: Precision) -> Result<Binary> {
        let a = self.node(from)?;
        if from == to {
            return Ok(Binary::from_u32(0));
        }
        if let Some(&value) = a.distance_cache.get(&to) {
            return b(value);
        }
        let z = self.node(to)?;
        let dx = b(b(a.position_bits[0])?
            .sub(&b(z.position_bits[0])?, p)?
            .store_f32()?)?;
        let dy = b(a.position_bits[1])?.sub(&b(z.position_bits[1])?, p)?;
        Ok(dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?.sqrt(p)?)
    }
}
fn push_node(path: &mut Vec<u32>, id: u32) -> Result<()> {
    path.try_reserve(1)?;
    path.insert(0, id);
    Ok(())
}
/// Strategy byte +0x0C (solver+0x40), stored by 0x53F320 from its flag argument.
/// Only 0x53E610 (early test), 0x53F230 (mark) and 0x53EBD0 (finish) read it.
pub(crate) enum Mode<'a> {
    /// Flag 0: start==goal early test, epoch/predecessor marks only, and the
    /// predecessor-chain finalizer.
    Uncached,
    /// Flag 1: the manager's route cache [[0xB3652C]+0x40]. `None` is the
    /// native null pointer between 0x7AC9A0 and 0x7ABC30.
    Cached(Option<&'a mut RouteCache>),
}
impl Mode<'_> {
    /// 0x53E610.
    fn early(&self, request: Request) -> Result<bool> {
        match self {
            Mode::Uncached => Ok(request.start == request.goal),
            Mode::Cached(cache) => {
                route_cache::early(cache.as_deref(), request.start, request.goal)
            }
        }
    }
    /// 0x53F230: the flag-1 cache record 0x53F120 precedes the epoch/parent mark.
    fn mark(&mut self, graph: &mut RegionGraph, start: u32, from: u32, to: u32) -> Result<()> {
        if let Mode::Cached(cache) = self
            && from != to
        {
            route_cache::record(cache.as_deref_mut(), graph, start, from, to)?;
        }
        graph.mark(from, to)
    }
    /// 0x53EBD0. Its boolean result is discarded by every native caller.
    fn finish(&self, graph: &RegionGraph, request: Request, path: &mut Vec<u32>) -> Result<()> {
        match self {
            Mode::Uncached => finish(graph, request, path),
            Mode::Cached(cache) => {
                route_cache::build_path(cache.as_deref(), graph, request.start, request.goal, path)
                    .map(drop)
            }
        }
    }
}
fn finish(graph: &RegionGraph, request: Request, path: &mut Vec<u32>) -> Result<()> {
    //53EBD0 deliberately constructs from the strategy's GOAL field, not the
    //candidate argument. Undefined native missing/cyclic chains become errors.
    let mut id = request.goal;
    let mut seen = HashSet::new();
    push_node(path, id)?;
    while id != request.start {
        if !seen.insert(id) {
            return Err(Error::InvalidPredecessor(id));
        }
        let n = graph.node(id)?;
        id = *n
            .neighbors
            .get(n.parent_neighbor as usize)
            .ok_or(Error::InvalidPredecessor(id))?;
        push_node(path, id)?;
    }
    Ok(())
}

/// Actual native uncached region route computation. A successful output includes
/// both start and goal IDs, ready for region::prepass. No edges/categories or
/// topology are inferred. The cached-route mode (flag 1) is
/// [`crate::route_cache::search_cached`].
pub fn search(
    graph: &mut RegionGraph,
    request: Request,
    state: &mut State,
    path: &mut Vec<u32>,
) -> Result<Outcome> {
    run(graph, request, state, path, Mode::Uncached)
}

/// Shared 0x53EDE0 engine. The mode changes only the three strategy virtuals
/// that read strategy+0x0C; queue, categories, epochs and statuses are shared.
pub(crate) fn run(
    graph: &mut RegionGraph,
    request: Request,
    state: &mut State,
    path: &mut Vec<u32>,
    mut mode: Mode<'_>,
) -> Result<Outcome> {
    graph.validate()?;
    graph.node(request.start)?;
    graph.node(request.goal)?;
    state.output_linked = true;
    let mut popped = Vec::new();
    let mut discovered = Vec::new();
    let outcome = |status, popped, discovered| Outcome {
        status,
        popped,
        discovered,
        queue_remaining: 0,
    };
    if state.busy || !graph.ready {
        path.clear();
        state.output_linked = false;
        state.busy = false;
        return Ok(outcome(0, popped, discovered));
    }
    graph.next_epoch();
    path.clear();
    if graph.node(request.start)?.category != graph.node(request.goal)?.category {
        state.output_linked = false;
        state.busy = false;
        return Ok(outcome(0, popped, discovered));
    }
    state.busy = true;
    mode.mark(graph, request.start, request.start, request.start)?;
    if mode.early(request)? {
        mode.finish(graph, request, path)?;
        state.busy = false;
        return Ok(outcome(1, popped, discovered));
    }
    let p = request.precision;
    // Concrete53ED30 ignores its candidate argument: start-to-goal is fixed.
    let heuristic = graph.distance(request.start, request.goal, p)?;
    let h_bits = heuristic.store_f32()?;
    let mut best = request.start;
    let mut best_bits = h_bits;
    let mut queue = Queue {
        entries: Vec::new(),
        limit: request.queue_limit,
    };
    if !queue.push(QueueEntry {
        node: request.start,
        depth: 0,
        priority_bits: b(h_bits)?.add(&Binary::from_u32(0), p)?.store_f32()?,
        cost_bits: 0,
    })? {
        state.busy = false;
        state.output_linked = false;
        return Ok(outcome(0, popped, discovered));
    }
    while let Some(current) = queue.pop() {
        popped.try_reserve(1)?;
        popped.push(current);
        if heuristic.compare(&b(best_bits)?) == Ordering::Less {
            best = current.node;
            best_bits = h_bits;
        }
        if current.depth >= 255 {
            if best == request.start {
                path.clear();
                state.output_linked = false;
                state.busy = false;
                return Ok(outcome(0, popped, discovered));
            }
            mode.finish(graph, request, path)?;
            state.busy = false;
            return Ok(outcome(2, popped, discovered));
        }
        for i in 0..graph.node(current.node)?.neighbors.len() {
            let next = graph.node(current.node)?.neighbors[i];
            discovered.try_reserve(1)?;
            discovered.push([current.node, next]);
            if graph.visited(next)? {
                continue;
            }
            mode.mark(graph, request.start, current.node, next)?;
            if next == request.goal {
                mode.finish(graph, request, path)?;
                state.busy = false;
                return Ok(outcome(1, popped, discovered));
            }
            if graph.node(next)?.category != graph.node(request.start)?.category {
                continue;
            }
            let cost = graph
                .distance(current.node, next, p)?
                .add(&b(current.cost_bits)?, p)?
                .store_f32()?;
            let priority = b(cost)?.add(&b(h_bits)?, p)?.store_f32()?;
            if !queue.push(QueueEntry {
                node: next,
                depth: current.depth.wrapping_add(1),
                priority_bits: priority,
                cost_bits: cost,
            })? {
                mode.finish(graph, request, path)?;
                state.busy = false;
                return Ok(outcome(2, popped, discovered));
            }
        }
    }
    // Concrete solver virtual+0xc returns0, but native still invokes its goal
    //list finalizer. Inconsistent topology may therefore have no valid chain.
    mode.finish(graph, request, path)?;
    state.busy = false;
    Ok(outcome(0, popped, discovered))
}

/// Direct provider for region::World::plan_regions. Topology and traversal state
/// still belong to the caller; this supplies the actual computed region chain.
pub fn region_route(
    graph: &mut RegionGraph,
    request: Request,
    state: &mut State,
) -> Result<crate::region::RegionRoute> {
    let mut path = Vec::new();
    let outcome = search(graph, request, state, &mut path)?;
    let mut nodes = Vec::new();
    nodes.try_reserve_exact(path.len())?;
    for id in path {
        nodes.push(i32::try_from(id).map_err(|_| Error::InvalidNode(id))?);
    }
    Ok(crate::region::RegionRoute {
        status: outcome.status,
        nodes,
    })
}
