//! The staging launch-type dialogs (SelectLaunchType 0x65C1C0 ->
//! 0x68DB60): the Scenario Map file dialog (StagingFileDialog, ctor
//! 0x68B0D0, Init 0x68D280, Activate 0x68B6B0, list 0x68E340, preview
//! 0x68A820 / 0x6612E0, Commit 0x68EF50) and the Random Map dialog
//! (RandomMapDialog, ctor 0x6846C0, Init 0x684880, Activate 0x685240,
//! types 0x6880B0, apply 0x686B10, description 0x683920, Commit 0x6875A0),
//! with the shared world-rules panel (KKC_WorldRulesPanel 0x6A2D80,
//! ConfigureUI 0x6A3070, rule widgets 0x6A2580 / 0x6A2730 / 0x6A29F0 /
//! 0x6A2B90 / 0x6A28A0, read back 0x6A0240).
//!
//! Widgets are built in the native AddChild order and laid out from the
//! original TGI nodes; the data comes from [`super::gamedata`].
use super::gamedata::{GameData, MapInfo, Rmc, WorldRule};
use aa_ui::controls::{Control, ListCallback, SliderCallback};
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};

pub const FILE_DIALOG: &str = "UI/Shared/staging_file_dialog.tgi:StagingFileDialog";
pub const RMC_DIALOG: &str = "/UI/Shared/random_map_dialog.tgi:RandomMapDialog";

fn w(class: C, name: &str) -> Widget {
    Widget::new(class, name, false)
}

/// Push button whose client is a code-set label (0x5F3410 + 0x5CF4F0).
fn label_button(name: &str, command: &str) -> Widget {
    let mut b = w(C::PushButton, name);
    b.command = Some(command.into());
    b.add_child(w(C::Label, "Client"));
    b
}

fn listbox(name: &str) -> Widget {
    let mut lb = Widget::composite(C::Listbox, name, false);
    if let Some(l) = lb.listbox_mut() {
        l.on_select = ListCallback::Command("Select \"%s\"".into());
        l.on_double = ListCallback::Command("SelectFromDouble \"%s\"".into());
    }
    lb
}

fn dropdown(name: &str, command: Option<&str>) -> Widget {
    let mut d = Widget::composite(C::DropDown, name, false);
    if let Some(Control::DropDown(x)) = d.control.as_deref_mut() {
        x.command = command.map(str::to_string);
    }
    d
}

fn slider(name: &str) -> Widget {
    let mut s = Widget::composite(C::SliderAdvanced, name, false);
    if let Some(x) = s.slider_mut() {
        x.callback = SliderCallback::None;
    }
    s
}

/// Load a code-built item against `parent_node` (a `file:node/...` path)
/// as the native item pools do (staging `0x6919D0`).
fn load_item(ui: &mut Ui, item: &mut Widget, parent_node: &str) {
    if let Ok(n) = ui.store.node(parent_node) {
        let strings = ui.strings.clone();
        let resolve = |v: &str| strings.resolve(v);
        let root = std::sync::Arc::new(n.clone());
        item.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
    }
}

fn set_text(ui: &mut Ui, path: &str, text: &str) {
    ui.set_label(path, text);
}

/// `(int)((UIMapPreviewTabWidth + 15) * sx)` (0x6612E0 tab stop).
fn tab_x(ui: &Ui) -> i32 {
    let tab = ui.avars.float("UIMapPreviewTabWidth").unwrap_or(100.0);
    let sx = ui.screen.width as f32 / ui.reference.width.max(1.0);
    ((tab + 15.0) * sx) as i32
}

fn line(ui: &Ui, label_key: &str, value: &str) -> String {
    format!("{}<SetX={}>{}", ui.strings.resolve(label_key), tab_x(ui), value)
}

fn gem(ui: &Ui) -> char {
    ui.strings.glyphs.get(&ui.avars.get("KingdomGlyphInfo").unwrap_or("ch_gem").to_ascii_lowercase()).copied().unwrap_or('#')
}

fn kingdom_row(ui: &Ui, color: [f32; 3], name: &str) -> String {
    let g = |v: f32| aa_ui::staging::fmt_g(v as f64);
    format!("<color={},{},{}>{} <rc>{}", g(color[0]), g(color[1]), g(color[2]), gem(ui), name)
}

/// A world rule's current value (as its ids / number text).
#[derive(Clone, Debug, PartialEq)]
pub struct RuleValue {
    pub ids: String,
    pub value: String,
}

