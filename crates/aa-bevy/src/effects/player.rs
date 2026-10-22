//! Plays one original effect NIF: node keyframes, billboard nodes, sprite
//! quads with alpha/colour/flip controllers and NiPSys particle emitters,
//! evaluated in NIF space and emitted as world-space quads. Ported from the
//! wgpu client's vfx/player.rs (same simulation); the camera-facing corner
//! maths of its particles.wgsl is done here on the CPU.
use super::nif::{Emitter, NifEffect, Sprite, color_at, step};
use super::render::{Quad, View};
use bevy::math::{Mat3, Mat4, Quat, Vec3, Vec4};

/// Native NIF (x, y, z) -> render (x, z, -y).
pub const NIF_TO_RENDER: Mat4 = Mat4::from_cols(
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, -1.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 0.0, 1.0),
);

/// Emission and simulation sub-step (seconds): short emitter windows such as
/// the 1/30 s spark bursts are never skipped at low frame rates.
const SUBSTEP: f32 = 1.0 / 60.0;
/// Particles alive per emitter when the NIF does not say.
const DEFAULT_MAX: usize = 48;

#[derive(Clone, Copy)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9) | 1)
    }
    /// Uniform [0, 1).
    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform [-1, 1).
    pub fn sym(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
}

#[derive(Clone, Copy)]
struct Particle {
    pos: Vec3,
    vel: Vec3,
    age: f32,
    life: f32,
    radius: f32,
    angle: f32,
    spin: f32,
}

#[derive(Default)]
struct EmitterState {
    acc: f32,
    particles: Vec<Particle>,
}

pub struct Player {
    fx: &'static NifEffect,
    /// NIF root -> render space (effect placement * axis conversion).
    transform: Mat4,
    scale: f32,
    pub age: f32,
    /// No emission after this age; sprites hidden after it too.
    pub emit_until: f32,
    /// Multiplies every colour (alpha in w).
    pub tint: Vec4,
    world: Vec<Mat4>,
    /// Per node: 1 = needed for emission, 2 = needed for sprites (the
    /// emitter / sprite nodes and their ancestors); others are skipped.
    needed: Vec<u8>,
    visible: Vec<bool>,
    emitters: Vec<EmitterState>,
    rng: Rng,
    /// Texture layer for NIF texture -1 (untextured).
    white: u16,
}

impl Player {
    pub fn new(fx: &'static NifEffect, placement: Mat4, scale: f32, emit_until: f32, seed: u32, white: u16) -> Self {
        let transform = placement * Mat4::from_scale(Vec3::splat(scale)) * NIF_TO_RENDER;
        let mut player = Self {
            fx,
            transform,
            scale,
            age: 0.0,
            emit_until,
            tint: Vec4::ONE,
            world: vec![Mat4::IDENTITY; fx.nodes.len()],
            needed: needed_nodes(fx),
            visible: vec![true; fx.nodes.len()],
            emitters: fx.emitters.iter().map(|_| EmitterState::default()).collect(),
            rng: Rng::new(seed),
            white,
        };
        // Negative controller start times are pre-roll: simulate them now.
        let preroll = fx.emitters.iter().map(Emitter::preroll).fold(0.0, f32::max).min(2.0);
        if preroll > 0.0 {
            player.age = -preroll;
            let mut left = preroll;
            while left > 1e-4 {
                let dt = left.min(SUBSTEP);
                player.step(dt, usize::MAX);
                left -= dt;
            }
            player.age = 0.0;
        }
        player.eval_nodes();
        player
    }

    /// Move an attached effect with its owner (new placement, same scale).
    pub fn set_placement(&mut self, placement: Mat4) {
        self.transform = placement * Mat4::from_scale(Vec3::splat(self.scale)) * NIF_TO_RENDER;
    }

    pub fn finished(&self) -> bool {
        self.age >= self.emit_until && self.emitters.iter().all(|e| e.particles.is_empty())
    }

