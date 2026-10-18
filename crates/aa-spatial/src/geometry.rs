//! Native gameplay shape placement and inclusive tile extents.
//! Shapes come from definition getters, never model vertices or visual scale.
use crate::collision::{Grid, Shape};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const EPSILON: u32 = 0x38d1_b717;
fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn stored(value: Binary) -> Result<Binary> {
    b(value.store_f32()?)
}
fn absolute(value: Binary) -> Binary {
    if value.is_negative_nonzero() {
        value.negated()
    } else {
        value
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Placement {
    pub shape: Shape,
    pub facing: [u32; 2],
    pub position: [u32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileRect {
    pub min: [i32; 2],
    pub max: [i32; 2],
}
impl TileRect {
    pub fn contains(self, cell: [i32; 2]) -> bool {
        (0..2).all(|a| self.min[a] <= cell[a] && cell[a] <= self.max[a])
    }
    pub fn union(self, other: Self) -> Self {
        Self {
            min: [self.min[0].min(other.min[0]), self.min[1].min(other.min[1])],
            max: [self.max[0].max(other.max[0]), self.max[1].max(other.max[1])],
        }
    }
    /// Native5539B0 treats touching inclusive rectangles as the full remove /
    /// insert branch. Its overlap predicate is deliberately strict here.
    pub fn overlaps_strict(self, other: Self) -> bool {
        self.max[0] > other.min[0]
            && other.max[0] > self.min[0]
            && self.max[1] > other.min[1]
            && other.max[1] > self.min[1]
    }
}

/// Native46DAB0 with a newly captured descriptor (cached orientation word0).
/// 3 means |facingX| < epsilon, 2 means |facingY| < epsilon, 1 is general.
pub fn orientation(facing: [u32; 2]) -> Result<u32> {
    let epsilon = b(EPSILON)?;
    if absolute(b(facing[0])?).compare(&epsilon) == Ordering::Less {
        Ok(3)
    } else if absolute(b(facing[1])?).compare(&epsilon) == Ordering::Less {
        Ok(2)
    } else {
        Ok(1)
    }
}

/// Original46E540. General box corners preserve native stored cross-products,
/// extended direct products, sequential adds, and final binary32 corner stores.
pub fn world_bounds(placement: &Placement, p: Precision) -> Result<[u32; 4]> {
    let shape = &placement.shape;
    let [x, y] = placement.position.map(b);
    let x = x?;
    let y = y?;
    if shape.kind == 0 {
        return Ok([
            placement.position[0],
            placement.position[1],
            placement.position[0],
            placement.position[1],
        ]);
    }
    if shape.kind == 1 {
        let radius = b(shape.radius_bits)?;
        return Ok([
            x.sub(&radius, p)?.store_f32()?,
            y.sub(&radius, p)?.store_f32()?,
            radius.add(&x, p)?.store_f32()?,
            radius.add(&y, p)?.store_f32()?,
        ]);
    }
    if shape.kind > 3 {
        return Err(Error::World(format!(
            "unsupported native shape {}",
            shape.kind
        )));
    }
    let fx = b(placement.facing[0])?;
    let fy = b(placement.facing[1])?;
    let hx = b(shape.bounds[2])?;
    let hy = b(shape.bounds[3])?;
    let hy_fx = stored(hy.mul(&fx, p)?)?;
    let hx_neg_fx = stored(fx.clone().negated().mul(&hx, p)?)?;
    let hx_fy = fy.mul(&hx, p)?;
    let hy_fy = hy.mul(&fy, p)?;
    let corners = [
        [
            x.sub(&hx_fy, p)?.sub(&hy_fx, p)?.store_f32()?,
            y.sub(&hx_neg_fx, p)?.sub(&hy_fy, p)?.store_f32()?,
        ],
        [
            hx_fy.add(&x, p)?.sub(&hy_fx, p)?.store_f32()?,
            hx_neg_fx.add(&y, p)?.sub(&hy_fy, p)?.store_f32()?,
        ],
        [
            hx_fy.add(&x, p)?.add(&hy_fx, p)?.store_f32()?,
            hx_neg_fx.add(&y, p)?.add(&hy_fy, p)?.store_f32()?,
        ],
        [
            x.sub(&hx_fy, p)?.add(&hy_fx, p)?.store_f32()?,
            y.sub(&hx_neg_fx, p)?.add(&hy_fy, p)?.store_f32()?,
        ],
    ];
    // Original432670/4326C0 use two pairs, choosing the second on equality.
    let select = |a: u32, c: u32, order: Ordering| -> Result<u32> {
        Ok(if b(a)?.compare(&b(c)?) == order { a } else { c })
    };
    let reduce = |axis, order| -> Result<u32> {
        select(
            select(corners[0][axis], corners[1][axis], order)?,
            select(corners[2][axis], corners[3][axis], order)?,
            order,
        )
    };
    Ok([
        reduce(0, Ordering::Less)?,
        reduce(1, Ordering::Less)?,
        reduce(0, Ordering::Greater)?,
        reduce(1, Ordering::Greater)?,
    ])
}

/// Original5534F0 followed by552550. The X half-extent is stored before use;
/// the generic Y half-extent remains extended until integer conversion.
pub fn tile_rectangle(grid: &Grid, placement: &Placement, p: Precision) -> Result<TileRect> {
    if grid.dimensions.iter().any(|&v| v <= 0) {
        return Err(Error::InvalidGrid);
    }
    let shape = &placement.shape;
    let half = b(0x3f00_0000)?;
    let (rx, ry) = if shape.kind <= 1 {
        (b(shape.radius_bits)?, b(shape.radius_bits)?)
    } else {
        if shape.kind > 3 {
            return Err(Error::World(format!(
                "unsupported native shape {}",
                shape.kind
            )));
        }
        match orientation(placement.facing)? {
            3 => (b(shape.bounds[2])?, b(shape.bounds[3])?),
            2 => (b(shape.bounds[3])?, b(shape.bounds[2])?),
            _ => {
                let bounds = world_bounds(placement, p)?;
                (
                    stored(absolute(b(bounds[2])?.sub(&b(bounds[0])?, p)?).mul(&half, p)?)?,
                    absolute(b(bounds[3])?.sub(&b(bounds[1])?, p)?).mul(&half, p)?,
                )
            }
        }
    };
    let mut rect = TileRect {
        min: [0; 2],
        max: [0; 2],
    };
    for (axis, radius) in [rx, ry].into_iter().enumerate() {
        let position = b(placement.position[axis])?;
        let low = position.sub(&radius, p)?.truncate_i32()? >> (grid.coordinate_shift & 31);
        let high = if radius.compare(&b(EPSILON)?) == Ordering::Less {
            low
        } else {
            position
                .add(&radius, p)?
                .sub(&b(EPSILON)?, p)?
                .truncate_i32()?
                >> (grid.coordinate_shift & 31)
        };
        rect.min[axis] = low.clamp(0, grid.dimensions[axis] - 1);
        rect.max[axis] = high.clamp(0, grid.dimensions[axis] - 1);
    }
    Ok(rect)
}

/// Original4EE700. Virtual+4 of an ordinary actor returns blocking shape+4;
/// the comparison is inclusive: radius*4 >= trunc(cell_size).
pub fn marks_blocker(
    flags: u32,
    radius_bits: u32,
    cell_size_bits: u32,
    p: Precision,
) -> Result<bool> {
    if flags & 0x2000_0400 != 0x2000_0400 {
        return Ok(false);
    }
    let cell = Binary::from_i32(b(cell_size_bits)?.truncate_i32()?);
    Ok(cell.compare(&b(radius_bits)?.mul(&Binary::from_u32(4), p)?) != Ordering::Greater)
}
