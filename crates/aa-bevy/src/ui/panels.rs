//! The game interface's owned displays, connected to the UI root while
//! open (GameInterface `0x6182B0` creates them, the toggles add / remove
//! them: `0x60F310` ObjectivesDisplayToggle, `0x60F3D0`
//! TechnologyDisplayToggle, MessageLogToggle, PoliticsDisplayToggle):
//! * F1 / ButtonObjectives: ObjectivesDisplay (Init `0x632E90`: the
//!   ObjectivesRequired / ObjectivesOptional panels with their viewports,
//!   ButtonClose and SecondaryButtonClose sending ObjectivesDisplayClose).
//!   Refresh `0x633A50` once a second: a world without scripted primary
//!   objectives gets the required `objective_defeat_all_enemies`
//!   (`0x633690`); each objective is an ObjectiveWidget (`0x633960`) whose
//!   Text takes text_color_required / _optional / _completed / _failed and
//!   whose Icon takes the matching icon_* texture;
//! * F2 / ButtonTechnology: TechnologyDisplay (Init `0x637AB0`), refresh
//!   `0x638170` once a second: one Technology item per technology property
//!   of the local kingdom that has an icon (Icon = the property icon, Name =
//!   the property name, `0x638070`);
//! * F3 / ButtonMessageLog: SideMessageLogPanel (game_interface.tgi
//!   Game/SideMessageLogPanel, ButtonClose / SecondaryButtonClose sending
//!   MessageLogToggle) holding SideMessageLog, the message log of every
//!   message (`0x5FD6D0` with MessageLogMaximumLines, no fading, fixed
//!   height);
//! * F4 / ButtonPolitics (a toggle button): PoliticsDisplay, a Label per
//!   kingdom (the colour gem and the name, the relation glyph ch_ally /
//!   ch_enemy / ch_neutral at offset_relation for the others) offset_top
//!   below the panel top, `separation` apart, the panel's bottom
//!   offset_bottom below the last label (the AA panel expands down).
//!
//! Compare against the original: the technology items' tooltips (the
//! property's modifier lines, `0x638D70` / `0x63B8C0`) show the data
//! tooltip only; the politics labels' code-set tooltip is not set.
use crate::session::Battle;
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};
use std::sync::Arc;

const OBJECTIVES: &str = "UI/Game/objectives_display.tgi:ObjectivesDisplay";
const TECHNOLOGY: &str = "UI/Game/technology_display.tgi:TechnologyDisplay";
const POLITICS: &str = "UI/Game/politics_display.tgi:PoliticsDisplay";
const GAME: &str = "UI/Game/game_interface.tgi:Game";

const TAG_OBJECTIVE: u64 = 9100;
const TAG_TECHNOLOGY: u64 = 9200;
const TAG_POLITICS: u64 = 9300;

/// Refresh period of the objectives and technology displays (`0x59ABA0`
/// >= 1000 ms).
const REFRESH_SECONDS: f32 = 1.0;

#[derive(Default)]
pub struct Panels {
    objectives: bool,
    technology: bool,
    log: bool,
    politics: bool,
    /// Game time of the last objectives / technology refresh.
    refreshed: Option<f32>,
    /// What the politics display shows (rebuilt when it changes).
    politics_rows: Vec<String>,
    shown_objectives: Vec<(String, u8)>,
    shown_technologies: Vec<String>,
}

fn w(class: C, name: &str) -> Widget {
    Widget::new(class, name, false)
}

/// Push button whose client is a label (`0x5F3440`).
fn label_button(name: &str, command: &str) -> Widget {
    let mut b = w(C::PushButton, name);
    b.command = Some(command.into());
    b.add_child(w(C::Label, "Client"));
    b
}

/// Load a code-built widget against the data node `parent` (a `file:node`
/// path), as the owned displays and their item pools do.
fn load_under(ui: &mut Ui, item: &mut Widget, parent: &str) {
    if let Ok(n) = ui.store.node(parent) {
        let strings = ui.strings.clone();
        let resolve = |v: &str| strings.resolve(v);
        let root = Arc::new(n.clone());
        item.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
    }
}

fn set_label_color(ui: &mut Ui, path: &str, rgb: [f32; 3]) {
    ui.with_widget(path, |w| {
        for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
            if let aa_ui::ve::VeKind::Label(l) = &mut ve.kind {
                l.color = [rgb[0], rgb[1], rgb[2], l.color[3]];
            }
        }
    });
}

