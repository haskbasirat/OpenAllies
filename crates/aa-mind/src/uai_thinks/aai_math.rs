//! Original aircraft frame helpers (cdecl, pure x87 arithmetic at the
//! control-word precision; "stored" values are rounded to binary32):
//! - `517EE0(out, a, b, c, d)`: `out.x = c*b + d*a`, `out.y = (-a*c)s +
//!   (d*b)s` (the two products stored first).
//! - `517DF0(out, P, D, Q)`: `P + Q.x*(D.y, -D.x, 0) + Q.y*(D.x, D.y, 0) +
//!   Q.z*U` with `U = [B281CC..B281D4]` (0, 0, 1 in the image) and the
//!   `[9F32DC]` (0.0) products, in the original operation order.
use crate::{
    Result,
    x87::{n, num, store},
};
use aa_numeric::Precision;

/// `[B281CC]`, `[B281D0]`, `[B281D4]`: binary32 (0, 0, 1).
pub const UP: [u32; 3] = [0, 0, 0x3f80_0000];
/// `[9F32DC]`: binary32 0.0.
pub const ZERO: u32 = 0;

/// `517EE0(out, a, b, c, d)`.
pub fn rotate(a: u32, b: u32, c: u32, d: u32, p: Precision) -> Result<[u32; 2]> {
    let neg_a = n(a)?.negated();
    let da = num(n(d)?.mul(&n(a)?, p))?;
    let t1 = store(num(n(d)?.mul(&n(b)?, p))?)?;
    let cb = num(n(c)?.mul(&n(b)?, p))?;
    let t2 = store(num(neg_a.mul(&n(c)?, p))?)?;
    let x = store(num(da.add(&cb, p))?)?;
    let y = store(num(n(t2)?.add(&n(t1)?, p))?)?;
    Ok([x, y])
}

/// `517DF0(out, P, D, Q)`.
pub fn transform(point: [u32; 3], d: [u32; 2], q: [u32; 3], p: Precision) -> Result<[u32; 3]> {
    let [px, py, pz] = point;
    let [dx, dy] = d;
    let [qx, qy, qz] = q;
    let neg_dx = n(dx)?.negated();
    let u0qz = num(n(UP[0])?.mul(&n(qz)?, p))?;
    let s10 = store(num(n(UP[1])?.mul(&n(qz)?, p))?)?;
    let s14 = store(num(n(UP[2])?.mul(&n(qz)?, p))?)?;
    let qydx = num(n(qy)?.mul(&n(dx)?, p))?;
    let qydy = num(n(qy)?.mul(&n(dy)?, p))?;
    let s20 = store(num(n(qy)?.mul(&n(ZERO)?, p))?)?;
    let qxdy = num(n(qx)?.mul(&n(dy)?, p))?;
    let ndxqx = num(neg_dx.mul(&n(qx)?, p))?;
    let s48 = store(num(n(qx)?.mul(&n(ZERO)?, p))?)?;
    let s0 = store(num(qxdy.add(&n(px)?, p))?)?;
    let ty = num(ndxqx.add(&n(py)?, p))?;
    let s8 = store(num(n(pz)?.add(&n(s48)?, p))?)?;
    let s40 = store(num(n(s0)?.add(&qydx, p))?)?;
    let s44 = store(num(ty.add(&qydy, p))?)?;
    let s48b = store(num(n(s8)?.add(&n(s20)?, p))?)?;
    let x = store(num(n(s40)?.add(&u0qz, p))?)?;
    let y = store(num(n(s44)?.add(&n(s10)?, p))?)?;
    let z = store(num(n(s48b)?.add(&n(s14)?, p))?)?;
    Ok([x, y, z])
}