/// The value line of a rule (0x65E360: bool true/false name, int "%d",
/// fixed "%.0f", enum value name).
fn rule_value_text(ui: &Ui, rule: &WorldRule, value: &str) -> String {
    match rule.kind.as_str() {
        "bool" => {
            let t = value.eq_ignore_ascii_case("true");
            ui.strings.resolve(if t { &rule.true_name } else { &rule.false_name })
        }
        "int" => value.parse::<f32>().map(|v| format!("{}", v as i32)).unwrap_or_else(|_| value.to_string()),
        "fixed" => value.parse::<f32>().map(|v| format!("{v:.0}")).unwrap_or_else(|_| value.to_string()),
        _ => rule.enum_values.iter().find(|(i, _)| i.eq_ignore_ascii_case(value)).map(|(_, n)| ui.strings.resolve(n)).unwrap_or_else(|| value.to_string()),
    }
}

/// A map's stored world rules as rule values (bool byte, int, fixed f32,
/// enum index into the rule's values).
fn map_rules(data: &GameData, m: &MapInfo) -> Vec<RuleValue> {
    m.rules
        .iter()
        .filter_map(|(ids, kind, raw)| {
            let rule = data.rules.iter().find(|r| r.ids.eq_ignore_ascii_case(ids))?;
            let value = match kind {
                0 => (*raw != 0).to_string(),
                1 => (*raw as i32).to_string(),
                2 => format!("{}", f32::from_bits(*raw)),
                _ => rule.enum_values.get(*raw as usize)?.0.clone(),
            };
            Some(RuleValue { ids: rule.ids.clone(), value })
        })
        .collect()
}

/// Rule lines whose value differs from the default (0x511460).
pub fn rule_lines(ui: &Ui, data: &GameData, values: &[RuleValue]) -> Vec<String> {
    let mut out = Vec::new();
    for v in values {
        let Some(rule) = data.rules.iter().find(|r| r.ids.eq_ignore_ascii_case(&v.ids)) else { continue };
        let same = match (v.value.parse::<f32>(), rule.default.parse::<f32>()) {
            (Ok(a), Ok(b)) => a == b,
            _ => v.value.eq_ignore_ascii_case(&rule.default),
        };
        if !same {
            out.push(format!("{}<SetX={}>{}", ui.strings.resolve(&rule.name), tab_x(ui), rule_value_text(ui, rule, &v.value)));
        }
    }
    out
}

/// The world-rules panel (KKC_WorldRulesPanel): one rule widget per
/// editable rule in its WorldRulesViewport.
struct RulesPanel {
    /// Widget path of the panel.
    path: String,
    /// Data node of the panel (`file:Dialog/WorldRulesPanel`).
    node: String,
    shown: Vec<(String, String)>,
    /// Two rules per row (ctor argument, +0xB9: the Scenario Map dialog's
    /// panel, `0x68D280` passes 1): each WorldRuleSlot row holds a rule in
    /// its Left then its Right half (`0x6A1360` / `0x69FE30`).
    columns: bool,
}

impl RulesPanel {
    fn widget() -> Widget {
        let mut p = w(C::Panel, "WorldRulesPanel");
        p.add_child(Widget::composite(C::Viewport, "WorldRulesViewport", false));
        p
    }

