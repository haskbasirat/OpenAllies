//! Measurement only (AA_BEVY_FX_BENCH=<soldiers>): puts that many Russian and
//! German infantry (whole original regiments, half each side) in two lines
//! near the local HQ and orders them to attack-move at each other, so the
//! frame cost of the effects and sound can be measured with ~100 soldiers
//! firing. Everything after the spawn is the real game systems.
use crate::session::Battle;
use aa_play::{Command, Unit};
use bevy::prelude::*;

pub fn spawn(battle: Option<ResMut<Battle>>, ready: Option<Res<super::FxState>>, mut done: Local<bool>) {
    if *done || ready.is_none() {
        return;
    }
    let Some(mut battle) = battle else { return };
    // AA_BEVY_FX_BENCH_SPEED: the game speed for the measurement (slow
    // motion lets short animations such as a tank's recoil or a bomber's
    // pass be captured frame by frame on a slow renderer).
    if let Some(speed) = std::env::var("AA_BEVY_FX_BENCH_SPEED").ok().and_then(|v| v.parse::<f32>().ok()) {
        battle.speed = speed;
    }
    // AA_BEVY_VIEW_TEST_HQ=<fraction>: the local kingdom's buildings start at
    // that fraction of their health (aspect skins: aspect_disabled at 0.40).
    if let Some(f) = std::env::var("AA_BEVY_VIEW_TEST_HQ").ok().and_then(|v| v.parse::<f32>().ok()) {
        let local = battle.local;
        let world = &mut battle.world;
        let ids: Vec<aa_play::ActorId> = world.actors.iter().filter(|a| a.kingdom == Some(local) && !a.removed && world.data.def(a.def).class == Some(aa_play::DefClass::Building)).map(|a| a.id).collect();
        for id in ids {
            let max = world.combat.actor(id).map_or(0.0, |c| c.max_health);
            let max = if max > 0.0 { max } else { world.data.def(world.actors[id as usize].def).health };
            world.actors[id as usize].health = max * f;
        }
        info!("view test: local buildings at {:.0}% health", f * 100.0);
    }
    // AA_BEVY_VIEW_TEST=<thing ids>: single actors of the local kingdom in a
    // row west of its HQ; trucks that unpack are sent to unpack 24 m away,
    // the others drive there (view animation checks: tires, treads,
    // trailers, tilt, unpacking).
    if let Ok(list) = std::env::var("AA_BEVY_VIEW_TEST") {
        let local = battle.local;
        let world = &mut battle.world;
        let hq = world
            .actors
            .iter()
            .find(|a| a.kingdom == Some(local) && !a.removed && world.data.def(a.def).class == Some(aa_play::DefClass::Building) && !world.data.def(a.def).invisible)
            .map(|a| a.pos.truncate())
            .unwrap_or(Vec2::new(100.0, 100.0));
        let mut orders = Vec::new();
        // AA_BEVY_VIEW_TEST_SLOPE=1: on the steepest ground within 60 m
        // (the tilt check), driving across the slope.
        let slope = std::env::var_os("AA_BEVY_VIEW_TEST_SLOPE").is_some().then(|| {
            let h = |p: Vec2| world.map.height_at(p);
            let mut best = (0.0f32, hq, Vec2::X);
            for gx in -15..=15 {
                for gy in -15..=15 {
                    let p = hq + Vec2::new(gx as f32 * 4.0, gy as f32 * 4.0);
                    let size = Vec2::new(world.map.width as f32, world.map.height as f32) * world.map.cell_size;
                    if p.x < 30.0 || p.y < 30.0 || p.x > size.x - 30.0 || p.y > size.y - 30.0 {
                        continue;
                    }
                    let g = Vec2::new(h(p + Vec2::X * 2.0) - h(p - Vec2::X * 2.0), h(p + Vec2::Y * 2.0) - h(p - Vec2::Y * 2.0)) / 4.0;
                    // drivable: steep, but no cliff or wall
                    if g.length() > best.0 && g.length() < 0.35 {
                        best = (g.length(), p, g.normalize_or(Vec2::X));
                    }
                }
            }
            info!("view test: steepest ground {:.2} at {:?}", best.0, best.1);
            best
        });
        for (i, id) in list.split(',').map(str::trim).filter(|s| !s.is_empty()).enumerate() {
            let Some(def) = world.data.find(id) else {
                warn!("view test: no thing {id}");
                continue;
            };
            // AA_BEVY_VIEW_TEST_EAST=1: east of the HQ, driving east (in view).
            let east = std::env::var_os("AA_BEVY_VIEW_TEST_EAST").is_some();
            // AA_BEVY_VIEW_TEST_NEAR=1: close west of the HQ, a short walk.
            let near = std::env::var_os("AA_BEVY_VIEW_TEST_NEAR").is_some();
            let (at, to) = if let Some((_, p, g)) = slope {
                // across the slope: perpendicular to the gradient
                let across = Vec2::new(-g.y, g.x);
                let at = p - across * 10.0 + g * (i as f32 * 5.0 - 5.0);
                (at, at + across * 20.0)
            } else if near {
                let at = hq + Vec2::new(-12.0, -8.0 + i as f32 * 4.0);
                (at, at + Vec2::new(-8.0, 0.0))
            } else if east {
                let at = hq + Vec2::new(14.0, -12.0 + i as f32 * 8.0);
                (at, at + Vec2::new(14.0, 3.0))
            } else {
                let at = hq + Vec2::new(-16.0, -14.0 + i as f32 * 7.0);
                (at, at + Vec2::new(-24.0, 4.0))
            };
            let a = aa_play::production::spawn_single(world, def, Some(local), at, 0.0);
            if world.data.def(def).unpacks_to.is_some() {
                orders.push(Command::Deploy { kingdom: local, truck: a, at: to, facing: std::f32::consts::PI });
            } else {
                orders.push(Command::Move { kingdom: local, units: vec![Unit::Actor(a)], to, attack_move: false });
            }
            info!("view test: {id} actor {a} at {at:?} -> {to:?}");
        }
        battle.pending.extend(orders);
    }
    let Some(soldiers) = std::env::var("AA_BEVY_FX_BENCH").ok().and_then(|v| v.parse::<usize>().ok()) else {
        *done = true;
        return;
    };
    *done = true;
    let world = &mut battle.world;
    // AA_BEVY_FX_BENCH_KIND: the regiment kind ("infantry", "tank_medium",
    // "infantry_artillery"...), the same for both nations.
    let kind = std::env::var("AA_BEVY_FX_BENCH_KIND").unwrap_or_else(|_| "infantry".into());
    // "<russian kind>/<german kind>" sets the two sides apart.
    let (rk, gk) = kind.split_once('/').unwrap_or((kind.as_str(), kind.as_str()));
    let (Some(rus), Some(ger)) = (world.data.find(&format!("regiment_rus_{rk}")), world.data.find(&format!("regiment_ger_{gk}"))) else {
        warn!("fx bench: no regiment_rus_{rk} / regiment_ger_{gk}");
        return;
    };
    if world.kingdoms.len() < 2 {
        return;
    }
    let hq = world
        .actors
        .iter()
        .find(|a| a.kingdom == Some(0) && !a.removed && world.data.def(a.def).class == Some(aa_play::DefClass::Building) && !world.data.def(a.def).invisible)
        .map(|a| a.pos.truncate())
        .unwrap_or(Vec2::new(100.0, 100.0));
    let a = hq + Vec2::new(-1.4, -18.4);
    let b = hq + Vec2::new(18.4, 1.4);
    let d = (b - a).normalize_or_zero();
    let perp = Vec2::new(-d.y, d.x);
    let per_side = soldiers.div_ceil(12).max(1);
    let mut orders = Vec::new();
    for i in 0..per_side {
        let off = perp * ((i as f32 - (per_side as f32 - 1.0) * 0.5) * 9.0) - d * 6.0;
        let ours = aa_play::production::spawn_organization(world, rus, 0, a + off, 0.0, None);
        let theirs = aa_play::production::spawn_organization(world, ger, 1, b + off + d * 12.0, std::f32::consts::PI, None);
        orders.push(Command::Move { kingdom: 0, units: vec![Unit::Org(ours)], to: b + off, attack_move: true });
        orders.push(Command::Move { kingdom: 1, units: vec![Unit::Org(theirs)], to: a + off, attack_move: true });
    }
    battle.pending.extend(orders);
    info!("fx bench: {} regiments a side ordered to attack-move", per_side);
}

