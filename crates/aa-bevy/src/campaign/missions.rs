//! ChooseCampaign (KKC_ChooseCampaignInterface, ChooseCampaignInterface.cpp):
//! the scripted campaigns of the Single Player menu (RequestCampaign): the
//! tutorial, Allies and Axis mission lists of `Data/Campaign/campaign_aa.tgi`.
//!
//! Init `0x642280`, in AddChild order: `/CampaignButtonsViewport` (a toggle
//! `CampaignButton` per campaign, `0x6420C0`, with the labelled decal
//! `CampaignButtonDecalLabel`, `0x642180`), `/MissionPanel` holding
//! `MissionListboxPanel` / `MissionListbox` (`CampaignSelectMission %s`, double
//! click `CampaignConfirmPlay`) and `DifficultyPanel` / `DifficultyDropdown`
//! (`CampaignDifficulty %s`), `/PlayButton` (`CampaignConfirmPlay`),
//! `/ReplayButton` (`CampaignPlay`), `/LoadGameButton` (`CampaignLoadGame`),
//! `/ExitButton` (`CampaignExit`), `/ChooseCampaignGlobeBink`.
//!
//! `0x643A40` fills the buttons with each campaign's name and its
//! complete_texture / incomplete_texture and selects the last campaign
//! (`[B39D04]+50`, the first without one). `0x643050` lists the selected
//! campaign's missions: those of the progress record (`0x411790`) not locked
//! (state 1), the row's parameter the mission's index, a cutscene mission in the
//! UVar colour UIColorCampaignCutscene. `0x644200` runs the commands;
//! `CampaignPlay` -> PlayMap `0x643CA0` makes the session: the map, its world
//! rules with the DifficultyLevel of the dropdown (`0x511B80`), a new map seed
//! (SetMapSeed `0x642D10`), the launch (`0x652960`). The mission's kingdoms come
//! from the map's header: the first playable one is the player's (a campaign
//! session has one human), the others are computer players on their own
//! profile (`sai_default_ids`).
//!
//! The progress of a campaign is [`CampaignState`] (KKC_CampaignState,
//! `Data/CampaignData.cpp`), the original's file `<profile>/data/CampaignSave/<ids>.rcs`
//! in the original's format, read when the record is first asked for (`0x411790`).
//!
//! The end of a mission (`0x60A140`, then `0x651BD0`): won, the current mission is
//! complete and the next one open (`0x46FE30`); the debriefing follows
//! (`campaign/debriefing.rs`), for a final mission (`is_final_mission`) the chooser.
//! `PlayNext` (`0x641800`) starts the next one, or ends the record (+4 = 0) after
//! the last. The prebriefing file is not read by the original.
//!
//! A cinematic mission (`mission_type = CINEMATIC`, Bink, `0x641D10`) plays in
//! `campaign/cinematic.rs`; one that was not baked ends at once, as its end does
//! (`0x641400`).
//!
//! Compare against the original: `CampaignLoadGame` (`0x68DB60(4, ..)`, a mission's
//! saved game) is refused: aa-play has no saved game.
use aa_ui::ui::Ui;
use aa_ui::controls::{Control, ListCallback};
use aa_ui::widget::{Widget, WidgetClass as C};
use std::path::Path;

pub const FILE: &str = "UI/Menus/choose_campaign.tgi";
const IFACE: &str = "ChooseCampaign";
const LIST: &str = "Root/ChooseCampaign/MissionPanel/MissionListboxPanel/MissionListbox";
const DIFFICULTY: &str = "Root/ChooseCampaign/MissionPanel/DifficultyPanel/DifficultyDropdown";
const BUTTONS: &str = "Root/ChooseCampaign/CampaignButtonsViewport";
const TAG_BUTTON: u64 = 9500;

/// One `[Mission]` of a campaign.
#[derive(Clone, Debug, Default)]
pub struct Mission {
    /// aa-play's map id: `Campaign/tutorial01` (from `/Maps/Campaign/Tutorial01.rmp`).
    pub map: Option<String>,
    pub name: String,
    /// `mission_type`: PLAYABLE, CINEMATIC, (CUTSCENE).
    pub kind: String,
    pub cinematic: Option<String>,
    pub loading_left: Option<String>,
    pub loading_right: Option<String>,
    pub debriefing_success: String,
    pub debriefing_failure: String,
}

