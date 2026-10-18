//! The custom-game staging menu (`StagingMenu`, ctor `0x661BC0`, Init
//! `0x65D350`, Activate `0x65CB80`, Update `0x661A80`) with its world
//! parameters panel and runtime slot rows (`0x69E240`, LoadData `0x69FAF0`,
//! Update `0x69FE00`, rebuild `0x69A120`, viewport fill `0x69F950`).
//!
//! The session/world state the original reads (`[0xB36550]` session,
//! `[0xB36530]` game data) is the explicit input [`StagingState`]; the
//! port has no multiplayer session, so callers supply placeholder data.
//! Only layout and widget behaviour are ported here: every slot field is
//! shown from that state by the native rules (slot refresh `0x69DC10`,
//! `0x694A30`, `0x69D300`; mode setters `0x695A30` / `0x6958F0` /
//! `0x695770`; action panels `0x694B40` / `0x695010`).
//!
//! Slot widgets are pooled as in the original (`0x696B80`, acquire
//! `0x6919D0`): a pooled widget keeps its identity ([`Widget::tag`]) while
//! it moves in and out of the Slots viewport.

use crate::screens::{composite, show};
use crate::state::widget_flag as wf;
use crate::ui::Ui;
use crate::ve::VeKind;
use crate::widget::{Widget, WidgetClass as C};
use serde::{Deserialize, Serialize};

pub const PATH: &str = "Root/StagingMenu";
const SLOTS: &str = "Root/StagingMenu/WorldParamsPanel/Slots";

/// `0x4982D0` launch types (`session+0x60`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LaunchType {
    #[default]
    None = 0,
    RandomMap = 1,
    LoadMap = 2,
    LoadSavedGame = 3,
    LoadSavedMission = 4,
    LoadFilm = 5,
    Editor = 6,
}

/// Who occupies a kingdom slot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum Occupant {
    #[default]
    None,
    Human {
        name: String,
        /// On the local connection.
        local: bool,
        /// On the host's connection.
        on_host: bool,
        ready: bool,
        ping_ms: i32,
        /// Map download progress 0..1 while downloading.
        download: Option<f32>,
        /// A swap is pending with the local player (offered by him, by us).
        swap_from: bool,
        swap_to: bool,
    },
    Ai {
        name: String,
        /// Owning client's name (remote AIs show "%s (%s)").
        client: String,
        /// SAI id, `None` = random.
        sai: Option<String>,
        /// Controlled by the local connection (`0x765DB0`).
        local: bool,
    },
}