/// Measurement only (AA_BEVY_FX_FRAMES=<ms>): every frame longer than that
/// (real time, not the 250 ms clamped game delta) in the log, with the app
/// state, so stalls between Launch and the battle can be found.
pub fn frame_times(time: Res<Time<Real>>, state: Res<State<crate::session::AppState>>, mut limit: Local<Option<f32>>) {
    let limit = *limit.get_or_insert_with(|| std::env::var("AA_BEVY_FX_FRAMES").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(f32::MAX));
    let ms = time.delta_secs() * 1000.0;
    if ms > limit {
        info!("frame time: {ms:.0} ms ({:?})", state.get());
    }
}

/// Measurement only (AA_BEVY_FX_RECOIL_SHOTS=<file prefix>): the first time
/// the actor nearest the camera target starts an object animation (a tank's
/// recoil) after 3 s of battle, saves screenshots 0.05, 0.25, 0.5 and 1.5 s
/// of battle time after it (AA_BEVY_FX_BENCH_SPEED slows the game so a slow
/// renderer still draws those moments). It also logs which of the actor's
/// drawn part meshes differ from the pose before the shot (battle_view
/// shares one mesh per clip frame, so a changed handle is another frame of
/// the clip). The prefix "none" skips the screenshots.
#[allow(clippy::too_many_arguments)]
pub fn recoil_shots(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    anims: Option<Res<crate::session::ObjectAnimations>>,
    cams: Query<&crate::camera::NativeCamera>,
    views: Query<(Entity, &crate::battle_view::ActorView)>,
    children: Query<&Children>,
    parts: Query<&Mesh3d>,
    mut state: Local<Option<(aa_play::ActorId, f32, usize)>>,
    mut before: Local<Vec<AssetId<Mesh>>>,
) {
    const AFTER: [f32; 4] = [0.05, 0.25, 0.5, 1.5];
    let Some(prefix) = std::env::var_os("AA_BEVY_FX_RECOIL_SHOTS") else { return };
    let (Some(battle), Some(anims)) = (battle, anims) else { return };
    let now = battle.world.time;
    // The actor's drawn part meshes.
    let pose = |a: aa_play::ActorId| -> Vec<AssetId<Mesh>> {
        let Some((e, _)) = views.iter().find(|(_, v)| v.id == a) else { return Vec::new() };
        let Ok(kids) = children.get(e) else { return Vec::new() };
        kids.iter().filter_map(|c| parts.get(c).ok()).map(|m| m.0.id()).collect()
    };
    if state.is_none() {
        let Ok(cam) = cams.single() else { return };
        let target = cam.target.truncate();
        let best = anims
            .0
            .iter()
            .filter(|(_, (_, start))| *start >= 3.0 && now - *start < 0.1)
            .filter_map(|(a, (_, start))| battle.world.actor(*a).map(|x| (*a, *start, x.pos.truncate().distance(target))))
            .filter(|(_, _, d)| *d < 15.0)
            .min_by(|x, y| x.2.total_cmp(&y.2));
        if let Some((a, start, d)) = best {
            info!("recoil shots: actor {a} {:?} ({d:.1} m from the camera target), started at battle time {start:.2}", battle.world.actor(a).map(|x| x.pos));
            *state = Some((a, start, 0));
            // battle_view drew this frame before the animation was written.
            *before = pose(a);
        }
        return;
    }
    let Some((a, start, next)) = state.as_mut() else { return };
    if *next < AFTER.len() && now >= *start + AFTER[*next] {
        let now_pose = pose(*a);
        let changed = now_pose.iter().zip(before.iter()).filter(|(p, b)| p != b).count();
        info!("recoil shots: actor {a} at {:.2} s after its start: {changed} of {} part meshes differ from the pose before the shot", now - *start, now_pose.len());
        if prefix != "none" {
            let path = format!("{}_{}_{:.2}.png", prefix.to_string_lossy(), *next, now - *start);
            info!("recoil shots: screenshot {path}");
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(path));
        }
        *next += 1;
    }
}

