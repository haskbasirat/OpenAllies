//! Low-path window grid (solver+0x44): a fixed sub-cell lattice placed around
//! the actor before every solve. Native layout: +0 world grid, +4/+8 origin
//! tile, +0xC/+0x10 origin (binary32), +0x14/+0x18 lattice offset, +0x1C sub
//! shift, +0x20 mask, +0x24/+0x28 size in sub-cells, +0x2C/+0x30 extent,
//! +0x34 cells of 0xC bytes (world tile pointer, visit stamp, predecessor).
use super::{Error, HALF, ONE, Result, TWO_POW_32, ZERO, f, ftol, st};
use crate::search::SearchGrid;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Arguments of 0x53BB90 / 0x53A490.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowConfig {
    pub sub_shift: u32,
    pub width_tiles: i32,
    pub height_tiles: i32,
}
impl WindowConfig {
    /// GWorld init 0x4F2A80..0x4F2A89 pushes (2, 0x10, 0x10): 16x16 world
    /// tiles split into 4x4 sub-cells, a 64x64 lattice. The size bounds every
    /// low-path route, so it stays an explicit search configuration.
    pub const NATIVE: Self = Self {
        sub_shift: 2,
        width_tiles: 16,
        height_tiles: 16,
    };
}

/// Window cell words +4 (visit stamp, compared with the world epoch) and +8
/// (predecessor direction). Word +0, the world tile pointer, is rewritten for
/// every cell by each placement and is derived from the origin here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowCell {
    pub stamp: u32,
    pub direction: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Window {
    pub origin_tile: [i32; 2],
    pub origin_bits: [u32; 2],
    pub offset_bits: [u32; 2],
    pub sub_shift: u32,
    pub mask: i32,
    pub size: [i32; 2],
    /// Both words come from the width (0x53A490 multiplies the width twice).
    pub extent_bits: [u32; 2],
    pub cells: Vec<WindowCell>,
}

fn less(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Less
}

