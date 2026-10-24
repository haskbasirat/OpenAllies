//! The game minimap (KKC_RMinimap) drawn into the GameMiniMap widget's
//! quad (aa-ui `minimap::quad`, the world square as a diamond):
//! * the terrain: every cell in its terrain variation's `terrain_color`
//!   (Data/Terrain/**/*.tgi: "What color should this terrain be in the
//!   minimap"), from the battle map's cell variations, darkened by the
//!   local kingdom's fog like the terrain (visible, grey explored, black
//!   unexplored), refreshed every AVar MinimapPeriodUpdateFog seconds;
//! * the objects, refreshed every MinimapPeriodUpdateObjects seconds: the
//!   ones the local kingdom sees (and the static ones it remembers, where it
//!   remembers them) in their kingdom colour, neutral (independent) objects
//!   in `color_neutral_*`, the selection in `color_owned_selected`; a
//!   definition with a `minimap_icon` (Data/MiniMap/Icons) draws the icon
//!   tinted with that colour, map-aligned (it turns with the map), the rest
//!   a point of `object_point_size` with a `color_object_border` edge.
//!   Sizes are minimap target pixels: the target is the next power of two of
//!   `minimap_size` (172 -> 256, `0x712D60`), drawn over the whole quad;
//! * the local kingdom's supply border (its supply zones' outline in
//!   `color_requires_supply`, every MinimapPeriodUpdateSupply seconds);
//! * the camera's view on the ground in `color_white`;
//! * while the local kingdom's enchantments carry MINIMAP_NO_FOG (modifier
//!   flag 8, Radar Scan) the fog layer (`0x433620`) is clear and the objects
//!   layer (`0x435BC0`) draws every object at its live position, not only
//!   the ones the kingdom sees (the next periodic refresh of each layer
//!   picks the change up).
use super::datafile;
use crate::session::Battle;
use aa_play::DefClass;
use aa_play::fog::{self, Sighting};
use bevy::prelude::*;
use std::collections::HashMap;

/// One map-aligned icon: the texture (Data path), its four screen corners
/// (image corners (0,1), (1,1), (1,0), (0,0)) and its tint.
#[derive(Clone, Debug, PartialEq)]
pub struct MinimapIcon {
    pub texture: String,
    pub corners: [(f32, f32); 4],
    pub color: [f32; 4],
}

/// What the front end hands the painter for the minimap.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MinimapDraw {
    /// The widget quad P0, P1, P2 (window pixels).
    pub quad: [(f32, f32); 3],
    /// Points: (u, v) map fractions, size px, colour (borders first).
    pub points: Vec<(f32, f32, f32, [f32; 4])>,
    pub icons: Vec<MinimapIcon>,
    /// Lines (window pixels): from, to, colour (the camera view).
    pub lines: Vec<((f32, f32), (f32, f32), [f32; 4])>,
    /// The terrain image is ready.
    pub has_terrain: bool,
}

#[derive(Resource, Default)]
pub struct MinimapArt {
    /// Variation ids (lower case) -> terrain colour.
    colors: Option<HashMap<String, [f32; 3]>>,
    pub image: Option<Handle<Image>>,
    built_for: Option<(String, u32, u32)>,
    /// The fog-free terrain colours (RGBA, image rows top first).
    base: Vec<u8>,
    /// Fog refresh: battle time of the last one and the fog revision seen.
    fog_at: f32,
    fog_revision: u64,
    fog_local: Option<u32>,
    /// MINIMAP_NO_FOG at the last fog refresh.
    fog_no_fog: bool,
    /// The objects of the last refresh and when it was made.
    objects: Option<(f32, Vec<Object>)>,
    /// Minimap TGI values.
    pub point_size: f32,
    pub selected: [f32; 3],
    pub border: [f32; 3],
    pub neutral_static: [f32; 3],
    pub neutral_mobile: [f32; 3],
    pub min_icon_radius: f32,
    /// AVars MinimapPeriodUpdateFog / MinimapPeriodUpdateObjects (s).
    pub period_fog: f32,
    pub period_objects: f32,
    /// Icon pixel sizes by Data path.
    icon_sizes: HashMap<String, Option<(u32, u32)>>,
    /// minimap.tgi rallypoint_icon (Data path).
    pub rally_icon: String,
    pub white: [f32; 3],
    pub requires_supply: [f32; 3],
    pub period_supply: f32,
    supply_at: f32,
    /// Supply border tiles (index) of the last supply refresh.
    supply_border: Vec<usize>,
}

/// An object as the last object refresh placed it.
#[derive(Clone, Debug)]
struct Object {
    pos: Vec2,
    color: [f32; 3],
    icon: Option<String>,
    selected: bool,
}

