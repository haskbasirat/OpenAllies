//! Core low-path loop 0x53BE30 with the solver vtable 0xA36460 and the
//! neighbour iterator 0x53BC80 / 0x53BB40 / 0x53B3C0.
//!
//! 0x53BE30 is the same template instantiation as the tile search 0x53D8D0
//! (identical control flow, closest-cell tracking and the shared queue class
//! 0x53D2A0 / 0x53D090 / 0x53D150, reused here through `NativeQueue`). It
//! differs in the heuristic (0x53ADB0), the window cell layout, the limit
//! test (0x53A660 instead of depth > 254), the neighbour iterator, the expand
//! method (0x53B0C0 instead of 0x53C900) and the exhausted-queue result
//! (0x53A790 instead of a constant 2).
use super::goal::Run;
use super::{Environment, LowPathSolver, LowPathWorld, Result, ZERO, f, st};
use crate::OwnedPath;
use crate::result_plan::CollisionContext;
use crate::search::{DIRECTIONS, NativeQueue, QueueEntry};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Solver words +0x10..+0x3C (closest cell, its binary32 distance, current
/// cell, popped queue entry, candidate cell).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchState {
    pub closest_cell: [i32; 2],
    pub closest_distance_bits: u32,
    pub current_cell: [i32; 2],
    pub entry_words: [u32; 5],
    pub candidate_cell: [i32; 2],
}

const NEXT: [u32; 9] = [0, 2, 3, 4, 5, 6, 7, 8, 1]; // 0xAB7804
const EVEN_START: [u32; 9] = [0, 8, 1, 2, 3, 4, 5, 6, 7]; // 0xAB7828
const ODD_END: [u32; 9] = [0, 3, 4, 5, 6, 7, 8, 1, 2]; // 0xAB784C
const ODD_START: [u32; 9] = [0, 7, 8, 1, 2, 3, 4, 5, 6]; // 0xAB7870
const REVERSE: [u32; 9] = [0, 5, 6, 7, 8, 1, 2, 3, 4]; // 0xAB7894

fn offset(cell: [i32; 2], direction: u32) -> [i32; 2] {
    let d = DIRECTIONS[direction as usize];
    [cell[0].wrapping_add(d[0]), cell[1].wrapping_add(d[1])]
}

/// 0x53D2A0: priority is the binary32 sum of cost and heuristic.
fn push(
    queue: &mut NativeQueue,
    cell: [i32; 2],
    depth: i32,
    cost_bits: u32,
    heuristic_bits: u32,
    p: Precision,
) -> Result<bool> {
    let priority_bits = st(&f(cost_bits)?.add(&f(heuristic_bits)?, p)?)?;
    Ok(queue.push(QueueEntry {
        cell,
        depth,
        priority_bits,
        cost_bits,
    })?)
}

/// Iterator object of 0x53BC80 (built on the core loop's stack).
struct Neighbours {
    cell: [i32; 2],
    parent: u32,
    direction: u32,
    end: u32,
    ignore_moving: bool,
    accuracy: bool,
}
impl Neighbours {
    /// 0x53BC80. Goal virtual +0x30 (0x6A71F0) returns false, so the fan
    /// always comes from the predecessor: all eight from the start, five
    /// around a diagonal predecessor, three around an orthogonal one.
    fn new<W: LowPathWorld>(run: &mut Run<'_, W>, cell: [i32; 2], depth: i32) -> Result<Self> {
        let parent = run.window.cells[run.window.cell_index(cell)?].direction;
        let (direction, end) = match parent {
            0 => (1, 8),
            9.. => (0, 0),
            odd if odd & 1 != 0 => (ODD_START[odd as usize], ODD_END[odd as usize]),
            even => (EVEN_START[even as usize], NEXT[even as usize]),
        };
        let mut iterator = Self {
            cell,
            parent,
            direction,
            end,
            ignore_moving: depth >= run.settings.ignore_moving_depth,
            accuracy: depth < run.settings.accuracy_depth,
        };
        if iterator.direction != 0 && !iterator.valid(run)? {
            iterator.advance(run)?;
        }
        Ok(iterator)
    }
    /// 0x53B3C0: not the reverse of the predecessor, inside the window, world
    /// tile flag bit 0 clear, and the 0x53AF70 line test passes.
    fn valid<W: LowPathWorld>(&self, run: &mut Run<'_, W>) -> Result<bool> {
        if REVERSE[self.parent as usize] == self.direction {
            return Ok(false);
        }
        let next = offset(self.cell, self.direction);
        if next[0] < 0
            || next[0] >= run.window.size[0]
            || next[1] < 0
            || next[1] >= run.window.size[1]
        {
            return Ok(false);
        }
        let index = run.window.cell_index(next)?;
        if !run.tile_open(index)? {
            return Ok(false);
        }
        run.line_clear(self.cell, next, self.ignore_moving, self.accuracy)
    }
    /// 0x53BB40.
    fn advance<W: LowPathWorld>(&mut self, run: &mut Run<'_, W>) -> Result<()> {
        let start = self.direction;
        if start == 0 {
            return Ok(());
        }
        loop {
            if self.direction == self.end {
                break;
            }
            self.direction = NEXT[self.direction as usize];
            if self.direction == start {
                break;
            }
            if self.valid(run)? {
                return Ok(());
            }
        }
        self.direction = 0;
        self.end = 0;
        Ok(())
    }
}

