//! Concrete native high-level tile search (0x53d8d0), not a generic A* substitute.
//!
//! First discovery closes a tile, including a subsequently rejected tile. Goal
//! detection occurs during discovery. Equal heap priorities deliberately rise
//! on insertion; the right child wins ties during removal. Those rules affect
//! route choice and must not be replaced with Rust's ordinary BinaryHeap order.

use aa_numeric::{Fast, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const DIRECTIONS: [[i32; 2]; 9] = [
    [0, 0],
    [-1, -1],
    [-1, 0],
    [-1, 1],
    [0, 1],
    [1, 1],
    [1, 0],
    [1, -1],
    [0, -1],
];
/// Native constructor's optional queue policy; storage itself grows on demand.
pub const NATIVE_QUEUE_LIMIT: usize = 0x1f9c;
const DIAGONAL: u32 = 0x3fb5_04f3;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("search allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error("invalid native search grid dimensions or cell size")]
    InvalidGrid,
    #[error("native search requires a blocked outer tile ring")]
    UnprotectedBoundary,
    #[error("invalid native predecessor direction {0}")]
    InvalidDirection(u8),
    #[error("search cell is outside the native grid: {0:?}")]
    InvalidCell([i32; 2]),
    #[error("native search world boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, SearchError>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCell {
    pub flags20: u32,
    pub visit_epoch: u32,
    pub predecessor_direction: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchGrid {
    pub width: i32,
    pub height: i32,
    pub coordinate_shift: u32,
    pub cell_size_bits: u32,
    pub epoch: u32,
    pub cells: Vec<SearchCell>,
}
impl SearchGrid {
    pub fn index(&self, cell: [i32; 2]) -> Result<usize> {
        if cell[0] < 0 || cell[1] < 0 || cell[0] >= self.width || cell[1] >= self.height {
            return Err(SearchError::InvalidCell(cell));
        }
        Ok(cell[1] as usize * self.width as usize + cell[0] as usize)
    }
    pub fn cell(&self, cell: [i32; 2]) -> Result<&SearchCell> {
        let index = self.index(cell)?;
        self.cells.get(index).ok_or(SearchError::InvalidGrid)
    }
    fn validate(&self) -> Result<()> {
        if self.width <= 0
            || self.height <= 0
            || (self.width as usize).checked_mul(self.height as usize) != Some(self.cells.len())
            || Fast::from_f32_bits(self.cell_size_bits)?.compare(&Fast::from_i32(0))
                != Ordering::Greater
        {
            return Err(SearchError::InvalidGrid);
        }
        for x in 0..self.width {
            if self.cell([x, 0])?.flags20 & 1 == 0
                || self.cell([x, self.height - 1])?.flags20 & 1 == 0
            {
                return Err(SearchError::UnprotectedBoundary);
            }
        }
        for y in 0..self.height {
            if self.cell([0, y])?.flags20 & 1 == 0
                || self.cell([self.width - 1, y])?.flags20 & 1 == 0
            {
                return Err(SearchError::UnprotectedBoundary);
            }
        }
        Ok(())
    }
    /// 0x552440, including the native zero-generation rollover behavior.
    fn next_epoch(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch >= 0x7fff_ffff {
            self.epoch = 0;
            for cell in &mut self.cells {
                cell.visit_epoch = 0;
            }
        }
    }
    /// 0x552440 for the low-path goal (0x53B720), which shares this epoch.
    pub(crate) fn advance_epoch(&mut self) {
        self.next_epoch();
    }
    fn mark(&mut self, from: [i32; 2], to: [i32; 2]) -> Result<()> {
        let delta = [to[0].wrapping_sub(from[0]), to[1].wrapping_sub(from[1])];
        let direction = DIRECTIONS
            .iter()
            .position(|&v| v == delta)
            .ok_or(SearchError::InvalidCell(to))? as u8;
        let index = self.index(to)?;
        self.cells[index].visit_epoch = self.epoch;
        self.cells[index].predecessor_direction = direction;
        Ok(())
    }
}

impl crate::materialize::PredecessorGrid for SearchGrid {
    fn dimensions(&self) -> [i32; 2] {
        [self.width, self.height]
    }
    fn coordinate_shift(&self) -> u32 {
        self.coordinate_shift
    }
    fn cell_size_bits(&self) -> u32 {
        self.cell_size_bits
    }
    fn predecessor(&self, cell: [i32; 2]) -> Option<u8> {
        self.cell(cell)
            .ok()
            .map(|value| value.predecessor_direction)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchInput {
    pub start_cell: [i32; 2],
    pub goal_cell: [i32; 2],
    pub goal_world_bits: [u32; 2],
    pub precision: Precision,
    /// `None` means no queue count limit. Use NATIVE_QUEUE_LIMIT when exact
    /// native bounded-search policy is required; this does not reserve memory.
    pub queue_limit: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEntry {
    pub cell: [i32; 2],
    pub depth: i32,
    pub priority_bits: u32,
    pub cost_bits: u32,
}
impl QueueEntry {
    pub fn words(self) -> [u32; 5] {
        [
            self.cell[0] as u32,
            self.cell[1] as u32,
            self.depth as u32,
            self.priority_bits,
            self.cost_bits,
        ]
    }
}

#[derive(Default, Debug)]
pub struct NativeQueue {
    entries: Vec<QueueEntry>,
    maximum_entries: Option<usize>,
}
impl NativeQueue {
    pub fn new(maximum_entries: Option<usize>) -> Self {
        Self {
            entries: Vec::new(),
            maximum_entries,
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// 0x53d090: ties move above their parent, so insertion is not stable.
    pub fn push(&mut self, entry: QueueEntry) -> Result<bool> {
        Fast::from_f32_bits(entry.priority_bits)?;
        if self
            .maximum_entries
            .is_some_and(|limit| limit > 0 && self.len() == limit)
        {
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
    /// 0x53d150: select the right equal child, but stop if the tail is equal.
    pub fn pop(&mut self) -> Option<QueueEntry> {
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

fn scalar(bits: u32) -> Result<Fast> {
    Ok(Fast::from_f32_bits(bits)?)
}
fn stored(value: Fast) -> Result<Fast> {
    scalar(value.store_f32()?)
}

/// 0x53c7a0. The extended result is retained for closest-cell comparison;
/// callers explicitly store binary32 when writing a queue or best-distance word.
pub fn heuristic(grid: &SearchGrid, input: &SearchInput, cell: [i32; 2]) -> Result<Fast> {
    if cell == input.goal_cell {
        return Ok(Fast::from_i32(0));
    }
    let p = input.precision;
    let half = scalar(grid.cell_size_bits)?.mul(&scalar(0x3f00_0000)?, p)?;
    let mut delta = [Fast::from_i32(0), Fast::from_i32(0)];
    for i in 0..2 {
        let shifted = cell[i].wrapping_shl(grid.coordinate_shift & 31);
        let center = stored(Fast::from_i32(shifted).add(&half, p)?)?;
        let value = center.sub(&scalar(input.goal_world_bits[i])?, p)?;
        delta[i] = if value.is_negative_nonzero() {
            value.negated()
        } else {
            value
        };
    }
    let [x, y] = delta;
    let (small, large) = if x.compare(&y) == Ordering::Less {
        (x, y)
    } else {
        (y, x)
    };
    Ok(small
        .mul(&scalar(DIAGONAL)?, p)?
        .add(&large.sub(&small, p)?, p)?)
}

/// 0x53c5c0 followed by the unmodified step branch of 0x53cb70. This is valid
/// only when the host has established that wall-entry and terrain-change penalties
/// do not apply; it is never a default for the required cost callback.
pub fn unmodified_step_cost(
    grid: &SearchGrid,
    current: QueueEntry,
    next: [i32; 2],
    p: Precision,
) -> Result<u32> {
    let delta = [next[0] - current.cell[0], next[1] - current.cell[1]];
    let direction = DIRECTIONS
        .iter()
        .position(|&v| v == delta)
        .ok_or(SearchError::InvalidCell(next))?;
    let multiplier = if direction & 1 != 0 {
        DIAGONAL
    } else {
        0x3f80_0000
    };
    let accumulated = scalar(current.cost_bits)?;
    let step = scalar(multiplier)?.mul(&scalar(grid.cell_size_bits)?, p)?;
    let step = stored(step.add(&accumulated, p)?.sub(&accumulated, p)?)?;
    Ok(step.add(&accumulated, p)?.store_f32()?)
}

/// Required world seams from strategy 0xa36610. Predicate calls intentionally
/// remain ordered, including duplicate neighbor tests. No permissive defaults
/// invent collision geometry, terrain speed or wall ownership.
pub trait SearchWorld {
    /// Interior tile predicate 0x54f370, after the native flags20 bit-0 check.
    fn collision_blocked(&mut self, grid: &SearchGrid, cell: [i32; 2]) -> Result<bool>;
    /// Native 0x53cb70 result at its caller's binary32 store boundary.
    fn accumulated_cost(
        &mut self,
        grid: &SearchGrid,
        current: QueueEntry,
        next: [i32; 2],
        precision: Precision,
    ) -> Result<u32>;
}
fn blocked<W: SearchWorld>(world: &mut W, grid: &SearchGrid, cell: [i32; 2]) -> Result<bool> {
    Ok(grid.cell(cell)?.flags20 & 1 != 0 || world.collision_blocked(grid, cell)?)
}
fn next_direction(direction: u8) -> u8 {
    if direction == 8 { 1 } else { direction + 1 }
}
fn previous_direction(direction: u8) -> u8 {
    if direction == 1 { 8 } else { direction - 1 }
}
fn offset(cell: [i32; 2], direction: u8) -> [i32; 2] {
    [
        cell[0] + DIRECTIONS[direction as usize][0],
        cell[1] + DIRECTIONS[direction as usize][1],
    ]
}

/// 0x554ed0/0x4f2000's concrete high-level policy: parent-facing five-neighbor
/// fan, exclude reverse, reject diagonals if either adjacent orthogonal blocks.
fn neighbor_directions(grid: &SearchGrid, cell: [i32; 2]) -> Result<(u8, u8, u8)> {
    let source = grid.cell(cell)?;
    if source.flags20 & 1 != 0 {
        return Ok((0, 0, 0));
    }
    let parent = source.predecessor_direction;
    if parent > 8 {
        return Err(SearchError::InvalidDirection(parent));
    }
    if parent == 0 {
        return Ok((1, 8, 0));
    }
    Ok((
        previous_direction(previous_direction(parent)),
        next_direction(next_direction(parent)),
        next_direction(next_direction(next_direction(next_direction(parent)))),
    ))
}
fn neighbor_allowed<W: SearchWorld>(
    world: &mut W,
    grid: &SearchGrid,
    cell: [i32; 2],
    direction: u8,
    reverse: u8,
) -> Result<bool> {
    if direction == reverse {
        return Ok(false);
    }
    let candidate = offset(cell, direction);
    if grid.cell(candidate)?.flags20 & 1 != 0 {
        return Ok(false);
    }
    if blocked(world, grid, candidate)? {
        return Ok(false);
    }
    if direction & 1 != 0 {
        let a = blocked(world, grid, offset(cell, previous_direction(direction)))?;
        // Native tests both orthogonals even after the first blocks.
        let b = blocked(world, grid, offset(cell, next_direction(direction)))?;
        if a || b {
            return Ok(false);
        }
    }
    Ok(true)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchOutcome {
    /// Native 0 failure, 1 goal, 2 partial route. No custom success mapping.
    pub status: u32,
    pub end_cell: Option<[i32; 2]>,
    pub expanded: Vec<QueueEntry>,
    pub queue_remaining: usize,
}

/// Concrete search after Solve's endpoint validation, path initialization and
/// actor-dependent strategy initialization. The caller owns 0x53d500 path
/// materialization and must not treat these predecessor cells as final waypoints.
pub fn search<W: SearchWorld>(
    grid: &mut SearchGrid,
    input: SearchInput,
    world: &mut W,
) -> Result<SearchOutcome> {
    grid.validate()?;
    grid.index(input.start_cell)?;
    grid.index(input.goal_cell)?;
    scalar(input.goal_world_bits[0])?;
    scalar(input.goal_world_bits[1])?;
    grid.next_epoch();
    grid.mark(input.start_cell, input.start_cell)?;
    let mut queue = NativeQueue::new(input.queue_limit);
    let mut expanded = Vec::new();
    let outcome = |status, end_cell, expanded, queue_remaining| SearchOutcome {
        status,
        end_cell,
        expanded,
        queue_remaining,
    };
    if input.start_cell == input.goal_cell {
        return Ok(outcome(1, Some(input.start_cell), expanded, 0));
    }
    let mut closest = input.start_cell;
    let mut closest_distance = heuristic(grid, &input, closest)?.store_f32()?;
    if !queue.push(QueueEntry {
        cell: closest,
        depth: 0,
        priority_bits: closest_distance,
        cost_bits: 0,
    })? {
        return Ok(outcome(0, None, expanded, queue.len()));
    }
    while let Some(current) = queue.pop() {
        let distance = heuristic(grid, &input, current.cell)?;
        if distance.compare(&scalar(closest_distance)?) == Ordering::Less {
            closest = current.cell;
            closest_distance = distance.store_f32()?;
        }
        expanded.try_reserve(1)?;
        expanded.push(current);
        if current.depth >= 255 {
            return Ok(if closest == input.start_cell {
                outcome(0, None, expanded, queue.len())
            } else {
                outcome(2, Some(closest), expanded, queue.len())
            });
        }
        let (mut direction, last, reverse) = neighbor_directions(grid, current.cell)?;
        while direction != 0 {
            if neighbor_allowed(world, grid, current.cell, direction, reverse)? {
                let candidate = offset(current.cell, direction);
                if grid.cell(candidate)?.visit_epoch != grid.epoch {
                    grid.mark(current.cell, candidate)?;
                    if candidate == input.goal_cell {
                        return Ok(outcome(1, Some(candidate), expanded, queue.len()));
                    }
                    if !blocked(world, grid, candidate)? {
                        let remaining = heuristic(grid, &input, candidate)?.store_f32()?;
                        let cost_bits =
                            world.accumulated_cost(grid, current, candidate, input.precision)?;
                        let priority_bits = scalar(cost_bits)?
                            .add(&scalar(remaining)?, input.precision)?
                            .store_f32()?;
                        if !queue.push(QueueEntry {
                            cell: candidate,
                            depth: current.depth.wrapping_add(1),
                            priority_bits,
                            cost_bits,
                        })? {
                            return Ok(outcome(2, Some(closest), expanded, queue.len()));
                        }
                    }
                }
            }
            if direction == last {
                break;
            }
            direction = next_direction(direction);
        }
    }
    Ok(outcome(2, Some(closest), expanded, 0))
}