    /// Stop emitting now; live particles finish their lives.
    pub fn stop(&mut self) {
        self.emit_until = self.emit_until.min(self.age);
    }

    fn eval_nodes(&mut self) {
        self.eval_nodes_masked(3);
    }

    fn eval_nodes_masked(&mut self, mask: u8) {
        let t = self.age.max(0.0);
        for (i, node) in self.fx.nodes.iter().enumerate() {
            if self.needed[i] & mask == 0 {
                continue;
            }
            let local = node.local(t);
            let mut vis = !node.hidden && node.vis.as_ref().is_none_or(|v| v.visible(t));
            let world = if node.p >= 0 && (node.p as usize) < i {
                vis &= self.visible[node.p as usize];
                self.world[node.p as usize] * local
            } else {
                local
            };
            self.world[i] = world;
            self.visible[i] = vis;
        }
    }

    /// Advance by `dt` seconds. `budget` caps newly emitted particles.
    pub fn update(&mut self, dt: f32, budget: usize) -> usize {
        let mut emitted = 0;
        let mut left = dt.max(0.0);
        while left > 1e-6 {
            let h = left.min(SUBSTEP);
            emitted += self.step(h, budget.saturating_sub(emitted));
            left -= h;
        }
        // Sprite nodes only matter while sprites draw.
        if self.age < self.emit_until && !self.fx.sprites.is_empty() {
            self.eval_nodes_masked(2);
        }
        emitted
    }

    fn step(&mut self, h: f32, budget: usize) -> usize {
        let t0 = self.age;
        self.age += h;
        let emitting = t0 < self.emit_until;
        if emitting && !self.fx.emitters.is_empty() {
            self.eval_at(t0);
        }
        let mut emitted = 0;
        for ei in 0..self.fx.emitters.len() {
            self.simulate(ei, h);
            if emitting && emitted < budget {
                emitted += self.emit(ei, t0, h, budget - emitted);
            }
        }
        emitted
    }

    fn eval_at(&mut self, t: f32) {
        let saved = self.age;
        self.age = t;
        self.eval_nodes_masked(1);
        self.age = saved;
    }

    fn simulate(&mut self, ei: usize, h: f32) {
        let fx: &'static NifEffect = self.fx;
        let e = &fx.emitters[ei];
        let t = self.age.max(0.0);
        let mut accel = Vec3::ZERO;
        for g in &e.gravity {
            if g.active.as_ref().is_some_and(|a| !a.visible(t)) {
                continue;
            }
            let axis = self.transform.transform_vector3(Vec3::from(g.axis));
            accel += axis.normalize_or_zero() * g.strength * self.scale;
        }
        let mut drags: [(Vec3, f32); 4] = [(Vec3::ZERO, 0.0); 4];
        let mut n_drags = 0;
        for d in e.drag.iter().filter(|d| d.active.as_ref().is_none_or(|a| a.visible(t))).take(4) {
            let axis = self.transform.transform_vector3(Vec3::from(d.axis)).normalize_or_zero();
            let k = -(1.0 - d.pct.clamp(0.0, 0.95)).ln() * 30.0;
            drags[n_drags] = (axis, 1.0 - (-k * h).exp());
            n_drags += 1;
        }
        let floor = e.collider.as_ref().map(|c| (self.transform.transform_point3(Vec3::new(0.0, 0.0, c.z)).y, c.bounce, c.die));
        let state = &mut self.emitters[ei];
        state.particles.retain_mut(|p| {
            p.age += h;
            if p.age >= p.life {
                return false;
            }
            p.vel += accel * h;
            for (axis, f) in &drags[..n_drags] {
                let along = p.vel.dot(*axis);
                p.vel -= *axis * along * *f;
            }
            p.pos += p.vel * h;
            p.angle += p.spin * h;
            if let Some((y, bounce, die)) = floor
                && p.pos.y < y
                && p.vel.y < 0.0
            {
                if die {
                    return false;
                }
                p.pos.y = y;
                p.vel.y = -p.vel.y * bounce;
                p.vel.x *= 0.7;
                p.vel.z *= 0.7;
            }
            true
        });
    }

