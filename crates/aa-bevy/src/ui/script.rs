//! Scripted runs for verification: `AA_UI_SCRIPT=<file.json>` plays a list
//! of steps through the same input path as the mouse and keyboard, and
//! saves screenshots.
//!
//! Steps (one key each): `{"wait": seconds}`, `{"shot": "out.png"}`,
//! `{"click": "<widget path below Root>"}`, `{"rclick": "<path>"}`,
//! `{"move": "<path>"}`, `{"click_at": [fx, fy]}` / `{"rclick_at": [fx, fy]}`
//! (fractions of the window), `{"drag": [fx0, fy0, fx1, fy1]}`,
//! `{"key": "Escape"}` (a KeyCode name), `{"text": "abc"}`,
//! `{"until_state": "Battle"}` (wait for an AppState, 120 s at most),
//! `{"look_at": "enemy"}` (camera to the enemy unit nearest the own HQ),
//! `{"click_actor": kind}` / `{"rclick_actor": kind}` / `{"dclick_actor": kind}`
//! (kind "enemy", "enemy_building", "own_unit", "own_building", "own_banner",
//! "enemy_banner": the on-screen actor / organization banner of that
//! kind nearest the window centre, clicked where it is drawn),
//! `{"dclick_at": [fx, fy]}` (double click), `{"zoom": z}` (camera zoom,
//! battle only), `{"exit": true}`.
use super::front::PointerInput;
use crate::session::AppState;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use serde_json::Value;

#[derive(Resource)]
pub struct Script {
    steps: Vec<Value>,
    at: usize,
    phase: u32,
    timer: f32,
    frames: u32,
    pointer: Option<Vec2>,
    size: Vec2,
}

impl Script {
    pub fn from_env() -> Option<Self> {
        let path = std::env::var_os("AA_UI_SCRIPT")?;
        let text = std::fs::read_to_string(&path).map_err(|e| error!("AA_UI_SCRIPT {:?}: {e}", path)).ok()?;
        let steps: Vec<Value> = serde_json::from_str(&text).map_err(|e| error!("AA_UI_SCRIPT: {e}")).ok()?;
        Some(Self { steps, at: 0, phase: 0, timer: 0.0, frames: 0, pointer: None, size: Vec2::ONE })
    }

    fn step(&self) -> Option<&Value> {
        self.steps.get(self.at)
    }

    fn next(&mut self) {
        self.at += 1;
        self.phase = 0;
        self.timer = 0.0;
        self.frames = 0;
    }

    fn frac(&self, v: &Value, i: usize) -> f32 {
        v.get(i).and_then(|x| x.as_f64()).unwrap_or(0.5) as f32
    }

    /// Pointer / key injection for the front end's frame.
    pub fn inject(&mut self, ui: &mut aa_ui::ui::Ui, input: &mut PointerInput, typed: &mut Vec<(KeyCode, Option<String>)>) {
        if let Some(p) = self.pointer {
            input.pos = Some(p);
        }
        let Some(step) = self.step().cloned() else { return };
        // {"select": ["<list box path>", "<entry text>"]}: select an entry as
        // a click on it does (list box Select + its command)
        if let Some(v) = step.get("select").and_then(|v| v.as_array()) {
            let path = v.first().and_then(|x| x.as_str()).unwrap_or_default();
            let full = if path.starts_with("Root/") { path.to_string() } else { format!("Root/{path}") };
            let text = v.get(1).and_then(|x| x.as_str()).unwrap_or_default();
            let entries = ui.listbox_entries(&full);
            match entries.iter().position(|e| e.text == text).or_else(|| entries.iter().position(|e| e.text.starts_with(text))) {
                Some(i) => ui.listbox_select_notify(&full, i),
                None => warn!("script: no entry {text} in {full}: {:?}", entries.iter().map(|e| e.text.clone()).collect::<Vec<_>>()),
            }
            self.next();
            return;
        }
        let widget_centre = |path: &str| -> Option<Vec2> {
            let full = if path.starts_with("Root/") { path.to_string() } else { format!("Root/{path}") };
            let w = ui.widget(&full)?;
            let r = w.root.outer.rect;
            Some(Vec2::new((r.l + r.r) * 0.5, (r.t + r.b) * 0.5))
        };
        let target = if let Some(p) = step.get("click").or(step.get("rclick")).or(step.get("move")).and_then(|v| v.as_str()) {
            match widget_centre(p) {
                Some(c) => Some(c),
                None => {
                    if self.frames > 120 {
                        warn!("script: widget {p} not found");
                        self.next();
                    }
                    self.frames += 1;
                    return;
                }
            }
        } else if let Some(v) = step.get("click_at").or(step.get("rclick_at")).or(step.get("dclick_at")) {
            Some(Vec2::new(self.frac(v, 0) * self.size.x, self.frac(v, 1) * self.size.y))
        } else {
            None
        };
        let right = step.get("rclick").is_some() || step.get("rclick_at").is_some();
        if let Some(t) = target {
            match self.phase {
                0 => {
                    self.pointer = Some(t);
                    input.pos = Some(t);
                    self.phase = 1;
                }
                1 if step.get("move").is_some() => self.next(),
                1 => {
                    input.pos = Some(t);
                    if right { input.right_pressed = true } else { input.left_pressed = true }
                    self.phase = 2;
                }
                2 if step.get("dclick_at").is_some() => {
                    input.pos = Some(t);
                    input.left_released = true;
                    self.phase = 3;
                }
                3 => {
                    input.pos = Some(t);
                    input.left_pressed = true;
                    self.phase = 4;
                }
                _ => {
                    input.pos = Some(t);
                    if right { input.right_released = true } else { input.left_released = true }
                    self.next();
                }
            }
            return;
        }
        if let Some(v) = step.get("drag") {
            let a = Vec2::new(self.frac(v, 0) * self.size.x, self.frac(v, 1) * self.size.y);
            let b = Vec2::new(self.frac(v, 2) * self.size.x, self.frac(v, 3) * self.size.y);
            let n = 10;
            match self.phase {
                0 => {
                    self.pointer = Some(a);
                    input.pos = Some(a);
                    self.phase = 1;
                }
                1 => {
                    input.pos = Some(a);
                    input.left_pressed = true;
                    self.phase = 2;
                }
                p if p < 2 + n => {
                    let t = (p - 1) as f32 / n as f32;
                    let q = a.lerp(b, t);
                    self.pointer = Some(q);
                    input.pos = Some(q);
                    self.phase += 1;
                }
                _ => {
                    input.pos = Some(b);
                    input.left_released = true;
                    self.next();
                }
            }
            return;
        }
        if let Some(k) = step.get("key").and_then(|v| v.as_str()) {
            if let Some(code) = key_code(k) {
                typed.push((code, None));
            }
            self.next();
            return;
        }
        if let Some(t) = step.get("text").and_then(|v| v.as_str()) {
            for c in t.chars() {
                typed.push((KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified), Some(c.to_string())));
            }
            self.next();
        }
    }
}

