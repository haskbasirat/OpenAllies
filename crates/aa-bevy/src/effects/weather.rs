//! The weather of a battle: World+290 (the map's `_wc.m_weather`, the random map's draw
//! 0x5925E0, the trigger's change_weather) read each frame by the renderer's update 0x725340,
//! which keeps one KKC_WeatherEffect (0x71A600 -> 0x7389B0) and runs it with the frame's time.
//!
//! The effect (0x738C20): a pool of `density * 128` instances of the WeatherData's model
//! (0x738B10, 0x718AD0), of which `min(128, density * visible blocks)` fall at once. A falling
//! one moves along its direction at its speed; past the ground it starts again at
//! `fall_height` over a random visible terrain block, or goes back to the pool when there are
//! too many; off the screen over a block not in view it goes back too. New ones start at a
//! random height (0x738F50). Each start draws its speed `fall_speed * (0.9 + 0.2 * r)` and its
//! direction, down and bent by the wind and by `fall_angle_range` (0x7386D0).
//!
//! Compare against the original (labelled rendition): the terrain block states (+B8: 1 and 2)
//! come from the renderer's cull, here a block counts when its centre is on the screen; the
//! wind (Terrain +44, +48, +4C) is not known and stands still; the second list of blocks
//! (0x738E2F, blocks that just came into view) is the same list; the instance's random numbers
//! are this view's own generator (0x85D840, which does not touch the simulation either).
use super::nif::{NifEffect, NifTable};
use super::player::{Player, Rng};
use super::render::{Quad, View};
use bevy::math::{Mat3, Mat4, Quat, Vec2, Vec3};
use std::path::Path;

/// The models of Data/Weather, for the effect table.
pub const MODELS: [&str; 4] = ["weather/rain/rainheavy.nif", "weather/rain/rainlight.nif", "weather/snow/snowheavy.nif", "weather/snow/snowlight.nif"];
/// AVars TerrainBlockSize.
const BLOCK: f32 = 8.0;
/// 0x738B10's `[0xA89FA4]`, 0x738E94's cap.
const POOL: f32 = 128.0;
const CAP: usize = 128;

/// Templates/template_weather.tgi.
#[derive(Clone, Debug)]
pub struct WeatherDef {
    pub ids: String,
    /// `resource` below Data/Weather, lower case.
    pub model: String,
    pub density: f32,
    pub fall_height: f32,
    pub fall_angle_range: f32,
    pub fall_speed: f32,
    pub wind_affect: f32,
}

/// Data/Weather/*.tgi.
pub fn load_defs(data_root: &Path) -> Vec<WeatherDef> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir(data_root.join("Weather")) else { return out };
    for e in dir.flatten() {
        let path = e.path();
        if !path.extension().is_some_and(|x| x.eq_ignore_ascii_case("tgi")) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let text = String::from_utf8_lossy(&bytes);
        let mut def: Option<WeatherDef> = None;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("[Weather") {
                out.extend(def.take());
                def = Some(WeatherDef { ids: String::new(), model: String::new(), density: 0.0, fall_height: 0.0, fall_angle_range: 0.0, fall_speed: 0.0, wind_affect: 0.0 });
                continue;
            }
            let (Some(d), Some((k, v))) = (def.as_mut(), line.split_once('=')) else { continue };
            let v = v.trim().trim_matches('"');
            let n = v.parse::<f32>().unwrap_or(0.0);
            match k.trim().to_ascii_lowercase().as_str() {
                "ids" => d.ids = v.to_string(),
                "resource" => d.model = format!("weather/{}", v.to_ascii_lowercase()),
                "density" => d.density = n,
                "fall_height" => d.fall_height = n,
                "fall_angle_range" => d.fall_angle_range = n,
                "fall_speed" => d.fall_speed = n,
                "wind_affect_percentage" => d.wind_affect = n,
                _ => {}
            }
        }
        out.extend(def.take());
    }
    out
}

struct Drop {
    falling: bool,
    /// The native point (x, y ground, z up).
    at: Vec3,
    dir: Vec3,
    speed: f32,
    player: Player,
}

