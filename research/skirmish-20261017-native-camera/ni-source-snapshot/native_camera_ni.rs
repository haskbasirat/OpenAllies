//!6FE050's same-owner NiCamera update: world transform, directions, culling
//!planes, projection/view matrix, position and four direction corners.
//!This does not construct NiCamera, bind a scene parent or certify a view.
use super::{Result, State, b, div, f, mul, sub};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

///Positive parent/controller paths require their actual bound owners. There
///is no successful empty callback, synthetic parent or assumed-null input.
pub trait Services {
    fn parent_transform_45cab0(&mut self, _camera: &State, _pc: Precision) -> Result<[u32; 13]> {
        Err("NiCamera parent45CAB0 transform owner is unbound".into())
    }
    fn attached_update_virtual_38(&mut self, _camera: &mut State, _pc: Precision) -> Result<()> {
        Err("NiCamera attached+B0 virtual38 owner is unbound".into())
    }
}
pub struct RequiredChildren;
impl Services for RequiredChildren {}
fn add(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.add(c, p).map_err(|e| e.to_string())
}
fn rounded(value: Binary) -> Result<Binary> {
    b(f(value)?)
}
fn words3(camera: &State, offset: u32) -> Result<[u32; 3]> {
    Ok([
        camera.word(offset)?,
        camera.word(offset + 4)?,
        camera.word(offset + 8)?,
    ])
}
fn values(words: [u32; 3]) -> Result<[Binary; 3]> {
    Ok([b(words[0])?, b(words[1])?, b(words[2])?])
}

///8A2C70 copies the actual local13-word transform when parent24 is null.
///The attached callback occurs AFTER all world stores, before basis copies.
fn world_transform(camera: &mut State, world: &mut impl Services, p: Precision) -> Result<()> {
    if camera.word(0x24)? == 0 {
        for i in 0..13 {
            let word = camera.word(0x38 + 4 * i)?;
            camera.put(0x6c + 4 * i, word);
        }
    } else {
        let transform = world.parent_transform_45cab0(camera, p)?;
        for (i, word) in transform.into_iter().enumerate() {
            camera.put(0x6c + 4 * i as u32, word);
        }
    }
    if camera.word(0xb0)? != 0 {
        world.attached_update_virtual_38(camera, p)?;
    }
    Ok(())
}
fn dot_zyx(normal: &[u32; 3], point: &[u32; 3], p: Precision) -> Result<Binary> {
    let zy = add(
        &mul(&b(normal[2])?, &b(point[2])?, p)?,
        &mul(&b(normal[1])?, &b(point[1])?, p)?,
        p,
    )?;
    add(&zy, &mul(&b(normal[0])?, &b(point[0])?, p)?, p)
}
///869C90 copies the normal unchanged and stores its Z,Y,X dot with the point.
fn plane(
    camera: &mut State,
    offset: u32,
    normal: [u32; 3],
    point: [u32; 3],
    p: Precision,
) -> Result<()> {
    let distance = f(dot_zyx(&normal, &point, p)?)?;
    for (i, word) in normal.into_iter().enumerate() {
        camera.put(offset + 4 * i as u32, word);
    }
    camera.put(offset + 12, distance);
    Ok(())
}
///42D750: the square root/inverse stay extended; each component stores32.
///The native threshold is the pinned binary32 word at9FD38C, not epsilon.
fn normalize(v: [u32; 3], p: Precision) -> Result<[u32; 3]> {
    let squared = add(
        &add(
            &mul(&b(v[0])?, &b(v[0])?, p)?,
            &mul(&b(v[1])?, &b(v[1])?, p)?,
            p,
        )?,
        &mul(&b(v[2])?, &b(v[2])?, p)?,
        p,
    )?;
    let length = squared.sqrt(p).map_err(|e| e.to_string())?;
    if length.compare(&b(0x358637bd)?) != Ordering::Greater {
        return Ok([0; 3]);
    }
    let inverse = div(&b(0x3f800000)?, &length, p)?;
    Ok([
        f(mul(&inverse, &b(v[0])?, p)?)?,
        f(mul(&inverse, &b(v[1])?, p)?)?,
        f(mul(&inverse, &b(v[2])?, p)?)?,
    ])
}
fn cross(a: &[Binary; 3], c: &[Binary; 3], p: Precision) -> Result<[u32; 3]> {
    Ok([
        f(sub(&mul(&a[1], &c[2], p)?, &mul(&a[2], &c[1], p)?, p)?)?,
        f(sub(&mul(&a[2], &c[0], p)?, &mul(&a[0], &c[2], p)?, p)?)?,
        f(sub(&mul(&a[0], &c[1], p)?, &mul(&a[1], &c[0], p)?, p)?)?,
    ])
}
///Near/far and orthographic point construction stores the Z product before
///the addition, whereas X/Y products remain extended until the sum store.
fn translated_point(camera: &State, axis: u32, scalar: u32, p: Precision) -> Result<[u32; 3]> {
    let z_product = f(mul(&b(scalar)?, &b(camera.word(axis + 8)?)?, p)?)?;
    let x = f(add(
        &mul(&b(scalar)?, &b(camera.word(axis)?)?, p)?,
        &b(camera.word(0x90)?)?,
        p,
    )?)?;
    let y = f(add(
        &mul(&b(scalar)?, &b(camera.word(axis + 4)?)?, p)?,
        &b(camera.word(0x94)?)?,
        p,
    )?)?;
    let z = f(add(&b(z_product)?, &b(camera.word(0x98)?)?, p)?)?;
    Ok([x, y, z])
}
fn planes(camera: &mut State, p: Precision) -> Result<()> {
    let direction = words3(camera, 0x114)?;
    let near = camera.word(0x148)?;
    let point = translated_point(camera, 0x114, near, p)?;
    plane(camera, 0x19c, direction, point, p)?;
    let far = camera.word(0x14c)?;
    let point = translated_point(camera, 0x114, far, p)?;
    plane(camera, 0x1ac, direction.map(|v| v ^ 0x80000000), point, p)?;
    let ortho = camera.byte(0x150)? != 0;
    for (index, (axis, other, negative)) in [
        (0x12c, 0x120, false),
        (0x12c, 0x120, true),
        (0x120, 0x12c, false),
        (0x120, 0x12c, true),
    ]
    .into_iter()
    .enumerate()
    {
        let scalar = camera.word(0x138 + index as u32 * 4)?;
        let other =
            values(words3(camera, other)?.map(|v| if negative { v ^ 0x80000000 } else { v }))?;
        let (vector, point) = if ortho {
            (
                values(words3(camera, 0x114)?)?,
                translated_point(camera, axis, scalar, p)?,
            )
        } else {
            //873430/500/5E0/6B0: X vector sum stores32; Y stays
            //extended and Z product stores32 before adding directionZ.
            let x = rounded(add(
                &mul(&b(scalar)?, &b(camera.word(axis)?)?, p)?,
                &b(camera.word(0x114)?)?,
                p,
            )?)?;
            let y = add(
                &mul(&b(scalar)?, &b(camera.word(axis + 4)?)?, p)?,
                &b(camera.word(0x118)?)?,
                p,
            )?;
            let z = add(
                &rounded(mul(&b(scalar)?, &b(camera.word(axis + 8)?)?, p)?)?,
                &b(camera.word(0x11c)?)?,
                p,
            )?;
            ([x, y, z], words3(camera, 0x90)?)
        };
        let normal = normalize(cross(&vector, &other, p)?, p)?;
        plane(camera, 0x1bc + index as u32 * 16, normal, point, p)?;
    }
    Ok(())
}