fn set_decal(ui: &mut Ui, path: &str, texture: Option<String>) {
    ui.with_widget(path, |w| {
        if let Some(aa_ui::ve::ViewElement { kind: aa_ui::ve::VeKind::Decal(d), .. }) = &mut w.sub_ve {
            d.texture = texture;
        }
    });
}

fn rel(path: &str) -> &str {
    path.strip_prefix("Root/").unwrap_or(path)
}

impl Panels {
    /// The display toggles and close commands. `true` = handled.
    pub fn command(&mut self, ui: &mut Ui, cmd: &str) -> bool {
        match cmd {
            "ObjectivesDisplayToggle" => self.set_objectives(ui, !self.objectives),
            "ObjectivesDisplayClose" => self.set_objectives(ui, false),
            "TechnologyDisplayToggle" => self.set_technology(ui, !self.technology),
            "TechnologyDisplayClose" => self.set_technology(ui, false),
            "MessageLogToggle" => self.set_log(ui, !self.log),
            "PoliticsDisplayToggle" => self.set_politics(ui, !self.politics),
            _ => return false,
        }
        true
    }

    /// hotkeys_custom.txt: f1 ButtonObjectives, f2 ButtonTechnology, f3
    /// ButtonMessageLog, f4 ButtonPolitics (and f1 / f2 / f3 close their
    /// display while it is open).
    pub fn hotkey(&mut self, ui: &mut Ui, key: bevy::prelude::KeyCode) -> bool {
        use bevy::prelude::KeyCode;
        let cmd = match key {
            KeyCode::F1 => "ObjectivesDisplayToggle",
            KeyCode::F2 => "TechnologyDisplayToggle",
            KeyCode::F3 => "MessageLogToggle",
            KeyCode::F4 => "PoliticsDisplayToggle",
            _ => return false,
        };
        self.command(ui, cmd)
    }

    fn set_objectives(&mut self, ui: &mut Ui, open: bool) {
        if open == self.objectives {
            return;
        }
        self.objectives = open;
        if !open {
            ui.disconnect("ObjectivesDisplay");
            ui.layout();
            return;
        }
        let mut d = w(C::Panel, OBJECTIVES);
        let mut req = w(C::Panel, "ObjectivesRequired");
        req.add_child(Widget::composite(C::Viewport, "ObjectivesViewport", false));
        d.add_child(req);
        let mut opt = w(C::Panel, "ObjectivesOptional");
        opt.add_child(Widget::composite(C::Viewport, "ObjectivesOptionalViewport", false));
        d.add_child(opt);
        d.add_child(label_button("ButtonClose", "ObjectivesDisplayClose"));
        d.add_child(label_button("SecondaryButtonClose", "ObjectivesDisplayClose"));
        aa_ui::screens::show(ui, d);
        ui.layout();
        self.shown_objectives.clear();
        self.refreshed = None;
    }

    fn set_technology(&mut self, ui: &mut Ui, open: bool) {
        if open == self.technology {
            return;
        }
        self.technology = open;
        if !open {
            ui.disconnect("TechnologyDisplay");
            ui.layout();
            return;
        }
        let mut d = w(C::Panel, TECHNOLOGY);
        d.add_child(label_button("ButtonClose", "TechnologyDisplayClose"));
        d.add_child(label_button("SecondaryButtonClose", "TechnologyDisplayClose"));
        d.add_child(Widget::composite(C::Viewport, "ViewportTechnologies", false));
        aa_ui::screens::show(ui, d);
        ui.layout();
        self.shown_technologies.clear();
        self.refreshed = None;
    }

    fn set_log(&mut self, ui: &mut Ui, open: bool) {
        if open == self.log {
            return;
        }
        self.log = open;
        if !open {
            ui.disconnect("SideMessageLogPanel");
            ui.layout();
            return;
        }
        let mut p = w(C::Panel, "SideMessageLogPanel");
        p.add_child(label_button("ButtonClose", "MessageLogToggle"));
        p.add_child(label_button("SecondaryButtonClose", "MessageLogToggle"));
        p.add_child(aa_ui::message_log::message_log("SideMessageLog", "MessageLogMaximumLines", false, false));
        load_under(ui, &mut p, GAME);
        ui.connect(p);
        ui.layout();
    }

