//! Native escape point for an actor standing inside other things, used by
//! the uai_sleep Think and element moves.
//!
//! * `4D1820(&point)`: stale head entries of the actor's `+110` overlap list
//!   (GTIDs that the `[B364C0]` registry no longer resolves) are unlinked and
//!   freed; the first live one is snapshotted (`4CF210`: its `+C` shape and
//!   `+20/+24` position) and handed to `4D04D0`. False on an empty list.
//! * `4D04D0(other, &point)`: the direction from the other thing to the actor
//!   (binary32 differences, `409230`-normalized, `(1, 0)` when both are
//!   zero) is tried rotated by `k * pi/4` for k = 0..7 (x87 `fcos`/`fsin` of
//!   `fild k * [A019F0]`) at the summed shape radii; the first candidate
//!   where the actor's spatial query (`554350` / `552250` / `553BA0`) finds
//!   nothing is written out.
//!
//! The x candidate is `binary32(rx * R + x)`, while the y candidate rounds
//! its product first: `binary32(y + binary32(R * ry))`.
//!
//! Oracle: research/phase8-combat/native_overlap_escape.py.
use crate::{Result, low_path};
use aa_numeric::{Binary, Precision};

/// `[A019F0]` binary32 pi/4.
const QUARTER_TURN: u32 = 0x3f49_0fdb;

pub trait World {
    /// GTID at the head of actor `+110`, None when the list is empty.
    fn overlap_head(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Unlinks and frees the head node (`+114` cleared when the list empties).
    fn pop_overlap_head(&mut self, actor: u32) -> Result<()>;
    /// Registry `[B364C0]+4+index*2` generation word.
    fn registry_generation(&mut self, index: u32) -> Result<u16>;
    /// Registry `[B364C0]+20004+index*4` object.
    fn registry_object(&mut self, index: u32) -> Result<Option<u32>>;
    /// Thing `+20/+24` (binary32 bits).
    fn position(&mut self, thing: u32) -> Result<[u32; 2]>;
    /// Thing virtual `+C` shape, its `+4` radius (the `4CF210` snapshot).
    fn snapshot_radius(&mut self, thing: u32) -> Result<u32>;
    /// Actor virtual `+10` shape, its `+4` radius (read every try).
    fn own_radius(&mut self, actor: u32) -> Result<u32>;
    /// x87 `fcos` and `fsin` of the extended angle (the native leaf).
    fn cos_sin(&mut self, angle: &Binary) -> Result<(Binary, Binary)>;
    /// The actor's spatial query at the point finds nothing.
    fn free_at(&mut self, actor: u32, point: [u32; 2]) -> Result<bool>;
}

/// The `4CF210` snapshot fields `4D04D0` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Other {
    pub radius: u32,
    pub position: [u32; 2],
}

fn f(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

fn st(value: &Binary) -> Result<u32> {
    Ok(value.store_f32()?)
}

fn is_zero(bits: u32) -> bool {
    // fucompp with [9F32DC]: only an equal compare skips the normalize.
    bits & 0x7fff_ffff == 0
}

/// Native `4D04D0(other, &point)` on `actor`.
pub fn escape_from(world: &mut impl World, actor: u32, other: Other, p: Precision) -> Result<Option<[u32; 2]>> {
    let own = world.position(actor)?;
    let mut direction = [
        st(&f(own[0])?.sub(&f(other.position[0])?, p)?)?,
        st(&f(own[1])?.sub(&f(other.position[1])?, p)?)?,
    ];
    if !is_zero(direction[0]) || !is_zero(direction[1]) {
        low_path::normalize(&mut direction, p).map_err(|e| crate::Error::World(e.to_string()))?;
    } else {
        direction = [0x3f80_0000, 0];
    }
    for k in 0..8 {
        let rotated = if k == 0 {
            direction
        } else {
            let angle = Binary::from_i32(k).mul(&f(QUARTER_TURN)?, p)?;
            let (c, s) = world.cos_sin(&angle)?;
            let (dx, dy) = (f(direction[0])?, f(direction[1])?);
            [
                st(&c.mul(&dx, p)?.sub(&s.mul(&dy, p)?, p)?)?,
                st(&s.mul(&dx, p)?.add(&c.mul(&dy, p)?, p)?)?,
            ]
        };
        let own_radius = world.own_radius(actor)?;
        let reach = f(other.radius)?.add(&f(own_radius)?, p)?;
        let x = st(&f(rotated[0])?.mul(&reach, p)?.add(&f(other.position[0])?, p)?)?;
        let ry = st(&reach.mul(&f(rotated[1])?, p)?)?;
        let y = st(&f(other.position[1])?.add(&f(ry)?, p)?)?;
        if world.free_at(actor, [x, y])? {
            return Ok(Some([x, y]));
        }
    }
    Ok(None)
}

/// Native `4D1820(&point)` on `actor`.
pub fn escape_point(world: &mut impl World, actor: u32, p: Precision) -> Result<Option<[u32; 2]>> {
    while let Some(gtid) = world.overlap_head(actor)? {
        let index = gtid & 0xFFFF;
        if (gtid >> 16) as u16 == world.registry_generation(index)?
            && let Some(thing) = world.registry_object(index)?
        {
            let other = Other { radius: world.snapshot_radius(thing)?, position: world.position(thing)? };
            return escape_from(world, actor, other, p);
        }
        world.pop_overlap_head(actor)?;
    }
    Ok(None)
}
