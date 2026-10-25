//! Original destination adjustment (0x53d340) and map-interior clamp (0x552640).
use aa_numeric::{Fast, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("native endpoint adjustment requires a positive integer grid step")]
    InvalidStep,
    #[error("native endpoint adjustment cannot make finite progress")]
    NoProgress,
    #[error("endpoint geometry boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bounds {
    /// Native map grid+8.
    pub cell_size_bits: u32,
    /// Native map grid+0xc/+0x10, in world coordinates.
    pub extent_bits: [u32; 2],
}
pub trait World {
    /// Original 0x53ce40: bounds/cell filtering followed by actor/world tests.
    /// This boundary may update the host's collision descriptor.
    fn acceptable(&mut self, original_start: [u32; 2], candidate: [u32; 2]) -> Result<bool>;
    /// Original 0x53ca30: candidate displacement around a blocking object.
    fn displace_blocked(&mut self, fallback: [u32; 2], candidate: &mut [u32; 2]) -> Result<bool>;
}
fn b(bits: u32) -> Result<Fast> {
    Ok(Fast::from_f32_bits(bits)?)
}

pub fn clamp_to_interior(bounds: Bounds, point: &mut [u32; 2], p: Precision) -> Result<()> {
    let min = b(bounds.cell_size_bits)?;
    for (axis, value) in point.iter_mut().enumerate() {
        let max = b(bounds.extent_bits[axis])?
            .sub(&min, p)?
            .sub(&b(0x38d1b717)?, p)?
            .store_f32()?;
        let current = b(*value)?;
        if min.compare(&current) == Ordering::Greater {
            *value = bounds.cell_size_bits;
        } else if current.compare(&b(max)?) == Ordering::Greater {
            *value = max;
        }
    }
    Ok(())
}

/// Recover a collision-accepted destination by the original ordered attempts.
/// The endpoint is updated even when the function returns false. No route or
/// fallback collision response is invented. Bounds are stable for this call.
pub fn adjust<W: World>(
    original_start: [u32; 2],
    destination: &mut [u32; 2],
    fallback: [u32; 2],
    bounds: Bounds,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    let step = b(bounds.cell_size_bits)?.truncate_i32()?;
    if step <= 0 {
        return Err(Error::InvalidStep);
    }
    let step = b(Fast::from_i32(step).store_f32()?)?;
    let dx = b(b(destination[0])?.sub(&b(fallback[0])?, p)?.store_f32()?)?;
    let dy = b(destination[1])?.sub(&b(fallback[1])?, p)?;
    let mut direction = [dx.store_f32()?, dy.store_f32()?];
    let distance = dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?.sqrt(p)?;
    let mut remaining = distance.store_f32()?;
    if distance.compare(&Fast::from_u32(0)) == Ordering::Greater {
        let ratio = step.div(&b(remaining)?, p)?;
        direction = [
            dx.mul(&ratio, p)?.store_f32()?,
            dy.mul(&ratio, p)?.store_f32()?,
        ];
    }
    let success = loop {
        let mut candidate = *destination;
        clamp_to_interior(bounds, &mut candidate, p)?;
        if world.acceptable(original_start, candidate)?
            || (world.displace_blocked(fallback, &mut candidate)?
                && world.acceptable(original_start, candidate)?)
        {
            *destination = candidate;
            break true;
        }
        if b(remaining)?.compare(&Fast::from_u32(0)) != Ordering::Greater {
            break false;
        }
        destination[0] = b(destination[0])?.sub(&b(direction[0])?, p)?.store_f32()?;
        destination[1] = b(destination[1])?.sub(&b(direction[1])?, p)?.store_f32()?;
        let previous = remaining;
        let next = b(remaining)?.sub(&step, p)?;
        remaining = next.store_f32()?;
        if next.compare(&Fast::from_u32(0)) != Ordering::Greater {
            *destination = fallback;
            remaining = 0;
        } else if remaining == previous {
            return Err(Error::NoProgress);
        }
    };
    let mut x = b(destination[0])?.sub(&b(original_start[0])?, p)?;
    let mut y = b(destination[1])?.sub(&b(original_start[1])?, p)?;
    if x.is_negative_nonzero() {
        x = x.negated();
    }
    if y.is_negative_nonzero() {
        y = y.negated();
    }
    Ok(success && y.add(&x, p)?.compare(&b(0x3951b717)?) != Ordering::Less)
}
