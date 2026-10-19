//! What is under the mouse: KKC_VWorld::FindObjectAt 0x43D240 over the
//! VGrid "On" list (VWorld+0x94 -> +0x100), the per-object hit test 0x454AC0
//! (selection-radius sphere or the model bound + inverse-transformed AABB of
//! 0x715C10), and the screen-point pick 0x43DBF0 / VWorld vt+0x64 0x43DC70.
//!
//! All arithmetic is the native x87 sequence at the caller's precision; the
//! only binary32 stores are where the native code stores.
use super::ui_world::{self, Leaves, State as Selection};
use super::x87::{self, b, st};
use crate::view_pointer::{self, CameraWords};
use aa_numeric::{Binary, Precision};

pub type Result<T> = std::result::Result<T, String>;

/// The model instance at VObject+0x78 (vtable 0xA889F8) as 0x454AC0 and
/// 0x715C10 read it. `None` from [`Scene::model_78`] stands for a NULL +0x78
/// or a false vt+0x10 (0x718BD0: node +0xC NULL).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    /// node+0x28..+0x30 world bound centre, node+0x34 radius.
    pub bound_center_28: [u32; 3],
    pub bound_radius_34: u32,
    /// node+0x6C world rotation (row-major 3x3), node+0x90 translation.
    pub rotate_6c: [u32; 9],
    pub translate_90: [u32; 3],
    /// (model+8)+0x4C: the resource AABB `{min xyz, max xyz}`; None when the
    /// resource is NULL (0x715C10 then dereferences it: an error).
    pub resource_box_4c: Option<[u32; 6]>,
}

/// The scene reads of the pick path.
pub trait Scene: Leaves {
    type Camera: CameraWords;
    /// VGrid+0x100 from the head through VThing+8.
    fn on_list_100(&mut self) -> Result<Vec<Self::View>>;
    /// VThing+0x18 proximity (2 = on).
    fn proximity_18(&mut self, view: Self::View) -> Result<i32>;
    fn model_78(&mut self, view: Self::View) -> Result<Option<Model>>;
    /// vt+0x104 (0x457DD0, the current aspect) then 0x469AC0:
    /// `aspect+0x60 > 0 ? aspect+0x60 : (aspect+0x28)+0x68`.
    fn selection_radius_469ac0(&mut self, view: Self::View) -> Result<u32>;
    /// 0x86E8D0(node, VWorld+0x8C, 1): the model node's world update that
    /// 0x715D40 runs before its model-space test; returns the updated data.
    fn update_model_86e8d0(&mut self, view: Self::View) -> Result<Model>;
    /// `**B40048`: the renderer's current camera.
    fn camera(&mut self) -> Result<&Self::Camera>;
    /// B396D0 / B396D4.
    fn viewport_b396d0(&mut self) -> Result<(i32, i32)>;
}

/// 0x875700(o, d, c, r): `!(r*r < |c-o|^2 - ((c-o).d)^2)`; no `t >= 0` test.
pub fn ray_sphere_875700(o: [u32; 3], d: [u32; 3], c: [u32; 3], r: u32, p: Precision) -> Result<bool> {
    let ex = x87::sub(&b(c[0])?, &b(o[0])?, p)?;
    let ey = x87::sub(&b(c[1])?, &b(o[1])?, p)?;
    let ez = x87::sub(&b(c[2])?, &b(o[2])?, p)?;
    let t = x87::add(
        &x87::add(&x87::mul(&ez, &b(d[2])?, p)?, &x87::mul(&ey, &b(d[1])?, p)?, p)?,
        &x87::mul(&ex, &b(d[0])?, p)?,
        p,
    )?;
    let l2 = x87::add(
        &x87::add(&x87::mul(&ex, &ex, p)?, &x87::mul(&ez, &ez, p)?, p)?,
        &x87::mul(&ey, &ey, p)?,
        p,
    )?;
    let distance = x87::sub(&l2, &x87::mul(&t, &t, p)?, p)?;
    let rr = x87::mul(&b(r)?, &b(r)?, p)?;
    Ok(!x87::lt(&rr, &distance))
}