/// One `[Campaign]` of `campaign_aa.tgi`.
#[derive(Clone, Debug, Default)]
pub struct ScriptedCampaign {
    pub ids: String,
    pub name: String,
    pub dependent: Option<String>,
    pub complete_texture: String,
    pub incomplete_texture: String,
    pub missions: Vec<Mission>,
    /// `0x411790`: the campaign's record.
    pub state: CampaignState,
    /// `is_final_mission` per mission (+30, `0x46FFA0`).
    finals: Vec<bool>,
}

/// KKC_CampaignState: the progress of one scripted campaign.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignState {
    /// +4: 1 until the campaign is finished.
    pub active: u32,
    /// +8: per mission 0 open, 1 locked, 2 complete.
    pub states: Vec<u32>,
    /// +14: the current mission.
    pub current: usize,
    /// +18: the mission the chooser played last (-1 none), not in the file.
    pub last: i32,
    /// +1C: the index of the difficulty level.
    pub difficulty: u32,
}

impl Default for CampaignState {
    fn default() -> Self {
        Self::new(0, 1)
    }
}

impl CampaignState {
    /// `0x470A30`: the first mission open, the others locked, the default
    /// difficulty (the DifficultyLevel rule's default).
    pub fn new(missions: usize, difficulty: u32) -> Self {
        Self { active: 1, states: (0..missions).map(|i| u32::from(i != 0)).collect(), current: 0, last: -1, difficulty }
    }

    /// `0x46FE30`: the current mission is complete and the next one open, or the
    /// campaign finished. Complete already, nothing changes.
    pub fn complete_current(&mut self) {
        let i = self.current;
        if self.states.get(i) == Some(&2) || i >= self.states.len() {
            return;
        }
        self.states[i] = 2;
        match self.states.get_mut(i + 1) {
            Some(next) => *next = 0,
            None => self.active = 0,
        }
    }

    /// `0x46F7F0`: version 0, the IDS (u16 length, UTF-16), +4, each mission's
    /// version 0 and state, the difficulty index.
    pub fn write(&self, ids: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0u32.to_le_bytes());
        let wide: Vec<u16> = ids.encode_utf16().collect();
        out.extend_from_slice(&(wide.len() as u16).to_le_bytes());
        for c in wide {
            out.extend_from_slice(&c.to_le_bytes());
        }
        out.extend_from_slice(&self.active.to_le_bytes());
        for state in &self.states {
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&state.to_le_bytes());
        }
        out.extend_from_slice(&self.difficulty.to_le_bytes());
        out
    }

    /// `0x470B00`: `None` for a corrupt file or another campaign's (the original
    /// logs "Corrupt CampaignState file" and starts a new record).
    pub fn read(bytes: &[u8], ids: &str, missions: usize) -> Option<Self> {
        let word = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let len = bytes.get(4..6).map(|b| u16::from_le_bytes(b.try_into().unwrap()) as usize)?;
        let name: Vec<u16> = bytes.get(6..6 + len * 2)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        if !String::from_utf16_lossy(&name).eq_ignore_ascii_case(ids) {
            return None;
        }
        let mut at = 6 + len * 2;
        let active = word(at)?;
        at += 4;
        let mut states = Vec::with_capacity(missions);
        for _ in 0..missions {
            states.push(word(at + 4)?);
            at += 8;
        }
        let difficulty = word(at)?;
        Some(Self { active, states, current: 0, last: -1, difficulty })
    }
}

/// `CampaignSaveDirectory` `/` ids `.` `CampaignStateExtension` under the profile's
/// `data/` (`0x46FC50`).
fn state_file(ui: &Ui, ids: &str) -> Option<std::path::PathBuf> {
    let dir = ui.avars.get("CampaignSaveDirectory").map(aa_ui::node::unquote).unwrap_or_else(|| "CampaignSave".into());
    let ext = ui.avars.get("CampaignStateExtension").map(aa_ui::node::unquote).unwrap_or_else(|| "rcs".into());
    Some(crate::session::own_profile()?.join("data").join(dir).join(format!("{ids}.{ext}")))
}

