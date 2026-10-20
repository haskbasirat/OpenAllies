//! The original gameplay camera (AA.exe, ported in aa-runtime
//! native_camera*.rs; behaviour-faithful f32 here):
//! * fixed yaw -45 deg (CameraYaw, forced while UVar CameraRotate is false,
//!   0x6FEA20) and pitch 40 deg (CameraPitch, pinned), FOV 15 deg on a
//!   frustum whose aspect stays 4:3 (Camera+0x230 = 0.75, 0x6FDF80), near
//!   1, far 512;
//! * eye = target - d * zoom * CameraHeight / sin(pitch) with the view
//!   direction d = (sin yaw cos pitch, cos yaw cos pitch, -sin pitch)
//!   (0x6FF220 / zoom 0x6FE470); the target follows the ground height
//!   (0x6FEDB0); zoom 0.7..2.1, default 1.0 (CameraZoomGameplay*);
//! * scrolling with the arrow keys and the outermost screen pixel
//!   (0x5CCC90), accelerating from CameraScrollSpeed by
//!   CameraScrollAcceleration per second up to CameraScrollSpeedMax, scaled
//!   by CameraScalePan (0x5CD320 / 0x5CCC20 / 0x700500), the target held
//!   CameraScrollEdgePadding / 2 inside the map (0x6FEDB0);
//! * mouse wheel: one notch = 0.1 zoom (0x5EFCCC / 0x43BC80); Page Up/Down
//!   zoom by CameraScaleZoom (the current zoom) * CameraScrollSpeed per
//!   second (0x5C29B0); Home / End = closest / farthest zoom.
//!
//! Native coordinates (X/Y ground, Z up) map to render as (x, z, -y)
//! (`battle_view::to_render`).
use crate::session::Battle;
use bevy::camera::CameraProjection;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// The scene's camera marker (spawned by the battle scene). Kept for the
/// scene's spawn code; the original camera state is [`NativeCamera`].
#[derive(Component)]
pub struct RtsCamera {
    pub focus: Vec3,
    pub home: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    /// Map extent in render X/Z.
    pub min: Vec2,
    pub max: Vec2,
}

/// Ground height under a render-space X/Z point.
#[derive(Resource)]
pub struct Ground(pub Box<dyn Fn(f32, f32) -> f32 + Send + Sync>);

/// AVar values (fast table 0xB39990, hard-coded flag 2 entries).
pub const FOV_DEG: f32 = 15.0;
pub const NEAR: f32 = 1.0;
pub const FAR: f32 = 512.0;
pub const YAW_DEG: f32 = -45.0;
pub const PITCH_DEG: f32 = 40.0;
pub const HEIGHT: f32 = 100.0;
pub const ZOOM_MIN: f32 = 0.7;
/// The v12/v14 runtime's patched maximum (the retail image has 1.4).
pub const ZOOM_MAX: f32 = 2.1;
pub const ZOOM_DEFAULT: f32 = 1.0;

/// The original camera state, in native coordinates.
#[derive(Component, Clone, Debug)]
pub struct NativeCamera {
    pub target: Vec3,
    pub zoom: f32,
    /// UVar CameraScaleZoom: 0x7005F0 writes the current zoom on every zoom.
    pub scale_zoom: f32,
    /// Scroll accumulator (µs) of 0x5CD320; reset when scrolling starts.
    scroll_acc: f32,
    scrolling: bool,
    /// Map size in metres (camera clamp).
    pub map: Vec2,
    placed: bool,
}

impl NativeCamera {
    pub fn view_dir() -> Vec3 {
        let (y, p) = (YAW_DEG.to_radians(), PITCH_DEG.to_radians());
        Vec3::new(y.sin() * p.cos(), y.cos() * p.cos(), -p.sin())
    }
    pub fn eye(&self) -> Vec3 {
        let base = HEIGHT / PITCH_DEG.to_radians().sin();
        self.target - Self::view_dir() * (self.zoom * base)
    }
    /// LookAt (0x6FF130) through the map clamp (0x6FEDB0).
    pub fn look_at(&mut self, p: Vec2, pad: f32) {
        let half = pad * 0.5;
        let x = if self.map.x > pad { p.x.clamp(half, self.map.x - half) } else { p.x };
        let y = if self.map.y > pad { p.y.clamp(half, self.map.y - half) } else { p.y };
        self.target.x = x;
        self.target.y = y;
    }
}

