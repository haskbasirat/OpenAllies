//!551140/551050/550B00 mode0/1 and558CE0/559470 on the original height owner.
//! A temporary native pending array is processed completely before550A10
//! commits samples. Source geometry, current terrain slopes and notifications
//! are mandatory reached services; undefined caller scratch is never defaulted.
use crate::binding::InitializedHeightfield;
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::{cmp::Ordering, collections::BTreeMap};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("height brush service: {0}; earlier stores remain committed")]
    Service(String),
    #[error("height brush invalid field or original integer domain")]
    Invalid,
}
pub type Result<T> = std::result::Result<T, Error>;
///Only the retained fields read/written by this bounded native brush family.
#[derive(Debug, Serialize)]
pub struct State {
    round_brush: bool,
    dirty_bounds: [u32; 4],
}
impl State {
    ///550940 writes byte20 and30/34/38/3C tozero. No claim of the full ctor.
    pub fn from_height_map_constructor() -> Self {
        Self {
            round_brush: false,
            dirty_bounds: [0; 4],
        }
    }
    pub fn dirty_bounds(&self) -> [u32; 4] {
        self.dirty_bounds
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub mode: u32,
    pub height: u32,
    pub slope: u32,
    pub minimum: u32,
    pub maximum: u32,
    pub protect: bool,
    pub smooth_edge: bool,
    pub strength: u32,
    pub area: u8,
}
///The placed shape is immutable for this synchronous native operation.
pub trait Geometry<E: Effects> {
    fn center(&self) -> [u32; 2];
    fn radius(&self) -> u32;
    ///46E060/46D8D0 + reached[B39990+64] point seed, before the first row.
    fn prepare(&mut self, effects: &mut E, pc: Precision) -> Result<()>;
    fn contains(&mut self, effects: &mut E, xy: [u32; 2], pc: Precision) -> Result<bool>;
    fn distance(&mut self, effects: &mut E, xy: [u32; 2], pc: Precision) -> Result<Binary>;
}
pub trait Effects {
    ///5512BF reads settings+1C ONLY after accepted raster calls. This word is
    ///unwritten by4EB500 itself; callers must supply its proved actual origin.
    fn falloff_distance(&mut self) -> Result<u32>;
    ///558A70: the actual Map cell TerrainData+5C, None when outside the map.
    fn terrain_slope(&mut self, xy: [u32; 2]) -> Result<Option<u32>>;
    ///550660 protected-area diagnostic; returningfalse is separate from error.
    fn protected_diagnostic(
        &mut self,
        cell: [i32; 2],
        area: u8,
        requested: u8,
        current: u32,
        target: u32,
        result: u32,
    ) -> Result<()>;
    ///550430->50C310, after every sample/dirty-bound store has committed.
    fn notify(&mut self, center: [u32; 2], radius: u32) -> Result<()>;
}
#[derive(Clone, Copy, Debug)]
struct Pending {
    cell: [i32; 2],
    height: u32,
    depth: i32,
}
fn b(v: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(v)?)
}
fn store(v: Binary) -> Result<Binary> {
    b(v.store_f32()?)
}
fn abs(v: Binary) -> Binary {
    if v.is_negative_nonzero() {
        v.negated()
    } else {
        v
    }
}
fn at(field: &InitializedHeightfield, cell: [i32; 2]) -> Result<usize> {
    let f = &field.field;
    if cell[0] < 0 || cell[1] < 0 || cell[0] >= f.width || cell[1] >= f.height {
        return Err(Error::Invalid);
    }
    Ok((cell[1] as usize) * (f.width as usize) + (cell[0] as usize))
}
fn xy(field: &InitializedHeightfield, cell: [i32; 2]) -> Result<[u32; 2]> {
    let shift = field.field.coordinate_shift & 31;
    Ok([
        Binary::from_i32(cell[0].wrapping_shl(shift)).store_f32()?,
        Binary::from_i32(cell[1].wrapping_shl(shift)).store_f32()?,
    ])
}
fn bound(
    field: &InitializedHeightfield,
    position: u32,
    radius: u32,
    upper: bool,
    pc: Precision,
) -> Result<i32> {
    //The CRT receives a REAL8 store. Its integer-valued floor/ceil is then
    //multiplied by cell size BEFORE96CC4C and the native bias/shift.
    let cell = b(field.field.cell_size_bits)?;
    let p = if upper {
        b(position)?.add(&b(radius)?, pc)?
    } else {
        b(position)?.sub(&b(radius)?, pc)?
    };
    let quotient = p
        .div(&cell, pc)?
        .mul(&Binary::from_u32(1), Precision::Double53)?;
    let truncated = quotient.truncate_i32()?;
    let relation = quotient.compare(&Binary::from_i32(truncated));
    let rounded = if upper && relation == Ordering::Greater {
        truncated.checked_add(1).ok_or(Error::Invalid)?
    } else if !upper && relation == Ordering::Less {
        truncated.checked_sub(1).ok_or(Error::Invalid)?
    } else {
        truncated
    };
    Ok(Binary::from_i32(rounded)
        .mul(&cell, pc)?
        .truncate_i32()?
        .wrapping_add(field.field.index_bias)
        >> (field.field.coordinate_shift & 31))
}
fn pending_set(
    rows: &mut Vec<Pending>,
    keys: &mut BTreeMap<[i32; 2], usize>,
    cell: [i32; 2],
    height: u32,
    depth: i32,
) -> Result<()> {
    if let Some(&index) = keys.get(&cell) {
        rows[index].height = height;
        return Ok(());
    }
    if rows.len() >= i32::MAX as usize {
        return Err(Error::Invalid);
    }
    keys.insert(cell, rows.len());
    rows.push(Pending {
        cell,
        height,
        depth,
    });
    Ok(())
}
fn raster<E: Effects>(
    field: &InitializedHeightfield,
    state: &State,
    geometry: &mut impl Geometry<E>,
    settings: Settings,
    rows: &mut Vec<Pending>,
    keys: &mut BTreeMap<[i32; 2], usize>,
    effects: &mut E,
    pc: Precision,
) -> Result<bool> {
    match settings.mode {
        0 | 1 => {}
        2 => {
            return Err(Error::Service(
                "551050 mode2 smoothing551D10 owner required".into(),
            ));
        }
        _ => return Ok(true),
    }
    let center = geometry.center();
    let radius = geometry.radius();
    let f = &field.field;
    let min = [
        bound(field, center[0], radius, false, pc)?.clamp(0, f.width - 1),
        bound(field, center[1], radius, false, pc)?.clamp(0, f.height - 1),
    ];
    let max = [
        bound(field, center[0], radius, true, pc)?.clamp(0, f.width - 1),
        bound(field, center[1], radius, true, pc)?.clamp(0, f.height - 1),
    ];
    geometry.prepare(effects, pc)?;
    for y in min[1]..=max[1] {
        for x in min[0]..=max[0] {
            let cell = [x, y];
            let point = xy(field, cell)?;
            let strength = if !state.round_brush {
                if geometry.contains(effects, point, pc)? {
                    Some(Binary::from_u32(1))
                } else if settings.smooth_edge {
                    let d = geometry.distance(effects, point, pc)?;
                    let diagonal = b(f.cell_size_bits)?.mul(&b(0x3fb504f3)?, pc)?;
                    if d.compare(&diagonal) != Ordering::Greater {
                        Some(store(diagonal.sub(&d, pc)?.div(&diagonal, pc)?)?)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                let dx = store(b(center[0])?.sub(&b(point[0])?, pc)?)?;
                let dy = b(center[1])?.sub(&b(point[1])?, pc)?;
                let radius = b(radius)?;
                let distance = store(
                    dx.mul(&dx, pc)?
                        .add(&dy.mul(&dy, pc)?, pc)?
                        .sqrt(pc)?
                        .div(&radius, pc)?,
                )?;
                Some(store(
                    Binary::from_u32(1).sub(&distance.mul(&distance, pc)?, pc)?,
                )?)
            };
            let Some(strength) = strength else { continue };
            if strength.compare(&Binary::from_u32(0)) != Ordering::Greater {
                continue;
            }
            let index = at(field, cell)?;
            let old = b(f.height_bits[index])?;
            let target = if settings.mode == 1 {
                store(old.add(&b(settings.height)?, pc)?)?
            } else {
                b(settings.height)?
            };
            let target = if target.compare(&b(settings.minimum)?) == Ordering::Less {
                b(settings.minimum)?
            } else if target.compare(&b(settings.maximum)?) == Ordering::Greater {
                b(settings.maximum)?
            } else {
                target
            };
            let delta = target.sub(&old, pc)?;
            if abs(delta.clone()).compare(&b(0x38d1b717)?) != Ordering::Greater {
                continue;
            }
            let result = delta
                .mul(&b(settings.strength)?, pc)?
                .mul(&strength, pc)?
                .add(&old, pc)?
                .store_f32()?;
            let area = field.sample_flags[index];
            if settings.protect && area != 0 && area != settings.area {
                effects.protected_diagnostic(
                    cell,
                    area,
                    settings.area,
                    f.height_bits[index],
                    target.store_f32()?,
                    result,
                )?;
                return Ok(false);
            }
            pending_set(rows, keys, cell, result, 0)?;
        }
    }
    Ok(true)
}
fn slope(
    field: &InitializedHeightfield,
    settings: Settings,
    center: [u32; 2],
    falloff: u32,
    current: Pending,
    effects: &mut impl Effects,
    pc: Precision,
) -> Result<u32> {
    let point = xy(field, current.cell)?;
    let mut value = settings.slope;
    if current.depth > 0 {
        if let Some(v) = effects.terrain_slope(point)? {
            if !b(v)?.is_zero() {
                value = v
            }
        }
    }
    if b(falloff)?.compare(&Binary::from_u32(0)) == Ordering::Greater {
        let cell = b(field.field.cell_size_bits)?;
        let dx = store(b(center[0])?.sub(&Binary::from_i32(current.cell[0]).mul(&cell, pc)?, pc)?)?;
        let dy = b(center[1])?.sub(&Binary::from_i32(current.cell[1]).mul(&cell, pc)?, pc)?;
        let distance = dx
            .mul(&dx, pc)?
            .add(&dy.mul(&dy, pc)?, pc)?
            .sqrt(pc)?
            .div(&b(falloff)?, pc)?;
        let distance = if distance.compare(&Binary::from_u32(0)) == Ordering::Less {
            Binary::from_u32(0)
        } else if distance.compare(&Binary::from_u32(1)) == Ordering::Greater {
            Binary::from_u32(1)
        } else {
            distance
        };
        return Ok(distance.mul(&b(value)?, pc)?.store_f32()?);
    }
    Ok(value)
}
const NEIGHBORS: [[i32; 2]; 8] = [
    [-1, -1],
    [-1, 0],
    [-1, 1],
    [0, 1],
    [1, 1],
    [1, 0],
    [1, -1],
    [0, -1],
];
fn finish(
    field: &mut InitializedHeightfield,
    state: &mut State,
    rows: &mut Vec<Pending>,
    keys: &mut BTreeMap<[i32; 2], usize>,
    settings: Settings,
    center: [u32; 2],
    falloff: u32,
    effects: &mut impl Effects,
    pc: Precision,
) -> Result<bool> {
    let diagonal = b(field.field.cell_size_bits)?
        .mul(&b(0x3fb504f3)?, pc)?
        .store_f32()?;
    let mut cursor = 0;
    while cursor < rows.len() {
        let current = rows[cursor];
        cursor += 1;
        let limit = slope(field, settings, center, falloff, current, effects, pc)?;
        for delta in NEIGHBORS {
            let next = [
                current.cell[0].wrapping_add(delta[0]),
                current.cell[1].wrapping_add(delta[1]),
            ];
            if next[0] < 0
                || next[1] < 0
                || next[0] >= field.field.width
                || next[1] >= field.field.height
            {
                continue;
            }
            let index = at(field, next)?;
            let old = if let Some(&index) = keys.get(&next) {
                if rows[index].depth < current.depth {
                    continue;
                }
                rows[index].height
            } else {
                field.field.height_bits[index]
            };
            let difference = store(b(old)?.sub(&b(current.height)?, pc)?)?;
            let distance = b(if delta[0] == 0 || delta[1] == 0 {
                field.field.cell_size_bits
            } else {
                diagonal
            })?;
            let magnitude = abs(store(difference.div(&distance, pc)?)?);
            if b(limit)?.add(&b(0x38d1b717)?, pc)?.compare(&magnitude) == Ordering::Less {
                let area = field.sample_flags[index];
                if settings.protect && area != 0 && area != settings.area {
                    return Ok(false);
                }
                let direction = if difference.is_negative_nonzero() {
                    Binary::from_i32(-1)
                } else {
                    Binary::from_u32(1)
                };
                let value = direction
                    .mul(&distance, pc)?
                    .mul(&b(limit)?, pc)?
                    .add(&b(current.height)?, pc)?
                    .store_f32()?;
                pending_set(
                    rows,
                    keys,
                    next,
                    value,
                    current.depth.checked_add(1).ok_or(Error::Invalid)?,
                )?;
            }
        }
    }
    for row in rows {
        let index = at(field, row.cell)?;
        let height = b(row.height)?;
        let height = if height.compare(&b(field.minimum_bits)?) == Ordering::Less {
            field.minimum_bits
        } else if height.compare(&b(field.maximum_bits)?) == Ordering::Greater {
            field.maximum_bits
        } else {
            row.height
        };
        field.field.height_bits[index] = height;
        let point = xy(field, row.cell)?;
        let cell = b(field.field.cell_size_bits)?;
        let bounds = [
            b(point[0])?.sub(&cell, pc)?.store_f32()?,
            b(point[1])?.sub(&cell, pc)?.store_f32()?,
            b(point[0])?.add(&cell, pc)?.store_f32()?,
            b(point[1])?.add(&cell, pc)?.store_f32()?,
        ];
        for axis in 0..2 {
            if b(bounds[axis])?.compare(&b(state.dirty_bounds[axis])?) == Ordering::Less {
                state.dirty_bounds[axis] = bounds[axis]
            }
            if b(bounds[axis + 2])?.compare(&b(state.dirty_bounds[axis + 2])?) == Ordering::Greater
            {
                state.dirty_bounds[axis + 2] = bounds[axis + 2]
            }
        }
    }
    Ok(true)
}
pub fn apply<E: Effects>(
    field: &mut InitializedHeightfield,
    state: &mut State,
    geometry: &mut impl Geometry<E>,
    start: [u32; 2],
    end: [u32; 2],
    settings: Settings,
    effects: &mut E,
    pc: Precision,
) -> Result<bool> {
    field.field.validate().map_err(|_| Error::Invalid)?;
    if field.sample_flags.len() != field.field.height_bits.len() {
        return Err(Error::Invalid);
    }
    state.dirty_bounds = [start[0], start[1], start[0], start[1]];
    let mut rows = Vec::new();
    let mut keys = BTreeMap::new();
    if !raster(
        field, state, geometry, settings, &mut rows, &mut keys, effects, pc,
    )? {
        return Ok(false);
    }
    let same = b(start[0])?.compare(&b(end[0])?) == Ordering::Equal
        && b(start[1])?.compare(&b(end[1])?) == Ordering::Equal;
    if !same {
        return Err(Error::Service(
            "551140 different-endpoint5504B0 swept connector owner required".into(),
        ));
    }
    let falloff = effects.falloff_distance()?;
    //5512D2 stores only theY product before final additions;X remains extended.
    let half = b(0x3f000000)?;
    let y = store(b(start[1])?.sub(&b(end[1])?, pc)?.mul(&half, pc)?)?
        .add(&b(start[1])?, pc)?
        .store_f32()?;
    let x = b(start[0])?
        .sub(&b(end[0])?, pc)?
        .mul(&half, pc)?
        .add(&b(start[0])?, pc)?
        .store_f32()?;
    if !finish(
        field,
        state,
        &mut rows,
        &mut keys,
        settings,
        [x, y],
        falloff,
        effects,
        pc,
    )? {
        return Ok(false);
    }
    let dx = b(state.dirty_bounds[2])?
        .sub(&b(state.dirty_bounds[0])?, pc)?
        .mul(&half, pc)?;
    let dy = b(state.dirty_bounds[3])?
        .sub(&b(state.dirty_bounds[1])?, pc)?
        .mul(&half, pc)?;
    //550453 uses extendedY for the center BEFORE55046D stores its half-span.
    let center = [
        dx.add(&b(state.dirty_bounds[0])?, pc)?.store_f32()?,
        dy.add(&b(state.dirty_bounds[1])?, pc)?.store_f32()?,
    ];
    let dy = store(dy)?;
    let max = if dx.compare(&dy) == Ordering::Greater {
        dx.clone()
    } else {
        dy
    };
    effects.notify(center, max.mul(&b(0x3fb504f3)?, pc)?.store_f32()?)?;
    Ok(true)
}
