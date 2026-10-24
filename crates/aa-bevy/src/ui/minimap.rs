//! The game minimap (KKC_RMinimap) drawn into the GameMiniMap widget's
//! quad (aa-ui `minimap::quad`, the world square as a diamond). It is one
//! image the size of the widget's `minimap_size` (172 x 172), drawn over
//! the whole quad, in which every layer is drawn in its pixels. KKC_RMinimap
//! (`0x712D60`) keeps the widget size (+0xC / +0x10), the texture size (the
//! next powers of two, `0x43A920`: 256) and their ratio as the texture
//! coordinate scale (172 / 256); every layer (`0x4345D0`, `0x433620`,
//! `0x433470`, `0x435BC0`) draws `*(RMinimap + 0xC)` x `*(RMinimap + 0x10)`
//! pixels, so the 256 texture only holds the 172 pixel image in its
//! corner:
//! * the terrain: every target pixel in the `terrain_color` of the cell
//!   under it (Data/Terrain/**/*.tgi: "What color should this terrain be in
//!   the minimap"), from the battle map's cell variations, multiplied by the
//!   local kingdom's fog layer: per target pixel the bilinear blend of the
//!   four tiles around it, visible 1.0, explored 0.7, never explored 0
//!   (`0x433620`), refreshed every AVar MinimapPeriodUpdateFog seconds;
//! * the local kingdom's supply border (`0x433470`): the zone renderer's
//!   border segments of its supply zones (`zone_border::segments`), one
//!   pixel wide in ViewZoneFriendlySupplyColor, the segment starts at half
//!   alpha, blended over the fogged terrain, every MinimapPeriodUpdateSupply
//!   seconds while UVar ViewShowZoneFriendlySupply is set;
//! * the objects layer (`0x435BC0`), redrawn every
//!   MinimapPeriodUpdateObjects seconds into a cleared layer with a
//!   priority byte per pixel (an object pixel replaces only a lower
//!   priority), over the layers above:
//!   - every radar thing (Thing [Flags] `radar`: actors yes, organizations,
//!     city blocks, banks, factories, doodads no, a city centre yes) the
//!     local kingdom sees, and every static one it remembers where it
//!     remembers it, owned as remembered;
//!   - its colour follows its relation to the local kingdom (`0x433B50`):
//!     own selected `color_owned_selected`, own `color_owned_static` /
//!     `_mobile`, allied `color_allied_*`, enemy `color_enemy_*`, an
//!     independent or unowned one `color_neutral_*`, which is drawn only by
//!     its icon;
//!   - its priority (`0x433E40`) starts at the definition's radar_factor, or
//!     `priority_multiplier_icon` when it has a minimap icon, times
//!     `_enemy`, `_owned` (and `_selected`) or `_friendly`, and `_static`;
//!   - how it is drawn (`0x434D70`): a definition with a `minimap_icon`
//!     (Data/MiniMap/Icons) blits the icon's pixels map-aligned (icon
//!     column along world x, icon row along world y), each opaque pixel the
//!     icon colour times the object colour (/256), solid white for a white
//!     colour; a mobile object whose shape radius is below `min_icon_radius`
//!     is a square of `object_point_size` pixels with a
//!     `color_object_border` ring; anything else is its blocking shape
//!     turned by its facing, filled, with a one pixel border;
//!   - last, over everything, the last own selected object's rally point
//!     (static: `rallypoint_icon`) or its current order's target (mobile:
//!     `waypoint_icon`);
//! * the camera's view on the ground in `color_white`;
//! * while the local kingdom's enchantments carry MINIMAP_NO_FOG (modifier
//!   flag 8, Radar Scan) the fog layer (`0x433620`) is clear and the objects
//!   layer (`0x435BC0`) draws every object at its live position, not only
//!   the ones the kingdom sees (the next periodic refresh of each layer
//!   picks the change up).
use super::datafile;
use crate::session::Battle;
use aa_play::fog::{self, Sighting};
use aa_play::{DefClass, Order, Unit};
use bevy::prelude::*;
use std::collections::HashMap;

/// What the front end hands the painter for the minimap.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MinimapDraw {
    /// The widget quad P0, P1, P2 (window pixels).
    pub quad: [(f32, f32); 3],
    /// Lines (window pixels): from, to, colour (the camera view).
    pub lines: Vec<((f32, f32), (f32, f32), [f32; 4])>,
    /// The minimap image is ready.
    pub has_terrain: bool,
}

/// A minimap icon's pixels (RGBA, rows top first).
struct IconPixels {
    w: i32,
    h: i32,
    rgba: Vec<u8>,
}