fn point3(v: &str) -> Option<[f32; 3]> {
    let p: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    (p.len() >= 3).then(|| [p[0], p[1], p[2]])
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
        self.point_size = node.and_then(|n| n.float("object_point_size")).unwrap_or(2.0);
        // textures/rallypoint.png, relative to Data/MiniMap
        self.rally_icon = node.and_then(|n| n.get("rallypoint_icon")).map(|s| format!("MiniMap/{}", s.trim())).unwrap_or_else(|| "MiniMap/Textures/Rallypoint.PNG".into());
        self.min_icon_radius = node.and_then(|n| n.float("min_icon_radius")).unwrap_or(2.0);
        self.selected = c("color_owned_selected", [1.0; 3]);
        self.border = c("color_object_border", [0.0; 3]);
        self.neutral_static = c("color_neutral_static", [0.729; 3]);
        self.neutral_mobile = c("color_neutral_mobile", [0.729; 3]);
        self.white = c("color_white", [1.0; 3]);
        self.requires_supply = c("color_requires_supply", [0.0, 1.0, 0.0]);
        // AVars.tgi (the hard-coded defaults of 0x401BD0 are 5 / 2 s)
        let avars = datafile::load(data, "AVars.tgi");
        let avar = |name: &str, d: f32| avars.iter().find_map(|n| n.float(name)).unwrap_or(d);
        self.period_fog = avar("MinimapPeriodUpdateFog", 5.0);
        self.period_objects = avar("MinimapPeriodUpdateObjects", 2.0);
        self.period_supply = avar("MinimapPeriodUpdateSupply", 5.0);
    }

    /// Build (once per map) the terrain colours, then darken them by the
    /// local kingdom's fog every MinimapPeriodUpdateFog seconds.
    pub fn terrain(&mut self, data: &aa_ui::fs::DataRoot, b: &Battle, images: &mut Assets<Image>) {
        if self.colors.is_none() {
            let t0 = std::time::Instant::now();
            self.load(data);
            info!("minimap: data loaded in {:.0} ms", t0.elapsed().as_secs_f32() * 1000.0);
        }
        let m = &b.world.map;
        let key = (m.id.clone(), m.width, m.height);
        if m.width == 0 || m.height == 0 {
            return;
        }
        let (w, h) = (m.width as usize, m.height as usize);
        if self.built_for.as_ref() != Some(&key) {
            self.built_for = Some(key);
            let colors = self.colors.as_ref().unwrap();
            let mut px = vec![0u8; w * h * 4];
            let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            for y in 0..h {
                for x in 0..w {
                    let cell = y * w + x;
                    let c = m
                        .cell_variation
                        .get(cell)
                        .and_then(|&v| m.variations.get(v as usize))
                        .and_then(|id| colors.get(&id.to_ascii_lowercase()))
                        .copied()
                        .unwrap_or([0.5, 0.5, 0.5]);
                    // image row 0 = the map's top (world y = H)
                    let i = ((h - 1 - y) * w + x) * 4;
                    px[i..i + 4].copy_from_slice(&[byte(c[0]), byte(c[1]), byte(c[2]), 255]);
                }
            }
            self.base = px.clone();
            let mut img = super::tex::make_image(w as u32, h as u32, px, super::tex::Variant::default(), true);
            img.sampler = bevy::image::ImageSampler::linear();
            self.image = Some(images.add(img));
            self.fog_at = f32::NEG_INFINITY;
            self.supply_at = f32::NEG_INFINITY;
            self.fog_local = None;
            self.objects = None;
        }
        // the supply border: tiles of the local kingdom's supply zones next
        // to a tile outside them
        let now = b.world.time;
        let mut supply_changed = false;
        if now - self.supply_at >= self.period_supply || now < self.supply_at {
            self.supply_at = now;
            let tile = m.cell_size.max(1.0);
            let mut inside = vec![false; w * h];
            for z in b.world.books.zones.iter().filter(|z| z.kingdom == b.local && z.radius > 0.0) {
                let (x0, x1) = (((z.centre.x - z.radius) / tile).floor().max(0.0) as usize, (((z.centre.x + z.radius) / tile).ceil() as usize).min(w));
                let (y0, y1) = (((z.centre.y - z.radius) / tile).floor().max(0.0) as usize, (((z.centre.y + z.radius) / tile).ceil() as usize).min(h));
                for y in y0..y1 {
                    for x in x0..x1 {
                        let c = Vec2::new((x as f32 + 0.5) * tile, (y as f32 + 0.5) * tile);
                        if c.distance_squared(z.centre) <= z.radius * z.radius {
                            inside[y * w + x] = true;
                        }
                    }
                }
            }
            let mut border = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if !inside[y * w + x] {
                        continue;
                    }
                    let out = |dx: i32, dy: i32| {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 || !inside[ny as usize * w + nx as usize]
                    };
                    if out(1, 0) || out(-1, 0) || out(0, 1) || out(0, -1) {
                        border.push(y * w + x);
                    }
                }
            }
            if border != self.supply_border {
                self.supply_border = border;
                supply_changed = true;
            }
        }
        let due = now - self.fog_at >= self.period_fog || now < self.fog_at;
        let no_fog = minimap_no_fog(b);
        let changed = self.fog_revision != b.world.fog.revision || self.fog_local != Some(b.local) || self.fog_no_fog != no_fog;
        if !(due && changed) && self.fog_local.is_some() && !supply_changed {
            return;
        }
        self.fog_at = now;
        self.fog_revision = b.world.fog.revision;
        self.fog_local = Some(b.local);
        self.fog_no_fog = no_fog;
        let mut shade = Vec::new();
        fog::shade(&b.world, Some(b.local), super::fog::gray_value(), &mut shade);
        // 0x433620: the kingdom's MINIMAP_NO_FOG flag shows every cell as
        // visible
        if no_fog {
            shade.iter_mut().for_each(|s| *s = 0);
        }
        let Some(mut img) = self.image.as_ref().and_then(|h| images.get_mut(h)) else { return };
        let Some(px) = img.data.as_mut() else { return };
        if px.len() != self.base.len() || shade.len() != w * h {
            return;
        }
        for y in 0..h {
            for x in 0..w {
                let keep = 255 - u16::from(shade[y * w + x]);
                let i = ((h - 1 - y) * w + x) * 4;
                for c in 0..3 {
                    px[i + c] = ((u16::from(self.base[i + c]) * keep) / 255) as u8;
                }
            }
        }
        let green = self.requires_supply.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        for &t in &self.supply_border {
            let (x, y) = (t % w, t / w);
            let i = ((h - 1 - y) * w + x) * 4;
            px[i..i + 3].copy_from_slice(&green);
        }
    }

    fn icon_size(&mut self, data: &aa_ui::fs::DataRoot, path: &str) -> Option<(u32, u32)> {
        if let Some(s) = self.icon_sizes.get(path) {
            return *s;
        }
        let found = data.find(path.trim_start_matches('/')).and_then(|p| image::image_dimensions(&p).ok());
        self.icon_sizes.insert(path.to_string(), found);
        found
    }

    /// The objects the local kingdom sees or remembers.
    fn refresh_objects(&mut self, b: &Battle) -> Vec<Object> {
        let w = &b.world;
        let selected: std::collections::HashSet<u32> = b
            .selection
            .iter()
            .flat_map(|u| match *u {
                aa_play::Unit::Org(o) => w.orgs.get(o as usize).map(|o| o.members.clone()).unwrap_or_default(),
                aa_play::Unit::Actor(a) => vec![a],
            })
            .collect();
        let mut out = Vec::new();
        // 0x435BC0: MINIMAP_NO_FOG draws every object as if it were seen
        let no_fog = minimap_no_fog(b);
        for a in w.actors.iter().filter(|a| a.alive && !a.removed) {
            let Some(k) = a.kingdom.and_then(|k| w.kingdoms.iter().find(|x| x.id == k)) else { continue };
            let def = w.data.def(a.def);
            if def.invisible {
                continue;
            }
            let sighting = if no_fog { Sighting::Visible } else { fog::sighting(w, a.id, Some(b.local)) };
            let (pos, owner) = match sighting {
                Sighting::Visible => (a.pos.truncate(), Some(k)),
                Sighting::Remembered(m) => (m.pos.truncate(), m.kingdom.and_then(|mk| w.kingdoms.iter().find(|x| x.id == mk))),
                Sighting::Hidden => continue,
            };
            let mobile = def.speed_max > 0.0 || matches!(def.class, Some(DefClass::Member) | Some(DefClass::Truck));
            // neutral objects show only by their icon (the original minimap
            // shows the cities' flags, not their blocks)
            if owner.is_none_or(|o| o.independent) && def.minimap_icon.is_none() {
                continue;
            }
            let color = match owner {
                Some(o) if !o.independent => [o.color[0] as f32 / 255.0, o.color[1] as f32 / 255.0, o.color[2] as f32 / 255.0],
                _ => {
                    if mobile {
                        self.neutral_mobile
                    } else {
                        self.neutral_static
                    }
                }
            };
            out.push(Object { pos, color, icon: def.minimap_icon.clone(), selected: selected.contains(&a.id) });
        }
        // ghosts of remembered actors already gone (none while everything
        // live is shown)
        for (id, m) in fog::memories(w, Some(b.local)).into_iter().filter(|_| !no_fog) {
            if w.actor(id).is_some_and(|a| a.alive) {
                continue;
            }
            let def = w.data.def(m.def);
            let owner = m.kingdom.and_then(|mk| w.kingdoms.iter().find(|x| x.id == mk));
            let color = match owner {
                Some(o) if !o.independent => [o.color[0] as f32 / 255.0, o.color[1] as f32 / 255.0, o.color[2] as f32 / 255.0],
                _ => self.neutral_static,
            };
            out.push(Object { pos: m.pos.truncate(), color, icon: def.minimap_icon.clone(), selected: false });
        }
        out
    }
}

