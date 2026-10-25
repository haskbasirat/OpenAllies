//! Native predecessor-chain conversion (0x53d500 / 0x53cf40).
//! Grid tracing and collision predicates are required native world boundaries.

use crate::{OwnedPath, PathNode};
use aa_numeric::{Fast, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;

pub const DELTAS: [[i32; 2]; 9] = [
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
const DIRECTIONS: [[u32; 2]; 9] = [
    [0, 0],
    [0xbf3504f3, 0xbf3504f3],
    [0xbf800000, 0],
    [0xbf3504f3, 0x3f3504f3],
    [0, 0x3f800000],
    [0x3f3504f3, 0x3f3504f3],
    [0x3f800000, 0],
    [0x3f3504f3, 0xbf3504f3],
    [0, 0xbf800000],
];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("invalid or cyclic native predecessor chain at {0:?}")]
    InvalidPredecessor([i32; 2]),
    #[error("native path materializer requires linked path/collision context")]
    MissingLinks,
    #[error("path head was removed by a world callback")]
    MissingHead,
    #[error("path allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error("native materialization world boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub trait PredecessorGrid {
    fn dimensions(&self) -> [i32; 2];
    fn coordinate_shift(&self) -> u32;
    fn cell_size_bits(&self) -> u32;
    /// Original cell byte +0x2d; storage is a stable completed-search snapshot.
    fn predecessor(&self, cell: [i32; 2]) -> Option<u8>;
}

/// Original path strategy fields. Link booleans model the original +4/+1c
/// pointers' presence without embedding native process addresses in Rust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub start_cell: [i32; 2],
    pub goal_cell: [i32; 2],
    pub start_bits: [u32; 2],
    pub goal_bits: [u32; 2],
    pub path_linked: bool,
    pub collision_linked: bool,
    /// Opaque descriptor+0x28 value, suppressed during each smoothing trace.
    pub collision_word28: u32,
}

/// Required world behavior, with no guessed line rasterization or collision
/// defaults. A native trace visits cells lazily and stops at the first blocked
/// cell. Callbacks may mutate strategy/path fields; later reads follow native
/// order. On callback failure prior path changes remain, but the temporary
/// descriptor+0x28 suppression is restored.
pub trait World {
    fn begin_trace(
        &mut self,
        from: [u32; 2],
        to: [u32; 2],
        state: &mut State,
        path: &mut OwnedPath,
    ) -> Result<()>;
    fn next_trace_cell(
        &mut self,
        state: &mut State,
        path: &mut OwnedPath,
    ) -> Result<Option<[i32; 2]>>;
    fn cell_blocked(
        &mut self,
        cell: [i32; 2],
        state: &mut State,
        path: &mut OwnedPath,
    ) -> Result<bool>;
}

fn b(bits: u32) -> Result<Fast> {
    Ok(Fast::from_f32_bits(bits)?)
}
fn direction<G: PredecessorGrid>(grid: &G, cell: [i32; 2]) -> Result<u8> {
    let [w, h] = grid.dimensions();
    if cell[0] < 0 || cell[1] < 0 || cell[0] >= w || cell[1] >= h {
        return Err(Error::InvalidPredecessor(cell));
    }
    grid.predecessor(cell)
        .filter(|d| *d <= 8)
        .ok_or(Error::InvalidPredecessor(cell))
}
fn parent(cell: [i32; 2], dir: u8) -> [i32; 2] {
    [
        cell[0].wrapping_sub(DELTAS[dir as usize][0]),
        cell[1].wrapping_sub(DELTAS[dir as usize][1]),
    ]
}
fn validate_chain<G: PredecessorGrid>(grid: &G, end: [i32; 2], start: [i32; 2]) -> Result<()> {
    let mut seen = HashSet::new();
    let mut cell = end;
    loop {
        let dir = direction(grid, cell)?;
        if cell == start {
            return Ok(());
        }
        if dir == 0 || !seen.insert(cell) {
            return Err(Error::InvalidPredecessor(cell));
        }
        cell = parent(cell, dir);
    }
}
fn center<G: PredecessorGrid>(
    grid: &G,
    cell: [i32; 2],
    p: Precision,
    stored_half: bool,
) -> Result<[Fast; 2]> {
    let mut half = b(grid.cell_size_bits())?.mul(&b(0x3f000000)?, p)?;
    if stored_half {
        half = b(half.store_f32()?)?;
    }
    let shift = grid.coordinate_shift() & 31;
    Ok([
        Fast::from_i32(cell[0].wrapping_shl(shift)).add(&half, p)?,
        Fast::from_i32(cell[1].wrapping_shl(shift)).add(&half, p)?,
    ])
}
fn stored(point: &[Fast; 2]) -> Result<[u32; 2]> {
    Ok([point[0].store_f32()?, point[1].store_f32()?])
}
fn prepend(path: &mut OwnedPath, xy: [u32; 2], dir: [u32; 2]) -> Result<()> {
    path.nodes.try_reserve(1)?;
    path.nodes.insert(
        0,
        PathNode {
            words: [xy[0], xy[1], dir[0], dir[1], 1, 0, xy[0], xy[1]],
        },
    );
    path.kind1_count = path.kind1_count.wrapping_add(1);
    path.dirty = true;
    Ok(())
}
fn normalize(delta: [u32; 2], p: Precision) -> Result<([u32; 2], bool)> {
    let x = b(delta[0])?;
    let y = b(delta[1])?;
    let length = x.mul(&x, p)?.add(&y.mul(&y, p)?, p)?.sqrt(p)?;
    if length.compare(&b(0x38d1b717)?) != Ordering::Greater {
        return Ok(([0, 0], false));
    }
    let reciprocal = Fast::from_u32(1).div(&length, p)?;
    Ok((
        [
            reciprocal.mul(&x, p)?.store_f32()?,
            reciprocal.mul(&y, p)?.store_f32()?,
        ],
        true,
    ))
}
fn update_head(path: &mut OwnedPath, from: &[Fast; 2], p: Precision) -> Result<()> {
    let head = path.nodes.first_mut().ok_or(Error::MissingHead)?;
    let dx = b(head.words[0])?.sub(&from[0], p)?.store_f32()?;
    let dy = b(head.words[1])?.sub(&from[1], p)?.store_f32()?;
    let (dir, _) = normalize([dx, dy], p)?;
    head.words[2] = dir[0];
    head.words[3] = dir[1];
    Ok(())
}
fn unlink(state: &mut State) {
    state.path_linked = false;
    state.collision_linked = false;
}