///8737B0. These explicit extended/32-bit temporaries mirror its FST/FSTP
///boundaries. In particular forward translation and twice-X stay extended.
fn view_projection(camera: &mut State, p: Precision) -> Result<()> {
    let position = words3(camera, 0x90)?;
    let right = words3(camera, 0x12c)?;
    let up = words3(camera, 0x120)?;
    let forward = words3(camera, 0x114)?;
    let tx = f(dot_zyx(&right.map(|v| v ^ 0x80000000), &position, p)?)?;
    let ty = f(dot_zyx(&up.map(|v| v ^ 0x80000000), &position, p)?)?;
    let tz = dot_zyx(&forward.map(|v| v ^ 0x80000000), &position, p)?;
    let sum_x = rounded(add(&b(camera.word(0x138)?)?, &b(camera.word(0x13c)?)?, p)?)?;
    let sum_y = add(&b(camera.word(0x144)?)?, &b(camera.word(0x140)?)?, p)?;
    let inv_x = rounded(div(
        &b(0x3f800000)?,
        &sub(&b(camera.word(0x13c)?)?, &b(camera.word(0x138)?)?, p)?,
        p,
    )?)?;
    let inv_y = rounded(div(
        &b(0x3f800000)?,
        &sub(&b(camera.word(0x140)?)?, &b(camera.word(0x144)?)?, p)?,
        p,
    )?)?;
    let inv_z = rounded(div(
        &b(0x3f800000)?,
        &sub(&b(camera.word(0x14c)?)?, &b(camera.word(0x148)?)?, p)?,
        p,
    )?)?;
    let twice_x = add(&inv_x, &inv_x, p)?;
    camera.put(0x10c, f(twice_x.clone())?);
    let twice_y = rounded(add(&inv_y, &inv_y, p)?)?;
    camera.put(0x110, f(twice_y.clone())?);
    if camera.byte(0x150)? != 0 {
        camera.put(0xf8, 0);
        camera.put(0x100, 0);
        camera.put(0x104, 0x3f800000);
        camera.put(0xfc, 0);
        let depth = rounded(div(&b(0x3f800000)?, &inv_z, p)?)?;
        for (i, word) in right.into_iter().enumerate() {
            camera.put(0xc8 + 4 * i as u32, f(mul(&twice_x, &b(word)?, p)?)?);
        }
        camera.put(
            0xd4,
            f(add(
                &mul(&twice_x, &b(tx)?, p)?,
                &div(&sum_x, &inv_x, p)?,
                p,
            )?)?,
        );
        for (i, word) in up.into_iter().enumerate() {
            camera.put(0xd8 + 4 * i as u32, f(mul(&twice_y, &b(word)?, p)?)?);
        }
        camera.put(
            0xe4,
            f(add(
                &mul(&twice_y, &b(ty)?, p)?,
                &div(&sum_y, &inv_y, p)?,
                p,
            )?)?,
        );
        for (i, word) in forward.into_iter().enumerate() {
            camera.put(0xe8 + 4 * i as u32, f(mul(&depth, &b(word)?, p)?)?);
        }
        let near = mul(
            &div(&b(0xbf800000)?, &inv_z, p)?,
            &b(camera.word(0x148)?)?,
            p,
        )?;
        camera.put(0xf4, f(add(&near, &mul(&depth, &tz, p)?, p)?)?);
    } else {
        let off_x = mul(&inv_x, &sum_x, p)?.negated();
        camera.put(0xf8, forward[0]);
        let off_y = rounded(mul(&inv_y, &sum_y, p)?.negated())?;
        let depth = rounded(mul(&inv_z, &b(camera.word(0x14c)?)?, p)?)?;
        for i in 0..3 {
            let a = mul(&twice_x, &b(right[i])?, p)?;
            let c = mul(&off_x, &b(forward[i])?, p)?;
            camera.put(0xc8 + 4 * i as u32, f(add(&a, &c, p)?)?);
        }
        camera.put(
            0xd4,
            f(add(&mul(&off_x, &tz, p)?, &mul(&twice_x, &b(tx)?, p)?, p)?)?,
        );
        for i in 0..3 {
            let a = mul(&twice_y, &b(up[i])?, p)?;
            let c = mul(&off_y, &b(forward[i])?, p)?;
            camera.put(0xd8 + 4 * i as u32, f(add(&a, &c, p)?)?);
        }
        camera.put(
            0xe4,
            f(add(&mul(&off_y, &tz, p)?, &mul(&twice_y, &b(ty)?, p)?, p)?)?,
        );
        for (i, word) in forward.into_iter().enumerate() {
            camera.put(0xe8 + 4 * i as u32, f(mul(&depth, &b(word)?, p)?)?);
        }
        let near_far = mul(
            &mul(&inv_z, &b(camera.word(0x148)?)?, p)?,
            &b(camera.word(0x14c)?)?,
            p,
        )?
        .negated();
        camera.put(0xf4, f(add(&near_far, &mul(&depth, &tz, p)?, p)?)?);
        camera.put(0x104, f(tz)?);
        camera.put(0x100, forward[2]);
        camera.put(0xfc, forward[1]);
    }
    Ok(())
}
fn corners(camera: &mut State, p: Precision) -> Result<()> {
    for (index, (horizontal, vertical)) in [
        (0x138, 0x140),
        (0x13c, 0x140),
        (0x138, 0x144),
        (0x13c, 0x144),
    ]
    .into_iter()
    .enumerate()
    {
        let h = camera.word(horizontal)?;
        let v = camera.word(vertical)?;
        let hx = mul(&b(h)?, &b(camera.word(0x12c)?)?, p)?;
        let hy = f(mul(&b(h)?, &b(camera.word(0x130)?)?, p)?)?;
        let hz = f(mul(&b(h)?, &b(camera.word(0x134)?)?, p)?)?;
        let vx = mul(&b(v)?, &b(camera.word(0x120)?)?, p)?;
        let vy = f(mul(&b(v)?, &b(camera.word(0x124)?)?, p)?)?;
        let vz = f(mul(&b(v)?, &b(camera.word(0x128)?)?, p)?)?;
        let x = add(&vx, &b(camera.word(0x114)?)?, p)?;
        let y = f(add(&b(vy)?, &b(camera.word(0x118)?)?, p)?)?;
        let z = f(add(&b(vz)?, &b(camera.word(0x11c)?)?, p)?)?;
        camera.put(0x2d4 + index as u32 * 12, f(add(&x, &hx, p)?)?);
        camera.put(0x2d8 + index as u32 * 12, f(add(&b(y)?, &b(hy)?, p)?)?);
        camera.put(0x2dc + index as u32 * 12, f(add(&b(z)?, &b(hz)?, p)?)?);
    }
    Ok(())
}
pub fn update_6fe050(camera: &mut State, world: &mut impl Services, p: Precision) -> Result<()> {
    world_transform(camera, world, p)?;
    for (to, from) in [
        (0x114, 0x6c),
        (0x118, 0x78),
        (0x11c, 0x84),
        (0x120, 0x70),
        (0x124, 0x7c),
        (0x128, 0x88),
        (0x12c, 0x74),
        (0x130, 0x80),
        (0x134, 0x8c),
    ] {
        let word = camera.word(from)?;
        camera.put(to, word);
    }
    planes(camera, p)?;
    view_projection(camera, p)?;
    for (to, from) in [(0x28, 0x90), (0x2c, 0x94), (0x30, 0x98)] {
        let word = camera.word(from)?;
        camera.put(to, word);
    }
    corners(camera, p)
}

#[cfg(test)]
#[path = "native_camera_ni_tests.rs"]
mod tests;
