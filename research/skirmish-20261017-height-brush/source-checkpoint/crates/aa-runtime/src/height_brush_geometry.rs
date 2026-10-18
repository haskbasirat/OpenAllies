//!Actual46EF30 point tests and46F280/46E150 distance for550B00.
//!Source geometry and current point-orientation cells are required inputs.
use aa_heightfield::brush::{self, Effects, Geometry};
use aa_numeric::{Binary, Precision};
use aa_spatial::{collision::Shape, geometry::Placement};
use std::cmp::Ordering;
pub(crate) trait Sources {
    fn orientation(&mut self) -> brush::Result<[u32; 3]>;
    fn point_shape(&mut self) -> brush::Result<Shape>;
}
pub(crate) struct Placed<'a> {
    pub placement: &'a Placement,
    pub orientation: Option<[u32; 3]>,
}
fn b(v: u32) -> brush::Result<Binary> {
    Ok(Binary::from_f32_bits(v)?)
}
fn store(v: Binary) -> brush::Result<Binary> {
    b(v.store_f32()?)
}
fn fail(e: impl std::fmt::Display) -> brush::Error {
    brush::Error::Service(e.to_string())
}
fn inside(shape: &Shape, xy: [u32; 2], p: Precision) -> brush::Result<bool> {
    let x = b(xy[0])?;
    let y = b(xy[1])?;
    match shape.kind {
        0 => Ok(x.is_zero() && y.is_zero()),
        1 => Ok(x
            .mul(&x, p)?
            .add(&y.mul(&y, p)?, p)?
            .add(&b(0x38d1b717)?, p)?
            .compare(&b(shape.radius_bits)?.mul(&b(shape.radius_bits)?, p)?)
            != Ordering::Greater),
        2 | 3 => Ok(x.compare(&b(shape.bounds[0])?) != Ordering::Less
            && x.compare(&b(shape.bounds[2])?) != Ordering::Greater
            && y.compare(&b(shape.bounds[1])?) != Ordering::Less
            && y.compare(&b(shape.bounds[3])?) != Ordering::Greater),
        _ => Err(fail("46EC70 unsupported native Shape kind")),
    }
}
impl<E: Effects + Sources> Geometry<E> for Placed<'_> {
    fn center(&self) -> [u32; 2] {
        self.placement.position
    }
    fn radius(&self) -> u32 {
        self.placement.shape.radius_bits
    }
    fn prepare(&mut self, effects: &mut E, _: Precision) -> brush::Result<()> {
        self.orientation = Some(effects.orientation()?);
        Ok(())
    }
    fn contains(&mut self, effects: &mut E, xy: [u32; 2], pc: Precision) -> brush::Result<bool> {
        let seed = self
            .orientation
            .ok_or_else(|| fail("550B00 actual point orientation was not read"))?;
        let shape = effects.point_shape()?;
        if shape.kind != 0 {
            return Err(fail("550B00 actual global PointShape is not POINT"));
        }
        aa_spatial::shape_overlap::overlaps(
            self.placement,
            &Placement {
                shape,
                facing: [seed[0], seed[1]],
                position: xy,
            },
            pc,
        )
        .map_err(fail)
    }
    fn distance(&mut self, _: &mut E, point: [u32; 2], pc: Precision) -> brush::Result<Binary> {
        let p = self.placement;
        let shape = &p.shape;
        let dx = store(b(point[0])?.sub(&b(p.position[0])?, pc)?)?;
        let dy = store(b(point[1])?.sub(&b(p.position[1])?, pc)?)?;
        let (fx, fy) = (b(p.facing[0])?, b(p.facing[1])?);
        let closest = match shape.kind {
            0 => [Binary::from_u32(0), Binary::from_u32(0)],
            1 => {
                let length = dy.mul(&dy, pc)?.add(&dx.mul(&dx, pc)?, pc)?.sqrt(pc)?;
                if length.compare(&b(0x38d1b717)?) == Ordering::Greater {
                    let inverse = Binary::from_u32(1).div(&length, pc)?;
                    [
                        store(store(inverse.mul(&dx, pc)?)?.mul(&b(shape.radius_bits)?, pc)?)?,
                        store(store(inverse.mul(&dy, pc)?)?.mul(&b(shape.radius_bits)?, pc)?)?,
                    ]
                } else {
                    [Binary::from_u32(0), Binary::from_u32(0)]
                }
            }
            2 | 3 => {
                let local_x = store(fy.mul(&dx, pc)?.sub(&dy.mul(&fx, pc)?, pc)?)?;
                let local_y = fx.mul(&dx, pc)?.add(&fy.mul(&dy, pc)?, pc)?;
                let clamp = |v: Binary, lo: u32, hi: u32| -> brush::Result<Binary> {
                    Ok(if v.compare(&b(lo)?) == Ordering::Less {
                        b(lo)?
                    } else if v.compare(&b(hi)?) == Ordering::Greater {
                        b(hi)?
                    } else {
                        v
                    })
                };
                let x = clamp(local_x, shape.bounds[0], shape.bounds[2])?;
                let y = clamp(local_y, shape.bounds[1], shape.bounds[3])?;
                [
                    store(fy.mul(&x, pc)?.add(&fx.mul(&y, pc)?, pc)?)?,
                    store(y.mul(&fy, pc)?.sub(&fx.mul(&x, pc)?, pc)?)?,
                ]
            }
            _ => return Err(fail("46E150 unsupported native Shape kind")),
        };
        let x = store(
            b(p.position[0])?
                .add(&closest[0], pc)?
                .sub(&b(point[0])?, pc)?,
        )?;
        let y = closest[1]
            .add(&b(p.position[1])?, pc)?
            .sub(&b(point[1])?, pc)?;
        let distance = store(y.mul(&y, pc)?.add(&x.mul(&x, pc)?, pc)?.sqrt(pc)?)?;
        if distance.compare(&b(shape.radius_bits)?) == Ordering::Less {
            let local = [
                dx.mul(&fy, pc)?.sub(&dy.mul(&fx, pc)?, pc)?.store_f32()?,
                dy.mul(&fy, pc)?.add(&dx.mul(&fx, pc)?, pc)?.store_f32()?,
            ];
            if inside(shape, local, pc)? {
                return Ok(Binary::from_u32(0));
            }
        }
        Ok(distance)
    }
}