/// One kingdom (player slot) of a team.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct KingdomSlot {
    /// `kingdom+0x30`.
    pub closed: bool,
    /// `kingdom+0x29`.
    pub required: bool,
    /// Setup colour (`setup+0x18..0x20`).
    pub color: [f32; 3],
    /// The kingdom record name in the map preview (`rec+8`, e.g.
    /// "Eisenhower AI").
    pub record_name: String,
    /// Current nation / faction / handicap ids (`kingdom+0x18/+0x1C/+0x24`,
    /// `None` = random / middle entry).
    pub nation: Option<String>,
    pub faction: Option<String>,
    pub handicap: Option<String>,
    /// Fixed by the setup (`setup+0x18` / `+0x1C`).
    pub fixed_nation: Option<String>,
    pub fixed_faction: Option<String>,
    pub occupant: Occupant,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TeamInfo {
    /// Team name (`team+8`), `None` = no team ("Independent").
    pub name: Option<String>,
    pub kingdoms: Vec<KingdomSlot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct FactionInfo {
    pub id: String,
    pub text: String,
    pub selectable: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct NationInfo {
    pub id: String,
    pub text: String,
    pub playable: bool,
    /// `flag_icon` (`nation+0x64`).
    pub flag_icon: Option<String>,
    pub factions: Vec<FactionInfo>,
}

/// Everything the staging screen reads from the session and game data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct StagingState {
    /// `session+0xF4` (multiplayer).
    pub networked: bool,
    pub local_is_host: bool,
    /// Logged in to the online service.
    pub online: bool,
    pub launch_type: LaunchType,
    /// `session+0x54` in {3, 4}.
    pub locked: bool,
    /// Launch countdown running (`session+0x118 > 0`).
    pub countdown: bool,
    pub game_name: String,
    pub password: String,
    pub players: (i32, i32),
    /// Team formats (id, text); the UI puts "Custom" first.
    pub team_formats: Vec<(String, String)>,
    pub team_format: Option<String>,
    pub scoring_available: bool,
    pub observers_allowed: bool,
    /// Map name (`map+0x24`) for the preview title.
    pub map_name: String,
    /// Map size (width, height) for the size line.
    pub map_size: (i32, i32),
    /// Rule lines of the preview (`0x65E360`, short form), already
    /// formatted; empty when no rule is shown.
    pub rules: String,
    pub local_has_kingdom: bool,
    pub local_ready: bool,
    pub teams: Vec<TeamInfo>,
    /// Humans without a kingdom.
    pub observers: Vec<Occupant>,
    pub nations: Vec<NationInfo>,
    pub handicaps: Vec<(String, String)>,
    /// (id, text, selectable).
    pub sais: Vec<(String, String, bool)>,
    /// `session.0x76C5D0(team, local kingdom)` per team.
    pub can_join_team: Vec<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotKind {
    Team,
    Player,
    Observer,
}

#[derive(Clone, Debug)]
struct Slot {
    tag: u64,
    kind: SlotKind,
    /// (team index, kingdom index) or observer index; team index for team
    /// slots (`teams.len()` = the observer header).
    team: usize,
    index: usize,
}

/// Slot modes (`+0xF8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Open = 1,
    Closed = 2,
    Ai = 3,
    Human = 4,
    Observer = 5,
}

pub struct Staging {
    pub state: StagingState,
    pools: Vec<(Slot, Option<Widget>)>,
    next_tag: u64,
    selected: Option<u64>,
    /// Localized WP texts (`0x69FAF0`).
    texts: WpTexts,
    pub diagnostics: Vec<String>,
}

#[derive(Default, Clone)]
struct WpTexts {
    random: String,
    open: String,
    open_required: String,
    closed: String,
    closed_required: String,
    observer: String,
    no_team: String,
}

fn w(class: C, name: &str) -> Widget {
    Widget::new(class, name, false)
}

fn button(name: &str, command: &str) -> Widget {
    Widget::push_button(name, false, command)
}

fn toggle(name: &str, command: &str) -> Widget {
    Widget::toggle_button(name, false, command)
}

fn dropdown(name: &str, command: Option<&str>, functor: bool) -> Widget {
    let mut d = composite(C::DropDown, name, false);
    if let Some(crate::controls::Control::DropDown(x)) = d.control.as_deref_mut() {
        x.command = command.map(str::to_string);
        x.functor = functor;
    }
    d
}

/// The code-built tree (`0x65D350` and the panel constructors).
fn build_tree(ui: &Ui) -> Widget {
    let mut st = w(C::Interface, "UI/Menus/staging.tgi:StagingMenu");
    // 1. WorldParamsPanel 0x69E240
    let mut wp = w(C::Panel, "WorldParamsPanel");
    wp.add_child(w(C::Static, "ActionPanelSeparation"));
    let mut slots = composite(C::Viewport, "Slots", false);
    slots.key = 0.0;
    wp.add_child(slots);
    let mut tap = w(C::Panel, "TeamActionPanel");
    tap.add_child(button("ChangeTeamButton", "StagingMoveToTeam"));
    wp.add_child(tap);
    let mut kap = w(C::Panel, "KingdomActionPanel");
    for (n, c) in [
        ("ChangeName", "StagingChangeName"),
        ("Kick", "StagingKick"),
        ("Whisper", "StagingWhisper"),
        ("Ban", "StagingBan"),
        ("OfferSwap", "StagingOfferSwap"),
        ("AcceptSwap", "StagingAcceptSwap"),
        ("CancelSwap", "StagingCancelSwap"),
        ("RejectSwap", "StagingRejectSwap"),
        ("SwapWithAI", "StagingSwapWithAI"),
        ("MoveHere", "StagingMoveHere"),
        ("Observer", "StagingMoveToObserver"),
    ] {
        kap.add_child(button(n, c));
    }
    wp.add_child(kap);
    st.add_child(wp);
    // 2. StagingChatPanel 0x6673A0
    let mut chat = w(C::Panel, "StagingChatPanel");
    let mut mlp = w(C::Panel, "MessageLogPanel");
    mlp.add_child(w(C::Static, "MessageLogSeparation"));
    // 0x5FD6D0(name, 0, slot 0x43, no fade, no auto height, "ChatOpenMessage %d")
    mlp.add_child(crate::message_log::message_log("MessageLog", "MessageLogMaximumLines", false, false));
    let mut edit = composite(C::Editbox, "MessageEditbox", false);
    if let Some(e) = edit.editbox_mut() {
        e.commit_command = Some("ChatMessage".into());
    }
    mlp.add_child(edit);
    mlp.add_child(button("SendButton", "ChatMessage"));
    chat.add_child(mlp);
    // native name "PlayersListBoxPanel"; the TGI node is "PlayersListboxPanel"
    // and node lookup is case-insensitive
    let mut plp = w(C::Panel, "PlayersListBoxPanel");
    plp.add_child(w(C::Static, "PlayersListboxSeparation"));
    let mut lb = composite(C::Listbox, "PlayersListbox", false);
    if let Some(l) = lb.listbox_mut() {
        l.on_double = crate::controls::ListCallback::Command("ChatOpenPlayer %s".into());
        l.on_right = crate::controls::ListCallback::Command("ChatOpenPlayer %s".into());
    }
    plp.add_child(lb);
    let mut filter = w(C::Panel, "FilterPanel");
    for (n, c) in [("AllToggle", "ChatToggleAll"), ("TeamToggle", "ChatToggleTeam"), ("WhisperToggle", "ChatToggleWhisper")] {
        let mut t = toggle(n, c);
        t.radio_group = 1;
        filter.add_child(t);
    }
    let mut buddy = button("BuddyButton", "ChatBuddyList");
    buddy.set_visible(false);
    filter.add_child(buddy);
    plp.add_child(filter);
    chat.add_child(plp);
    st.add_child(chat);
    // 3. InfoPanel 0x692A70
    let mut info = w(C::Panel, "InfoPanel");
    info.add_child(w(C::Label, "GameNameLabel"));
    info.add_child(w(C::Label, "NumPlayersLabel"));
    info.add_child(w(C::Decal, "PasswordIndicatorDecal"));
    info.add_child(dropdown("TeamFormatDropdown", Some("TeamFormatDropdown %s"), false));
    let mut tfl = w(C::Label, "TeamFormatLabel");
    tfl.set_visible(ui.avars.flag("UIStagingUseTeamFormatLabel").unwrap_or(false));
    info.add_child(tfl);
    info.add_child(button("GameSettingsButton", "OpenSettings"));
    st.add_child(info);
    // 4. ActionPanel 0x6897F0
    let mut act = w(C::Panel, "ActionPanel");
    for (n, c) in [
        ("OptionsButton", "StagingOpenOptions"),
        ("GameOptionsButton", "OpenSettings"),
        ("ScoreButton", "StagingOpenScoring"),
        ("DetailsButton", "StagingOpenDetails"),
    ] {
        act.add_child(button(n, c));
    }
    // ActionPanel+0x84: +0xE8 auto flip cleared at the end of the ctor
    // (0x689E8F)
    let mut ready = toggle("ReadyToggle", "StagingReady %d");
    ready.no_auto_toggle = true;
    act.add_child(ready);
    for (n, c) in [("LaunchButton", "StagingLaunch"), ("AbortLaunchButton", "StagingAbortLaunch"), ("ExitButton", "StagingConfirmExit")] {
        act.add_child(button(n, c));
    }
    st.add_child(act);
    // 5. MapPreviewPanel (the label is added with viewport AddItem later)
    let mut mp = w(C::Panel, "MapPreviewPanel");
    mp.add_child(composite(C::Viewport, "MapPreviewViewport", false));
    mp.add_child(w(C::Static, "MapPreviewPanelSeparation"));
    st.add_child(mp);
    // 6. launch-type buttons
    for (n, c) in [
        ("MapButton", "SelectLaunchType LOAD_MAP"),
        ("SaveButton", "SelectLaunchType LOAD_SAVED_GAME"),
        ("FilmButton", "SelectLaunchType LOAD_FILM"),
        ("RMCButton", "SelectLaunchType RANDOM_MAP"),
    ] {
        st.add_child(button(n, c));
    }
    st
}

/// Player/observer slot children (`0x69EF80`, AddChild order).
fn player_slot(name: &str, ui: &Ui) -> Widget {
    let mut s = w(C::SelectablePanel, name);
    s.radio_group = 1;
    s.add_child(w(C::Label, "KingdomColorIcon"));
    s.add_child(w(C::Label, "Name"));
    s.add_child(w(C::Decal, "SwapRequest"));
    s.add_child(dropdown("Nation", None, true));
    s.add_child(dropdown("Faction", None, true));
    s.add_child(dropdown("AddAI", Some("StagingSelectSAI %s"), false));
    s.add_child(dropdown("Handicap", None, true));
    s.add_child(w(C::Label, "Ping"));
    // slot+0xCC: disabled, +0xE8 auto flip cleared (0x69F6A0 / 0x69F6AC)
    let mut ready = toggle("Ready", "StagingReady %d");
    ready.no_auto_toggle = true;
    s.add_child(ready);
    s.add_child(w(C::Decal, "HostIcon"));
    s.add_child(w(C::Label, "Progress"));
    s.add_child(w(C::Label, "NationLabel"));
    s.add_child(w(C::Label, "FactionLabel"));
    if ui.avars.flag("UIStagingUseNationFlagDecal").unwrap_or(false) {
        let mut flag = w(C::Decal, "NationFlagDecalIcon");
        flag.set_visible(false);
        s.add_child(flag);
    }
    s
}

/// Team slot (`0x69A5A0`): TeamName.
fn team_slot() -> Widget {
    let mut s = w(C::SelectablePanel, "TeamSlot");
    s.radio_group = 1;
    s.add_child(w(C::Label, "TeamName"));
    s
}

fn set_text(w: &mut Widget, child: &str, text: &str) {
    if let Some(c) = w.child_mut(child) {
        if let Some(crate::ve::ViewElement { kind: VeKind::Label(l), .. }) = &mut c.sub_ve {
            l.text = text.to_string();
        }
    }
}

fn visible(w: &mut Widget, child: &str, v: bool) {
    if let Some(c) = w.child_mut(child) {
        c.set_visible(v);
    }
}

impl Staging {
    /// Build, load and connect the staging menu, then run Activate and the
    /// first rebuild.
    pub fn show(ui: &mut Ui, state: StagingState) -> Self {
        ui.clear();
        let st = build_tree(ui);
        show(ui, st);
        let node = ui.store.node("UI/Menus/staging.tgi:StagingMenu/WorldParamsPanel").ok();
        let txt = |k: &str| node.as_ref().map(|n| ui.strings.resolve(&n.string_or_empty(k))).unwrap_or_default();
        let texts = WpTexts {
            random: ui.strings.resolve("#staging_and_session_random_sai_name"),
            open: txt("player_option_open_text"),
            open_required: txt("player_option_open_required_text"),
            closed: txt("player_option_closed_text"),
            closed_required: txt("player_option_closed_required_text"),
            observer: txt("kingdom_option_observer_text"),
            no_team: ui.strings.resolve("#team_management_no_Team_label"),
        };
        let mut s = Staging { state, pools: Vec::new(), next_tag: 1000, selected: None, texts, diagnostics: Vec::new() };
        s.activate(ui);
        s.rebuild(ui);
        s.update(ui);
        s
    }

    /// Activate (`0x65CB80`, InfoPanel `0x691440`, ActionPanel `0x689640`,
    /// chat `0x666440`).
    fn activate(&mut self, ui: &mut Ui) {
        let net = self.state.networked;
        let host = self.state.local_is_host;
        ui.with_widget("StagingMenu/StagingChatPanel/PlayersListBoxPanel", |w| w.set_visible(net));
        // MessageLogPanel bounds: TGI bounds when networked, else R = 100%
        // (LoadData 0x65DAB0 parsed both)
        // LoadData 0x65DAB0 parses MessageLogPanel/View twice: as written
        // (+0x128) and with R = "100%" (+0xF4)
        if let Ok(v) = ui.store.node("UI/Menus/staging.tgi:StagingMenu/StagingChatPanel/MessageLogPanel/View") {
            let g = |k: &str| v.string_or_empty(k);
            let r = if net { g("R") } else { "100%".to_string() };
            let mut nb = crate::layout::Bounds::default();
            if crate::layout::parse_bounds(&mut nb, &g("L"), &g("T"), &r, &g("B")) {
                ui.with_widget("StagingMenu/StagingChatPanel/MessageLogPanel", |w| w.root.outer = nb);
            }
        }
        for b in ["MapButton", "SaveButton", "FilmButton", "RMCButton"] {
            ui.with_widget(&format!("StagingMenu/{b}"), |w| w.set_enabled(host));
        }
        for c in ["GameNameLabel", "NumPlayersLabel", "PasswordIndicatorDecal"] {
            ui.with_widget(&format!("StagingMenu/InfoPanel/{c}"), |w| w.set_visible(net));
        }
        ui.with_widget("StagingMenu/InfoPanel/TeamFormatDropdown", |w| w.set_enabled(host));
        ui.with_widget("StagingMenu/InfoPanel/GameSettingsButton", |w| w.set_enabled(net && host));
        ui.with_widget("StagingMenu/ActionPanel/GameOptionsButton", |w| w.set_enabled(net && host));
        let score = self.state.scoring_available;
        ui.with_widget("StagingMenu/ActionPanel/ScoreButton", |w| w.set_enabled(score));
        let online = self.state.online;
        ui.with_widget("StagingMenu/StagingChatPanel/PlayersListBoxPanel/FilterPanel/BuddyButton", |w| w.set_visible(online));
        ui.set_flag("Root/StagingMenu/StagingChatPanel/PlayersListBoxPanel/FilterPanel/AllToggle", wf::SELECTED, true);
        ui.layout();
    }

    fn new_tag(&mut self) -> u64 {
        self.next_tag += 1;
        self.next_tag
    }

    /// Acquire a pooled slot widget (`0x6919D0`): reuse a free one of the
    /// kind, else create it and load it from the cloned `Slots/ItemPanel`
    /// node.
    fn acquire(&mut self, ui: &mut Ui, kind: SlotKind) -> Widget {
        if let Some(i) = self.pools.iter().position(|(s, w)| s.kind == kind && w.is_some()) {
            return self.pools[i].1.take().unwrap();
        }
        let mut wdg = match kind {
            SlotKind::Team => team_slot(),
            SlotKind::Player => player_slot("PlayerSlot", ui),
            SlotKind::Observer => player_slot("ObserverSlot", ui),
        };
        let node = ui.store.node("UI/Menus/staging.tgi:StagingMenu/WorldParamsPanel/Slots/ItemPanel").ok();
        if let Some(n) = node {
            let strings = &ui.strings;
            let resolve = |v: &str| strings.resolve(v);
            let root = std::sync::Arc::new(n.clone());
            wdg.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
        }
        wdg.tag = self.new_tag();
        wdg
    }

    /// Rebuild (`0x69A120`) and viewport fill (`0x69F950`).
    pub fn rebuild(&mut self, ui: &mut Ui) {
        // return every slot to the pools
        let items = ui.viewport_clear(SLOTS);
        for it in items {
            if let Some(p) = self.pools.iter_mut().find(|(s, _)| s.tag == it.tag) {
                p.1 = Some(it);
            }
        }
        let state = self.state.clone();
        let film = state.launch_type == LaunchType::LoadFilm;
        let teams = if film { 0 } else { state.teams.len() };
        let mut order: Vec<Slot> = Vec::new();
        for t in 0..teams {
            order.push(Slot { tag: 0, kind: SlotKind::Team, team: t, index: 0 });
            for k in 0..state.teams[t].kingdoms.len() {
                order.push(Slot { tag: 0, kind: SlotKind::Player, team: t, index: k });
            }
        }
        // the observer header, always
        order.push(Slot { tag: 0, kind: SlotKind::Team, team: teams, index: 0 });
        for (i, _) in state.observers.iter().enumerate() {
            order.push(Slot { tag: 0, kind: SlotKind::Observer, team: teams, index: i });
        }
        let mut first_team_tag = None;
        let mut used = Vec::new();
        for mut slot in order {
            let mut wdg = self.acquire(ui, slot.kind);
            slot.tag = wdg.tag;
            if slot.kind == SlotKind::Team && first_team_tag.is_none() {
                first_team_tag = Some(wdg.tag);
            }
            self.refresh(ui, &slot, &mut wdg);
            ui.viewport_add(SLOTS, wdg);
            used.push(slot);
        }
        ui.layout();
        for slot in &used {
            self.fill_lists(ui, slot);
        }
        // keep the pool bookkeeping (pooled widgets are now in the tree)
        let mut pools: Vec<(Slot, Option<Widget>)> = self.pools.drain(..).filter(|(_, w)| w.is_some()).collect();
        for s in used {
            pools.push((s, None));
        }
        self.pools = pools;
        // autoselect the first team slot (0x5DCA90)
        if self.selected.is_none_or(|t| ui.find_tag(t).is_none()) {
            self.selected = first_team_tag;
        }
        if let Some(t) = self.selected {
            if let Some(p) = ui.find_tag(t) {
                ui.select_panel(&p);
                ui.take_events();
            }
        }
        ui.layout();
        ui.update(ui.now_ms);
    }

    fn mode(&self, slot: &Slot) -> Mode {
        if slot.kind == SlotKind::Observer {
            return Mode::Observer;
        }
        let k = &self.state.teams[slot.team].kingdoms[slot.index];
        match &k.occupant {
            Occupant::None => {
                if k.closed {
                    Mode::Closed
                } else {
                    Mode::Open
                }
            }
            Occupant::Ai { .. } => Mode::Ai,
            Occupant::Human { .. } => Mode::Human,
        }
    }

    /// Slot refresh (`0x698FB0` team slots; `0x69DC10`, `0x694A30`,
    /// `0x69D300` player slots).
    fn refresh(&self, ui: &mut Ui, slot: &Slot, wdg: &mut Widget) {
        let st = &self.state;
        if slot.kind == SlotKind::Team {
            let teams = if st.launch_type == LaunchType::LoadFilm { 0 } else { st.teams.len() };
            let text = if slot.team >= teams {
                self.texts.observer.clone()
            } else {
                st.teams[slot.team].name.clone().unwrap_or_else(|| self.texts.no_team.clone())
            };
            set_text(wdg, "TeamName", &text);
            return;
        }
        let mode = self.mode(slot);
        let (kingdom, occupant) = match slot.kind {
            SlotKind::Observer => (None, st.observers[slot.index].clone()),
            _ => {
                let k = &st.teams[slot.team].kingdoms[slot.index];
                (Some(k.clone()), k.occupant.clone())
            }
        };
        // KingdomColorIcon: the KingdomGlyphInfo glyph in the setup colour
        // (0x660610; colour wrap 0x731980). Hidden for observers.
        let glyph_ids = ui.avars.get("KingdomGlyphInfo").unwrap_or("ch_gem").to_ascii_lowercase();
        let glyph = ui.strings.glyphs.get(&glyph_ids).copied();
        match (&kingdom, glyph) {
            (Some(k), Some(g)) => {
                let c = k.color;
                set_text(wdg, "KingdomColorIcon", &crate::text::color_wrap(&g.to_string(), [c[0], c[1], c[2]]));
                visible(wdg, "KingdomColorIcon", true);
            }
            _ => visible(wdg, "KingdomColorIcon", false),
        }
        // Name
        let name = match (&occupant, &kingdom) {
            (Occupant::Human { name, .. }, _) => name.clone(),
            (Occupant::Ai { name, local: true, .. }, _) => name.clone(),
            (Occupant::Ai { name, client, .. }, _) => ui.strings.format("staging_ainame_clientname|ss", &[name, client]),
            (Occupant::None, Some(k)) => match (k.closed, k.required) {
                (false, false) => self.texts.open.clone(),
                (false, true) => self.texts.open_required.clone(),
                (true, false) => self.texts.closed.clone(),
                (true, true) => self.texts.closed_required.clone(),
            },
            (Occupant::None, None) => String::new(),
        };
        set_text(wdg, "Name", &name);
        // AddAI: open slots and local AIs
        let ai_local = matches!(occupant, Occupant::Ai { local: true, .. });
        let add_ai = mode == Mode::Open || (mode == Mode::Ai && ai_local);
        visible(wdg, "AddAI", add_ai);
        visible(wdg, "Name", !add_ai);
        // editable 0x694860
        let editable = !(st.locked
            || mode == Mode::Observer
            || (matches!(occupant, Occupant::None)
                && kingdom.as_ref().is_some_and(|k| k.fixed_nation.is_none() && k.fixed_faction.is_none() && !k.required)));
        // nation / faction / flag (0x69DC10)
        let has_nations = !st.nations.is_empty();
        let show_nf = editable && has_nations && kingdom.is_some();
        let flag_avar = ui.avars.flag("UIStagingUseNationFlagDecal").unwrap_or(false);
        let nation_list: Vec<(String, String)> = match kingdom.as_ref().and_then(|k| k.fixed_nation.clone()) {
            Some(id) => st.nations.iter().filter(|n| n.id == id).map(|n| (n.id.clone(), n.text.clone())).collect(),
            None => std::iter::once(("_random".to_string(), self.texts.random.clone()))
                .chain(st.nations.iter().filter(|n| n.playable).map(|n| (n.id.clone(), n.text.clone())))
                .collect(),
        };
        let cur_nation = kingdom.as_ref().and_then(|k| k.fixed_nation.clone().or(k.nation.clone()));
        let nation_text = cur_nation
            .as_ref()
            .and_then(|id| st.nations.iter().find(|n| &n.id == id).map(|n| n.text.clone()))
            .unwrap_or_else(|| self.texts.random.clone());
        visible(wdg, "Nation", show_nf && nation_list.len() > 1);
        visible(wdg, "NationLabel", show_nf && nation_list.len() <= 1 && !flag_avar);
        set_text(wdg, "NationLabel", &nation_text);
        if flag_avar {
            let flag = cur_nation
                .as_ref()
                .and_then(|id| st.nations.iter().find(|n| &n.id == id))
                .and_then(|n| n.flag_icon.clone())
                .or_else(|| {
                    ui.store
                        .node("UI/Menus/staging.tgi:StagingMenu/WorldParamsPanel/Slots/ItemPanel/PlayerSlot")
                        .ok()
                        .map(|n| n.string_or_empty("default_flag_icon"))
                });
            if let Some(f) = wdg.child_mut("NationFlagDecalIcon") {
                f.set_visible(show_nf);
                if let Some(crate::ve::ViewElement { kind: VeKind::Decal(d), .. }) = &mut f.sub_ve {
                    d.texture = flag;
                }
            }
        }
        // faction (0x697860)
        let fixed_faction = kingdom.as_ref().and_then(|k| k.fixed_faction.clone());
        let nation = cur_nation.as_ref().and_then(|id| st.nations.iter().find(|n| &n.id == id));
        let faction_text = kingdom
            .as_ref()
            .and_then(|k| k.fixed_faction.clone().or(k.faction.clone()))
            .and_then(|id| nation.and_then(|n| n.factions.iter().find(|f| f.id == id)).map(|f| f.text.clone()))
            .unwrap_or_else(|| self.texts.random.clone());
        visible(wdg, "Faction", show_nf && fixed_faction.is_none());
        visible(wdg, "FactionLabel", show_nf && fixed_faction.is_some());
        set_text(wdg, "FactionLabel", &faction_text);
        // handicap
        let handicap = editable
            && matches!(occupant, Occupant::Ai { .. })
            && !st.handicaps.is_empty()
            && !matches!(st.launch_type, LaunchType::LoadSavedGame | LaunchType::LoadSavedMission);
        visible(wdg, "Handicap", handicap);
        // ready / host / progress / ping / swap
        let (human, on_host, ready, ping, download, local_h, swap) = match &occupant {
            Occupant::Human { on_host, ready, ping_ms, download, local, swap_from, swap_to, .. } => {
                (true, *on_host, *ready, *ping_ms, *download, *local, *swap_from || *swap_to)
            }
            _ => (false, false, false, 0, None, false, false),
        };
        let ready_vis = !st.locked && kingdom.is_some() && human && !on_host && download.is_none();
        if let Some(r) = wdg.child_mut("Ready") {
            r.set_visible(ready_vis);
            r.set_enabled(local_h);
            if ready {
                r.flags |= wf::SELECTED;
            } else {
                r.flags &= !wf::SELECTED;
            }
        }
        visible(wdg, "HostIcon", mode == Mode::Human && on_host);
        match download {
            Some(p) => {
                set_text(wdg, "Progress", &ui.strings.format("staging_percent_downloaded|d", &[&((p * 100.0) as i32).to_string()]));
                visible(wdg, "Progress", true);
            }
            None => visible(wdg, "Progress", false),
        }
        visible(wdg, "Ping", human && st.networked);
        if human && st.networked {
            let low = ui.avars.float("ServerBrowserLowPingCutoff").unwrap_or(200.0) as i32;
            let med = ui.avars.float("ServerBrowserMediumPingCutoff").unwrap_or(500.0) as i32;
            let alt = ui.avars.flag("UIStagingUseAlternatePingIcon").unwrap_or(false);
            let colour = if ping < low { "Green" } else if ping < med { "Yellow" } else { "Red" };
            let ids = format!("ch_{}ping{}icon", if alt { "staging" } else { "" }, colour.to_ascii_lowercase());
            let g = ui.strings.glyphs.get(&ids).map(|c| c.to_string()).unwrap_or_default();
            set_text(wdg, "Ping", &g);
        }
        visible(wdg, "SwapRequest", swap);
    }

    /// Drop-down lists of a player slot (AddAI `0x69BE50`/`0x699F00`,
    /// nation/faction `0x69DC10`/`0x697860`, handicap): entries and the
    /// current selection, without firing commands.
    fn fill_lists(&self, ui: &mut Ui, slot: &Slot) {
        if slot.kind != SlotKind::Player {
            return;
        }
        let Some(path) = ui.find_tag(slot.tag) else { return };
        let st = &self.state;
        let k = &st.teams[slot.team].kingdoms[slot.index];
        let fill = |ui: &mut Ui, dd: &str, items: &[(String, String)], sel: &str| {
            let lb = format!("{path}/{dd}/Listbox");
            ui.listbox_clear(&lb);
            for (id, text) in items {
                ui.listbox_add(&lb, text, id);
            }
            if let Some(i) = ui.listbox_entries(&lb).iter().position(|e| e.param == sel) {
                ui.dropdown_select(&format!("{path}/{dd}"), i);
            }
        };
        // AddAI: "_open", then "_random" and the selectable SAIs
        let mut ai: Vec<(String, String)> = vec![("_open".into(), self.texts.open.clone())];
        if !st.sais.is_empty() {
            ai.push(("_random".into(), self.texts.random.clone()));
            ai.extend(st.sais.iter().filter(|s| s.2).map(|s| (s.0.clone(), s.1.clone())));
        }
        let ai_sel = match &k.occupant {
            Occupant::Ai { sai: Some(s), .. } => s.clone(),
            Occupant::Ai { sai: None, .. } => "_random".into(),
            _ => "_open".into(),
        };
        fill(ui, "AddAI", &ai, &ai_sel);
        // nation
        let nations: Vec<(String, String)> = match &k.fixed_nation {
            Some(id) => st.nations.iter().filter(|n| &n.id == id).map(|n| (n.id.clone(), n.text.clone())).collect(),
            None => std::iter::once(("_random".to_string(), self.texts.random.clone()))
                .chain(st.nations.iter().filter(|n| n.playable).map(|n| (n.id.clone(), n.text.clone())))
                .collect(),
        };
        let nsel = k.fixed_nation.clone().or(k.nation.clone()).unwrap_or_else(|| "_random".into());
        fill(ui, "Nation", &nations, &nsel);
        // faction
        let nation = st.nations.iter().find(|n| n.id == nsel);
        let factions: Vec<(String, String)> = std::iter::once(("_random".to_string(), self.texts.random.clone()))
            .chain(nation.into_iter().flat_map(|n| n.factions.iter().filter(|f| f.selectable).map(|f| (f.id.clone(), f.text.clone()))))
            .collect();
        let fsel = k.fixed_faction.clone().or(k.faction.clone()).unwrap_or_else(|| "_random".into());
        fill(ui, "Faction", &factions, &fsel);
        // handicap: null = the middle entry
        let hsel = k.handicap.clone().or_else(|| st.handicaps.get(st.handicaps.len() / 2).map(|h| h.0.clone())).unwrap_or_default();
        fill(ui, "Handicap", &st.handicaps, &hsel);
    }

    /// Per-frame update (`0x661A80` with InfoPanel `0x6916B0`/`0x65C5B0`,
    /// ActionPanel `0x689700`, action panels `0x694B40`).
    pub fn update(&mut self, ui: &mut Ui) {
        let st = self.state.clone();
        // InfoPanel per launch type
        let tf = matches!(st.launch_type, LaunchType::RandomMap | LaunchType::LoadMap | LaunchType::LoadSavedGame | LaunchType::LoadSavedMission);
        let tf_label = tf && ui.avars.flag("UIStagingUseTeamFormatLabel").unwrap_or(false);
        ui.with_widget("StagingMenu/InfoPanel/TeamFormatDropdown", |w| {
            w.set_visible(tf);
            w.set_enabled(tf);
        });
        ui.with_widget("StagingMenu/InfoPanel/TeamFormatLabel", |w| w.set_visible(tf_label));
        // 0x65C5B0: name, password, team formats, players label
        ui.set_label("StagingMenu/InfoPanel/GameNameLabel", &st.game_name);
        let pw = st.networked && !st.password.is_empty();
        ui.with_widget("StagingMenu/InfoPanel/PasswordIndicatorDecal", |w| w.set_visible(pw));
        let players = ui.strings.format("staging_players_label|dd", &[&st.players.0.to_string(), &st.players.1.to_string()]);
        ui.set_label("StagingMenu/InfoPanel/NumPlayersLabel", &players);
        let dd = "Root/StagingMenu/InfoPanel/TeamFormatDropdown";
        let lb = format!("{dd}/Listbox");
        if ui.listbox_entries(&lb).len() != st.team_formats.len() + 1 {
            ui.listbox_clear(&lb);
            let custom = ui.strings.resolve("#team_format_custom");
            ui.listbox_add(&lb, &custom, "team_format_custom");
            for (id, text) in &st.team_formats {
                ui.listbox_add(&lb, text, id);
            }
        }
        let want = st.team_format.clone().unwrap_or_else(|| "team_format_custom".into());
        if let Some(i) = ui.listbox_entries(&lb).iter().position(|e| e.param == want) {
            if ui.listbox_selected(&lb) != Some(i) {
                ui.dropdown_select(dd, i);
            }
        }
        // MapPreviewLabel through viewport AddItem (once), text 0x660E90,
        // fit-height 0x5EF5C0, viewport refresh 0x5ED6B0
        let mpv = "Root/StagingMenu/MapPreviewPanel/MapPreviewViewport";
        let lp = format!("{mpv}/ItemPanel/MapPreviewLabel");
        if ui.widget(&lp).is_none() {
            let mut label = Widget::new(C::Label, "MapPreviewLabel", false);
            if let Ok(n) = ui.store.node("UI/Menus/staging.tgi:StagingMenu/MapPreviewPanel/MapPreviewViewport/ItemPanel") {
                let strings = &ui.strings;
                let resolve = |v: &str| strings.resolve(v);
                let root = std::sync::Arc::new(n.clone());
                label.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
            }
            ui.viewport_add(mpv, label);
        }
        let text = map_preview_text(ui, &st);
        ui.set_label(&lp["Root/".len()..], &text);
        label_fit_height(ui, &lp);
        ui.viewport_refresh(mpv);
        // ActionPanel
        let host = st.local_is_host;
        ui.with_widget("StagingMenu/ActionPanel/ReadyToggle", |w| {
            w.set_visible(!host);
            w.set_enabled(st.local_has_kingdom);
            if st.local_ready {
                w.flags |= wf::SELECTED;
            } else {
                w.flags &= !wf::SELECTED;
            }
        });
        ui.with_widget("StagingMenu/ActionPanel/LaunchButton", |w| w.set_visible(host && !st.countdown));
        ui.with_widget("StagingMenu/ActionPanel/AbortLaunchButton", |w| w.set_visible(host && st.countdown));
        self.action_panels(ui);
        ui.layout();
    }

    /// Action panels (`0x694B40`, `0x695010`).
    fn action_panels(&mut self, ui: &mut Ui) {
        let sel = self.selected.and_then(|t| self.pools.iter().find(|(s, _)| s.tag == t).map(|(s, _)| s.clone()));
        let tap = "StagingMenu/WorldParamsPanel/TeamActionPanel";
        let kap = "StagingMenu/WorldParamsPanel/KingdomActionPanel";
        let (team_vis, king_vis) = match &sel {
            None => (false, false),
            Some(s) if s.kind == SlotKind::Team => (true, false),
            Some(_) => (false, true),
        };
        ui.with_widget(tap, |w| w.set_visible(team_vis));
        ui.with_widget(kap, |w| w.set_visible(king_vis));
        let Some(sel) = sel else { return };
        // dynamic location: the shown panel's top = the selected item's T
        if ui.avars.flag("UIStagingActionButtonsDynamicLocation").unwrap_or(false) {
            let g = ui.geo();
            let item_t = ui.find_tag(sel.tag).and_then(|p| ui.widget(&p).map(|w| w.root.outer.raw[1]));
            let panel = if team_vis { tap } else { kap };
            if let Some(t) = item_t {
                let (screen, reference, p) = (ui.screen, ui.reference, ui.precision);
                let _ = g;
                ui.with_widget(panel, |w| crate::geometry::set_top(w, t, screen, reference, p));
            }
        }
        let st = &self.state;
        if sel.kind == SlotKind::Team {
            let observer_header = sel.team >= st.teams.len();
            let enabled = if observer_header { st.local_has_kingdom } else { st.can_join_team.get(sel.team).copied().unwrap_or(false) };
            ui.with_widget(&format!("{tap}/ChangeTeamButton"), |w| w.set_enabled(enabled));
            return;
        }
        let mode = self.mode(&sel);
        let occupant = match sel.kind {
            SlotKind::Observer => st.observers[sel.index].clone(),
            _ => st.teams[sel.team].kingdoms[sel.index].occupant.clone(),
        };
        let (own, ai) = match &occupant {
            Occupant::Human { local, .. } => (*local, false),
            Occupant::Ai { local, .. } => (*local, true),
            Occupant::None => (false, false),
        };
        let (swap_from, swap_to) = match &occupant {
            Occupant::Human { swap_from, swap_to, .. } => (*swap_from, *swap_to),
            _ => (false, false),
        };
        let host = st.local_is_host;
        let online = st.online;
        let human_mode = matches!(mode, Mode::Human | Mode::Observer);
        let occupied = !matches!(mode, Mode::Open | Mode::Closed);
        let vis = [
            ("ChangeName", own && human_mode && !online),
            ("Kick", occupied && (!own || ai) && host),
            ("Whisper", occupied && !own && !ai),
            ("Ban", human_mode && !own && host && online),
            ("SwapWithAI", ai && own),
            ("MoveHere", mode == Mode::Open),
            ("Observer", own && st.local_has_kingdom && st.observers_allowed && !ai),
            ("OfferSwap", occupied && !own && !swap_from && !swap_to),
            ("CancelSwap", occupied && !own && swap_to),
            ("AcceptSwap", occupied && !own && swap_from),
            ("RejectSwap", occupied && !own && swap_from),
        ];
        for (b, v) in vis {
            ui.with_widget(&format!("{kap}/{b}"), |w| w.set_visible(v));
        }
    }

    /// Handle UI events for the staging screen (slot selection).
    pub fn event(&mut self, ui: &mut Ui, e: &crate::ui::UiEvent) {
        if let crate::ui::UiEvent::Selected { tag, .. } = e {
            if self.pools.iter().any(|(s, _)| s.tag == *tag) {
                self.selected = Some(*tag);
                self.update(ui);
            }
        }
    }

    /// Replace the session state (dirty flags c8/c9: a full rebuild).
    pub fn set_state(&mut self, ui: &mut Ui, state: StagingState) {
        self.state = state;
        self.rebuild(ui);
        self.update(ui);
    }
}

/// MSVC `%g` (6 significant digits, trailing zeros dropped).
pub fn fmt_g(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let exp = v.abs().log10().floor() as i32;
    if !(-5..6).contains(&exp) {
        let s = format!("{:.5e}", v);
        let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
        let m = m.trim_end_matches('0').trim_end_matches('.');
        let e: i32 = e.parse().unwrap_or(0);
        return format!("{m}e{}{:03}", if e < 0 { '-' } else { '+' }, e.abs());
    }
    let decimals = (5 - exp).max(0) as usize;
    let s = format!("{:.*}", decimals, v);
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

/// MapPreviewLabel text (`0x660E90`): title, the rules block and the
/// kingdom rows (`0x65F880` / `0x50F970`: `<color=r,g,b>G <rc>name`).
/// Types 1/2 also add the size line (`0x65FB20`); the online official /
/// checksum lines and RMC extras are not shown offline.
pub fn map_preview_text(ui: &Ui, st: &StagingState) -> String {
    let title = match st.launch_type {
        LaunchType::LoadSavedGame => ui.strings.format("staging_map_preview_append_save|s", &[&st.map_name]),
        LaunchType::LoadFilm => ui.strings.format("staging_map_preview_append_film|s", &[&st.map_name]),
        _ => st.map_name.clone(),
    };
    let mut out = String::new();
    if matches!(st.launch_type, LaunchType::None | LaunchType::LoadSavedMission | LaunchType::Editor) {
        return out;
    }
    out.push_str(&title);
    out.push('\n');
    if matches!(st.launch_type, LaunchType::LoadMap | LaunchType::RandomMap | LaunchType::LoadFilm) {
        let size = ui.strings.format("staging_map_preview_map_size_format_string|dd", &[&st.map_size.0.to_string(), &st.map_size.1.to_string()]);
        out.push_str(&size);
        out.push('\n');
    }
    if !st.rules.is_empty() {
        out.push_str(&st.rules);
        out.push('\n');
    }
    out.push('\n');
    let glyph = ui
        .strings
        .glyphs
        .get(&ui.avars.get("KingdomGlyphInfo").unwrap_or("ch_gem").to_ascii_lowercase())
        .copied()
        .unwrap_or('#');
    for t in &st.teams {
        for k in &t.kingdoms {
            let c = k.color;
            out.push_str(&format!("<color={},{},{}>{} <rc>{}\n", fmt_g(c[0] as f64), fmt_g(c[1] as f64), fmt_g(c[2] as f64), glyph, k.record_name));
        }
    }
    out
}

/// Label fit-height (`0x5EF5C0`): B = the measured text height at the
/// label's width, as a size.
pub fn label_fit_height(ui: &mut Ui, path: &str) {
    let g = ui.geo();
    let Some(w) = ui.widget(path) else { return };
    let width = (w.root.outer.rect.r - w.root.outer.rect.l).abs() / g.sx;
    let (font, text) = match &w.sub_ve {
        Some(crate::ve::ViewElement { kind: VeKind::Label(l), .. }) => (l.font.clone(), l.text.clone()),
        _ => return,
    };
    let (_, h, _) = ui.measure_text(&font, &text, width);
    if let Some(w) = ui.widget_mut(path) {
        let o = &mut w.root.outer;
        o.raw[3] = h;
        o.flags = (o.flags & !0x88) | 0x200;
    }
    ui.layout();
}

fn general_name(faction: &str) -> String {
    let mut c = faction.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Placeholder session matching the original's 02-custom.png: a saved
/// 8v8 single-player game, the local player on Team 1 (USA, Nimitz), every
/// other kingdom a local random AI. The names and nations are the ones the
/// screenshot shows; nothing here is a game rule.
pub fn placeholder_state() -> StagingState {
    let nation = |id: &str, text: &str, flag: &str, factions: &[(&str, &str)]| NationInfo {
        id: id.into(),
        text: text.into(),
        playable: true,
        flag_icon: Some(flag.into()),
        factions: factions.iter().map(|(i, t)| FactionInfo { id: (*i).into(), text: (*t).into(), selectable: true }).collect(),
    };
    let nations = vec![
        nation("usa", "United States", "/UI/%s/Default/Staging/FlagUSA.png", &[("nimitz", "Nimitz"), ("eisenhower", "Eisenhower"), ("arnold", "Arnold")]),
        nation("jpn", "Japan", "/UI/%s/Default/Staging/FlagJPN.png", &[("yamamoto", "Yamamoto"), ("mikawa", "Mikawa")]),
        nation("ger", "Germany", "/UI/%s/Default/Staging/FlagGER.png", &[("manstein", "Manstein"), ("kesselring", "Kesselring"), ("rundstedt", "Rundstedt")]),
        nation("gbr", "United Kingdom", "/UI/%s/Default/Staging/FlagGBR.png", &[("wavell", "Wavell")]),
        nation("rus", "Soviet Union", "/UI/%s/Default/Staging/FlagRUS.png", &[("chuikov", "Chuikov")]),
    ];
    let colors: [[f32; 3]; 16] = [
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.8, 0.0],
        [0.5, 0.5, 0.5],
        [0.0, 0.8, 0.8],
        [0.6, 0.4, 0.1],
        [1.0, 1.0, 0.0],
        [0.8, 0.0, 0.8],
        [0.7, 0.7, 0.7],
        [0.6, 0.0, 0.0],
        [0.0, 0.6, 0.0],
        [0.5, 0.0, 1.0],
        [0.0, 0.4, 0.8],
        [1.0, 0.5, 0.0],
        [1.0, 0.6, 0.8],
        [0.3, 0.3, 0.3],
    ];
    let team1 = [("usa", "nimitz"), ("usa", "eisenhower"), ("jpn", "yamamoto"), ("ger", "manstein"), ("gbr", "wavell"), ("ger", "kesselring"), ("jpn", "mikawa"), ("gbr", "wavell")];
    let team2 = [("ger", "rundstedt"), ("rus", "chuikov"), ("usa", "arnold"), ("jpn", "mikawa"), ("rus", "chuikov"), ("ger", "manstein"), ("usa", "eisenhower"), ("gbr", "wavell")];
    let mut idx = 0;
    let mut team = |name: &str, list: &[(&str, &str)], human_first: bool| {
        let kingdoms = list
            .iter()
            .enumerate()
            .map(|(i, (n, f))| {
                let c = colors[idx % colors.len()];
                idx += 1;
                let occupant = if human_first && i == 0 {
                    Occupant::Human {
                        name: "Player".into(),
                        local: true,
                        on_host: true,
                        ready: false,
                        ping_ms: 0,
                        download: None,
                        swap_from: false,
                        swap_to: false,
                    }
                } else {
                    Occupant::Ai { name: String::new(), client: String::new(), sai: None, local: true }
                };
                let record_name = if human_first && i == 0 { "Player".to_string() } else { format!("{} AI", general_name(f)) };
                KingdomSlot {
                    closed: false,
                    required: false,
                    color: c,
                    record_name,
                    nation: Some((*n).into()),
                    faction: Some((*f).into()),
                    handicap: None,
                    fixed_nation: Some((*n).into()),
                    fixed_faction: Some((*f).into()),
                    occupant,
                }
            })
            .collect();
        TeamInfo { name: Some(name.into()), kingdoms }
    };
    let teams = vec![team("Team 1", &team1, true), team("Team 2", &team2, false)];
    StagingState {
        networked: false,
        local_is_host: true,
        online: false,
        launch_type: LaunchType::LoadSavedGame,
        locked: false,
        countdown: false,
        game_name: String::new(),
        password: String::new(),
        players: (1, 16),
        team_formats: vec![("8v8".into(), "8v8".into())],
        team_format: Some("8v8".into()),
        scoring_available: false,
        observers_allowed: true,
        map_name: "V10 Graphics Switch QA".into(),
        map_size: (0, 0),
        rules: String::new(),
        local_has_kingdom: true,
        local_ready: false,
        teams,
        observers: Vec::new(),
        nations,
        handicaps: vec![("easy".into(), "Easy".into()), ("normal".into(), "Normal".into()), ("hard".into(), "Hard".into())],
        sais: vec![("random".into(), "Random".into(), true)],
        can_join_team: vec![false, true],
    }
}