fn key_code(name: &str) -> Option<KeyCode> {
    Some(match name {
        "Escape" => KeyCode::Escape,
        "Enter" => KeyCode::Enter,
        "Tab" => KeyCode::Tab,
        "Space" => KeyCode::Space,
        "Home" => KeyCode::Home,
        "Delete" => KeyCode::Delete,
        "Backspace" => KeyCode::Backspace,
        "F1" => KeyCode::F1,
        "F10" => KeyCode::F10,
        "Pause" => KeyCode::Pause,
        s if s.len() == 1 && s.chars().all(|c| c.is_ascii_alphabetic()) => match s.to_ascii_uppercase().as_str() {
            "A" => KeyCode::KeyA,
            "D" => KeyCode::KeyD,
            "H" => KeyCode::KeyH,
            "P" => KeyCode::KeyP,
            "S" => KeyCode::KeyS,
            "W" => KeyCode::KeyW,
            _ => return None,
        },
        _ => return None,
    })
}

/// Timed steps (wait, shot, until_state, exit).
pub fn run(
    mut commands: Commands,
    script: Option<ResMut<Script>>,
    time: Res<Time>,
    state: Res<State<AppState>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    draw: Option<Res<super::render::UiDraw>>,
    mut exit: MessageWriter<AppExit>,
    battle: Option<Res<crate::session::Battle>>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::camera::NativeCamera>>,
    views: Query<(&crate::battle_view::ActorView, &GlobalTransform)>,
    mut requests: ResMut<crate::camera::CameraRequests>,
    banners: Option<Res<super::banners::BannerArt>>,
) {
    let Some(mut s) = script else { return };
    let mut logical = Vec2::ONE;
    if let Ok(w) = windows.single() {
        s.size = Vec2::new(w.physical_width() as f32, w.physical_height() as f32);
        logical = Vec2::new(w.width(), w.height());
    }
    // actor steps become pointer steps at the actor's screen position
    if let (Some(step), Some(b)) = (s.step().cloned(), &battle) {
        let kind = |k: &str| step.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let w = &b.world;
        let team = |k: Option<aa_play::KingdomId>| k.and_then(|k| w.kingdoms.iter().find(|x| x.id == k)).map(|x| x.team);
        let local_team = team(Some(b.local));
        let matches = |id: aa_play::ActorId, what: &str| -> bool {
            let Some(a) = w.actor(id).filter(|a| a.alive) else { return false };
            let building = w.data.def(a.def).class == Some(aa_play::DefClass::Building);
            match what {
                // a hostile object with a body (not a start-location marker),
                // units before buildings
                "enemy" => a.kingdom.is_some() && team(a.kingdom) != local_team && w.data.def(a.def).health > 0.0 && !building,
                "enemy_building" => a.kingdom.is_some() && team(a.kingdom) != local_team && building,
                "own_unit" => a.kingdom == Some(b.local) && a.org.is_some(),
                "own_building" => a.kingdom == Some(b.local) && building,
                _ => false,
            }
        };
        if let Some(z) = step.get("zoom").and_then(|v| v.as_f64()) {
            requests.zoom = Some(z as f32);
            s.next();
            return;
        }
        if let Some(what) = kind("look_at") {
            let hq = w.actors.iter().find(|a| a.alive && a.kingdom == Some(b.local) && w.data.def(a.def).class == Some(aa_play::DefClass::Building)).map(|a| a.pos.truncate()).unwrap_or_default();
            let best = w.actors.iter().filter(|a| matches(a.id, &what)).min_by(|x, y| x.pos.truncate().distance(hq).total_cmp(&y.pos.truncate().distance(hq)));
            match best {
                Some(a) => {
                    info!("script: look at {what} actor {} ({}) at {:?}", a.id, w.data.def(a.def).name, a.pos);
                    requests.look_at = Some(a.pos.truncate());
                }
                None => warn!("script: no {what} actor"),
            }
            s.next();
            return;
        }
        for (key, out) in [("click_actor", "click_at"), ("rclick_actor", "rclick_at"), ("dclick_actor", "dclick_at")] {
            let Some(what) = kind(key) else { continue };
            let Ok((cam, cam_tf)) = cameras.single() else { return };
            let centre = logical * 0.5;
            // "own_banner" / "enemy_banner": an organization's banner
            if let Some(side) = what.strip_suffix("_banner") {
                let own = side == "own";
                let best = banners
                    .iter()
                    .flat_map(|bn| bn.placed.iter())
                    .filter(|(o, _, _)| w.orgs.get(*o as usize).is_some_and(|g| (g.kingdom == b.local) == own))
                    .filter_map(|(o, c, _)| cam.world_to_viewport(cam_tf, *c).ok().map(|p| (*o, p)))
                    .filter(|(_, p)| p.x > 0.0 && p.y > 0.0 && p.x < logical.x && p.y < logical.y * 0.75)
                    .min_by(|x, y| x.1.distance(centre).total_cmp(&y.1.distance(centre)));
                match best {
                    Some((o, p)) => {
                        let f = p / logical;
                        info!("script: {key} {what} org {o} at {:.3},{:.3}", f.x, f.y);
                        let at = s.at;
                        s.steps[at] = serde_json::json!({ out: [f.x, f.y] });
                    }
                    None => {
                        warn!("script: no on-screen {what}");
                        s.next();
                    }
                }
                return;
            }
            let best = views
                .iter()
                .filter(|(v, _)| matches(v.id, &what))
                .filter_map(|(v, tf)| cam.world_to_viewport(cam_tf, tf.translation() + Vec3::Y * 0.3).ok().map(|p| (v.id, p)))
                .filter(|(_, p)| p.x > 0.0 && p.y > 0.0 && p.x < logical.x && p.y < logical.y * 0.75)
                .min_by(|x, y| x.1.distance(centre).total_cmp(&y.1.distance(centre)));
            match best {
                Some((id, p)) => {
                    let f = p / logical;
                    info!("script: {key} {what} actor {id} at {:.3},{:.3}", f.x, f.y);
                    let at = s.at;
                    s.steps[at] = serde_json::json!({ out: [f.x, f.y] });
                }
                None => {
                    let seen: Vec<_> = views.iter().filter(|(v, _)| matches(v.id, &what)).map(|(v, tf)| (v.id, cam.world_to_viewport(cam_tf, tf.translation()).ok())).collect();
                    warn!("script: no on-screen {what} actor (views {seen:?})");
                    s.next();
                }
            }
            return;
        }
    }
    let dt = time.delta_secs();
    let Some(step) = s.step().cloned() else { return };
    if let Some(secs) = step.get("wait").and_then(|v| v.as_f64()) {
        s.timer += dt;
        if s.timer >= secs as f32 {
            s.next();
        }
    } else if let Some(path) = step.get("shot").and_then(|v| v.as_str()) {
        // let the frame settle, shoot, then give the capture a few frames
        s.frames += 1;
        if s.frames == 2 {
            info!("script: screenshot {path}");
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.to_string()));
        }
        if s.frames > 8 {
            s.next();
        }
    } else if let Some(want) = step.get("until_state").and_then(|v| v.as_str()) {
        s.timer += dt;
        let now = format!("{:?}", state.get());
        if now == want || s.timer > 120.0 {
            if s.timer > 120.0 {
                warn!("script: state {want} not reached (still {now})");
            }
            s.next();
        }
    } else if let Some(path) = step.get("dump").and_then(|v| v.as_str()) {
        // the current draw list as JSON (debugging)
        if let Some(d) = &draw {
            let _ = std::fs::write(path, serde_json::to_string_pretty(&d.list).unwrap_or_default());
        }
        s.next();
    } else if step.get("exit").is_some() {
        s.frames += 1;
        if s.frames > 2 {
            exit.write(AppExit::Success);
        }
    }
}
