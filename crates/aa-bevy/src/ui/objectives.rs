//! What the map's triggers show (aa_play::triggers): the game interface's
//! DisplayBannerPanel, the Counter, the objective and mission game events,
//! the trigger speech and sounds, and chat lines. Owner: scenarios.
//!
//! As the original's GameInterface (Init `0x6182B0`, update `0x612010`):
//! * `DisplayBanner "<text>" <seconds>` (50CD00 -> the command `0x60EB20`:
//!   seconds * 1000 (`[0x9F4978]`) to whole milliseconds, then `0x60D9D0`):
//!   an empty text hides the panel; otherwise the DisplayBanner label and
//!   the DisplayBannerBackground decal get alpha 1, the label the text, the
//!   interface's +19C the duration, +1A4 0, +1A8 the paused byte and +1A0
//!   the application timer, and the panel is shown. Each update (`0x612010`)
//!   accumulates the timer while the simulation runs; past the duration the
//!   panel fades over the AVar BannerFadeDurationMS (both alphas
//!   1 - elapsed / fade) and is hidden after it;
//! * the Counter (`UI/Game/time_message.tgi:Counter`, `0x5D26D0`) is added
//!   while the world has a counter (World+27C, `0x60B9D0`) and removed
//!   without one; its text (`0x5D27B0`) is `%d:%02d:%02d` of the duration
//!   minus the game time since its start (at least 0), or of the game time
//!   for a counter without duration;
//! * a 42D4B0 game event of the triggers goes to the game event manager
//!   (codes of the enum `0x4083F0`): the objective events (0x2A..0x31) of the
//!   local kingdom, the mission events (kingdom_defeated, kingdom_victory,
//!   team_defeated, team_victory: `global` in game_events.tgi) for every
//!   player, the event's text as its `%s`;
//! * speech (7476E0) and sounds (7474F0) to the sound queue; the chat lines
//!   (40BC50 / 40BF70, message type 0xF GENERIC_EVENT) to the message log.
//!
//! Compare against the original: the banner panel keeps its data height
//! (`0x60D9D0` measures the text, 735C00, and sets the panel height to it
//! plus a pad).
//!
//! The UI commands of the triggers (50D120 on the UI script): `MoveCamera`
//! (0x43DD10), `CameraFollow` (0x43E150) and `ShakeCamera` (0x43E020) go to the
//! camera (`crate::camera`), each ignored when it names another kingdom than
//! the local one; a cutscene's camera store and restore (0x6FE710 / 0x700A90)
//! too. Cutscene mode (4F1080) takes the game speed (76F370 with SVar
//! GameSpeed, the speed before it back at its end). The letterbox and the screen
//! fade (`aa_play::triggers::presentation`, read by 0x60B4C0 / 0x60B580 every
//! update) run as the world effects KKC_WELetterbox 0x5F35B0..0x5F3830 and the
//! fade 0x5F39E0 / 0x5F3B30: state 3 while they come in, 2 while they go, 1 on
//! (progress = time since the start / duration); the letterbox is two black
//! bars of progress * AVar LetterboxHeightRatio of the window, the fade a black
//! quad of alpha progress, under the interface; while the letterbox is on the
//! UI render level is 0x14 (`letterbox hiding UI`).
//!
//! The UI command heard by a script: the HUD's commands in the original's text
//! (`ActorCommand kill`, `ActorDataSelectCommand <ids>`, `FormationChange 0 1`:
//! [`native_text`]) go to 4F3FC0 (`aa_play::triggers::listening`) and, when a
//! script listens, as the order 61E420 to 54C170 (`script_heard`), as the
//! console does for every command it evaluates (0x426C20 -> 0x5C44D0).
//!
//! An actor's flash (50C3E0 -> 0x459600, the view's +98 and every member's):
//! [`ViewFlash`], read by `ui::selection` and `ui::banners`: 0x458ED0 runs the
//! object's focus VFX for UIVObjFlashTogglesWhenTargeted *
//! UIVObjFlashToggleSeconds, blinking with that period (0x42E670 -> 0x444460,
//! hidden while 0x4446F0's (now - start) / period is odd).
//!
//! PlayVideo (545C60 -> 0x411070) is `campaign/cinematic.rs`.
//!
//! Still not presented, each logged once when it happens: the
//! weather, a VFX on an actor (50D230, no shipped map uses it) and team command
//! flares' markers. The flash of a UI button
//! (50C880: the TutorialWidget of a command, 0x619BF0, virtual +B4 = 0x5EE2F0)
//! toggles the button's TARGETED state every AVar UIFlashToggleSeconds,
//! UIFlashTogglesWhenTargeted times, over the action button that shows the
//! command or the control panel's. The mission end comes to the front end
//! through the kingdoms' results (`ui::front` mission boxes).
use crate::session::Battle;
use bevy::prelude::*;

