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
//!   zoom by CameraScaleZoom (the last requested zoom) * CameraScrollSpeed
//!   per second (0x5C29B0); Home / End = closest / farthest zoom.
//! * the screen edge starts scrolling only while the world view is the
//!   widget under the cursor (0x5CD420 / 0x5EFA80); keys and a running
//!   scroll then also take the edge, and nothing pans while the cursor is
//!   outside the window (0x5CD320 KKC_Input+24);
//! * the battle opens on the local kingdom's start location (0x4F61A0 ->
//!   0x43F0A0).
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

/// The kingdom's start location (native ground point): Kingdom+1F8, the
/// start-location thing of its map slot, whose position 4F61A0
/// (VWorldState::StartMission) hands to the camera.
pub fn start_location(world: &aa_play::World, kingdom: aa_play::KingdomId) -> Option<Vec2> {
    let slot = world.ids.kingdom_slot(kingdom)?;
    let index = world.map.player_slots.iter().position(|&s| s == slot)?;
    world.map.start_locations.get(index).copied()
}

/// The original camera state, in native coordinates.
#[derive(Component, Clone, Debug)]
pub struct NativeCamera {
    pub target: Vec3,
    pub zoom: f32,
    /// UVar CameraScaleZoom: 0x7005F0 stores every requested zoom (before
    /// the clamp) through 0x40E0A0; the zoom keys' rate reads it back.
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

impl NativeCamera {
    /// 0x7005F0 (through 0x7010F0 for a relative zoom): the zoom clamped to
    /// the gameplay range (0x6FE470), the requested value stored to UVar
    /// CameraScaleZoom (0x40E0A0).
    fn set_zoom_7005f0(&mut self, value: f32) {
        if !value.is_finite() {
            return;
        }
        self.zoom = value.clamp(ZOOM_MIN, ZOOM_MAX);
        self.scale_zoom = value;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    logical: Res<ButtonInput<bevy::input::keyboard::Key>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ground: Option<Res<Ground>>,
    battle: Option<Res<Battle>>,
    mut front: Option<ResMut<crate::ui::front::Front>>,
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
    // UVar 34 CameraScaleZoom (UVars.tgi 0.5, or the profile's last zoom)
    let scale_zoom_uvar = front.as_ref().map(|f| f.ui.uvars.get_f32("CameraScaleZoom")).filter(|v| v.is_finite());
    // the UI takes the keyboard while an edit box has focus or a dialog is up
    let ui_busy = front.as_ref().is_some_and(|f| f.ui.message_box.is_some() || f.hud.as_ref().is_some_and(|h| h.menu_open()));
    let dt_us = (time.delta_secs() * 1.0e6).clamp(10_000.0, 100_000.0);
    let ms = (dt_us / 1000.0).trunc();
    // one 0x5EFCCC "deltazoom" command per wheel message (delta / 1200)
    let notches: Vec<f32> = wheel
        .read()
        .map(|e| match e.unit {
            MouseScrollUnit::Line => e.y,
            MouseScrollUnit::Pixel => e.y / 120.0,
        })
        .filter(|n| *n != 0.0)
        .collect();
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
            // the game starts on the local kingdom's start location
            // (4F61A0 -> 43F0A0 LookAt), else on its first building
            if let Some(b) = &battle {
                cam.map = Vec2::new(b.world.map.width as f32 * b.world.map.cell_size, b.world.map.height as f32 * b.world.map.cell_size);
                let hq = b.world.actors.iter().find(|a| a.kingdom == Some(b.local) && !a.removed && b.world.data.def(a.def).class == Some(aa_play::DefClass::Building)).map(|a| a.pos.truncate());
                if let Some(p) = start_location(&b.world, b.local).or(hq) {
                    cam.look_at(p, pad);
                }
            }
            if let Some(s) = scale_zoom_uvar {
                cam.scale_zoom = s;
            }
            cam.placed = true;
        }
        let zoom_before = cam.zoom;
        let mut stored = None;
        if let Some(p) = requests.look_at.take() {
            cam.look_at(p, pad);
        }
        if let Some(z) = requests.zoom.take() {
            cam.set_zoom_7005f0(z);
            stored = Some(cam.scale_zoom);
        }
        // 5C92F0 / 5CD320 run the scroller and the impulses only while the
        // application is active (App+34 bit 1, window activation)
        let active = window.is_none_or(|w| w.focused);
        if !requests.blocked && !ui_busy && active {
            // scroll bits: arrows (+left/+right/+up/+down) ...
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
            // ... and the outermost pixels (5CCC90); an edge only starts a
            // scroll with the world view under the cursor (5CD420/5EFA80),
            // an active scroller keeps taking it (5CD410)
            let edge_counts = over_world || cam.scrolling || s != Vec2::ZERO;
            if let Some((c, size)) = cursor.filter(|_| edge_counts) {
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
                // 5CD320 pans only with the cursor inside the client area
                if cursor.is_some() {
                    cam.scroll_acc += dt_us;
                    let v = (speed + accel * cam.scroll_acc * 1.0e-6).min(speed_max);
                    let factor = v / speed;
                    let k = scale_pan * speed * ms / 1000.0;
                    let (sx, sy) = (factor * s.x * k, factor * s.y * k);
                    let d = NativeCamera::view_dir();
                    let next = Vec2::new(cam.target.x + sx * d.y + sy * d.x, cam.target.y + sy * d.y - sx * d.x);
                    cam.look_at(next, pad);
                }
            } else {
                cam.scrolling = false;
            }
            // zoom: each wheel message over the world (43BC80 -> 7010F0)
            if over_world {
                for n in &notches {
                    let value = cam.zoom - n * 0.1;
                    cam.set_zoom_7005f0(value);
                    stored = Some(cam.scale_zoom);
                }
            }
            // +zoomin / +zoomout impulses: pageup / pagedown, numpad-9 /
            // numpad-3 and their NumLock-off pageup / pagedown (5C29B0:
            // ms * CameraScaleZoom * CameraScrollSpeed * -/+0.001)
            let rate = ms * cam.scale_zoom * speed / 1000.0;
            if keys.pressed(KeyCode::PageUp) || keys.pressed(KeyCode::Numpad9) {
                let value = cam.zoom - rate;
                cam.set_zoom_7005f0(value);
                stored = Some(cam.scale_zoom);
            }
            if keys.pressed(KeyCode::PageDown) || keys.pressed(KeyCode::Numpad3) {
                let value = cam.zoom + rate;
                cam.set_zoom_7005f0(value);
                stored = Some(cam.scale_zoom);
            }
            // ZoomMin / ZoomMax (43BC20 / 43BC50): home / end and the
            // NumLock-off numpad-home / numpad-end (the logical keys)
            use bevy::input::keyboard::Key;
            if keys.just_pressed(KeyCode::Home) || logical.just_pressed(Key::Home) {
                cam.set_zoom_7005f0(ZOOM_MIN);
                stored = Some(cam.scale_zoom);
            }
            if keys.just_pressed(KeyCode::End) || logical.just_pressed(Key::End) {
                cam.set_zoom_7005f0(ZOOM_MAX);
                stored = Some(cam.scale_zoom);
            }
            if cam.zoom != zoom_before && std::env::var_os("AA_UI_CAMLOG").is_some() {
                info!("camera zoom {zoom_before} -> {} (wheel {notches:?}, pgup {}, pgdn {}, scale {})", cam.zoom, keys.pressed(KeyCode::PageUp), keys.pressed(KeyCode::PageDown), cam.scale_zoom);
            }
        } else {
            cam.scrolling = false;
        }
        // 7005F0 -> 40E0A0: the requested zoom becomes UVar CameraScaleZoom
        if let (Some(value), Some(front)) = (stored, front.as_mut()) {
            let uvars = &mut front.ui.uvars;
            if uvars.store_camera_scale_zoom_34(value.to_bits()).is_err() {
                uvars.set_f32("CameraScaleZoom", value);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_requested_zoom_is_clamped_and_stored_unclamped() {
        let mut cam = NativeCamera { target: Vec3::ZERO, zoom: ZOOM_DEFAULT, scale_zoom: 0.5, scroll_acc: 0.0, scrolling: false, map: Vec2::splat(1000.0), placed: true };
        // above the gameplay range: the camera stops, the UVar keeps the request
        cam.set_zoom_7005f0(2.6);
        assert_eq!((cam.zoom, cam.scale_zoom), (ZOOM_MAX, 2.6));
        // 7010F0 is relative to the clamped zoom, not the stored request
        cam.set_zoom_7005f0(cam.zoom - 0.1);
        assert_eq!((cam.zoom, cam.scale_zoom), (ZOOM_MAX - 0.1, ZOOM_MAX - 0.1));
        cam.set_zoom_7005f0(0.1);
        assert_eq!((cam.zoom, cam.scale_zoom), (ZOOM_MIN, 0.1));
        // no native domain for a non-finite request: nothing changes
        cam.set_zoom_7005f0(f32::NAN);
        assert_eq!((cam.zoom, cam.scale_zoom), (ZOOM_MIN, 0.1));
    }
}