    /// ConfigureUI 0x6A3070: rules not never_editable; mode 0 shows the
    /// unlocked ones, mode 1 also always_editable ones.
    fn configure(&mut self, ui: &mut Ui, data: &GameData, values: &[RuleValue], locked: &[String], mode: u8) {
        let vp = format!("{}/WorldRulesViewport", self.path);
        let _ = ui.viewport_clear(&vp);
        self.shown.clear();
        for v in values {
            let Some(rule) = data.rules.iter().find(|r| r.ids.eq_ignore_ascii_case(&v.ids)) else { continue };
            if rule.never_editable {
                continue;
            }
            let is_locked = locked.iter().any(|l| l.eq_ignore_ascii_case(&rule.ids));
            let show = match mode {
                0 => !is_locked,
                _ => !is_locked || rule.always_editable,
            };
            if !show {
                continue;
            }
            let (class_name, mut item) = if rule.use_slider {
                let mut it = w(C::Panel, "WorldRuleSlider");
                it.add_child(w(C::Label, "Name"));
                it.add_child(slider("RangeSlider"));
                ("WorldRuleSlider", it)
            } else {
                match rule.kind.as_str() {
                    "bool" => {
                        let mut it = w(C::Panel, "WorldRuleBool");
                        it.add_child(w(C::Label, "Name"));
                        it.add_child(Widget::toggle_button("Checkbox", false, ""));
                        ("WorldRuleBool", it)
                    }
                    "int" | "fixed" => {
                        let n = if rule.kind == "int" { "WorldRuleInt" } else { "WorldRuleFixed" };
                        let mut it = w(C::Panel, n);
                        it.add_child(w(C::Label, "Name"));
                        it.add_child(Widget::composite(C::Editbox, "Editbox", false));
                        (n, it)
                    }
                    _ => {
                        let mut it = w(C::Panel, "WorldRuleEnum");
                        it.add_child(w(C::Label, "Name"));
                        it.add_child(dropdown("Dropdown", None));
                        ("WorldRuleEnum", it)
                    }
                }
            };
            load_item(ui, &mut item, &self.node);
            item.tag = 7000 + self.shown.len() as u64;
            if self.columns {
                // a new WorldRuleSlot when there is none or the last is full
                let n = self.shown.len();
                if n % 2 == 0 {
                    let mut slot = w(C::Panel, "WorldRuleSlot");
                    slot.add_child(w(C::Panel, "Left"));
                    slot.add_child(w(C::Panel, "Right"));
                    load_item(ui, &mut slot, &self.node);
                    slot.tag = 7500 + (n / 2) as u64;
                    ui.viewport_add(&vp, slot);
                }
                if let Some(slot) = ui.find_tag(7500 + (n / 2) as u64) {
                    let half = if n % 2 == 0 { "Left" } else { "Right" };
                    let _ = ui.attach(&format!("{slot}/{half}"), item);
                }
            } else {
                ui.viewport_add(&vp, item);
            }
            self.shown.push((rule.ids.clone(), class_name.to_string()));
        }
        ui.layout();
        // values and labels
        for (i, (ids, class_name)) in self.shown.clone().into_iter().enumerate() {
            let Some(rule) = data.rules.iter().find(|r| r.ids == ids).cloned() else { continue };
            let Some(item) = ui.find_tag(7000 + i as u64) else { continue };
            let value = values.iter().find(|v| v.ids.eq_ignore_ascii_case(&ids)).map(|v| v.value.clone()).unwrap_or(rule.default.clone());
            set_text(ui, &format!("{}/Name", rel(&item)), &ui.strings.resolve(&rule.name));
            match class_name.as_str() {
                "WorldRuleSlider" => {
                    let sp = format!("{item}/RangeSlider");
                    ui.slider_set_range(&sp, rule.slider.0, rule.slider.1);
                    if let Some(s) = ui.widget_mut(&sp).and_then(|w| w.slider_mut()) {
                        s.step = rule.slider.2;
                    }
                    ui.slider_set_value(&sp, value.parse().unwrap_or(0.0), true);
                }
                "WorldRuleBool" => {
                    let on = value.eq_ignore_ascii_case("true");
                    ui.with_widget(&format!("{}/Checkbox", rel(&item)), |w| {
                        if on {
                            w.flags |= aa_ui::state::widget_flag::SELECTED;
                        } else {
                            w.flags &= !aa_ui::state::widget_flag::SELECTED;
                        }
                    });
                }
                "WorldRuleInt" | "WorldRuleFixed" => {
                    let ep = format!("{item}/Editbox");
                    let text = match value.parse::<f32>() {
                        Ok(v) if class_name == "WorldRuleInt" => format!("{}", v as i32),
                        // "%f" with the trailing zeros and '.' removed
                        Ok(v) => {
                            let s = format!("{v:.6}");
                            s.trim_end_matches('0').trim_end_matches('.').to_string()
                        }
                        Err(_) => value.clone(),
                    };
                    if let Some(max) = rule.editbox_max {
                        if let Some(e) = ui.widget_mut(&ep).and_then(|w| w.editbox_mut()) {
                            e.integer_max = max as i32;
                        }
                    }
                    ui.editbox_set_text(&ep, &text);
                }
                _ => {
                    let dd = format!("{item}/Dropdown");
                    let lb = format!("{dd}/Listbox");
                    ui.listbox_clear(&lb);
                    for (eid, ename) in &rule.enum_values {
                        let t = ui.strings.resolve(ename);
                        ui.listbox_add(&lb, &t, eid);
                    }
                    if let Some(i) = rule.enum_values.iter().position(|(e, _)| e.eq_ignore_ascii_case(&value)) {
                        ui.dropdown_select(&dd, i);
                    }
                }
            }
        }
        ui.viewport_refresh(&vp);
        ui.layout();
    }

