//! The score screen after a battle (menu slot 5, UI/Menus/scoring.tgi
//! `Scoring`): ctor `0x6594D0`, Init `0x659890`, Activate `0x6590A0` ->
//! `0x658FF0`, commands ScoringSaveFilm `0x658F90`, FilmCancel `0x658F80`,
//! ScoringDebriefing `0x658FD0`, ScoringExit `0x659290`,
//! UpdateSaveFilmButton `0x658FB0`, ScoringReplay `0x6590B0`.
//!
//! * ScoreChart (`0x657F70`, Init `0x658100`, LoadData `0x657D20`, Activate
//!   `0x657DD0`): one Tab toggle per scoring tab (L = i * tab_width +
//!   tab_left_border), `SelectScoreChartTab %i` (`0x657E70` -> `0x657BB0`)
//!   swaps the one connected ScoreSheet;
//! * ScoreSheet (`0x65A580`, LoadData `0x65A300`, Init `0x65A780`, Activate
//!   `0x65A460`): a ColumnHeader toggle per statistic of the tab (radio
//!   group, `SelectScoreGraphData %i`), the ViewportPanel/Viewport with one
//!   DataRowEven / DataRowOdd per kingdom of the world (GWorld+0x154);
//!   Activate selects the first header and runs its command;
//! * DataRow (`0x65AAA0`, LoadData `0x65AA40`, Init `0x65ABA0`, Activate
//!   `0x65B170` -> `0x65ADA0`): the RowHeader toggle (the kingdom name of
//!   `0x4E9560`, `ToggleScoreGraphKingdom %i`, checked while the graph shows
//!   the kingdom) and a Datum label per statistic of the newest snapshot
//!   (INT `%i` of floor, FIXED `%.0f`, STRING the favourite or
//!   `score_graph_none`; no snapshot reads 0);
//! * ScoreGraph (`0x658D80`, LoadData `0x658700`, Init `0x6588C0`, Activate
//!   `0x658430`, `SelectScoreGraphData` `0x658D40`,
//!   `ToggleScoreGraphKingdom` `0x658500` -> `0x5EC120`) and its GraphVE,
//!   the SCORE_GRAPH view element (`0x5ECC60`, Load `0x5EC030`, ranges
//!   `0x5EC140`, lines `0x5EC5C0`, Draw `0x5ECA60`), with the XTickLabel /
//!   YTickLabel labels of `0x658960`.
//!
//! The statistics come from Data/Scoring/Scoring.tgi (StatData registry: the
//! 62 predefined statistics of `0x58E1A0` first, then the new IDS in file
//! order; ScoringTab `0x4B58E0`: default_graphed_stat then its [Statistic]
//! list; ScoringPanel `0x4B5B80`: default_tab then its [Tab] list). The
//! values are the kingdoms' statistics snapshots (GKingdom+0x338 list,
//! oldest first; the sheet reads the newest, +0x33C).
use super::datafile::{self, Node};
use aa_ui::draw::{Blend, DrawCmd, DrawKind, DrawList, Quad};
use aa_ui::layout::{Rect, resolve_bounds, ui_scale};
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};
use std::collections::HashMap;

pub const FILE: &str = "UI/Menus/scoring.tgi";
const IFACE: &str = "UI/Menus/scoring.tgi:Scoring";
/// `0xA51FB4`: the tick step mantissas.
const STEPS: [i32; 3] = [1, 2, 5];
/// `0xA11740`: the graph's kingdom slots (+0x130 / +0x140 arrays).
const GRAPH_KINGDOMS: usize = 16;

const TAG_TAB: u64 = 9100;
const TAG_HEADER: u64 = 9200;
const TAG_ROW: u64 = 9300;

/// StatData display types (+0x18): INT 0, FIXED 1, STRING 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    Int,
    Fixed,
    Str,
}