/// `0x46FC50`.
fn save_state(ui: &Ui, c: &ScriptedCampaign) {
    let Some(path) = state_file(ui, &c.ids) else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, c.state.write(&c.ids)) {
        bevy::log::warn!("campaign state {}: {e}", path.display());
    }
}

/// What a chooser command asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    None,
    /// CampaignExit: back to the Single Player menu.
    Exit,
    /// PlayMap: launch the mission's map with this difficulty.
    Play { map: String, difficulty: String },
    /// `0x66F880`: the confirmation of the dependent campaign is on the screen.
    Asked,
    /// `0x641D10`: the movie plays (`campaign/cinematic.rs`), then [`Chooser::cinematic_ended`].
    Cinematic(String),
}

/// The chooser.
#[derive(Default)]
pub struct Chooser {
    pub campaigns: Vec<ScriptedCampaign>,
    pub campaign: usize,
    pub mission: usize,
    /// (ids, name key) of the DifficultyLevel bank.
    levels: Vec<(String, String)>,
    /// The mission on the way (its result opens the next one).
    pub playing: Option<(usize, usize)>,
    /// The file is the original's: an empty record is read once.
    loaded: bool,
}

/// `/Maps/Campaign/Tutorial01.rmp` -> `Campaign/tutorial01` (aa-play ids are
/// the file's stem and the folder below Maps; the file system ignores the case
/// of the original's spelling, the map's lower case id is the file's).
pub fn map_id(path: &str) -> String {
    let p = path.trim_start_matches(['/', '\\']).replace('\\', "/");
    let p = p.strip_prefix("Maps/").or_else(|| p.strip_prefix("maps/")).unwrap_or(&p).to_string();
    let stem = p.rsplit_once('.').map_or(p.as_str(), |x| x.0);
    match stem.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/{}", file.to_ascii_lowercase()),
        None => stem.to_ascii_lowercase(),
    }
}

impl Chooser {
    fn load(&mut self, ui: &Ui) {
        if !self.campaigns.is_empty() {
            return;
        }
        for file in ["Campaign/campaign_aa.tgi"] {
            for n in crate::ui::datafile::load(&ui.store.root, file).iter().filter(|c| c.name.eq_ignore_ascii_case("Campaign")) {
                let get = |k: &str| n.get(k).unwrap_or_default().trim_matches('"').to_string();
                let mut finals = Vec::new();
                let missions: Vec<Mission> = n
                    .children_named("Mission")
                    .into_iter()
                    .map(|m| {
                        let s = |k: &str| m.get(k).map(|v| v.trim_matches('"').to_string()).filter(|v| !v.is_empty());
                        finals.push(m.get("is_final_mission").is_some_and(|v| v.trim().eq_ignore_ascii_case("true")));
                        Mission {
                            map: s("map").map(|p| map_id(&p)),
                            name: s("name").unwrap_or_default(),
                            kind: s("mission_type").unwrap_or_default(),
                            cinematic: s("cinematic"),
                            loading_left: s("loading_texture_left"),
                            loading_right: s("loading_texture_right"),
                            debriefing_success: s("debriefing_success").unwrap_or_default(),
                            debriefing_failure: s("debriefing_failure").unwrap_or_default(),
                        }
                    })
                    .collect();
                let state = CampaignState::new(missions.len(), 1);
                self.campaigns.push(ScriptedCampaign {
                    ids: get("ids"),
                    name: get("name"),
                    dependent: Some(get("dependent_campaign_ids")).filter(|d| !d.is_empty()),
                    complete_texture: get("complete_texture"),
                    incomplete_texture: get("incomplete_texture"),
                    missions,
                    state,
                    finals,
                });
            }
        }
        // Game/difficulty_level.tgi, the DifficultyLevel world rule's enum
        for n in crate::ui::datafile::load(&ui.store.root, "Game/difficulty_level.tgi").iter().filter(|c| c.name.eq_ignore_ascii_case("DifficultyLevel")) {
            if let Some(ids) = n.get("ids") {
                self.levels.push((ids.to_string(), n.get("name").unwrap_or_default().trim_matches('"').to_string()));
            }
        }
        // 0x411790: the records of the profile; a new one is written at once
        if !self.loaded {
            self.loaded = true;
            let default = self.levels.iter().position(|(ids, _)| ids.eq_ignore_ascii_case("medium")).unwrap_or(0) as u32;
            for c in &mut self.campaigns {
                let read = state_file(ui, &c.ids).and_then(|p| std::fs::read(p).ok());
                match read.as_deref().and_then(|b| CampaignState::read(b, &c.ids, c.missions.len())) {
                    Some(state) => c.state = state,
                    None => {
                        c.state = CampaignState::new(c.missions.len(), default);
                        save_state(ui, c);
                    }
                }
            }
        }
        let open = self.campaigns.iter().flat_map(|c| c.state.states.iter()).filter(|s| **s != 1).count();
        bevy::log::info!("scripted campaigns: {} with {open} open missions", self.campaigns.len());
    }