/// 0x53BE30 followed by the queue count that 0x53C170 hands to the stats.
/// `expanded` receives every popped queue entry in native order.
pub(crate) fn run<W: LowPathWorld>(
    solver: &mut LowPathSolver,
    env: &mut Environment<'_>,
    path: &mut OwnedPath,
    context: &mut CollisionContext,
    expanded: &mut Vec<QueueEntry>,
    world: &mut W,
) -> Result<u32> {
    let LowPathSolver {
        window,
        goal,
        search,
        queue_remaining,
    } = solver;
    let mut run = Run {
        window,
        goal,
        grid: env.grid,
        path,
        context,
        settings: env.settings,
        p: env.options.precision,
        world,
    };
    let busy = &mut *env.search_busy;
    if *busy {
        // Queue +0x14 (0x4B2830) is empty; goal OnFailed; flag cleared.
        run.on_failed();
        *busy = false;
        return Ok(0);
    }
    // Queue +0x10 (0x53D2F0) empties the queue and reports success.
    let mut queue = NativeQueue::new(env.options.queue_limit);
    *queue_remaining = 0;
    let status = core(&mut run, search, &mut queue, expanded, busy);
    *queue_remaining = queue.len();
    status
}

fn core<W: LowPathWorld>(
    run: &mut Run<'_, W>,
    state: &mut SearchState,
    queue: &mut NativeQueue,
    expanded: &mut Vec<QueueEntry>,
    busy: &mut bool,
) -> Result<u32> {
    let p = run.p;
    if !run.on_solve_started()? {
        run.on_failed();
        *busy = false;
        return Ok(0);
    }
    *busy = true;
    let start = run.goal.start_cell;
    let start_index = run.window.cell_index(start)?;
    // Solver vtable [2] 0x4F0550 -> goal Mark(start, start).
    run.mark(start, start, start_index)?;
    if run.goal.direct || start == run.goal.goal_cell {
        run.on_solved(start)?;
        *busy = false;
        return Ok(1);
    }
    let heuristic = st(&run.heuristic(start)?)?;
    // Start depth 0x4C9FA0 = 0, start cost 0x4DB6B0 = +0.0.
    if !push(queue, start, 0, ZERO, heuristic, p)? {
        run.on_failed();
        *busy = false;
        return Ok(0);
    }
    state.closest_cell = start;
    state.closest_distance_bits = st(&run.heuristic(start)?)?;
    state.current_cell = start;
    loop {
        let Some(entry) = queue.pop() else {
            // Solver vtable [3] 0x53A790, then 0x539AD0 with the closest cell.
            let status = if state.closest_cell == run.goal.start_cell {
                0
            } else {
                2
            };
            run.on_solved(state.closest_cell)?;
            *busy = false;
            return Ok(status);
        };
        expanded.try_reserve(1)?;
        expanded.push(entry);
        state.entry_words = entry.words();
        state.current_cell = entry.cell;
        let distance = run.heuristic(entry.cell)?;
        if distance.compare(&f(state.closest_distance_bits)?) == Ordering::Less {
            state.closest_cell = entry.cell;
            state.closest_distance_bits = st(&distance)?;
        }
        // Solver vtable [1] 0x7C7840 always continues (3).
        if run.limit_reached(&entry) {
            if state.closest_cell != start {
                run.on_solved(state.closest_cell)?;
                *busy = false;
                return Ok(2);
            }
            run.on_failed();
            *busy = false;
            return Ok(0);
        }
        let mut neighbours = Neighbours::new(run, entry.cell, entry.depth)?;
        while neighbours.direction != 0 {
            let candidate = offset(neighbours.cell, neighbours.direction);
            state.candidate_cell = candidate;
            let result = expand(run, state, queue, &entry, candidate, busy)?;
            if result != 3 {
                return Ok(result);
            }
            neighbours.advance(run)?;
        }
    }
}

/// 0x53B0C0 (solver vtable [0]).
fn expand<W: LowPathWorld>(
    run: &mut Run<'_, W>,
    state: &SearchState,
    queue: &mut NativeQueue,
    entry: &QueueEntry,
    candidate: [i32; 2],
    busy: &mut bool,
) -> Result<u32> {
    let index = run.window.cell_index(candidate)?;
    if run.visited(index) {
        return Ok(3);
    }
    run.mark(state.current_cell, candidate, index)?;
    if candidate == run.goal.goal_cell {
        run.on_solved(candidate)?;
        *busy = false;
        return Ok(1);
    }
    if run.blocked(index)? {
        return Ok(3);
    }
    let depth = entry.depth.wrapping_add(1);
    let heuristic = st(&run.heuristic(candidate)?)?;
    let cost = st(&run.cost(entry, candidate)?)?;
    if !push(queue, candidate, depth, cost, heuristic, run.p)? {
        run.on_solved(state.closest_cell)?;
        *busy = false;
        return Ok(2);
    }
    Ok(3)
}
