//! Original4DD030/4DD150/4DDDE0 tile traversal, including boundary ties and
//! asymmetric float32 stores. This enumerates candidate cells, not collision.
use crate::{Error, Result};
use aa_numeric::{Fast, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileGrid {
    pub width: i32,
    pub height: i32,
    pub coordinate_shift: u32,
    pub cell_size_bits: u32,
}
impl TileGrid {
    pub fn contains(self, cell: [i32; 2]) -> bool {
        cell[0] >= 0 && cell[1] >= 0 && cell[0] < self.width && cell[1] < self.height
    }
    fn validate(self) -> Result<()> {
        if self.width <= 0
            || self.height <= 0
            || b(self.cell_size_bits)?.compare(&Fast::from_i32(0)) != Ordering::Greater
        {
            return Err(Error::InvalidGrid);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkState {
    pub current: [i32; 2],
    pub end: [i32; 2],
    pub delta_bits: [u32; 2],
    pub point_bits: [u32; 2],
    pub started: bool,
    pub remaining: i32,
}
pub struct TileWalk {
    grid: TileGrid,
    precision: Precision,
    state: WalkState,
}
fn b(bits: u32) -> Result<Fast> {
    Ok(Fast::from_f32_bits(bits)?)
}
fn abs(value: Fast) -> Fast {
    if value.is_negative_nonzero() {
        value.negated()
    } else {
        value
    }
}
fn step(value: &Fast) -> i32 {
    if value.is_negative_nonzero() { -1 } else { 1 }
}

impl TileWalk {
    /// Endpoints outside the grid are preserved: native WalkStart diagnoses but
    /// does not clamp them. next_cell_with_budget performs the original skips.
    pub fn new(
        grid: TileGrid,
        start_bits: [u32; 2],
        end_bits: [u32; 2],
        precision: Precision,
    ) -> Result<Self> {
        grid.validate()?;
        let shift = grid.coordinate_shift & 31;
        let indices = |bits: [u32; 2]| -> Result<[i32; 2]> {
            Ok([
                b(bits[0])?.truncate_i32()? >> shift,
                b(bits[1])?.truncate_i32()? >> shift,
            ])
        };
        let current = indices(start_bits)?;
        let end = indices(end_bits)?;
        let distance = |axis: usize| -> Result<i32> {
            end[axis]
                .checked_sub(current[axis])
                .and_then(i32::checked_abs)
                .ok_or(Error::InvalidGrid)
        };
        let remaining = distance(0)?
            .checked_add(distance(1)?)
            .ok_or(Error::InvalidGrid)?;
        let delta_bits = [
            b(end_bits[0])?
                .sub(&b(start_bits[0])?, precision)?
                .store_f32()?,
            b(end_bits[1])?
                .sub(&b(start_bits[1])?, precision)?
                .store_f32()?,
        ];
        Ok(Self {
            grid,
            precision,
            state: WalkState {
                current,
                end,
                delta_bits,
                point_bits: start_bits,
                started: false,
                remaining,
            },
        })
    }
    pub fn state(&self) -> &WalkState {
        &self.state
    }
    pub fn remaining_candidates(&self) -> usize {
        self.state.remaining as usize + usize::from(!self.state.started)
    }
    /// One original4DD150 candidate. Arithmetic errors can follow the native
    /// remaining-count decrement; the cursor is not transactional or rewound.
    pub fn next_candidate(&mut self) -> Result<Option<[i32; 2]>> {
        if !self.state.started {
            self.state.started = true;
            return Ok(Some(self.state.current));
        }
        if self.state.remaining <= 0 {
            return Ok(None);
        }
        self.state.remaining -= 1;
        let pc = self.precision;
        let dx = b(self.state.delta_bits[0])?;
        let dy = b(self.state.delta_bits[1])?;
        let epsilon = b(0x38d1_b717)?;
        let axis = if abs(dx).compare(&epsilon) == Ordering::Less {
            1
        } else if abs(dy).compare(&epsilon) == Ordering::Less {
            0
        } else {
            let cell = b(self.grid.cell_size_bits)?;
            let boundary_index = |axis: usize, delta: &Fast| -> Result<i32> {
                self.state.current[axis]
                    .checked_add(i32::from(!delta.is_negative_nonzero()))
                    .ok_or(Error::InvalidGrid)
            };
            // Y boundary is stored first; X stays extended until selected.
            let by_bits = Fast::from_i32(boundary_index(1, &dy)?)
                .mul(&cell, pc)?
                .store_f32()?;
            let bx = cell.mul(&Fast::from_i32(boundary_index(0, &dx)?), pc)?;
            let tx = bx.sub(&b(self.state.point_bits[0])?, pc)?.div(&dx, pc)?;
            let tx_stored = b(tx.store_f32()?)?;
            let ty = b(b(by_bits)?
                .sub(&b(self.state.point_bits[1])?, pc)?
                .div(&dy, pc)?
                .store_f32()?)?;
            // Native compares EXTENDED tx with STORED ty; exact ties choose Y.
            if tx.compare(&ty) == Ordering::Less {
                self.state.point_bits[0] = bx.store_f32()?;
                self.state.point_bits[1] = tx_stored
                    .mul(&dy, pc)?
                    .add(&b(self.state.point_bits[1])?, pc)?
                    .store_f32()?;
                0
            } else {
                self.state.point_bits[1] = by_bits;
                self.state.point_bits[0] = ty
                    .mul(&dx, pc)?
                    .add(&b(self.state.point_bits[0])?, pc)?
                    .store_f32()?;
                1
            }
        };
        self.state.current[axis] = self.state.current[axis]
            .checked_add(step(if axis == 0 { &dx } else { &dy }))
            .ok_or(Error::InvalidGrid)?;
        Ok(Some(self.state.current))
    }
    /// Original4DDDE0 bounds filtering with a caller-owned work budget. Every
    /// candidate, including skipped out-of-bounds cells, consumes one unit.
    /// Exhaustion preserves cursor progress and can be resumed deliberately;
    /// no fixed game/map/actor limit or permissive collision answer is added.
    pub fn next_cell_with_budget(&mut self, budget: &mut usize) -> Result<Option<[i32; 2]>> {
        loop {
            if self.remaining_candidates() == 0 {
                return Ok(None);
            }
            if *budget == 0 {
                return Err(Error::WalkBudget);
            }
            *budget -= 1;
            let Some(cell) = self.next_candidate()? else {
                return Ok(None);
            };
            if self.grid.contains(cell) {
                return Ok(Some(cell));
            }
        }
    }
}
