//! Fog of war on screen, for the local kingdom (aa-play `fog`):
//! * the terrain and unit fog texture (`look::FOG_IMAGE`) every original
//!   material darkens by: per GMap tile 0 visible, TerrainFogGrayPercentage
//!   (hard-coded AVar 0x40, 0.5) x 255 explored, 255 never explored
//!   (`0x71BD00`), each change fading over TerrainFogFadeTime (AVar 0x41,
//!   0.5 s) from what was shown to the new value (`0x7208D0` timer +0x108,
//!   blend `0x71C290`);
//! * the actors the local kingdom cannot see are hidden, the static ones it
//!   saw stay drawn as it remembers them (detection memory);
//! * the FogOfWar world rule of the staging screen (`PendingFog`).
use crate::battle_view::ActorView;
use crate::look::{FOG_IMAGE, FOG_SIZE, FOG_TEXEL};
use crate::session::Battle;
use aa_play::fog::{self, FogMode, Sighting};
use bevy::prelude::*;

/// Hard-coded AVar TerrainFogGrayPercentage (index 0x40, `0x401BD0`).
pub const TERRAIN_FOG_GRAY_PERCENTAGE: f32 = 0.5;
/// Hard-coded AVar TerrainFogFadeTime (index 0x41), seconds.
pub const TERRAIN_FOG_FADE_TIME: f32 = 0.5;

/// The FogOfWar world rule chosen on the staging screen, applied to the
/// next battle before its first tick.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PendingFog(pub FogMode);

/// The shown fog per tile and its fades.
#[derive(Resource, Default)]
pub struct FogDraw {
    built_for: Option<(String, u32, u32, u32)>,
    revision: u64,
    /// Per tile: the target value, the value the fade starts from, the fade
    /// start (s) and what is shown.
    target: Vec<u8>,
    from: Vec<f32>,
    start: Vec<f32>,
    shown: Vec<u8>,
    fading: bool,
    scratch: Vec<u8>,
    tile: f32,
    size: (u32, u32),
    /// Microseconds the last texture update took (for the cost report).
    pub last_us: f32,
    cost_at: f32,
    cost_frames: u32,
    cost_us: f32,
    cost_max_us: f32,
}

/// The grey fog value of an explored tile (`DAT_00B285D4`, `0x725340`).
pub fn gray_value() -> u8 {
    (TERRAIN_FOG_GRAY_PERCENTAGE * 255.0) as u8
}

/// Battle start: the staging screen's FogOfWar rule, then the fog state
/// before the first frame draws.
pub fn start(mut battle: Option<ResMut<Battle>>, pending: Option<Res<PendingFog>>, mut commands: Commands) {
    let Some(b) = battle.as_mut() else { return };
    if b.is_added() {
        if let Some(p) = pending.as_deref() {
            fog::set_mode(&mut b.world, p.0);
            commands.remove_resource::<PendingFog>();
        }
        // AA_BEVY_FOG=none|explored|full: the rule for test runs started
        // without the staging screen
        if let Some(mode) = std::env::var("AA_BEVY_FOG").ok().and_then(|v| FogMode::from_ids(&v)) {
            fog::set_mode(&mut b.world, mode);
        }
    }
    if b.world.fog.stats.ticks == 0 {
        fog::ensure(&mut b.world);
    }
}

