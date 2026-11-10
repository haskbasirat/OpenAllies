//! Actor shape point `0x4CFB70` (the launch origin `0x4BE780` takes for a
//! non-Kingdom source whose virtual +B4 holds when the ability definition has
//! no projectile entry at +3C), with the shape operation `0x46E150` and the
//! in-place normalize `0x409230` it runs.
//!
//! For the actor classes whose virtual +B4 is true and +A8 false (vtables
//! `A273F0` and `A29490`), virtual +0 is `4D1E30` (the f32 at definition
//! +54, reached through virtual +10 `4EA020`) and virtual +14 is `4EA030`
//! (the shape at definition +68 when the byte at definition +80 is nonzero,
//! else the one at definition +50). [`ShapeDefinition`] holds those fields.
//!
//! Proven bit for bit at PC24, PC53 and PC64 by
//! research/phase8-combat/native_launch_point.py (real vtables, no seams).
use crate::Result;
use aa_body::F32;
use aa_numeric::{Fast, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Six words of a definition shape (+0 type, +4 radius, +8/+C minimum,
/// +10/+14 maximum of the box in the facing frame).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shape {
    pub kind: u32,
    pub radius: F32,
    pub min: [F32; 2],
    pub max: [F32; 2],
}

/// The definition fields `0x4CFB70` reads through the actor virtuals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShapeDefinition {
    /// Definition +50 (its +4 word, definition +54, is also the scale).
    pub base: Shape,
    /// Definition +68.
    pub alternate: Shape,
    /// Definition byte +80 is nonzero.
    pub use_alternate: bool,
    /// Definition +1DC, added to the actor's z.
    pub height: F32,
}

/// The actor words `0x4CFB70` reads directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShapeActor {
    /// Actor +20, +24, +28.
    pub position: [F32; 3],
    /// Actor +2C, +30.
    pub facing: [F32; 2],
}

/// Native constant pair at `B279C0` (0.0, 0.0 in the image), returned for a
/// shape type other than 1, 2 or 3.
pub const DEFAULT_SHAPE_POINT: [F32; 2] = [F32(0), F32(0)];
/// Normalize length gate at `9F3228` (1e-4).
const NORMALIZE_MINIMUM: u32 = 0x38d1_b717;

fn load(value: F32) -> Result<Fast> {
    Ok(Fast::from_f32_bits(value.0)?)
}

/// `0x409230` on a binary32 pair: unchanged direction scaled to unit length
/// (each word stored once), or (0, 0) when the length is not above 1e-4.
pub fn normalize(v: [F32; 2], p: Precision) -> Result<[F32; 2]> {
    let x = load(v[0])?;
    let y = load(v[1])?;
    let length = x.mul(&x, p)?.add(&y.mul(&y, p)?, p)?.sqrt(p)?;
    if length.compare(&load(F32(NORMALIZE_MINIMUM))?) != Ordering::Greater {
        return Ok([F32(0), F32(0)]);
    }
    let inverse = Fast::from_i32(1).div(&length, p)?;
    Ok([
        F32(inverse.mul(&x, p)?.store_f32()?),
        F32(inverse.mul(&y, p)?.store_f32()?),
    ])
}

/// `0x46E150`: the shape's point in direction `point` for the frame `axis`
/// (both binary32 pairs).
pub fn shape_point(shape: &Shape, axis: [F32; 2], point: [F32; 2], p: Precision) -> Result<[F32; 2]> {
    match shape.kind {
        1 => {
            let unit = normalize(point, p)?;
            let radius = load(shape.radius)?;
            Ok([
                F32(load(unit[0])?.mul(&radius, p)?.store_f32()?),
                F32(load(unit[1])?.mul(&radius, p)?.store_f32()?),
            ])
        }
        2 | 3 => {
            let (ax, ay) = (load(axis[0])?, load(axis[1])?);
            let (px, py) = (load(point[0])?, load(point[1])?);
            // Across the axis: stored once to binary32 before the clamp.
            let across = F32(ay.mul(&px, p)?.sub(&py.mul(&ax, p)?, p)?.store_f32()?);
            // Along the axis: kept in the register through the clamp.
            let along = ay.mul(&py, p)?.add(&ax.mul(&px, p)?, p)?;
            let across = if load(across)?.compare(&load(shape.min[0])?) == Ordering::Less {
                shape.min[0]
            } else if load(across)?.compare(&load(shape.max[0])?) == Ordering::Greater {
                shape.max[0]
            } else {
                across
            };
            let along = if along.compare(&load(shape.min[1])?) == Ordering::Less {
                load(shape.min[1])?
            } else if along.compare(&load(shape.max[1])?) == Ordering::Greater {
                load(shape.max[1])?
            } else {
                along
            };
            let across = load(across)?;
            let x = ax.mul(&along, p)?.add(&ay.mul(&across, p)?, p)?;
            let y = along.mul(&ay, p)?.sub(&ax.mul(&across, p)?, p)?;
            Ok([F32(x.store_f32()?), F32(y.store_f32()?)])
        }
        _ => Ok(DEFAULT_SHAPE_POINT),
    }
}