    /// Read the shown widgets back into `values` (0x6A0240).
    fn read(&self, ui: &Ui, values: &mut [RuleValue]) {
        for (i, (ids, class_name)) in self.shown.iter().enumerate() {
            let Some(item) = ui.find_tag(7000 + i as u64) else { continue };
            let v = match class_name.as_str() {
                "WorldRuleSlider" => ui.slider_value(&format!("{item}/RangeSlider")).map(|v| format!("{v}")),
                "WorldRuleBool" => ui.widget(&format!("{item}/Checkbox")).map(|w| (w.flags & aa_ui::state::widget_flag::SELECTED != 0).to_string()),
                "WorldRuleInt" | "WorldRuleFixed" => ui.editbox_text(&format!("{item}/Editbox")),
                _ => {
                    let lb = format!("{item}/Dropdown/Listbox");
                    ui.listbox_selected(&lb).and_then(|s| ui.listbox_entries(&lb).get(s).map(|e| e.param.clone()))
                }
            };
            if let (Some(v), Some(slot)) = (v, values.iter_mut().find(|x| x.ids.eq_ignore_ascii_case(ids))) {
                slot.value = v;
            }
        }
    }
}

fn rel(path: &str) -> &str {
    path.strip_prefix("Root/").unwrap_or(path)
}

/// Result of a dialog command.
pub enum Outcome {
    /// Still open.
    Open,
    /// Closed without changes (Cancel).
    Cancelled,
    /// Scenario Map chosen.
    Map { map: MapInfo, rules: Vec<RuleValue> },
    /// Random Map settings chosen.
    Random { rmc: String, size: (f32, f32), terrain: Vec<f32>, rules: Vec<RuleValue>, prefs: std::collections::HashMap<String, RmcPrefs> },
}

/// The world rule values a setup starts from (rule defaults).
pub fn default_rules(data: &GameData) -> Vec<RuleValue> {
    data.rules.iter().map(|r| RuleValue { ids: r.ids.clone(), value: r.default.clone() }).collect()
}

// ------------------------------------------------------------ Scenario Map

pub struct FileDialog {
    maps: Vec<MapInfo>,
    selected: Option<usize>,
    rules_panel: RulesPanel,
    rules: Vec<RuleValue>,
}

const FD: &str = "Root/StagingFileDialog";

impl FileDialog {
    /// Init 0x68D280 + Activate 0x68B6B0 + FillList / PopulateFromScan.
    pub fn open(ui: &mut Ui, data: &GameData, current_map: Option<&str>, rules: Vec<RuleValue>) -> Self {
        let mut d = Widget::new(C::TopLevel, FILE_DIALOG, true);
        let mut sp = w(C::Panel, "SelectPanel");
        sp.add_child(listbox("SelectListBox"));
        d.add_child(sp);
        d.add_child(label_button("SelectButton", "Commit"));
        d.add_child(Widget::push_button("CancelButton", false, "Cancel"));
        let mut mp = w(C::Panel, "MapPreviewPanel");
        mp.add_child(Widget::composite(C::Viewport, "MapPreviewViewport", false));
        d.add_child(mp);
        d.add_child(RulesPanel::widget());
        let mut del = Widget::push_button("DeleteButton", false, "Delete");
        let mut edit = Widget::composite(C::Editbox, "DescriptionEditBox", false);
        // Activate: offline LOAD_MAP hides and disables both
        del.set_visible(false);
        del.set_enabled(false);
        edit.set_visible(false);
        edit.set_enabled(false);
        d.add_child(del);
        d.add_child(edit);
        let mut notice = w(C::Panel, "NoticeDialog");
        notice.add_child(w(C::Label, "StatLabel"));
        notice.set_visible(false);
        d.add_child(notice);
        aa_ui::screens::show(ui, d);
        let load = ui.strings.resolve("#staging_file_load_button");
        set_text(ui, "StagingFileDialog/SelectButton/Client", &load);
        // MapPreviewLabel (Activate, viewport AddItem 0x5ED730)
        let mut label = w(C::Label, "MapPreviewLabel");
        load_item(ui, &mut label, &format!("{FILE_DIALOG}/MapPreviewPanel/MapPreviewViewport/ItemPanel"));
        ui.viewport_add(&format!("{FD}/MapPreviewPanel/MapPreviewViewport"), label);
        // the scan (0x68E340): Maps/*.RMP, directories skipped; entry text
        // = the map name
        let mut maps: Vec<MapInfo> = super::gamedata::list_maps(&ui.store.root).iter().filter_map(|p| super::gamedata::read_map(&ui.store.root, p)).collect();
        maps.sort_by_key(|m| m.name.to_lowercase());
        // duplicates get "%s [%s]" (text, file title)
        let names: Vec<String> = maps.iter().map(|m| m.name.clone()).collect();
        let lb = format!("{FD}/SelectPanel/SelectListBox");
        ui.listbox_clear(&lb);
        for (i, m) in maps.iter().enumerate() {
            let dup = (i > 0 && names[i - 1] == names[i]) || (i + 1 < names.len() && names[i + 1] == names[i]);
            let text = if dup { format!("{} [{}]", m.name, m.title) } else { m.name.clone() };
            ui.listbox_add(&lb, &text, &m.path);
        }
        let mut dlg = FileDialog {
            maps,
            selected: None,
            rules_panel: RulesPanel { path: format!("{FD}/WorldRulesPanel"), node: format!("{FILE_DIALOG}/WorldRulesPanel"), shown: Vec::new(), columns: true },
            rules,
        };
        // pre-selection: the session's map, else the first entry
        let entries = ui.listbox_entries(&lb);
        let pre = current_map.and_then(|cur| entries.iter().position(|e| e.param.eq_ignore_ascii_case(cur))).or((!entries.is_empty()).then_some(0));
        if let Some(i) = pre {
            ui.listbox_select(&lb, i, false);
            dlg.select_path(ui, data, &entries[i].param.clone());
        }
        ui.layout();
        dlg
    }