/// Measurement only (AA_BEVY_FX_DEATH_SHOTS=<file prefix>): the first time a
/// vehicle within 25 m of the camera target starts dying, moves the camera
/// to it and saves screenshots 0.1 to 4 s of battle time later (its death
/// explosion, the flying debris and the wreck).
pub fn death_shots(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    cams: Query<&crate::camera::NativeCamera>,
    mut requests: ResMut<crate::camera::CameraRequests>,
    mut state: Local<Option<(aa_play::ActorId, f32, usize)>>,
) {
    const AFTER: [f32; 6] = [0.1, 0.4, 0.8, 1.4, 2.2, 4.0];
    let Some(prefix) = std::env::var_os("AA_BEVY_FX_DEATH_SHOTS") else { return };
    let Some(battle) = battle else { return };
    let world = &battle.world;
    let now = world.time;
    if state.is_none() {
        let Ok(cam) = cams.single() else { return };
        let target = cam.target.truncate();
        let best = world
            .actors
            .iter()
            .filter(|a| a.activity == aa_play::Activity::Dying && now - a.activity_since < 0.1 && world.data.def(a.def).name.contains("tank"))
            .map(|a| (a.id, a.activity_since, a.pos.truncate().distance(target), a.pos.truncate()))
            .filter(|x| x.2 < 25.0)
            .min_by(|x, y| x.2.total_cmp(&y.2));
        if let Some((a, start, d, pos)) = best {
            info!("death shots: actor {a} ({}) at {pos:?} ({d:.1} m from the camera target) started dying at battle time {start:.2}", world.actor(a).map(|x| world.data.def(x.def).name.as_str()).unwrap_or(""));
            requests.look_at = Some(pos);
            *state = Some((a, start, 0));
        }
        return;
    }
    let Some((a, start, next)) = state.as_mut() else { return };
    if *next < AFTER.len() && now >= *start + AFTER[*next] {
        let path = format!("{}_{}_{:.2}.png", prefix.to_string_lossy(), *next, now - *start);
        info!("death shots: actor {a} at {:.2} s after its death -> {path}", now - *start);
        commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(path));
        *next += 1;
    }
}