/// The launch origin's source (`0x4BE780` argument 2) as its virtuals see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchSource<'a> {
    /// No source object.
    None,
    /// Virtual +A8 true (the Kingdom class, vtable `A299D8`).
    Kingdom,
    /// Virtual +A8 false; `ordinary` is virtual +B4 (true for vtables
    /// `A273F0` / `A29490`, false for example for `A28A00`).
    Actor {
        actor: &'a ShapeActor,
        definition: &'a ShapeDefinition,
        ordinary: bool,
    },
}

/// The local definition words `0x4BE780` reads (it is `this`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchLocal {
    /// The ability definition's +3C (reached through this object's first
    /// word) is nonzero.
    pub has_entry: bool,
    /// +8, +C, +10: the spawn offset in the source's facing frame.
    pub offset: [F32; 3],
}

/// `0x4BE780`: the launch origin for `source`, `fallback` the caller's point.
pub fn launch_origin(local: &LaunchLocal, source: LaunchSource<'_>, fallback: [F32; 3], p: Precision) -> Result<[F32; 3]> {
    match source {
        LaunchSource::Kingdom => Ok(fallback),
        LaunchSource::None if !local.has_entry => Ok(fallback),
        // 4BE740 copies the point; the offset's z is added.
        LaunchSource::None => Ok([
            fallback[0],
            fallback[1],
            F32(load(fallback[2])?.add(&load(local.offset[2])?, p)?.store_f32()?),
        ]),
        LaunchSource::Actor { actor, definition, ordinary } => {
            if local.has_entry {
                let [ox, oy, oz] = local.offset.map(load);
                let (ox, oy, oz) = (ox?, oy?, oz?);
                let (fx, fy) = (load(actor.facing[0])?, load(actor.facing[1])?);
                let x = ox.mul(&fy, p)?.add(&oy.mul(&fx, p)?, p)?;
                let y = oy.mul(&fy, p)?.sub(&ox.mul(&fx, p)?, p)?;
                Ok([
                    F32(x.add(&load(actor.position[0])?, p)?.store_f32()?),
                    F32(y.add(&load(actor.position[1])?, p)?.store_f32()?),
                    F32(oz.add(&load(actor.position[2])?, p)?.store_f32()?),
                ])
            } else if ordinary {
                actor_shape_point(actor, definition, p)
            } else {
                Ok(actor.position)
            }
        }
    }
}

/// `0x4CFB70`: the actor's shape point in world space.
pub fn actor_shape_point(actor: &ShapeActor, definition: &ShapeDefinition, p: Precision) -> Result<[F32; 3]> {
    // Virtual +0 (4D1E30): the f32 at definition +54, loaded exactly.
    let scale = load(definition.base.radius)?;
    let scaled = [
        F32(scale.mul(&load(actor.facing[0])?, p)?.store_f32()?),
        F32(scale.mul(&load(actor.facing[1])?, p)?.store_f32()?),
    ];
    // Virtual +14 (4EA030).
    let shape = if definition.use_alternate {
        &definition.alternate
    } else {
        &definition.base
    };
    let local = shape_point(shape, actor.facing, scaled, p)?;
    Ok([
        F32(load(local[0])?.add(&load(actor.position[0])?, p)?.store_f32()?),
        F32(load(local[1])?.add(&load(actor.position[1])?, p)?.store_f32()?),
        F32(load(actor.position[2])?.add(&load(definition.height)?, p)?.store_f32()?),
    ])
}