#[derive(Clone, Debug)]
pub struct StatDef {
    pub ids: String,
    /// StatData registry index (the snapshot value index).
    pub index: usize,
    pub display: Display,
    /// Localization key of `name`.
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct TabDef {
    pub name: String,
    /// Statistics in column order (indices into [`Defs::stats`]).
    pub stats: Vec<usize>,
}

/// The scoring definitions of Data/Scoring/Scoring.tgi.
#[derive(Clone, Debug, Default)]
pub struct Defs {
    pub stats: Vec<StatDef>,
    /// Panel order.
    pub tabs: Vec<TabDef>,
}

/// Template chain fields of a node (`template` / `inherit`), the node's own
/// fields on top.
fn resolved_field(templates: &HashMap<String, Node>, n: &Node, key: &str) -> Option<String> {
    if let Some(v) = n.get(key) {
        return Some(v.to_string());
    }
    let mut parent = n.template().map(str::to_ascii_lowercase);
    for _ in 0..16 {
        let t = templates.get(parent.as_deref()?)?;
        if let Some(v) = t.get(key) {
            return Some(v.to_string());
        }
        parent = t.template().map(str::to_ascii_lowercase);
    }
    None
}

impl Defs {
    pub fn load(data: &aa_ui::fs::DataRoot) -> Self {
        let mut templates = HashMap::new();
        for n in datafile::load(data, "Templates/template_scoring.tgi") {
            if n.name.eq_ignore_ascii_case("Template") {
                if let Some(name) = n.args.first() {
                    templates.insert(name.to_ascii_lowercase(), n.clone());
                }
            }
        }
        let nodes = datafile::load(data, "Scoring/Scoring.tgi");
        // 0x58E1A0 then the registry's appended IDS
        let mut names: Vec<String> = aa_rules::native::statistics::PREDEFINED_STATISTICS.iter().map(|s| s.to_ascii_lowercase()).collect();
        let mut defs = Defs::default();
        for n in nodes.iter().filter(|n| n.name.eq_ignore_ascii_case("Statistic")) {
            let ids = n.get("ids").unwrap_or_default().to_string();
            if ids.is_empty() {
                continue;
            }
            let key = ids.to_ascii_lowercase();
            let index = match names.iter().position(|x| *x == key) {
                Some(i) => i,
                None => {
                    names.push(key);
                    names.len() - 1
                }
            };
            let display = match resolved_field(&templates, n, "display_type").unwrap_or_else(|| "INT".into()).to_ascii_uppercase().as_str() {
                "FIXED" => Display::Fixed,
                "STRING" => Display::Str,
                _ => Display::Int,
            };
            defs.stats.push(StatDef { ids, index, display, name: n.get("name").unwrap_or_default().to_string() });
        }
        let stat = |ids: &str, defs: &Defs| defs.stats.iter().position(|s| s.ids.eq_ignore_ascii_case(ids.trim()));
        let mut tabs: Vec<(String, TabDef)> = Vec::new();
        for n in nodes.iter().filter(|n| n.name.eq_ignore_ascii_case("ScoringTab")) {
            // 0x4B58E0: default_graphed_stat, then each [Statistic] stat_ids
            let mut list = Vec::new();
            if let Some(i) = n.get("default_graphed_stat").and_then(|s| stat(s, &defs)) {
                list.push(i);
            }
            for c in n.children_named("Statistic") {
                if let Some(i) = c.get("stat_ids").and_then(|s| stat(s, &defs)) {
                    list.push(i);
                }
            }
            tabs.push((n.get("ids").unwrap_or_default().to_ascii_lowercase(), TabDef { name: n.get("name").unwrap_or_default().to_string(), stats: list }));
        }
        let tab = |ids: &str| tabs.iter().find(|(i, _)| *i == ids.trim().to_ascii_lowercase()).map(|(_, t)| t.clone());
        // 0x4B5B80: default_tab, then each [Tab] tab_ids
        if let Some(p) = nodes.iter().find(|n| n.name.eq_ignore_ascii_case("ScoringPanel")) {
            if let Some(t) = p.get("default_tab").and_then(tab) {
                defs.tabs.push(t);
            }
            for c in p.children_named("Tab") {
                if let Some(t) = c.get("tab_ids").and_then(tab) {
                    defs.tabs.push(t);
                }
            }
        }
        defs
    }
}

/// One kingdom of the finished world as the score screen reads it.
#[derive(Clone, Debug, Default)]
pub struct KingdomScores {
    /// `0x4E9560` (no colour): the kingdom glyph prefix of `0x4E93C0`
    /// (`<color=%g,%g,%g>%c` unless independent) and ` <rc>` + the name.
    pub name: String,
    /// GKingdom+0x1E8 colour (+0x18..+0x20).
    pub color: [f32; 3],
    /// The statistics snapshots (+0x338), oldest first: each the `0x58F3C0`
    /// value by StatData index.
    pub snapshots: Vec<Vec<f32>>,
    /// The newest snapshot's favourites (`0x58F820`) by StatData index.
    pub favorites: HashMap<usize, String>,
}

/// The finished world kept for the score screen (the original keeps the
/// world until the next one replaces it: StagingOpenScoring `0x65BD60`
/// needs `0xB36534`).
#[derive(Clone, Debug, Default)]
pub struct ScoreData {
    /// GWorld+0x154 in order (`None` = a null slot).
    pub kingdoms: Vec<Option<KingdomScores>>,
    /// The session kind (`[0xB39690+0xFC]+0xC`: 0 campaign, 1 custom).
    pub kind: i32,
    pub networked: bool,
    /// A film of the game exists (`0x772BB0`): shows SaveFilm (`0x658FF0`,
    /// UpdateSaveFilmButton). The hook of the film work: `false` until games
    /// are recorded, and with it the button stays hidden.
    pub film: bool,
}

impl ScoreData {
    fn value(&self, k: usize, snapshot: usize, index: usize) -> f32 {
        self.kingdoms.get(k).and_then(|x| x.as_ref()).and_then(|x| x.snapshots.get(snapshot)).and_then(|s| s.get(index)).copied().unwrap_or(0.0)
    }
}

/// What a score screen command asks the front end to do.
pub enum Outcome {
    None,
    /// ScoringExit (`0x659290`): the staging (slot 3) for a custom game.
    Exit,
    /// ScoringReplay (`0x6590B0` -> `0x641660` / `0x652960`): launch the
    /// session's settings again.
    Replay,
    /// ScoringDebriefing (`0x658FD0`): the debriefing (slot 4).
    Debriefing,
}

/// The SCORE_GRAPH view element's ranges (`0x5EC140`).
#[derive(Clone, Copy, Debug, Default)]
struct Ranges {
    /// +0xE8 the most snapshots of a kingdom.
    count: i32,
    /// +0xF4 / +0xF8: x ticks and their snapshot step.
    xticks: i32,
    xstep: i32,
    /// +0x114 / +0x118 / +0x11C: ticks below / above zero and their step.
    ylow: i32,
    yhigh: i32,
    ystep: i32,
    /// +0xFC / +0x100: pixels per snapshot and per x tick.
    x_px: f32,
    xtick_px: f32,
    /// +0x120 / +0x124: pixels per value unit and per y tick.
    y_px: f32,
    ytick_px: f32,
}

/// The score screen.
pub struct Scoring {
    data: ScoreData,
    /// ScoreChart +0x88: the connected sheet.
    tab: usize,
    /// GraphVE +0x13C: the graphed statistic (index into [`Defs::stats`]).
    stat: Option<usize>,
    /// GraphVE +0x130: the kingdoms drawn (initially all, `0x5ECBF0`).
    shown: Vec<bool>,
    /// GraphVE Load (`0x5EC030`): colour, tick lengths and maxima.
    color: [f32; 3],
    x_tick_length: f32,
    x_max_ticks: i32,
    y_max_ticks: i32,
    ranges: Ranges,
}

fn label(name: &str) -> Widget {
    Widget::new(C::Label, name, false)
}

/// A toggle button whose client is a code-set label (`0x5CF4F0` +
/// `0x5F3160`).
fn label_toggle(name: &str, command: &str) -> Widget {
    let mut t = Widget::new(C::ToggleButton, name, false);
    t.command = Some(command.to_string());
    t.add_child(label("Client"));
    t
}

fn rel(path: &str) -> &str {
    path.strip_prefix("Root/").unwrap_or(path)
}

/// Load a code-built item against `parent_node` (`file:node/...`).
fn load_item(ui: &mut Ui, item: &mut Widget, parent_node: &str) {
    if let Ok(n) = ui.store.node(parent_node) {
        let strings = ui.strings.clone();
        let resolve = |v: &str| strings.resolve(v);
        let root = std::sync::Arc::new(n.clone());
        item.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
    }
}

/// Load a code-built child of the node at `parent` (a path in
/// scoring.tgi) with the file's root for `/Name` lookups.
fn load_in_file(ui: &mut Ui, item: &mut Widget, parent: &str) {
    let Ok(root) = ui.store.load_file(FILE) else { return };
    let Some(n) = aa_ui::tgi::find_path(&root, parent).cloned() else { return };
    let strings = ui.strings.clone();
    let resolve = |v: &str| strings.resolve(v);
    item.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
}

fn node_float(ui: &mut Ui, node: &str, key: &str, default: f32) -> f32 {
    ui.store.node(node).ok().and_then(|n| n.float(key).ok()).unwrap_or(default)
}

/// `0x5EEFA0` SetLeft / `0x5EF000` SetTop on a laid-out widget.
fn set_left(ui: &mut Ui, path: &str, x: f32) {
    let (screen, reference, p) = (ui.screen, ui.reference, ui.precision);
    ui.with_widget(rel(path), |w| aa_ui::geometry::set_left(w, x, screen, reference, p));
}

fn set_top(ui: &mut Ui, path: &str, y: f32) {
    let (screen, reference, p) = (ui.screen, ui.reference, ui.precision);
    ui.with_widget(rel(path), |w| aa_ui::geometry::set_top(w, y, screen, reference, p));
}

fn set_selected(ui: &mut Ui, path: &str, on: bool) {
    ui.with_widget(rel(path), |w| {
        if on {
            w.flags |= aa_ui::state::widget_flag::SELECTED;
        } else {
            w.flags &= !aa_ui::state::widget_flag::SELECTED;
        }
    });
}

impl Scoring {
    /// Init `0x659890` and Activate `0x6590A0`.
    pub fn show(ui: &mut Ui, defs: &Defs, data: ScoreData) -> Self {
        let mut s = Scoring {
            data,
            tab: 0,
            stat: None,
            shown: vec![true; GRAPH_KINGDOMS],
            color: [1.0; 3],
            x_tick_length: 0.0,
            x_max_ticks: 2,
            y_max_ticks: 3,
            ranges: Ranges::default(),
        };
        s.build(ui, defs);
        s
    }