    /// RequestCampaign.
    pub fn enter(&mut self, ui: &mut Ui) {
        self.load(ui);
    }

    /// `0x411790` for another page: `+4` of the campaign's record, `None` for an
    /// unknown campaign.
    pub fn unfinished(&mut self, ui: &Ui, ids: &str) -> Option<bool> {
        self.load(ui);
        self.campaigns.iter().find(|c| c.ids.eq_ignore_ascii_case(ids)).map(|c| c.state.active != 0)
    }

    /// `0x642280`.
    fn tree() -> Widget {
        let mut iface = Widget::new(C::Interface, &format!("{FILE}:{IFACE}"), false);
        iface.add_child(Widget::composite(C::Viewport, "/CampaignButtonsViewport", false));
        let mut panel = Widget::new(C::Panel, "/MissionPanel", false);
        let mut list_panel = Widget::new(C::Panel, "MissionListboxPanel", false);
        let mut list = Widget::composite(C::Listbox, "MissionListbox", false);
        if let Some(l) = list.listbox_mut() {
            l.on_select = ListCallback::Command("CampaignSelectMission %s".into());
            l.on_double = ListCallback::Command("CampaignConfirmPlay".into());
        }
        list_panel.add_child(list);
        panel.add_child(list_panel);
        let mut difficulty = Widget::new(C::Panel, "DifficultyPanel", false);
        let mut dropdown = Widget::composite(C::DropDown, "DifficultyDropdown", false);
        if let Some(Control::DropDown(d)) = dropdown.control.as_deref_mut() {
            d.command = Some("CampaignDifficulty %s".into());
        }
        difficulty.add_child(dropdown);
        panel.add_child(difficulty);
        iface.add_child(panel);
        for (name, command) in [("/PlayButton", "CampaignConfirmPlay"), ("/ReplayButton", "CampaignPlay"), ("/LoadGameButton", "CampaignLoadGame"), ("/ExitButton", "CampaignExit")] {
            let mut b = Widget::new(C::PushButton, name, false);
            b.command = Some(command.into());
            b.add_child(Widget::new(C::Label, "Client", false));
            iface.add_child(b);
        }
        let mut globe = Widget::new(C::Cinematic, "/ChooseCampaignGlobeBink", true);
        globe.key = -f32::MAX;
        iface.add_child(globe);
        iface
    }

