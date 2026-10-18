//! Original terrain heightfield sampling; native grid construction is separate.
#![forbid(unsafe_code)]

use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
pub mod binding;
pub mod brush;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("heightfield dimensions and storage do not describe a valid native grid")]
    InvalidGrid,
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("invalid or unsupported native heightfield resolution/extents")]
    InvalidLayout,
    #[error("heightfield source chunk is truncated")]
    TruncatedChunk,
    #[error("unsupported heightfield chunk version {0}")]
    UnsupportedVersion(u32),
    #[error("heightfield source resolution or sample count differs from constructed layout")]
    LayoutMismatch,
    #[error("heightfield storage allocation failed")]
    Allocation,
}
pub type Result<T> = std::result::Result<T, Error>;

/// Stable native heightfield fields, without inferred map-coordinate conversion.
/// Width/height are +0x0c/+0x10; shift +0x14; cell size +0x18; bias +0x1c;
/// row-major binary32 sample storage is reached through +0x40.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heightfield {
    pub width: i32,
    pub height: i32,
    pub coordinate_shift: u32,
    pub cell_size_bits: u32,
    /// Used only by nearest sampling, before the arithmetic coordinate shift.
    pub index_bias: i32,
    pub height_bits: Vec<u32>,
}

impl Heightfield {
    pub fn validate(&self) -> Result<()> {
        if self.width <= 0
            || self.height <= 0
            || self.width.checked_mul(self.height).map(|n| n as usize)
                != Some(self.height_bits.len())
        {
            return Err(Error::InvalidGrid);
        }
        Ok(())
    }

    fn height_at(&self, x: i32, y: i32) -> Result<Binary> {
        b(self.height_bits[(y * self.width + x) as usize])
    }

    /// Entire original 0x550190. Truncation precedes wrapping signed addition,
    /// arithmetic shift (count masked to five bits), and edge clamping.
    pub fn sample_nearest(&self, position_bits: [u32; 2]) -> Result<Binary> {
        self.validate()?;
        let shift = self.coordinate_shift & 31;
        let x = (b(position_bits[0])?
            .truncate_i32()?
            .wrapping_add(self.index_bias)
            >> shift)
            .clamp(0, self.width - 1);
        let y = (b(position_bits[1])?
            .truncate_i32()?
            .wrapping_add(self.index_bias)
            >> shift)
            .clamp(0, self.height - 1);
        self.height_at(x, y)
    }

    /// Entire original 0x550210 for finite, defined arithmetic. The result stays
    /// extended until the caller requests a store, as the original ABI requires.
    /// Native grid construction/mapping and the game-thread precision are not
    /// inferred here. Cell size must be finite and nonzero; sampled values and
    /// used coordinates must be finite and their integer conversions representable.
    pub fn sample_interpolated(
        &self,
        position_bits: [u32; 2],
        precision: Precision,
    ) -> Result<Binary> {
        self.validate()?;
        let p = precision;
        let x = b(position_bits[0])?;
        let y = b(position_bits[1])?;
        let cell = b(self.cell_size_bits)?;
        // Original CRT FPREM loop, with no rounded division/subtraction shortcut.
        // Only X is stored to binary32 at this boundary.
        let rx = b(x.remainder_trunc(&cell)?.store_f32()?)?;
        let ry = y.remainder_trunc(&cell)?;
        let epsilon = b(0x38d1b717)?;
        let half = b(0x3f000000)?;
        if rx.compare(&epsilon) == Ordering::Less {
            if ry.compare(&epsilon) == Ordering::Less {
                return self.sample_nearest(position_bits);
            }
            let half_cell = b(cell.mul(&half, p)?.store_f32()?)?;
            let first =
                self.sample_nearest([position_bits[0], y.sub(&half_cell, p)?.store_f32()?])?;
            let second =
                self.sample_nearest([position_bits[0], half_cell.add(&y, p)?.store_f32()?])?;
            return Ok(second.add(&b(first.store_f32()?)?, p)?.mul(&half, p)?);
        }
        if ry.compare(&epsilon) == Ordering::Less {
            let half_cell = b(cell.mul(&half, p)?.store_f32()?)?;
            let first =
                self.sample_nearest([half_cell.add(&x, p)?.store_f32()?, position_bits[1]])?;
            let second =
                self.sample_nearest([x.sub(&half_cell, p)?.store_f32()?, position_bits[1]])?;
            return Ok(second.add(&b(first.store_f32()?)?, p)?.mul(&half, p)?);
        }
        let one = Binary::from_u32(1);
        let reciprocal = one.div(&cell, p)?;
        let tx = b(rx.mul(&reciprocal, p)?.store_f32()?)?;
        let ty = b(reciprocal.mul(&ry, p)?.store_f32()?)?;
        // This branch does not add the nearest sampler's index_bias.
        let shift = self.coordinate_shift & 31;
        let ix = (x.truncate_i32()? >> shift).clamp(0, self.width - 1);
        let iy = (y.truncate_i32()? >> shift).clamp(0, self.height - 1);
        let h00 = self.height_at(ix, iy)?;
        let h10 = if ix + 1 < self.width {
            self.height_at(ix + 1, iy)?
        } else {
            h00.clone()
        };
        let h01 = if iy + 1 < self.height {
            self.height_at(ix, iy + 1)?
        } else {
            h00.clone()
        };
        let h11 = if ix + 1 < self.width && iy + 1 < self.height {
            self.height_at(ix + 1, iy + 1)?
        } else {
            // The original repeats h00 even when one other neighbor is valid.
            h00.clone()
        };
        let w11 = ty.mul(&tx, p)?;
        let minus_x = one.sub(&tx, p)?;
        let w01 = b(ty.mul(&minus_x, p)?.store_f32()?)?;
        let minus_y = one.sub(&ty, p)?;
        let w10 = b(tx.mul(&minus_y, p)?.store_f32()?)?;
        let sum = minus_y
            .mul(&minus_x, p)?
            .mul(&h00, p)?
            .add(&w10.mul(&h10, p)?, p)?
            .add(&w01.mul(&h01, p)?, p)?;
        Ok(sum.add(&w11.mul(&h11, p)?, p)?)
    }
}

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