/// The original frustum: horizontal FOV 15 deg, vertical extent
/// tan(FOV/2) * 0.75 (aspect fixed at 4:3, not adapted to the window).
#[derive(Clone, Debug)]
pub struct NativeProjection(PerspectiveProjection);

impl NativeProjection {
    pub fn new() -> Self {
        let h = (FOV_DEG.to_radians() * 0.5).tan();
        let fov_v = 2.0 * (h * 0.75).atan();
        Self(PerspectiveProjection { fov: fov_v, aspect_ratio: 4.0 / 3.0, near: NEAR, far: FAR, ..default() })
    }
}

impl CameraProjection for NativeProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        self.0.get_clip_from_view()
    }
    fn get_clip_from_view_for_sub(&self, sub_view: &bevy::camera::SubCameraView) -> Mat4 {
        self.0.get_clip_from_view_for_sub(sub_view)
    }
    fn update(&mut self, _width: f32, _height: f32) {
        // Camera+0x230 stays 0.75: the 4:3 frustum is stretched to the window
    }
    fn far(&self) -> f32 {
        self.0.far
    }
    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [bevy::math::Vec3A; 8] {
        self.0.get_frustum_corners(z_near, z_far)
    }
}

fn to_render(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

fn from_render(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.z, p.y)
}

/// Requests from the HUD / input for the camera this frame.
#[derive(Resource, Default)]
pub struct CameraRequests {
    /// Jump to a native ground point (minimap, group double tap).
    pub look_at: Option<Vec2>,
    /// Options dialog zoom (Effect::CameraZoom).
    pub zoom: Option<f32>,
    /// Pan/zoom blocked (drag box or facing drag active: camera byte +0x214).
    pub blocked: bool,
}