    fn select_path(&mut self, ui: &mut Ui, data: &GameData, path: &str) {
        self.selected = self.maps.iter().position(|m| m.path.eq_ignore_ascii_case(path));
        let text = match self.selected.map(|i| self.maps[i].clone()) {
            Some(m) => self.preview_text(ui, data, &m),
            None => String::new(),
        };
        let lp = format!("{FD}/MapPreviewPanel/MapPreviewViewport/ItemPanel/MapPreviewLabel");
        ui.set_label(rel(&lp), &text);
        aa_ui::staging::label_fit_height(ui, &lp);
        ui.viewport_refresh(&format!("{FD}/MapPreviewPanel/MapPreviewViewport"));
        // WorldRulesPanel: mode 1, the map's rules (no locks known)
        let rules = self.rules.clone();
        self.rules_panel.configure(ui, data, &rules, &[], 1);
        ui.layout();
    }

    /// 0x6612E0 case 2: title, File, Size, Sides, rules, blank, kingdom
    /// rows, description.
    fn preview_text(&self, ui: &Ui, data: &GameData, m: &MapInfo) -> String {
        let mut lines = vec![m.name.clone()];
        let file = m.path.rsplit('/').next().unwrap_or(&m.path).to_string();
        lines.push(line(ui, "#staging_map_preview_file_name", &file));
        let size = ui.strings.format("staging_map_preview_map_size_format_string|dd", &[&m.size.0.to_string(), &m.size.1.to_string()]);
        lines.push(line(ui, "#staging_map_preview_map_size", &size));
        let playable: Vec<_> = m.kingdoms.iter().filter(|k| !k.independent && k.playable).collect();
        let avail = playable.len() as i32;
        let mut req = playable.iter().filter(|k| k.required).count() as i32;
        if req < 1 && avail > 0 {
            req = 1;
        }
        let sides = if req == avail { avail.to_string() } else { ui.strings.format("staging_map_preview_num_req_kingdoms_format_string|dd", &[&req.to_string(), &avail.to_string()]) };
        lines.push(line(ui, "#staging_map_preview_num_kingdoms_format_string", &sides));
        // case 2: the map's own world rules that differ from the rule
        // defaults (0x65EF50), then a blank line
        lines.extend(rule_lines(ui, data, &map_rules(data, m)));
        lines.push(String::new());
        for k in &playable {
            lines.push(kingdom_row(ui, data.color(&k.color), &k.name));
        }
        // the side rows' line end and 0x65EBB0's: a blank line, then the
        // description (0x65E360)
        if !m.description.is_empty() {
            lines.push(String::new());
            lines.push(m.description.clone());
        }
        lines.join("\n")
    }

    pub fn command(&mut self, ui: &mut Ui, data: &GameData, cmd: &str) -> Option<Outcome> {
        let arg = |c: &str| c.split_once(' ').map(|(_, a)| a.trim().trim_matches('"').to_string());
        if cmd.starts_with("Select \"") || cmd.starts_with("Select ") {
            if let Some(p) = arg(cmd) {
                self.select_path(ui, data, &p);
            }
            return Some(Outcome::Open);
        }
        if cmd.starts_with("SelectFromDouble") {
            if let Some(p) = arg(cmd) {
                self.select_path(ui, data, &p);
            }
            return Some(self.commit(ui));
        }
        match cmd {
            "Commit" => Some(self.commit(ui)),
            "Cancel" => {
                close(ui, "StagingFileDialog");
                Some(Outcome::Cancelled)
            }
            "Delete" | "EditBoxChanged" => Some(Outcome::Open),
            _ => None,
        }
    }