    fn set_politics(&mut self, ui: &mut Ui, open: bool) {
        if open == self.politics {
            return;
        }
        self.politics = open;
        // the control panel's toggle button shows the state
        ui.with_widget("Game/ControlPanel/ButtonPolitics", |w| {
            if open {
                w.flags |= aa_ui::state::widget_flag::SELECTED;
            } else {
                w.flags &= !aa_ui::state::widget_flag::SELECTED;
            }
        });
        if !open {
            ui.disconnect("PoliticsDisplay");
            ui.layout();
            return;
        }
        self.politics_rows.clear();
    }

    /// Per frame: the open displays' contents.
    pub fn update(&mut self, ui: &mut Ui, b: &Battle, property_icons: &std::collections::HashMap<String, String>) {
        let now = b.world.time;
        let due = self.refreshed.is_none_or(|t| now - t >= REFRESH_SECONDS || now < t);
        if due && (self.objectives || self.technology) {
            self.refreshed = Some(now);
            if self.objectives {
                self.refresh_objectives(ui, b);
            }
            if self.technology {
                self.refresh_technology(ui, b, property_icons);
            }
        }
        if self.politics {
            self.refresh_politics(ui, b);
        }
    }

    /// `0x633A50` with the default objective of `0x633690`: state 0
    /// required, 1 completed, 2 failed.
    fn refresh_objectives(&mut self, ui: &mut Ui, b: &Battle) {
        let world = &b.world;
        let state = match world.winner {
            Some(team) if world.kingdoms.iter().find(|k| k.id == b.local).is_some_and(|k| k.team == team) => 1,
            Some(_) => 2,
            None => 0,
        };
        let want = vec![("objective_defeat_all_enemies".to_string(), state)];
        if want == self.shown_objectives {
            return;
        }
        self.shown_objectives = want.clone();
        let vp = format!("Root/{}/ObjectivesRequired/ObjectivesViewport", "ObjectivesDisplay");
        let _ = ui.viewport_clear(&vp);
        let node = ui.store.node(OBJECTIVES).ok();
        let color = |k: &str, d: [f32; 3]| node.as_ref().and_then(|n| n.point3(k).ok()).unwrap_or(d);
        let icon = |k: &str| node.as_ref().map(|n| n.string_or_empty(k)).filter(|s| !s.is_empty());
        for (i, (key, st)) in want.iter().enumerate() {
            let mut item = w(C::Panel, "ObjectiveWidget");
            item.add_child(w(C::Label, "Text"));
            item.add_child(w(C::Decal, "Icon"));
            load_under(ui, &mut item, OBJECTIVES);
            item.tag = TAG_OBJECTIVE + i as u64;
            ui.viewport_add(&vp, item);
            let Some(p) = ui.find_tag(TAG_OBJECTIVE + i as u64) else { continue };
            let text = ui.strings.resolve(&format!("#{key}"));
            ui.set_label(&format!("{}/Text", rel(&p)), &text);
            let (c, t) = match st {
                1 => (color("text_color_completed", [0.0, 1.0, 0.0]), icon("icon_completed")),
                2 => (color("text_color_failed", [1.0, 0.0, 0.0]), icon("icon_failed")),
                _ => (color("text_color_required", [1.0, 1.0, 1.0]), icon("icon_required")),
            };
            set_label_color(ui, &format!("{}/Text", rel(&p)), c);
            set_decal(ui, &format!("{}/Icon", rel(&p)), t);
        }
        ui.viewport_refresh(&vp);
        ui.layout();
    }

    /// `0x638170`: the local kingdom's technologies with an icon.
    fn refresh_technology(&mut self, ui: &mut Ui, b: &Battle, property_icons: &std::collections::HashMap<String, String>) {
        let world = &b.world;
        let techs: Vec<String> = world
            .books
            .kingdom(b.local)
            .map(|k| k.technologies.iter().filter(|t| property_icons.contains_key(&t.to_ascii_lowercase())).cloned().collect())
            .unwrap_or_default();
        if techs == self.shown_technologies {
            return;
        }
        self.shown_technologies = techs.clone();
        let vp = "Root/TechnologyDisplay/ViewportTechnologies".to_string();
        let _ = ui.viewport_clear(&vp);
        for (i, t) in techs.iter().enumerate() {
            let mut item = w(C::Panel, "Technology");
            item.add_child(w(C::Decal, "Icon"));
            item.add_child(w(C::Label, "Name"));
            load_under(ui, &mut item, TECHNOLOGY);
            item.tag = TAG_TECHNOLOGY + i as u64;
            ui.viewport_add(&vp, item);
            let Some(p) = ui.find_tag(TAG_TECHNOLOGY + i as u64) else { continue };
            let name = world.data.property(t).map(|d| d.name.clone()).unwrap_or_else(|| t.clone());
            let name = ui.strings.resolve(&name);
            ui.set_label(&format!("{}/Name", rel(&p)), &name);
            set_decal(ui, &format!("{}/Icon", rel(&p)), property_icons.get(&t.to_ascii_lowercase()).cloned());
        }
        ui.viewport_refresh(&vp);
        ui.layout();
    }