    /// (Re)build the interface keeping the tab, graphed statistic and shown
    /// kingdoms (a new screen size rebuilds every interface).
    pub fn rebuild(&mut self, ui: &mut Ui, defs: &Defs) {
        self.build(ui, defs);
    }

    fn build(&mut self, ui: &mut Ui, defs: &Defs) {
        ui.clear();
        let mut iface = Widget::new(C::Interface, IFACE, false);
        // Header: only with the AVar UIScoringScreenUseTitle (registered
        // false at 0x4045BD)
        if ui.avars.flag("UIScoringScreenUseTitle").unwrap_or(false) {
            iface.add_child(label("Header"));
        }
        let mut graph = Widget::new(C::Panel, "/ScoreGraph", false);
        // Init 0x6588C0: YLabel only with UIScoringGraphUseYAxisLabel
        // (index 0x5F, registered false at 0x403D79)
        if ui.avars.flag("UIScoringGraphUseYAxisLabel").unwrap_or(false) {
            graph.add_child(label("YLabel"));
        }
        iface.add_child(graph);
        // ScoreChart Init 0x658100: a Tab per tab, then the first sheet
        let mut chart = Widget::new(C::Panel, "/ScoreChart", false);
        for (i, _) in defs.tabs.iter().enumerate() {
            let mut t = label_toggle("Tab", &format!("SelectScoreChartTab {i}"));
            t.radio_group = 1;
            t.tag = TAG_TAB + i as u64;
            chart.add_child(t);
        }
        if let Some(tab) = defs.tabs.get(self.tab) {
            chart.add_child(Self::sheet(tab, defs));
        }
        iface.add_child(chart);
        for (name, command) in [
            ("/SaveFilm", "ScoringSaveFilm"),
            ("/Debriefing", "ScoringDebriefing"),
            ("/Exit", "ScoringExit"),
            ("/ScoringReplay", "ScoringReplay"),
        ] {
            iface.add_child(Widget::push_button(name, false, command));
        }
        aa_ui::screens::show(ui, iface);
        ui.layout();
        // ScoreChart LoadData 0x657D20: the tab names; Activate 0x657DD0:
        // L = i * tab_width + tab_left_border
        let chart_node = format!("{FILE}:ScoreChart");
        let tab_left = node_float(ui, &chart_node, "tab_left_border", 0.0);
        let tab_width = node_float(ui, &chart_node, "tab_width", 0.0);
        for (i, t) in defs.tabs.iter().enumerate() {
            let Some(p) = ui.find_tag(TAG_TAB + i as u64) else { continue };
            let text = ui.strings.resolve(&t.name);
            ui.set_label(rel(&p), &text);
            set_left(ui, &p, i as f32 * tab_width + tab_left);
            set_selected(ui, &p, i == self.tab);
        }
        // Activate 0x658FF0: SaveFilm while a film exists (0x772BB0),
        // Debriefing for a campaign (kind 0) else Exit, Replay offline
        let (film, campaign, networked) = (self.data.film, self.data.kind == 0, self.data.networked);
        ui.with_widget("Scoring/SaveFilm", |w| w.set_visible(film));
        ui.with_widget("Scoring/Debriefing", |w| w.set_visible(campaign));
        ui.with_widget("Scoring/Exit", |w| w.set_visible(!campaign));
        ui.with_widget("Scoring/ScoringReplay", |w| w.set_visible(!networked));
        // GraphVE Load 0x5EC030
        if let Ok(n) = ui.store.node(&format!("{FILE}:ScoreGraph/GraphVE")) {
            if let Ok(c) = n.point3("color") {
                self.color = c;
            }
            self.x_tick_length = n.float("x_tick_length").unwrap_or(0.0);
            self.x_max_ticks = n.int("x_max_ticks").unwrap_or(2).max(2);
            self.y_max_ticks = n.int("y_max_ticks").unwrap_or(3).max(3);
        }
        ui.layout();
        self.activate_sheet(ui, defs);
    }