/// Enum `0x4083F0`: game event codes 0x00..0x3D.
const EVENTS: [&str; 0x3e] = [
    "hero_experience_gained_level",
    "building_construction_complete",
    "company_experience_gained_level",
    "kingdom_upkeep_shortage",
    "kingdom_upkeep_repaired",
    "technology_gained",
    "technology_lost",
    "building_under_siege",
    "building_repair_complete",
    "marker_discovered",
    "lair_reward",
    "research_complete",
    "company_routed",
    "company_engaged",
    "company_has_been_engaged",
    "actor_captured",
    "actor_captured_by_enemy",
    "actor_destroyed",
    "static_actor_lost",
    "mobile_actor_lost",
    "company_recruited",
    "company_commissioned",
    "hero_killed",
    "building_sold",
    "wall_breached",
    "my_flare_created",
    "ally_flare_created",
    "my_flare_created_on_actor",
    "ally_flare_created_on_actor",
    "resources_given",
    "resources_received",
    "actor_given",
    "actor_received",
    "resource_request_received",
    "actor_request_received",
    "building_invalid_placement",
    "static_actor_bombarded",
    "mobile_actor_bombarded",
    "bombard_target_lost_in_fog",
    "teleport_failed_no_destination",
    "teleport_failed_cannot_place",
    "militia_dispatched",
    "display_when_loading_primary_objective",
    "display_when_loading_secondary_objective",
    "remove_primary_objective",
    "remove_secondary_objective",
    "failed_primary_objective",
    "failed_secondary_objective",
    "complete_primary_objective",
    "complete_secondary_objective",
    "chat_sound",
    "kingdom_resigned",
    "kingdom_defeated",
    "kingdom_victory",
    "team_resigned",
    "team_defeated",
    "team_victory",
    "city_ratio_victory",
    "territory_positive",
    "territory_negative",
    "hq_attach",
    "hq_detach",
];

const BANNER_PANEL: &str = "Game/DisplayBannerPanel";
const BANNER_LABEL: &str = "Game/DisplayBannerPanel/DisplayBanner";
const BANNER_BACKGROUND: &str = "Game/DisplayBannerPanel/DisplayBannerBackground";
const COUNTER: &str = "UI/Game/time_message.tgi:Counter";

/// The interface's banner words (+19C duration, +1A0 last timer value, +1A4
/// accumulated, +1A8 previous paused byte) and the trigger notices handled.
#[derive(Resource, Default)]
pub struct ScenarioUi {
    seen: usize,
    banner_19c: u32,
    banner_1a0: u64,
    banner_1a4: u64,
    banner_1a8: bool,
    banner_visible: bool,
    counter_attached: bool,
    /// The world the notices belong to (a new battle starts over).
    world_tick: u64,
    /// KKC_WELetterbox+1C: the render level before the letterbox hid the UI.
    letterbox_level: Option<i32>,
    /// World+278: the game speed before the cutscene (4F1080).
    speed_before_cutscene: Option<f32>,
    /// What was logged as not presented (once per kind).
    logged: Vec<String>,
    /// The flashing commands and when each started (UI clock, ms).
    flashing: Vec<(String, u64)>,
    /// AA_BEVY_TRIGGER_LOG: the triggers' enabled flags last seen.
    trigger_log: Vec<(u32, String, bool)>,
}

/// The views' flash starts (+98, application seconds, 0x459600).
#[derive(Resource, Default)]
pub struct ViewFlash {
    pub actors: std::collections::HashMap<aa_play::world::ActorId, f32>,
    pub orgs: std::collections::HashMap<aa_play::world::OrgId, f32>,
    /// AVars UIVObjFlashToggleSeconds and UIVObjFlashTogglesWhenTargeted.
    pub period: f32,
    pub toggles: f32,
}

impl ViewFlash {
    /// 0x458ED0 and 0x4446F0: the focus VFX of a flash that started at `start` shows now.
    pub fn lit(&self, start: f32, now: f32) -> bool {
        let t = now - start;
        t < self.toggles * self.period && ((t / self.period.max(1e-3)) as i64) & 1 == 0
    }
}

