//! Selection feedback in the battle, from the original art and rules
//! (VWorld selection VFX 0x458ED0, Data/Effects/game_vfx.tgi):
//! * a selected object with a Body shows the status-bar VFX
//!   (Data/Effects/game_vfx.tgi VFXStatusBar models: a billboard whose
//!   NiFlipController frames encode the fill, at the NIF's own size)
//!   instead of a selection decal: BodyComponent `health_bar_vfx_IDS`, NULL
//!   = AVar VFXHealthBar `health_bar_vfx` (HealthBar.NIF), buildings
//!   `health_bar_medium_vfx` (template_building_aa.tgi: HealthBarMedium.NIF);
//!   a selected regiment shows it under every member; a supply relay adds
//!   VFXSupplyStockpileBar (SupplyStockpileBar.nif, SUPPLY_STOCKPILE);
//! * the object under the cursor shows its `focused_vfx`: FocusedSquare
//!   (buildings) or FocusedCircle, a projected decal on the ground in the
//!   diplomatic colour (0x454DC0: self green, friend blue, neutral grey,
//!   enemy red);
//! * the drag box (0x5CD850 / 0x5CCA80): a flat screen rectangle of vertex
//!   colour (0, 0, 0.5) at 30 % alpha under the HUD;
//! * the mouse cursor: the original animated cursor frames
//!   (Data/Cursors/cursors_aa.tgi, 32 x 32 TGA frames), `primary`,
//!   `scroll_*` at the screen edges, `multiple_select` while dragging,
//!   the armed order's cursor (`attack`, `unpack`).
//!
//! Behaviour-faithful: the decal is a flat quad at the ground under the
//! object, not a terrain projection; the bar's placement (at the object's
//! front edge toward the camera) is read from the original screenshots.
use super::nifart;
use super::render::UI_LAYER;
use super::tex::{Key, Variant};
use crate::battle_view::{ActorView, to_render};
use crate::camera::NativeCamera;
use crate::input::Selecting;
use crate::session::Battle;
use aa_play::Unit;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};
use std::collections::HashMap;

#[derive(Component)]
pub struct Indicator;

#[derive(Component)]
pub struct CursorSprite;

/// One status-bar VFX model: its fill frames (full to empty) and size.
#[derive(Default)]
struct Bar {
    frames: Vec<Handle<StandardMaterial>>,
    size: Vec2,
}

/// The original selection art as Bevy materials.
#[derive(Resource, Default)]
pub struct SelectionArt {
    loaded: bool,
    quad: Handle<Mesh>,
    health: Bar,
    health_medium: Bar,
    supply: Bar,
    /// Focused decals per relation (0 self, 1 friend, 2 neutral, 3 enemy).
    focused_square: Vec<Handle<StandardMaterial>>,
    focused_circle: Vec<Handle<StandardMaterial>>,
    pool: Vec<Entity>,
}

pub(super) fn material(images: &mut Assets<Image>, materials: &mut Assets<StandardMaterial>, rgba: nifart::Rgba, tint: Color) -> Handle<StandardMaterial> {
    let (w, h, data) = rgba;
    let mut img = super::tex::make_image(w, h, data, Variant::default(), false);
    img.sampler = bevy::image::ImageSampler::linear();
    materials.add(StandardMaterial {
        base_color: tint,
        base_color_texture: Some(images.add(img)),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        depth_bias: 100.0,
        ..default()
    })
}

fn load_png(path: &std::path::Path) -> Option<nifart::Rgba> {
    let img = image::open(path).ok()?.to_rgba8();
    Some((img.width(), img.height(), img.into_raw()))
}