    fn commit(&mut self, ui: &mut Ui) -> Outcome {
        let Some(m) = self.selected.map(|i| self.maps[i].clone()) else { return Outcome::Open };
        let mut rules = self.rules.clone();
        self.rules_panel.read(ui, &mut rules);
        close(ui, "StagingFileDialog");
        Outcome::Map { map: m, rules }
    }
}

// ------------------------------------------------------------ Random Map

pub struct RandomDialog {
    rmcs: Vec<Rmc>,
    selected: Option<usize>,
    rules_panel: RulesPanel,
    rules: Vec<RuleValue>,
    /// Shown terrain sliders: index into the RMC's terrains, item tag.
    sliders: Vec<(usize, u64)>,
    /// The profile's RMC preferences (0x41F140): size and slider values
    /// per RMC, saved by Commit.
    prefs: std::collections::HashMap<String, RmcPrefs>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RmcPrefs {
    pub size: (f32, f32),
    pub terrain: Vec<f32>,
}

const RD: &str = "Root/RandomMapDialog";

impl RandomDialog {
    /// Init 0x684880 + Activate 0x685240 + FillList 0x6880B0.
    pub fn open(ui: &mut Ui, data: &GameData, current: Option<&str>, prefs: std::collections::HashMap<String, RmcPrefs>, rules: Vec<RuleValue>) -> Self {
        let mut d = Widget::new(C::TopLevel, RMC_DIALOG, true);
        let mut sp = w(C::Panel, "SelectPanel");
        sp.add_child(listbox("SelectListBox"));
        d.add_child(sp);
        let mut mp = w(C::Panel, "MapPreviewPanel");
        mp.add_child(Widget::composite(C::Viewport, "MapPreviewViewport", false));
        d.add_child(mp);
        d.add_child(RulesPanel::widget());
        d.add_child(label_button("SelectButton", "Commit"));
        d.add_child(Widget::push_button("CancelButton", false, "Cancel"));
        let mut ms = w(C::Panel, "MapSizePanel");
        ms.add_child(w(C::Label, "MapSizeLabel"));
        // native name MapSizeDropdown; the TGI node is MapSizeDropDown
        // (node lookup is case-insensitive)
        ms.add_child(dropdown("MapSizeDropdown", Some("SelectMapSize")));
        d.add_child(ms);
        let mut mk = w(C::Panel, "MaxKingdomsPanel");
        mk.add_child(w(C::Label, "MaxKingdomsLabel"));
        mk.add_child(dropdown("MaxKingdomsDropDown", None));
        // Activate: only in the editor
        mk.set_visible(false);
        d.add_child(mk);
        let mut tp = w(C::Panel, "TerrainsSlidersPanel");
        tp.add_child(Widget::composite(C::Viewport, "TerrainsSliders", false));
        d.add_child(tp);
        let mut op = w(C::Panel, "ObjectsSlidersPanel");
        op.add_child(Widget::composite(C::Viewport, "ObjectsSliders", false));
        d.add_child(op);
        let mut notice = w(C::Panel, "NoticeDialog");
        notice.add_child(w(C::Label, "StatLabel"));
        notice.set_visible(false);
        d.add_child(notice);
        aa_ui::screens::show(ui, d);
        let ok = ui.strings.resolve("#staging_file_ok_button");
        set_text(ui, "RandomMapDialog/SelectButton/Client", &ok);
        let mut label = w(C::Label, "MapPreviewLabel");
        load_item(ui, &mut label, &format!("{RMC_DIALOG}/MapPreviewPanel/MapPreviewViewport/ItemPanel"));
        ui.viewport_add(&format!("{RD}/MapPreviewPanel/MapPreviewViewport"), label);
        // the types: RMCs listed in staging, sorted by name
        let mut rmcs: Vec<Rmc> = data.rmcs.iter().filter(|r| r.list_in_staging).cloned().collect();
        rmcs.sort_by_key(|r| ui.strings.resolve(&r.name).to_lowercase());
        let lb = format!("{RD}/SelectPanel/SelectListBox");
        ui.listbox_clear(&lb);
        for r in &rmcs {
            let t = ui.strings.resolve(&r.name);
            ui.listbox_add(&lb, &t, &r.ids);
        }
        let mut dlg = RandomDialog {
            rmcs,
            selected: None,
            rules_panel: RulesPanel { path: format!("{RD}/WorldRulesPanel"), node: format!("{RMC_DIALOG}/WorldRulesPanel"), shown: Vec::new(), columns: false },
            rules,
            sliders: Vec::new(),
            prefs,
        };
        let entries = ui.listbox_entries(&lb);
        let pre = current.and_then(|ids| entries.iter().position(|e| e.param.eq_ignore_ascii_case(ids))).or((!entries.is_empty()).then_some(0));
        if let Some(i) = pre {
            ui.listbox_select(&lb, i, false);
            let ids = entries[i].param.clone();
            dlg.select(ui, data, &ids);
        }
        ui.layout();
        dlg
    }