/// The control panel's button of a command (game_interface's TutorialWidget
/// list, the buttons of hud.rs).
fn panel_button(command: &str) -> Option<&'static str> {
    Some(match command {
        "ObjectivesDisplayToggle" | "ButtonObjectives" => "Game/ControlPanel/ButtonObjectives",
        "TechnologyDisplayToggle" | "ButtonTechnology" => "Game/ControlPanel/ButtonTechnology",
        "MessageLogToggle" | "ButtonMessageLog" => "Game/ControlPanel/ButtonMessageLog",
        "PoliticsDisplayToggle" | "ButtonPolitics" => "Game/ControlPanel/ButtonPolitics",
        "ToggleShowZoneSupply" | "ButtonSupplyBorder" => "Game/ControlPanel/ButtonSupplyBorder",
        _ => return None,
    })
}

fn set_targeted(ui: &mut aa_ui::ui::Ui, path: &str, on: bool) {
    ui.with_widget(path, |w| {
        if on {
            w.flags |= aa_ui::state::widget_flag::TARGETED;
        } else {
            w.flags &= !aa_ui::state::widget_flag::TARGETED;
        }
    });
}

/// Whether a flash that started `elapsed_ms` ago is lit, and whether it still
/// runs (0x5EE2F0's period and count).
fn flash_state(elapsed_ms: u64, period_ms: u64, toggles: u64) -> (bool, bool) {
    let n = elapsed_ms / period_ms.max(1);
    (n % 2 == 0, n < toggles)
}

/// `4F3FC0` is `_wcsicmp`: the text the original's console would have
/// evaluated for a command of this HUD. `AAPlay` commands are this client's
/// spelling of the action panel's buttons (0x5F6860 `ActorCommand %s`,
/// 0x5F6D30 `ActorDataSelectCommand %s`, 0x5F6EA0 `PropertyDataSelectCommand %s`);
/// the formation buttons send `FormationChange %i %i` (0x628490, the second the
/// toggle's new state, 1 for a press).
pub fn native_text(world: &aa_play::World, raw: &str) -> Option<String> {
    let Some(rest) = raw.strip_prefix("AAPlay ") else {
        if let Some(rest) = raw.strip_prefix("FormationChange") {
            let id = rest.split_whitespace().next()?;
            return Some(format!("FormationChange {id} 1"));
        }
        return Some(raw.to_string());
    };
    let mut it = rest.split_whitespace();
    let verb = it.next()?;
    let arg = |i: usize| rest.split_whitespace().nth(i).and_then(|v| v.parse::<u32>().ok());
    let actor_command = |ids: &str| Some(format!("ActorCommand {ids}"));
    match verb {
        "recruit_page" => actor_command("recruit"),
        "construct_page" => actor_command("construct"),
        "research_page" => actor_command("research"),
        "set_rally_point" => actor_command("recruit_location"),
        "team_page" => Some("GoToTeamCommands".to_string()),
        "recruit" => Some(format!("ActorDataSelectCommand {}", world.data.defs.get(arg(2)? as usize)?.id)),
        "construct" => Some(format!("ActorDataSelectCommand {}", world.data.defs.get(arg(1)? as usize)?.id)),
        "research" => {
            let a = world.actor(arg(1)?)?;
            let r = world.data.def(a.def).research.get(arg(2)? as usize)?;
            Some(format!("PropertyDataSelectCommand {}", r.property))
        }
        "move" | "kill" | "retreat" | "guard" | "explore" | "bombard" | "repair" | "stop" | "air_attack" | "air_recon" | "hq_attach" | "hq_detach" | "unpack" | "pack" | "sell" | "disband" => actor_command(verb),
        _ => None,
    }
}

/// The kingdom a UI command of a trigger names is the local one (0x43DD10,
/// 0x43E150, 0x43E020 compare it with the local kingdom's name; without a local
/// kingdom, an observer, every command runs).
fn for_local(b: &Battle, kingdom: &str) -> bool {
    match b.local_kingdom().and_then(|k| aa_play::triggers::kingdom_script_name(&b.world, k)) {
        Some(name) => name == kingdom,
        None => true,
    }
}

