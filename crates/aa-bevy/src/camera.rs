//! RTS camera: pan (arrows/WASD/screen edges), zoom (wheel, Page Up/Down),
//! rotate (Q/E), tilt (R/F), Home returns to the HQ.
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

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

pub fn update(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ground: Res<Ground>,
    mut cameras: Query<(&mut RtsCamera, &mut Transform)>,
) {
    let dt = time.delta_secs().min(0.1);
    let scroll: f32 = wheel.read().map(|e| e.y.signum()).sum();
    let cursor = windows.single().ok().and_then(|w| w.cursor_position().map(|c| (c, w.size())));
    for (mut cam, mut transform) in &mut cameras {
        let mut pan = Vec2::ZERO;
        let held = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
        if held(KeyCode::ArrowLeft, KeyCode::KeyA) { pan.x -= 1.0; }
        if held(KeyCode::ArrowRight, KeyCode::KeyD) { pan.x += 1.0; }
        if held(KeyCode::ArrowUp, KeyCode::KeyW) { pan.y += 1.0; }
        if held(KeyCode::ArrowDown, KeyCode::KeyS) { pan.y -= 1.0; }
        if let Some((c, size)) = cursor {
            let edge = 6.0;
            if c.x <= edge { pan.x -= 1.0; }
            if c.x >= size.x - edge { pan.x += 1.0; }
            if c.y <= edge { pan.y += 1.0; }
            if c.y >= size.y - edge { pan.y -= 1.0; }
        }
        if keys.pressed(KeyCode::KeyQ) { cam.yaw += 1.6 * dt; }
        if keys.pressed(KeyCode::KeyE) { cam.yaw -= 1.6 * dt; }
        if keys.pressed(KeyCode::KeyR) { cam.pitch = (cam.pitch + 0.8 * dt).min(1.45); }
        if keys.pressed(KeyCode::KeyF) { cam.pitch = (cam.pitch - 0.8 * dt).max(0.35); }
        let mut zoom = -scroll * 0.12;
        if keys.pressed(KeyCode::PageUp) { zoom -= 1.2 * dt; }
        if keys.pressed(KeyCode::PageDown) { zoom += 1.2 * dt; }
        cam.distance = (cam.distance * (1.0 + zoom)).clamp(12.0, 260.0);
        if keys.just_pressed(KeyCode::Home) { cam.focus = cam.home; }
        if pan != Vec2::ZERO {
            // Screen right and screen-up projected on the ground.
            let forward = Vec2::new(-cam.yaw.sin(), -cam.yaw.cos());
            let right = Vec2::new(cam.yaw.cos(), -cam.yaw.sin());
            let step = (right * pan.x + forward * pan.y).normalize() * cam.distance * 0.9 * dt;
            cam.focus.x = (cam.focus.x + step.x).clamp(cam.min.x, cam.max.x);
            cam.focus.z = (cam.focus.z + step.y).clamp(cam.min.y, cam.max.y);
        }
        let target = (ground.0)(cam.focus.x, cam.focus.z);
        cam.focus.y += (target - cam.focus.y) * (1.0 - (-8.0 * dt).exp());
        let offset = Vec3::new(
            cam.yaw.sin() * cam.pitch.cos(),
            cam.pitch.sin(),
            cam.yaw.cos() * cam.pitch.cos(),
        ) * cam.distance;
        *transform = Transform::from_translation(cam.focus + offset).looking_at(cam.focus, Vec3::Y);
    }
}