    fn rmc(&self) -> Option<&Rmc> {
        self.selected.map(|i| &self.rmcs[i])
    }

    /// 0x688020: map sizes 0x683E00, then 0x686B10 (sliders, rules) and
    /// the description 0x683920.
    fn select(&mut self, ui: &mut Ui, data: &GameData, ids: &str) {
        self.selected = self.rmcs.iter().position(|r| r.ids.eq_ignore_ascii_case(ids));
        let Some(rmc) = self.rmc().cloned() else { return };
        // the profile's preferences for this RMC, else the first size and
        // the slider midpoints (0x506370)
        let pref = self.prefs.get(&rmc.ids).cloned();
        let size = pref.as_ref().map(|p| p.size);
        // sizes: "%d x %d", sorted (string order)
        let dd = format!("{RD}/MapSizePanel/MapSizeDropdown");
        let lb = format!("{dd}/Listbox");
        ui.listbox_clear(&lb);
        let mut texts: Vec<String> = rmc.sizes.iter().map(|s| format!("{} x {}", s.0 as i32, s.1 as i32)).collect();
        texts.sort();
        texts.dedup();
        for t in &texts {
            ui.listbox_add(&lb, t, t);
        }
        let want = size.map(|s| format!("{} x {}", s.0 as i32, s.1 as i32));
        let first = rmc.sizes.first().map(|s| format!("{} x {}", s.0 as i32, s.1 as i32));
        let pick = want.and_then(|w| texts.iter().position(|t| *t == w)).or_else(|| first.and_then(|f| texts.iter().position(|t| *t == f))).unwrap_or(0);
        if !texts.is_empty() {
            ui.dropdown_select(&dd, pick);
        }
        // terrain sliders (shown, unlocked enabled; step 0.1 for spans <= 1)
        let vp = format!("{RD}/TerrainsSlidersPanel/TerrainsSliders");
        let _ = ui.viewport_clear(&vp);
        self.sliders.clear();
        for (i, t) in rmc.terrains.iter().enumerate().filter(|(_, t)| t.show_ui) {
            let mut item = w(C::Panel, "Slider");
            item.add_child(w(C::Label, "NameLabel"));
            item.add_child(slider("RangeSlider"));
            load_item(ui, &mut item, &format!("{RMC_DIALOG}/TerrainsSlidersPanel/TerrainsSliders/ItemPanel"));
            let tag = 8000 + self.sliders.len() as u64;
            item.tag = tag;
            ui.viewport_add(&vp, item);
            self.sliders.push((i, tag));
        }
        ui.layout();
        for (i, tag) in self.sliders.clone() {
            let t = &rmc.terrains[i];
            let Some(p) = ui.find_tag(tag) else { continue };
            set_text(ui, &format!("{}/NameLabel", rel(&p)), &ui.strings.resolve(&t.name));
            let sp = format!("{p}/RangeSlider");
            ui.slider_set_range(&sp, t.range.0, t.range.1);
            if let Some(s) = ui.widget_mut(&sp).and_then(|w| w.slider_mut()) {
                s.step = if (t.range.1 - t.range.0).abs() <= 1.0 { 0.1 } else { 1.0 };
            }
            // no saved preference: the midpoint (0x506370)
            let v = pref.as_ref().and_then(|p| p.terrain.get(i).copied()).unwrap_or(t.range.0 + 0.5 * (t.range.1 - t.range.0));
            ui.slider_set_value(&sp, v, true);
            let locked = t.locked;
            ui.with_widget(&format!("{}/RangeSlider", rel(&p)), |w| w.set_enabled(!locked));
        }
        ui.viewport_refresh(&vp);
        // world rules (mode 0): the RMC's hidden rules are locked
        let locked: Vec<String> = rmc.rules.iter().filter(|r| !r.2).map(|r| r.0.clone()).collect();
        for (ids, default, _) in &rmc.rules {
            if let Some(v) = self.rules.iter_mut().find(|v| v.ids.eq_ignore_ascii_case(ids)) {
                v.value = default.clone();
            }
        }
        let rules = self.rules.clone();
        self.rules_panel.configure(ui, data, &rules, &locked, 0);
        self.describe(ui, data);
    }