    /// ScoreSheet Init `0x65A780`: a ColumnHeader per statistic (radio
    /// group, `SelectScoreGraphData %i`), then ViewportPanel/Viewport.
    fn sheet(tab: &TabDef, defs: &Defs) -> Widget {
        let mut sheet = Widget::new(C::Panel, "/ScoreSheet", false);
        for (i, &s) in tab.stats.iter().enumerate() {
            let index = defs.stats[s].index;
            // "%s %i" with "SelectScoreGraphData " (0xA6767C)
            let mut h = label_toggle("ColumnHeader", &format!("SelectScoreGraphData  {index}"));
            h.radio_group = 1;
            h.tag = TAG_HEADER + i as u64;
            sheet.add_child(h);
        }
        let mut panel = Widget::new(C::Panel, "ViewportPanel", false);
        panel.add_child(Widget::composite(C::Viewport, "Viewport", false));
        sheet.add_child(panel);
        sheet
    }

    /// ScoreSheet LoadData `0x65A300` + Activate `0x65A460`, then the first
    /// header's command (`0x5F04C0` + `0x5F0270`).
    fn activate_sheet(&mut self, ui: &mut Ui, defs: &Defs) {
        let Some(tab) = defs.tabs.get(self.tab).cloned() else { return };
        let sheet_node = format!("{FILE}:ScoreSheet");
        let col_left = node_float(ui, &sheet_node, "column_left_border", 0.0);
        let col_width = node_float(ui, &sheet_node, "column_width", 0.0);
        for (i, &s) in tab.stats.iter().enumerate() {
            let Some(p) = ui.find_tag(TAG_HEADER + i as u64) else { continue };
            let text = ui.strings.resolve(&defs.stats[s].name);
            ui.set_label(rel(&p), &text);
        }
        // the rows: DataRowEven for even kingdom indices, DataRowOdd for odd
        let vp = "Root/Scoring/ScoreChart/ScoreSheet/ViewportPanel/Viewport";
        let _ = ui.viewport_clear(vp);
        let pool_node = format!("{FILE}:ScoreSheet/ViewportPanel/Viewport");
        for (k, kingdom) in self.data.kingdoms.iter().enumerate() {
            if kingdom.is_none() {
                continue;
            }
            let name = if k % 2 == 0 { "DataRowEven" } else { "DataRowOdd" };
            // DataRow Init 0x65ABA0: RowHeader (+Client), then a Datum each
            let mut row = Widget::new(C::Panel, name, false);
            row.add_child(label_toggle("RowHeader", "ToggleScoreGraphKingdom -1"));
            for _ in &tab.stats {
                row.add_child(label("Datum"));
            }
            load_item(ui, &mut row, &pool_node);
            row.tag = TAG_ROW + k as u64;
            ui.viewport_add(vp, row);
        }
        ui.layout();
        // header L = i * column_width + column_left_border
        for i in 0..tab.stats.len() {
            if let Some(p) = ui.find_tag(TAG_HEADER + i as u64) {
                set_left(ui, &p, i as f32 * col_width + col_left);
            }
        }
        // DataRow LoadData 0x65AA40 / Activate 0x65B170 -> 0x65ADA0
        let row_node = format!("{FILE}:ScoreSheet/ViewportPanel/Viewport/DataRowEven");
        let d_left = node_float(ui, &row_node, "column_left_border", 0.0);
        let d_width = node_float(ui, &row_node, "column_width", 0.0);
        for k in 0..self.data.kingdoms.len() {
            self.fill_row(ui, defs, &tab, k, d_left, d_width);
        }
        ui.viewport_refresh(vp);
        ui.layout();
        // 0x5F04C0(header[0]) + 0x5F0270: the tab's default statistic
        if let Some(p) = ui.find_tag(TAG_HEADER) {
            set_selected(ui, &p, true);
            if let Some(&s) = tab.stats.first() {
                self.select_stat(ui, defs, s);
            }
        }
    }