/// Original same-cell finalizer 0x53cf40; this may still emit one destination
/// node when the world coordinates differ within that cell.
pub fn same_cell(state: &mut State, path: &mut OwnedPath, p: Precision) -> Result<()> {
    if !state.path_linked || !state.collision_linked {
        return Err(Error::MissingLinks);
    }
    let sx = b(state.start_bits[0])?;
    let sy = b(state.start_bits[1])?;
    let gx = b(state.goal_bits[0])?;
    let gy = b(state.goal_bits[1])?;
    if gx.compare(&sx) != Ordering::Equal || gy.compare(&sy) != Ordering::Equal {
        let delta = [gx.sub(&sx, p)?.store_f32()?, gy.sub(&sy, p)?.store_f32()?];
        let (dir, nonzero) = normalize(delta, p)?;
        if nonzero {
            prepend(path, state.goal_bits, dir)?;
        }
    }
    unlink(state);
    Ok(())
}

/// Original 0x53d500, including direction-run reduction and collision-ordered
/// line-of-sight smoothing. Requires an actual native-compatible trace world.
pub fn materialize<G: PredecessorGrid, W: World>(
    state: &mut State,
    path: &mut OwnedPath,
    end: [i32; 2],
    grid: &G,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    if !state.path_linked || !state.collision_linked {
        return Err(Error::MissingLinks);
    }
    if end == state.start_cell {
        return same_cell(state, path, p);
    }
    validate_chain(grid, end, state.start_cell)?;
    let mut last_dir = direction(grid, end)?;
    let initial_direction = DIRECTIONS[last_dir as usize];
    let xy = if end == state.goal_cell {
        state.goal_bits
    } else {
        stored(&center(grid, end, p, false)?)?
    };
    prepend(path, xy, initial_direction)?;
    let mut anchor = end;
    let mut previous = end;
    let mut current = parent(end, last_dir);
    loop {
        let current_dir = direction(grid, current)?;
        if previous == anchor {
            if current == state.start_cell {
                break;
            }
            if current_dir != last_dir {
                previous = current;
            }
        } else {
            let saved = state.collision_word28;
            state.collision_word28 = 0;
            let trace_result: Result<bool> = (|| {
                let to = {
                    let h = path.nodes.first().ok_or(Error::MissingHead)?;
                    [h.words[0], h.words[1]]
                };
                world.begin_trace(stored(&center(grid, current, p, false)?)?, to, state, path)?;
                while let Some(cell) = world.next_trace_cell(state, path)? {
                    if world.cell_blocked(cell, state, path)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            })();
            state.collision_word28 = saved;
            if !trace_result? {
                let point = center(grid, previous, p, true)?;
                update_head(path, &point, p)?;
                prepend(path, stored(&point)?, initial_direction)?;
                anchor = previous;
                last_dir = direction(grid, previous)?;
                continue; // Original retries this predecessor after inserting.
            }
            if current == state.start_cell {
                break;
            }
            previous = current;
        }
        current = parent(current, current_dir);
    }
    update_head(path, &[b(state.start_bits[0])?, b(state.start_bits[1])?], p)?;
    unlink(state);
    Ok(())
}