/// Write the local kingdom's fog into the fog texture (fading changes).
pub fn update(battle: Option<Res<Battle>>, mut fd: ResMut<FogDraw>, mut images: ResMut<Assets<Image>>, time: Res<Time>) {
    let now = time.elapsed_secs();
    let Some(b) = battle else {
        if fd.built_for.is_some() {
            // no battle: no fog
            fd.built_for = None;
            if let Some(mut img) = images.get_mut(&FOG_IMAGE) {
                if let Some(d) = img.data.as_mut() {
                    d.fill(0);
                }
            }
        }
        return;
    };
    let started = std::time::Instant::now();
    let w = &b.world;
    let key = (w.map.id.clone(), w.map.width, w.map.height, b.local);
    let mut changed = false;
    if fd.built_for.as_ref() != Some(&key) {
        let n = (w.map.width * w.map.height) as usize;
        fd.built_for = Some(key);
        fd.target = vec![0; n];
        fd.from = vec![0.0; n];
        fd.start = vec![f32::NEG_INFINITY; n];
        fd.shown = vec![0; n];
        fd.size = (w.map.width, w.map.height);
        fd.tile = if w.map.cell_size > 0.0 { w.map.cell_size } else { FOG_TEXEL };
        fd.revision = w.fog.revision.wrapping_sub(1);
        // the first picture has no fade
        let mut scratch = std::mem::take(&mut fd.scratch);
        fog::shade(w, Some(b.local), gray_value(), &mut scratch);
        if scratch.len() == n {
            fd.target.copy_from_slice(&scratch);
            fd.shown.copy_from_slice(&scratch);
        }
        fd.scratch = scratch;
        changed = true;
    }
    if fd.revision != w.fog.revision {
        fd.revision = w.fog.revision;
        let mut scratch = std::mem::take(&mut fd.scratch);
        fog::shade(w, Some(b.local), gray_value(), &mut scratch);
        if scratch.len() == fd.target.len() {
            for t in 0..scratch.len() {
                if scratch[t] != fd.target[t] {
                    // restart from what is shown now
                    fd.from[t] = f32::from(fd.shown[t]);
                    fd.start[t] = now;
                    fd.target[t] = scratch[t];
                    fd.fading = true;
                }
            }
        }
        fd.scratch = scratch;
    }
    if fd.fading {
        let mut any = false;
        let fade = TERRAIN_FOG_FADE_TIME.max(1e-3);
        for t in 0..fd.target.len() {
            let target = fd.target[t];
            if fd.shown[t] == target {
                continue;
            }
            let k = ((now - fd.start[t]) / fade).clamp(0.0, 1.0);
            let v = (fd.from[t] + (f32::from(target) - fd.from[t]) * k).round().clamp(0.0, 255.0) as u8;
            if v != fd.shown[t] {
                fd.shown[t] = v;
                changed = true;
            }
            if k < 1.0 {
                any = true;
            } else {
                fd.shown[t] = target;
            }
        }
        fd.fading = any;
    }
    if changed {
        if let Some(mut img) = images.get_mut(&FOG_IMAGE) {
            if let Some(data) = img.data.as_mut() {
                write_texels(&fd, data);
            }
        }
    }
    fd.last_us = started.elapsed().as_secs_f32() * 1.0e6;
    // the cost report: every 10 s of battle time
    fd.cost_frames += 1;
    fd.cost_us += fd.last_us;
    fd.cost_max_us = fd.cost_max_us.max(fd.last_us);
    if w.time >= fd.cost_at + 10.0 {
        fd.cost_at = w.time;
        let s = w.fog.stats;
        info!(
            "fog: {} actors, tick {:.0} us (max {:.0} us), texture {:.0} us/frame (max {:.0} us)",
            w.actors.len(),
            s.last_us,
            s.max_us,
            fd.cost_us / fd.cost_frames.max(1) as f32,
            fd.cost_max_us
        );
        fd.cost_frames = 0;
        fd.cost_us = 0.0;
        fd.cost_max_us = 0.0;
    }
}

/// Texel (i, j) covers native ground [4i, 4i + 4) x [4j, 4j + 4) and takes
/// the tile under its centre (`0x71BD00` maps texels to world points).
fn write_texels(fd: &FogDraw, data: &mut [u8]) {
    let (w, h) = fd.size;
    let size = FOG_SIZE as usize;
    if (fd.tile - FOG_TEXEL).abs() < 1e-4 {
        for y in 0..(h as usize).min(size) {
            let row = &fd.shown[y * w as usize..y * w as usize + (w as usize).min(size)];
            data[y * size..y * size + row.len()].copy_from_slice(row);
        }
        return;
    }
    for j in 0..size {
        let ty = (((j as f32) + 0.5) * FOG_TEXEL / fd.tile).floor() as usize;
        if ty >= h as usize {
            break;
        }
        for i in 0..size {
            let tx = (((i as f32) + 0.5) * FOG_TEXEL / fd.tile).floor() as usize;
            if tx >= w as usize {
                break;
            }
            data[j * size + i] = fd.shown[ty * w as usize + tx];
        }
    }
}

/// Hide the actors the local kingdom does not see. Remembered static
/// actors stay drawn; a remembered one that died out of sight is hidden.
pub fn hide_unseen(battle: Option<Res<Battle>>, mut views: Query<(&ActorView, &mut Visibility)>) {
    let Some(b) = battle else { return };
    let w = &b.world;
    for (view, mut vis) in &mut views {
        let show = match fog::sighting(w, view.id, Some(b.local)) {
            Sighting::Visible => true,
            Sighting::Remembered(_) => w.actor(view.id).is_some_and(|a| a.alive),
            Sighting::Hidden => false,
        };
        let want = if show { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
    }
}

/// The local kingdom sees the actor (drawn live, not as a memory).
pub fn sees(b: &Battle, actor: aa_play::ActorId) -> bool {
    matches!(fog::sighting(&b.world, actor, Some(b.local)), Sighting::Visible)
}

/// The local kingdom sees the actor or remembers it (it is drawn).
pub fn drawn(b: &Battle, actor: aa_play::ActorId) -> bool {
    match fog::sighting(&b.world, actor, Some(b.local)) {
        Sighting::Visible => true,
        Sighting::Remembered(_) => b.world.actor(actor).is_some_and(|a| a.alive),
        Sighting::Hidden => false,
    }
}