    /// `0x65ADA0`: the row header text, command and check, the data.
    fn fill_row(&self, ui: &mut Ui, defs: &Defs, tab: &TabDef, k: usize, d_left: f32, d_width: f32) {
        let Some(Some(kingdom)) = self.data.kingdoms.get(k) else { return };
        let Some(row) = ui.find_tag(TAG_ROW + k as u64) else { return };
        let header = format!("{row}/RowHeader");
        let name = kingdom.name.clone();
        ui.set_label(rel(&header), &name);
        let shown = self.shown.get(k).copied().unwrap_or(false);
        ui.with_widget(rel(&header), |w| w.command = Some(format!("ToggleScoreGraphKingdom {k}")));
        set_selected(ui, &header, shown);
        let snapshots = kingdom.snapshots.len();
        let none = ui.strings.resolve("#score_graph_none");
        for (i, &s) in tab.stats.iter().enumerate() {
            let def = &defs.stats[s];
            let v = if snapshots == 0 { 0.0 } else { self.data.value(k, snapshots - 1, def.index) };
            let text = match def.display {
                Display::Int => format!("{}", v.floor() as i32),
                Display::Fixed => format!("{v:.0}"),
                Display::Str => {
                    if snapshots == 0 {
                        none.clone()
                    } else {
                        kingdom.favorites.get(&def.index).cloned().unwrap_or_else(|| none.clone())
                    }
                }
            };
            let seg = if i == 0 { "Datum".to_string() } else { format!("Datum#{i}") };
            let dp = format!("{row}/{seg}");
            ui.set_label(rel(&dp), &text);
            set_left(ui, &dp, i as f32 * d_width + d_left);
        }
    }

    /// The GraphVE rectangle (window pixels): its bounds against the
    /// ScoreGraph's root view.
    fn graph_rect(&self, ui: &mut Ui) -> Option<Rect> {
        let parent = ui.widget("Root/Scoring/ScoreGraph").map(|w| w.root.outer.rect)?;
        let node = ui.store.node(&format!("{FILE}:ScoreGraph/GraphVE")).ok()?;
        let ve = aa_ui::ve::ViewElement::load(&node).ok()?;
        let mut b = ve.layout.outer;
        let (sx, sy) = ui_scale(ui.screen, ui.reference, ui.precision);
        resolve_bounds(&mut b, &parent, sx, sy, false, ui.precision);
        Some(b.rect)
    }

