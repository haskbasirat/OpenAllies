//! Original AIRCRAFT path leg planning (`AirCraft.cpp`): the leg builder
//! `0x55F230` with its helpers, and the multi-leg planner `0x5601E0`.
//!
//! - `0x55D920` [`altitude_band`]: the target height kept inside the band
//!   0.25..1.25 path altitudes above the ground;
//! - `0x55DEF0` [`flat_length`]: adaptive horizontal (XY) length of a
//!   segment; `0x55E0F0` [`length_parameter`]: the parameter one piece
//!   length further along it (bisection);
//! - `0x55E6E0` [`new_segment`] (segment constructor `0x55DC00` + init);
//! - `0x55F230` [`build_leg`]: one cubic leg from a pose to a target point,
//!   subdivided into pieces of about 20 units when long, appended to the
//!   path (`0x55E6A0`);
//! - `0x55FE60` [`leg_target`]: a turning leg one radius to a side;
//! - `0x55E1E0` [`circle_side`]: the turning circle that contains a point;
//! - `0x5601E0` [`append_legs`]: up to 99 legs steering to a target point and
//!   direction (100 attempts clear the path and fail).
//!
//! The ground height `0x4F0E50(&xy, 0x123)` is the one boundary
//! ([`TerrainWorld`]). All arithmetic is `aa_numeric::Binary` at the given x87
//! precision with the native binary32 store points; nonfinite operands stop
//! with an error.
use super::*;

const TEN_THOUSANDTH: u32 = 0x38d1_b717; // 0xA3AF38, 0xA3AA10
const BEND_SLOPE: u32 = 0x3de1_47ac; // 0xA3AFC0 (0.11)
const BEND_BASE: u32 = 0x3ea8_f5c3; // 0xA3AFBC (0.33)
const SHORT_HANDLE: u32 = 0x3ef3_cf3e; // 0xA3AFB8 (0.476)
const SHRINK: u32 = 0x3f4c_cccd; // 0xA16F30 (0.8)
const PIECE_LIMIT: u32 = 0x41a0_cccd; // 0xA3AFB4 (20.1)
const PIECE_SCALE: u32 = 0x3d4c_cccd; // 0xA2A278 (0.05)
const BAND_LOW: u32 = 0xbe80_0000; // 0xA34D3C (-0.25)
const BAND_HIGH: u32 = 0x3f40_0000; // 0xA046F0 (0.75)
const BAND_TOP: u32 = 0x3fa0_0000; // 0xA3AF3C (1.25)
const BAND_FLOOR: u32 = 0x3e80_0000; // 0xA033BC (0.25)
const WIDE: u32 = 0x3fc0_0000; // 0xA01458 (1.5)
/// Native loops without a bound end here with [`AircraftError::UnboundedLoop`].
const LOOP_LIMIT: u32 = 1_000_000;

/// The terrain height service of the path planners.
pub trait TerrainWorld {
    /// `0x4F0E50(&xy, flags)` on GWorld: the extended result.
    fn ground_height(&mut self, xy: [f32; 2], flags: u32) -> AircraftResult<Binary>;
}

fn lt(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Less
}
fn gt(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Greater
}
fn abs(value: Binary) -> Binary {
    if value.is_negative_nonzero() { value.negated() } else { value }
}
/// `(a.y*y_factor + a.x*x_factor) + a.z*0`, the native operand order of the
/// leg builder's side tests (with the factors `(-dir.x, dir.y)`: the side of
/// `a` relative to the line along `dir`).
fn side_of(a: [Binary; 3], y_factor: f32, x_factor: f32, pc: Precision) -> AircraftResult<Binary> {
    Ok(a[1].mul(&b(y_factor)?, pc)?.add(&a[0].mul(&b(x_factor)?, pc)?, pc)?.add(&a[2].mul(&k(ZERO)?, pc)?, pc)?)
}