    fn emit(&mut self, ei: usize, t: f32, h: f32, budget: usize) -> usize {
        let fx: &'static NifEffect = self.fx;
        let e = &fx.emitters[ei];
        let shape = &e.emitter;
        let Some(ctl) = &e.ctl.emit else { return 0 };
        let lt = ctl.timing.local(t);
        if !ctl.vis.is_empty() && !step(&ctl.vis, lt) {
            self.emitters[ei].acc = 0.0;
            return 0;
        }
        if shape.active.as_ref().is_some_and(|a| !a.visible(t.max(0.0))) {
            return 0;
        }
        let rate = ctl.rate.as_ref().map_or(0.0, |r| r.eval1(lt)).max(0.0);
        let max = e.max.map_or(DEFAULT_MAX, |m| m as usize).clamp(1, 512);
        let state = &mut self.emitters[ei];
        state.acc += rate * h;
        let room = max.saturating_sub(state.particles.len());
        let mut n = state.acc.floor() as usize;
        state.acc -= n as f32;
        if n > room {
            n = room;
            state.acc = 0.0;
        }
        n = n.min(budget);
        if n == 0 {
            return 0;
        }
        let obj = if shape.obj >= 0 && (shape.obj as usize) < self.world.len() {
            self.world[shape.obj as usize]
        } else if e.node < self.world.len() {
            self.world[e.node]
        } else {
            Mat4::IDENTITY
        };
        let obj_rot = Mat3::from_mat4(obj);
        let size = shape.size.unwrap_or([0.0; 3]);
        let radius = e.ctl.radius.as_ref().map_or(shape.radius, |r| r.eval1(t.max(0.0)));
        let base_speed = e.ctl.speed.as_ref().map_or(shape.speed, |r| r.eval1(t.max(0.0)));
        for _ in 0..n {
            let rng = &mut self.rng;
            let decl = shape.decl + shape.decl_var * rng.sym();
            let planar = shape.planar + shape.planar_var * rng.sym();
            let local_dir = Vec3::new(decl.sin() * planar.cos(), decl.sin() * planar.sin(), decl.cos());
            let (pos_root, dir_root) = match shape.shape.as_str() {
                "mesh" => {
                    let points = shape.points.as_deref().unwrap_or(&[]);
                    if points.is_empty() {
                        (obj.transform_point3(Vec3::ZERO), obj_rot * local_dir)
                    } else {
                        let p = points[((rng.next() * points.len() as f32) as usize).min(points.len() - 1)];
                        let base = match shape.vel_type.unwrap_or(0) {
                            0 => Vec3::new(p[3], p[4], p[5]),
                            1 => Vec3::new(rng.sym(), rng.sym(), rng.sym()),
                            _ => Vec3::from(shape.axis.unwrap_or([0.0, 0.0, 1.0])),
                        }
                        .normalize_or(Vec3::Z);
                        // tilt the base direction by the declination cone
                        let tilt = Quat::from_rotation_arc(Vec3::Z, base);
                        (Vec3::new(p[0], p[1], p[2]), tilt * local_dir)
                    }
                }
                "sphere" => {
                    let d = Vec3::new(rng.sym(), rng.sym(), rng.sym()).normalize_or_zero();
                    let r = size[0] * rng.next().cbrt();
                    (obj.transform_point3(d * r), obj_rot * local_dir)
                }
                "cylinder" => {
                    let a = rng.next() * std::f32::consts::TAU;
                    let r = size[0] * rng.next().sqrt();
                    let z = (rng.next() - 0.5) * size[1];
                    (obj.transform_point3(Vec3::new(a.cos() * r, a.sin() * r, z)), obj_rot * local_dir)
                }
                _ => {
                    let o = Vec3::new((rng.next() - 0.5) * size[0], (rng.next() - 0.5) * size[1], (rng.next() - 0.5) * size[2]);
                    (obj.transform_point3(o), obj_rot * local_dir)
                }
            };
            let speed = (base_speed + shape.speed_var * 0.5 * rng.sym()).max(0.0);
            let life = (shape.life + shape.life_var * 0.5 * rng.sym()).max(shape.life * 0.3).max(0.05);
            let spin = e.rot_speed.unwrap_or(0.0) * if rng.next() < 0.5 { -1.0 } else { 1.0 } * (0.6 + 0.4 * rng.next());
            let angle = if e.rot_speed.is_some() { rng.next() * std::f32::consts::TAU } else { 0.0 };
            // spread spawn times across the sub-step for smooth streams
            let lag = rng.next() * h;
            let vel = self.transform.transform_vector3(dir_root.normalize_or_zero() * speed);
            let pos = self.transform.transform_point3(pos_root) + vel * lag;
            self.emitters[ei].particles.push(Particle { pos, vel, age: lag, life, radius: radius.max(0.005) * self.scale, angle, spin });
        }
        n
    }