    /// `SelectScoreGraphData` (`0x658D40`): clear the tick labels
    /// (`0x658520`), the statistic and ranges (`0x5EC4D0` -> `0x5EC140`),
    /// the new tick labels (`0x658960`).
    fn select_stat(&mut self, ui: &mut Ui, defs: &Defs, s: usize) {
        self.stat = Some(s);
        let Some(rect) = self.graph_rect(ui) else { return };
        self.ranges = self.ranges(defs, rect);
        // 0x658520: the old tick labels
        let graph = "Root/Scoring/ScoreGraph";
        if let Some(w) = ui.widget_mut(graph) {
            w.children.retain(|c| !(c.name.eq_ignore_ascii_case("XTickLabel") || c.name.eq_ignore_ascii_case("YTickLabel")));
        }
        if ui.avars.flag("UIScoringGraphUseYAxisLabel").unwrap_or(false) {
            let text = ui.strings.resolve(&defs.stats[s].name);
            ui.set_label("Scoring/ScoreGraph/YLabel", &text);
        }
        let (sx, sy) = ui_scale(ui.screen, ui.reference, ui.precision);
        let Some(wr) = ui.widget(graph).map(|w| w.root.outer.rect) else { return };
        // Activate 0x658430: the GraphVE offsets in the widget
        let off_l = (rect.l - wr.l) / sx;
        let off_b = (rect.b - wr.t) / sy;
        let r = self.ranges;
        let node = format!("{FILE}:ScoreGraph");
        // x: "%i" of i * step at i * tick spacing, centred
        let spacing = r.xtick_px / sx;
        for i in 0..r.xticks {
            let mut l = label("XTickLabel");
            load_item(ui, &mut l, &node);
            l.tag = 9500 + i as u64;
            let Some(p) = ui.attach(graph, l) else { continue };
            ui.set_label(rel(&p), &format!("{}", i * r.xstep));
            ui.layout();
            let width = ui.widget(&p).map(|w| w.root.outer.raw[2] - w.root.outer.raw[0]).unwrap_or(0.0);
            set_left(ui, &p, i as f32 * spacing + off_l - (width - 1.0) * 0.5);
        }
        // y: "%i" of j * step from -low to high, centred on the tick
        let spacing = r.ytick_px / sy;
        let zero = off_b - r.ylow as f32 * spacing;
        let mut j = -r.ylow;
        while j <= r.yhigh {
            let mut l = label("YTickLabel");
            load_item(ui, &mut l, &node);
            l.tag = 9600 + (j + r.ylow) as u64;
            if let Some(p) = ui.attach(graph, l) {
                ui.set_label(rel(&p), &format!("{}", j * r.ystep));
                ui.layout();
                let height = ui.widget(&p).map(|w| w.root.outer.raw[3] - w.root.outer.raw[1]).unwrap_or(0.0);
                set_top(ui, &p, (zero - j as f32 * spacing) - (height - 1.0) * 0.5);
            }
            j += 1;
        }
        ui.layout();
    }

    /// `0x5EC140` for the graphed statistic over the GraphVE `rect`.
    fn ranges(&self, defs: &Defs, rect: Rect) -> Ranges {
        let Some(def) = self.stat.map(|s| &defs.stats[s]) else { return Ranges::default() };
        // the kingdoms in reverse order, each its statistic per snapshot
        let series: Vec<Vec<f32>> = (0..self.data.kingdoms.len())
            .rev()
            .filter_map(|k| self.data.kingdoms.get(k).and_then(|x| x.as_ref()).map(|kd| (0..kd.snapshots.len()).map(|i| self.data.value(k, i, def.index)).collect()))
            .collect();
        ranges(&series, def.display == Display::Str, self.x_max_ticks, self.y_max_ticks, rect)
    }
}

/// `0x5EC140`: the snapshot count, the floor of the least and the ceiling
/// of the greatest value (0 included; STRING statistics have no values),
/// the x tick step from {1, 2, 5} x 10^k (`0xA51FB4`) until the ticks fit
/// x_max_ticks, the y step likewise until below + 1 + above fit
/// y_max_ticks, and the pixel scales of the GraphVE rectangle.
fn ranges(series: &[Vec<f32>], strings: bool, x_max_ticks: i32, y_max_ticks: i32, rect: Rect) -> Ranges {
    let mut r = Ranges::default();
    {
        let (mut min, mut max) = (0i32, 0i32);
        for values in series {
            if !strings {
                for &v in values {
                    if v < min as f32 {
                        min = v.floor() as i32;
                    }
                    if (max as f32) < v {
                        max = v.ceil() as i32;
                    }
                }
            }
            r.count = r.count.max(values.len() as i32);
        }
        let bump = |i: &mut usize, e: &mut u32| {
            *i += 1;
            if *i > 2 {
                *i = 0;
                *e += 1;
            }
            STEPS[*i] * 10i32.pow(*e)
        };
        let (mut i, mut e) = (0usize, 0u32);
        r.xstep = 1;
        let mut xticks;
        loop {
            xticks = r.count / r.xstep;
            r.xticks = xticks;
            if xticks <= x_max_ticks {
                break;
            }
            r.xstep = bump(&mut i, &mut e);
        }
        let (mut i, mut e) = (0usize, 0u32);
        r.ystep = 1;
        loop {
            r.yhigh = 0;
            r.ylow = 0;
            if max > 0 {
                r.yhigh = max / r.ystep + i32::from(max % r.ystep != 0);
            }
            if min < 0 {
                r.ylow = -(min / r.ystep) + i32::from(-min % r.ystep != 0);
            }
            if r.ylow + 1 + r.yhigh <= y_max_ticks {
                break;
            }
            r.ystep = bump(&mut i, &mut e);
        }
        if xticks < 2 {
            r.xticks = 2;
        }
        if r.ylow + r.yhigh < 1 {
            r.yhigh = 1;
            if min < 0 {
                r.ylow = 1;
            }
        }
        let mut w = rect.r - rect.l;
        if r.count > 1 {
            w /= (r.count - 1) as f32;
        }
        r.x_px = w;
        r.xtick_px = r.xstep as f32 * w;
        let mut h = rect.b - rect.t;
        let span = (r.yhigh * r.ystep) as f32 - (-(r.ylow * r.ystep)) as f32;
        if span != 0.0 {
            h /= span;
        }
        r.y_px = h;
        r.ytick_px = r.ystep as f32 * h;
    }
    r
}

impl Scoring {
    /// Score screen commands; `None` when the command is not one.
    pub fn command(&mut self, ui: &mut Ui, defs: &Defs, cmd: &str) -> Option<Outcome> {
        let mut it = cmd.split_whitespace();
        let name = it.next()?;
        let arg = it.next().and_then(|a| a.parse::<i64>().ok());
        Some(match name {
            "SelectScoreChartTab" => {
                // 0x657BB0: the sheet of tab i replaces the connected one
                let i = arg.unwrap_or(0).max(0) as usize;
                if i < defs.tabs.len() && i != self.tab {
                    self.tab = i;
                    if let Some(w) = ui.widget_mut("Root/Scoring/ScoreChart") {
                        w.children.retain(|c| !c.name.trim_start_matches('/').eq_ignore_ascii_case("ScoreSheet"));
                    }
                    let mut sheet = Self::sheet(&defs.tabs[i], defs);
                    load_in_file(ui, &mut sheet, "ScoreChart");
                    let _ = ui.attach("Root/Scoring/ScoreChart", sheet);
                    ui.layout();
                    self.activate_sheet(ui, defs);
                }
                Outcome::None
            }
            "SelectScoreGraphData" => {
                if let Some(s) = arg.and_then(|a| defs.stats.iter().position(|d| d.index as i64 == a)) {
                    self.select_stat(ui, defs, s);
                }
                Outcome::None
            }
            "ToggleScoreGraphKingdom" => {
                // 0x5EC120: flip the kingdom's byte
                if let Some(k) = arg.filter(|&k| k >= 0).map(|k| k as usize) {
                    if let Some(v) = self.shown.get_mut(k) {
                        *v = !*v;
                    }
                }
                Outcome::None
            }
            "ScoringExit" => Outcome::Exit,
            "ScoringReplay" => Outcome::Replay,
            "ScoringDebriefing" => Outcome::Debriefing,
            // 0x658F90: the film save dialog 0x68DCE0(5, 0, 0). Refused until
            // films are recorded (the button is hidden without one, `film`).
            "ScoringSaveFilm" => {
                bevy::log::warn!("ScoringSaveFilm refused: the film save dialog (0x68DCE0) is not built");
                Outcome::None
            }
            // 0x658F80: closes that dialog (0x68C190)
            "FilmCancel" => Outcome::None,
            "UpdateSaveFilmButton" => {
                let film = self.data.film;
                ui.with_widget("Scoring/SaveFilm", |w| w.set_visible(film));
                Outcome::None
            }
            _ => return None,
        })
    }