impl Window {
    /// 0x53A490. The lattice offset starts as the static zero vector 0xB279C0.
    pub fn new(grid: &SearchGrid, config: WindowConfig, p: Precision) -> Result<Self> {
        let shift = config.sub_shift & 31;
        let unit = 1i32.wrapping_shl(shift);
        let inverse = f(ONE)?.div(&Binary::from_i32(unit), p)?;
        let width = config.width_tiles.wrapping_shl(shift);
        let height = config.height_tiles.wrapping_shl(shift);
        let extent = st(&Binary::from_i32(width)
            .mul(&f(grid.cell_size_bits)?, p)?
            .mul(&inverse, p)?)?;
        if width <= 0 || height <= 0 {
            return Err(Error::InvalidWindow);
        }
        let count = (width as usize)
            .checked_mul(height as usize)
            .ok_or(Error::InvalidWindow)?;
        let mut cells = Vec::new();
        cells.try_reserve_exact(count)?;
        cells.resize(count, WindowCell::default());
        Ok(Self {
            origin_tile: [0, 0],
            origin_bits: [ZERO, ZERO],
            offset_bits: [ZERO, ZERO],
            sub_shift: config.sub_shift,
            mask: unit.wrapping_sub(1),
            size: [width, height],
            extent_bits: [extent, extent],
            cells,
        })
    }
    fn shift(&self) -> u32 {
        self.sub_shift & 31
    }
    /// Native `cells + (width * y + x) * 0xC`.
    pub fn cell_index(&self, cell: [i32; 2]) -> Result<usize> {
        let index = i64::from(self.size[0]) * i64::from(cell[1]) + i64::from(cell[0]);
        if index < 0 || index >= self.cells.len() as i64 {
            return Err(Error::WindowCell(index));
        }
        Ok(index as usize)
    }
    /// World tile written into window cell `index` by the last placement.
    pub fn tile(&self, grid: &SearchGrid, index: usize) -> Result<[i32; 2]> {
        let width = self.size[0] as usize;
        let x = (index % width) as i32 >> self.shift();
        let y = (index / width) as i32 >> self.shift();
        let linear = i64::from(grid.width) * i64::from(y.wrapping_add(self.origin_tile[1]))
            + i64::from(x.wrapping_add(self.origin_tile[0]));
        if linear < 0 || linear >= grid.cells.len() as i64 || grid.width <= 0 {
            return Err(Error::WorldTile(linear));
        }
        let width = i64::from(grid.width);
        Ok([(linear % width) as i32, (linear / width) as i32])
    }
    pub fn tile_flags(&self, grid: &SearchGrid, index: usize) -> Result<u32> {
        let tile = self.tile(grid, index)?;
        let linear = tile[1] as usize * grid.width as usize + tile[0] as usize;
        Ok(grid.cells[linear].flags20)
    }
    /// 0x53B1F0: center the window on the start tile, clamp it to the world,
    /// and shift the lattice so the start lies on its sub-cell center.
    pub fn place(&mut self, grid: &SearchGrid, start: [u32; 2], p: Precision) -> Result<()> {
        self.offset_bits = [ZERO, ZERO];
        let world_shift = grid.coordinate_shift & 31;
        let tile_x = ftol(&f(start[0])?)? >> world_shift;
        let tile_y = ftol(&f(start[1])?)? >> world_shift;
        let tiles_w = self.size[0] >> self.shift();
        let tiles_h = self.size[1] >> self.shift();
        let clamp = |origin: i32, limit: i32| {
            if origin < 0 {
                0
            } else if origin > limit {
                limit
            } else {
                origin
            }
        };
        self.origin_tile = [
            clamp(
                tile_x.wrapping_sub(tiles_w / 2),
                grid.width.wrapping_sub(tiles_w).wrapping_sub(1),
            ),
            clamp(
                tile_y.wrapping_sub(tiles_h / 2),
                grid.height.wrapping_sub(tiles_h).wrapping_sub(1),
            ),
        ];
        self.origin_bits = [
            st(&Binary::from_i32(
                self.origin_tile[0].wrapping_shl(world_shift),
            ))?,
            st(&Binary::from_i32(
                self.origin_tile[1].wrapping_shl(world_shift),
            ))?,
        ];
        let sub_size =
            f(grid.cell_size_bits)?.div(&Binary::from_i32(1i32.wrapping_shl(self.shift())), p)?;
        let cell_y = ftol(
            &f(start[1])?
                .sub(&f(self.origin_bits[1])?, p)?
                .div(&sub_size, p)?,
        )?;
        let cell_x = ftol(
            &f(start[0])?
                .sub(&f(self.origin_bits[0])?, p)?
                .div(&sub_size, p)?,
        )?;
        let center = self.center(grid, [cell_x, cell_y], p)?;
        self.offset_bits = [
            st(&f(start[0])?.sub(&f(center[0])?, p)?)?,
            st(&f(start[1])?.sub(&f(center[1])?, p)?)?,
        ];
        Ok(())
    }
    /// 0x53AAC0: world position of a sub-cell center (X partial sum stored to
    /// binary32 before the offset is added; Y kept extended).
    pub fn center(&self, grid: &SearchGrid, cell: [i32; 2], p: Precision) -> Result<[u32; 2]> {
        let world_shift = grid.coordinate_shift & 31;
        let tile_x = self.origin_tile[0].wrapping_add(cell[0] >> self.shift());
        let tile_y = self.origin_tile[1].wrapping_add(cell[1] >> self.shift());
        let corner_x = Binary::from_i32(tile_x.wrapping_shl(world_shift));
        let corner_y = Binary::from_i32(tile_y.wrapping_shl(world_shift));
        let inverse = f(ONE)?.div(&Binary::from_i32(1i32.wrapping_shl(self.shift())), p)?;
        let cell_size = f(grid.cell_size_bits)?;
        let fraction = |value: i32| -> Result<Binary> {
            let mut v = Binary::from_i32(value);
            if value < 0 {
                v = v.add(&f(TWO_POW_32)?, p)?;
            }
            Ok(v.add(&f(HALF)?, p)?.mul(&cell_size, p)?.mul(&inverse, p)?)
        };
        let x_part = st(&fraction(self.mask & cell[0])?.add(&corner_x, p)?)?;
        let y_part = fraction(self.mask & cell[1])?.add(&corner_y, p)?;
        Ok([
            st(&f(x_part)?.add(&f(self.offset_bits[0])?, p)?)?,
            st(&y_part.add(&f(self.offset_bits[1])?, p)?)?,
        ])
    }
    /// 0x53A570: world position to sub-cell (Y converted first).
    pub fn world_to_cell(&self, grid: &SearchGrid, xy: [u32; 2], p: Precision) -> Result<[i32; 2]> {
        let sub_size =
            f(grid.cell_size_bits)?.div(&Binary::from_i32(1i32.wrapping_shl(self.shift())), p)?;
        let y = ftol(
            &f(xy[1])?
                .sub(&f(self.origin_bits[1])?, p)?
                .div(&sub_size, p)?,
        )?;
        let x = ftol(
            &f(xy[0])?
                .sub(&f(self.origin_bits[0])?, p)?
                .div(&sub_size, p)?,
        )?;
        Ok([x, y])
    }
    /// Shared bounds test of 0x539E90 and 0x53A6F0: lower edge inclusive,
    /// upper edge (extent + origin, extended sum) exclusive.
    pub fn contains(&self, xy: [u32; 2], p: Precision) -> Result<bool> {
        for ((bits, origin), extent) in xy.into_iter().zip(self.origin_bits).zip(self.extent_bits) {
            let value = f(bits)?;
            let origin = f(origin)?;
            if less(&value, &origin) {
                return Ok(false);
            }
            let upper = f(extent)?.add(&origin, p)?;
            if upper.compare(&value) != Ordering::Greater {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// 0x53A990: bring `point` into the window along the segment towards
    /// `start`, then clamp it to [origin, origin + extent - 0.0001].
    pub fn clip(&self, start: [u32; 2], point: &mut [u32; 2], p: Precision) -> Result<()> {
        let [ox, oy] = self.origin_bits;
        let right = st(&f(self.extent_bits[0])?.add(&f(ox)?, p)?)?;
        let top = st(&f(oy)?.add(&f(self.extent_bits[1])?, p)?)?;
        let inside = !less(&f(point[0])?, &f(ox)?)
            && f(point[0])?.compare(&f(right)?) != Ordering::Greater
            && !less(&f(point[1])?, &f(oy)?)
            && f(point[1])?.compare(&f(top)?) != Ordering::Greater;
        if inside {
            return Ok(());
        }
        if let Some(hit) = intersect([ox, oy, right, top], *point, start, p)? {
            *point = hit;
        }
        let epsilon = f(super::EPSILON)?;
        let limit_x = st(&f(self.extent_bits[0])?.add(&f(ox)?, p)?.sub(&epsilon, p)?)?;
        if f(ox)?.compare(&f(point[0])?) == Ordering::Greater {
            point[0] = ox;
        } else if f(point[0])?.compare(&f(limit_x)?) == Ordering::Greater {
            point[0] = limit_x;
        }
        let limit_y = st(&f(oy)?.add(&f(self.extent_bits[1])?, p)?.sub(&epsilon, p)?)?;
        if f(oy)?.compare(&f(point[1])?) == Ordering::Greater {
            point[1] = oy;
        } else if f(point[1])?.compare(&f(limit_y)?) == Ordering::Greater {
            point[1] = limit_y;
        }
        Ok(())
    }
}

/// 0x53A820 with the rectangle {x0, y0, x1, y1}: first edge (bottom, right,
/// top, left) crossed by segment a->b, interpolated from `a`.
fn intersect(rect: [u32; 4], a: [u32; 2], b: [u32; 2], p: Precision) -> Result<Option<[u32; 2]>> {
    let [x0, y0, x1, y1] = rect;
    let edges = [
        ([x0, y0], [x1, y0]),
        ([x1, y0], [x1, y1]),
        ([x1, y1], [x0, y1]),
        ([x0, y1], [x0, y0]),
    ];
    for (c, d) in edges {
        if let Some(t) = segment(a, b, c, d, p)? {
            let t = f(t)?;
            let rest = f(ONE)?.sub(&t, p)?;
            let tail_y = st(&t.mul(&f(b[1])?, p)?)?;
            let head_y = st(&rest.mul(&f(a[1])?, p)?)?;
            let x = rest.mul(&f(a[0])?, p)?.add(&t.mul(&f(b[0])?, p)?, p)?;
            let y = f(head_y)?.add(&f(tail_y)?, p)?;
            return Ok(Some([st(&x)?, st(&y)?]));
        }
    }
    Ok(None)
}

/// 0x53A3A0: parameter along p1->p2 of its crossing with p3->p4 when both
/// stored parameters lie in [0, 1].
pub fn segment(
    p1: [u32; 2],
    p2: [u32; 2],
    p3: [u32; 2],
    p4: [u32; 2],
    p: Precision,
) -> Result<Option<u32>> {
    let a = f(p4[1])?.sub(&f(p3[1])?, p)?;
    let b = f(p2[0])?.sub(&f(p1[0])?, p)?;
    let c = f(p2[1])?.sub(&f(p1[1])?, p)?;
    let d = f(st(&f(p4[0])?.sub(&f(p3[0])?, p)?)?)?;
    let denominator = st(&b.mul(&a, p)?.sub(&d.mul(&c, p)?, p)?)?;
    if f(denominator)?.is_zero() {
        return Ok(None);
    }
    let e = f(p1[1])?.sub(&f(p3[1])?, p)?;
    let g = f(p1[0])?.sub(&f(p3[0])?, p)?;
    let inverse = f(ONE)?.div(&f(denominator)?, p)?;
    let t1 = st(&e.mul(&d, p)?.sub(&g.mul(&a, p)?, p)?.mul(&inverse, p)?)?;
    let t2 = st(&e.mul(&b, p)?.sub(&g.mul(&c, p)?, p)?.mul(&inverse, p)?)?;
    let zero = f(ZERO)?;
    let one = f(ONE)?;
    for t in [t1, t2] {
        let t = f(t)?;
        if less(&t, &zero) || t.compare(&one) == Ordering::Greater {
            return Ok(None);
        }
    }
    Ok(Some(t1))
}