#[derive(Resource, Default)]
pub struct MinimapArt {
    /// Variation ids (lower case) -> terrain colour.
    colors: Option<HashMap<String, [f32; 3]>>,
    pub image: Option<Handle<Image>>,
    /// Map id, cells and the target size the layers were built for.
    built_for: Option<(String, u32, u32, u32)>,
    /// The fog-free terrain colours (RGBA, image rows top first).
    base: Vec<u8>,
    /// The terrain under the fog with the supply border (the image without
    /// the objects layer).
    fogged: Vec<u8>,
    /// Fog refresh: battle time of the last one and the fog revision seen.
    fog_at: f32,
    fog_revision: u64,
    fog_local: Option<u32>,
    /// MINIMAP_NO_FOG at the last fog refresh.
    fog_no_fog: bool,
    /// The objects layer (RGBA like the image, alpha 0 where empty) and the
    /// battle time it was drawn at.
    objects: Vec<u8>,
    objects_at: Option<f32>,
    /// Minimap TGI values (`0x49B110`).
    pub point_size: i32,
    pub min_icon_radius: f32,
    pub white: [f32; 3],
    pub border: [f32; 3],
    pub owned_selected: [f32; 3],
    pub owned_static: [f32; 3],
    pub owned_mobile: [f32; 3],
    pub enemy_static: [f32; 3],
    pub enemy_mobile: [f32; 3],
    pub allied_static: [f32; 3],
    pub allied_mobile: [f32; 3],
    pub neutral_static: [f32; 3],
    pub neutral_mobile: [f32; 3],
    pub requires_supply: [f32; 3],
    /// priority_multiplier_icon / _selected / _enemy / _owned / _friendly /
    /// _static (bytes).
    pub mul_icon: u8,
    pub mul_selected: u8,
    pub mul_enemy: u8,
    pub mul_owned: u8,
    pub mul_friendly: u8,
    pub mul_static: u8,
    /// minimap.tgi rallypoint_icon / waypoint_icon (Data paths).
    pub rally_icon: String,
    pub waypoint_icon: String,
    /// AVars MinimapPeriodUpdateFog / MinimapPeriodUpdateObjects (s).
    pub period_fog: f32,
    pub period_objects: f32,
    pub period_supply: f32,
    supply_at: f32,
    /// The supply layer's alpha per pixel (image order) of the last supply
    /// refresh, and whether it was shown.
    supply_alpha: Vec<u8>,
    supply_shown: bool,
    /// AVar ViewZoneFriendlySupplyColor (AVars_AA 0,1,0).
    pub supply_color: [f32; 3],
    /// Icon pixels by Data path.
    icons: HashMap<String, Option<IconPixels>>,
}

/// The fog layer tones of `0x433620` (AA.exe data): a visible tile
/// (`0xB28130`, 1.0), an explored one (`0xB2813C`, 0.7) and a never explored
/// one (`0xB39E6C`, zero-initialised: black).
const FOG_VISIBLE: f32 = 1.0;
const FOG_EXPLORED: f32 = 0.7;
const FOG_UNEXPLORED: f32 = 0.0;

/// The definition radar_factor (template_object.tgi `int radar_factor = 1`,
/// def +0x144): only the objective markers set another value and they are
/// invisible things here.
const RADAR_FACTOR: u8 = 1;

fn point3(v: &str) -> Option<[f32; 3]> {
    let p: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    (p.len() >= 3).then(|| [p[0], p[1], p[2]])
}