    /// Build the page (`0x642280`, then `0x643A40` and `0x643050`).
    pub fn show(&mut self, ui: &mut Ui) {
        ui.clear();
        self.load(ui);
        let mut iface = Self::tree();
        ui.load_widget(&mut iface, None);
        ui.connect(iface);
        // 0x643A40: a button per campaign
        for (i, c) in self.campaigns.clone().iter().enumerate() {
            let mut b = Widget::new(C::ToggleButton, "CampaignButton", false);
            b.command = Some(format!("CampaignSelectCampaign {}", c.ids));
            b.radio_group = 1;
            let mut label = Widget::new(C::Panel, "CampaignButtonDecalLabel", false);
            label.add_child(Widget::new(C::Decal, "Decal", false));
            label.add_child(Widget::new(C::Label, "Label", false));
            b.tag = TAG_BUTTON + i as u64;
            // 0x642F70: both are the interface's data nodes; the label panel is the
            // toggle's client (Client = CampaignButtonDecalLabel, 0x6420C0)
            if let Ok(n) = ui.store.node(&format!("{FILE}:{IFACE}")) {
                let strings = ui.strings.clone();
                let resolve = |v: &str| strings.resolve(v);
                let root = std::sync::Arc::new(n.clone());
                b.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
                label.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
            }
            b.add_child(label);
            ui.viewport_add(BUTTONS, b);
            let Some(path) = ui.find_tag(TAG_BUTTON + i as u64) else { continue };
            let rel = path.strip_prefix("Root/").unwrap_or(&path).to_string();
            let name = ui.strings.resolve(&c.name);
            ui.set_label(&format!("{rel}/CampaignButtonDecalLabel/Label"), &name);
            let texture = if c.state.active == 0 { &c.complete_texture } else { &c.incomplete_texture };
            let texture = Some(format!("/Campaign/{}", texture.trim_start_matches('/')));
            ui.with_widget(&format!("{rel}/CampaignButtonDecalLabel/Decal"), |w| {
                for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
                    if let aa_ui::ve::VeKind::Decal(d) = &mut ve.kind {
                        d.texture = texture.clone();
                    }
                }
            });
        }
        ui.viewport_refresh(BUTTONS);
        // the DifficultyLevel bank in the dropdown
        ui.listbox_clear(&format!("{DIFFICULTY}/Listbox"));
        for (ids, key) in self.levels.clone() {
            let text = ui.strings.resolve(&key);
            ui.listbox_add(&format!("{DIFFICULTY}/Listbox"), &text, &ids);
        }
        self.select_campaign(ui, self.campaign);
        ui.layout();
    }

    /// `0x643050` for the campaign (the toggles show it, `0x5F04C0`).
    fn select_campaign(&mut self, ui: &mut Ui, index: usize) {
        self.campaign = index.min(self.campaigns.len().saturating_sub(1));
        for i in 0..self.campaigns.len() {
            if let Some(p) = ui.find_tag(TAG_BUTTON + i as u64) {
                let rel = p.strip_prefix("Root/").unwrap_or(&p).to_string();
                let selected = i == self.campaign;
                ui.with_widget(&rel, |w| {
                    if selected {
                        w.flags |= aa_ui::state::widget_flag::SELECTED;
                    } else {
                        w.flags &= !aa_ui::state::widget_flag::SELECTED;
                    }
                });
            }
        }
        ui.listbox_clear(LIST);
        let Some(c) = self.campaigns.get(self.campaign).cloned() else { return };
        let mut last = None;
        for (i, m) in c.missions.iter().enumerate() {
            if c.state.states.get(i) == Some(&1) {
                continue;
            }
            let text = ui.strings.resolve(&m.name);
            let row = ui.listbox_add(LIST, &text, &i.to_string());
            last = Some((row, i));
        }
        if let Some((row, i)) = last {
            ui.listbox_select(LIST, row, false);
            self.mission = i;
        }
        // the record's difficulty (+1C)
        if (c.state.difficulty as usize) < self.levels.len() {
            ui.dropdown_select(DIFFICULTY, c.state.difficulty as usize);
        }
        ui.layout();
    }

    /// `0x46F760`: the DifficultyLevel IDS of the campaign's record.
    fn difficulty(&self) -> String {
        self.campaigns
            .get(self.campaign)
            .and_then(|c| self.levels.get(c.state.difficulty as usize))
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "medium".to_string())
    }

    /// `0x644200`.
    pub fn command(&mut self, ui: &mut Ui, command: &str) -> Option<Out> {
        let mut words = command.split_whitespace();
        let head = words.next()?.to_ascii_lowercase();
        let arg = words.next().map(|a| a.trim_matches('"').to_string());
        Some(match head.as_str() {
            "campaignselectcampaign" => {
                let ids = arg?;
                if let Some(i) = self.campaigns.iter().position(|c| c.ids.eq_ignore_ascii_case(&ids)) {
                    self.select_campaign(ui, i);
                }
                Out::None
            }
            "campaignselectmission" => {
                if let Some(i) = arg.and_then(|a| a.parse::<usize>().ok()) {
                    self.mission = i;
                }
                Out::None
            }
            "campaigndifficulty" => {
                let level = arg.and_then(|a| self.levels.iter().position(|(ids, _)| ids.eq_ignore_ascii_case(&a)));
                if let (Some(level), Some(c)) = (level, self.campaigns.get_mut(self.campaign)) {
                    c.state.difficulty = level as u32;
                    save_state(ui, c);
                }
                Out::None
            }
            // 0x642AB0: with UVar UICampaignConfirmDependentCampaign and a dependent
            // campaign not finished, the confirmation; else CampaignPlay
            "campaignconfirmplay" => {
                if !ui.uvars.get_bool("UICampaignConfirmDependentCampaign") {
                    return Some(self.play(ui));
                }
                let dependent = self.campaigns.get(self.campaign).and_then(|c| c.dependent.clone());
                let record = dependent.as_deref().and_then(|d| self.campaigns.iter().find(|c| c.ids.eq_ignore_ascii_case(d)));
                let Some(record) = record.filter(|r| r.state.active != 0) else { return Some(self.play(ui)) };
                let dependent_name = ui.strings.resolve(&record.name);
                let name = self.campaigns.get(self.campaign).map(|c| ui.strings.resolve(&c.name)).unwrap_or_default();
                let text = ui.strings.format("campaign_dependent_campaign_confirm|ss", &[&dependent_name, &name]);
                let ok = ui.strings.resolve("#button_ok");
                let cancel = ui.strings.resolve("#button_cancel");
                ui.show_message_box(
                    aa_ui::message_box::SHARED,
                    aa_ui::message_box::MessageBoxRequest {
                        text,
                        buttons: vec![(ok, "Uvar UICampaignConfirmDependentCampaign 0 ; CampaignPlay".into()), (cancel, String::new())],
                    },
                );
                Out::Asked
            }
            "campaignplay" => self.play(ui),
            "campaignloadgame" => {
                bevy::log::warn!("CampaignLoadGame refused: the saved-game dialog (0x68DB60) is not built");
                Out::None
            }
            "campaignexit" => Out::Exit,
            _ => return None,
        })
    }

    /// `0x644130` -> PlayMap `0x643CA0`.
    fn play(&mut self, ui: &mut Ui) -> Out {
        let mission = self.mission;
        // 0x644190: +18 = -1, a cinematic through 0x644100 (+18 = the mission: its
        // end does not go on to the next one), a playable mission through 0x6440A0
        if let Some(c) = self.campaigns.get_mut(self.campaign) {
            c.state.last = if c.missions.get(mission).is_some_and(|m| m.map.is_none()) { mission as i32 } else { -1 };
        }
        self.start(ui, mission)
    }

    /// `0x6440A0`: the mission becomes the current one (`0x46FE70`), a cinematic
    /// plays (`0x641D10`), a map launches (`0x643CA0`).
    fn start(&mut self, ui: &mut Ui, mission: usize) -> Out {
        let Some(c) = self.campaigns.get_mut(self.campaign) else { return Out::None };
        if mission >= c.missions.len() {
            return Out::None;
        }
        if c.state.current != mission {
            c.state.current = mission;
            save_state(ui, c);
        }
        self.mission = mission;
        let m = &c.missions[mission];
        // verification aid: AA_BEVY_MISSION=<map id> plays that mission
        if let Ok(map) = std::env::var("AA_BEVY_MISSION") {
            self.playing = Some((self.campaign, mission));
            return Out::Play { map, difficulty: self.difficulty() };
        }
        let Some(map) = m.map.clone() else {
            let name = m.cinematic.as_deref().map(super::cinematic::stem).unwrap_or_default();
            if super::cinematic::baked(&name) {
                return Out::Cinematic(name);
            }
            // 0x641D10: not baked: it ends at once (0x641400)
            bevy::log::warn!("campaign cinematic {:?} is not baked for the video player", m.cinematic);
            return self.cinematic_ended(ui);
        };
        self.playing = Some((self.campaign, mission));
        Out::Play { map, difficulty: self.difficulty() }
    }

    /// `0x641400`, the end of a cinematic: complete, and the next mission when the
    /// chooser did not ask for this one alone (+18), or the record finishes.
    pub fn cinematic_ended(&mut self, ui: &mut Ui) -> Out {
        let Some(c) = self.campaigns.get_mut(self.campaign) else { return Out::None };
        let at = c.state.current;
        c.state.complete_current();
        save_state(ui, c);
        if c.state.last == at as i32 {
            self.select_campaign(ui, self.campaign);
            return Out::None;
        }
        self.play_next(ui)
    }

    /// `0x641800` / `0x6413B0`: the next mission of the current one, after the last
    /// the record ends (`0x7CC750`: +4 = 0).
    pub fn play_next(&mut self, ui: &mut Ui) -> Out {
        let Some(c) = self.campaigns.get_mut(self.campaign) else { return Out::None };
        let next = c.state.current + 1;
        if next >= c.missions.len() {
            c.state.active = 0;
            save_state(ui, c);
            self.select_campaign(ui, self.campaign);
            return Out::None;
        }
        self.start(ui, next)
    }

    /// `0x644130`: the current mission again.
    pub fn replay(&mut self, ui: &mut Ui) -> Out {
        let Some(at) = self.campaigns.get(self.campaign).map(|c| c.state.current) else { return Out::None };
        self.start(ui, at)
    }

    /// The end of a mission (`0x60A140` + `0x651BD0`): a win completes it
    /// (`0x46FE30`). What the debriefing shows, `None` for the final mission won,
    /// which goes back to the chooser.
    pub fn mission_ended(&mut self, ui: &Ui, result: super::debriefing::MissionResult) -> Option<super::debriefing::Shown> {
        let (campaign, mission) = self.playing.take().unwrap_or((self.campaign, self.mission));
        let c = self.campaigns.get_mut(campaign)?;
        c.state.current = mission;
        if result.won {
            c.state.complete_current();
            save_state(ui, c);
        }
        let m = c.missions.get(mission)?;
        if result.won && c.finals.get(mission).copied().unwrap_or(false) {
            return None;
        }
        Some(super::debriefing::Shown {
            campaign_name: c.name.clone(),
            text: if result.won { m.debriefing_success.clone() } else { m.debriefing_failure.clone() },
            play_next: c.state.states.get(mission) == Some(&2) && mission + 1 < c.missions.len(),
            result,
        })
    }
}