/// 0x8707E0: adjugate (each entry stored binary32), determinant from the
/// stored entries; when `|det| > 1e-6` every entry is scaled by `1/det`.
/// Returns the output words and AL.
pub fn inverse_8707e0(m: [u32; 9], p: Precision) -> Result<([u32; 9], bool)> {
    let e = |a: usize, c: usize, d: usize, f: usize| -> Result<u32> {
        st(&x87::sub(&x87::mul(&b(m[a])?, &b(m[c])?, p)?, &x87::mul(&b(m[d])?, &b(m[f])?, p)?, p)?)
    };
    let mut o = [0u32; 9];
    o[0] = e(4, 8, 7, 5)?;
    o[1] = e(7, 2, 8, 1)?;
    o[2] = e(5, 1, 4, 2)?;
    o[3] = e(5, 6, 8, 3)?;
    o[4] = e(0, 8, 6, 2)?;
    o[5] = e(3, 2, 5, 0)?;
    o[6] = e(7, 3, 4, 6)?;
    o[7] = e(6, 1, 7, 0)?;
    o[8] = e(4, 0, 3, 1)?;
    let det = x87::add(
        &x87::add(&x87::mul(&b(o[6])?, &b(m[2])?, p)?, &x87::mul(&b(o[0])?, &b(m[0])?, p)?, p)?,
        &x87::mul(&b(o[3])?, &b(m[1])?, p)?,
        p,
    )?;
    let magnitude = if det.compare(&Binary::from_i32(0)) == std::cmp::Ordering::Less { det.clone().negated() } else { det.clone() };
    if x87::le(&magnitude, &b(0x3586_37bd)?) {
        return Ok((o, false));
    }
    let inverse = x87::div(&b(0x3f80_0000)?, &det, p)?;
    for word in o.iter_mut() {
        *word = st(&x87::mul(&inverse, &b(*word)?, p)?)?;
    }
    Ok((o, true))
}

/// 0x870720(M, v): `out0 = (M2*z + M1*y) + x*M0`, `out1 = (M5*z + M3*x) +
/// M4*y`, `out2 = (M8*z + M6*x) + M7*y`, each stored binary32.
pub fn transform_870720(m: [u32; 9], v: [u32; 3], p: Precision) -> Result<[u32; 3]> {
    let (x, y, z) = (b(v[0])?, b(v[1])?, b(v[2])?);
    let mm = |i: usize| b(m[i]);
    let a = x87::add(&x87::add(&x87::mul(&mm(8)?, &z, p)?, &x87::mul(&mm(6)?, &x, p)?, p)?, &x87::mul(&mm(7)?, &y, p)?, p)?;
    let c = x87::add(&x87::add(&x87::mul(&mm(5)?, &z, p)?, &x87::mul(&mm(3)?, &x, p)?, p)?, &x87::mul(&mm(4)?, &y, p)?, p)?;
    let d = x87::add(&x87::add(&x87::mul(&mm(2)?, &z, p)?, &x87::mul(&mm(1)?, &y, p)?, p)?, &x87::mul(&x, &mm(0)?, p)?, p)?;
    Ok([st(&d)?, st(&c)?, st(&a)?])
}