    /// Append this frame's quads.
    pub fn render(&self, view: &View, out: &mut Vec<Quad>, limit: usize) {
        if self.age < self.emit_until {
            for sprite in &self.fx.sprites {
                if out.len() >= limit {
                    return;
                }
                self.render_sprite(sprite, view, out, limit);
            }
        }
        for (ei, e) in self.fx.emitters.iter().enumerate() {
            let m = &e.material;
            let layer = if m.tex >= 0 { m.tex as u16 } else { self.white };
            let blend = if m.blended { m.blend } else { 0 };
            let grow = e.grow.unwrap_or(0.0);
            let fade = e.fade.unwrap_or(0.0);
            for p in &self.emitters[ei].particles {
                if out.len() >= limit {
                    return;
                }
                let mut s: f32 = 1.0;
                if grow > 1e-4 && p.age < grow {
                    s = p.age / grow;
                }
                if fade > 1e-4 && p.age > p.life - fade {
                    s = s.min((p.life - p.age) / fade);
                }
                if s <= 0.0 {
                    continue;
                }
                let color = match &e.colors {
                    Some(keys) if !keys.is_empty() => color_at(keys, (p.age / p.life).clamp(0.0, 1.0)),
                    _ => Vec4::from(e.emitter.color),
                } * self.tint;
                if color.w <= 0.002 && blend < 2 {
                    continue;
                }
                out.push(Quad::billboard(view, p.pos, p.radius * s, p.angle, color, layer, blend));
            }
        }
    }