/// A black quad of the given alpha over the rectangle.
fn black(rect: aa_ui::layout::Rect, alpha: f32, what: &str) -> aa_ui::draw::DrawCmd {
    aa_ui::draw::DrawCmd {
        widget: what.to_string(),
        ve: String::new(),
        kind: aa_ui::draw::DrawKind::Quad(aa_ui::draw::Quad {
            texture: None,
            rect,
            uv: [0.0, 0.0, 1.0, 1.0],
            color: [0.0, 0.0, 0.0, alpha],
            blend: aa_ui::draw::Blend::Alpha,
            address: [aa_ui::draw::Address::Clamp, aa_ui::draw::Address::Clamp],
            clip: None,
            effect: 0,
            tint: [0.0, 0.0, 0.0],
        }),
    }
}

/// The progress of a world effect (0x5F3D40): state 3 (0x5F3C20) up, state 2
/// (0x5F3CB0) down, over the duration from the start; at the end on (1) or off.
/// None: nothing to draw.
fn effect_progress(transition: aa_play::triggers::TransitionView, now: f32) -> Option<f32> {
    let (enabled, start, duration) = transition;
    let end = start + duration;
    if enabled {
        Some(if now <= end && duration > 0.0 { ((now - start) / duration).clamp(0.0, 1.0) } else { 1.0 })
    } else if start != 0.0 || duration != 0.0 {
        if now <= end && duration > 0.0 {
            Some(1.0 - ((now - start) / duration).clamp(0.0, 1.0))
        } else {
            None
        }
    } else {
        None
    }
}

fn set_alpha(ui: &mut aa_ui::ui::Ui, path: &str, alpha: f32) {
    ui.with_widget(path, |w| {
        for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
            match &mut ve.kind {
                aa_ui::ve::VeKind::Label(l) => l.color[3] = alpha,
                aa_ui::ve::VeKind::Decal(d) => d.color[3] = alpha,
                _ => {}
            }
        }
    });
}

/// `0x5EFB00` and the layout that follows a visibility change.
fn set_visible(ui: &mut aa_ui::ui::Ui, path: &str, visible: bool) {
    if !ui.with_widget(path, |w| w.set_visible(visible)) {
        warn!("scenario UI: no widget {path}");
    }
    ui.layout();
}

/// `0x60D9D0`.
fn show_banner(s: &mut ScenarioUi, ui: &mut aa_ui::ui::Ui, text: &str, ms: u32, paused: bool) {
    if text.is_empty() {
        set_visible(ui, BANNER_PANEL, false);
        s.banner_visible = false;
        return;
    }
    info!("display banner ({ms} ms): {text}");
    set_alpha(ui, BANNER_LABEL, 1.0);
    ui.set_label(BANNER_LABEL, text);
    set_alpha(ui, BANNER_BACKGROUND, 1.0);
    s.banner_19c = ms;
    s.banner_1a4 = 0;
    s.banner_1a8 = paused;
    s.banner_1a0 = ui.now_ms;
    set_visible(ui, BANNER_PANEL, true);
    s.banner_visible = true;
}

/// The banner part of `0x612010`.
fn fade_banner(s: &mut ScenarioUi, ui: &mut aa_ui::ui::Ui, paused: bool) {
    if !s.banner_visible {
        return;
    }
    let now = ui.now_ms;
    let delta = if s.banner_1a8 { 0 } else { now.wrapping_sub(s.banner_1a0) };
    s.banner_1a4 = s.banner_1a4.wrapping_add(delta);
    s.banner_1a0 = now;
    s.banner_1a8 = paused;
    if s.banner_1a4 <= u64::from(s.banner_19c) {
        return;
    }
    let elapsed = (s.banner_1a4 - u64::from(s.banner_19c)) as f32;
    let fade = ui.avars.get("BannerFadeDurationMS").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(1000.0);
    if elapsed > fade {
        set_visible(ui, BANNER_PANEL, false);
        s.banner_visible = false;
    } else {
        let alpha = 1.0 - elapsed / fade;
        set_alpha(ui, BANNER_BACKGROUND, alpha);
        set_alpha(ui, BANNER_LABEL, alpha);
    }
}

fn log_once(logged: &mut Vec<String>, what: String) {
    if !logged.contains(&what) {
        warn!("{what}");
        logged.push(what);
    }
}

/// The session's difficulty (the DifficultyLevel world rule of the staging
/// screen or the campaign's choice, *(GameVars+104)), applied to the next
/// battle before its first tick; read by difficulty_compare 17.
#[derive(Resource, Clone, Debug)]
pub struct PendingDifficulty(pub String);

