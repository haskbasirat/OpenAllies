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
//! Not presented yet, each logged when it happens: the scripted camera
//! (MoveCamera, CameraFollow, ShakeCamera; `aa_play::triggers::parse_move_camera`
//! reads the first), PlayVideo, cutscene mode, letterbox and screen fade
//! (`aa_play::triggers::presentation`), weather, music, flashes, VFX, flares
//! and actor animations. The mission end comes to the front end through the
//! kingdoms' results (`ui::front` mission boxes).
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

/// `0x5D27B0` of the Counter.
fn counter_text(now: f32, start: f32, duration: f32) -> String {
    let t = if duration > 0.0 { (duration - (now - start)).max(0.0) } else { now };
    let s = t as i32;
    format!("{}:{:02}:{:02}", s / 60 / 60, (s / 60) % 60, s % 60)
}

/// One frame: the new trigger notices and the banner / counter updates.
pub fn update(
    mut s: ResMut<ScenarioUi>,
    mut front: ResMut<super::front::Front>,
    battle: Option<ResMut<Battle>>,
    sfx: Option<ResMut<crate::audio::SfxQueue>>,
) {
    let Some(mut b) = battle else { return };
    if front.page != super::front::Page::Game || front.hud.is_none() {
        return;
    }
    // The selection conditions 18 / 19 read the local player's selection.
    let (local, selection) = (b.local, b.selection.clone());
    aa_play::triggers::set_selection(&mut b.world, local, selection);
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
    let mut sfx = sfx;
    for n in &new {
        match n {
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
            other => info!("trigger notice not presented yet: {other:?}"),
        }
    }
    fade_banner(&mut s, &mut front.ui, paused);
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
    fn counter_counts_down_to_zero_and_up_without_duration() {
        assert_eq!(super::counter_text(10.0, 0.0, 3725.5), "1:01:55");
        assert_eq!(super::counter_text(100.0, 0.0, 30.0), "0:00:00");
        assert_eq!(super::counter_text(61.9, 0.0, 0.0), "0:01:01");
    }
}