    fn render_sprite(&self, sprite: &Sprite, view: &View, out: &mut Vec<Quad>, limit: usize) {
        let fx = self.fx;
        let t = self.age.max(0.0);
        let node = sprite.node;
        if node >= self.world.len() || !self.visible[node] {
            return;
        }
        let m = &sprite.material;
        let mut alpha = m.akeys.as_ref().map_or(m.alpha, |k| k.eval1(t));
        let mut diffuse = Vec3::from(m.color);
        let mut emissive = Vec3::from(m.emissive);
        if let Some(c) = &m.ckeys {
            let v = Vec3::from(c.track.eval3(t));
            match c.target {
                1 => diffuse = v,
                3 => emissive = v,
                _ => {}
            }
        }
        // Lit approximation: emissive + diffuse under the default sun/ambient.
        let mut rgb = (emissive + diffuse * 0.8).min(Vec3::ONE);
        if let Some(vc) = sprite.vcolor {
            rgb *= Vec3::new(vc[0], vc[1], vc[2]);
            alpha *= vc[3];
        }
        let color = Vec4::new(rgb.x, rgb.y, rgb.z, alpha.clamp(0.0, 1.0)) * self.tint;
        let blend = if m.blended { m.blend } else { 0 };
        if color.w <= 0.003 && blend < 2 {
            return;
        }
        let mut tex = m.tex;
        if let Some(flip) = &m.flip
            && !flip.frames.is_empty()
            && flip.delta > 1e-4
        {
            let lt = flip.timing.local(t) - flip.timing.start;
            let f = ((lt / flip.delta).floor().max(0.0) as usize) % flip.frames.len();
            tex = flip.frames[f];
        }
        let layer = if tex >= 0 { tex as u16 } else { self.white };
        // nearest billboard ancestor (including the node itself)
        let mut bb = None;
        if sprite.post.is_none() {
            let mut i = node as i32;
            while i >= 0 {
                let n = &fx.nodes[i as usize];
                if let Some(mode) = n.bb {
                    bb = Some((i as usize, mode));
                    break;
                }
                i = n.p;
            }
        }
        match bb {
            Some((b, mode)) => {
                let wb = self.world[b];
                if wb.determinant().abs() < 1e-12 {
                    return;
                }
                let rel = wb.inverse() * self.world[node];
                let scale = wb.x_axis.truncate().length() * self.scale;
                let anchor = self.transform.transform_point3(wb.w_axis.truncate());
                // Camera-facing basis, or the up-axis billboard (bb mode 1
                // with the node's Y vertical).
                let (mut right, mut up, mut back) = (view.right, view.up, view.back);
                let mut flip = false;
                if mode == 1 {
                    let y = self.transform.transform_vector3(wb.y_axis.truncate()).normalize_or_zero();
                    if y.y.abs() > 0.5 {
                        flip = y.y < 0.0;
                        let mut h = view.eye - anchor;
                        h.y = 0.0;
                        back = if h.length_squared() > 1e-6 { h.normalize() } else { view.back };
                        up = Vec3::Y;
                        right = up.cross(back);
                    }
                }
                for q in &sprite.quads {
                    if out.len() >= limit || q.len() < 20 {
                        return;
                    }
                    let mut corners = [Vec3::ZERO; 4];
                    let mut uv = [[0.0; 2]; 4];
                    for c in 0..4 {
                        let p = rel.transform_point3(Vec3::new(q[c * 5], q[c * 5 + 1], q[c * 5 + 2])) * scale;
                        let (x, y) = if flip { (-p.x, -p.y) } else { (p.x, p.y) };
                        corners[c] = anchor + right * x + up * y + back * p.z;
                        uv[c] = [q[c * 5 + 3], q[c * 5 + 4]];
                    }
                    out.push(Quad { corners, uv, color: color.to_array(), layer, blend });
                }
            }
            None => {
                let mut to_root = self.world[node];
                if let Some(post) = &sprite.post {
                    to_root = Mat4::from_cols_array(post) * to_root;
                }
                let full = self.transform * to_root;
                for q in &sprite.quads {
                    if out.len() >= limit || q.len() < 20 {
                        return;
                    }
                    let mut corners = [Vec3::ZERO; 4];
                    let mut uv = [[0.0; 2]; 4];
                    for c in 0..4 {
                        corners[c] = full.transform_point3(Vec3::new(q[c * 5], q[c * 5 + 1], q[c * 5 + 2]));
                        uv[c] = [q[c * 5 + 3], q[c * 5 + 4]];
                    }
                    out.push(Quad { corners, uv, color: color.to_array(), layer, blend });
                }
            }
        }
    }
}

/// The nodes emission and sprites read (with their ancestors).
fn needed_nodes(fx: &NifEffect) -> Vec<u8> {
    let mut needed = vec![0u8; fx.nodes.len()];
    let mark = |start: i32, bit: u8, needed: &mut Vec<u8>| {
        let mut i = start;
        while i >= 0 && (i as usize) < fx.nodes.len() {
            if needed[i as usize] & bit != 0 {
                break;
            }
            needed[i as usize] |= bit;
            i = fx.nodes[i as usize].p;
        }
    };
    for e in &fx.emitters {
        mark(e.node as i32, 1, &mut needed);
        mark(e.emitter.obj, 1, &mut needed);
    }
    for s in &fx.sprites {
        mark(s.node as i32, 2, &mut needed);
    }
    needed
}