    fn refresh_politics(&mut self, ui: &mut Ui, b: &Battle) {
        let world = &b.world;
        let node = ui.store.node(POLITICS).ok();
        let num = |k: &str, d: f32| node.as_ref().and_then(|n| n.float(k).ok()).unwrap_or(d);
        let glyph = |k: &str| node.as_ref().map(|n| n.string_or_empty(k)).filter(|s| !s.is_empty());
        let (glyph_ally, glyph_enemy, glyph_neutral) = (glyph("glyph_ally"), glyph("glyph_enemy"), glyph("glyph_neutral"));
        let offset_relation = num("offset_relation", 161.0);
        let gem = ui.strings.glyphs.get(&ui.avars.get("KingdomGlyphInfo").unwrap_or("ch_gem").to_ascii_lowercase()).copied().unwrap_or('#');
        let g = |v: f32| aa_ui::staging::fmt_g(v as f64);
        let mut rows = Vec::new();
        for k in world.kingdoms.iter().filter(|k| !k.independent) {
            let c = k.color.map(|v| v as f32 / 255.0);
            let mut text = format!("<color={},{},{}>{} <rc>{}", g(c[0]), g(c[1]), g(c[2]), gem, k.name);
            if k.id != b.local {
                let rel = aa_play::setup::relation(world, Some(b.local), Some(k.id));
                let gl = match rel {
                    0 | 1 => glyph_ally.as_ref(),
                    3 => glyph_enemy.as_ref(),
                    _ => glyph_neutral.as_ref(),
                };
                // SetX is in window pixels: offset_relation (reference
                // units) times the horizontal scale
                if let Some(ch) = gl.and_then(|n| ui.strings.glyphs.get(&n.to_ascii_lowercase()).copied()) {
                    text.push_str(&format!("<SetX={}>{}", (offset_relation * ui.geo().sx) as i32, ch));
                }
            }
            rows.push(text);
        }
        if rows == self.politics_rows && ui.widget("Root/PoliticsDisplay").is_some() {
            return;
        }
        self.politics_rows = rows.clone();
        ui.disconnect("PoliticsDisplay");
        let mut d = w(C::Panel, POLITICS);
        d.add_child(w(C::Decal, "Background"));
        for (i, _) in rows.iter().enumerate() {
            let mut l = w(C::Label, "Label");
            l.tag = TAG_POLITICS + i as u64;
            d.add_child(l);
        }
        aa_ui::screens::show(ui, d);
        // the Background decal's opacity is the display's `transparency`
        // (0: only its tooltip panel frame shows)
        let alpha = num("transparency", 0.0).clamp(0.0, 1.0);
        ui.with_widget("PoliticsDisplay/Background", |wd| {
            if let Some(aa_ui::ve::ViewElement { kind: aa_ui::ve::VeKind::Decal(dc), .. }) = &mut wd.sub_ve {
                dc.color[3] = alpha;
            }
        });
        // the labels from offset_top down, the bottom offset_bottom below the
        // last one (reference units)
        let (top, bottom, sep) = (num("offset_top", 20.0), num("offset_bottom", 20.0), num("separation", 3.0));
        let label_h = 15.0;
        let geo = ui.geo();
        let ref_w = ui.screen.width as f32 / geo.sx;
        let n = rows.len() as f32;
        let h = top + n * label_h + (n - 1.0).max(0.0) * sep + bottom;
        ui.with_widget("PoliticsDisplay", |wd| aa_ui::controls::set_bounds(&mut wd.root.outer, [800.0, 0.0, ref_w - 800.0, h], 0x300));
        for (i, text) in rows.iter().enumerate() {
            let Some(p) = ui.find_tag(TAG_POLITICS + i as u64) else { continue };
            let t = top + i as f32 * (label_h + sep);
            ui.with_widget(rel(&p), |wd| aa_ui::controls::set_bounds(&mut wd.root.outer, [23.0, t, 188.0, label_h], 0x300));
            ui.set_label(rel(&p), text);
        }
        ui.layout();
    }
}