/// The local kingdom's enchantments carry MINIMAP_NO_FOG (Radar Scan;
/// `0x4CF110` flags bit 8).
fn minimap_no_fog(b: &Battle) -> bool {
    aa_play::special_ops::kingdom_has_flag(&b.world, b.local, "MINIMAP_NO_FOG")
}

/// Next power of two of the minimap target size (`0x43A920`).
fn target_size(n: i32) -> f32 {
    (n.max(1) as u32).next_power_of_two() as f32
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
    // the objects move on the minimap every MinimapPeriodUpdateObjects
    let now = b.world.time;
    let stale = art.objects.as_ref().is_none_or(|(t, _)| now - *t >= art.period_objects || now < *t);
    if stale {
        let objects = art.refresh_objects(b);
        art.objects = Some((now, objects));
    }
    let objects = art.objects.as_ref().map(|(_, o)| o.clone()).unwrap_or_default();
    // minimap target pixels -> window pixels along the quad edge
    let [p0, p1, p2] = quad;
    let side = ((p1.0 - p0.0).hypot(p1.1 - p0.1)).max(1.0);
    let target = target_size(m.size.0.max(m.size.1));
    let px = side / target;
    let pt = |u: f32, v: f32| (p0.0 + u * (p2.0 - p1.0) + v * (p1.0 - p0.0), p0.1 + u * (p2.1 - p1.1) + v * (p1.1 - p0.1));
    let mut points = Vec::new();
    let mut icons = Vec::new();
    for o in &objects {
        let (u, v) = (o.pos.x / mw, o.pos.y / mh);
        let color = if o.selected { art.selected } else { o.color };
        let c = [color[0], color[1], color[2], 1.0];
        if let Some(icon) = &o.icon {
            if let Some((iw, ih)) = art.icon_size(&ui.store.root, icon) {
                // map-aligned: the icon's pixels are minimap target pixels
                let (hu, hv) = (iw as f32 * 0.5 / target, ih as f32 * 0.5 / target);
                let corners = [pt(u - hu, v - hv), pt(u + hu, v - hv), pt(u + hu, v + hv), pt(u - hu, v + hv)];
                icons.push(MinimapIcon { texture: icon.clone(), corners, color: c });
                continue;
            }
        }
        let size = (art.point_size.max(1.0) * 2.0 * px).max(1.0);
        let edge = px.max(1.0);
        points.push((u, v, size + 2.0 * edge, [art.border[0], art.border[1], art.border[2], 1.0]));
        points.push((u, v, size, c));
    }
    // the selected building's rally point (minimap.tgi rallypoint_icon)
    if let [aa_play::Unit::Actor(id)] = b.selection.as_slice() {
        if let Some(p) = aa_play::player_orders::rally_point(&b.world, *id).filter(|_| b.world.actor(*id).is_some_and(|a| a.kingdom == Some(b.local))) {
            let icon = art.rally_icon.clone();
            if let Some((iw, ih)) = art.icon_size(&ui.store.root, &icon) {
                let (u, v) = (p.x / mw, p.y / mh);
                let (hu, hv) = (iw as f32 * 0.5 / target, ih as f32 * 0.5 / target);
                icons.push(MinimapIcon { texture: icon, corners: [pt(u - hu, v - hv), pt(u + hu, v - hv), pt(u + hu, v + hv), pt(u - hu, v + hv)], color: [1.0; 4] });
            }
        }
    }
    // borders under every fill
    let (borders, fills): (Vec<_>, Vec<_>) = points.chunks(2).map(|p| (p[0], p[1])).unzip();
    let mut points = borders;
    points.extend(fills);
    let mut lines = Vec::new();
    if let Some(v) = view {
        let c = [art.white[0], art.white[1], art.white[2], 1.0];
        for i in 0..4 {
            let (a, e) = (v[i], v[(i + 1) % 4]);
            lines.push((pt(a.x / mw, a.y / mh), pt(e.x / mw, e.y / mh), c));
        }
    }
    Some(MinimapDraw { quad, points, icons, lines, has_terrain: art.image.is_some() })
}