    fn size(&self, ui: &Ui) -> Option<(f32, f32)> {
        let lb = format!("{RD}/MapSizePanel/MapSizeDropdown/Listbox");
        let s = ui.listbox_selected(&lb).and_then(|i| ui.listbox_entries(&lb).get(i).map(|e| e.param.clone()))?;
        let (a, b) = s.split_once('x')?;
        Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
    }

    /// 0x683920 -> 0x6612E0 case 1.
    fn describe(&mut self, ui: &mut Ui, data: &GameData) {
        let Some(rmc) = self.rmc().cloned() else { return };
        let size = self.size(ui).unwrap_or((0.0, 0.0));
        let mut lines = vec![ui.strings.resolve(&rmc.name)];
        let st = ui.strings.format("staging_map_preview_map_size_format_string|dd", &[&(size.0 as i32).to_string(), &(size.1 as i32).to_string()]);
        lines.push(line(ui, "#staging_map_preview_map_size", &st));
        if let Some(s) = rmc.sizes.iter().find(|s| s.0 == size.0 && s.1 == size.1).filter(|s| s.2 >= 0 && s.3 >= 0) {
            let range = if s.2 == s.3 { s.2.to_string() } else { ui.strings.format("staging_map_preview_recommended_range|dd", &[&s.2.to_string(), &s.3.to_string()]) };
            let rec = ui.strings.format("staging_map_preview_recommended|s", &[&range]);
            lines.push(line(ui, "#staging_map_preview_recommended_label_generic", &rec));
        }
        let mut rules = self.rules.clone();
        self.rules_panel.read(ui, &mut rules);
        lines.extend(rule_lines(ui, data, &rules));
        // case 1 runs the side rows straight on (0x65FB20 after 0x65D050)
        for k in rmc.kingdoms.iter().filter(|k| k.playable && !k.independent) {
            lines.push(kingdom_row(ui, data.color(&k.color), &ui.strings.resolve(&k.name)));
        }
        let desc = ui.strings.resolve(&rmc.description);
        if !desc.is_empty() {
            lines.push(String::new());
            lines.push(desc);
        }
        let lp = format!("{RD}/MapPreviewPanel/MapPreviewViewport/ItemPanel/MapPreviewLabel");
        ui.set_label(rel(&lp), &lines.join("\n"));
        aa_ui::staging::label_fit_height(ui, &lp);
        ui.viewport_refresh(&format!("{RD}/MapPreviewPanel/MapPreviewViewport"));
        ui.layout();
    }

    pub fn command(&mut self, ui: &mut Ui, data: &GameData, cmd: &str) -> Option<Outcome> {
        let arg = |c: &str| c.split_once(' ').map(|(_, a)| a.trim().trim_matches('"').to_string());
        if cmd.starts_with("SelectMapSize") {
            self.describe(ui, data);
            return Some(Outcome::Open);
        }
        if cmd.starts_with("SelectFromDouble") {
            if let Some(p) = arg(cmd) {
                self.select(ui, data, &p);
            }
            return Some(self.commit(ui));
        }
        if cmd.starts_with("Select ") {
            if let Some(p) = arg(cmd) {
                self.select(ui, data, &p);
            }
            return Some(Outcome::Open);
        }
        match cmd {
            "Commit" => Some(self.commit(ui)),
            "Cancel" => {
                close(ui, "RandomMapDialog");
                Some(Outcome::Cancelled)
            }
            _ => None,
        }
    }

    /// Commit 0x6875A0: slider values (hidden or locked terrains are left
    /// to the generator's seed), size, rules.
    fn commit(&mut self, ui: &mut Ui) -> Outcome {
        let Some(rmc) = self.rmc().cloned() else { return Outcome::Open };
        let size = self.size(ui).or(rmc.sizes.first().map(|s| (s.0, s.1))).unwrap_or((384.0, 384.0));
        let mut terrain: Vec<f32> = rmc.terrains.iter().map(|t| t.range.0 + 0.5 * (t.range.1 - t.range.0)).collect();
        for (i, tag) in &self.sliders {
            if let Some(p) = ui.find_tag(*tag) {
                if let Some(v) = ui.slider_value(&format!("{p}/RangeSlider")) {
                    terrain[*i] = v;
                }
            }
        }
        let mut rules = self.rules.clone();
        self.rules_panel.read(ui, &mut rules);
        // the host stores the profile RMCPreferences (0x41F140)
        self.prefs.insert(rmc.ids.clone(), RmcPrefs { size, terrain: terrain.clone() });
        close(ui, "RandomMapDialog");
        Outcome::Random { rmc: rmc.ids, size, terrain, rules, prefs: self.prefs.clone() }
    }
}

fn close(ui: &mut Ui, name: &str) {
    ui.disconnect(name);
    ui.layout();
}