/// What a mission's session takes from the map: the header's world rules
/// (`0x5A44B0` copies them into the session) and each kingdom's general.
#[derive(Clone, Debug)]
pub struct MissionSession {
    pub setup: aa_play::SkirmishSetup,
    /// (player index, the kingdom's faction of the map's header).
    pub factions: Vec<(usize, Option<String>)>,
    /// The rule StartingExperience.
    pub starting_experience: Option<f32>,
    /// The rule FogOfWar's value IDS.
    pub fog: Option<String>,
}

/// A mission's session (`0x643CA0`, the map's header): every kingdom with a
/// start location plays, the first playable one is the player's, the others
/// computer players with the map's nation, colour and team; a kingdom without a
/// map team is on its own side.
pub fn mission_setup(root: &Path, map_id: &str, data: &aa_play::GameData, player_name: &str, ui: &Ui) -> anyhow::Result<MissionSession> {
    let map = aa_play::GameMap::load_with(root, map_id, data)?;
    let info = crate::ui::gamedata::read_map(&ui.store.root, &format!("Maps/{map_id}.RMP")).or_else(|| crate::ui::gamedata::read_map(&ui.store.root, &format!("Maps/{map_id}.rmp")));
    let playable: Option<Vec<String>> = info.as_ref().map(|m| m.kingdoms.iter().filter(|k| k.playable && !k.independent).map(|k| k.ids.clone()).collect());
    let mut players = Vec::new();
    let mut factions = Vec::new();
    let mut human_seen = false;
    let teams = map.teams.len() as u32;
    for (start, &slot) in map.player_slots.iter().enumerate() {
        let s = &map.slots[slot];
        let nation = s.nation.clone().filter(|n| data.nation(n).is_some()).unwrap_or_else(|| if s.team == 0 { "usa".into() } else { "ger".into() });
        let can_play = playable.as_ref().is_none_or(|p| p.iter().any(|k| k.eq_ignore_ascii_case(&s.id)));
        let human = !human_seen && can_play;
        human_seen |= human;
        let color = s.color.and_then(|c| data.kingdom_colors.get(c as usize)).map_or([255, 255, 255], |c| c.1.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8));
        let team = if s.team == u32::MAX || s.team_id.is_empty() { teams + start as u32 } else { s.team };
        let player = aa_play::PlayerSetup {
            name: if human { player_name.to_string() } else { s.id.clone() },
            nation,
            team,
            color,
            controller: if human { aa_play::Controller::Human } else { aa_play::Controller::Computer(1) },
            start,
        };
        // the battle's local kingdom is kingdom 0 (scene.rs)
        if human {
            players.insert(0, player);
            for f in &mut factions {
                let (i, _): &mut (usize, Option<String>) = f;
                *i += 1;
            }
            factions.insert(0, (0, s.faction.clone()));
        } else {
            factions.push((players.len(), s.faction.clone()));
            players.push(player);
        }
    }
    anyhow::ensure!(human_seen, "{map_id}: no kingdom for the player");
    // the header's world rules (kind 2 fixed as binary32, 3 an enum index)
    let rule = |ids: &str| info.as_ref().and_then(|m| m.rules.iter().find(|r| r.0.eq_ignore_ascii_case(ids)).cloned());
    let fixed = |ids: &str| rule(ids).filter(|r| r.1 == 2).map(|r| f32::from_bits(r.2));
    let starting_resources = fixed("ResourceStockpileDefault");
    let starting_experience = fixed("StartingExperience");
    let fog = rule("FogOfWar").filter(|r| r.1 == 3).and_then(|r| {
        crate::ui::datafile::load(&ui.store.root, "Game/world_rules_aa.tgi")
            .iter()
            .filter(|n| n.name.eq_ignore_ascii_case("WorldRule"))
            .find(|n| n.get("ids").or(n.get("IDS")).is_some_and(|i| i.eq_ignore_ascii_case("FogOfWar")))
            .and_then(|n| n.children_named("EnumValue").into_iter().nth(r.2 as usize).and_then(|e| e.get("ids").or(e.get("IDS")).map(str::to_string)))
    });
    Ok(MissionSession {
        setup: aa_play::SkirmishSetup { map_id: map_id.to_string(), players, starting_resources, seed: 1 },
        factions,
        starting_experience,
        fog,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_reads_and_writes_the_originals_file() {
        // 70 bytes of a new tutorial record: version, IDS, +4, two missions, difficulty
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&20u16.to_le_bytes());
        for c in "campaign_aa_tutorial".encode_utf16() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
        for v in [1u32, 0, 0, 0, 1, 1] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        assert_eq!(bytes.len(), 70);
        let s = CampaignState::read(&bytes, "campaign_aa_tutorial", 2).unwrap();
        assert_eq!((s.active, s.states.clone(), s.difficulty), (1, vec![0, 1], 1));
        assert_eq!(s.write("campaign_aa_tutorial"), bytes);
        assert_eq!(s, CampaignState::new(2, 1));
        assert!(CampaignState::read(&bytes, "campaign_aa_allies", 2).is_none());
        assert!(CampaignState::read(&bytes[..60], "campaign_aa_tutorial", 2).is_none());
    }

    #[test]
    fn mission_map_paths_become_map_ids() {
        assert_eq!(map_id("/Maps/Campaign/Tutorial01.rmp"), "Campaign/tutorial01");
        assert_eq!(map_id("/Maps/Campaign/allies01_el_alamein.rmp"), "Campaign/allies01_el_alamein");
    }

    #[test]
    fn a_won_mission_opens_the_next_and_a_lost_one_does_not() {
        let mut s = CampaignState::new(4, 1);
        assert_eq!(s.states, [0, 1, 1, 1]);
        s.complete_current();
        assert_eq!(s.states, [2, 0, 1, 1]);
        // 0x46FE30 on a complete mission changes nothing
        s.states[1] = 1;
        s.complete_current();
        assert_eq!(s.states, [2, 1, 1, 1]);
        s.current = 3;
        s.complete_current();
        assert_eq!(s.states, [2, 1, 1, 2]);
        assert_eq!(s.active, 0);
    }
}