/// Battle start: [`PendingDifficulty`].
pub fn start(mut battle: Option<ResMut<Battle>>, pending: Option<Res<PendingDifficulty>>, front: Res<super::front::Front>, mut commands: Commands) {
    let (Some(b), Some(p)) = (battle.as_mut(), pending.as_deref()) else { return };
    if !b.is_added() {
        return;
    }
    if let Some(game_root) = front.root.parent()
        && !aa_play::triggers::set_difficulty_ids(&mut b.world, &p.0, game_root)
    {
        warn!("no DifficultyLevel {}", p.0);
    }
    commands.remove_resource::<PendingDifficulty>();
}

/// `0x5D27B0` of the Counter.
fn counter_text(now: f32, start: f32, duration: f32) -> String {
    let t = if duration > 0.0 { (duration - (now - start)).max(0.0) } else { now };
    let s = t as i32;
    format!("{}:{:02}:{:02}", s / 60 / 60, (s / 60) % 60, s % 60)
}

/// One frame: the new trigger notices and the banner / counter updates.
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut s: ResMut<ScenarioUi>,
    mut front: ResMut<super::front::Front>,
    battle: Option<ResMut<Battle>>,
    sfx: Option<ResMut<crate::audio::SfxQueue>>,
    mut camera: ResMut<crate::camera::CameraRequests>,
    animations: Option<ResMut<crate::session::ObjectAnimations>>,
    mut marks: ResMut<super::order_marks::PointVfx>,
    mut draw: ResMut<super::render::UiDraw>,
    mut flash: ResMut<ViewFlash>,
    time: Res<Time>,
) {
    let Some(mut b) = battle else { return };
    if front.page != super::front::Page::Game || front.hud.is_none() {
        return;
    }
    // The selection conditions 18 / 19 read the local player's selection. A film's
    // viewer has none: its selection stays out of the film world's script.
    let (local, selection) = (b.local, b.selection.clone());
    if b.local_kingdom().is_some() {
        aa_play::triggers::set_selection(&mut b.world, local, selection);
    }
    if b.world.tick < s.world_tick {
        *s = ScenarioUi::default();
    }
    s.world_tick = b.world.tick;
    let paused = b.paused;
    let front = &mut *front;
    let notices = aa_play::triggers::notices(&b.world);
    if notices.len() < s.seen {
        s.seen = 0;
    }
    let new = notices[s.seen..].to_vec();
    s.seen = notices.len();
    // verification aid: AA_BEVY_TRIGGER_LOG=1 logs each trigger that turns on or off
    if std::env::var_os("AA_BEVY_TRIGGER_LOG").is_some() {
        let states = aa_play::triggers::trigger_states(&b.world);
        for t in &states {
            if s.trigger_log.iter().find(|o| o.0 == t.0).is_none_or(|o| o.2 != t.2) {
                info!("trigger {} '{}' {} at {:.1}", t.0, t.1, if t.2 { "on" } else { "off" }, b.world.time);
            }
        }
        s.trigger_log = states;
    }
    let mut sfx = sfx;
    let mut animations = animations;
    // the commands the HUD ran this frame, as the console's 0x5C44D0 sees them
    let heard: Vec<String> = front.hud.as_mut().map(|h| std::mem::take(&mut h.heard)).unwrap_or_default();
    for raw in heard {
        if let Some(text) = native_text(&b.world, &raw)
            && aa_play::triggers::listening(&b.world, &text)
        {
            info!("the script hears {text}");
            aa_play::triggers::script_heard(&mut b.world, &text);
        }
    }
    for n in &new {
        match n {
            aa_play::triggers::Notice::UiCommand(command) => {
                let word = command.split_whitespace().next().unwrap_or("");
                match word {
                    "MoveCamera" => match aa_play::triggers::parse_move_camera(command) {
                        Some(m) if for_local(&b, &m.kingdom) => {
                            // 0x43DD10: the command's zoom divides 1, its angles
                            // are in degrees
                            camera.script.push(crate::camera::ScriptRequest::Move {
                                until: m.until,
                                realtime: m.realtime,
                                at: m.at,
                                zoom: 1.0 / m.zoom,
                                pitch: m.pitch * 0.017_453_292,
                                yaw: m.yaw * 0.017_453_292,
                                ease: m.ease,
                            });
                        }
                        Some(m) => info!("MoveCamera ignored: kingdom '{}'", m.kingdom),
                        None => warn!("trigger UI command does not parse: {command}"),
                    },
                    "CameraFollow" => {
                        let mut it = command.split_whitespace().skip(1);
                        let id = it.next().and_then(|v| v.parse::<i64>().ok()).map(|v| v as u32);
                        let kingdom = it.next().unwrap_or("");
                        match id.and_then(|raw| b.world.ids.thing(raw)) {
                            Some(aa_play::ids::Thing::Actor(a)) if for_local(&b, kingdom) => camera.script.push(crate::camera::ScriptRequest::Follow(aa_play::Unit::Actor(a))),
                            Some(aa_play::ids::Thing::Org(o)) if for_local(&b, kingdom) => camera.script.push(crate::camera::ScriptRequest::Follow(aa_play::Unit::Org(o))),
                            _ => {}
                        }
                    }
                    "ShakeCamera" => {
                        let mut it = command.split_whitespace().skip(1);
                        let amount = it.next().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
                        if for_local(&b, it.next().unwrap_or("")) {
                            camera.script.push(crate::camera::ScriptRequest::Shake(amount));
                        }
                    }
                    // 545C60 -> 0x411070: `PlayVideo <file> <widget> <end command> 0`
                    "PlayVideo" => {
                        if let Some(file) = command.split_whitespace().nth(1) {
                            front.campaign.movie = Some(crate::campaign::cinematic::stem(file));
                        }
                    }
                    other => log_once(&mut s.logged, format!("the trigger UI command {other} is not presented")),
                }
            }
            aa_play::triggers::Notice::CameraStore => camera.script.push(crate::camera::ScriptRequest::Store),
            aa_play::triggers::Notice::CameraRestore => camera.script.push(crate::camera::ScriptRequest::Restore),
            // 4F1080: the speed changes to SVar GameSpeed, and back at the end
            aa_play::triggers::Notice::CutsceneMode(on) => {
                if *on {
                    s.speed_before_cutscene = Some(b.speed);
                    let exponent = b.world.data.rule_ci("GameSpeed").unwrap_or(0.0);
                    b.speed = 2f32.powf(exponent);
                } else if let Some(speed) = s.speed_before_cutscene.take() {
                    b.speed = speed;
                }
            }
            // 50C3E0 -> 0x459600: the view's time and every member's
            aa_play::triggers::Notice::FlashActor(unit) => {
                let now = time.elapsed_secs();
                let avar = |ui: &aa_ui::ui::Ui, k: &str, d: f32| ui.avars.get(k).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(d);
                flash.period = avar(&front.ui, "UIVObjFlashToggleSeconds", 0.25);
                flash.toggles = avar(&front.ui, "UIVObjFlashTogglesWhenTargeted", 8.0);
                match *unit {
                    aa_play::Unit::Actor(a) => {
                        flash.actors.insert(a, now);
                    }
                    aa_play::Unit::Org(o) => {
                        flash.orgs.insert(o, now);
                        if let Some(org) = b.world.orgs.get(o as usize) {
                            for m in &org.members {
                                flash.actors.insert(*m, now);
                            }
                        }
                    }
                }
            }
            // what the world or the front end takes care of; a trigger's ability
            // shows through its Event::SpecialOperation (effects)
            aa_play::triggers::Notice::MissionEnded { .. } | aa_play::triggers::Notice::SaiProfile { .. } | aa_play::triggers::Notice::Ability { .. } => {}
            // 50C880: again while it runs is a restart (0x5EE2F0 stores the time)
            aa_play::triggers::Notice::FlashButton(command) => {
                let now = front.ui.now_ms;
                match s.flashing.iter_mut().find(|f| f.0 == *command) {
                    Some(f) => f.1 = now,
                    None => s.flashing.push((command.clone(), now)),
                }
            }
            // 7476E0's music voice (747630): the same queue, its sound's category
            aa_play::triggers::Notice::Music(Some(ids)) => {
                if let Some(q) = sfx.as_mut() {
                    q.play.push(crate::audio::Sfx { ids: ids.clone(), place: crate::audio::Place::Global, owner: None, voice: None, looping: true });
                }
            }
            aa_play::triggers::Notice::Music(None) => log_once(&mut s.logged, "stopping the game music from a trigger is not presented".into()),
            // 543BA0: the transport on its way (the sim's Drop lands the cargo)
            aa_play::triggers::Notice::Transport { kingdom, ids, from, to, seconds } => {
                marks.transports.push(super::order_marks::TransportRun { kingdom: *kingdom, ids: ids.clone(), from: *from, to: *to, seconds: *seconds });
            }
            // 50D280: the VFX at its point
            aa_play::triggers::Notice::Vfx { ids, at, .. } => {
                let ground = b.world.map.height_at(Vec2::new(at[0], at[1]));
                marks.requests.push(super::order_marks::PointVfxRequest { vfx_ids: ids.clone(), at: Vec2::new(at[0], at[1]), z_offset: at[2] - ground });
            }
            // 4CE730: the actor's animation of that name
            aa_play::triggers::Notice::Animation { actor: aa_play::Unit::Actor(a), name } => {
                if let Some(m) = animations.as_mut() {
                    m.0.insert(*a, (name.to_ascii_lowercase(), b.world.time));
                }
            }
            aa_play::triggers::Notice::Animation { actor: aa_play::Unit::Org(o), name } => {
                if let (Some(m), Some(org)) = (animations.as_mut(), b.world.orgs.get(*o as usize)) {
                    for a in &org.members {
                        m.0.insert(*a, (name.to_ascii_lowercase(), b.world.time));
                    }
                }
            }
            aa_play::triggers::Notice::Banner { text, seconds } => {
                // fmul [0x9F4978] (1000.0), _ftol2
                let ms = (seconds * 1000.0) as i32;
                show_banner(&mut s, &mut front.ui, text, ms.max(0) as u32, paused);
            }
            aa_play::triggers::Notice::GameEvent { code, kingdom, text } => {
                let Some(name) = EVENTS.get(*code as usize) else { continue };
                let objective = (0x2a..=0x31).contains(code);
                if objective && *kingdom != local {
                    continue;
                }
                let text = if text.starts_with('#') { front.ui.strings.resolve(text) } else { text.clone() };
                let Some(h) = front.hud.as_mut() else { continue };
                // the kingdom's display name for the mission events
                let arg = if objective { text } else { h.names.kingdom(&text).to_string() };
                h.events.fire(&mut front.ui, name, &[arg.as_str()], None, Some(*kingdom));
            }
            aa_play::triggers::Notice::Chat { kingdom, text } => {
                if kingdom.is_none_or(|k| k == local)
                    && let Some(h) = front.hud.as_mut()
                {
                    h.events.send_message(&mut front.ui, "GENERIC_EVENT", text, None);
                }
            }
            aa_play::triggers::Notice::Speech(ids) => {
                if let Some(q) = sfx.as_mut() {
                    q.play.push(crate::audio::Sfx { ids: ids.clone(), place: crate::audio::Place::Global, owner: None, voice: None, looping: false });
                }
            }
            aa_play::triggers::Notice::Sound { ids, at } => {
                if let Some(q) = sfx.as_mut() {
                    let place = match at {
                        Some(p) => crate::audio::Place::Location(Vec3::new(p[0], p[1], p[2])),
                        None => crate::audio::Place::Global,
                    };
                    q.play.push(crate::audio::Sfx { ids: ids.clone(), place, owner: None, voice: None, looping: false });
                }
            }
            // 76F370 (game_speed_set 68): the speed exponent
            aa_play::triggers::Notice::GameSpeed(exponent) => b.speed = 2f32.powf(*exponent).clamp(0.25, 4.0),
            other => {
                // the Debug text up to its fields: one line per kind
                let text = format!("{other:?}");
                let kind = text.split(['(', ' ']).next().unwrap_or("").to_string();
                log_once(&mut s.logged, format!("the trigger notice {kind} is not presented: {text}"));
            }
        }
    }
    fade_banner(&mut s, &mut front.ui, paused);
    // the flashes (UIFlashToggleSeconds, UIFlashTogglesWhenTargeted)
    if !s.flashing.is_empty() {
        let period = front.ui.avars.get("UIFlashToggleSeconds").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.5);
        let period = (period * 1000.0) as u64;
        let toggles = front.ui.avars.get("UIFlashTogglesWhenTargeted").and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(16);
        let now = front.ui.now_ms;
        let slots: Vec<(usize, Option<String>)> = front.hud.as_ref().map(|h| h.slot_command_list().into_iter().enumerate().collect()).unwrap_or_default();
        let list = std::mem::take(&mut s.flashing);
        for (command, start) in list {
            let (lit, running) = flash_state(now.saturating_sub(start), period, toggles);
            let on = lit && running;
            if let Some(path) = panel_button(&command) {
                set_targeted(&mut front.ui, path, on);
            }
            for (i, slot) in &slots {
                if slot.as_ref().and_then(|c| native_text(&b.world, c)).is_some_and(|t| t.eq_ignore_ascii_case(&command)) {
                    let name = if *i == 0 { "ActionButtonsButton".to_string() } else { format!("ActionButtonsButton#{i}") };
                    set_targeted(&mut front.ui, &format!("Game/ControlPanel/ActionButtons/{name}"), on);
                }
            }
            if running {
                s.flashing.push((command, start));
            }
        }
    }
    // 0x60B4C0 / 0x60B580: the letterbox and the fade follow World+BC..+D0
    if let Some((letterbox, fade)) = aa_play::triggers::presentation(&b.world) {
        let now = b.world.time + crate::session::tick_fraction(&b) * aa_play::DT;
        let (width, height) = (draw.list.screen[0] as f32, draw.list.screen[1] as f32);
        let mut bars = Vec::new();
        // 0x5F3830: LetterboxHeightRatio (hard-coded AVar 0.22222) of the window
        if let Some(progress) = effect_progress(letterbox, now) {
            let ratio = front.ui.avars.get("LetterboxHeightRatio").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.22222);
            let h = progress * ratio * height;
            bars.push(black(aa_ui::layout::Rect { l: 0.0, t: 0.0, r: width, b: h }, 1.0, "WELetterbox"));
            bars.push(black(aa_ui::layout::Rect { l: 0.0, t: height - h, r: width, b: height }, 1.0, "WELetterbox"));
        }
        // 0x5F3B30: the fade's quad alpha
        if let Some(progress) = effect_progress(fade, now) {
            bars.push(black(aa_ui::layout::Rect { l: 0.0, t: 0.0, r: width, b: height }, progress, "WEFade"));
        }
        for (i, q) in bars.into_iter().enumerate() {
            draw.list.commands.insert(i, q);
        }
        // 0x5F35B0: on or coming in hides the UI (level 0x14); going out or off
        // gives it back (0x5F3620)
        let showing = letterbox.0;
        match (showing, s.letterbox_level) {
            (true, None) => {
                s.letterbox_level = Some(front.ui.render_level);
                front.ui.render_level = 0x14;
            }
            (false, Some(level)) => {
                front.ui.render_level = level;
                s.letterbox_level = None;
            }
            _ => {}
        }
    }
    // 0x60B9D0: the Counter while the world has one.
    match aa_play::triggers::counter(&b.world) {
        Some((start, duration)) => {
            if !s.counter_attached {
                aa_ui::screens::show(&mut front.ui, aa_ui::widget::Widget::new(aa_ui::widget::WidgetClass::Label, COUNTER, false));
                front.ui.layout();
                s.counter_attached = true;
            }
            let text = counter_text(b.world.time, start, duration);
            front.ui.set_label("Counter", &text);
        }
        None => {
            if s.counter_attached {
                front.ui.disconnect("Counter");
                front.ui.layout();
                s.counter_attached = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_flash_toggles_and_ends() {
        assert_eq!(super::flash_state(0, 500, 16), (true, true));
        assert_eq!(super::flash_state(600, 500, 16), (false, true));
        assert_eq!(super::flash_state(1000, 500, 16), (true, true));
        assert!(!super::flash_state(8000, 500, 16).1);
    }

    #[test]
    fn world_effects_come_in_and_go_out() {
        // 0x5F3D40: off, then state 3 up over the duration, then on
        assert_eq!(super::effect_progress((false, 0.0, 0.0), 5.0), None);
        assert_eq!(super::effect_progress((true, 10.0, 2.0), 11.0), Some(0.5));
        assert_eq!(super::effect_progress((true, 10.0, 2.0), 20.0), Some(1.0));
        assert_eq!(super::effect_progress((true, 10.0, 0.0), 10.0), Some(1.0));
        // state 2 down, then off
        assert_eq!(super::effect_progress((false, 10.0, 2.0), 11.0), Some(0.5));
        assert_eq!(super::effect_progress((false, 10.0, 2.0), 13.0), None);
        assert_eq!(super::effect_progress((false, 10.0, 0.0), 10.0), None);
    }

    #[test]
    fn counter_counts_down_to_zero_and_up_without_duration() {
        assert_eq!(super::counter_text(10.0, 0.0, 3725.5), "1:01:55");
        assert_eq!(super::counter_text(100.0, 0.0, 30.0), "0:00:00");
        assert_eq!(super::counter_text(61.9, 0.0, 0.0), "0:01:01");
    }
}
