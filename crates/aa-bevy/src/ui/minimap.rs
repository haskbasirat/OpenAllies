//! The game minimap (KKC_RMinimap) drawn into the GameMiniMap widget's
//! quad (aa-ui `minimap::quad`, the world square as a diamond):
//! * the terrain: every cell in its terrain variation's `terrain_color`
//!   (Data/Terrain/**/*.tgi: "What color should this terrain be in the
//!   minimap"), from the battle map's cell variations;
//! * the objects: a point per kingdom actor in its kingdom colour
//!   (Data/MiniMap/minimap.tgi `object_point_size` 2, scaled with the
//!   screen; buildings drawn larger), the selection in
//!   `color_owned_selected` white.
//!
//! Not drawn yet (missing): the buildings' `minimap_icon` images, fog of
//! war, the supply rings.
use super::datafile;
use crate::session::Battle;
use aa_play::DefClass;
use bevy::prelude::*;
use std::collections::HashMap;

/// What the front end hands the painter for the minimap.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MinimapDraw {
    /// The widget quad P0, P1, P2 (window pixels).
    pub quad: [(f32, f32); 3],
    /// Points: (u, v) map fractions, size px, colour.
    pub points: Vec<(f32, f32, f32, [f32; 4])>,
    /// The terrain image is ready.
    pub has_terrain: bool,
}

#[derive(Resource, Default)]
pub struct MinimapArt {
    /// Variation ids (lower case) -> terrain colour.
    colors: Option<HashMap<String, [f32; 3]>>,
    pub image: Option<Handle<Image>>,
    built_for: Option<(String, u32, u32)>,
    /// Minimap TGI values.
    pub point_size: f32,
    pub selected: [f32; 3],
}

fn point3(v: &str) -> Option<[f32; 3]> {
    let p: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    (p.len() >= 3).then(|| [p[0], p[1], p[2]])
}

impl MinimapArt {
    fn load(&mut self, data: &aa_ui::fs::DataRoot) {
        let mut colors = HashMap::new();
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
        self.colors = Some(colors);
        let mm = datafile::load(data, "MiniMap/minimap.tgi");
        let node = mm.iter().find(|n| n.name.eq_ignore_ascii_case("Minimap"));
        self.point_size = node.and_then(|n| n.float("object_point_size")).unwrap_or(2.0);
        self.selected = node.and_then(|n| n.get("color_owned_selected")).and_then(point3).unwrap_or([1.0; 3]);
    }

    /// Build (once per map) the terrain image: one texel per cell.
    pub fn terrain(&mut self, data: &aa_ui::fs::DataRoot, b: &Battle, images: &mut Assets<Image>) {
        if self.colors.is_none() {
            self.load(data);
        }
        let m = &b.world.map;
        let key = (m.id.clone(), m.width, m.height);
        if self.built_for.as_ref() == Some(&key) || m.width == 0 || m.height == 0 {
            return;
        }
        self.built_for = Some(key);
        let colors = self.colors.as_ref().unwrap();
        let (w, h) = (m.width as usize, m.height as usize);
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
        let mut img = super::tex::make_image(w as u32, h as u32, px, super::tex::Variant::default(), false);
        img.sampler = bevy::image::ImageSampler::linear();
        self.image = Some(images.add(img));
    }
}

/// The minimap content for this frame.
pub fn draw(ui: &aa_ui::ui::Ui, art: &MinimapArt, b: &Battle, rect: aa_ui::layout::Rect) -> Option<MinimapDraw> {
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
    let scale = ui.screen.height as f32 / ui.reference.height.max(1.0);
    let base = art.point_size.max(1.0) * scale;
    let selected: std::collections::HashSet<u32> = b
        .selection
        .iter()
        .flat_map(|u| match *u {
            aa_play::Unit::Org(o) => b.world.orgs.get(o as usize).map(|o| o.members.clone()).unwrap_or_default(),
            aa_play::Unit::Actor(a) => vec![a],
        })
        .collect();
    let mut points = Vec::new();
    for a in b.world.actors.iter().filter(|a| a.alive && !a.removed) {
        // Independent kingdoms (cities, farms) are neutral: not drawn.
        let Some(k) = a.kingdom.and_then(|k| b.world.kingdoms.iter().find(|x| x.id == k && !x.independent)) else { continue };
        let building = b.world.data.def(a.def).class == Some(DefClass::Building);
        let c = if selected.contains(&a.id) {
            [art.selected[0], art.selected[1], art.selected[2], 1.0]
        } else {
            [k.color[0] as f32 / 255.0, k.color[1] as f32 / 255.0, k.color[2] as f32 / 255.0, 1.0]
        };
        let size = if building { base * 2.0 } else { base };
        points.push((a.pos.x / mw, a.pos.y / mh, size, c));
    }
    Some(MinimapDraw { quad, points, has_terrain: art.image.is_some() })
}
