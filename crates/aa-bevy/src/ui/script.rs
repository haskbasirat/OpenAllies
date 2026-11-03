//! Scripted runs for verification: `AA_UI_SCRIPT=<file.json>` plays a list
//! of steps through the same input path as the mouse and keyboard, and
//! saves screenshots.
//!
//! Steps (one key each): `{"wait": seconds}`, `{"shot": "out.png"}`,
//! `{"click": "<widget path below Root>"}`, `{"rclick": "<path>"}`,
//! `{"move": "<path>"}`, `{"click_at": [fx, fy]}` / `{"rclick_at": [fx, fy]}`
//! (fractions of the window), `{"drag": [fx0, fy0, fx1, fy1]}`,
//! `{"press_at": [fx, fy]}` / `{"release_at": [fx, fy]}` (only the left
//! press / release there), `{"key": "Escape"}` (a KeyCode name),
//! `{"text": "abc"}`,
//! `{"until_state": "Battle"}` (wait for an AppState, 120 s at most),
//! `{"until_box": seconds}` (wait for a message box, the end of a film),
//! `{"until_tick": n}` (wait for the world's tick n, 600 s at most),
//! `{"look_at": "enemy"}` (camera to the enemy unit nearest the own HQ),
//! `{"click_actor": kind}` / `{"rclick_actor": kind}` / `{"dclick_actor": kind}`
//! (kind "enemy", "enemy_building", "own_unit", "own_building", "own_truck",
//! "own_hq", "own_corps", "city_center", "own_banner",
//! "enemy_banner": the on-screen actor / organization banner of that
//! kind nearest the window centre, clicked where it is drawn, or at
//! `"offset": [dx, dy]` window fractions from there),
//! `{"dclick_at": [fx, fy]}` (double click), `{"zoom": z}` (camera zoom,
//! battle only), `{"exit": true}`; `{"move_at": [fx, fy]}` /
//! `{"move_actor": kind}` move the pointer there without a click
//! (appended by the combat owner); `{"spawn_org": "<Thing IDS>"}` creates
//! an own organization beside the own HQ (`"offset": [dx, dy]` metres), a
//! verification aid for the HUD's order buttons (`{"spawn_actor": ...}` the
//! same for a single actor, a truck); `{"tint_actions": [r, g,
//! b]}` (or null) draws every action button tinted, a verification aid for
//! texture effect modes 2 and 3; `{"stats": "label"}` logs each kingdom's
//! counts and the selection's orders. For following a mission's script (`"limit"`
//! seconds at most, 300 by default): `{"until_listen": "<UI command>"}` waits until
//! the script listens to that command, `{"until_trigger": "<name prefix>", "on": false}`
//! until that trigger is on or off (off by default); the actor kind `slot:<GTID slot>`
//! is the map's thing of that low word; `{"click_command": "<UI command>"}` clicks the
//! action button that shows that command, or logs that none does;
//! `{"rclick_world": [x, y]}` / `{"click_world": [x, y]}` / `{"move_world": [x, y]}` /
//! `{"look_at_point": [x, y]}` take a native ground point; `{"destroy": "enemy"}` kills
//! every hostile actor by the real death path, for the end of a battle.
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
        } else if let Some(v) = step.get("click_at").or(step.get("rclick_at")).or(step.get("dclick_at")).or(step.get("move_at")) {
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
                1 if step.get("move").is_some() || step.get("move_at").is_some() => self.next(),
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
        // {"press_at": [fx, fy]} / {"release_at": [fx, fy]}: the pointer
        // there, then only a left press / only a left release (a held drag
        // the script can photograph halfway; move_at moves it meanwhile)
        // ({"rpress_at": ...} / {"rrelease_at": ...}: the same for the right button)
        let right_held = step.get("rpress_at").is_some() || step.get("rrelease_at").is_some();
        if let Some((v, press)) = step.get("press_at").or(step.get("rpress_at")).map(|v| (v, true)).or(step.get("release_at").or(step.get("rrelease_at")).map(|v| (v, false))) {
            let t = Vec2::new(self.frac(v, 0) * self.size.x, self.frac(v, 1) * self.size.y);
            if self.phase == 0 {
                self.pointer = Some(t);
                input.pos = Some(t);
                self.phase = 1;
            } else {
                input.pos = Some(t);
                match (right_held, press) {
                    (false, true) => input.left_pressed = true,
                    (false, false) => input.left_released = true,
                    (true, true) => input.right_pressed = true,
                    (true, false) => input.right_released = true,
                }
                self.next();
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
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F10" => KeyCode::F10,
        "Equal" => KeyCode::Equal,
        "Minus" => KeyCode::Minus,
        "Backslash" => KeyCode::Backslash,
        "NumpadAdd" => KeyCode::NumpadAdd,
        "NumpadSubtract" => KeyCode::NumpadSubtract,
        "Pause" => KeyCode::Pause,
        // a letter, also as its KeyCode name ("KeyQ")
        s if (s.len() == 1 || (s.len() == 4 && s.starts_with("Key"))) && s.chars().last().is_some_and(|c| c.is_ascii_alphabetic()) => {
            const LETTERS: [KeyCode; 26] = [
                KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC, KeyCode::KeyD, KeyCode::KeyE, KeyCode::KeyF, KeyCode::KeyG, KeyCode::KeyH, KeyCode::KeyI,
                KeyCode::KeyJ, KeyCode::KeyK, KeyCode::KeyL, KeyCode::KeyM, KeyCode::KeyN, KeyCode::KeyO, KeyCode::KeyP, KeyCode::KeyQ, KeyCode::KeyR,
                KeyCode::KeyS, KeyCode::KeyT, KeyCode::KeyU, KeyCode::KeyV, KeyCode::KeyW, KeyCode::KeyX, KeyCode::KeyY, KeyCode::KeyZ,
            ];
            let c = s.chars().last()?.to_ascii_uppercase();
            LETTERS[(c as u8 - b'A') as usize]
        }
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
    mut quit: ResMut<super::Quit>,
    mut battle: Option<ResMut<crate::session::Battle>>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::camera::NativeCamera>>,
    views: Query<(&crate::battle_view::ActorView, &GlobalTransform)>,
    mut requests: ResMut<crate::camera::CameraRequests>,
    banners: Option<Res<super::banners::BannerArt>>,
    mut front: Option<ResMut<super::front::Front>>,
) {
    let Some(mut s) = script else { return };
    // {"tint_actions": [r, g, b]} / {"tint_actions": null}: a verification
    // aid that draws every action button tinted (texture effect mode 2, 3
    // when disabled), for the renderer's tint stages
    if let Some(v) = s.step().and_then(|v| v.get("tint_actions")).cloned() {
        let tint = v.as_array().map(|a| [0, 1, 2].map(|i| a.get(i).and_then(|x| x.as_f64()).unwrap_or(1.0) as f32));
        if let Some(h) = front.as_mut().and_then(|f| f.hud.as_mut()) {
            h.debug_tint = tint;
        }
        s.next();
        return;
    }
    // {"spawn_org": "<organization Thing IDS>", "offset": [dx, dy]}: a
    // verification aid that creates an own organization next to the own
    // HQ (metres), so the HUD's buttons for it can be checked
    // verification aid: the local kingdom's mission result (4E9720)
    if let (Some(r), Some(b)) = (s.step().and_then(|v| v.get("mission_result")).and_then(|v| v.as_u64()), battle.as_mut()) {
        let local = b.local;
        aa_play::triggers::set_mission_result(&mut b.world, local, r as u32);
        s.next();
        return;
    }
    if let (Some(id), Some(b)) = (s.step().and_then(|v| v.get("spawn_org")).and_then(|v| v.as_str()).map(str::to_string), battle.as_mut()) {
        let off = s.step().and_then(|v| v.get("offset")).and_then(|v| v.as_array()).map(|a| Vec2::new(a.first().and_then(|x| x.as_f64()).unwrap_or(0.0) as f32, a.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32)).unwrap_or(Vec2::new(-10.0, -15.0));
        let local = b.local;
        let world = &mut b.world;
        // "$nation" in the IDS: the local kingdom's nation (a random one)
        let nation = world.kingdoms.iter().find(|k| k.id == local).map(|k| k.nation.clone()).unwrap_or_default();
        let id = id.replace("$nation", &nation);
        let hq = world.actors.iter().find(|a| a.alive && a.kingdom == Some(local) && world.data.def(a.def).class == Some(aa_play::DefClass::Building)).map(|a| a.pos.truncate()).unwrap_or_default();
        match world.data.find(&id) {
            Some(def) => {
                let org = aa_play::production::spawn_organization(world, def, local, hq + off, 0.0, None);
                info!("script: spawned {id} as organization {org}");
            }
            None => warn!("script: no {id} in the data"),
        }
        s.next();
        return;
    }
    // {"spawn_actor": "<Thing IDS>", "offset": [dx, dy]}: the same aid for a
    // single own actor (a truck)
    if let (Some(id), Some(b)) = (s.step().and_then(|v| v.get("spawn_actor")).and_then(|v| v.as_str()).map(str::to_string), battle.as_mut()) {
        let off = s.step().and_then(|v| v.get("offset")).and_then(|v| v.as_array()).map(|a| Vec2::new(a.first().and_then(|x| x.as_f64()).unwrap_or(0.0) as f32, a.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32)).unwrap_or(Vec2::new(-10.0, -15.0));
        let local = b.local;
        let world = &mut b.world;
        let hq = world.actors.iter().find(|a| a.alive && a.kingdom == Some(local) && world.data.def(a.def).class == Some(aa_play::DefClass::Building)).map(|a| a.pos.truncate()).unwrap_or_default();
        match world.data.find(&id) {
            Some(def) => {
                let a = aa_play::production::spawn_single(world, def, Some(local), hq + off, -std::f32::consts::FRAC_PI_2);
                info!("script: spawned {id} as actor {a}");
            }
            None => warn!("script: no {id} in the data"),
        }
        s.next();
        return;
    }
    // {"destroy": "enemy"}: a verification aid, every hostile actor dies by the real
    // death path, so the battle's end comes as it comes after a fight
    if let (Some(_), Some(b)) = (s.step().and_then(|v| v.get("destroy")), battle.as_mut()) {
        let local_team = b.world.kingdoms.iter().find(|k| k.id == b.local).map(|k| k.team);
        let enemies: Vec<aa_play::KingdomId> = b.world.kingdoms.iter().filter(|k| !k.independent && Some(k.team) != local_team).map(|k| k.id).collect();
        let list: Vec<aa_play::ActorId> = b.world.actors.iter().filter(|a| a.alive && a.kingdom.is_some_and(|k| enemies.contains(&k))).map(|a| a.id).collect();
        info!("script: destroying {} actors of {enemies:?}", list.len());
        // as the trigger's 4D7960: the death follows in the next tick's combat pass
        for a in list {
            b.world.actors[a as usize].health = 0.0;
        }
        s.next();
        return;
    }
    // follow the mission's script: until the script listens to a command, until a
    // trigger is on or off
    if let (Some(step), Some(b)) = (s.step().cloned(), battle.as_ref()) {
        let limit = step.get("limit").and_then(|v| v.as_f64()).unwrap_or(300.0) as f32;
        let mut wanted = None;
        if let Some(c) = step.get("until_listen").and_then(|v| v.as_str()) {
            wanted = Some((format!("the script to listen to {c}"), aa_play::triggers::listening(&b.world, c)));
        }
        if let Some(name) = step.get("until_trigger").and_then(|v| v.as_str()) {
            let on = step.get("on").and_then(|v| v.as_bool()).unwrap_or(false);
            let state = aa_play::triggers::trigger_states(&b.world).iter().find(|t| t.1.starts_with(name)).map(|t| t.2);
            wanted = Some((format!("trigger {name} to be {on}"), state == Some(on)));
        }
        if let Some((what, done)) = wanted {
            s.timer += time.delta_secs();
            if done {
                info!("script: waited {:.1} s for {what} (game time {:.1})", s.timer, b.world.time);
                s.next();
            } else if s.timer > limit {
                warn!("script: gave up waiting for {what} after {limit} s (game time {:.1})", b.world.time);
                s.next();
            }
            return;
        }
    }
    // {"stats": "label"}: a verification aid, a log line per kingdom and the
    // selection's orders
    if let Some(label) = s.step().and_then(|v| v.get("stats")).and_then(|v| v.as_str()).map(str::to_string) {
        if let Some(b) = battle.as_ref() {
            let w = &b.world;
            info!("stats {label}: tick {} winner {:?}", w.tick, w.winner);
            for k in w.kingdoms.iter().filter(|k| !k.independent) {
                let alive = |a: &&aa_play::Actor| a.alive && a.kingdom == Some(k.id);
                let buildings = w.actors.iter().filter(alive).filter(|a| w.data.def(a.def).class == Some(aa_play::DefClass::Building)).count();
                let units = w.actors.iter().filter(alive).count() - buildings;
                let dead = w.actors.iter().filter(|a| !a.alive && a.kingdom == Some(k.id)).count();
                let orgs = w.orgs.iter().filter(|o| o.alive && o.kingdom == k.id).count();
                info!("stats {label}: {} {} team {} orgs {orgs} units {units} buildings {buildings} dead {dead} money {:.0} defeated {}", k.name, k.nation, k.team, k.stockpile[aa_play::data::MONEY], k.defeated);
            }
            for u in &b.selection {
                info!("stats {label}: command mask {:#x} guess at a point {:?}", aa_play::orders::guess::action_bits(w, *u), aa_play::orders::guess::guess(w, *u, aa_play::orders::guess::Fields { flags: 0x14, target: 0 }, b.selection.len() > 1));
                match u {
                    aa_play::Unit::Org(o) => {
                        if let Some(g) = w.orgs.get(*o as usize) {
                            let pos = g.members.iter().filter_map(|m| w.actor(*m)).find(|a| a.alive).map(|a| a.pos.truncate());
                            info!("stats {label}: selected org {o} {} {:?} at {pos:?} members {}", w.data.def(g.def).id, g.order, g.members.len());
                        }
                    }
                    aa_play::Unit::Actor(a) => {
                        if let Some(x) = w.actor(*a) {
                            info!("stats {label}: selected actor {a} {} {:?} at {:?} alive {}", w.data.def(x.def).id, x.order, x.pos.truncate(), x.alive);
                        }
                    }
                }
            }
        }
        s.next();
        return;
    }
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
            if let Some(slot) = what.strip_prefix("slot:") {
                let slot = u16::from_str_radix(slot.trim_start_matches("0x"), 16).unwrap_or(0);
                return match w.ids.thing_at_slot(slot) {
                    Some(aa_play::ids::Thing::Actor(x)) => x == id,
                    Some(aa_play::ids::Thing::Org(o)) => a.org == Some(o),
                    _ => false,
                };
            }
            match what {
                // a hostile object with a body (not a start-location marker),
                // units before buildings
                "enemy" => a.kingdom.is_some() && team(a.kingdom) != local_team && w.data.def(a.def).health > 0.0 && !building,
                "enemy_building" => a.kingdom.is_some() && team(a.kingdom) != local_team && building,
                "own_unit" => a.kingdom == Some(b.local) && a.org.is_some(),
                "own_building" => a.kingdom == Some(b.local) && building,
                // a mobile actor outside any organization (a truck)
                "own_truck" => a.kingdom == Some(b.local) && !building && a.org.is_none() && w.data.def(a.def).health > 0.0,
                // a member of a fighting regiment (not an HQ's organization)
                "own_regiment" => a.kingdom == Some(b.local) && a.org.is_some() && !building && !w.data.def(a.def).weapons.is_empty(),
                // a recruiting structure: a deployed division HQ (in its HQ
                // organization) or a lone one (the Corps HQ)
                "own_hq" => a.kingdom == Some(b.local) && building && a.org.is_some() && !w.data.def(a.def).recruits.is_empty(),
                "own_corps" => a.kingdom == Some(b.local) && building && a.org.is_none() && !w.data.def(a.def).recruits.is_empty(),
                // a city center (CityComponent)
                "city_center" => w.data.def(a.def).city,
                _ => false,
            }
        };
        if let Some(z) = step.get("zoom").and_then(|v| v.as_f64()) {
            requests.zoom = Some(z as f32);
            s.next();
            return;
        }
        // a native ground point
        let point = |key: &str| step.get(key).and_then(|v| v.as_array()).map(|a| Vec2::new(a.first().and_then(|x| x.as_f64()).unwrap_or(0.0) as f32, a.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32));
        if let Some(p) = point("look_at_point") {
            requests.look_at = Some(p);
            s.next();
            return;
        }
        for (key, out) in [("rclick_world", "rclick_at"), ("click_world", "click_at"), ("move_world", "move_at")] {
            let Some(p) = point(key) else { continue };
            let Ok((cam, cam_tf)) = cameras.single() else { return };
            let at = crate::battle_view::to_render(p.extend(w.map.height_at(p)));
            match cam.world_to_viewport(cam_tf, at) {
                Ok(v) => {
                    let f = v / logical;
                    info!("script: {key} {p:?} at {:.3},{:.3}", f.x, f.y);
                    let at = s.at;
                    s.steps[at] = serde_json::json!({ out: [f.x, f.y] });
                }
                Err(e) => {
                    warn!("script: {key} {p:?}: {e:?}");
                    s.next();
                }
            }
            return;
        }
        // the action button that shows a command
        if let Some(command) = kind("click_command") {
            let slot = front.as_ref().and_then(|f| f.hud.as_ref()).and_then(|h| {
                h.slot_command_list().iter().position(|c| c.as_ref().and_then(|c| super::objectives::native_text(w, c)).is_some_and(|t| t.eq_ignore_ascii_case(&command)))
            });
            let rect = slot.and_then(|i| {
                let name = if i == 0 { "ActionButtonsButton".to_string() } else { format!("ActionButtonsButton#{i}") };
                front.as_ref().and_then(|f| f.ui.widget_by(&format!("Game/ControlPanel/ActionButtons/{name}")).map(|x| x.root.outer.rect))
            });
            match rect {
                Some(r) => {
                    let (x, y) = ((r.l + r.r) * 0.5 / s.size.x, (r.t + r.b) * 0.5 / s.size.y);
                    info!("script: click_command {command} at {x:.3},{y:.3}");
                    let at = s.at;
                    s.steps[at] = serde_json::json!({ "click_at": [x, y] });
                }
                None => {
                    warn!("script: no action button shows {command}: {:?}", front.as_ref().and_then(|f| f.hud.as_ref()).map(|h| h.slot_command_list().iter().flatten().filter_map(|c| super::objectives::native_text(w, c)).collect::<Vec<_>>()));
                    s.next();
                }
            }
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
        for (key, out) in [("click_actor", "click_at"), ("rclick_actor", "rclick_at"), ("dclick_actor", "dclick_at"), ("move_actor", "move_at")] {
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
                    // "offset": [dx, dy] (window fractions) from the actor
                    let off = step.get("offset").and_then(|v| v.as_array()).map(|a| Vec2::new(a.first().and_then(|x| x.as_f64()).unwrap_or(0.0) as f32, a.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32)).unwrap_or(Vec2::ZERO);
                    let f = p / logical + off;
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
    } else if let Some(want) = step.get("until_tick").and_then(|v| v.as_u64()) {
        s.timer += dt;
        if battle.as_ref().is_some_and(|b| b.world.tick >= want) || s.timer > 600.0 {
            s.next();
        }
    } else if let Some(limit) = step.get("until_box").and_then(|v| v.as_f64()) {
        s.timer += dt;
        let up = front.as_ref().is_some_and(|f| f.ui.message_box.is_some());
        if up || s.timer > limit as f32 {
            if !up {
                warn!("script: no message box after {limit} s");
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
            quit.requested = true;
        }
    }
}
