//! Original Gamebryo effect NIF data (node keyframes, billboards, sprite quads
//! with alpha/colour/flip controllers, NiPSys particle emitters) extracted by
//! `extract_nif_effects.py` into `nif_effects.json`, plus key evaluation.
//! All positions are in the NIF's Z-up space.

use glam::{Mat4, Quat, Vec3, Vec4};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct NifTable {
    pub format: String,
    pub textures: Vec<String>,
    pub effects: HashMap<String, NifEffect>,
    #[serde(default)]
    pub failures: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct NifEffect {
    pub duration: f32,
    pub nodes: Vec<Node>,
    pub sprites: Vec<Sprite>,
    pub emitters: Vec<Emitter>,
}

#[derive(Deserialize, Debug, Clone, Copy, Default)]
pub struct Timing {
    #[serde(default)]
    pub start: f32,
    #[serde(default)]
    pub stop: f32,
    #[serde(default = "one")]
    pub freq: f32,
    #[serde(default)]
    pub phase: f32,
    /// 0 loop, 1 reverse, 2 clamp.
    #[serde(default)]
    pub cycle: u8,
}

fn one() -> f32 {
    1.0
}

impl Timing {
    /// Controller-local time for an effect age `t`.
    pub fn local(&self, t: f32) -> f32 {
        let t = t * self.freq + self.phase;
        let span = self.stop - self.start;
        if span <= 1e-5 {
            return t.max(self.start);
        }
        match self.cycle {
            0 => self.start + (t - self.start).rem_euclid(span),
            1 => {
                let p = (t - self.start).rem_euclid(span * 2.0);
                self.start + if p > span { span * 2.0 - p } else { p }
            }
            _ => t.clamp(self.start, self.stop),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct KeyGroup {
    /// 1 linear, 2 quadratic (Hermite tangents follow the value), 3 TBC, 5 constant.
    #[serde(default = "linear")]
    pub i: u8,
    #[serde(default)]
    pub k: Vec<Vec<f32>>,
}

fn linear() -> u8 {
    1
}

impl KeyGroup {
    pub fn is_empty(&self) -> bool {
        self.k.is_empty()
    }
    /// Evaluate an `n`-component key track at time `t`.
    pub fn eval<const N: usize>(&self, t: f32) -> [f32; N] {
        let mut out = [0.0; N];
        let Some(first) = self.k.first() else { return out };
        let value = |key: &Vec<f32>, off: usize| -> [f32; N] {
            let mut v = [0.0; N];
            for (i, slot) in v.iter_mut().enumerate() {
                *slot = key.get(1 + off + i).copied().unwrap_or(0.0);
            }
            v
        };
        if t <= first[0] || self.k.len() == 1 {
            return value(first, 0);
        }
        let last = self.k.last().unwrap();
        if t >= last[0] {
            return value(last, 0);
        }
        let idx = self.k.partition_point(|k| k[0] <= t).max(1) - 1;
        let a = &self.k[idx];
        let b = &self.k[(idx + 1).min(self.k.len() - 1)];
        let span = (b[0] - a[0]).max(1e-6);
        let s = ((t - a[0]) / span).clamp(0.0, 1.0);
        let va = value(a, 0);
        let vb = value(b, 0);
        match self.i {
            5 => out = va,
            2 if a.len() >= 1 + 3 * N && b.len() >= 1 + 3 * N => {
                // Gamebryo quadratic keys: Hermite with outgoing (forward) and
                // incoming (backward) tangents per segment.
                let fa = value(a, N);
                let bb = value(b, 2 * N);
                let s2 = s * s;
                let s3 = s2 * s;
                let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
                let h10 = s3 - 2.0 * s2 + s;
                let h01 = -2.0 * s3 + 3.0 * s2;
                let h11 = s3 - s2;
                for i in 0..N {
                    out[i] = h00 * va[i] + h10 * fa[i] + h01 * vb[i] + h11 * bb[i];
                }
            }
            _ => {
                for i in 0..N {
                    out[i] = va[i] + (vb[i] - va[i]) * s;
                }
            }
        }
        out
    }
    pub fn eval1(&self, t: f32) -> f32 {
        self.eval::<1>(t)[0]
    }
    /// Quaternion keys stored as (x, y, z, w); spherical interpolation.
    pub fn eval_quat(&self, t: f32) -> Quat {
        let Some(first) = self.k.first() else { return Quat::IDENTITY };
        let q = |k: &Vec<f32>| {
            Quat::from_xyzw(
                k.get(1).copied().unwrap_or(0.0),
                k.get(2).copied().unwrap_or(0.0),
                k.get(3).copied().unwrap_or(0.0),
                k.get(4).copied().unwrap_or(1.0),
            )
            .normalize()
        };
        if t <= first[0] || self.k.len() == 1 {
            return q(first);
        }
        let last = self.k.last().unwrap();
        if t >= last[0] {
            return q(last);
        }
        let idx = self.k.partition_point(|k| k[0] <= t).max(1) - 1;
        let a = &self.k[idx];
        let b = &self.k[(idx + 1).min(self.k.len() - 1)];
        let s = ((t - a[0]) / (b[0] - a[0]).max(1e-6)).clamp(0.0, 1.0);
        q(a).slerp(q(b), s)
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Keyframes {
    #[serde(flatten)]
    pub timing: Timing,
    pub t: Option<KeyGroup>,
    pub q: Option<KeyGroup>,
    pub s: Option<KeyGroup>,
    /// Euler X, Y, Z float tracks (radians), composed Rz * Ry * Rx.
    pub e: Option<Vec<KeyGroup>>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct VisTrack {
    #[serde(flatten)]
    pub timing: Timing,
    #[serde(default)]
    pub k: Vec<[f32; 2]>,
}

impl VisTrack {
    pub fn visible(&self, t: f32) -> bool {
        step(&self.k, self.timing.local(t))
    }
}

/// Step evaluation of (time, on/off) keys.
pub fn step(keys: &[[f32; 2]], t: f32) -> bool {
    let Some(first) = keys.first() else { return true };
    let mut v = first[1];
    for k in keys {
        if k[0] <= t + 1e-5 {
            v = k[1];
        } else {
            break;
        }
    }
    v > 0.5
}

#[derive(Deserialize, Debug, Clone)]
pub struct Node {
    #[serde(default)]
    pub name: String,
    pub p: i32,
    pub t: [f32; 3],
    pub q: [f32; 4],
    pub s: f32,
    /// Billboard mode: 0 always face camera, 1 rotate about up, 2 rigid face camera.
    pub bb: Option<u8>,
    pub kf: Option<Keyframes>,
    pub vis: Option<VisTrack>,
    #[serde(default)]
    pub hidden: bool,
}

impl Node {
    /// Local transform at effect age `t`.
    pub fn local(&self, t: f32) -> Mat4 {
        let mut tr = Vec3::from(self.t);
        let mut rot = Quat::from_xyzw(self.q[0], self.q[1], self.q[2], self.q[3]);
        let mut sc = self.s;
        if let Some(kf) = &self.kf {
            let lt = kf.timing.local(t);
            if let Some(k) = kf.t.as_ref().filter(|k| !k.is_empty()) {
                tr = Vec3::from(k.eval::<3>(lt));
            }
            if let Some(k) = kf.q.as_ref().filter(|k| !k.is_empty()) {
                rot = k.eval_quat(lt);
            } else if let Some(e) = &kf.e
                && e.len() == 3
                && e.iter().any(|g| !g.is_empty())
            {
                let x = e[0].eval1(lt);
                let y = e[1].eval1(lt);
                let z = e[2].eval1(lt);
                rot = Quat::from_rotation_z(z) * Quat::from_rotation_y(y) * Quat::from_rotation_x(x);
            }
            if let Some(k) = kf.s.as_ref().filter(|k| !k.is_empty()) {
                sc = k.eval1(lt);
            }
        }
        Mat4::from_scale_rotation_translation(Vec3::splat(sc), rot, tr)
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct FloatTrack {
    #[serde(flatten)]
    pub timing: Timing,
    #[serde(flatten)]
    pub keys: KeyGroup,
}

impl FloatTrack {
    pub fn eval1(&self, t: f32) -> f32 {
        self.keys.eval::<1>(self.timing.local(t))[0]
    }
    pub fn eval3(&self, t: f32) -> [f32; 3] {
        self.keys.eval::<3>(self.timing.local(t))
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ColorTrack {
    #[serde(flatten)]
    pub track: FloatTrack,
    /// NiMaterialColorController target: 0 ambient, 1 diffuse, 2 specular, 3 emissive.
    #[serde(default)]
    pub target: u8,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Flip {
    #[serde(flatten)]
    pub timing: Timing,
    #[serde(default)]
    pub delta: f32,
    #[serde(default)]
    pub frames: Vec<i32>,
}

/// Material fields shared by sprites and particle systems.
#[derive(Deserialize, Debug, Clone)]
pub struct Material {
    #[serde(default = "white3")]
    pub color: [f32; 3],
    #[serde(default)]
    pub emissive: [f32; 3],
    #[serde(default = "one")]
    pub alpha: f32,
    /// 0 alpha blend, 1 additive (src alpha, one), 2 additive (one, one).
    #[serde(default)]
    pub blend: u8,
    #[serde(default)]
    pub blended: bool,
    #[serde(default = "minus_one")]
    pub tex: i32,
    #[serde(default)]
    pub vc_emissive: bool,
    pub akeys: Option<FloatTrack>,
    pub ckeys: Option<ColorTrack>,
    pub flip: Option<Flip>,
    pub atest: Option<f32>,
}

fn white3() -> [f32; 3] {
    [1.0; 3]
}
fn minus_one() -> i32 {
    -1
}

#[derive(Deserialize, Debug, Clone)]
pub struct Sprite {
    pub node: usize,
    /// Quads: 4 corners x (x, y, z, u, v); corner order 00, 10, 01, 11.
    pub quads: Vec<Vec<f32>>,
    #[serde(flatten)]
    pub material: Material,
    pub vcolor: Option<[f32; 4]>,
    /// Skin post-transform (column-major) applied after the bone's world.
    pub post: Option<[f32; 16]>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct EmitCtl {
    #[serde(flatten)]
    pub timing: Timing,
    pub rate: Option<KeyGroup>,
    #[serde(default)]
    pub vis: Vec<[f32; 2]>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Controllers {
    pub emit: Option<EmitCtl>,
    pub radius: Option<FloatTrack>,
    pub speed: Option<FloatTrack>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EmitterShape {
    pub speed: f32,
    pub speed_var: f32,
    pub decl: f32,
    pub decl_var: f32,
    pub planar: f32,
    pub planar_var: f32,
    pub color: [f32; 4],
    pub radius: f32,
    pub life: f32,
    pub life_var: f32,
    pub shape: String,
    #[serde(default = "minus_one")]
    pub obj: i32,
    pub size: Option<[f32; 3]>,
    /// Mesh emitter points: position xyz + normal xyz (root space).
    pub points: Option<Vec<[f32; 6]>>,
    pub vel_type: Option<u8>,
    pub axis: Option<[f32; 3]>,
    pub active: Option<VisTrack>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Gravity {
    pub axis: [f32; 3],
    pub strength: f32,
    #[serde(default)]
    pub decay: f32,
    #[serde(rename = "type", default)]
    pub kind: u8,
    #[serde(default)]
    pub turb: f32,
    #[serde(default)]
    pub pos: [f32; 3],
    pub active: Option<VisTrack>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Drag {
    pub axis: [f32; 3],
    pub pct: f32,
    pub active: Option<VisTrack>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Collider {
    pub bounce: f32,
    #[serde(default)]
    pub die: bool,
    pub z: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Emitter {
    pub node: usize,
    #[serde(default)]
    pub world: bool,
    pub max: Option<u32>,
    #[serde(flatten)]
    pub material: Material,
    #[serde(default)]
    pub ctl: Controllers,
    pub emitter: EmitterShape,
    pub grow: Option<f32>,
    pub fade: Option<f32>,
    pub colors: Option<KeyGroup>,
    pub rot_speed: Option<f32>,
    #[serde(default)]
    pub gravity: Vec<Gravity>,
    #[serde(default)]
    pub drag: Vec<Drag>,
    pub collider: Option<Collider>,
}

impl Emitter {
    /// Seconds after which this emitter never emits again (None: loops).
    pub fn emit_end(&self) -> Option<f32> {
        let ctl = self.ctl.emit.as_ref()?;
        if ctl.timing.cycle != 2 {
            return None;
        }
        // last time the visibility track switches on
        let mut end = ctl.timing.stop;
        if let Some(last_on) = ctl.vis.iter().rposition(|k| k[1] > 0.5) {
            end = ctl
                .vis
                .get(last_on + 1)
                .map_or(ctl.timing.stop, |k| k[0]);
        } else if !ctl.vis.is_empty() {
            end = ctl.timing.start;
        }
        Some(end.max(0.0))
    }
    /// Seconds of pre-roll (negative controller start).
    pub fn preroll(&self) -> f32 {
        self.ctl
            .emit
            .as_ref()
            .map_or(0.0, |c| (-c.timing.start).max(0.0))
    }
}

impl NifEffect {
    /// Time after which nothing new appears (all controllers clamped at their end).
    pub fn active_span(&self) -> f32 {
        self.duration.max(0.0)
    }
    pub fn loops(&self) -> bool {
        self.emitters.iter().any(|e| e.emit_end().is_none())
    }
}

/// Colour key track (r, g, b, a over normalized particle age).
pub fn color_at(keys: &KeyGroup, s: f32) -> Vec4 {
    Vec4::from(keys.eval::<4>(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_cycles() {
        let t = Timing { start: 0.0, stop: 2.0, freq: 1.0, phase: 0.0, cycle: 0 };
        assert!((t.local(2.5) - 0.5).abs() < 1e-5);
        let c = Timing { cycle: 2, ..t };
        assert!((c.local(5.0) - 2.0).abs() < 1e-5);
        let r = Timing { cycle: 1, ..t };
        assert!((r.local(3.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn key_interpolation() {
        let g = KeyGroup { i: 1, k: vec![vec![0.0, 0.0], vec![1.0, 10.0]] };
        assert!((g.eval1(0.25) - 2.5).abs() < 1e-4);
        assert!((g.eval1(5.0) - 10.0).abs() < 1e-4);
        let q = KeyGroup { i: 2, k: vec![vec![0.0, 1.0, 0.0, 0.0], vec![1.0, 2.0, 0.0, 0.0]] };
        assert!((q.eval1(0.5) - 1.5).abs() < 1e-4);
    }
}