/// 0x715430(box, o, d): slab tests in the order Z min/max, Y min/max, X
/// min/max (X uses `1/dx` once); an axis with `d == 0` is skipped; the
/// first plane whose hit point lies inside the other two ranges hits.
pub fn ray_box_715430(bx: [u32; 6], o: [u32; 3], d: [u32; 3], p: Precision) -> Result<bool> {
    let zero = b(0)?;
    let inside = |v: u32, lo: u32, hi: u32| -> Result<bool> {
        let v = b(v)?;
        Ok(x87::le(&b(lo)?, &v) && x87::le(&v, &b(hi)?))
    };
    // Hit point of parameter `t` (register): x stays a register product.
    let point = |t: &Binary| -> Result<[u32; 3]> {
        let px = st(&x87::add(&x87::mul(t, &b(d[0])?, p)?, &b(o[0])?, p)?)?;
        let ty = st(&x87::mul(t, &b(d[1])?, p)?)?;
        let tz = st(&x87::mul(t, &b(d[2])?, p)?)?;
        let py = st(&x87::add(&b(ty)?, &b(o[1])?, p)?)?;
        let pz = st(&x87::add(&b(tz)?, &b(o[2])?, p)?)?;
        Ok([px, py, pz])
    };
    let nonzero = |w: u32| -> Result<bool> { Ok(b(w)?.compare(&zero) != std::cmp::Ordering::Equal) };
    if nonzero(d[2])? {
        for plane in [bx[2], bx[5]] {
            let t = x87::div(&x87::sub(&b(plane)?, &b(o[2])?, p)?, &b(d[2])?, p)?;
            let q = point(&t)?;
            if inside(q[0], bx[0], bx[3])? && inside(q[1], bx[1], bx[4])? {
                return Ok(true);
            }
        }
    }
    if nonzero(d[1])? {
        for plane in [bx[1], bx[4]] {
            let t = x87::div(&x87::sub(&b(plane)?, &b(o[1])?, p)?, &b(d[1])?, p)?;
            let q = point(&t)?;
            if inside(q[0], bx[0], bx[3])? && inside(q[2], bx[2], bx[5])? {
                return Ok(true);
            }
        }
    }
    if nonzero(d[0])? {
        let inverse = x87::div(&b(0x3f80_0000)?, &b(d[0])?, p)?;
        for plane in [bx[0], bx[3]] {
            let t = x87::mul(&x87::sub(&b(plane)?, &b(o[0])?, p)?, &inverse, p)?;
            let q = point(&t)?;
            if inside(q[1], bx[1], bx[4])? && inside(q[2], bx[2], bx[5])? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// 0x715C10 without the pick-context branch (reached only when
/// FindObjectAt's fifth argument is 1, which the game UIWorldState never
/// passes): bound sphere, then the AABB in model space.
pub fn model_pick_715c10(model: &Model, o: [u32; 3], d: [u32; 3], p: Precision) -> Result<bool> {
    if !ray_sphere_875700(o, d, model.bound_center_28, model.bound_radius_34, p)? {
        return Ok(false);
    }
    // AL of 0x8707E0 is ignored.
    let (inverse, _) = inverse_8707e0(model.rotate_6c, p)?;
    let relative = [
        st(&x87::sub(&b(o[0])?, &b(model.translate_90[0])?, p)?)?,
        st(&x87::sub(&b(o[1])?, &b(model.translate_90[1])?, p)?)?,
        st(&x87::sub(&b(o[2])?, &b(model.translate_90[2])?, p)?)?,
    ];
    let local_o = transform_870720(inverse, relative, p)?;
    let local_d = transform_870720(inverse, d, p)?;
    let bx = model.resource_box_4c.ok_or("0x715CEF reads the AABB of a NULL model resource (native NULL dereference)")?;
    ray_box_715430(bx, local_o, local_d, p)
}

/// 0x454AC0(origin, direction): the per-object hit test.
pub fn hit_454ac0<S: Scene>(scene: &mut S, view: S::View, o: [u32; 3], d: [u32; 3]) -> Result<bool> {
    let p = scene.precision();
    let Some(model) = scene.model_78(view)? else { return Ok(false) };
    if !x87::le(&b(model.bound_radius_34)?, &b(0)?) {
        return model_pick_715c10(&model, o, d, p);
    }
    let radius = scene.selection_radius_469ac0(view)?;
    ray_sphere_875700(o, d, scene.position_38(view)?, radius, p)
}

/// 0x43D240 FindObjectAt(origin, direction, include, hits) for `include`
/// false (the game UIWorldState's vt+0x24 is 0x6A71F0). `hits` receives
/// every hit by head insertion (reverse list order).
pub fn find_object_at_43d240<H: ui_world::Host + Scene>(
    host: &mut H,
    origin: [u32; 3],
    direction: [u32; 3],
    mut hits: Option<&mut Vec<<H as Leaves>::View>>,
) -> Result<Option<<H as Leaves>::View>> {
    let p = Leaves::precision(host);
    let direction = crate::native_camera::ni::normalize(direction, p)?;
    if let Some(list) = hits.as_deref_mut() {
        list.clear();
    }
    let mut best = None;
    let mut best_distance = 0x7f7f_ffff;
    for (index, view) in host.on_list_100()?.into_iter().enumerate() {
        if index >= 50_000 {
            return Err("0x43D2E3 FindObjectAt loop_count diagnostic (more than 50000 VThings) is not ported".into());
        }
        if host.proximity_18(view)? != 2 || !host.pickable_9c(view)? {
            continue;
        }
        if !Selection::<<H as Leaves>::View>::pick_filter_5c19e0(host, Some(view))? {
            continue;
        }
        if !hit_454ac0(host, view, origin, direction)? {
            continue;
        }
        let distance = st(&x87::distance_436480(origin, host.position_38(view)?, p)?)?;
        if host.choose_5c1970(best, best_distance, Some(view), distance)? == Some(view) {
            best = Some(view);
            best_distance = distance;
        }
        if let Some(list) = hits.as_deref_mut() {
            list.insert(0, view);
        }
    }
    Ok(best)
}

/// 0x43DBF0's ray: camera+0x90 and 0x6FE590 of the pixel normalized Y
/// first (`fild`/`fidiv`, binary32 stores).
pub fn screen_ray_43dbf0<S: Scene>(scene: &mut S, pixel: [i32; 2]) -> Result<([u32; 3], [u32; 3])> {
    let p = scene.precision();
    let (width, height) = scene.viewport_b396d0()?;
    let normalized = view_pointer::normalized_pixel(pixel, width, height, p)?;
    let camera = scene.camera()?;
    let origin = [camera.word(0x90)?, camera.word(0x94)?, camera.word(0x98)?];
    let direction = view_pointer::ray_6fe590(camera, normalized, p)?;
    Ok((origin, direction))
}

/// VWorld vt+0x64 = 0x43DC70(pixel): FindObjectAt with a hit list, then
/// UIWorldState vt+0x4C (0x436D70, `single_selected` is its single selected
/// object) and vt+0x54 (0x6409A0).
pub fn pick_at_pixel_43dc70<H: ui_world::Host + Scene>(
    host: &mut H,
    pixel: [i32; 2],
    single_selected: Option<<H as Leaves>::View>,
) -> Result<Option<<H as Leaves>::View>> {
    let (origin, direction) = screen_ray_43dbf0(host, pixel)?;
    let mut hits = Vec::new();
    let best = find_object_at_43d240(host, origin, direction, Some(&mut hits))?;
    let chosen = match single_selected {
        Some(single) if hits.contains(&single) => Some(single),
        _ => best,
    };
    Selection::<<H as Leaves>::View>::pick_target_6409a0(host, chosen)
}

// ---- box selection (0x436DD0 / 0x4371A0 planes, 0x454B50 -> 0x715D40) ----

/// One NiPlane `{normal, constant}` (0x869C30 / 0x869C60).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plane {
    pub normal: [u32; 3],
    pub constant: u32,
}

/// 0x436310(a, b): `normalize(a x b)`; cy is stored binary32 before the
/// length, cx/cz stay in registers; lengths <= 0x9FD38C give zero.
pub fn cross_436310(a: [u32; 3], c: [u32; 3], p: Precision) -> Result<[u32; 3]> {
    let (ax, ay, az) = (b(a[0])?, b(a[1])?, b(a[2])?);
    let (bx, by, bz) = (b(c[0])?, b(c[1])?, b(c[2])?);
    let cx = x87::sub(&x87::mul(&ay, &bz, p)?, &x87::mul(&by, &az, p)?, p)?;
    let cy = st(&x87::sub(&x87::mul(&bx, &az, p)?, &x87::mul(&ax, &bz, p)?, p)?)?;
    let cz = x87::sub(&x87::mul(&by, &ax, p)?, &x87::mul(&ay, &bx, p)?, p)?;
    let cyb = b(cy)?;
    let length = x87::sqrt(
        &x87::add(&x87::add(&x87::mul(&cyb, &cyb, p)?, &x87::mul(&cz, &cz, p)?, p)?, &x87::mul(&cx, &cx, p)?, p)?,
        p,
    )?;
    if x87::le(&length, &b(0x3586_37bd)?) {
        return Ok([0; 3]);
    }
    let inverse = x87::div(&b(0x3f80_0000)?, &length, p)?;
    Ok([st(&x87::mul(&inverse, &cx, p)?)?, st(&x87::mul(&cyb, &inverse, p)?)?, st(&x87::mul(&inverse, &cz, p)?)?])
}

/// `(n.z*q.z + n.y*q.y) + q.x*n.x`, stored: the plane constants of 0x436DD0
/// (q = apex P) and 0x4371A0 (q = eye).
fn constant_zyx(n: [u32; 3], q: [u32; 3], p: Precision) -> Result<u32> {
    st(&x87::add(
        &x87::add(&x87::mul(&b(n[2])?, &b(q[2])?, p)?, &x87::mul(&b(n[1])?, &b(q[1])?, p)?, p)?,
        &x87::mul(&b(q[0])?, &b(n[0])?, p)?,
        p,
    )?)
}

/// 0x436DD0: the four planes of the release box between two events'
/// world points `s` (press, event+0xC) and `e` (release), from the current
/// camera's eye +0x90, point +0x24C and direction +0x114.
pub fn box_planes_436dd0(camera: &impl CameraWords, s: [u32; 3], e: [u32; 3], p: Precision) -> Result<[Plane; 4]> {
    let w = |o: u32| -> Result<Binary> { b(camera.word(o)?) };
    let (dx, dy_bits, dz) = (w(0x114)?, camera.word(0x118)?, w(0x11c)?);
    let dy = b(dy_bits)?;
    let ex = x87::sub(&w(0x90)?, &w(0x24c)?, p)?;
    let ey = x87::sub(&w(0x94)?, &w(0x250)?, p)?;
    let ez = x87::sub(&w(0x98)?, &w(0x254)?, p)?;
    let l = x87::sqrt(&x87::add(&x87::add(&x87::mul(&ez, &ez, p)?, &x87::mul(&ey, &ey, p)?, p)?, &x87::mul(&ex, &ex, p)?, p)?, p)?;
    let half = b(0x3f00_0000)?;
    let (s0, s1, s2) = (b(s[0])?, b(s[1])?, b(s[2])?);
    let (e0, e1, e2) = (b(e[0])?, b(e[1])?, b(e[2])?);
    let sum_z = st(&x87::add(&e2, &s2, p)?)?;
    let mx = st(&x87::mul(&x87::add(&e0, &s0, p)?, &half, p)?)?;
    let my = x87::mul(&x87::add(&e1, &s1, p)?, &half, p)?;
    let mz = st(&x87::mul(&b(sum_z)?, &half, p)?)?;
    let ldx = x87::mul(&l, &dx, p)?;
    let ldy = st(&x87::mul(&l, &dy, p)?)?;
    let ldz = st(&x87::mul(&l, &dz, p)?)?;
    let px = st(&x87::sub(&b(mx)?, &ldx, p)?)?;
    let py = st(&x87::sub(&my, &b(ldy)?, p)?)?;
    let pz = st(&x87::sub(&b(mz)?, &b(ldz)?, p)?)?;
    let wx = st(&x87::sub(&s0, &e0, p)?)?;
    let wy = x87::sub(&s1, &e1, p)?;
    let k = st(&x87::add(&x87::mul(&wy, &dy, p)?, &x87::mul(&b(wx)?, &dx, p)?, p)?)?;
    let kb = b(k)?;
    let dxk = x87::mul(&dx, &kb, p)?;
    let dxk_bits = st(&dxk)?;
    let dyk = x87::mul(&dy, &kb, p)?;
    let k0 = x87::mul(&kb, &b(0)?, p)?;
    let k0_bits = st(&k0)?;
    let ax = st(&x87::add(&b(dxk_bits)?, &e0, p)?)?;
    let ay = st(&x87::add(&dyk, &e1, p)?)?;
    let az = st(&x87::add(&k0, &e2, p)?)?;
    let bxw = st(&x87::sub(&s0, &b(dxk_bits)?, p)?)?;
    let byw = st(&x87::sub(&s1, &dyk, p)?)?;
    let bz = x87::sub(&s2, &b(k0_bits)?, p)?;
    let apex = [px, py, pz];
    let rel = |q: [u32; 3]| -> Result<[u32; 3]> {
        Ok([
            st(&x87::sub(&b(q[0])?, &b(px)?, p)?)?,
            st(&x87::sub(&b(q[1])?, &b(py)?, p)?)?,
            st(&x87::sub(&b(q[2])?, &b(pz)?, p)?)?,
        ])
    };
    let vs = rel(s)?;
    let va = rel([ax, ay, az])?;
    let ve = rel(e)?;
    let vb = [
        st(&x87::sub(&b(bxw)?, &b(px)?, p)?)?,
        st(&x87::sub(&b(byw)?, &b(py)?, p)?)?,
        st(&x87::sub(&bz, &b(pz)?, p)?)?,
    ];
    let plane = |a: [u32; 3], c: [u32; 3]| -> Result<Plane> {
        let normal = cross_436310(a, c, p)?;
        Ok(Plane { normal, constant: constant_zyx(normal, apex, p)? })
    };
    Ok([plane(vs, va)?, plane(va, ve)?, plane(ve, vb)?, plane(vb, vs)?])
}

/// 0x4371A0's planes for the screen rectangle (x0, y0)-(x1, y1) of the
/// box being dragged: the corner rays (min,min), (max,min), (max,max),
/// (min,max) through 0x6FE590 and the eye +0x90.
pub fn screen_box_planes_4371a0(camera: &impl CameraWords, viewport: (i32, i32), rect: [i32; 4], p: Precision) -> Result<[Plane; 4]> {
    let (mut x0, mut y0, mut x1, mut y1) = (rect[0], rect[1], rect[2], rect[3]);
    if x1 < x0 {
        std::mem::swap(&mut x0, &mut x1);
    }
    if y1 < y0 {
        std::mem::swap(&mut y0, &mut y1);
    }
    // fild -> fstp m32 of each coordinate, then `fild B396D4/B396D0; fdivr
    // m32` (y first, then x), each quotient stored binary32 for 0x6FE590.
    let ray = |x: i32, y: i32| -> Result<[u32; 3]> {
        let yf = st(&Binary::from_i32(y))?;
        let ny = st(&x87::div(&b(yf)?, &Binary::from_i32(viewport.1), p)?)?;
        let xf = st(&Binary::from_i32(x))?;
        let nx = st(&x87::div(&b(xf)?, &Binary::from_i32(viewport.0), p)?)?;
        view_pointer::ray_6fe590(camera, [nx, ny], p)
    };
    let r1 = ray(x0, y0)?;
    let r2 = ray(x1, y0)?;
    let r3 = ray(x1, y1)?;
    let r4 = ray(x0, y1)?;
    let eye = [camera.word(0x90)?, camera.word(0x94)?, camera.word(0x98)?];
    let plane = |a: [u32; 3], c: [u32; 3]| -> Result<Plane> {
        let normal = cross_436310(a, c, p)?;
        Ok(Plane { normal, constant: constant_zyx(normal, eye, p)? })
    };
    Ok([plane(r1, r2)?, plane(r2, r3)?, plane(r3, r4)?, plane(r4, r1)?])
}

/// 0x715850: classify the 8 corners of an AABB against a plane: 1 all in
/// front, 2 all behind, 0 when a corner lies on it or they differ.
pub fn classify_box_715850(plane: &Plane, bx: [u32; 6], p: Precision) -> Result<u32> {
    let n = plane.normal;
    let corners = [
        [bx[0], bx[1], bx[2]],
        [bx[3], bx[4], bx[5]],
        [bx[3], bx[1], bx[2]],
        [bx[0], bx[4], bx[5]],
        [bx[3], bx[4], bx[2]],
        [bx[0], bx[1], bx[5]],
        [bx[0], bx[4], bx[2]],
        [bx[3], bx[1], bx[5]],
    ];
    let mut first = None;
    for c in corners {
        let d = x87::sub(
            &x87::add(
                &x87::add(&x87::mul(&b(c[0])?, &b(n[0])?, p)?, &x87::mul(&b(c[2])?, &b(n[2])?, p)?, p)?,
                &x87::mul(&b(c[1])?, &b(n[1])?, p)?,
                p,
            )?,
            &b(plane.constant)?,
            p,
        )?;
        let class = match d.compare(&b(0)?) {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => 2,
            std::cmp::Ordering::Equal => return Ok(0),
        };
        match first {
            None => first = Some(class),
            Some(f) if f != class => return Ok(0),
            Some(_) => {}
        }
    }
    first.ok_or_else(|| "0x715850 corner loop did not run".into())
}

/// 0x454B50 -> 0x715D40(planes): the model bound sphere against the four
/// planes; for each plane it straddles, the node (updated by 0x86E8D0
/// first, through `update_86e8d0`) and the resource AABB are classified in
/// model space. Returns AL.
pub fn model_in_planes_715d40(
    model: &Model,
    planes: &[Plane; 4],
    p: Precision,
    update_86e8d0: &mut dyn FnMut() -> Result<Model>,
) -> Result<bool> {
    let c = model.bound_center_28;
    let r = b(model.bound_radius_34)?;
    let mut straddle = 0u32;
    for (i, plane) in planes.iter().enumerate() {
        let n = plane.normal;
        let d = x87::sub(
            &x87::add(
                &x87::add(&x87::mul(&b(n[2])?, &b(c[2])?, p)?, &x87::mul(&b(n[0])?, &b(c[0])?, p)?, p)?,
                &x87::mul(&b(c[1])?, &b(n[1])?, p)?,
                p,
            )?,
            &b(plane.constant)?,
            p,
        )?;
        if !x87::le(&d, &r) {
            continue;
        }
        if x87::lt(&d, &r.clone().negated()) {
            return Ok(false);
        }
        straddle |= 1 << i;
    }
    if straddle == 0 {
        return Ok(true);
    }
    let model = update_86e8d0()?;
    let (inverse, _) = inverse_8707e0(model.rotate_6c, p)?;
    let t = model.translate_90;
    let bx = model.resource_box_4c.ok_or("0x715E47 reads the AABB of a NULL model resource (native NULL dereference)")?;
    for (i, plane) in planes.iter().enumerate() {
        if straddle & 1 << i == 0 {
            continue;
        }
        let n = plane.normal;
        let nt = x87::add(
            &x87::add(&x87::mul(&b(n[1])?, &b(t[1])?, p)?, &x87::mul(&b(t[2])?, &b(n[2])?, p)?, p)?,
            &x87::mul(&b(n[0])?, &b(t[0])?, p)?,
            p,
        )?;
        let constant = st(&x87::sub(&b(plane.constant)?, &nt, p)?)?;
        let local = Plane { normal: transform_870720(inverse, n, p)?, constant };
        if classify_box_715850(&local, bx, p)? == 2 {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
#[path = "pick_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "box_tests.rs"]
mod box_tests;