/// Turn the scene's camera into the original camera on its first frame.
pub fn attach(
    mut commands: Commands,
    cams: Query<(Entity, &RtsCamera), Without<NativeCamera>>,
    battle: Option<Res<Battle>>,
) {
    for (e, rts) in &cams {
        let map = battle
            .as_ref()
            .map(|b| Vec2::new(b.world.map.width as f32 * b.world.map.cell_size, b.world.map.height as f32 * b.world.map.cell_size))
            .unwrap_or_else(|| Vec2::new(rts.max.x - rts.min.x, rts.max.y - rts.min.y).abs());
        let target = from_render(rts.home);
        commands.entity(e).insert((
            NativeCamera { target, zoom: ZOOM_DEFAULT, scale_zoom: 0.5, scroll_acc: 0.0, scrolling: false, map, placed: false },
            Projection::custom(NativeProjection::new()),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ground: Option<Res<Ground>>,
    battle: Option<Res<Battle>>,
    front: Option<Res<crate::ui::front::Front>>,
    mut requests: ResMut<CameraRequests>,
    mut cameras: Query<(&mut NativeCamera, &mut Transform)>,
) {
    // UVars (Data/UVars.tgi, options-dialog overrides)
    let uv = |name: &str, default: f32| front.as_ref().map(|f| f.ui.uvars.get_f32(name)).filter(|v| v.is_finite() && *v != 0.0).unwrap_or(default);
    let speed = uv("CameraScrollSpeed", 2.0);
    let accel = front.as_ref().map(|f| f.ui.uvars.get_f32("CameraScrollAcceleration")).unwrap_or(4.0);
    let speed_max = uv("CameraScrollSpeedMax", 10.0);
    let pad = front.as_ref().map(|f| f.ui.uvars.get_f32("CameraScrollEdgePadding")).unwrap_or(10.0);
    let scale_pan = uv("CameraScalePan", 10.0);
    // the UI takes the keyboard while an edit box has focus or a dialog is up
    let ui_busy = front.as_ref().is_some_and(|f| f.ui.message_box.is_some() || f.hud.as_ref().is_some_and(|h| h.menu_open()));
    let dt_us = (time.delta_secs() * 1.0e6).clamp(10_000.0, 100_000.0);
    let ms = (dt_us / 1000.0).trunc();
    let notches: f32 = wheel
        .read()
        .map(|e| match e.unit {
            MouseScrollUnit::Line => e.y,
            MouseScrollUnit::Pixel => e.y / 120.0,
        })
        .sum();
    let window = windows.single().ok();
    let cursor = window.and_then(|w| front.as_ref().and_then(|f| f.pointer.pos).or(w.physical_cursor_position()).map(|c| (c, Vec2::new(w.physical_width() as f32, w.physical_height() as f32))));
    let over_world = front.as_ref().is_none_or(|f| !f.pointer_over_ui);
    let height = |p: Vec2| -> f32 {
        if let Some(b) = &battle {
            return b.world.map.height_at(p);
        }
        ground.as_ref().map(|g| (g.0)(p.x, -p.y)).unwrap_or(0.0)
    };
    for (mut cam, mut transform) in &mut cameras {
        if !cam.placed {
            // the game starts on the local player's HQ (LookAt of the
            // start location)
            if let Some(b) = &battle {
                let hq = b.world.actors.iter().find(|a| a.kingdom == Some(b.local) && !a.removed && b.world.data.def(a.def).class == Some(aa_play::DefClass::Building));
                cam.map = Vec2::new(b.world.map.width as f32 * b.world.map.cell_size, b.world.map.height as f32 * b.world.map.cell_size);
                if let Some(hq) = hq {
                    cam.target = hq.pos;
                }
            }
            cam.placed = true;
        }
        if let Some(p) = requests.look_at.take() {
            cam.look_at(p, pad);
        }
        if let Some(z) = requests.zoom.take() {
            cam.zoom = z.clamp(ZOOM_MIN, ZOOM_MAX);
            cam.scale_zoom = cam.zoom;
        }
        if !requests.blocked && !ui_busy {
            // scroll bits: arrows (+left/+right/+up/+down), edge pixels
            let mut s = Vec2::ZERO;
            if keys.pressed(KeyCode::ArrowLeft) {
                s.x -= 1.0;
            }
            if keys.pressed(KeyCode::ArrowRight) {
                s.x += 1.0;
            }
            if keys.pressed(KeyCode::ArrowUp) {
                s.y += 1.0;
            }
            if keys.pressed(KeyCode::ArrowDown) {
                s.y -= 1.0;
            }
            if let Some((c, size)) = cursor {
                if c.x <= 0.0 {
                    s.x = -1.0;
                }
                if c.x >= size.x - 1.0 {
                    s.x = 1.0;
                }
                if c.y <= 0.0 {
                    s.y = 1.0;
                }
                if c.y >= size.y - 1.0 {
                    s.y = -1.0;
                }
            }
            if s != Vec2::ZERO {
                if !cam.scrolling {
                    cam.scroll_acc = 0.0;
                    cam.scrolling = true;
                }
                cam.scroll_acc += dt_us;
                let v = (speed + accel * cam.scroll_acc * 1.0e-6).min(speed_max);
                let factor = v / speed;
                let k = scale_pan * speed * ms / 1000.0;
                let (sx, sy) = (factor * s.x * k, factor * s.y * k);
                let d = NativeCamera::view_dir();
                let next = Vec2::new(cam.target.x + sx * d.y + sy * d.x, cam.target.y + sy * d.y - sx * d.x);
                cam.look_at(next, pad);
            } else {
                cam.scrolling = false;
            }
            // zoom: wheel over the world, Page Up / Down, Home / End
            let mut zoom = cam.zoom;
            if notches != 0.0 && over_world {
                zoom -= notches * 0.1;
            }
            let rate = ms * cam.scale_zoom * speed / 1000.0;
            if keys.pressed(KeyCode::PageUp) || keys.pressed(KeyCode::Numpad9) {
                zoom -= rate;
            }
            if keys.pressed(KeyCode::PageDown) || keys.pressed(KeyCode::Numpad3) {
                zoom += rate;
            }
            if keys.just_pressed(KeyCode::Home) {
                zoom = ZOOM_MIN;
            }
            if keys.just_pressed(KeyCode::End) {
                zoom = ZOOM_MAX;
            }
            let zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
            if zoom != cam.zoom && std::env::var_os("AA_UI_CAMLOG").is_some() {
                info!("camera zoom {} -> {zoom} (wheel {notches}, pgup {}, pgdn {})", cam.zoom, keys.pressed(KeyCode::PageUp), keys.pressed(KeyCode::PageDown));
            }
            if zoom != cam.zoom {
                cam.zoom = zoom;
                cam.scale_zoom = zoom;
            }
        } else {
            cam.scrolling = false;
        }
        // the target follows the ground (ground-tracking byte +0x33C)
        cam.target.z = height(cam.target.truncate());
        let eye = cam.eye();
        if std::env::var_os("AA_UI_CAMLOG").is_some() && (time.elapsed_secs() * 2.0).fract() < time.delta_secs() * 2.0 {
            info!("camera target {:?} zoom {} eye {:?}", cam.target, cam.zoom, eye);
        }
        *transform = Transform::from_translation(to_render(eye)).looking_at(to_render(cam.target), Vec3::Y);
    }
}
