//! Game events (KKC_GameEventManager, `.\Source\View\GameEventManager.cpp`,
//! AddEvent `0x42D130`; the events Data/Game/game_events.tgi, their enum
//! `0x4083F0`). Owner: UI.
//!
//! An event of the local kingdom (or a `global` one):
//! * is dropped while its `filter` UVar is false (UVars.tgi:
//!   UIEventFilterSiege, UIEventFilterProduction true, UIEventFilterEngagement
//!   false, so routs and engagements are not announced by default);
//! * shows its `game_message` (the localized string, `%s` arguments) in the
//!   message log as its `message_type`, unless its `message_filter` UVar is
//!   false; the message system's send (`0x40B5C0`) plays the type's
//!   `message_sound` in the battle (message_types.tgi: GENERIC_ERROR `error`);
//! * plays its AdditionalFX: `FXC_GameEventSFX` (its `audio_ids` at the
//!   event's ground point, SOURCE_GROUND) and `FXC_CreateRFX` (a minimap
//!   ping: `radar_icon` blinking from `src_color` to `dst_color` for
//!   `blink_count` object refreshes, `0x434AD0`).
use super::datafile;
use bevy::prelude::*;
use std::collections::HashMap;

/// A minimap ping (`FXC_CreateRFX`).
#[derive(Clone, Debug, PartialEq)]
pub struct Ping {
    pub icon: String,
    pub src: [f32; 3],
    pub dst: [f32; 3],
    pub blinks: u32,
    /// Native ground point.
    pub at: Vec2,
}

/// A sound of an event (`FXC_GameEventSFX`).
#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    pub ids: String,
    /// Native ground point; None: 2-D.
    pub at: Option<Vec2>,
    /// The event's kingdom (the sound's who_flags).
    pub owner: Option<u32>,
}

#[derive(Clone, Debug, Default)]
struct EventDef {
    message_type: String,
    message: Option<String>,
    filter: Option<String>,
    message_filter: Option<String>,
    sounds: Vec<String>,
    /// (radar_icon, src_color, dst_color, blink_count)
    pings: Vec<(String, [f32; 3], [f32; 3], u32)>,
}

/// The event definitions and this frame's effects for the minimap and the
/// sound.
#[derive(Default)]
pub struct GameEvents {
    defs: HashMap<String, EventDef>,
    message_sounds: HashMap<String, String>,
    loaded: bool,
    /// The newest message id whose sound was played.
    last_message: u32,
    pub pings: Vec<Ping>,
    pub sounds: Vec<Sound>,
}

fn point3(v: &str) -> Option<[f32; 3]> {
    let p: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    (p.len() >= 3).then(|| [p[0], p[1], p[2]])
}

fn some(v: Option<&str>) -> Option<String> {
    v.map(str::trim).filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null")).map(str::to_string)
}

impl GameEvents {
    fn load(&mut self, data: &aa_ui::fs::DataRoot) {
        self.loaded = true;
        for n in datafile::load(data, "Game/game_events.tgi").iter().filter(|n| n.name.eq_ignore_ascii_case("GameEvent")) {
            let Some(ids) = some(n.get("IDS")) else { continue };
            let mut def = EventDef {
                message_type: n.get("message_type").unwrap_or("GENERIC_EVENT").trim().to_ascii_uppercase(),
                message: some(n.get("game_message|*")),
                filter: some(n.get("filter")),
                message_filter: some(n.get("message_filter")),
                ..Default::default()
            };
            for fx in n.children_named("AdditionalFX") {
                match fx.template().map(str::to_ascii_lowercase).as_deref() {
                    Some("fxc_gameeventsfx") => def.sounds.extend(some(fx.get("audio_ids"))),
                    Some("fxc_createrfx") => {
                        if let Some(icon) = some(fx.get("radar_icon")) {
                            // template_fx.tgi defaults: yellow to white, 3 blinks
                            let src = fx.get("src_color").and_then(point3).unwrap_or([1.0, 1.0, 0.0]);
                            let dst = fx.get("dst_color").and_then(point3).unwrap_or([1.0, 1.0, 1.0]);
                            let blinks = fx.int("blink_count").unwrap_or(3).max(0) as u32;
                            def.pings.push((icon, src, dst, blinks));
                        }
                    }
                    _ => {}
                }
            }
            self.defs.insert(ids.to_ascii_lowercase(), def);
        }
        for n in datafile::load(data, "Game/message_types.tgi").iter().filter(|n| n.name.eq_ignore_ascii_case("MessageType")) {
            if let (Some(ids), Some(sound)) = (some(n.get("IDS")), some(n.get("message_sound"))) {
                self.message_sounds.insert(ids.to_ascii_uppercase(), sound);
            }
        }
    }

    /// Send a message (`0x40B5C0`).
    pub fn send_message(&mut self, ui: &mut aa_ui::ui::Ui, kind: &str, text: &str, at: Option<(f32, f32)>) {
        let uv = ui.uvars.clone();
        ui.messages.send(&uv, kind, text, at);
    }

    /// `0x40B5C0` in the battle: every message sent since the last call
    /// (by any sender) plays its type's `message_sound` (2-D).
    pub fn message_sounds(&mut self, ui: &aa_ui::ui::Ui) {
        if !self.loaded {
            let data = ui.store.root.clone();
            self.load(&data);
        }
        let last = ui.messages.last;
        if last < self.last_message {
            self.last_message = last;
        }
        for m in ui.messages.messages.iter().filter(|m| m.id > self.last_message) {
            if let Some(s) = self.message_sounds.get(&m.kind) {
                self.sounds.push(Sound { ids: s.clone(), at: None, owner: None });
            }
        }
        self.last_message = last;
    }

    /// AddEvent for the local player: `args` fill the message's `%s`.
    pub fn fire(&mut self, ui: &mut aa_ui::ui::Ui, ids: &str, args: &[&str], at: Option<Vec2>, owner: Option<u32>) {
        if !self.loaded {
            let data = ui.store.root.clone();
            self.load(&data);
        }
        let Some(def) = self.defs.get(&ids.to_ascii_lowercase()).cloned() else {
            warn!("game event {ids} is not in game_events.tgi");
            return;
        };
        let on = |name: &Option<String>| name.as_ref().is_none_or(|f| ui.uvars.def(f).is_none() || ui.uvars.get_bool(f));
        if !on(&def.filter) {
            return;
        }
        if let Some(m) = &def.message
            && on(&def.message_filter)
        {
            let mut text = ui.strings.resolve(m);
            // printf arguments in order (%s or %d), then %% -> %
            for a in args {
                let at = [text.find("%s"), text.find("%d")].into_iter().flatten().min();
                if let Some(i) = at {
                    text.replace_range(i..i + 2, a);
                }
            }
            let text = text.replace("%%", "%");
            self.send_message(ui, &def.message_type, &text, at.map(|p| (p.x, p.y)));
        }
        for s in &def.sounds {
            self.sounds.push(Sound { ids: s.clone(), at, owner });
        }
        if let Some(at) = at {
            for (icon, src, dst, blinks) in &def.pings {
                self.pings.push(Ping { icon: icon.clone(), src: *src, dst: *dst, blinks: *blinks, at });
            }
        }
    }
}
