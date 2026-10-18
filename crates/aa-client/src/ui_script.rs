//! Scripted user input for end-to-end testing (`--ui-script steps.json`).
//! Steps inject real egui pointer/keyboard events through eframe's raw input
//! hook, so menus, selection and orders are exercised exactly as by a player.
//!
//! Step format (JSON array), each with an optional "wait" (seconds before it):
//!   {"click": [x, y], "button": "left"|"right"|"middle", "mods": "shift|ctrl|alt"}
//!   {"drag": [x0, y0, x1, y1]}            left-button drag (box select)
//!   {"move": [x, y]}
//!   {"key": "F2", "mods": "ctrl"}         egui key name
//!   {"text": "Hello"}
//!   {"scroll": -300}
//!   {"shot": "path.png"}                  full-window screenshot
//!   {"until_game_time": 120}              wait until the battle clock reaches t
//!   {"quit": true}
//! Coordinates are logical points in the window.

use eframe::egui;
use serde::Deserialize;
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Step {
    pub wait: f32,
    pub click: Option<[f32; 2]>,
    pub button: Option<String>,
    pub mods: Option<String>,
    pub drag: Option<[f32; 4]>,
    #[serde(rename = "move")]
    pub move_to: Option<[f32; 2]>,
    pub key: Option<String>,
    pub text: Option<String>,
    pub scroll: Option<f32>,
    pub shot: Option<PathBuf>,
    pub until_game_time: Option<f32>,
    pub quit: bool,
    pub log: Option<String>,
}

pub struct Script {
    steps: VecDeque<Step>,
    timer: f32,
    /// Pending low-level events queued by the current step (one per frame).
    queue: VecDeque<egui::Event>,
    pub shot_request: Option<PathBuf>,
    pub quit: bool,
    pub game_time: f32,
}

fn modifiers(s: &Option<String>) -> egui::Modifiers {
    let mut m = egui::Modifiers::NONE;
    if let Some(s) = s {
        for part in s.split(['+', '|', ',']) {
            match part.trim().to_ascii_lowercase().as_str() {
                "shift" => m.shift = true,
                "ctrl" | "control" => {
                    m.ctrl = true;
                    m.command = true;
                }
                "alt" => m.alt = true,
                _ => {}
            }
        }
    }
    m
}

impl Script {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let steps: Vec<Step> = serde_json::from_slice(&std::fs::read(path)?)?;
        Ok(Self {
            steps: steps.into(),
            timer: 0.0,
            queue: VecDeque::new(),
            shot_request: None,
            quit: false,
            game_time: 0.0,
        })
    }

    fn button(b: &Option<String>) -> egui::PointerButton {
        match b.as_deref() {
            Some("right") => egui::PointerButton::Secondary,
            Some("middle") => egui::PointerButton::Middle,
            _ => egui::PointerButton::Primary,
        }
    }

    /// Called from eframe's raw input hook every frame.
    pub fn feed(&mut self, raw: &mut egui::RawInput, dt: f32) {
        if let Some(e) = self.queue.pop_front() {
            if let egui::Event::PointerButton { modifiers, .. } | egui::Event::Key { modifiers, .. } = &e {
                raw.modifiers = *modifiers;
            }
            raw.events.push(e);
            return;
        }
        let Some(step) = self.steps.front().cloned() else { return };
        if let Some(t) = step.until_game_time
            && self.game_time < t
        {
            return;
        }
        self.timer += dt;
        if self.timer < step.wait {
            return;
        }
        self.timer = 0.0;
        self.steps.pop_front();
        if let Some(l) = &step.log {
            println!("ui-script: {l}");
        }
        let mods = modifiers(&step.mods);
        if let Some([x, y]) = step.move_to {
            self.queue.push_back(egui::Event::PointerMoved(egui::pos2(x, y)));
        }
        if let Some([x, y]) = step.click {
            let pos = egui::pos2(x, y);
            let button = Self::button(&step.button);
            self.queue.push_back(egui::Event::PointerMoved(pos));
            self.queue.push_back(egui::Event::PointerButton { pos, button, pressed: true, modifiers: mods });
            self.queue.push_back(egui::Event::PointerButton { pos, button, pressed: false, modifiers: mods });
        }
        if let Some([x0, y0, x1, y1]) = step.drag {
            let a = egui::pos2(x0, y0);
            let b = egui::pos2(x1, y1);
            self.queue.push_back(egui::Event::PointerMoved(a));
            self.queue.push_back(egui::Event::PointerButton { pos: a, button: egui::PointerButton::Primary, pressed: true, modifiers: mods });
            for i in 1..=8 {
                self.queue.push_back(egui::Event::PointerMoved(a + (b - a) * (i as f32 / 8.0)));
            }
            self.queue.push_back(egui::Event::PointerButton { pos: b, button: egui::PointerButton::Primary, pressed: false, modifiers: mods });
        }
        if let Some(k) = &step.key
            && let Some(key) = egui::Key::from_name(k)
        {
            self.queue.push_back(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: mods });
            self.queue.push_back(egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: mods });
        }
        if let Some(t) = &step.text {
            self.queue.push_back(egui::Event::Text(t.clone()));
        }
        if let Some(s) = step.scroll {
            self.queue.push_back(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, s),
                modifiers: mods,
            });
        }
        if let Some(p) = step.shot {
            self.shot_request = Some(p);
        }
        if step.quit {
            self.quit = true;
        }
    }

    pub fn finished(&self) -> bool {
        self.steps.is_empty() && self.queue.is_empty()
    }
}