/// KKC_WeatherEffect.
pub struct Weather {
    defs: Vec<WeatherDef>,
    def: Option<WeatherDef>,
    drops: Vec<Drop>,
    falling: usize,
    rng: Rng,
    /// +18: the top of a start at a random height.
    top: f32,
    /// +0C: how far under the ground a model goes before it starts again.
    under: f32,
    warned: bool,
}

impl Weather {
    pub fn new(data_root: &Path) -> Self {
        Self { defs: load_defs(data_root), def: None, drops: Vec::new(), falling: 0, rng: Rng::new(0x57EA_7E12), top: 0.0, under: 0.0, warned: false }
    }

    /// 0x71A600: the effect follows World+290.
    fn change(&mut self, ids: Option<&str>, table: &'static NifTable, white: u16, seed: u32) {
        let same = match (&self.def, ids) {
            (None, None) => true,
            (Some(d), Some(i)) => d.ids.eq_ignore_ascii_case(i),
            _ => false,
        };
        if same {
            return;
        }
        self.drops.clear();
        self.falling = 0;
        self.def = ids.and_then(|i| self.defs.iter().find(|d| d.ids.eq_ignore_ascii_case(i)).cloned());
        let Some(def) = &self.def else { return };
        let Some(fx) = table.effects.get(&def.model) else {
            if !self.warned {
                self.warned = true;
                bevy::log::warn!("weather: {} has no model {} in the effect table", def.ids, def.model);
            }
            self.def = None;
            return;
        };
        let fx: &'static NifEffect = fx;
        // 0x738B10
        self.under = 0.0;
        self.top = self.under + def.fall_height;
        let count = (def.density * POOL) as usize;
        for i in 0..count {
            let player = Player::new(fx, Mat4::IDENTITY, 1.0, f32::INFINITY, seed.wrapping_add(i as u32), white);
            self.drops.push(Drop { falling: false, at: Vec3::ZERO, dir: Vec3::NEG_Z, speed: 0.0, player });
        }
        bevy::log::info!("weather: {}, {} models of {}", def.ids, count, def.model);
    }

    /// 0x7386D0: a start's speed and direction.
    fn aim(rng: &mut Rng, def: &WeatherDef) -> (Vec3, f32) {
        let speed = (rng.next() * 0.2 + 0.9) * def.fall_speed;
        let mut dir = Vec3::new(0.0, 0.0, -1.0);
        if def.fall_angle_range != 0.0 && def.wind_affect * 0.01 > 0.0 {
            // the axis 7385C0 picks, bent by the range, turned round the fall
            let side = Vec3::X;
            let bend = Quat::from_axis_angle(side, rng.next() * def.fall_angle_range * std::f32::consts::PI / 180.0);
            let turn = Quat::from_axis_angle(dir, rng.next() * std::f32::consts::TAU);
            dir = (turn * bend * dir).normalize();
        }
        (dir, speed)
    }

    /// 0x738C20. `blocks` are the block corners (native x, y) the screen shows.
    pub fn step(&mut self, dt: f32, ids: Option<&str>, table: &'static NifTable, white: u16, seed: u32, view: &View, ground: &dyn Fn(f32, f32) -> f32, blocks: &[Vec2]) {
        self.change(ids, table, white, seed);
        let Some(def) = self.def.clone() else { return };
        let target = ((def.density * blocks.len() as f32) as usize).min(CAP);
        let mut free: Vec<usize> = Vec::new();
        for i in 0..self.drops.len() {
            if !self.drops[i].falling {
                free.push(i);
                continue;
            }
            let d = &mut self.drops[i];
            let p = d.at + d.dir * d.speed * dt;
            d.at = p;
            if !on_screen(view, p) && !blocks.iter().any(|b| p.x >= b.x && p.y >= b.y && p.x < b.x + BLOCK && p.y < b.y + BLOCK) {
                d.falling = false;
                self.falling -= 1;
                free.push(i);
                continue;
            }
            if p.z < ground(p.x, p.y) - self.under {
                if self.falling > target {
                    d.falling = false;
                    self.falling -= 1;
                    free.push(i);
                    continue;
                }
                if let Some(at) = pick(&mut self.rng, blocks) {
                    d.at = at.extend(def.fall_height);
                    let (dir, speed) = Self::aim(&mut self.rng, &def);
                    let d = &mut self.drops[i];
                    d.dir = dir;
                    d.speed = speed;
                }
            }
        }
        if dt > 0.0 {
            while self.falling < target {
                let Some(i) = free.pop() else { break };
                let Some(at) = pick(&mut self.rng, blocks) else { break };
                let z = self.rng.next() * self.top;
                let (dir, speed) = Self::aim(&mut self.rng, &def);
                let d = &mut self.drops[i];
                d.falling = true;
                d.at = at.extend(z);
                d.dir = dir;
                d.speed = speed;
                self.falling += 1;
            }
        }
        for d in self.drops.iter_mut().filter(|d| d.falling) {
            d.player.set_placement(placement(d.at, d.dir));
            d.player.update(dt, usize::MAX);
        }
    }