    /// The GraphVE's lines (Draw `0x5ECA60`, lines `0x5EC5C0`), placed after
    /// the ScoreGraph's own view and before its tick labels.
    pub fn draw(&self, ui: &mut Ui, defs: &Defs, list: &mut DrawList) {
        let Some(rect) = self.graph_rect(ui) else { return };
        let mut quads: Vec<Quad> = Vec::new();
        let c = [self.color[0], self.color[1], self.color[2], 1.0];
        // the axes: (L,T) -> (L,B) -> (R,B)
        line(&mut quads, (rect.l, rect.t), (rect.l, rect.b), c);
        line(&mut quads, (rect.l, rect.b), (rect.r, rect.b), c);
        if let Some(def) = self.stat.map(|s| &defs.stats[s]) {
            let r = self.ranges;
            let half = (self.x_tick_length - 1.0) * 0.5;
            for i in 0..r.xticks {
                let x = i as f32 * r.xtick_px + rect.l;
                line(&mut quads, (x, rect.b + half), (x, rect.b - half), c);
            }
            // the y ticks use x_tick_length too (0x5EC5C0 reads +0xEC)
            let zero = rect.b - r.ylow as f32 * r.ytick_px;
            let mut j = -r.ylow;
            while j <= r.yhigh {
                let y = zero - j as f32 * r.ytick_px;
                line(&mut quads, (rect.l - half, y), (rect.l + half, y), c);
                j += 1;
            }
            if def.display != Display::Str && r.count > 1 {
                for k in (0..self.data.kingdoms.len()).rev() {
                    let Some(Some(kd)) = self.data.kingdoms.get(k) else { continue };
                    if !self.shown.get(k).copied().unwrap_or(false) {
                        continue;
                    }
                    let kc = [kd.color[0], kd.color[1], kd.color[2], 1.0];
                    let pts: Vec<(f32, f32)> =
                        (0..kd.snapshots.len()).map(|i| (i as f32 * r.x_px + rect.l, zero - self.data.value(k, i, def.index) * r.y_px)).collect();
                    for w in pts.windows(2) {
                        line(&mut quads, w[0], w[1], kc);
                    }
                }
            }
        }
        let at = list.commands.iter().rposition(|cmd| cmd.widget.eq_ignore_ascii_case("Scoring/ScoreGraph")).map(|i| i + 1).unwrap_or(list.commands.len());
        let cmds: Vec<DrawCmd> = quads
            .into_iter()
            .map(|q| DrawCmd { widget: "Scoring/ScoreGraph".into(), ve: "GraphVE".into(), kind: DrawKind::Quad(q) })
            .collect();
        list.commands.splice(at..at, cmds);
    }
}

/// A one-pixel line as the device draws a line list: the pixels from the
/// first point up to (not including) the second.
fn line(out: &mut Vec<Quad>, a: (f32, f32), b: (f32, f32), color: [f32; 4]) {
    let quad = |l: f32, t: f32, r: f32, bt: f32| Quad {
        texture: None,
        rect: Rect { l, t, r, b: bt },
        uv: [0.0, 0.0, 1.0, 1.0],
        color,
        blend: Blend::Alpha,
        address: Default::default(),
        clip: None,
        effect: 0,
        tint: [0.0; 3],
    };
    let (x0, y0) = (a.0.floor(), a.1.floor());
    let (x1, y1) = (b.0.floor(), b.1.floor());
    if x0 == x1 {
        let (t, bt) = if y0 <= y1 { (y0, y1) } else { (y1 + 1.0, y0 + 1.0) };
        out.push(quad(x0, t, x0 + 1.0, bt.max(t + 1.0)));
        return;
    }
    if y0 == y1 {
        let (l, r) = if x0 <= x1 { (x0, x1) } else { (x1 + 1.0, x0 + 1.0) };
        out.push(quad(l, y0, r.max(l + 1.0), y0 + 1.0));
        return;
    }
    // a diagonal: one pixel per step along the major axis
    let (dx, dy) = (x1 - x0, y1 - y0);
    let n = dx.abs().max(dy.abs()) as i32;
    for s in 0..n {
        let f = s as f32 / n as f32;
        let (x, y) = ((x0 + dx * f).round(), (y0 + dy * f).round());
        out.push(quad(x, y, x + 1.0, y + 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: Rect = Rect { l: 100.0, t: 50.0, r: 970.0, b: 245.0 };

    #[test]
    fn ranges_without_snapshots() {
        // 0x5EC140 with no snapshot: two x ticks, one step above zero
        let r = ranges(&[vec![], vec![]], false, 20, 5, RECT);
        assert_eq!((r.count, r.xticks, r.xstep), (0, 2, 1));
        assert_eq!((r.ylow, r.yhigh, r.ystep), (0, 1, 1));
        assert_eq!((r.x_px, r.xtick_px), (870.0, 870.0));
        assert_eq!((r.y_px, r.ytick_px), (195.0, 195.0));
    }

    #[test]
    fn ranges_steps() {
        // y: 20 needs a step of 5 to fit 1 + 4 <= 5 ticks (1, 2, 5)
        let r = ranges(&[vec![0.0, 5.0, 10.0, 20.0]], false, 20, 5, RECT);
        assert_eq!((r.count, r.xticks, r.xstep), (4, 4, 1));
        assert_eq!((r.ylow, r.yhigh, r.ystep), (0, 4, 5));
        assert_eq!(r.x_px, 290.0);
        assert_eq!(r.y_px, 195.0 / 20.0);
        // x: 45 snapshots with 20 ticks at most: a step of 5 (45 / 2 = 22)
        let r = ranges(&[vec![1.0; 45]], false, 20, 5, RECT);
        assert_eq!((r.xticks, r.xstep), (9, 5));
        // negative values: floor of the least, ticks below zero
        // (-3 and 2: a step of 1 needs 3 + 1 + 2 = 6 ticks, 2 gives 2 + 1 + 1)
        let r = ranges(&[vec![-2.5, 1.2]], false, 20, 5, RECT);
        assert_eq!((r.ylow, r.yhigh, r.ystep), (2, 1, 2));
        // STRING statistics graph no values
        let r = ranges(&[vec![50.0, 70.0]], true, 20, 5, RECT);
        assert_eq!((r.count, r.ylow, r.yhigh), (2, 0, 1));
    }

    #[test]
    fn scoring_definitions() {
        let Some(root) = crate::ui::reconstruction_root() else { return };
        let data = aa_ui::fs::DataRoot::with_overlay(root.join("assets/original/Data"), root.join("native-game/runtime-expanded-v14/Data"));
        let defs = Defs::load(&data);
        // Scoring.tgi: 7 tabs, overall first (default_tab), its graph the
        // overall score, then economic and military
        assert_eq!(defs.tabs.len(), 7);
        let ids = |t: &TabDef| t.stats.iter().map(|&s| defs.stats[s].ids.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&defs.tabs[0]), ["overall_score", "economic_score", "military_score"]);
        assert_eq!(ids(&defs.tabs[1]), ["FAVORITE_ENEMY", "FAVORITE_BUILDING", "FAVORITE_UNIT", "FAVORITE_ORGANIZATION"]);
        // the StatData indices: predefined first (0x58E1A0), then the new
        // IDS (7 productions at 62, 7 rates at 69, 4 scores at 76)
        let index = |ids: &str| defs.stats.iter().find(|s| s.ids == ids).map(|s| s.index);
        assert_eq!(index("UNITS_KILLED"), Some(9));
        assert_eq!(index("CITIES_OWNED"), Some(55));
        assert_eq!(index("money_production"), Some(62));
        assert_eq!(index("money_rate"), Some(69));
        assert_eq!(index("unit_limit_available"), Some(76));
        assert_eq!(index("overall_score"), Some(79));
        assert_eq!(defs.stats.iter().find(|s| s.ids == "military_score").map(|s| s.display), Some(Display::Fixed));
        assert_eq!(defs.stats.iter().find(|s| s.ids == "FAVORITE_UNIT").map(|s| s.display), Some(Display::Str));
    }
}