/// `0x55D920` (thiscall path, args &target, &from): with q = (target z - from
/// z) / altitude, a target more than a quarter altitude under the start
/// climbs to 1.25 altitudes above its ground (target z - altitude), one more
/// than 0.75 over it descends to 0.25 altitudes above it, and otherwise the
/// start height is kept. A zero altitude gives the native infinities (a
/// nonzero numerator) or the unordered 0/0, which keeps the start height.
pub fn altitude_band(path: &AircraftPath, target_z: f32, from_z: f32, pc: Precision) -> AircraftResult<f32> {
    let altitude = b(path.altitude)?;
    let numerator = b(target_z)?.sub(&b(from_z)?, pc)?;
    let ground = st(&b(target_z)?.sub(&altitude, pc)?)?;
    // None: unordered.
    let quotient: Option<Ordering> = if altitude.is_zero() {
        if numerator.is_zero() {
            None
        } else if numerator.is_negative_nonzero() != sign_bit(&altitude)? {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    } else {
        let q = numerator.div(&altitude, pc)?;
        if lt(&q, &k(BAND_LOW)?) {
            Some(Ordering::Less)
        } else if gt(&q, &k(BAND_HIGH)?) {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
    };
    Ok(match quotient {
        Some(Ordering::Less) => st(&altitude.mul(&k(BAND_TOP)?, pc)?.add(&b(ground)?, pc)?)?,
        Some(Ordering::Greater) => st(&altitude.mul(&k(BAND_FLOOR)?, pc)?.add(&b(ground)?, pc)?)?,
        _ => from_z,
    })
}

/// `0x55DEF0` (thiscall segment, args t0, t1): adaptive XY length. Samples at
/// t0, t0+h, t0+2h and t1 (h = (t1-t0)/3, stored); when the stored polyline
/// exceeds the (extended) chord by more than +0x54 it recurses on both halves
/// and returns `L(mid, t1) + binary32(L(t0, mid))`, else `(chord +
/// polyline) / 2`. A midpoint equal to an endpoint recurses forever natively.
pub fn flat_length(basis: &[f32; 16], segment: &PathSegment, t0: f32, t1: f32, pc: Precision) -> AircraftResult<Binary> {
    let h = st(&b(t1)?.sub(&b(t0)?, pc)?.mul(&k(THIRD)?, pc)?)?;
    let a = evaluate_curve(basis, &segment.control, t0, pc)?;
    let p1 = evaluate_curve(basis, &segment.control, st(&b(h)?.add(&b(t0)?, pc)?)?, pc)?;
    let p2 = evaluate_curve(basis, &segment.control, st(&b(h)?.add(&b(h)?, pc)?.add(&b(t0)?, pc)?)?, pc)?;
    let d = evaluate_curve(basis, &segment.control, t1, pc)?;
    let planar = |dx: &Binary, dy: &Binary| -> AircraftResult<Binary> {
        Ok(dy.mul(dy, pc)?.add(&dx.mul(dx, pc)?, pc)?.sqrt(pc)?)
    };
    let chord = planar(&b(d[0])?.sub(&b(a[0])?, pc)?, &b(d[1])?.sub(&b(a[1])?, pc)?)?;
    let ab = [st(&b(p1[0])?.sub(&b(a[0])?, pc)?)?, st(&b(p1[1])?.sub(&b(a[1])?, pc)?)?];
    let cb = [b(p2[0])?.sub(&b(p1[0])?, pc)?, b(p2[1])?.sub(&b(p1[1])?, pc)?];
    let dc = [b(d[0])?.sub(&b(p2[0])?, pc)?, b(d[1])?.sub(&b(p2[1])?, pc)?];
    let l3 = planar(&dc[0], &dc[1])?;
    let l2 = cb[1].mul(&cb[1], pc)?.add(&cb[0].mul(&cb[0], pc)?, pc)?.sqrt(pc)?;
    let l1 = planar(&b(ab[0])?, &b(ab[1])?)?;
    let polyline = st(&l3.add(&l2, pc)?.add(&l1, pc)?)?;
    if gt(&b(polyline)?.sub(&chord, pc)?, &b(segment.tolerance)?) {
        let mid = st(&b(t0)?.add(&b(t1)?, pc)?.mul(&k(HALF)?, pc)?)?;
        let m = b(mid)?;
        if m.compare(&b(t0)?) == Ordering::Equal || m.compare(&b(t1)?) == Ordering::Equal {
            return Err(AircraftError::UnboundedRecursion);
        }
        let first = st(&flat_length(basis, segment, t0, mid, pc)?)?;
        return Ok(flat_length(basis, segment, mid, t1, pc)?.add(&b(first)?, pc)?);
    }
    Ok(chord.add(&b(polyline)?, pc)?.mul(&k(HALF)?, pc)?)
}

/// `0x55E0F0` (thiscall segment, args t0, piece): the parameter whose XY
/// length from t0 is `piece` within +0x54, by bisection from 1.0 with steps
/// (1 - t0)/2, /4, ...; 1.0 when the rest of the segment is shorter than
/// `piece` + +0x54. The result is binary32 (loaded from its stack slot).
pub fn length_parameter(
    basis: &[f32; 16],
    segment: &PathSegment,
    t0: f32,
    piece: f32,
    pc: Precision,
) -> AircraftResult<f32> {
    let tolerance = b(segment.tolerance)?;
    let piece_b = b(piece)?;
    let mut length = flat_length(basis, segment, t0, 1.0, pc)?;
    if lt(&length.sub(&piece_b, pc)?, &tolerance) {
        return Ok(1.0);
    }
    let mut t = 1.0f32;
    let mut step = st(&k(ONE)?.sub(&b(t0)?, pc)?)?;
    for _ in 0..LOOP_LIMIT {
        step = st(&b(step)?.mul(&k(HALF)?, pc)?)?;
        t = if gt(&length, &piece_b) { st(&b(t)?.sub(&b(step)?, pc)?)? } else { st(&b(t)?.add(&b(step)?, pc)?)? };
        length = flat_length(basis, segment, t0, t, pc)?;
        if !gt(&abs(length.sub(&piece_b, pc)?), &tolerance) {
            return Ok(t);
        }
    }
    Err(AircraftError::UnboundedLoop("0x55E0F0 length bisection"))
}

/// `0x55E6E0` on a `0x55DC00` segment: the control points, tolerance and end
/// direction and speeds, then +0x38 = binary32 arc length (`0x55DCA0(0, 1)`)
/// and +0x3C = binary32 XY length (`0x55DEF0(0, 1)`).
#[allow(clippy::too_many_arguments)]
pub fn new_segment(
    basis: &[f32; 16],
    control: [[f32; 3]; 4],
    tolerance: f32,
    end_direction: [f32; 3],
    start_speed: f32,
    end_speed: f32,
    pc: Precision,
) -> AircraftResult<PathSegment> {
    let mut segment = PathSegment {
        control,
        length: 0.0,
        field_3c: 0.0,
        end_direction,
        end_speed,
        start_speed,
        tolerance,
    };
    segment.length = st(&arc_length(basis, &segment, 0.0, 1.0, pc)?)?;
    segment.field_3c = st(&flat_length(basis, &segment, 0.0, 1.0, pc)?)?;
    Ok(segment)
}

fn diff3(p: [f32; 3], q: [f32; 3], pc: Precision) -> AircraftResult<[Binary; 3]> {
    sub3(p, q, pc)
}
fn stored3(v: &[Binary; 3]) -> AircraftResult<[f32; 3]> {
    Ok([st(&v[0])?, st(&v[1])?, st(&v[2])?])
}
/// `((v.z*w.z + v.y*w.y) + v.x*w.x)`, the native dot order.
fn dot_zyx(v: &[Binary; 3], w: [f32; 3], pc: Precision) -> AircraftResult<Binary> {
    Ok(v[2].mul(&b(w[2])?, pc)?.add(&v[1].mul(&b(w[1])?, pc)?, pc)?.add(&v[0].mul(&b(w[0])?, pc)?, pc)?)
}
/// `base + scale * dir` stored as the leg builder does for its start handle:
/// x from the extended product, y from `scale`, z from the stored product.
fn ahead(base: [f32; 3], x_scale: &Binary, scale: f32, dir: [f32; 3], pc: Precision) -> AircraftResult<[f32; 3]> {
    Ok([
        st(&x_scale.mul(&b(dir[0])?, pc)?.add(&b(base[0])?, pc)?)?,
        st(&b(scale)?.mul(&b(dir[1])?, pc)?.add(&b(base[1])?, pc)?)?,
        st(&b(st(&b(scale)?.mul(&b(dir[2])?, pc)?)?)?.add(&b(base[2])?, pc)?)?,
    ])
}
/// `target - scale * dir` with x, y from the extended products and z from
/// the stored one.
fn behind(target: [f32; 3], scale: &Binary, dir: [f32; 3], pc: Precision) -> AircraftResult<[f32; 3]> {
    Ok([
        st(&b(target[0])?.sub(&scale.mul(&b(dir[0])?, pc)?, pc)?)?,
        st(&b(target[1])?.sub(&scale.mul(&b(dir[1])?, pc)?, pc)?)?,
        st(&b(target[2])?.sub(&b(st(&scale.mul(&b(dir[2])?, pc)?)?)?, pc)?)?,
    ])
}

/// `0x55F230` (thiscall path, args &from, &out, target x, y, z, &direction
/// or null, flag byte, extra byte): one cubic leg.
///
/// The target height becomes the ground under it plus the path altitude,
/// kept in the [`altitude_band`] unless `flag`. The start handle P1 runs
/// along the start direction and the end handle P2 back from the target
/// along the end direction (the given one, or toward the target from the
/// midpoint of target and the doubled start handle); both handles shrink by
/// 0.8 until they lie on the side of the start/end lines the target/start
/// does. The leg ends at the target with the end direction and the path's
/// leg speed. A leg longer than 20.1 in XY (and neither `flag` nor `extra`)
/// is not appended itself: it is flown as pieces of about 20 (`extra`
/// legs through points along it, the last one to the target). Otherwise the
/// segment is appended and its XY length added to the path total. Returns
/// the out pose.
#[allow(clippy::too_many_arguments)]
pub fn build_leg<W: TerrainWorld + ?Sized>(
    path: &mut AircraftPath,
    from: &Pose,
    target: [f32; 3],
    direction: Option<[f32; 3]>,
    flag: u8,
    extra: u8,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<Pose> {
    let fp = from.position;
    let fd = from.direction;
    let ground = world.ground_height([target[0], target[1]], GROUND_HEIGHT_FLAGS)?;
    let mut t = target;
    t[2] = st(&ground.add(&b(path.altitude)?, pc)?)?;
    if flag == 0 {
        t[2] = altitude_band(path, t[2], fp[2], pc)?;
    }
    let d = stored3(&diff3(t, fp, pc)?)?;
    let dist = st(&b(d[0])?
        .mul(&b(d[0])?, pc)?
        .add(&b(d[2])?.mul(&b(d[2])?, pc)?, pc)?
        .add(&b(d[1])?.mul(&b(d[1])?, pc)?, pc)?
        .sqrt(pc)?)?;
    let speed = if gt(&b(from.speed)?, &k(TEN_THOUSANDTH)?) { b(from.speed)? } else { k(TEN_THOUSANDTH)? };
    let inverse_planning = k(ONE)?.div(&b(path.planning_speed)?, pc)?;
    let start_scale = st(&inverse_planning.mul(&speed, pc)?)?;
    let leg_scale = inverse_planning.mul(&b(path.leg_speed)?, pc)?;
    let w = match direction {
        Some(dir) => dir,
        None => {
            let inverse = k(ONE)?.div(&b(dist)?, pc)?;
            [
                st(&b(d[0])?.mul(&inverse, pc)?)?,
                st(&b(d[1])?.mul(&inverse, pc)?)?,
                st(&b(d[2])?.mul(&inverse, pc)?)?,
            ]
        }
    };
    let alignment = dot_zyx(&[b(w[0])?, b(w[1])?, b(w[2])?], fd, pc)?;
    let bend = k(ONE)?.sub(&alignment, pc)?.mul(&k(BEND_SLOPE)?, pc)?.add(&k(BEND_BASE)?, pc)?;
    let mut start_handle = st(&b(start_scale)?.mul(&bend, pc)?.mul(&b(dist)?, pc)?)?;
    let mut end_handle = leg_scale.mul(&bend, pc)?.mul(&b(dist)?, pc)?;
    if direction.is_none() && gt(&b(start_handle)?.add(&b(start_handle)?, pc)?, &b(dist)?) {
        end_handle = b(dist)?.mul(&k(SHORT_HANDLE)?, pc)?;
        start_handle = st(&end_handle)?;
    }
    let inverse = k(ONE)?.div(&b(dist)?, pc)?;
    let toward = [st(&b(d[0])?.mul(&inverse, pc)?)?, st(&b(d[1])?.mul(&inverse, pc)?)?];
    let mut facing = b(d[2])?
        .mul(&inverse, pc)?
        .mul(&b(fd[2])?, pc)?
        .add(&b(toward[1])?.mul(&b(fd[1])?, pc)?, pc)?
        .add(&b(toward[0])?.mul(&b(fd[0])?, pc)?, pc)?;
    if lt(&facing, &k(HALF)?) {
        facing = k(HALF)?;
    }
    let start_ext = facing.mul(&b(start_handle)?, pc)?;
    start_handle = st(&start_ext)?;
    let mut p1 = ahead(fp, &start_ext, start_handle, fd, pc)?;
    let mut out = Pose { position: t, direction: [0.0; 3], speed: 0.0 };
    let mut p2;
    let end_length;
    match direction {
        Some(dir) => {
            end_length = st(&end_handle)?;
            p2 = behind(t, &end_handle, dir, pc)?;
            out.direction = dir;
        }
        None => {
            let ax = b(p1[0])?.sub(&b(fp[0])?, pc)?;
            let ay = b(p1[1])?.sub(&b(fp[1])?, pc)?;
            let az = st(&b(p1[2])?.sub(&b(fp[2])?, pc)?)?;
            let sx = st(&b(t[0])?.add(&b(p1[0])?, pc)?.add(&ax, pc)?)?;
            let sy = st(&b(t[1])?.add(&b(p1[1])?, pc)?)?;
            let sz = st(&b(t[2])?.add(&b(p1[2])?, pc)?)?;
            let half = k(HALF)?;
            p2 = [
                st(&b(sx)?.mul(&half, pc)?)?,
                st(&b(sy)?.add(&ay, pc)?.mul(&half, pc)?)?,
                st(&b(sz)?.add(&b(az)?, pc)?.mul(&half, pc)?)?,
            ];
            if lt(&b(fd[2])?, &k(ZERO)?) && gt(&b(p2[2])?, &b(p1[2])?) {
                p2[2] = p1[2];
            }
            out.direction = stored3(&diff3(t, p2, pc)?)?;
            let [ox, oy, oz] = [b(out.direction[0])?, b(out.direction[1])?, b(out.direction[2])?];
            end_length = st(&ox.mul(&ox, pc)?.add(&oy.mul(&oy, pc)?, pc)?.add(&oz.mul(&oz, pc)?, pc)?.sqrt(pc)?)?;
        }
    }
    norm(&mut out.direction, pc)?;
    let od = out.direction;
    let fd_b = [b(fd[0])?, b(fd[1])?, b(fd[2])?];
    let heading = b(od[2])?
        .mul(&fd_b[2], pc)?
        .add(&b(od[1])?.mul(&fd_b[1], pc)?, pc)?
        .add(&fd_b[0].mul(&b(od[0])?, pc)?, pc)?;
    // (q0, q1): the end and start handle lengths the side checks shrink.
    let (mut q0, mut q1) = (b(end_length)?, b(start_handle)?);
    if !lt(&heading, &k(SHRINK)?) {
        let start_past = dot_zyx(&diff3(p1, t, pc)?, od, pc)?;
        let end_before = !gt(&start_past, &k(ZERO)?) || !gt(&dot_zyx(&diff3(p2, fp, pc)?, fd, pc)?, &k(ZERO)?);
        if end_before {
            let third_ext = b(dist)?.mul(&k(THIRD)?, pc)?;
            let third = st(&third_ext)?;
            p1 = ahead(fp, &b(third)?, third, fd, pc)?;
            p2 = behind(t, &b(third)?, od, pc)?;
            q0 = b(third)?;
            q1 = third_ext;
        }
    }
    let shrink = k(SHRINK)?;
    let tenth = k(TENTH)?;
    // The end handle P2 must lie on the target's side of the start line.
    let (ny, nx) = (negate(fd[0])?, fd[1]);
    let side = side_of([b(d[0])?, b(d[1])?, b(d[2])?], ny, nx, pc)?;
    if gt(&abs(side.clone()), &tenth) {
        let wanted = !side.is_negative_nonzero();
        for count in 0.. {
            if count == LOOP_LIMIT {
                return Err(AircraftError::UnboundedLoop("0x55F230 end handle"));
            }
            let s = side_of(diff3(p2, fp, pc)?, ny, nx, pc)?;
            if !s.is_negative_nonzero() == wanted {
                break;
            }
            q0 = q0.mul(&shrink, pc)?;
            p2 = behind(t, &q0, od, pc)?;
        }
    }
    // The start handle P1 must lie on the start's side of the end line.
    let (my, mx) = (negate(od[0])?, od[1]);
    let side = side_of(diff3(fp, t, pc)?, my, mx, pc)?;
    if !lt(&abs(side.clone()), &tenth) {
        let wanted = !side.is_negative_nonzero();
        for count in 0.. {
            if count == LOOP_LIMIT {
                return Err(AircraftError::UnboundedLoop("0x55F230 start handle"));
            }
            let s = side_of(diff3(p1, t, pc)?, my, mx, pc)?;
            if !s.is_negative_nonzero() == wanted {
                break;
            }
            q1 = q1.mul(&shrink, pc)?;
            p1 = [
                st(&q1.mul(&b(fd[0])?, pc)?.add(&b(fp[0])?, pc)?)?,
                st(&q1.mul(&b(fd[1])?, pc)?.add(&b(fp[1])?, pc)?)?,
                st(&b(st(&q1.mul(&b(fd[2])?, pc)?)?)?.add(&b(fp[2])?, pc)?)?,
            ];
        }
    }
    out.speed = path.leg_speed;
    let segment = new_segment(&path.basis, [fp, p1, p2, t], f32::from_bits(TENTH), od, from.speed, path.leg_speed, pc)?;
    let pieces = flag == 0 && extra == 0 && gt(&b(segment.field_3c)?, &k(PIECE_LIMIT)?);
    if pieces {
        let count = b(segment.field_3c)?.mul(&k(PIECE_SCALE)?, pc)?.truncate_i32()?;
        let piece = st(&b(segment.field_3c)?.div(&Binary::from_i32(count), pc)?)?;
        let mut param = length_parameter(&path.basis, &segment, 0.0, piece, pc)?;
        if lt(&b(param)?, &k(ONE)?) {
            let last_direction = direction.unwrap_or(od);
            let mut current = *from;
            loop {
                let (point, heading) = if lt(&b(param)?, &k(ONE)?) {
                    let point = evaluate_curve(&path.basis, &segment.control, param, pc)?;
                    let before = b(param)?.sub(&k(PIECE_SCALE)?, pc)?;
                    let before = if gt(&before, &k(ZERO)?) { st(&before)? } else { 0.0 };
                    let a = evaluate_curve(&path.basis, &segment.control, before, pc)?;
                    let after = b(param)?.add(&k(PIECE_SCALE)?, pc)?;
                    let after = if lt(&after, &k(ONE)?) { st(&after)? } else { 1.0 };
                    let c = evaluate_curve(&path.basis, &segment.control, after, pc)?;
                    let mut tangent = stored3(&diff3(c, a, pc)?)?;
                    norm(&mut tangent, pc)?;
                    (point, tangent)
                } else {
                    ([target[0], target[1], t[2]], last_direction)
                };
                current = build_leg(path, &current, point, Some(heading), 0, 1, pc, world)?;
                let next = if b(param)?.compare(&k(ONE)?) == Ordering::Equal {
                    b(param)?.add(&k(ONE)?, pc)?
                } else {
                    b(length_parameter(&path.basis, &segment, param, piece, pc)?)?
                };
                param = st(&next)?;
                if gt(&b(param)?, &k(ONE)?) {
                    return Ok(current);
                }
            }
        }
    }
    path.segments.push_back(segment);
    path.total_length = st(&b(segment.field_3c)?.add(&b(path.total_length)?, pc)?)?;
    Ok(out)
}

/// `0x55FE60` (thiscall path, args &from, &out, side x, y, z, radius, flag):
/// a [`build_leg`] toward `from + radius * (side + direction)` in XY at the
/// path altitude, ending along `side`.
pub fn leg_target<W: TerrainWorld + ?Sized>(
    path: &mut AircraftPath,
    from: &Pose,
    side: [f32; 3],
    radius: f32,
    flag: u8,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<Pose> {
    let radius = b(radius)?;
    let x = st(&b(side[0])?.add(&b(from.direction[0])?, pc)?.mul(&radius, pc)?)?;
    let target = [
        st(&b(x)?.add(&b(from.position[0])?, pc)?)?,
        st(&b(side[1])?.add(&b(from.direction[1])?, pc)?.mul(&radius, pc)?.add(&b(from.position[1])?, pc)?)?,
        path.altitude,
    ];
    build_leg(path, from, target, Some(side), flag, 0, pc, world)
}

/// `0x55E1E0` (thiscall path, args &pose, &side, target x, y, z): 1 when the
/// target is inside the turning circle around `pose + R * side`, else 2 when
/// inside the one around `pose - R * side`, else 0, with R = speed /
/// planning speed * turn radius (extended).
pub fn circle_side(path: &AircraftPath, pose: &Pose, side: [f32; 3], target: [f32; 3], pc: Precision) -> AircraftResult<u32> {
    let radius = b(pose.speed)?.div(&b(path.planning_speed)?, pc)?.mul(&b(path.turn_radius)?, pc)?;
    let rx = radius.mul(&b(side[0])?, pc)?;
    let ry = radius.mul(&b(side[1])?, pc)?;
    let ry_stored = st(&ry)?;
    let rz_stored = st(&radius.mul(&b(side[2])?, pc)?)?;
    let p = pose.position;
    let inside = |cx: f32, cy: &Binary, cz: &Binary| -> AircraftResult<bool> {
        let x = st(&b(target[0])?.sub(&b(cx)?, pc)?)?;
        let y = st(&b(target[1])?.sub(cy, pc)?)?;
        let z = b(target[2])?.sub(cz, pc)?;
        let distance =
            z.mul(&z, pc)?.add(&b(y)?.mul(&b(y)?, pc)?, pc)?.add(&b(x)?.mul(&b(x)?, pc)?, pc)?.sqrt(pc)?;
        Ok(lt(&distance, &radius))
    };
    let first = (st(&rx.add(&b(p[0])?, pc)?)?, ry.add(&b(p[1])?, pc)?, b(rz_stored)?.add(&b(p[2])?, pc)?);
    if inside(first.0, &first.1, &first.2)? {
        return Ok(1);
    }
    let second = (st(&b(p[0])?.sub(&rx, pc)?)?, b(p[1])?.sub(&b(ry_stored)?, pc)?, b(p[2])?.sub(&b(rz_stored)?, pc)?);
    Ok(if inside(second.0, &second.1, &second.2)? { 2 } else { 0 })
}

fn within(a: f32, b_: f32, pc: Precision) -> AircraftResult<bool> {
    Ok(lt(&abs(b(a)?.sub(&b(b_)?, pc)?), &k(TEN_THOUSANDTH)?))
}

/// `0x5601E0` (thiscall path, args &pose, target x, y, z, &direction or
/// null, flag byte, single byte): legs from `pose` until it reaches the
/// target (XY within 1e-4) with the direction (XY within 1e-4). Each attempt
/// turns away from a target inside a turning circle, turns toward one
/// behind or beside, swings out wide for a direction more than 60 degrees
/// off, or flies the direct [`build_leg`]. With `single` only one attempt is
/// made (and true returned); after 99 attempts the path is cleared
/// (`0x55E1B0`) and false returned. `flag` skips the steering.
#[allow(clippy::too_many_arguments)]
pub fn append_legs<W: TerrainWorld + ?Sized>(
    path: &mut AircraftPath,
    pose: &Pose,
    target: [f32; 3],
    direction: Option<[f32; 3]>,
    flag: u8,
    single: u8,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<bool> {
    let ground = world.ground_height([target[0], target[1]], GROUND_HEIGHT_FLAGS)?;
    let mut t = target;
    t[2] = st(&ground.add(&b(path.altitude)?, pc)?)?;
    if flag == 0 {
        t[2] = altitude_band(path, t[2], pose.position[2], pc)?;
    }
    let mut current = *pose;
    let mut attempts = 0u32;
    loop {
        if attempts == 1 && single != 0 {
            return Ok(true);
        }
        attempts += 1;
        if attempts >= 100 {
            clear_path(path);
            return Ok(false);
        }
        let radius = st(&b(path.turn_radius)?.div(&b(path.planning_speed)?, pc)?.mul(&b(current.speed)?, pc)?)?;
        let cd = current.direction;
        let mut direct = flag != 0;
        if !direct {
            let back_x = b(cd[0])?.negated();
            let length = b(cd[1])?.mul(&b(cd[1])?, pc)?.add(&back_x.mul(&back_x, pc)?, pc)?.sqrt(pc)?;
            let side = if gt(&length, &k(TEN_THOUSANDTH)?) {
                let inverse = k(ONE)?.div(&length, pc)?;
                [
                    st(&b(cd[1])?.mul(&inverse, pc)?)?,
                    st(&back_x.mul(&inverse, pc)?)?,
                    st(&inverse.mul(&k(ZERO)?, pc)?)?,
                ]
            } else {
                [0.0; 3]
            };
            let circle = circle_side(path, &current, side, t, pc)?;
            let e = stored3(&diff3(t, current.position, pc)?)?;
            let eb = [b(e[0])?, b(e[1])?, b(e[2])?];
            let toward_side = eb[0]
                .mul(&b(side[0])?, pc)?
                .add(&eb[2].mul(&b(side[2])?, pc)?, pc)?
                .add(&eb[1].mul(&b(side[1])?, pc)?, pc)?;
            let chosen = if gt(&toward_side, &k(ZERO)?) {
                side
            } else {
                [negate(side[0])?, negate(side[1])?, negate(side[2])?]
            };
            let ahead_distance = st(&eb[1]
                .mul(&b(cd[1])?, pc)?
                .add(&eb[0].mul(&b(cd[0])?, pc)?, pc)?
                .add(&b(cd[2])?.mul(&eb[2], pc)?, pc)?)?;
            if circle != 0 {
                let away = if single == 0 {
                    [negate(chosen[0])?, negate(chosen[1])?, negate(chosen[2])?]
                } else {
                    chosen
                };
                current = leg_target(path, &current, away, radius, 0, pc, world)?;
                continue;
            }
            let distance = st(&eb[0]
                .mul(&eb[0], pc)?
                .add(&eb[2].mul(&eb[2], pc)?, pc)?
                .add(&eb[1].mul(&eb[1], pc)?, pc)?
                .sqrt(pc)?)?;
            let back = [negate(cd[0])?, negate(cd[1])?, negate(cd[2])?];
            let ahead_b = b(ahead_distance)?;
            if let Some(dir) = direction {
                if !gt(&ahead_b, &k(TEN_THOUSANDTH)?) {
                    current = leg_target(path, &current, chosen, radius, 0, pc, world)?;
                    if single == 0 {
                        current = leg_target(path, &current, back, radius, 0, pc, world)?;
                    }
                    continue;
                }
                let alignment_ext = b(cd[1])?
                    .mul(&b(dir[1])?, pc)?
                    .add(&b(cd[0])?.mul(&b(dir[0])?, pc)?, pc)?
                    .add(&b(cd[2])?.mul(&b(dir[2])?, pc)?, pc)?;
                let alignment = st(&alignment_ext)?;
                let normal = (dir[1], negate(dir[0])?);
                if lt(&alignment_ext, &k(HALF)?) {
                    // Swing out: aim short of the target against the
                    // direction, half a swing to the target's side.
                    let swing_ext = k(ONE)?.sub(&b(alignment)?, pc)?.mul(&b(radius)?, pc)?;
                    let swing_ext = swing_ext.add(&swing_ext, pc)?;
                    let swing = st(&swing_ext)?;
                    let x = st(&b(t[0])?.sub(&swing_ext.mul(&b(dir[0])?, pc)?, pc)?)?;
                    let y = st(&b(t[1])?.sub(&b(swing)?.mul(&b(dir[1])?, pc)?, pc)?)?;
                    let z_ext = b(t[2])?.sub(&b(st(&b(swing)?.mul(&b(dir[2])?, pc)?)?)?, pc)?;
                    let cross = b(normal.0)?
                        .mul(&eb[0], pc)?
                        .add(&b(normal.1)?.mul(&eb[1], pc)?, pc)?
                        .add(&eb[2].mul(&k(ZERO)?, pc)?, pc)?;
                    let half = k(HALF)?;
                    let a = b(normal.0)?.mul(&b(swing)?, pc)?;
                    let c = b(normal.1)?.mul(&b(swing)?, pc)?;
                    let zero_part = st(&b(swing)?.mul(&k(ZERO)?, pc)?)?;
                    let a_half = st(&a.mul(&half, pc)?)?;
                    let c_half = c.mul(&half, pc)?;
                    let z_half = b(zero_part)?.mul(&half, pc)?;
                    let point = if lt(&cross, &k(ZERO)?) {
                        [
                            st(&b(a_half)?.add(&b(x)?, pc)?)?,
                            st(&c_half.add(&b(y)?, pc)?)?,
                            st(&z_half.add(&z_ext, pc)?)?,
                        ]
                    } else {
                        [
                            st(&b(x)?.sub(&b(a_half)?, pc)?)?,
                            st(&b(y)?.sub(&c_half, pc)?)?,
                            st(&z_ext.sub(&z_half, pc)?)?,
                        ]
                    };
                    current = build_leg(path, &current, point, None, 0, 0, pc, world)?;
                    continue;
                }
                // `0x5607A3`: the alignment is recomputed identically, so
                // its "negative" branch (a 1.5-radius turn) cannot be taken.
                if alignment_ext.is_negative_nonzero() {
                    return Err(AircraftError::World("0x5607C3 is unreachable".into()));
                }
            }
            direct = gt(&ahead_b, &k(TEN_THOUSANDTH)?)
                && !lt(&ahead_b.div(&b(distance)?, pc)?, &k(HALF)?);
            if !direct {
                current = leg_target(path, &current, chosen, radius, 0, pc, world)?;
                continue;
            }
        }
        let p = current.position;
        if !within(p[0], t[0], pc)? || !within(p[1], t[1], pc)? {
            current = build_leg(path, &current, t, direction, flag, 0, pc, world)?;
        }
        let p = current.position;
        let arrived = within(p[0], t[0], pc)?
            && within(p[1], t[1], pc)?
            && match direction {
                None => true,
                Some(dir) => {
                    within(current.direction[0], dir[0], pc)? && within(current.direction[1], dir[1], pc)?
                }
            };
        if arrived {
            return Ok(true);
        }
    }
}

/// `0x517DF0` (cdecl: &out, position, facing x/y, offset): `position +
/// rotate(offset)` in the frame of the horizontal facing (right = (fy, -fx)),
/// with the native partial stores and zero terms.
pub fn slot_point(position: [f32; 3], facing: [f32; 2], offset: [f32; 3], pc: Precision) -> AircraftResult<[f32; 3]> {
    let [ox, oy, oz] = [b(offset[0])?, b(offset[1])?, b(offset[2])?];
    let [fx, fy] = [b(facing[0])?, b(facing[1])?];
    let zero = k(ZERO)?;
    let z_by_zero = zero.mul(&oz, pc)?;
    let zero_z_stored = st(&zero.mul(&oz, pc)?)?;
    let z_stored = st(&k(ONE)?.mul(&oz, pc)?)?;
    let y_fx = oy.mul(&fx, pc)?;
    let y_fy = oy.mul(&fy, pc)?;
    let y_zero = st(&oy.mul(&zero, pc)?)?;
    let x_fy = ox.mul(&fy, pc)?;
    let x_back = fx.negated().mul(&ox, pc)?;
    let x_zero = st(&ox.mul(&zero, pc)?)?;
    let x0 = st(&x_fy.add(&b(position[0])?, pc)?)?;
    let y0 = b(position[1])?.add(&x_back, pc)?;
    let z0 = st(&b(position[2])?.add(&b(x_zero)?, pc)?)?;
    let x1 = st(&b(x0)?.add(&y_fx, pc)?)?;
    let y1 = st(&y0.add(&y_fy, pc)?)?;
    let z1 = st(&b(z0)?.add(&b(y_zero)?, pc)?)?;
    Ok([
        st(&b(x1)?.add(&z_by_zero, pc)?)?,
        st(&b(y1)?.add(&b(zero_z_stored)?, pc)?)?,
        st(&b(z1)?.add(&b(z_stored)?, pc)?)?,
    ])
}

/// `0x55E7E0` (thiscall path, arg speed): leg speed (+0x5C) and path speed
/// (+0x64), and both speeds of every segment.
pub fn set_path_speed(path: &mut AircraftPath, speed: f32) {
    path.leg_speed = speed;
    path.speed = speed;
    for segment in &mut path.segments {
        segment.end_speed = speed;
        segment.start_speed = speed;
    }
}

/// The end pose of a segment: +0x2C end point, +0x40 end direction, +0x4C
/// end speed.
fn segment_end(segment: &PathSegment) -> Pose {
    Pose { position: segment.control[3], direction: segment.end_direction, speed: segment.end_speed }
}

/// `0x560A70` (thiscall path, args &pose, leader path, offset x, y, z,
/// delta): plan this aircraft's path to its formation slot behind the
/// leader. The slot at the end of the leader's current segment is reached
/// with the leader's end direction ([`append_legs`]); when its length fits
/// the leader's remaining time between 1.5 and 0.5 planning speeds, the path
/// speed is set to arrive with the leader (true). Otherwise the path is
/// cleared and one [`append_legs`] attempt (`single`) flies to the slot of
/// the leader's next segment end (or, without one, a second ahead along the
/// pose), and an arrival not limited by the slow side flies at 1.5 planning
/// speeds (false). `delta` is not read.
pub fn follow_path<W: TerrainWorld + ?Sized>(
    path: &mut AircraftPath,
    pose: &Pose,
    leader: &AircraftPath,
    offset: [f32; 3],
    pc: Precision,
    world: &mut W,
) -> AircraftResult<bool> {
    let head = leader.segments.front().ok_or(AircraftError::NullDereference("0x560A70 reads the leader's null head segment"))?;
    let fastest = st(&b(path.planning_speed)?.mul(&k(WIDE)?, pc)?)?;
    let slowest = st(&b(path.planning_speed)?.mul(&k(HALF)?, pc)?)?;
    let (s0, s1) = (b(head.start_speed)?, b(head.end_speed)?);
    let average = st(&s1.sub(&s0, pc)?.mul(&b(leader.t)?, pc)?.add(&s0, pc)?.add(&s1, pc)?.mul(&k(HALF)?, pc)?)?;
    let remaining = st(&arc_length(&leader.basis, head, leader.t, 1.0, pc)?.div(&b(average)?, pc)?)?;
    let end = segment_end(head);
    let slot = slot_point(end.position, [end.direction[0], end.direction[1]], offset, pc)?;
    clear_path(path);
    let mut slow = false;
    if append_legs(path, pose, slot, Some(end.direction), 0, 0, pc, world)? {
        let at_fastest = b(path.total_length)?.div(&b(fastest)?, pc)?;
        let at_slowest = st(&b(path.total_length)?.div(&b(slowest)?, pc)?)?;
        if !lt(&b(remaining)?, &at_fastest) {
            if gt(&b(remaining)?, &b(at_slowest)?) {
                slow = true;
            } else {
                let speed = st(&b(path.total_length)?.div(&b(remaining)?, pc)?)?;
                set_path_speed(path, speed);
                return Ok(true);
            }
        }
    }
    clear_path(path);
    let mut side = [pose.direction[1], negate(pose.direction[0])?, 0.0];
    norm(&mut side, pc)?;
    let behind_slot = !lt(&dot_zyx(&[b(slot[0])?, b(slot[1])?, b(slot[2])?], pose.direction, pc)?, &k(ZERO)?);
    let (point, direction) = if behind_slot && circle_side(path, pose, side, slot, pc)? == 0 {
        (slot, end.direction)
    } else if let Some(next) = leader.segments.get(1) {
        let next = segment_end(next);
        (slot_point(next.position, [next.direction[0], next.direction[1]], offset, pc)?, next.direction)
    } else {
        slow = true;
        let speed = b(pose.speed)?;
        let x = speed.mul(&b(pose.direction[0])?, pc)?;
        let y = st(&speed.mul(&b(pose.direction[1])?, pc)?)?;
        let z = st(&speed.mul(&b(pose.direction[2])?, pc)?)?;
        let p = pose.position;
        (
            [st(&x.add(&b(p[0])?, pc)?)?, st(&b(y)?.add(&b(p[1])?, pc)?)?, st(&b(z)?.add(&b(p[2])?, pc)?)?],
            end.direction,
        )
    };
    append_legs(path, pose, point, Some(direction), 0, 1, pc, world)?;
    if !slow {
        set_path_speed(path, fastest);
    }
    Ok(false)
}

/// The services of the AIRCRAFT flight entry points `0x560DB0`, `0x560ED0`
/// and `0x560F10`.
pub trait FlightWorld: GroupWorld + TerrainWorld {
    /// The GMap (`[GWorld]+0x30`) cell size (+8) and extents (+0xC/+0x10)
    /// the `0x552640` clamp reads.
    fn map_bounds(&mut self) -> AircraftResult<crate::falling::MapBounds>;
}

/// The pose a new leg starts from: the end of the last segment, else the
/// actor's position and facing at the path speed (+0x8C).
fn flight_start(unit: &AircraftUnit) -> Pose {
    match unit.aircraft.path.segments.back() {
        Some(last) => segment_end(last),
        None => Pose {
            position: unit.motion.position,
            direction: unit.motion.facing,
            speed: unit.aircraft.path.speed,
        },
    }
}

/// `0x560DB0` (thiscall aircraft, args x, y, z, &direction or null): leave
/// the formation (`0x55EDC0`), stop circling (clearing the path), clamp the
/// point into the map (`0x552640`) and [`append_legs`] to it from the end of
/// the path (with +0x10 as the flag); on success the formation members'
/// paths are reset (`0x55EBE0`). Returns the planner's result.
pub fn fly_to<W: FlightWorld + ?Sized>(
    unit: &mut AircraftUnit,
    point: [f32; 3],
    direction: Option<[f32; 3]>,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<bool> {
    leave_formation(unit, world)?;
    if unit.aircraft.circling != 0 {
        clear_path(&mut unit.aircraft.path);
        unit.aircraft.circling = 0;
    }
    let mut xy = [point[0], point[1]];
    crate::falling::clamp_to_interior(world.map_bounds()?, &mut xy, pc)?;
    let from = flight_start(unit);
    let flag = unit.aircraft.on_ground;
    let reached =
        append_legs(&mut unit.aircraft.path, &from, [xy[0], xy[1], point[2]], direction, flag, 0, pc, world)?;
    if reached {
        reset_member_paths(unit, world)?;
    }
    Ok(reached)
}

/// `0x560ED0` (thiscall aircraft, args x, y, z, &direction or null): leave
/// the formation, clear the path, then [`fly_to`].
pub fn replace_path<W: FlightWorld + ?Sized>(
    unit: &mut AircraftUnit,
    point: [f32; 3],
    direction: Option<[f32; 3]>,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<bool> {
    leave_formation(unit, world)?;
    clear_path(&mut unit.aircraft.path);
    fly_to(unit, point, direction, pc, world)
}

/// `0x560F10` (thiscall aircraft, args offset x, y, z, goal x, y, z,
/// &direction or null): a formation leader's flight. The goal is clamped
/// into the map; an empty path starts at the formation's reference point
/// (`0x517DF0` of the negated offset from the actor's pose). Then
/// [`append_legs`] (result ignored), circling stops and the members' paths
/// are reset (`0x55EBE0`).
pub fn fly_formation<W: FlightWorld + ?Sized>(
    unit: &mut AircraftUnit,
    offset: [f32; 3],
    goal: [f32; 3],
    direction: Option<[f32; 3]>,
    pc: Precision,
    world: &mut W,
) -> AircraftResult<()> {
    let mut xy = [goal[0], goal[1]];
    crate::falling::clamp_to_interior(world.map_bounds()?, &mut xy, pc)?;
    let from = match unit.aircraft.path.segments.back() {
        Some(last) => segment_end(last),
        None => {
            let back = [negate(offset[0])?, negate(offset[1])?, negate(offset[2])?];
            let facing = unit.motion.facing;
            Pose {
                position: slot_point(unit.motion.position, [facing[0], facing[1]], back, pc)?,
                direction: facing,
                speed: unit.aircraft.path.speed,
            }
        }
    };
    append_legs(&mut unit.aircraft.path, &from, [xy[0], xy[1], goal[2]], direction, 0, 0, pc, world)?;
    unit.aircraft.circling = 0;
    reset_member_paths(unit, world)
}