    pub fn render(&self, view: &View, out: &mut Vec<Quad>, limit: usize) {
        for d in self.drops.iter().filter(|d| d.falling) {
            if out.len() >= limit {
                break;
            }
            d.player.render(view, out, limit);
        }
    }
}

/// A random point of a random block (0x738F50's draw of a tile, then x and y in it).
fn pick(rng: &mut Rng, blocks: &[Vec2]) -> Option<Vec2> {
    if blocks.is_empty() {
        return None;
    }
    let b = blocks[(rng.next() * blocks.len() as f32) as usize % blocks.len()];
    Some(b + Vec2::new(rng.next(), rng.next()) * BLOCK)
}

/// The camera's frustum at its fixed 15 degrees and 4:3 (camera.rs), native point.
fn on_screen(view: &View, p: Vec3) -> bool {
    let r = crate::battle_view::to_render(p) - view.eye;
    let ahead = -r.dot(view.back);
    if ahead <= 0.0 {
        return false;
    }
    let half = (crate::camera::FOV_DEG.to_radians() * 0.5).tan();
    r.dot(view.up).abs() <= ahead * half && r.dot(view.right).abs() <= ahead * half * (4.0 / 3.0)
}

/// 0x716570 -> 0x871660: the model's up turned against its fall, in render space.
fn placement(at: Vec3, dir: Vec3) -> Mat4 {
    let up = crate::battle_view::to_render(-dir).normalize_or(Vec3::Y);
    let side = if up.y.abs() > 0.99 { Vec3::X } else { up.cross(Vec3::Y).normalize() };
    let front = side.cross(up);
    Mat4::from_rotation_translation(Quat::from_mat3(&Mat3::from_cols(side, up, front)) * Quat::IDENTITY, crate::battle_view::to_render(at))
}

/// The blocks whose centre is on the screen (the states 1 and 2 of 0x738D79).
pub fn visible_blocks(view: &View, ground: &dyn Fn(f32, f32) -> f32, width: f32, height: f32, out: &mut Vec<Vec2>) {
    out.clear();
    // the part of the map under the view: from the eye, at most its reach along the ground
    let eye = view.eye;
    let reach = eye.y.max(1.0) * 1.6;
    let c = Vec2::new(eye.x, -eye.z) + Vec2::new(-view.back.x, view.back.z) * eye.y.max(1.0);
    let (x0, x1) = (((c.x - reach) / BLOCK).floor().max(0.0) as i32, ((c.x + reach) / BLOCK).ceil().min(width / BLOCK) as i32);
    let (y0, y1) = (((c.y - reach) / BLOCK).floor().max(0.0) as i32, ((c.y + reach) / BLOCK).ceil().min(height / BLOCK) as i32);
    for y in y0..y1 {
        for x in x0..x1 {
            let corner = Vec2::new(x as f32 * BLOCK, y as f32 * BLOCK);
            let centre = corner + Vec2::splat(BLOCK * 0.5);
            if on_screen(view, centre.extend(ground(centre.x, centre.y))) {
                out.push(corner);
            }
        }
    }
}