fn byte(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn rgba(c: [f32; 3]) -> [u8; 4] {
    [byte(c[0]), byte(c[1]), byte(c[2]), 255]
}

impl MinimapArt {
    fn load(&mut self, data: &aa_ui::fs::DataRoot) {
        // the terrain colours, parsed in the background since startup
        let warmed = WARM.lock().ok().and_then(|mut g| g.take()).and_then(|h| h.join().ok());
        self.colors = Some(warmed.unwrap_or_else(|| terrain_colors(data)));
        self.load_minimap(data);
    }
}

/// The terrain colours (every Data/Terrain TGI file) parsed on a background
/// thread from the start, so the first battle frame does not stall on them.
static WARM: std::sync::Mutex<Option<std::thread::JoinHandle<HashMap<String, [f32; 3]>>>> = std::sync::Mutex::new(None);

/// Start parsing the terrain colours in the background (once, at startup).
pub fn warm(root: aa_ui::fs::DataRoot) {
    if let Ok(mut g) = WARM.lock() {
        if g.is_none() {
            *g = Some(std::thread::spawn(move || terrain_colors(&root)));
        }
    }
}

/// Every terrain and variation IDS -> `terrain_color`.
fn terrain_colors(data: &aa_ui::fs::DataRoot) -> HashMap<String, [f32; 3]> {
    let mut colors = HashMap::new();
    {
        let mut dirs = vec!["Terrain".to_string()];
        while let Some(d) = dirs.pop() {
            for f in data.list_dir(&d) {
                if !f.to_ascii_lowercase().ends_with(".tgi") {
                    continue;
                }
                for t in datafile::load(data, &format!("{d}/{f}")).iter().filter(|n| n.name.eq_ignore_ascii_case("Terrain")) {
                    let base = t.get("terrain_color").and_then(point3);
                    if let Some(c) = base {
                        if let Some(ids) = t.get("IDS") {
                            colors.insert(ids.to_ascii_lowercase(), c);
                        }
                    }
                    for v in t.children_named("Variation") {
                        if let (Some(ids), Some(c)) = (v.get("IDS"), v.get("terrain_color").and_then(point3).or(base)) {
                            colors.insert(ids.to_ascii_lowercase(), c);
                        }
                    }
                }
            }
            if let Some(Ok(rd)) = data.find(&d).map(std::fs::read_dir) {
                for e in rd.flatten().filter(|e| e.path().is_dir()) {
                    if let Ok(n) = e.file_name().into_string() {
                        dirs.push(format!("{d}/{n}"));
                    }
                }
            }
        }
    }
    colors
}

impl MinimapArt {
    fn load_minimap(&mut self, data: &aa_ui::fs::DataRoot) {
        let mm = datafile::load(data, "MiniMap/minimap.tgi");
        let node = mm.iter().find(|n| n.name.eq_ignore_ascii_case("Minimap"));
        let c = |name: &str, d: [f32; 3]| node.and_then(|n| n.get(name)).and_then(point3).unwrap_or(d);
        let int = |name: &str, d: f32| node.and_then(|n| n.float(name)).unwrap_or(d);
        self.point_size = int("object_point_size", 2.0) as i32;
        // textures/rallypoint.png, textures/waypoint.png (relative to
        // Data/MiniMap)
        self.rally_icon = node.and_then(|n| n.get("rallypoint_icon")).map(|s| format!("MiniMap/{}", s.trim())).unwrap_or_else(|| "MiniMap/Textures/Rallypoint.PNG".into());
        self.waypoint_icon = node.and_then(|n| n.get("waypoint_icon")).map(|s| format!("MiniMap/{}", s.trim())).unwrap_or_else(|| "MiniMap/Textures/Waypoint.PNG".into());
        self.min_icon_radius = int("min_icon_radius", 2.0);
        self.white = c("color_white", [1.0; 3]);
        self.border = c("color_object_border", [0.0; 3]);
        self.owned_selected = c("color_owned_selected", [1.0; 3]);
        self.owned_static = c("color_owned_static", [0.169, 0.600, 0.145]);
        self.owned_mobile = c("color_owned_mobile", [0.533, 0.863, 0.604]);
        self.enemy_static = c("color_enemy_static", [0.612, 0.145, 0.145]);
        self.enemy_mobile = c("color_enemy_mobile", [0.776, 0.294, 0.294]);
        self.allied_static = c("color_allied_static", [0.106, 0.192, 0.510]);
        self.allied_mobile = c("color_allied_mobile", [0.290, 0.404, 0.827]);
        self.neutral_static = c("color_neutral_static", [0.729; 3]);
        self.neutral_mobile = c("color_neutral_mobile", [0.729; 3]);
        self.requires_supply = c("color_requires_supply", [0.0, 1.0, 0.0]);
        self.mul_selected = int("priority_multiplier_selected", 10.0) as u8;
        self.mul_icon = int("priority_multiplier_icon", 2.0) as u8;
        self.mul_enemy = int("priority_multiplier_enemy", 4.0) as u8;
        self.mul_owned = int("priority_multiplier_owned", 3.0) as u8;
        self.mul_friendly = int("priority_multiplier_friendly", 2.0) as u8;
        self.mul_static = int("priority_multiplier_static", 2.0) as u8;
        // AVars_AA.tgi over AVars.tgi (the hard-coded defaults of 0x401BD0
        // are 5 / 2 s)
        let avars: Vec<_> = datafile::load(data, "AVars_AA.tgi").into_iter().chain(datafile::load(data, "AVars.tgi")).collect();
        let avar = |name: &str, d: f32| avars.iter().find_map(|n| n.float(name)).unwrap_or(d);
        self.supply_color = avars.iter().find_map(|n| n.get("ViewZoneFriendlySupplyColor")).and_then(point3).unwrap_or([0.0, 1.0, 0.0]);
        self.period_fog = avar("MinimapPeriodUpdateFog", 5.0);
        self.period_objects = avar("MinimapPeriodUpdateObjects", 2.0);
        self.period_supply = avar("MinimapPeriodUpdateSupply", 5.0);
    }

    /// Build (once per map) the terrain layer at the minimap target size
    /// (`target` pixels square), lay the local kingdom's fog over it every
    /// MinimapPeriodUpdateFog seconds and its supply border every
    /// MinimapPeriodUpdateSupply seconds, and draw the objects layer every
    /// MinimapPeriodUpdateObjects seconds over them.
    pub fn terrain(&mut self, data: &aa_ui::fs::DataRoot, b: &Battle, images: &mut Assets<Image>, target: u32, show_supply: bool) {
        if self.colors.is_none() {
            let t0 = std::time::Instant::now();
            self.load(data);
            info!("minimap: data loaded in {:.0} ms", t0.elapsed().as_secs_f32() * 1000.0);
        }
        let m = &b.world.map;
        let key = (m.id.clone(), m.width, m.height, target);
        if m.width == 0 || m.height == 0 {
            return;
        }
        let (w, h) = (m.width as usize, m.height as usize);
        let t = target.max(1) as usize;
        // the map cell under a target pixel (row 0 = world y = 0)
        let cell = |tx: usize, ty: usize| (ty * h / t).min(h - 1) * w + (tx * w / t).min(w - 1);
        if self.built_for.as_ref() != Some(&key) {
            self.built_for = Some(key);
            let colors = self.colors.as_ref().unwrap();
            // 0x4345D0: every target pixel in the colour of the cell under it
            let mut px = vec![0u8; t * t * 4];
            for ty in 0..t {
                for tx in 0..t {
                    let c = m
                        .cell_variation
                        .get(cell(tx, ty))
                        .and_then(|&v| m.variations.get(v as usize))
                        .and_then(|id| colors.get(&id.to_ascii_lowercase()))
                        .copied()
                        .unwrap_or([0.5, 0.5, 0.5]);
                    // image row 0 = the map's top (world y = H)
                    let i = ((t - 1 - ty) * t + tx) * 4;
                    px[i..i + 4].copy_from_slice(&[byte(c[0]), byte(c[1]), byte(c[2]), 255]);
                }
            }
            self.base = px.clone();
            self.fogged = px.clone();
            self.objects = vec![0u8; t * t * 4];
            let mut img = super::tex::make_image(t as u32, t as u32, px, super::tex::Variant::default(), true);
            img.sampler = bevy::image::ImageSampler::linear();
            self.image = Some(images.add(img));
            self.fog_at = f32::NEG_INFINITY;
            self.supply_at = f32::NEG_INFINITY;
            self.fog_local = None;
            self.objects_at = None;
        }
        // the supply border (0x433470), every MinimapPeriodUpdateSupply
        // seconds while UVar ViewShowZoneFriendlySupply is set: every border
        // segment of the local kingdom's supply zones (the zone renderer's
        // axis-aligned tile-edge segments, `zone_border::segments`) drawn by
        // 0x4329A0 in ViewZoneFriendlySupplyColor, one pixel wide: from the
        // lower end to the upper one in whole pixel steps, the first (and
        // an exactly reached last) pixel at half alpha; the layer is then
        // blended over the fogged terrain by its alpha (0x712900 mode 1)
        let now = b.world.time;
        let mut supply_changed = false;
        if now - self.supply_at >= self.period_supply || now < self.supply_at || show_supply != self.supply_shown {
            self.supply_at = now;
            self.supply_shown = show_supply;
            let mut alpha = vec![0u8; t * t];
            if show_supply {
                let inside = super::zone_border::supply_tiles(b);
                let (mw, mh) = (m.width as f32 * m.cell_size, m.height as f32 * m.cell_size);
                let (sx, sy) = (t as f32 / mw.max(1e-3), t as f32 / mh.max(1e-3));
                let mut put = |col: i32, row: i32, a: u8| {
                    if col >= 0 && row >= 0 && (col as usize) < t && (row as usize) < t {
                        // image row 0 = the map's top
                        alpha[(t - 1 - row as usize) * t + col as usize] = a;
                    }
                };
                for (p, q) in super::zone_border::segments(m.width, m.height, &inside) {
                    let (x0, y0) = (p.x as f32 * m.cell_size * sx, p.y as f32 * m.cell_size * sy);
                    let (x1, y1) = (q.x as f32 * m.cell_size * sx, q.y as f32 * m.cell_size * sy);
                    if x0 != x1 {
                        if y0 != y1 {
                            continue;
                        }
                        let (lo, hi) = (x0.min(x1), x0.max(x1));
                        let mut x = lo;
                        while x <= hi {
                            put(x as i32, y0 as i32, if x == lo || x == hi { 127 } else { 255 });
                            x += 1.0;
                        }
                    } else {
                        let (lo, hi) = (y0.min(y1), y0.max(y1));
                        let mut y = lo;
                        while y <= hi {
                            put(x0 as i32, y as i32, if y == lo || y == hi { 127 } else { 255 });
                            y += 1.0;
                        }
                    }
                }
            }
            if alpha != self.supply_alpha {
                self.supply_alpha = alpha;
                supply_changed = true;
            }
        }
        let mut dirty = self.fog_layer(b, t, supply_changed);
        // the objects layer every MinimapPeriodUpdateObjects
        if self.objects_at.is_none_or(|at| now - at >= self.period_objects || now < at) {
            self.objects_at = Some(now);
            self.objects_layer(data, b, t);
            dirty = true;
        }
        if !dirty {
            return;
        }
        let Some(mut img) = self.image.as_ref().and_then(|h| images.get_mut(h)) else { return };
        let Some(px) = img.data.as_mut() else { return };
        if px.len() != self.fogged.len() || self.objects.len() != px.len() {
            return;
        }
        // the objects over the layers below them
        for i in (0..px.len()).step_by(4) {
            let src = if self.objects[i + 3] != 0 { &self.objects[i..i + 4] } else { &self.fogged[i..i + 4] };
            px[i..i + 4].copy_from_slice(src);
        }
    }

    /// The fog layer over the terrain, with the supply border, into
    /// `fogged`; whether it changed.
    fn fog_layer(&mut self, b: &Battle, t: usize, supply_changed: bool) -> bool {
        let now = b.world.time;
        let due = now - self.fog_at >= self.period_fog || now < self.fog_at;
        let no_fog = minimap_no_fog(b);
        let changed = self.fog_revision != b.world.fog.revision || self.fog_local != Some(b.local) || self.fog_no_fog != no_fog;
        if !(due && changed) && self.fog_local.is_some() && !supply_changed {
            return false;
        }
        self.fog_at = now;
        self.fog_revision = b.world.fog.revision;
        self.fog_local = Some(b.local);
        self.fog_no_fog = no_fog;
        // tile states: 0 visible, 1 explored, 255 never explored
        let mut shade = Vec::new();
        fog::shade(&b.world, Some(b.local), 1, &mut shade);
        let (fw, fh) = (b.world.fog.width.max(0) as usize, b.world.fog.height.max(0) as usize);
        let px = &mut self.fogged;
        if px.len() != self.base.len() || fw == 0 || fh == 0 || shade.len() != fw * fh {
            return false;
        }
        // 0x433620: each target pixel blends the tones of the four tiles
        // around its point (bilinear, the tile index at the point's lower
        // corner); the kingdom's MINIMAP_NO_FOG flag shows every tile as
        // visible. The fog layer multiplies the terrain layer.
        let tone = |s: u8| {
            if no_fog || s == 0 {
                FOG_VISIBLE
            } else if s == 255 {
                FOG_UNEXPLORED
            } else {
                FOG_EXPLORED
            }
        };
        let axis = |p: usize, tiles: usize| {
            let f = p as f32 * tiles as f32 / t as f32;
            let i = (f as usize).min(tiles - 1);
            let j = if (i as f32) < tiles as f32 - 1.0 { i + 1 } else { i };
            (i, j, f - i as f32)
        };
        for ty in 0..t {
            let (y0, y1, wy) = axis(ty, fh);
            for tx in 0..t {
                let (x0, x1, wx) = axis(tx, fw);
                let s = |x: usize, y: usize| tone(shade[y * fw + x]);
                let v = s(x0, y0) * (1.0 - wx) * (1.0 - wy) + s(x1, y0) * wx * (1.0 - wy) + s(x0, y1) * (1.0 - wx) * wy + s(x1, y1) * wx * wy;
                // 0x4324A0: the tone as a byte (truncated)
                let fog = u16::from((v * 255.0).clamp(0.0, 255.0) as u8);
                let i = ((t - 1 - ty) * t + tx) * 4;
                for c in 0..3 {
                    px[i + c] = ((u16::from(self.base[i + c]) * fog) / 255) as u8;
                }
            }
        }
        // 0x712900 mode 1: the supply layer blended by its alpha
        let green = self.supply_color.map(byte);
        if self.supply_alpha.len() * 4 == px.len() {
            for (i, &a) in self.supply_alpha.iter().enumerate().filter(|(_, a)| **a != 0) {
                let a = u16::from(a);
                for c in 0..3 {
                    let d = u16::from(px[i * 4 + c]);
                    px[i * 4 + c] = (((255 - a) * d + u16::from(green[c]) * a) >> 8) as u8;
                }
            }
        }
        true
    }

    /// The icon's pixels (loaded once per path).
    fn icon(&mut self, data: &aa_ui::fs::DataRoot, path: &str) -> Option<&IconPixels> {
        if !self.icons.contains_key(path) {
            let found = data.find(path.trim_start_matches('/')).and_then(|p| image::open(&p).ok()).map(|img| {
                let img = img.to_rgba8();
                IconPixels { w: img.width() as i32, h: img.height() as i32, rgba: img.into_raw() }
            });
            if found.is_none() {
                warn!("minimap: icon {path} not found");
            }
            self.icons.insert(path.to_string(), found);
        }
        self.icons.get(path).and_then(|i| i.as_ref())
    }

    /// `0x435BC0`: the objects layer, drawn from scratch.
    fn objects_layer(&mut self, data: &aa_ui::fs::DataRoot, b: &Battle, t: usize) {
        let w = &b.world;
        let map = &w.map;
        let (mw, mh) = (map.width as f32 * map.cell_size, map.height as f32 * map.cell_size);
        let mut r = Raster { t: t as i32, px: vec![[0u8; 4]; t * t], prio: vec![0u8; t * t], border: rgba(self.border) };
        let (sx, sy) = (t as f32 / mw.max(1e-3), t as f32 / mh.max(1e-3));
        let selected: std::collections::HashSet<u32> = b
            .selection
            .iter()
            .flat_map(|u| match *u {
                Unit::Org(o) => w.orgs.get(o as usize).map(|o| o.members.clone()).unwrap_or_default(),
                Unit::Actor(a) => vec![a],
            })
            .collect();
        let local_team = w.kingdoms.iter().find(|k| k.id == b.local).map(|k| k.team);
        // 0x435BC0: MINIMAP_NO_FOG draws every object as if it were seen
        let no_fog = minimap_no_fog(b);
        // the last own selected object (0x433E40 records it)
        let mut marked: Option<aa_play::ActorId> = None;
        let mut objects: Vec<(aa_play::DefId, Vec3, f32, Option<u32>, bool, Option<aa_play::ActorId>)> = Vec::new();
        for a in w.actors.iter().filter(|a| a.alive && !a.removed) {
            // only radar things are on the minimap (Thing [Flags] radar:
            // actors yes; organizations, city blocks, banks, factories,
            // doodads, mines and crates no; a city centre yes)
            let d = w.data.def(a.def);
            if a.kingdom.is_none() || d.invisible || !d.radar {
                continue;
            }
            let sighting = if no_fog { Sighting::Visible } else { fog::sighting(w, a.id, Some(b.local)) };
            // the owner as the local kingdom saw it last
            let (pos, facing, owner) = match sighting {
                Sighting::Visible => (a.pos, a.facing, a.kingdom),
                Sighting::Remembered(m) => (m.pos, m.facing, m.kingdom),
                Sighting::Hidden => continue,
            };
            objects.push((a.def, pos, facing, owner, selected.contains(&a.id), Some(a.id)));
        }
        // ghosts of remembered actors already gone (none while everything
        // live is shown)
        for (id, m) in fog::memories(w, Some(b.local)).into_iter().filter(|_| !no_fog) {
            if w.actor(id).is_some_and(|a| a.alive) || !w.data.def(m.def).radar {
                continue;
            }
            objects.push((m.def, m.pos, m.facing, m.kingdom, false, None));
        }
        for (def_id, pos, facing, owner, is_selected, id) in objects {
            let def = w.data.def(def_id);
            // actor flags bit 29: a structure that does not move
            let mobile = def.speed_max > 0.0 || matches!(def.class, Some(DefClass::Member) | Some(DefClass::Truck));
            let kingdom = owner.and_then(|k| w.kingdoms.iter().find(|x| x.id == k));
            let relation = match kingdom {
                Some(k) if k.id == b.local => Relation::Own,
                Some(k) if k.independent => Relation::Other,
                Some(k) if Some(k.team) == local_team => Relation::Allied,
                Some(_) => Relation::Enemy,
                None => Relation::Other,
            };
            let selected = relation == Relation::Own && is_selected;
            let icon = def.minimap_icon.clone();
            // 0x433B50: the relation colour; an object of no relation is
            // drawn only by its icon
            let color = match relation {
                Relation::Own if selected => self.owned_selected,
                Relation::Own if mobile => self.owned_mobile,
                Relation::Own => self.owned_static,
                Relation::Allied if mobile => self.allied_mobile,
                Relation::Allied => self.allied_static,
                Relation::Enemy if mobile => self.enemy_mobile,
                Relation::Enemy => self.enemy_static,
                Relation::Other if mobile => self.neutral_mobile,
                Relation::Other => self.neutral_static,
            };
            if relation == Relation::Other && icon.is_none() {
                continue;
            }
            // 0x433E40: the priority (byte arithmetic)
            let mut p = if icon.is_some() { self.mul_icon } else { RADAR_FACTOR };
            match relation {
                Relation::Enemy => p = p.wrapping_mul(self.mul_enemy),
                Relation::Own => {
                    p = p.wrapping_mul(self.mul_owned);
                    if selected {
                        p = p.wrapping_mul(self.mul_selected);
                        marked = id.or(marked);
                    }
                }
                Relation::Allied => p = p.wrapping_mul(self.mul_friendly),
                Relation::Other => {}
            }
            if !mobile {
                p = p.wrapping_mul(self.mul_static);
            }
            let c = rgba(color);
            let at = pos.truncate();
            // 0x434D70
            if let Some(px) = icon.as_deref().and_then(|i| self.icon(data, i)) {
                blit(&mut r, px, at, sx, c, p);
                continue;
            }
            let shape = Shape::of(def);
            if mobile && shape.radius() < self.min_icon_radius {
                // the point at the truncated position
                let x0 = ((at.x.trunc()) * sx) as i32;
                let y0 = ((at.y.trunc()) * sy) as i32;
                point(&mut r, x0, y0, self.point_size, c, p);
            } else {
                footprint(&mut r, &shape, at, Vec2::new(facing.cos(), facing.sin()), sx, sy, c, p);
            }
        }
        // the marked object's rally point (static) or its current order's
        // target (mobile), over everything (priority 0xFF, colour
        // 0xFFFEFFFF: the icon's own colours)
        if let Some(id) = marked {
            let a = &w.actors[id as usize];
            let def = w.data.def(a.def);
            let mobile = def.speed_max > 0.0 || matches!(def.class, Some(DefClass::Member) | Some(DefClass::Truck));
            let (icon, at) = if !mobile {
                (self.rally_icon.clone(), aa_play::player_orders::rally_point(w, id))
            } else {
                let order = match a.org.and_then(|o| w.orgs.get(o as usize)) {
                    Some(g) => g.order.clone(),
                    None => a.order.clone(),
                };
                let at = match order {
                    Order::Move { to, .. } => Some(to),
                    Order::Deploy { at, .. } => Some(at),
                    Order::Attack { target } => w.actor(target).filter(|t| t.alive).map(|t| t.pos.truncate()),
                    Order::Idle => None,
                };
                (self.waypoint_icon.clone(), at)
            };
            if let Some(at) = at {
                if let Some(px) = self.icon(data, &icon) {
                    blit(&mut r, px, at, sx, [0xFE, 0xFF, 0xFF, 0xFF], 0xFF);
                }
            }
        }
        // into the image's row order (row 0 = world y = H)
        let out = &mut self.objects;
        out.clear();
        out.resize(t * t * 4, 0);
        for y in 0..t {
            for x in 0..t {
                let i = ((t - 1 - y) * t + x) * 4;
                out[i..i + 4].copy_from_slice(&r.px[y * t + x]);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Relation {
    Own,
    Allied,
    Enemy,
    Other,
}

/// The blocking shape (`0x46D8E0` circle, `0x46E0E0` square / rect: half
/// extents and the bounding radius).
enum Shape {
    Point,
    Circle(f32),
    Box(f32, f32),
}

impl Shape {
    fn of(def: &aa_play::Def) -> Shape {
        match (def.blocking_shape.as_deref().map(str::to_ascii_uppercase).as_deref(), def.blocking_size) {
            (Some("CIRCLE"), Some(s)) => Shape::Circle(s[0]),
            (Some("SQUARE") | Some("RECT"), Some(s)) => Shape::Box(s[0] * 0.5, s[1] * 0.5),
            _ => Shape::Point,
        }
    }

    /// Shape +4.
    fn radius(&self) -> f32 {
        match *self {
            Shape::Point => 0.0,
            Shape::Circle(r) => r,
            Shape::Box(hx, hy) => (hx * hx + hy * hy).sqrt(),
        }
    }

    /// The half extents the outline uses (a circle: its radius both ways).
    fn half(&self) -> (f32, f32) {
        match *self {
            Shape::Point => (0.0, 0.0),
            Shape::Circle(r) => (r, r),
            Shape::Box(hx, hy) => (hx, hy),
        }
    }
}

/// The objects layer of one refresh: colour and priority per target pixel
/// (row 0 = world y = 0).
struct Raster {
    t: i32,
    px: Vec<[u8; 4]>,
    prio: Vec<u8>,
    border: [u8; 4],
}

impl Raster {
    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.t && y < self.t).then(|| (y * self.t + x) as usize)
    }

    /// An object pixel: it replaces only a lower priority.
    fn put(&mut self, x: i32, y: i32, c: [u8; 4], p: u8) {
        if let Some(i) = self.idx(x, y) {
            if self.prio[i] < p {
                self.px[i] = c;
                self.prio[i] = p;
            }
        }
    }

    /// `0x432C30`: the border colour next to (x, y) where that pixel is not
    /// the fill colour and holds a lower priority than `p` (the border does
    /// not take the priority).
    fn edge(&mut self, x: i32, y: i32, dx: i32, dy: i32, fill: [u8; 4], p: u8) {
        if let Some(i) = self.idx(x + dx, y + dy) {
            if self.px[i] != fill && self.prio[i] < p {
                self.px[i] = self.border;
            }
        }
    }
}

/// `0x434D70` icon: the icon's opaque pixels centred on the object
/// (column x - w/2 .. along world x, row y - h/2 .. along world y; both
/// from the x scale), each the icon colour times `c` / 256, solid white
/// for a white `c`.
fn blit(r: &mut Raster, icon: &IconPixels, at: Vec2, s: f32, c: [u8; 4], p: u8) {
    let (cx, cy) = ((at.x * s) as i32, (at.y * s) as i32);
    let white = c[0] == 0xFF && c[1] == 0xFF && c[2] == 0xFF;
    for row in 0..icon.h {
        let y = row - (icon.h >> 1) + cy;
        for col in 0..icon.w {
            let x = cx - (icon.w >> 1) + col;
            let Some(i) = r.idx(x, y) else { continue };
            let k = ((row * icon.w + col) * 4) as usize;
            let src = &icon.rgba[k..k + 4];
            if r.prio[i] >= p || src[3] == 0 {
                continue;
            }
            r.px[i] = if white {
                [0xFF, 0xFF, 0xFF, 0xFF]
            } else {
                let m = |a: u8, b: u8| ((u16::from(a) * u16::from(b)) >> 8) as u8;
                [m(src[0], c[0]), m(src[1], c[1]), m(src[2], c[2]), 0xFF]
            };
            r.prio[i] = p;
        }
    }
}

/// `0x434D70` point: an `n` pixel square (from x - n/2) with a border ring.
fn point(r: &mut Raster, x0: i32, y0: i32, n: i32, c: [u8; 4], p: u8) {
    let half = n >> 1;
    let t = r.t;
    for row in 0..n {
        let y = row - half + y0;
        if y < 0 || y >= t {
            continue;
        }
        for col in 0..n {
            let x = col - half + x0;
            if x < 0 || x >= t {
                continue;
            }
            r.put(x, y, c, p);
            let (first_col, last_col) = (col == 0 && x > 0, col == n - 1 && x < t - 1);
            if row == 0 && y > 0 {
                if first_col {
                    r.edge(x, y, -1, -1, c, p);
                }
                r.edge(x, y, 0, -1, c, p);
                if last_col {
                    r.edge(x, y, 1, -1, c, p);
                }
            }
            if first_col {
                r.edge(x, y, -1, 0, c, p);
            }
            if last_col {
                r.edge(x, y, 1, 0, c, p);
            }
            if row == n - 1 && y < t - 1 {
                if first_col {
                    r.edge(x, y, -1, 1, c, p);
                }
                r.edge(x, y, 0, 1, c, p);
                if last_col {
                    r.edge(x, y, 1, 1, c, p);
                }
            }
        }
    }
}

/// `0x42F330`: where segment p0-p1 crosses segment q0-q1 (both ends
/// included).
fn cross(p0: Vec2, p1: Vec2, q0: Vec2, q1: Vec2) -> Option<Vec2> {
    let d = p1 - p0;
    let e = q1 - q0;
    let den = d.x * e.y - e.x * d.y;
    if den == 0.0 {
        return None;
    }
    let s = ((p0.y - q0.y) * e.x - (p0.x - q0.x) * e.y) * (1.0 / den);
    let u = ((p0.y - q0.y) * d.x - (p0.x - q0.x) * d.y) * (1.0 / den);
    ((0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&u)).then(|| p0 + d * s)
}

/// `0x434D70` outline: the shape's box turned by the facing, filled row by
/// row inside the shape's bounds (`0x46E540`), with a one pixel border.
#[allow(clippy::too_many_arguments)]
fn footprint(r: &mut Raster, shape: &Shape, at: Vec2, dir: Vec2, sx: f32, sy: f32, c: [u8; 4], p: u8) {
    let (hx, hy) = shape.half();
    // bounds (world units)
    let (lo, hi) = match shape {
        Shape::Point => (at, at),
        Shape::Circle(rad) => (at - Vec2::splat(*rad), at + Vec2::splat(*rad)),
        Shape::Box(..) => {
            let a = Vec2::new(dir.y * hx, -dir.x * hx);
            let b = Vec2::new(dir.x * hy, dir.y * hy);
            let cs = [at - a - b, at - a + b, at + a + b, at + a - b];
            (cs.iter().copied().reduce(Vec2::min).unwrap(), cs.iter().copied().reduce(Vec2::max).unwrap())
        }
    };
    let (x_lo, x_hi) = (lo.x * sx - 1.0, hi.x * sx + 1.0);
    let (mut y, y_hi) = (lo.y * sy, hi.y * sy);
    let height = y_hi - y;
    if height < 1.0 {
        y += height * 0.5;
    }
    // the corners in target pixels
    let (cx, cy) = (at.x * sx, at.y * sy);
    let (px, py) = (hx * sx, hy * sy);
    let a = Vec2::new(dir.y * px, -dir.x * px);
    let b = Vec2::new(dir.x * py, dir.y * py);
    let c0 = Vec2::new(cx, cy);
    let corners = [c0 - a - b, c0 - a + b, c0 + a + b, c0 + a - b];
    let t = r.t;
    while y <= y_hi {
        // rows and columns truncate toward zero (ftol)
        let row = y as i32;
        if row < 0 {
            y += 1.0;
            continue;
        }
        if row >= t {
            return;
        }
        let mut xs = Vec::with_capacity(4);
        for i in 0..4 {
            if let Some(q) = cross(Vec2::new(x_lo, y), Vec2::new(x_hi, y), corners[i], corners[(i + 1) % 4]) {
                xs.push(q.x);
            }
        }
        if xs.len() >= 2 {
            let from = xs.iter().copied().fold(f32::MAX, f32::min);
            let to = xs.iter().copied().fold(f32::MIN, f32::max);
            let mut x = from;
            while x <= to {
                let col = x as i32;
                if col >= 0 {
                    if col >= t {
                        break;
                    }
                    let q = p.wrapping_sub(1);
                    if x == from && col > 0 {
                        if row > 0 {
                            r.edge(col, row, -1, -1, c, q);
                        }
                        r.edge(col, row, -1, 0, c, q);
                        if row < t - 1 {
                            r.edge(col, row, -1, 1, c, q);
                        }
                    }
                    if row > 0 {
                        r.edge(col, row, 0, -1, c, q);
                    }
                    r.put(col, row, c, p);
                    if row < t - 1 {
                        r.edge(col, row, 0, 1, c, q);
                    }
                    if to <= x + 1.0 && col < t - 1 {
                        if row > 0 {
                            r.edge(col, row, 1, -1, c, q);
                        }
                        r.edge(col, row, 1, 0, c, q);
                        if row < t - 1 {
                            r.edge(col, row, 1, 1, c, q);
                        }
                    }
                }
                x += 1.0;
            }
        }
        y += 1.0;
    }
}

/// The local kingdom's enchantments carry MINIMAP_NO_FOG (Radar Scan;
/// `0x4CF110` flags bit 8).
fn minimap_no_fog(b: &Battle) -> bool {
    aa_play::special_ops::kingdom_has_flag(&b.world, b.local, "MINIMAP_NO_FOG")
}

/// The GameMiniMap widget's layer size in pixels (`minimap_size`, 172; the
/// layers are drawn at the widget size, not at the 256 texture size).
pub fn target_pixels(ui: &aa_ui::ui::Ui) -> u32 {
    let size = match ui.widget("Root/Game/ControlPanel/GameMiniMap").and_then(|w| w.control.as_deref()) {
        Some(aa_ui::controls::Control::MiniMap(m)) => m.size,
        _ => aa_ui::minimap::MiniMap::default().size,
    };
    size.0.max(size.1).max(1) as u32
}

/// The minimap content for this frame. `view` is the camera's view on the
/// ground (native points of the window corners, clockwise from top left).
pub fn draw(ui: &aa_ui::ui::Ui, art: &mut MinimapArt, b: &Battle, rect: aa_ui::layout::Rect, view: Option<[Vec2; 4]>) -> Option<MinimapDraw> {
    let w = ui.widget("Root/Game/ControlPanel/GameMiniMap")?;
    let m = match w.control.as_deref() {
        Some(aa_ui::controls::Control::MiniMap(m)) => m.clone(),
        _ => aa_ui::minimap::MiniMap::default(),
    };
    let quad = aa_ui::minimap::quad(&m, rect);
    let map = &b.world.map;
    let (mw, mh) = (map.width as f32 * map.cell_size, map.height as f32 * map.cell_size);
    if mw <= 0.0 || mh <= 0.0 {
        return None;
    }
    let [p0, p1, p2] = quad;
    let pt = |u: f32, v: f32| (p0.0 + u * (p2.0 - p1.0) + v * (p1.0 - p0.0), p0.1 + u * (p2.1 - p1.1) + v * (p1.1 - p0.1));
    let mut lines = Vec::new();
    if let Some(v) = view {
        let c = [art.white[0], art.white[1], art.white[2], 1.0];
        for i in 0..4 {
            let (a, e) = (v[i], v[(i + 1) % 4]);
            lines.push((pt(a.x / mw, a.y / mh), pt(e.x / mw, e.y / mh), c));
        }
    }
    Some(MinimapDraw { quad, lines, has_terrain: art.image.is_some() })
}