fn load_art(art: &mut SelectionArt, front: &super::front::Front, images: &mut Assets<Image>, materials: &mut Assets<StandardMaterial>, meshes: &mut Assets<Mesh>) {
    art.loaded = true;
    art.quad = meshes.add(Rectangle::new(1.0, 1.0));
    let data = &front.ui.store.root;
    let dump = std::env::var_os("AA_UI_DUMP_NIF");
    for (file, out) in [("Effects/HealthBar/HealthBar.NIF", 0), ("Effects/HealthBar/HealthBarMedium.NIF", 1), ("Effects/SupplyStockpileBar/SupplyStockpileBar.nif", 2)] {
        let bytes = data.read(file).unwrap_or_default();
        let frames = nifart::flip_frames(&bytes);
        if frames.is_empty() {
            warn!("selection: no frames in {file}");
        }
        let size = nifart::extent(&bytes).map(|(w, h)| Vec2::new(w, h)).unwrap_or(Vec2::new(1.12, 0.16));
        debug!("selection: {file} {} frames, {size}", frames.len());
        let bar = match out {
            0 => &mut art.health,
            1 => &mut art.health_medium,
            _ => &mut art.supply,
        };
        bar.size = size;
        for (i, f) in frames.into_iter().enumerate() {
            if let Some(dir) = &dump {
                if let Some(img) = image::RgbaImage::from_raw(f.0, f.1, f.2.clone()) {
                    let _ = img.save(std::path::Path::new(dir).join(format!("{out}-{i:02}.png")));
                }
            }
            let m = material(images, materials, f, Color::WHITE);
            let bar = match out {
                0 => &mut art.health,
                1 => &mut art.health_medium,
                _ => &mut art.supply,
            };
            bar.frames.push(m);
        }
    }
    // the converted DDS decals (reconstruction/assets/converted/textures)
    let conv = front.root.join("assets/converted/textures/Effects/Selection");
    // tinted with the diplomatic colour (0x4590DF -> 0x454DC0)
    for c in super::banners::relation_colors() {
        let tint = Color::srgb(c[0], c[1], c[2]);
        if let Some(r) = load_png(&conv.join("FocusedSquare.png")) {
            art.focused_square.push(material(images, materials, r, tint));
        }
        if let Some(r) = load_png(&conv.join("FocusedCircle.png")) {
            art.focused_circle.push(material(images, materials, r, tint));
        }
    }
}

/// World-space bounds (render coordinates) of an actor's model parts.
fn bounds(children: Option<&Children>, parts: &Query<(&Aabb, &GlobalTransform)>) -> Option<(Vec3, Vec3)> {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    let mut any = false;
    for c in children.into_iter().flatten() {
        let Ok((aabb, tf)) = parts.get(*c) else { continue };
        let (c0, e) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    let p = tf.transform_point(c0 + e * Vec3::new(sx, sy, sz));
                    lo = lo.min(p);
                    hi = hi.max(p);
                    any = true;
                }
            }
        }
    }
    any.then_some((lo, hi))
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    mut art: ResMut<SelectionArt>,
    front: Res<super::front::Front>,
    battle: Option<Res<Battle>>,
    sel: Res<Selecting>,
    cameras: Query<&GlobalTransform, With<NativeCamera>>,
    views: Query<(&ActorView, &GlobalTransform, Option<&Children>)>,
    parts: Query<(&Aabb, &GlobalTransform)>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut indicators: Query<(&mut Transform, &mut MeshMaterial3d<StandardMaterial>, &mut Visibility), With<Indicator>>,
) {
    let Some(b) = battle else { return };
    if !art.loaded {
        load_art(&mut art, &front, &mut images, &mut materials, &mut meshes);
    }
    let Ok(cam) = cameras.single() else { return };
    let cam_rot = cam.compute_transform().rotation;
    // toward the camera on the ground (render X/Z)
    let back = { let f = cam.forward(); Vec3::new(-f.x, 0.0, -f.z).normalize_or_zero() };
    let by_id: HashMap<u32, (Vec3, Option<&Children>)> = views.iter().map(|(v, tf, c)| (v.id, (tf.translation(), c))).collect();
    let w = &b.world;
    // (material, transform) of every indicator this frame
    let mut want: Vec<(Handle<StandardMaterial>, Transform)> = Vec::new();
    let bar = |bar: &Bar, fraction: f32| -> Option<(Handle<StandardMaterial>, Vec3)> {
        let frames = &bar.frames;
        if frames.is_empty() {
            return None;
        }
        // health01 = full (frames run from full to empty)
        let i = ((1.0 - fraction.clamp(0.0, 1.0)) * (frames.len() - 1) as f32).round() as usize;
        frames.get(i.min(frames.len() - 1)).cloned().map(|m| (m, bar.size.extend(1.0)))
    };
    let add_bars = |id: u32, want: &mut Vec<(Handle<StandardMaterial>, Transform)>| {
        let Some(a) = w.actor(id).filter(|a| a.alive) else { return };
        let Some((pos, children)) = by_id.get(&id) else { return };
        let def = w.data.def(a.def);
        let radius = bounds(*children, &parts).map(|(lo, hi)| ((hi - lo) * Vec3::new(0.5, 0.0, 0.5)).length()).unwrap_or(0.5);
        let base = *pos + back * radius + Vec3::Y * 0.05;
        let health = if def.health > 0.0 { a.health / def.health } else { 1.0 };
        let building = def.class == Some(aa_play::DefClass::Building);
        let up = cam_rot * Vec3::Y;
        let mut below = 0.0;
        if let Some((m, s)) = bar(if building { &art.health_medium } else { &art.health }, health) {
            want.push((m, Transform::from_translation(base).with_rotation(cam_rot).with_scale(s)));
            below = s.y;
        }
        // a supply relay (HQSupplyComponent is_relay, stockpile_max 90;
        // depots 60) adds its stockpile bar under the health bar (the cyan
        // bar under the HQ in the original)
        if def.supply_relay && def.supply_stockpile_max > 0.0 {
            let stock = w.books.actor(id).map_or(0.0, |s| s.supply_stock);
            if let Some((m, s)) = bar(&art.supply, stock / def.supply_stockpile_max) {
                want.push((m, Transform::from_translation(base - up * (below + s.y) * 0.5).with_rotation(cam_rot).with_scale(s)));
            }
        }
    };
    for u in &b.selection {
        match *u {
            Unit::Org(o) => {
                if let Some(org) = w.orgs.get(o as usize) {
                    for m in &org.members {
                        add_bars(*m, &mut want);
                    }
                }
            }
            Unit::Actor(a) => add_bars(a, &mut want),
        }
    }
    // the hovered object's focused decal
    if let Some(id) = sel.hover.filter(|_| !sel.hover_banner) {
        if let (Some(a), Some((pos, children))) = (w.actor(id), by_id.get(&id)) {
            let building = w.data.def(a.def).class == Some(aa_play::DefClass::Building);
            let relation = aa_play::setup::relation(w, Some(b.local), a.kingdom) as usize;
            let m = if building { art.focused_square.get(relation.min(3)).cloned() } else { art.focused_circle.get(relation.min(3)).cloned() };
            if let Some(m) = m {
                let (size, y) = match bounds(*children, &parts) {
                    Some((lo, hi)) => ((hi.x - lo.x).max(hi.z - lo.z).max(1.0) * 1.25, lo.y),
                    None => (1.5, pos.y),
                };
                let ground = to_render(w.ground(a.pos.truncate())).y.max(y.min(pos.y));
                let rot = Quat::from_rotation_y(crate::battle_view::render_yaw(a.facing)) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
                want.push((m, Transform::from_translation(Vec3::new(pos.x, ground + 0.08, pos.z)).with_rotation(rot).with_scale(Vec3::splat(size))));
            }
        }
    }
    // pooled entities
    while art.pool.len() < want.len() {
        let e = commands
            .spawn((
                Mesh3d(art.quad.clone()),
                MeshMaterial3d(art.health.frames.first().cloned().or(art.focused_circle.first().cloned()).unwrap_or_default()),
                Transform::default(),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
                Indicator,
            ))
            .id();
        art.pool.push(e);
    }
    for (i, e) in art.pool.iter().enumerate() {
        let Ok((mut tf, mut mat, mut vis)) = indicators.get_mut(*e) else { continue };
        match want.get(i) {
            Some((m, t)) => {
                *tf = *t;
                if mat.0 != *m {
                    mat.0 = m.clone();
                }
                *vis = Visibility::Visible;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

/// One cursor definition (cursors_aa.tgi).
struct CursorDef {
    frames: Vec<String>,
    loop_ms: u64,
    hotspot: Vec2,
}

#[derive(Resource, Default)]
pub struct Cursors {
    defs: HashMap<String, CursorDef>,
    loaded: bool,
    entity: Option<Entity>,
    materials: HashMap<String, Handle<ColorMaterial>>,
    started: Option<std::time::Instant>,
}

impl Cursors {
    fn load(&mut self, front: &super::front::Front) {
        self.loaded = true;
        let data = &front.ui.store.root;
        for n in super::datafile::load(data, "Cursors/cursors_aa.tgi").iter().filter(|n| n.name.eq_ignore_ascii_case("Cursor")) {
            let ids = n.get("IDS").unwrap_or_default().to_ascii_lowercase();
            let pattern = n.get("frame_filename").unwrap_or_default().to_string();
            let mut frames = Vec::new();
            for i in 0..64 {
                let f = format!("Cursors/{}", pattern.replace("%d", &i.to_string()));
                if !data.exists(&f) {
                    break;
                }
                frames.push(format!("/{f}"));
            }
            let hs: Vec<f32> = n.get("hotspot_offset").unwrap_or("0,0").split(',').filter_map(|x| x.trim().parse().ok()).collect();
            if !frames.is_empty() {
                self.defs.insert(
                    ids,
                    CursorDef { frames, loop_ms: n.get("loop_length_ms").and_then(|v| v.trim().parse().ok()).unwrap_or(1000), hotspot: Vec2::new(hs.first().copied().unwrap_or(0.0), hs.get(1).copied().unwrap_or(0.0)) },
                );
            }
        }
    }
}

/// Which cursor the current UI state shows (the state's vt+0x1C).
fn cursor_ids(front: &super::front::Front, sel: Option<&Selecting>, edge: Option<&str>) -> String {
    if front.page == super::front::Page::Game && !front.pointer_over_ui {
        if let Some(s) = sel {
            if s.drag.is_some() {
                return "multiple_select".into();
            }
            match &s.armed {
                Some(crate::ui::hud::TargetOrder::Attack) => return "attack".into(),
                Some(crate::ui::hud::TargetOrder::Deploy(_)) => return "unpack".into(),
                None => {}
            }
        }
    }
    if let Some(e) = edge {
        return e.to_string();
    }
    "primary".into()
}

#[allow(clippy::too_many_arguments)]
pub fn cursor(
    mut commands: Commands,
    mut cursors: ResMut<Cursors>,
    mut renderer: ResMut<super::render::UiRenderer>,
    front: Res<super::front::Front>,
    sel: Option<Res<Selecting>>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sprite: Query<(&mut Transform, &mut MeshMaterial2d<ColorMaterial>, &mut Visibility), With<CursorSprite>>,
) {
    if !cursors.loaded {
        cursors.load(&front);
    }
    let Ok((window, mut opts)) = windows.single_mut() else { return };
    let started = *cursors.started.get_or_insert_with(std::time::Instant::now);
    let Some(p) = front.pointer.pos.or(window.physical_cursor_position()) else {
        if let Some(e) = cursors.entity {
            if let Ok((_, _, mut v)) = sprite.get_mut(e) {
                *v = Visibility::Hidden;
            }
        }
        return;
    };
    let (w, h) = (window.physical_width() as f32, window.physical_height() as f32);
    // the edge scroller's cursor (0x5CD480) in the battle
    let edge = (front.page == super::front::Page::Game).then(|| {
        let (l, r, t, b) = (p.x <= 0.0, p.x >= w - 1.0, p.y <= 0.0, p.y >= h - 1.0);
        match (l, r, t, b) {
            (true, _, true, _) => Some("scroll_upleft"),
            (_, true, true, _) => Some("scroll_upright"),
            (true, _, _, true) => Some("scroll_downleft"),
            (_, true, _, true) => Some("scroll_downright"),
            (true, ..) => Some("scroll_left"),
            (_, true, ..) => Some("scroll_right"),
            (_, _, true, _) => Some("scroll_up"),
            (.., true) => Some("scroll_down"),
            _ => None,
        }
    });
    let ids = cursor_ids(&front, sel.as_deref(), edge.flatten());
    let Some(def) = cursors.defs.get(&ids).or(cursors.defs.get("primary")) else { return };
    let frame = ((started.elapsed().as_millis() as u64 % def.loop_ms.max(1)) * def.frames.len() as u64 / def.loop_ms.max(1)) as usize;
    let path = def.frames[frame.min(def.frames.len() - 1)].clone();
    let hot = def.hotspot;
    let key = Key { path: path.clone(), skin: None, variant: Variant::default() };
    renderer.tex.prepare([key.clone()], &mut images);
    let Some(entry) = renderer.tex.get(&key).cloned() else { return };
    opts.visible = false;
    let mat = cursors
        .materials
        .entry(path)
        .or_insert_with(|| materials.add(ColorMaterial { texture: Some(entry.image.clone()), alpha_mode: bevy::sprite_render::AlphaMode2d::Blend, ..default() }))
        .clone();
    let size = Vec2::new(entry.size[0] as f32, entry.size[1] as f32);
    // top-left at the cursor minus the hotspot, window pixels y down
    let x = p.x - hot.x + size.x * 0.5 - w * 0.5;
    let y = h * 0.5 - (p.y - hot.y + size.y * 0.5);
    let t = Transform::from_xyz(x.floor() + 0.5 * (size.x % 2.0), y.floor(), 999.0).with_scale(Vec3::new(size.x, size.y, 1.0));
    match cursors.entity.and_then(|e| sprite.get_mut(e).ok()) {
        Some((mut tf, mut m, mut v)) => {
            *tf = t;
            if m.0 != mat {
                m.0 = mat;
            }
            *v = Visibility::Visible;
        }
        None => {
            let quad = meshes.add(Rectangle::new(1.0, 1.0));
            let e = commands.spawn((Mesh2d(quad), MeshMaterial2d(mat), t, Visibility::Visible, RenderLayers::layer(UI_LAYER), CursorSprite)).id();
            cursors.entity = Some(e);
        }
    }
}
