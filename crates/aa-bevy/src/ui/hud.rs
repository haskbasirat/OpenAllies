//! The original game interface over the battle (`KKC_UIGame`, aa-ui
//! `hud::Hud`): economy bar, object display, action buttons, minimap,
//! message log, game time, game menu.
//!
//! aa-ui builds and lays out every widget from the original TGI data; this
//! module is the `HudWorld` the native world implements, filled from the
//! aa-play battle (behaviour-faithful mapping, see each method), and the
//! action-button pages:
//! * the buttons of a selection are the original actor commands
//!   (Data/Game/actor_commands*.tgi: IDS, icon, preferred_position) that
//!   the aa-play world can carry out;
//! * a building that recruits shows `recruit` (slot 0); pressing it opens
//!   the recruit page: one button per recruitable organization (its
//!   RegimentButtons icon), greyed while a required structure or property
//!   is missing, with a "Requires" tooltip row (`0x5F7350`, `0x63BDF0`),
//!   and `structure_cancel_queued`'s IconCancel in the last slot to go
//!   back (07-recruit.png);
//! * own regiments: move, kill, retreat (Run), stop, guard, explore and, for
//!   one regiment that attaches to HQs, hq_attach / hq_detach (the
//!   05-regiment layout), plus the formation buttons of the unit panel
//!   (`FormationChange <id>`, `0x629AB0`, Organizations/Formations icons);
//! * the members' abilities add kill, bombard and repair (`0x530510`: an
//!   organization's command mask is the OR of its members'), builders
//!   with a mover construct (its page lists the builders' structures);
//! * an airfield adds air_attack / air_recon (its denizen's AirAttackCost /
//!   AirReconCost); a research structure research (its page: `0x5F77B0`);
//! * own structures: recruit, CommandAll (ActionButtonInfo index 4, an HQ:
//!   select its attached regiments, `0x60CCD0`), set_rally_point, pack /
//!   unpack, sell (SellFactor of the cost in the tooltip, `0x5F6860`;
//!   disabled under siege) (the 06-corps layout);
//! * every button carries its command's name and tooltip for the tooltip
//!   (`0x5F6860`: name in TextFontIDSToolTipName, then the tooltip).
use crate::session::Battle;
use aa_play::{DefClass, Unit, World};
use aa_ui::hud::{
    CompanyInfo, HqSlot, Hud, HudAction, HudEconomy, HudImpact, HudResource, HudSelection, HudWorld, LayoutSlot, OwnerInfo,
    UnitIcon,
};
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};
use bevy::prelude::*;

/// What the HUD asks the battle side to do (consumed by `input.rs`).
#[derive(Clone, Debug, PartialEq)]
pub enum GameAction {
    /// `stop` for the selection.
    Stop,
    /// `recruit`: queue an organization at a building.
    Recruit { building: aa_play::ActorId, def: aa_play::DefId },
    /// A research page item (`PropertyDataSelectCommand`): research the
    /// building definition's research entry `index`.
    Research { building: aa_play::ActorId, index: usize },
    /// `pack`: a building back into its truck.
    Pack { building: aa_play::ActorId },
    /// `sell` (0x14).
    Sell { building: aa_play::ActorId },
    /// `hq_detach` (0x20) of the selected regiment.
    Detach { org: aa_play::OrgId },
    /// A formation button (`FormationChange <id>`): index into the layout
    /// formations, for the selected own regiments.
    SetFormation(usize),
    /// `CommandAll` (`0x60CCD0`): select every regiment attached to the HQ.
    CommandAll { building: aa_play::ActorId },
    /// `disband` (10) of the selection.
    Disband,
    /// An idle button / F5-F6 (`CycleSelection 1` idle builder, `2` idle
    /// detached organization): select the next one and centre on it.
    CycleIdle(u32),
    /// Arm a targeted order: the next left click picks its target.
    Target(TargetOrder),
    /// ActorCommandCancel: drop the armed order (and its placement).
    CancelTarget,
    /// Minimap press/drag: jump the camera to (u, v) of the map.
    MinimapJump(f32, f32),
    TogglePause,
    /// The game menu pauses a single-player game while open.
    SetPaused(bool),
    /// The options dialog's game speed (GVar GameSpeed: 2^v).
    SetSpeed(f32),
    /// The options dialog's camera zoom (camera 0x7005F0).
    CameraZoom(f32),
    SpeedUp,
    SpeedDown,
    /// A +SELF general ability used at once (ui/abilities.rs, combat owner).
    UseAbility(usize),
    /// A recruit queue button (the structure's queue panel or an HQ slot
    /// holding a queued organization): cancel that queue entry.
    CancelRecruit { building: aa_play::ActorId, index: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub enum TargetOrder {
    /// `kill`: attack the clicked enemy.
    Attack,
    /// Truck `unpack`: deploy at the clicked ground point.
    Deploy(aa_play::ActorId),
    /// `move` (5): move to the clicked point.
    Move,
    /// `retreat` (7, "Run"): move ignoring enemies.
    Run,
    /// `guard` (2): guard the clicked object or area.
    Guard,
    /// `explore` (9): explore the clicked area.
    Explore,
    /// `hq_attach` (0x1F): attach to the clicked own HQ.
    AttachHq,
    /// `set_rally_point` (0x10) of a building.
    Rally(aa_play::ActorId),
    /// A general ability's targeting state (`UseFactionAbility %i`,
    /// ui/abilities.rs, combat owner).
    Ability(usize),
    /// `bombard` (1): bombard the clicked actor or location.
    Bombard,
    /// `repair` (3): repair the clicked friendly actor.
    Repair,
    /// A construct page item (`construct` 4): build that structure at the
    /// clicked location.
    Construct(aa_play::DefId),
    /// An airfield's `air_attack` (0x19) / `air_recon` (0x1A) at the
    /// clicked location.
    AirAttack(aa_play::ActorId),
    AirRecon(aa_play::ActorId),
}

/// HUD output for the battle input this frame.
#[derive(Resource, Default)]
pub struct GameInput {
    pub actions: Vec<GameAction>,
    /// Keys the UI did not take (the in-game hotkeys).
    pub keys: Vec<KeyCode>,
}

/// One original actor command (Data/Game/actor_commands*.tgi).
#[derive(Clone, Debug)]
struct ActorCommand {
    ids: String,
    icon: String,
    position: i32,
    /// `name` and `tooltip` string keys.
    name: String,
    tooltip: String,
}

/// One Organizations/Formations formation: name key and icon (Data path).
#[derive(Clone, Debug)]
struct FormationInfo {
    name: String,
    icon: String,
    /// The formation's property icon (shown in the unit panel).
    property_icon: Option<String>,
}

/// Team commands (Data/Game/team_commands.tgi) for the team page.
#[derive(Clone, Debug)]
struct TeamCommand {
    icon: String,
    position: i32,
    name: String,
    tooltip: String,
}

/// An ActionButtonInfo of UI/Game/action_buttons.tgi (CommandAll...).
#[derive(Clone, Debug, Default)]
struct ButtonInfo {
    name: String,
    tooltip: String,
    index: i32,
    texture: String,
}

#[derive(Clone, Debug, PartialEq)]
enum ActionPage {
    Main,
    Recruit(aa_play::ActorId),
    /// `research` (`0x5F77B0`): the structure's research items.
    Research(aa_play::ActorId),
    /// `construct` (`0x5F7350`, flag 0x40000): the structures the selected
    /// builders can build (actor +0x108, `0x4D4CF0`).
    Construct,
    /// GoToTeamCommands (ActionButtonInfo index 8): the team commands.
    Team,
}

/// A Property's research button data (PropertyData +0x30
/// action_button_icon, +0x18 description) and each of its modifiers'
/// `repair` field (modifier +0x14 bit 0, in source order).
#[derive(Clone, Debug, Default)]
struct ResearchArt {
    icon: String,
    description: String,
    repair: Vec<bool>,
}

pub struct GameHud {
    hud: Hud,
    resources: Vec<HudResource>,
    commands: Vec<ActorCommand>,
    formations: std::collections::HashMap<String, FormationInfo>,
    command_all: ButtonInfo,
    team_go: ButtonInfo,
    team_cancel: ButtonInfo,
    /// ActionButtonInfo ActorCommandCancel (action panel +0xCC): the only
    /// button while a construct item is being placed (`0x5F85A0`).
    command_cancel: ButtonInfo,
    team_commands: Vec<TeamCommand>,
    property_icons: std::collections::HashMap<String, String>,
    research_art: std::collections::HashMap<String, ResearchArt>,
    /// The command of the action button under the pointer (the economic
    /// impact preview of 0x602640).
    hovered: Option<String>,
    /// The structure the placement mode is placing and whether it is a
    /// construct item (its impact is previewed: Unpack `0x603A00` ->
    /// `0x603620` -> `0x602640`, a construct item `0x603B50` -> `0x603A00`).
    placing: Option<(aa_play::DefId, bool)>,
    /// Verification aid (script step tint_actions): every action button
    /// drawn tinted.
    pub debug_tint: Option<[f32; 3]>,
    /// Tooltip (name, text) of each action slot and formation button.
    slot_tips: Vec<Option<(String, String)>>,
    formation_tips: Vec<String>,
    page: ActionPage,
    /// Action slot commands of the current page (slot -> command string).
    slot_commands: Vec<Option<String>>,
    menu_open: bool,
    game_over: bool,
    last_events: usize,
    /// Economy notices already announced (books.notices accumulates).
    seen_notices: usize,
    /// The game speed of the last frame (the change message).
    last_speed: Option<f32>,
    /// The owned displays (F1 objectives, F2 technology, F3 message log,
    /// F4 politics).
    panels: super::panels::Panels,
    /// The session's StartingExperience world rule (the faction's
    /// experience at time 0).
    pub starting_experience: f32,
    /// General id -> its 3D portrait model (Data path), parsed once.
    general_portraits: std::collections::HashMap<String, Option<String>>,
    /// Actor names (cities) and kingdom display names.
    pub names: super::names::Names,
    /// The owners of the capturable actors last frame (who lost a capture).
    owned_before: std::collections::HashMap<aa_play::ActorId, aa_play::KingdomId>,
    /// The local general's abilities and their tooltips this frame
    /// (ui/abilities.rs, combat owner).
    abilities: Vec<aa_play::special_ops::AbilityView>,
    ability_tips: Vec<(String, String)>,
    /// Game events (message, sound, minimap ping; ui/game_events.rs).
    pub events: super::game_events::GameEvents,
}

const RESOURCE_KINDS: [(&str, u8); 3] = [("PurchaseResource", 0), ("UpkeepResource", 1), ("LimitedConsumedResource", 2)];

/// The resource registry in declaration order (Data/Game/resources.tgi,
/// `[0xB36530]+0x34`).
fn load_resources(ui: &mut Ui) -> Vec<HudResource> {
    let mut rows: Vec<(String, u8, String, Option<String>, Vec<(String, f32)>)> = Vec::new();
    for n in super::datafile::load(&ui.store.root, "Game/resources.tgi").iter().filter(|c| c.name.eq_ignore_ascii_case("Resource")) {
        let kind = RESOURCE_KINDS.iter().find(|(t, _)| n.template().is_some_and(|x| x.eq_ignore_ascii_case(t))).map(|(_, k)| *k);
        let Some(kind) = kind else { continue };
        let ids = n.get("IDS").unwrap_or_default().to_string();
        let icon = n.get("icon").unwrap_or_default().to_string();
        let limited = n.get("limited_by").filter(|s| !s.is_empty()).map(str::to_string);
        let cost = n
            .child("PurchaseCost")
            .map(|c| c.fields.iter().filter_map(|(k, v)| Some((k.clone(), v.trim().parse::<f32>().ok()?))).collect())
            .unwrap_or_default();
        rows.push((ids, kind, icon, limited, cost));
    }
    if rows.is_empty() {
        warn!("HUD: Game/resources.tgi gave no resources");
    }
    let ids: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
    rows.iter()
        .map(|(id, kind, icon, limited, cost)| HudResource {
            ids: id.clone(),
            kind: *kind,
            icon: icon.clone(),
            limited_by: limited.as_ref().and_then(|l| ids.iter().position(|x| x.eq_ignore_ascii_case(l))),
            limit_provider: rows.iter().any(|r| r.3.as_ref().is_some_and(|l| l.eq_ignore_ascii_case(id))),
            purchase_cost: ids.iter().map(|x| cost.iter().find(|(k, _)| k.eq_ignore_ascii_case(x)).map(|c| c.1).unwrap_or(0.0)).collect(),
        })
        .collect()
}

fn load_commands(ui: &mut Ui) -> Vec<ActorCommand> {
    let mut out = Vec::new();
    let mut files = vec!["Game/actor_commands.tgi".to_string(), "Game/actor_commands_aa.tgi".to_string()];
    files.extend(
        ui.store
            .root
            .list_dir("Game/ActorCommands")
            .into_iter()
            .filter(|f| f.to_ascii_lowercase().ends_with(".tgi"))
            .map(|f| if f.contains('/') { f } else { format!("Game/ActorCommands/{f}") }),
    );
    for f in files {
        for n in super::datafile::load(&ui.store.root, &f).iter().filter(|c| c.name.eq_ignore_ascii_case("ActorCommand")) {
            let ids = n.get("IDS").unwrap_or_default().to_string();
            if ids.is_empty() || out.iter().any(|c: &ActorCommand| c.ids == ids) {
                continue;
            }
            out.push(ActorCommand {
                ids,
                icon: n.get("icon").unwrap_or_default().to_string(),
                position: n.int("preferred_position").unwrap_or(-1),
                name: n.get("name").unwrap_or_default().trim_matches('"').to_string(),
                tooltip: n.get("tooltip").unwrap_or_default().trim_matches('"').to_string(),
            });
        }
    }
    if out.is_empty() {
        warn!("HUD: no actor commands read from Data/Game/actor_commands*.tgi");
    }
    out
}

/// Organizations/Formations/*.tgi: IDS (lower case) -> name and icon (the
/// icon path is relative to the formation file's folder).
fn load_formations(ui: &mut Ui) -> std::collections::HashMap<String, FormationInfo> {
    let mut out = std::collections::HashMap::new();
    for f in ui.store.root.list_dir("Organizations/Formations").into_iter().filter(|f| f.to_ascii_lowercase().ends_with(".tgi")) {
        for n in super::datafile::load(&ui.store.root, &format!("Organizations/Formations/{f}")).iter().filter(|n| n.name.eq_ignore_ascii_case("Formation")) {
            let Some(ids) = n.get("IDS") else { continue };
            let icon = n.get("icon").unwrap_or_default().trim_matches('"').to_string();
            let icon = if icon.starts_with('/') || icon.eq_ignore_ascii_case("null") { icon } else { format!("/Organizations/Formations/{icon}") };
            let property_icon = n
                .children_named("Property")
                .into_iter()
                .find_map(|p| p.get("icon").map(|s| s.trim_matches('"').to_string()))
                .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));
            out.insert(ids.to_ascii_lowercase(), FormationInfo { name: n.get("name").unwrap_or_default().trim_matches('"').to_string(), icon, property_icon });
        }
    }
    out
}

/// Properties/*.tgi: property IDS (lower case) -> icon (Data path).
fn load_property_icons(ui: &mut Ui) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for f in ui.store.root.list_dir("Properties").into_iter().filter(|f| f.to_ascii_lowercase().ends_with(".tgi")) {
        for n in super::datafile::load(&ui.store.root, &format!("Properties/{f}")).iter() {
            let (Some(ids), Some(icon)) = (n.get("IDS"), n.get("icon")) else { continue };
            let icon = icon.trim_matches('"').trim().to_string();
            if !icon.is_empty() && !icon.eq_ignore_ascii_case("null") {
                // a relative icon is below Properties/ (TechIcons/...,
                // PropertyIcons/...)
                let icon = if icon.starts_with('/') { icon } else { format!("/Properties/{icon}") };
                out.insert(ids.to_ascii_lowercase(), icon);
            }
        }
    }
    out
}

/// Properties/*.tgi: property IDS (lower case) -> its research button icon
/// (action_button_icon, below Properties/ when relative) and description.
fn load_research_art(ui: &mut Ui) -> std::collections::HashMap<String, ResearchArt> {
    let mut out = std::collections::HashMap::new();
    for f in ui.store.root.list_dir("Properties").into_iter().filter(|f| f.to_ascii_lowercase().ends_with(".tgi")) {
        for n in super::datafile::load(&ui.store.root, &format!("Properties/{f}")).iter() {
            let Some(ids) = n.get("IDS") else { continue };
            let clean = |s: Option<&str>| s.unwrap_or_default().trim_matches('"').trim().to_string();
            let icon = clean(n.get("action_button_icon"));
            let icon = if icon.is_empty() || icon.eq_ignore_ascii_case("null") || icon.starts_with('/') { icon } else { format!("/Properties/{icon}") };
            let description = clean(n.get("description"));
            let repair = n.children_named("Modifier").into_iter().map(|m| m.get("repair").is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))).collect();
            out.insert(ids.to_ascii_lowercase(), ResearchArt { icon, description, repair });
        }
    }
    out
}

fn load_team_commands(ui: &mut Ui) -> Vec<TeamCommand> {
    super::datafile::load(&ui.store.root, "Game/team_commands.tgi")
        .iter()
        .filter(|n| n.name.eq_ignore_ascii_case("TeamCommand"))
        .map(|n| TeamCommand {
            icon: n.get("icon").unwrap_or_default().split(';').next().unwrap_or_default().trim().to_string(),
            position: n.int("preferred_position").unwrap_or(-1),
            name: n.get("name").unwrap_or_default().trim_matches('"').to_string(),
            tooltip: n.get("tooltip").unwrap_or_default().trim_matches('"').to_string(),
        })
        .collect()
}

/// The numbered name of an actor (`0x4D8AB0`, the number per kingdom and
/// definition from `0x4E4980`): "%dst %s", "%dnd %s", "%drd %s", "%dth %s".
fn numbered(strings: Option<&aa_ui::resources::StringTable>, n: usize, name: &str) -> String {
    let (m10, m100) = (n % 10, n % 100);
    let key = if (11..=20).contains(&m100) || m10 == 0 || m10 > 3 {
        "#actor_name_numbered|ds"
    } else if m10 == 1 {
        "#actor_name_numbered_st|ds"
    } else if m10 == 2 {
        "#actor_name_numbered_nd|ds"
    } else {
        "#actor_name_numbered_rd|ds"
    };
    let Some(s) = strings else { return name.to_string() };
    s.resolve(key).replacen("%d", &n.to_string(), 1).replacen("%s", name, 1)
}

/// UI/Game/action_buttons.tgi `[<name> Template = ActionButtonInfo]`.
fn load_button_info(ui: &mut Ui, name: &str) -> ButtonInfo {
    let Ok(n) = ui.store.node(&format!("UI/Game/action_buttons.tgi:ActionButtons/{name}")) else { return ButtonInfo::default() };
    ButtonInfo {
        name: n.string_or_empty("name"),
        tooltip: n.string_or_empty("tooltip"),
        index: n.int("index").unwrap_or(-1),
        texture: n.string_or_empty("texture"),
    }
}

/// A resource cost for a tooltip: every non-zero stockpiled value with its
/// resource glyph (`0x5C4D10`).
fn cost_text(ui: &Ui, res: &[HudResource], values: &[f32]) -> String {
    let mut out = Vec::new();
    for (i, r) in res.iter().enumerate() {
        let v = values.get(i).copied().unwrap_or(0.0);
        if v == 0.0 || r.kind != 0 {
            continue;
        }
        let icon = ui.strings.glyphs.get(r.icon.to_ascii_lowercase().as_str()).copied().map(String::from).unwrap_or_default();
        out.push(format!("{icon}{v:.0}"));
    }
    out.join("  ")
}

/// An upkeep line for a tooltip (the economy label's format, `0x5C4CB0`):
/// every non-zero value with its glyph, signed, limited resources as their
/// amount.
fn upkeep_text(ui: &Ui, res: &[HudResource], values: &[f32]) -> String {
    let mut out = Vec::new();
    for (i, r) in res.iter().enumerate() {
        let v = values.get(i).copied().unwrap_or(0.0);
        if v == 0.0 || r.limit_provider {
            continue;
        }
        let icon = ui.strings.glyphs.get(r.icon.to_ascii_lowercase().as_str()).copied().map(String::from).unwrap_or_default();
        if r.kind == 2 {
            out.push(format!("{icon}{:.0}", v.abs()));
        } else if v > 0.0 {
            out.push(format!("{icon}+{v}"));
        } else {
            out.push(format!("{icon}{v}"));
        }
    }
    out.join(" ")
}

/// A definition's display name: a localization key (with or without '#'),
/// else the text as written.
pub fn name(ui: &Ui, key: &str) -> String {
    let k = if key.starts_with('#') { key.to_string() } else { format!("#{key}") };
    let s = ui.strings.resolve(&k);
    if s.is_empty() || s == ui.strings.resolve("#error_missing_string") { key.trim_start_matches('#').to_string() } else { s }
}

/// The nation skin of the local kingdom (`Skins/<NATION>`, KKC_UI::Reskin).
pub fn local_nation(b: &Battle) -> Option<String> {
    b.world.kingdoms.iter().find(|k| k.id == b.local).map(|k| k.nation.to_ascii_uppercase())
}

impl GameHud {
    pub fn show(ui: &mut Ui, battle: Option<&Battle>) -> Self {
        let resources = load_resources(ui);
        let commands = load_commands(ui);
        let formations = load_formations(ui);
        let command_all = load_button_info(ui, "CommandAll");
        let team_go = load_button_info(ui, "GoToTeamCommands");
        let team_cancel = load_button_info(ui, "CancelTeamCommands");
        let command_cancel = load_button_info(ui, "ActorCommandCancel");
        let team_commands = load_team_commands(ui);
        let property_icons = load_property_icons(ui);
        let research_art = load_research_art(ui);
        let names = super::names::Names::default();
        let view = View {
            battle,
            resources: &resources,
            commands: &commands,
            page: &ActionPage::Main,
            ui_strings: None,
            formations: &formations,
            command_all: &command_all,
            team_go: &team_go,
            team_cancel: &team_cancel,
            command_cancel: &command_cancel,
            team_commands: &team_commands,
            property_icons: &property_icons,
            research_art: &research_art,
            hovered: None,
            placing: None,
            debug_tint: None,
            starting_experience: 0.0,
            general_portrait: None,
            names: &names,
        };
        let hud = Hud::show(ui, &view);
        drop(view);
        let mut g = GameHud {
            hud,
            resources,
            commands,
            formations,
            command_all,
            team_go,
            team_cancel,
            command_cancel,
            team_commands,
            property_icons,
            research_art,
            hovered: None,
            placing: None,
            debug_tint: None,
            slot_tips: vec![None; 12],
            formation_tips: Vec::new(),
            page: ActionPage::Main,
            slot_commands: vec![None; 12],
            menu_open: false,
            game_over: false,
            last_events: 0,
            seen_notices: 0,
            last_speed: None,
            panels: Default::default(),
            starting_experience: 0.0,
            general_portraits: Default::default(),
            names: Default::default(),
            owned_before: Default::default(),
            abilities: Vec::new(),
            ability_tips: Vec::new(),
            events: Default::default(),
        };
        if let Some(b) = battle {
            g.update(ui, b);
        }
        g
    }

    /// Per-frame update from the battle.
    pub fn update(&mut self, ui: &mut Ui, b: &Battle) {
        // a recruit or research page whose building left the selection
        // closes
        if let ActionPage::Recruit(id) | ActionPage::Research(id) = self.page {
            if selected_structure(b) != Some(id) || b.world.actor(id).is_none() {
                self.page = ActionPage::Main;
            }
        }
        // the construct page closes without a builder in the selection
        if self.page == ActionPage::Construct {
            let orgs: Vec<&aa_play::Organization> = b
                .selection
                .iter()
                .filter_map(|u| match u {
                    Unit::Org(o) => b.world.orgs.get(*o as usize).filter(|o| o.alive && o.kingdom == b.local),
                    _ => None,
                })
                .collect();
            if construct_list(&b.world, &orgs).is_empty() {
                self.page = ActionPage::Main;
            }
        }
        self.names.update(&ui.store.root, &ui.strings, b);
        self.messages(ui, b);
        self.panels.update(ui, b, &self.property_icons);
        self.game_over = b.world.winner.is_some();
        let strings = ui.strings.clone();
        // a selected division HQ shows its kingdom's general
        let general_portrait = match b.selection.as_slice() {
            [Unit::Org(o)] => b.world.orgs.get(*o as usize).filter(|o| hq_structure(&b.world, o).is_some()).and_then(|o| self.general_portrait(ui, b, o.kingdom)),
            _ => None,
        };
        let view = View {
            battle: Some(b),
            resources: &self.resources,
            commands: &self.commands,
            page: &self.page,
            ui_strings: Some(&strings),
            formations: &self.formations,
            command_all: &self.command_all,
            team_go: &self.team_go,
            team_cancel: &self.team_cancel,
            command_cancel: &self.command_cancel,
            team_commands: &self.team_commands,
            property_icons: &self.property_icons,
            research_art: &self.research_art,
            hovered: self.hovered.as_deref(),
            placing: self.placing,
            debug_tint: self.debug_tint,
            starting_experience: self.starting_experience,
            general_portrait: general_portrait.as_deref(),
            names: &self.names,
        };
        let buttons = view.action_buttons();
        self.slot_commands = buttons.iter().map(|a| a.as_ref().map(|a| a.command.clone())).collect();
        self.slot_tips = buttons.iter().map(|a| a.as_ref().and_then(|a| view.tip(ui, &a.command))).collect();
        self.formation_tips = match view.selection() {
            HudSelection::Company(c) => c.formations.iter().map(|f| view.formation_name(f.id as usize)).collect(),
            _ => Vec::new(),
        };
        self.hud.update(ui, &view);
        // the generals' ability buttons: enabled state and tooltips
        self.abilities = aa_play::special_ops::abilities(&b.world, b.local);
        self.ability_tips = self.abilities.iter().map(|v| super::abilities::tip(ui, v)).collect();
        super::abilities::apply_enabled(ui, &self.abilities);
        // a truck is a simple actor in the original (SimpleActorDisplay: no
        // banner view); shown here in the company display without its
        // banner button
        let truck = match b.selection.first() {
            Some(Unit::Actor(a)) => b.world.actor(*a).is_some_and(|x| b.world.data.def(x.def).class == Some(DefClass::Truck)),
            _ => false,
        };
        // a structure outside any HQ organization (the Corps HQ) is a
        // simple actor too: its queue panel takes the banner's place
        let simple = match b.selection.as_slice() {
            [Unit::Actor(a)] => b.world.actor(*a).is_some_and(|x| b.world.data.def(x.def).class == Some(DefClass::Building) && aa_play::production::hq_org(&b.world, x.id).is_none()),
            _ => false,
        };
        let truck = truck || simple;
        for part in ["BannerButton", "Morale", "AttackEfficiency"] {
            ui.with_widget(&format!("Game/ControlPanel/ObjectDisplay/CompanyDisplay/{part}"), |w| w.set_visible(!truck));
        }
        recruit_queue(ui, b, &self.research_art);
    }

    /// The 3D portrait model of a kingdom's general (its Faction
    /// `portrait_ids` Thing: View resource_prefix beside the general's file,
    /// Generals/<NATION>/<general>.tgi).
    fn general_portrait(&mut self, ui: &Ui, b: &Battle, kingdom: aa_play::KingdomId) -> Option<String> {
        let g = aa_play::special_ops::general(&b.world, kingdom)?;
        let id = g.id.clone();
        let portrait = g.portrait.clone()?;
        if let Some(p) = self.general_portraits.get(&id) {
            return p.clone();
        }
        let nation = b.world.kingdoms.iter().find(|k| k.id == kingdom).map(|k| k.nation.to_ascii_uppercase()).unwrap_or_default();
        let file = format!("Generals/{nation}/{id}.tgi");
        let found = super::datafile::load(&ui.store.root, &file)
            .iter()
            .find(|n| n.name.eq_ignore_ascii_case("Thing") && n.get("ids").or(n.get("IDS")).is_some_and(|i| i.eq_ignore_ascii_case(&portrait)))
            .and_then(|n| n.children_named("View").find_map(|v| v.get("resource_prefix")).map(|p| format!("Generals/{nation}/{}.nif", p.trim())));
        self.general_portraits.insert(id, found.clone());
        found
    }

    /// This frame's game events (KKC_GameEventManager AddEvent 0x42D130,
    /// ui/game_events.rs: the message, its sound and its minimap ping) from
    /// the battle events and the economy notices, and the other messages
    /// (message system send 0x40B5C0 with the original message types).
    fn messages(&mut self, ui: &mut Ui, b: &Battle) {
        let w = &b.world;
        let names = &self.names;
        let kingdom_name = |k: Option<aa_play::KingdomId>| w.kingdoms.iter().find(|x| Some(x.id) == k).map(|x| names.kingdom(&x.name).to_string()).unwrap_or_default();
        // an actor's name: its own (a city) or its definition's
        let actor_name = |ui: &Ui, a: &aa_play::Actor| names.actor(a.id).map(str::to_string).unwrap_or_else(|| name(ui, &w.data.def(a.def).name));
        let org_name = |ui: &Ui, o: aa_play::OrgId| w.orgs.get(o as usize).map(|o| name(ui, &w.data.def(o.def).name)).unwrap_or_default();
        let anchor = |o: aa_play::OrgId| w.orgs.get(o as usize).map(|o| o.anchor);
        // (event, its %s arguments, its ground point, its kingdom)
        let mut events: Vec<(&str, Vec<String>, Option<Vec2>, Option<aa_play::KingdomId>)> = Vec::new();
        let mut plain: Vec<(&str, String)> = Vec::new();
        let local = Some(b.local);
        for e in &b.frame_events {
            match e {
                aa_play::Event::Recruited { kingdom, org: Some(o), .. } if *kingdom == b.local => {
                    events.push(("company_recruited", vec![org_name(ui, *o)], anchor(*o), local));
                }
                aa_play::Event::OrgDestroyed { org } => {
                    let Some(o) = w.orgs.get(*org as usize) else { continue };
                    if o.kingdom == b.local {
                        events.push(("mobile_actor_lost", vec![org_name(ui, *org)], Some(o.anchor), local));
                    } else {
                        events.push(("actor_destroyed", vec![kingdom_name(Some(o.kingdom)), org_name(ui, *org)], Some(o.anchor), local));
                    }
                }
                aa_play::Event::Died { actor } => {
                    let Some(a) = w.actors.get(*actor as usize) else { continue };
                    let def = w.data.def(a.def);
                    if def.class != Some(DefClass::Building) {
                        continue;
                    }
                    let at = Some(a.pos.truncate());
                    if a.kingdom == Some(b.local) {
                        events.push(("static_actor_lost", vec![name(ui, &def.name)], at, local));
                    } else if a.kingdom.is_some() {
                        events.push(("actor_destroyed", vec![kingdom_name(a.kingdom), name(ui, &def.name)], at, local));
                    }
                }
                // KKC_ConstructionComponent::OnConstructionFinished (0x565B00):
                // a finished structure (an unpacked truck's building) without a
                // wall link announces building_construction_complete to its
                // owner, the building's name
                aa_play::Event::Deployed { building } => {
                    let Some(a) = w.actors.get(*building as usize).filter(|a| a.kingdom == local) else { continue };
                    events.push(("building_construction_complete", vec![actor_name(ui, a)], Some(a.pos.truncate()), local));
                }
                // the same 0x565B00 announcement for a structure the
                // builders finished (construct)
                aa_play::Event::ConstructionFinished { actor, kingdom } if *kingdom == b.local => {
                    let Some(a) = w.actors.get(*actor as usize) else { continue };
                    events.push(("building_construction_complete", vec![actor_name(ui, a)], Some(a.pos.truncate()), local));
                }
                aa_play::Event::Promoted { org, .. } => {
                    if w.orgs.get(*org as usize).is_some_and(|o| o.kingdom == b.local) {
                        events.push(("company_experience_gained_level", vec![org_name(ui, *org)], anchor(*org), local));
                    }
                }
                // company_routed: UVar UIEventFilterEngagement (false by
                // default) drops it
                aa_play::Event::Routed { org } => {
                    if w.orgs.get(*org as usize).is_some_and(|o| o.kingdom == b.local) {
                        events.push(("company_routed", vec![org_name(ui, *org)], anchor(*org), local));
                    }
                }
                // global: every player hears allies_lost / enemies_won by the
                // defeated kingdom's relation (the sounds' who_flags)
                aa_play::Event::KingdomDefeated { kingdom } => {
                    events.push(("kingdom_defeated", vec![kingdom_name(Some(*kingdom))], None, Some(*kingdom)));
                }
                // game_events actor_captured (BUILDING_CAPTURED, "You have
                // captured %s") / actor_captured_by_enemy
                // (BUILDING_NEGATIVE, "%s captured by %s")
                aa_play::Event::Captured { actor, kingdom } => {
                    let Some(a) = w.actors.get(*actor as usize) else { continue };
                    let at = Some(a.pos.truncate());
                    if *kingdom == b.local {
                        events.push(("actor_captured", vec![actor_name(ui, a)], at, local));
                    } else if self.owned_before.get(actor).is_some_and(|k| *k == b.local) {
                        events.push(("actor_captured_by_enemy", vec![actor_name(ui, a), kingdom_name(Some(*kingdom))], at, local));
                    }
                }
                // refusals play a sound in the original, no log line (the
                // reason goes to the program log)
                aa_play::Event::Refused { kingdom, reason } if *kingdom == b.local => info!("command refused: {reason}"),
                // KKC_AbilityEffectDescribeEconomy::Use (0x4CC050): the user
                // gets "ability_effect_describe_economy|ss" (the target
                // kingdom's name, its economy line 0x4E7200) as GENERIC_EVENT
                aa_play::Event::EconomyDescribed { kingdom, of } if *kingdom == b.local => {
                    let name = kingdom_name(Some(*of));
                    let economy = describe_economy(ui, &self.resources, w, *of);
                    let text = ui.strings.resolve("#ability_effect_describe_economy|ss").replacen("%s", &name, 1).replacen("%s", &economy, 1);
                    info!("economy described: {text}");
                    plain.push(("GENERIC_EVENT", text));
                }
                _ => {}
            }
        }
        // the economy's notices since the last frame (they accumulate)
        let notices = &w.books.notices;
        if notices.len() < self.seen_notices {
            self.seen_notices = 0;
        }
        for n in &notices[self.seen_notices..] {
            match n {
                // 0x4E9C40: kingdom_upkeep_shortage (the short percent) /
                // kingdom_upkeep_repaired
                aa_play::books::Notice::UpkeepShortage { kingdom, percent } if *kingdom == b.local => {
                    events.push(("kingdom_upkeep_shortage", vec![percent.to_string()], None, local));
                }
                aa_play::books::Notice::UpkeepRepaired { kingdom } if *kingdom == b.local => {
                    events.push(("kingdom_upkeep_repaired", Vec::new(), None, local));
                }
                // 0x584120: research_complete, the property's name
                aa_play::books::Notice::Researched { kingdom, property } if *kingdom == b.local => {
                    let p = w.data.property(property).map(|p| name(ui, &p.name)).unwrap_or_else(|| property.clone());
                    events.push(("research_complete", vec![p], None, local));
                }
                // an unaffordable purchase (0x582950 recruit, 0x5845F0
                // research, 0x574A10 HQ attach): the error message with the
                // name and the price (0x5C4D10) as GENERIC_ERROR (0x40C230
                // type 0x11) to the buyer
                aa_play::books::Notice::Unaffordable { kingdom, what, cost, .. } if *kingdom == b.local => {
                    let price: Vec<f32> = self.resources.iter().map(|r| w.data.resource_index(&r.ids).and_then(|j| cost.get(j).copied()).unwrap_or(0.0)).collect();
                    let price = cost_text(ui, &self.resources, &price);
                    let (key, subject) = match what {
                        aa_play::books::Purchase::Recruit(d) => ("#recruit_error_affording|ss", name(ui, &w.data.def(*d).name)),
                        aa_play::books::Purchase::Research(p) => ("#research_error_affording|ss", w.data.property(p).map(|x| name(ui, &x.name)).unwrap_or_else(|| p.clone())),
                        // the organization actor's name (actor vfunc +0x24,
                        // 0x4D99D0 -> 0x4D8CA0 -> 0x4D8AB0): numbered per
                        // kingdom and definition
                        aa_play::books::Purchase::Attach(o) => {
                            let rank = w.orgs.get(*o as usize).map_or(1, |org| 1 + w.orgs.iter().filter(|x| x.id < org.id && x.kingdom == org.kingdom && x.def == org.def).count());
                            ("#hq_error_attach_affording|ss", numbered(Some(&ui.strings), rank, &org_name(ui, *o)))
                        }
                    };
                    let text = ui.strings.resolve(key).replacen("%s", &subject, 1).replacen("%s", &price, 1);
                    plain.push(("GENERIC_ERROR", text));
                }
                // an unaffordable air attack / air recon (0x56D5A0 /
                // 0x56D7C0 / 0x56DA00): denizen_error_affording|ss with the
                // subject string (denizen_air_attack / denizen_air_recon)
                // and the price (0x5C4D10), GENERIC_ERROR (type 0x11)
                aa_play::books::Notice::DenizenUnaffordable { kingdom, what, cost, .. } if *kingdom == b.local => {
                    let price: Vec<f32> = self.resources.iter().map(|r| w.data.resource_index(&r.ids).and_then(|j| cost.get(j).copied()).unwrap_or(0.0)).collect();
                    let price = cost_text(ui, &self.resources, &price);
                    let subject = ui.strings.resolve(&format!("#{what}"));
                    let text = ui.strings.resolve("#denizen_error_affording|ss").replacen("%s", &subject, 1).replacen("%s", &price, 1);
                    plain.push(("GENERIC_ERROR", text));
                }
                _ => {}
            }
        }
        self.seen_notices = notices.len();
        // a game speed change (0x76F3F0: |old - new| >= 0.0001 posts
        // networking_change_game_speed|f, 100 * 2^v, as GENERIC_EVENT)
        if self.last_speed.is_some_and(|s| (s.log2() - b.speed.log2()).abs() >= 0.0001) {
            let t = ui.strings.resolve("#networking_change_game_speed|f");
            let text = t.replacen("%.0f", &format!("{:.0}", b.speed * 100.0), 1).replace("%%", "%");
            plain.push(("GENERIC_EVENT", text));
        }
        self.last_speed = Some(b.speed);
        for (ids, args, at, owner) in events {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            self.events.fire(ui, ids, &args, at, owner);
        }
        for (kind, text) in plain {
            self.events.send_message(ui, kind, &text, None);
        }
        self.owned_before = w
            .actors
            .iter()
            .filter(|a| a.alive && w.data.def(a.def).captureable)
            .filter_map(|a| a.kingdom.map(|k| (a.id, k)))
            .collect();
        self.last_events = b.frame_events.len();
    }

    /// HUD commands: action buttons, control panel buttons, game menu.
    /// `None` = not a HUD command; `Some(actions)` = handled.
    pub fn command(&mut self, ui: &mut Ui, cmd: &str) -> Option<Vec<GameAction>> {
        if let Some(rest) = cmd.strip_prefix("AAPlay ") {
            return Some(self.action(rest).into_iter().collect());
        }
        // the idle buttons (0x623520): "CycleSelection <kind>"
        if let Some(rest) = cmd.strip_prefix("CycleSelection") {
            return Some(rest.trim().parse::<u32>().ok().map(GameAction::CycleIdle).into_iter().collect());
        }
        // the owned displays' toggles and close buttons
        if self.panels.command(ui, cmd) {
            return Some(Vec::new());
        }
        // the formation buttons (0x628490): "FormationChange <id> "
        if let Some(rest) = cmd.strip_prefix("FormationChange") {
            return Some(rest.trim().parse::<usize>().ok().map(GameAction::SetFormation).into_iter().collect());
        }
        // the general ability buttons (CommandUseFactionAbility 0x612750)
        if let Some(rest) = cmd.strip_prefix("UseFactionAbility") {
            return Some(rest.trim().parse::<usize>().ok().and_then(|i| super::abilities::command(ui, &self.abilities, i)).into_iter().collect());
        }
        Some(match cmd {
            // ControlPanel ButtonMenu -> the in-game menu (0x5C0C40)
            "OpenGameMenuDialog" => {
                self.open_menu(ui);
                vec![GameAction::SetPaused(true)]
            }
            "ReturnToGame" => {
                self.close_menu(ui);
                vec![GameAction::SetPaused(false)]
            }
            "GamePause" => vec![GameAction::TogglePause],
            // ControlPanel ButtonSupplyBorder / ctrl-s: UVar
            // ViewShowZoneFriendlySupply (the button shows it selected,
            // game_interface_update)
            "ToggleShowZoneSupply" => {
                let on = ui.uvars.get_bool("ViewShowZoneFriendlySupply");
                ui.uvars.set_bool("ViewShowZoneFriendlySupply", !on);
                vec![]
            }
            "SpeedUp" => vec![GameAction::SpeedUp],
            "SpeedDown" => vec![GameAction::SpeedDown],
            _ => return None,
        })
    }

    fn action(&mut self, rest: &str) -> Option<GameAction> {
        let mut it = rest.split_whitespace();
        let verb = it.next()?;
        let arg = it.next().and_then(|s| s.parse::<u32>().ok());
        let arg2 = it.next().and_then(|s| s.parse::<u32>().ok());
        match verb {
            "recruit_page" => {
                self.page = ActionPage::Recruit(arg?);
                None
            }
            "recruit_back" => {
                self.page = ActionPage::Main;
                None
            }
            "research_page" => {
                self.page = ActionPage::Research(arg?);
                None
            }
            "recruit" => Some(GameAction::Recruit { building: arg?, def: arg2? }),
            "research" => Some(GameAction::Research { building: arg?, index: arg2? as usize }),
            "cancel_recruit" => Some(GameAction::CancelRecruit { building: arg?, index: arg2? as usize }),
            "stop" => Some(GameAction::Stop),
            "kill" => Some(GameAction::Target(TargetOrder::Attack)),
            "move" => Some(GameAction::Target(TargetOrder::Move)),
            "retreat" => Some(GameAction::Target(TargetOrder::Run)),
            "guard" => Some(GameAction::Target(TargetOrder::Guard)),
            "explore" => Some(GameAction::Target(TargetOrder::Explore)),
            "bombard" => Some(GameAction::Target(TargetOrder::Bombard)),
            "repair" => Some(GameAction::Target(TargetOrder::Repair)),
            "construct_page" => {
                self.page = ActionPage::Construct;
                None
            }
            "construct" => Some(GameAction::Target(TargetOrder::Construct(arg?))),
            "air_attack" => Some(GameAction::Target(TargetOrder::AirAttack(arg?))),
            "air_recon" => Some(GameAction::Target(TargetOrder::AirRecon(arg?))),
            "hq_attach" => Some(GameAction::Target(TargetOrder::AttachHq)),
            "hq_detach" => Some(GameAction::Detach { org: arg? }),
            "unpack" => Some(GameAction::Target(TargetOrder::Deploy(arg?))),
            "pack" => Some(GameAction::Pack { building: arg? }),
            "set_rally_point" => Some(GameAction::Target(TargetOrder::Rally(arg?))),
            "sell" => Some(GameAction::Sell { building: arg? }),
            "command_all" => Some(GameAction::CommandAll { building: arg? }),
            "disband" => Some(GameAction::Disband),
            "team_page" => {
                self.page = ActionPage::Team;
                None
            }
            "team_back" => {
                self.page = ActionPage::Main;
                None
            }
            "command_cancel" => Some(GameAction::CancelTarget),
            _ => None,
        }
    }

    /// The in-game menu dialog (`0x5C0C40` -> `0x61D3B0`, Init `0x61D5D0`).
    fn open_menu(&mut self, ui: &mut Ui) {
        if self.menu_open {
            return;
        }
        let w = |c: C, n: &str| Widget::new(c, n, false);
        let mut d = Widget::new(C::TopLevel, "UI/Game/game_menu.tgi:GameMenuDialog", true);
        d.add_child(w(C::Label, "GamePausedLabel"));
        d.add_child(Widget::push_button("ButtonOptions", false, "OptionsDialog"));
        d.add_child(Widget::push_button("ButtonLoad", false, "GameLoadDialog"));
        d.add_child(Widget::push_button("ButtonSave", false, "GameSaveDialog"));
        let mut finished = w(C::PushButton, "ButtonFinished");
        finished.command = Some("RequestFinishedConfirm".into());
        finished.add_child(w(C::Label, "Client"));
        d.add_child(finished);
        d.add_child(Widget::push_button("ButtonQuit", false, "QuitConfirm"));
        d.add_child(Widget::push_button("ButtonConnectionStatus", false, "DisplayConnectionStatus"));
        d.add_child(Widget::push_button("ButtonReturnToGame", false, "ReturnToGame"));
        aa_ui::screens::show(ui, d);
        // the leave-game button's text by the game state (host, single
        // player against computer opponents); no connection status offline
        let key = if self.game_over { "#ui_game_menu_leave_game_host_game_over" } else { "#ui_game_menu_leave_game_host_game_not_over_only_human_vs_ai_left" };
        let text = ui.strings.resolve(key);
        ui.set_label("GameMenuDialog/ButtonFinished", &text);
        ui.with_widget("GameMenuDialog/ButtonConnectionStatus", |w| w.set_enabled(false));
        ui.layout();
        self.menu_open = true;
    }

    fn close_menu(&mut self, ui: &mut Ui) {
        if self.menu_open {
            ui.disconnect("GameMenuDialog");
            ui.layout();
            self.menu_open = false;
        }
    }

    pub fn menu_open(&self) -> bool {
        self.menu_open
    }

    /// The classic action hotkeys (Localization/Hotkeys/Actions/
    /// hotkeys_standard_aa.txt, `game|action|verify "ActorCommand <ids>"`):
    /// a key fires the action button showing that command.
    pub fn hotkey(&mut self, ui: &mut Ui, key: KeyCode) -> Option<Vec<GameAction>> {
        // the formation buttons: q / w / e (CompanyFormationButtons_0..2)
        let formation = match key {
            KeyCode::KeyQ => Some(0),
            KeyCode::KeyW => Some(1),
            KeyCode::KeyE => Some(2),
            _ => None,
        };
        if let Some(i) = formation {
            return (i < self.formation_tips.len()).then(|| vec![GameAction::SetFormation(i)]);
        }
        // F1..F4: the control panel's display buttons
        if self.panels.hotkey(ui, key) {
            return Some(Vec::new());
        }
        // the idle buttons: F5 idle builder, F6 idle detached organization
        match key {
            KeyCode::F5 => return Some(vec![GameAction::CycleIdle(1)]),
            KeyCode::F6 => return Some(vec![GameAction::CycleIdle(2)]),
            _ => {}
        }
        // hotkeys_custom.txt: y / h / n / j "UseFactionAbility 0..3"
        let ability = match key {
            KeyCode::KeyY => Some(0),
            KeyCode::KeyH => Some(1),
            KeyCode::KeyN => Some(2),
            KeyCode::KeyJ => Some(3),
            _ => None,
        };
        if let Some(i) = ability {
            return self.command(ui, &format!("UseFactionAbility {i}"));
        }
        let verbs: &[&str] = match key {
            KeyCode::KeyS => &["stop"],
            KeyCode::KeyA => &["kill", "command_all"],
            KeyCode::KeyM => &["move"],
            KeyCode::KeyR => &["retreat"],
            KeyCode::KeyG => &["guard"],
            KeyCode::KeyX => &["explore"],
            KeyCode::KeyF => &["set_rally_point"],
            KeyCode::KeyP => &["pack", "unpack"],
            KeyCode::KeyD => &["recruit_page"],
            KeyCode::KeyC => &["recruit_back", "hq_attach"],
            KeyCode::Delete => &["sell", "hq_detach", "disband"],
            _ => return None,
        };
        let cmd = self.slot_commands.iter().flatten().find(|c| {
            let v = c.strip_prefix("AAPlay ").and_then(|r| r.split_whitespace().next()).unwrap_or("");
            verbs.contains(&v)
        })?;
        let cmd = cmd.clone();
        self.command(ui, &cmd)
    }

    /// The widget under the pointer: an action button's command is
    /// previewed in the economic impact bar.
    pub fn set_hover(&mut self, path: Option<&str>) {
        self.hovered = path.and_then(|p| {
            let at = p.find("ActionButtons/ActionButtonsButton")? + "ActionButtons/ActionButtonsButton".len();
            let i = match p[at..].strip_prefix('#') {
                Some(n) => n.split('/').next()?.parse::<usize>().ok()?,
                None => 0,
            };
            self.slot_commands.get(i).cloned().flatten()
        });
    }

    /// The structure being placed (the placement mode) and whether it is a
    /// construct item, previewed in the economic impact bar. Entering the
    /// construct placement closes the construct page (`0x5F85A0` ->
    /// `0x5F5300` releases the page command).
    pub fn set_placing(&mut self, placing: Option<(aa_play::DefId, bool)>) {
        if placing.is_some_and(|(_, construct)| construct) && self.page == ActionPage::Construct {
            self.page = ActionPage::Main;
        }
        self.placing = placing;
    }

    /// The tooltip (name, text) of an action button slot / formation button
    /// widget path, for the tooltip module.
    pub fn tooltip(&self, path: &str) -> Option<(String, String)> {
        let index = |seg: &str| -> Option<usize> {
            let at = path.find(seg)? + seg.len();
            let rest = &path[at..];
            if let Some(n) = rest.strip_prefix('#') {
                n.split('/').next()?.parse().ok()
            } else {
                Some(0)
            }
        };
        if path.contains("ActionButtons/ActionButtonsButton") {
            let i = index("ActionButtonsButton")?;
            return self.slot_tips.get(i).cloned().flatten();
        }
        if path.contains("FormationButtons/FormationButton") {
            let i = index("FormationButton")?;
            return self.formation_tips.get(i).map(|n| (String::new(), n.clone()));
        }
        // the general ability items (0x6081B0)
        if let Some(i) = super::abilities::item_index(path) {
            return self.ability_tips.get(i).cloned();
        }
        None
    }
}

/// A kingdom's economy line (`0x4E7200`): every resource that is not a
/// limit provider, two spaces apart, a limited one as `0x5C8F10` (used /
/// the provider's amount), the others as `0x5C9830` (net income, with the
/// stock for a stockpiled one), then the commander's experience
/// (`0x4CE5F0` -> `0x4CE3A0`: the experience glyph, balance / maximum).
fn describe_economy(ui: &Ui, resources: &[HudResource], w: &World, of: aa_play::KingdomId) -> String {
    let Some(k) = w.kingdoms.iter().find(|k| k.id == of) else { return String::new() };
    let n = resources.len();
    let index = |ids: &str| resources.iter().position(|r| r.ids.eq_ignore_ascii_case(ids));
    let (mut production, mut upkeep, mut stock) = (vec![0.0f32; n], vec![0.0f32; n], vec![0.0f32; n]);
    for (ids, i) in [("money", aa_play::data::MONEY), ("ammo", aa_play::data::AMMO), ("oil", aa_play::data::OIL)] {
        if let Some(j) = index(ids) {
            production[j] = k.income[i];
            stock[j] = k.stockpile[i];
        }
    }
    for ids in ["unit_limit_provided", "unit_limit_consumed", "building_limit_provided", "building_limit_consumed"] {
        if let (Some(j), Some((p, c))) = (index(ids), aa_play::economy::rates(w, of, ids)) {
            production[j] = p;
            upkeep[j] = c;
        }
    }
    let invalid = ui.uvars.get_point3("UIColorOptionInvalid");
    let mut s = String::new();
    for (i, r) in resources.iter().enumerate() {
        if r.limit_provider {
            continue;
        }
        if !s.is_empty() {
            s.push_str("  ");
        }
        let icon = ui.strings.glyphs.get(r.icon.to_ascii_lowercase().as_str()).copied().map(|c| c as u32).unwrap_or(0);
        if r.kind == 2 {
            let j = r.limited_by.unwrap_or(i);
            s.push_str(&aa_ui::hud::limit_text(icon, upkeep[i], production[j], invalid));
        } else {
            s.push_str(&aa_ui::hud::resource_text(icon, production[i] - upkeep[i], invalid, r.kind == 0, stock[i]));
        }
    }
    if let Some((value, max)) = aa_play::special_ops::experience(w, of) {
        let glyph = ui.avars.float("FontFactionExperienceGlyphIndex").map(|v| v as u32).unwrap_or(0);
        let mut t = match char::from_u32(glyph).filter(|_| glyph != 0) {
            Some(c) => format!("{c} {value:.0}"),
            None => format!("{value:.0}"),
        };
        if max >= 0.0 {
            t.push_str(&format!("/{max:.0}"));
        }
        s.push_str(&t);
    }
    s
}

/// The structure of a deployed division HQ organization: its living member
/// building with the recruit component (the HQ organization itself has
/// none).
fn hq_structure(w: &World, org: &aa_play::Organization) -> Option<aa_play::ActorId> {
    w.data.def(org.def).max_attachments?;
    org.members
        .iter()
        .copied()
        .find(|m| w.actor(*m).is_some_and(|a| a.alive && !a.removed && w.data.def(a.def).class == Some(DefClass::Building) && !w.data.def(a.def).recruits.is_empty()))
}

/// DenizenGroup behaviour bits (denizen +0x38 capability, +0x3C readiness):
/// AIR_OFFENSIVE (air_attack) and AIR_RECON (air_recon).
const AIR_OFFENSIVE: u32 = aa_play::denizen::AIR_OFFENSIVE;
const AIR_RECON: u32 = aa_play::denizen::AIR_RECON;

/// The denizen's capability (+0x38, `0x567F80(0)` / `0x567FA0(0)`).
fn air_capable(w: &World, actor: aa_play::ActorId, bit: u32) -> bool {
    w.books.actor(actor).and_then(|b| b.denizen.as_ref()).is_some_and(|d| d.behaviors & bit != 0)
}

/// The command mask bit of air_attack (0x19) / air_recon (0x1A)
/// (`0x569C70`: the capability and the readiness +0x3C, which `0x5696B0`
/// rebuilds from the aircraft at home; aa-play `denizen::enabled`).
fn air_ready(w: &World, actor: aa_play::ActorId, bit: u32) -> bool {
    let command = if bit == AIR_OFFENSIVE { aa_play::denizen::COMMAND_AIR_ATTACK } else { aa_play::denizen::COMMAND_AIR_RECON };
    aa_play::denizen::enabled(w, actor, command)
}

/// The one selected own structure (building or deployed HQ organization).
fn selected_structure(b: &Battle) -> Option<aa_play::ActorId> {
    let w = &b.world;
    match b.selection.as_slice() {
        [Unit::Actor(id)] => w.actor(*id).filter(|a| a.kingdom == Some(b.local)).map(|a| a.id),
        [Unit::Org(o)] => w.orgs.get(*o as usize).filter(|o| o.alive && o.kingdom == b.local).and_then(|o| hq_structure(w, o)),
        _ => None,
    }
}

/// The structures the selected own organizations can build (`0x4D4CF0`):
/// each member with a builder list (ActorData +0x504, `Def::builds`) and a
/// mover (`0x530510` gives construct only with one) adds the structures
/// it lists that are not in the list yet, each at the list's head (the
/// member's own list is itself head-built, so one member keeps the
/// authored order).
fn construct_list(w: &World, orgs: &[&aa_play::Organization]) -> Vec<aa_play::DefId> {
    let mut list: Vec<aa_play::DefId> = Vec::new();
    for o in orgs {
        for m in o.members.iter().filter_map(|m| w.actor(*m)).filter(|a| a.alive && !a.removed) {
            let d = w.data.def(m.def);
            if !d.has_component("MoverComponent") {
                continue;
            }
            for s in d.builds.iter().rev() {
                if !list.contains(s) {
                    list.insert(0, *s);
                }
            }
        }
    }
    list
}

/// A queue entry as the HUD shows it.
struct QueueEntry {
    def: aa_play::DefId,
    /// The item reserves an HQ attachment (shown in the HQ slots).
    attaches: bool,
    /// Index in the building's queue (CancelRecruit).
    index: usize,
    /// `0x5889B0` for the head (1 - (deadline - now or pause) / duration),
    /// 0 for the waiting ones.
    progress: f32,
    /// A research item (queue kind 1): its property id.
    research: Option<String>,
}

fn queue_items(w: &World, building: aa_play::ActorId) -> Vec<QueueEntry> {
    let Some(q) = w.books.actor(building).map(|b| &b.queue) else { return Vec::new() };
    let now = q.paused_at.unwrap_or(w.time);
    q.items
        .iter()
        .enumerate()
        .map(|(i, it)| QueueEntry {
            def: it.def,
            attaches: it.attaches,
            index: i,
            progress: if i == 0 && q.duration > 0.0 { (1.0 - (q.deadline - now) / q.duration).clamp(0.0, 1.0) } else { 0.0 },
            research: it.research.clone(),
        })
        .collect()
}

const DISPLAY: &str = "Root/Game/ControlPanel/ObjectDisplay/CompanyDisplay";
const SIMPLE_NODE: &str = "UI/Game/object_display.tgi:ObjectDisplay/SimpleActorDisplay";
const SLOT_NODE: &str = "UI/Game/object_display.tgi:ObjectDisplay/CompanyDisplay/HQAttached";
/// QueuePanel max_queue_widgets (KKD_MAX_WIDGETS_QUEUE_DISPLAY_BUTTONS 6:
/// the current button and 5 queue buttons), 3 per row when stacked.
const QUEUE_WIDGETS: usize = 6;
const QUEUE_ROW: usize = 3;

fn seg(name: &str, i: usize) -> String {
    if i == 0 { name.to_string() } else { format!("{name}#{i}") }
}

/// Load a code-built widget against the data node `parent`.
fn load_under(ui: &mut Ui, item: &mut Widget, parent: &str) {
    if let Ok(n) = ui.store.node(parent) {
        let strings = ui.strings.clone();
        let resolve = |v: &str| strings.resolve(v);
        let root = std::sync::Arc::new(n.clone());
        item.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
    }
}

fn set_decal(ui: &mut Ui, path: &str, texture: Option<String>) {
    ui.with_widget(path, |w| {
        if let Some(aa_ui::ve::ViewElement { kind: aa_ui::ve::VeKind::Decal(d), .. }) = &mut w.sub_ve {
            d.texture = texture;
        }
    });
}

/// SimpleActorDisplay QueuePanel (`0x629F20`): CurrentButton, Progress,
/// Queue with 5 QueueButtons and the QueueMoreIndicator.
fn queue_panel(ui: &mut Ui) -> Widget {
    let w = |c: C, n: &str| Widget::new(c, n, false);
    let button = |n: &str| {
        let mut b = w(C::PushButton, n);
        b.client_name = "UnitButtonDecal".into();
        b.add_child(w(C::Decal, "UnitButtonDecal"));
        b
    };
    let mut p = w(C::Panel, "QueuePanel");
    p.add_child(button("CurrentButton"));
    p.add_child(w(C::StatusBar, "Progress"));
    let mut q = w(C::Panel, "Queue");
    for _ in 1..QUEUE_WIDGETS {
        q.add_child(button("QueueButton"));
    }
    q.add_child(w(C::Decal, "QueueMoreIndicator"));
    p.add_child(q);
    load_under(ui, &mut p, SIMPLE_NODE);
    p
}

/// The selected structure's recruit queue, as the original shows it:
/// * a structure without an HQ organization (the Corps HQ, a factory) has
///   the SimpleActorDisplay QueuePanel (`0x6286F0`): the head item in the
///   CurrentButton with the recruit progress below it (RecruitProgressBar:
///   red, yellow, green), the next ones in the QueueButtons, the
///   QueueMoreIndicator when more are queued than fit; no HQ slots;
/// * a division HQ shows its queued organizations in the HQ slots
///   (`0x62E0F0` / `0x62D9D0` type 2): the icon and the RecruitProgress bar
///   (the head's progress, 0 for the waiting ones) instead of health and
///   morale.
/// A research item (queue kind 1) shows its property's action_button_icon
/// (PropertyData +0x30, `0x6286F0`), a recruit its definition's icon
/// (ActorData +0xF0).
/// A queue button cancels its entry (CancelRecruit, the paid cost comes
/// back).
fn recruit_queue(ui: &mut Ui, b: &Battle, research_art: &std::collections::HashMap<String, ResearchArt>) {
    let w = &b.world;
    let structure = selected_structure(b).filter(|id| w.actor(*id).is_some_and(|a| !w.data.def(a.def).recruits.is_empty() || !w.data.def(a.def).research.is_empty()));
    let hq = structure.and_then(|id| aa_play::production::hq_org(w, id));
    let simple = structure.filter(|_| hq.is_none());
    if ui.widget(DISPLAY).is_none() {
        return;
    }
    // the structure queue panel
    let qp = format!("{DISPLAY}/QueuePanel");
    if simple.is_some() && ui.widget(&qp).is_none() {
        let p = queue_panel(ui);
        let _ = ui.attach(DISPLAY, p);
        ui.layout();
        // the queue buttons stacked 3 per row (should_queue_stack)
        let g = ui.geo();
        let bw = ui.widget(&format!("{qp}/Queue/QueueButton")).map(|x| (x.root.outer.rect.r - x.root.outer.rect.l) / g.sx).unwrap_or(32.0);
        let bh = ui.widget(&format!("{qp}/Queue/QueueButton")).map(|x| (x.root.outer.rect.b - x.root.outer.rect.t) / g.sy).unwrap_or(32.0);
        for i in 0..QUEUE_WIDGETS - 1 {
            ui.set_position(&format!("{qp}/Queue/{}", seg("QueueButton", i)), (i % QUEUE_ROW) as f32 * bw, (i / QUEUE_ROW) as f32 * bh);
        }
    }
    let rel = |p: &str| p.strip_prefix("Root/").unwrap_or(p).to_string();
    // any structure outside an HQ organization is a simple actor: no HQ
    // slots (a city block, a depot, a bunker, the Corps HQ)
    let lone = match b.selection.as_slice() {
        [Unit::Actor(a)] => w.actor(*a).is_some_and(|x| w.data.def(x.def).class == Some(DefClass::Building) && aa_play::production::hq_org(w, x.id).is_none()),
        _ => false,
    };
    // 0x628E60 -> 0x6247B0: the QueuePanel is there only while the
    // structure (own or allied) has something queued; with an empty queue
    // the panel is removed, no button, bar or indicator stays
    let items = simple.map(|id| queue_items(w, id)).unwrap_or_default();
    let friendly = simple.and_then(|id| w.actor(id)).and_then(|a| a.kingdom).is_some_and(|k| {
        let team = |k: u32| w.kingdoms.iter().find(|x| x.id == k).map(|x| x.team);
        k == b.local || team(k) == team(b.local)
    });
    let shown = simple.filter(|_| friendly && !items.is_empty());
    ui.with_widget(&rel(&qp), |x| x.set_visible(shown.is_some()));
    ui.with_widget(&rel(&format!("{DISPLAY}/HQAttached")), |x| x.set_visible(!lone));
    if let Some(id) = shown {
        let progress = items.first().map_or(0.0, |it| it.progress);
        aa_ui::hud::set_status(ui, &format!("{qp}/Progress"), progress);
        let tip = ui.strings.resolve("#object_display_SimpleActorDisplay_QueuePanel_Progress_ToolTip|f").replacen("%.0f", &format!("{:.0}", progress * 100.0), 1);
        ui.with_widget(&rel(&format!("{qp}/Progress")), |x| x.tooltip = tip.clone());
        for i in 0..QUEUE_WIDGETS {
            let bp = if i == 0 { format!("{qp}/CurrentButton") } else { format!("{qp}/Queue/{}", seg("QueueButton", i - 1)) };
            let item = items.get(i);
            let name = item
                .map(|it| match &it.research {
                    Some(p) => w.data.property(p).map(|x| name(ui, &x.name)).unwrap_or_else(|| p.clone()),
                    None => name(ui, &w.data.def(it.def).name),
                })
                .unwrap_or_default();
            let cmd = item.map(|it| format!("AAPlay cancel_recruit {id} {}", it.index));
            let icon = item.and_then(|it| match &it.research {
                Some(p) => research_art.get(&p.to_ascii_lowercase()).map(|x| x.icon.clone()).filter(|s| !s.is_empty()),
                None => w.data.def(it.def).icon.clone(),
            });
            set_decal(ui, &rel(&format!("{bp}/UnitButtonDecal")), icon);
            ui.with_widget(&rel(&bp), |x| {
                x.set_visible(item.is_some());
                x.command = cmd.clone();
                x.tooltip = name.clone();
            });
        }
        let more = items.len() > QUEUE_WIDGETS;
        ui.with_widget(&rel(&format!("{qp}/Queue/QueueMoreIndicator")), |x| x.set_visible(more));
    }
    // the HQ slots' queued organizations
    let queued: Vec<QueueEntry> = match (structure, hq) {
        (Some(id), Some(_)) => queue_items(w, id).into_iter().filter(|it| it.attaches).collect(),
        _ => Vec::new(),
    };
    let attached = match (structure, hq) {
        (Some(id), Some(o)) => {
            let members = &w.orgs[o as usize].members;
            w.orgs.iter().filter(|r| r.alive && r.hq.is_some_and(|h| h == id || members.contains(&h))).count()
        }
        _ => 0,
    };
    let slots = format!("{DISPLAY}/HQAttached");
    if ui.widget(&slots).is_none() {
        return;
    }
    for i in 0..6 {
        let sp = format!("{slots}/{}", seg("HQAttachedOrganization", i));
        if ui.widget(&sp).is_none() {
            continue;
        }
        let rp = format!("{sp}/RecruitProgress");
        if ui.widget(&rp).is_none() {
            let mut bar = Widget::new(C::StatusBar, "RecruitProgress", false);
            load_under(ui, &mut bar, &format!("{SLOT_NODE}/HQAttachedOrganization"));
            let _ = ui.attach(&sp, bar);
            ui.layout();
        }
        let entry = i.checked_sub(attached).and_then(|q| queued.get(q));
        ui.with_widget(&rel(&rp), |x| x.set_visible(entry.is_some()));
        if let (Some(e), Some(id)) = (entry, structure) {
            ui.with_widget(&rel(&format!("{sp}/Health")), |x| x.set_visible(false));
            ui.with_widget(&rel(&format!("{sp}/Morale")), |x| x.set_visible(false));
            aa_ui::hud::set_status(ui, &rp, e.progress);
            let tip = name(ui, &w.data.def(e.def).name);
            let cmd = format!("AAPlay cancel_recruit {id} {}", e.index);
            ui.with_widget(&rel(&format!("{sp}/Button")), |x| {
                x.command = Some(cmd.clone());
                x.tooltip = tip.clone();
            });
        } else {
            ui.with_widget(&rel(&format!("{sp}/Button")), |x| {
                if x.command.as_deref().is_some_and(|c| c.starts_with("AAPlay cancel_recruit")) {
                    x.command = None;
                }
            });
        }
    }
}

/// The battle as the HUD reads it.
struct View<'a> {
    battle: Option<&'a Battle>,
    resources: &'a [HudResource],
    commands: &'a [ActorCommand],
    page: &'a ActionPage,
    ui_strings: Option<&'a aa_ui::resources::StringTable>,
    formations: &'a std::collections::HashMap<String, FormationInfo>,
    command_all: &'a ButtonInfo,
    team_go: &'a ButtonInfo,
    team_cancel: &'a ButtonInfo,
    command_cancel: &'a ButtonInfo,
    team_commands: &'a [TeamCommand],
    property_icons: &'a std::collections::HashMap<String, String>,
    research_art: &'a std::collections::HashMap<String, ResearchArt>,
    hovered: Option<&'a str>,
    placing: Option<(aa_play::DefId, bool)>,
    /// Verification aid: every action button tinted (script tint_actions).
    debug_tint: Option<[f32; 3]>,
    starting_experience: f32,
    /// The selected division HQ's general portrait model.
    general_portrait: Option<&'a str>,
    names: &'a super::names::Names,
}

impl View<'_> {
    fn world(&self) -> Option<&World> {
        self.battle.map(|b| &b.world)
    }
    fn local(&self) -> Option<&aa_play::Kingdom> {
        let b = self.battle?;
        b.world.kingdoms.iter().find(|k| k.id == b.local)
    }
    fn name(&self, key: &str) -> String {
        match self.ui_strings {
            Some(s) => {
                let k = if key.starts_with('#') { key.to_string() } else { format!("#{key}") };
                let r = s.resolve(&k);
                if r.is_empty() || r == s.resolve("#error_missing_string") { key.trim_start_matches('#').to_string() } else { r }
            }
            None => key.to_string(),
        }
    }
    fn res_index(&self, ids: &str) -> Option<usize> {
        self.resources.iter().position(|r| r.ids.eq_ignore_ascii_case(ids))
    }
    fn command(&self, ids: &str) -> Option<&ActorCommand> {
        self.commands.iter().find(|c| c.ids == ids)
    }
    fn owner(&self, kingdom: Option<aa_play::KingdomId>) -> Option<OwnerInfo> {
        let b = self.battle?;
        // Independent kingdoms (cities, farms) show as neutral (no owner).
        let k = b.world.kingdoms.iter().find(|k| Some(k.id) == kingdom && !k.independent)?;
        let local = self.local()?;
        let c = k.color;
        // diplomatic colour 0x457CD0 -> 0x454D30 by the local kingdom's
        // relation (0 self green, 1 friend blue, 2 neutral grey 0.8, 3 enemy red)
        let relation = super::banners::relation_colors()[(aa_play::setup::relation(&b.world, Some(local.id), Some(k.id)) as usize).min(3)];
        Some(OwnerInfo {
            name: k.name.clone(),
            rank_glyph: None,
            kingdom_color: [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0],
            relation_color: Some(relation),
        })
    }
    /// The economy line of a definition (`0x56FE00`): production minus
    /// upkeep per resource in registry order (06-corps: "+30 +10 +5 +5 1",
    /// 05-regiment: "-10 1"; limited resources shown as their amount).
    fn upkeep(&self, def: &aa_play::Def) -> Vec<f32> {
        if let Some(w) = self.world() {
            if !def.production_all.is_empty() || !def.upkeep_all.is_empty() {
                return self
                    .resources
                    .iter()
                    .map(|r| match w.data.resource_index(&r.ids) {
                        Some(j) => def.production_all.get(j).copied().unwrap_or(0.0) - def.upkeep_all.get(j).copied().unwrap_or(0.0),
                        None => 0.0,
                    })
                    .collect();
            }
        }
        let mut v = vec![0.0; self.resources.len()];
        for (ids, i) in [("money", aa_play::data::MONEY), ("ammo", aa_play::data::AMMO), ("oil", aa_play::data::OIL)] {
            if let Some(j) = self.res_index(ids) {
                v[j] = -def.upkeep[i];
            }
        }
        if let Some(j) = self.res_index("unit_limit_consumed") {
            v[j] = def.unit_limit_consumed;
        }
        v
    }
    /// The highest-priority actor state the original would show
    /// (`0x4D1510`; Data/Game/actor_states*.tgi priorities: resting 4,
    /// entrenched 6, out of supply 6, moving / attacking 8, engaged /
    /// routing 12, packing / unpacking 14).
    fn state_text(&self, states: &[&str]) -> Option<String> {
        let w = self.world()?;
        let mut best: Option<(i32, &str)> = None;
        for s in states {
            let p = w.data.actor_states.get(*s).map(|d| d.priority).unwrap_or(0);
            if best.is_none_or(|(bp, _)| p > bp) {
                best = Some((p, s));
            }
        }
        best.map(|(_, s)| self.name(&format!("#actor_states_{s}_name")))
    }
    fn org_states(&self, org: &aa_play::Organization) -> Vec<&'static str> {
        let Some(w) = self.world() else { return Vec::new() };
        let mut v = vec!["mind_resting"];
        match &org.order {
            aa_play::Order::Move { .. } | aa_play::Order::Deploy { .. } => v.push("mind_moving"),
            aa_play::Order::Attack { .. } => v.push("mind_attacking"),
            aa_play::Order::Idle => {}
        }
        if let Some(c) = aa_play::combat::org_state(w, org.id).filter(|c| c.initialized) {
            if c.entrenched {
                v.push("organization_entrenched");
            }
            if c.engaging() {
                v.push("mind_engaged");
            }
            if c.routing.is_some() {
                v.push("mind_routing");
            }
        }
        if w.books.org(org.id).is_some_and(|s| s.out_of_supply) {
            v.push("out_of_supply");
        }
        v
    }
    /// The numbered name (`0x4D8AB0`): an organization or an HQ structure
    /// counts the kingdom's earlier ones of its definition (`0x4E4980`).
    fn display_name(&self, def: aa_play::DefId, kingdom: Option<aa_play::KingdomId>, rank: Option<usize>) -> String {
        let Some(w) = self.world() else { return String::new() };
        let name = self.name(&w.data.def(def).name);
        let _ = kingdom;
        match rank {
            Some(n) => numbered(self.ui_strings, n, &name),
            None => name,
        }
    }
    /// The portrait model (`0x624270`): the leader's `<thing>_portrait`
    /// Thing (View portrait_ids), else the leader's or the object's own
    /// model.
    fn portrait(&self, def: aa_play::DefId) -> Option<String> {
        let w = self.world()?;
        let d = w.data.def(def);
        let leader = d.leader_slot.and_then(|l| d.slots.get(l as usize)).map(|s| s.0).filter(|&x| x != aa_play::data::NO_DEF);
        let who = leader.or_else(|| d.slots.iter().map(|s| s.0).find(|&x| x != aa_play::data::NO_DEF)).unwrap_or(def);
        let wd = w.data.def(who);
        w.data.find(&format!("{}_portrait", wd.id)).and_then(|p| w.data.def(p).model.clone()).or_else(|| wd.model.clone()).or_else(|| d.model.clone())
    }
    fn org_rank(&self, org: &aa_play::Organization) -> usize {
        let Some(w) = self.world() else { return 1 };
        1 + w.orgs.iter().filter(|o| o.id < org.id && o.kingdom == org.kingdom && o.def == org.def).count()
    }
    /// The economy line of a definition through the production quote
    /// (`0x56F850` / `0x56F580` / `0x56F660`: the definition and, for an
    /// organization, every layout slot): production minus upkeep per
    /// resource in registry order.
    fn econ_of(&self, def: aa_play::DefId) -> Vec<f32> {
        let Some(w) = self.world() else { return vec![0.0; self.resources.len()] };
        let up = aa_play::production::quote(w, def, aa_play::production::Vector::Upkeep);
        let pr = aa_play::production::quote(w, def, aa_play::production::Vector::Production);
        self.resources
            .iter()
            .map(|r| match w.data.resource_index(&r.ids) {
                Some(j) => pr.get(j).copied().unwrap_or(0.0) - up.get(j).copied().unwrap_or(0.0),
                None => 0.0,
            })
            .collect()
    }
    fn cost_of(&self, def: aa_play::DefId) -> Vec<f32> {
        let Some(w) = self.world() else { return vec![0.0; self.resources.len()] };
        let c = aa_play::production::quote(w, def, aa_play::production::Vector::Cost);
        self.resources.iter().map(|r| w.data.resource_index(&r.ids).and_then(|j| c.get(j).copied()).unwrap_or(0.0)).collect()
    }
    fn company_for_org(&self, org: &aa_play::Organization) -> Option<CompanyInfo> {
        let w = self.world()?;
        let def = w.data.def(org.def);
        let members: Vec<&aa_play::Actor> = org.members.iter().filter_map(|m| w.actor(*m)).collect();
        let (mut hp, mut max) = (0.0, 0.0);
        for a in &members {
            hp += a.health.max(0.0);
            max += w.data.def(a.def).health;
        }
        // layout slots: one position per member slot, definition = the
        // member definition's order in the layout
        let mut defs: Vec<aa_play::DefId> = Vec::new();
        let mut slots = Vec::new();
        for (i, (mdef, _)) in def.slots.iter().enumerate() {
            let d = defs.iter().position(|x| x == mdef).unwrap_or_else(|| {
                defs.push(*mdef);
                defs.len() - 1
            });
            let unit = members.iter().find(|a| a.slot == Some(i as u16)).map(|a| UnitIcon {
                texture: w.data.def(a.def).icon.clone(),
                health: (a.health / w.data.def(a.def).health.max(1e-6)).clamp(0.0, 1.0),
                alt_background: false,
            });
            slots.push(LayoutSlot { position: i, definition: d as i32, unit });
        }
        let breakpoint = def.morale_breakpoint;
        // the formation buttons: up to 3, only for an own layout with more
        // than one formation (0x629AB0)
        let own = self.battle.is_some_and(|b| b.local == org.kingdom);
        let current = aa_play::player_orders::formation(w, org.id);
        let formations: Vec<aa_ui::hud::Formation> = if own && def.formations.len() > 1 {
            def.formations
                .iter()
                .enumerate()
                .take(3)
                .map(|(i, f)| aa_ui::hud::Formation {
                    id: i as i32,
                    texture: self.formations.get(&f.id.to_ascii_lowercase()).map(|x| x.icon.clone()).unwrap_or_default(),
                    selected: current == Some(i),
                })
                .collect()
        } else {
            Vec::new()
        };
        // properties (0x627240): the formation's property and the
        // entrenchment of an entrenched regiment
        let mut properties = Vec::new();
        if let Some(icon) = current.and_then(|i| def.formations.get(i)).and_then(|f| self.formations.get(&f.id.to_ascii_lowercase())).and_then(|f| f.property_icon.clone()) {
            properties.push(icon);
        }
        if aa_play::combat::org_state(w, org.id).is_some_and(|c| c.initialized && c.entrenched) {
            // infantry_entrench requires the infantry property
            let infantry = members.iter().any(|a| w.data.def(a.def).properties.iter().any(|p| p.eq_ignore_ascii_case("infantry")));
            let key = if infantry { "infantry_entrench" } else { "vehicle_entrench" };
            if let Some(icon) = self.property_icons.get(key) {
                properties.push(icon.clone());
            }
        }
        // an HQ truck organization (its members trucks) is a truck
        let truck = members.iter().all(|a| w.data.def(a.def).class == Some(DefClass::Truck)) && !members.is_empty();
        Some(CompanyInfo {
            name: if truck { self.display_name(org.def, Some(org.kingdom), None) } else { self.display_name(org.def, Some(org.kingdom), Some(self.org_rank(org))) },
            owner: self.owner(Some(org.kingdom)),
            portrait_model: if truck { members.first().and_then(|a| w.data.def(a.def).model.clone()) } else { self.portrait(org.def) },
            banner_model: None,
            health: if max > 0.0 { hp / max } else { 1.0 },
            morale: (org.morale.clamp(0.0, 1.0), breakpoint.clamp(0.0, 1.0)),
            attack_efficiency: (1.0, 1.0),
            supply: None,
            economy: Some(self.econ_of(org.def)),
            state: self.state_text(&self.org_states(org)),
            slots: if truck { Vec::new() } else { slots },
            leader_slot: -1,
            hq: None,
            formations,
            properties,
        })
    }
    /// A truck or another single actor (the SimpleActor display is not
    /// ported in aa-ui; the company display shows its name, owner, health,
    /// economy and state, without members).
    fn company_for_actor(&self, a: &aa_play::Actor) -> Option<CompanyInfo> {
        let w = self.world()?;
        let def = w.data.def(a.def);
        let mut states = vec!["mind_resting"];
        if matches!(a.order, aa_play::Order::Move { .. } | aa_play::Order::Deploy { .. }) {
            states.push("mind_moving");
        }
        Some(CompanyInfo {
            name: self.display_name(a.def, a.kingdom, None),
            owner: self.owner(a.kingdom),
            portrait_model: def.model.clone(),
            banner_model: None,
            health: (a.health / def.health.max(1e-6)).clamp(0.0, 1.0),
            morale: (1.0, 0.0),
            attack_efficiency: (1.0, 1.0),
            supply: None,
            economy: Some(self.econ_of(a.def)),
            state: self.state_text(&states),
            slots: Vec::new(),
            leader_slot: -1,
            hq: None,
            formations: Vec::new(),
            properties: Vec::new(),
        })
    }
    fn company_for_building(&self, a: &aa_play::Actor) -> Option<CompanyInfo> {
        let w = self.world()?;
        let def = w.data.def(a.def);
        // HQ attachments: the regiments attached to this HQ
        // the attachments of the building or of its HQ organization
        let hq_org = aa_play::production::hq_org(w, a.id);
        let hq_members: Vec<aa_play::ActorId> = hq_org.map(|o| w.orgs[o as usize].members.clone()).unwrap_or_default();
        let attached: Vec<Option<HqSlot>> = w
            .orgs
            .iter()
            .filter(|o| o.alive && o.hq.is_some_and(|h| h == a.id || hq_members.contains(&h)))
            .map(|o| {
                let (mut hp, mut max) = (0.0, 0.0);
                for m in o.members.iter().filter_map(|m| w.actor(*m)) {
                    hp += m.health.max(0.0);
                    max += w.data.def(m.def).health;
                }
                Some(HqSlot { texture: w.data.def(o.def).icon.clone(), health: if max > 0.0 { hp / max } else { 1.0 }, morale: o.morale.clamp(0.0, 1.0) })
            })
            .collect();
        let mut states = vec!["mind_resting"];
        if let Some(d) = w.books.actor(a.id).and_then(|s| s.deploy.as_ref()) {
            states.push(if d.kind == aa_play::books::DeployKind::Packing { "structure_packing" } else { "structure_unpacking" });
        }
        if !a.queue.is_empty() {
            // 4D64F0 by the item in progress (queue kind 1 is research)
            let research = w.books.actor(a.id).and_then(|s| s.queue.items.first()).is_some_and(|it| it.research.is_some());
            states.push(if research { "structure_researching" } else { "structure_recruiting" });
        }
        if aa_play::player_orders::under_siege(w, a.id) {
            states.push("under_siege");
        }
        // an HQ structure is numbered like its organization
        let rank = hq_org.map(|o| self.org_rank(&w.orgs[o as usize]));
        let mut hq = attached;
        // 0x62E0F0: the HQ's attachment list also holds the organizations
        // queued there (0x62D9D0 type 2: their icon and the recruit
        // progress, see `recruit_queue`), then the free attachments, up to
        // 6 slots
        if hq_org.is_some() {
            for item in queue_items(w, a.id).into_iter().filter(|it| it.attaches) {
                hq.push(Some(HqSlot { texture: w.data.def(item.def).icon.clone(), health: 1.0, morale: 1.0 }));
            }
        }
        let slots = hq_org.and_then(|o| w.data.def(w.orgs[o as usize].def).max_attachments).map_or(5, |m| m as usize).min(6);
        hq.resize(slots.max(hq.len()).min(6), None);
        Some(CompanyInfo {
            // a named actor (a city) shows its name (GetName vtable +0x24)
            name: self.names.building(w, a.id).map(str::to_string).unwrap_or_else(|| self.display_name(a.def, a.kingdom, rank)),
            owner: self.owner(a.kingdom),
            portrait_model: def.model.clone(),
            banner_model: None,
            health: (a.health / def.health.max(1e-6)).clamp(0.0, 1.0),
            morale: (1.0, 0.0),
            attack_efficiency: (1.0, 1.0),
            // a supply relay's stockpile (SupplyComponent +18 / stockpile_max)
            supply: (def.supply_relay && def.supply_stockpile_max > 0.0)
                .then(|| (w.books.actor(a.id).map_or(0.0, |s| s.supply_stock) / def.supply_stockpile_max).clamp(0.0, 1.0)),
            economy: Some(self.econ_of(a.def)),
            state: self.state_text(&states),
            slots: Vec::new(),
            leader_slot: -1,
            hq: Some(hq),
            formations: Vec::new(),
            properties: Vec::new(),
        })
    }
    /// `0x63BDF0`, the "Requires" row of an item tooltip while some of its
    /// requirements are missing (`0x581F70`'s / `0x5842F0`'s out lists):
    /// ui_tooltip_item_title_requires in UIColorOptionInvalid and, in the
    /// value column, every required structure (`0x638FB0`) and then every
    /// required property (`0x63BA00`) of the definition (an organization:
    /// of each layout slot), each list without repeats and sorted by display
    /// name, then id (`0x638650`), joined by ", " (`0x638A00`). A missing
    /// one is in UIColorOptionInvalid, and so is the separator after it. A
    /// missing property reads recruit_missing_required_property ("%s
    /// (Tech)"), or recruit_missing_required_property_tech_known ("%s (%s)")
    /// with the name of a structure of the kingdom's nation that researches
    /// it (ActorData +0x558).
    fn requires_row(&self, ui: &Ui, def: aa_play::DefId, missing_structures: &[aa_play::DefId], missing_properties: &[String]) -> Option<String> {
        if missing_structures.is_empty() && missing_properties.is_empty() {
            return None;
        }
        let w = self.world()?;
        let d = w.data.def(def);
        let reqs: Vec<&aa_play::data::Requirements> = if d.has_component("OrganizationComponent") {
            d.slots.iter().map(|s| s.0).filter(|&s| s != aa_play::data::NO_DEF).map(|s| &w.data.def(s).requirements).collect()
        } else {
            vec![&d.requirements]
        };
        let sorted = |mut v: Vec<(String, String)>| {
            v.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()).then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase())));
            v
        };
        let mut structures: Vec<(String, String)> = Vec::new();
        let mut structure_defs: Vec<aa_play::DefId> = Vec::new();
        for s in reqs.iter().flat_map(|r| r.structures.iter()) {
            if let Some(sd) = w.data.find(s).filter(|sd| !structure_defs.contains(sd)) {
                structure_defs.push(sd);
                structures.push((self.name(&w.data.def(sd).name), w.data.def(sd).id.clone()));
            }
        }
        let mut properties: Vec<(String, String)> = Vec::new();
        for p in reqs.iter().flat_map(|r| r.properties.iter()) {
            if !properties.iter().any(|(_, id)| id.eq_ignore_ascii_case(p)) {
                let name = w.data.property(p).map(|x| self.name(&x.name)).unwrap_or_else(|| p.clone());
                properties.push((name, p.clone()));
            }
        }
        let invalid = ui.uvars.get_point3("UIColorOptionInvalid");
        let red = |s: &str| aa_ui::text::color_wrap(s, invalid);
        let mut text = String::new();
        let mut prev_red = false;
        let mut add = |item: String, missing: bool| {
            if !text.is_empty() {
                text.push_str(&if prev_red { red(", ") } else { ", ".to_string() });
            }
            text.push_str(&if missing { red(&item) } else { item });
            prev_red = missing;
        };
        for (name, id) in sorted(structures) {
            let missing = w.data.find(&id).is_some_and(|sd| missing_structures.contains(&sd));
            add(name, missing);
        }
        let nation = self.local().map(|k| k.nation.clone()).unwrap_or_default();
        for (name, id) in sorted(properties) {
            if !missing_properties.iter().any(|m| m.eq_ignore_ascii_case(&id)) {
                add(name, false);
                continue;
            }
            let researcher = w.data.defs.iter().find(|x| {
                x.nation.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(&nation)) && x.research.iter().any(|r| r.property.eq_ignore_ascii_case(&id))
            });
            let item = match (researcher, self.ui_strings) {
                (Some(r), Some(s)) => s.resolve("#recruit_missing_required_property_tech_known|ss").replacen("%s", &name, 1).replacen("%s", &self.name(&r.name), 1),
                (None, Some(s)) => s.resolve("#recruit_missing_required_property|s").replacen("%s", &name, 1),
                (_, None) => name,
            };
            add(item, true);
        }
        Some(format!("{}<setx=100>{text}", red(&self.name("#ui_tooltip_item_title_requires"))))
    }
    fn act(&self, ids: &str, cmd: String) -> Option<(i32, HudAction)> {
        let c = self.command(ids)?;
        Some((c.position, HudAction { texture: c.icon.clone(), command: cmd, enabled: true, selected: false, targeted: false, tint: None }))
    }
    fn act_enabled(&self, ids: &str, cmd: String, enabled: bool) -> Option<(i32, HudAction)> {
        self.act(ids, cmd).map(|(p, mut a)| {
            a.enabled = enabled;
            (p, a)
        })
    }
    /// The formation's display name (layout formation index).
    fn formation_name(&self, index: usize) -> String {
        let Some((b, w)) = self.battle.zip(self.world()) else { return String::new() };
        let org = b.selection.iter().find_map(|u| match u {
            Unit::Org(o) => w.orgs.get(*o as usize),
            _ => None,
        });
        let Some(f) = org.and_then(|o| w.data.def(o.def).formations.get(index)) else { return String::new() };
        let name = self.formations.get(&f.id.to_ascii_lowercase()).map(|i| i.name.clone()).unwrap_or_else(|| f.id.clone());
        let text = self.name(&name);
        // template_game_FormationButton_ToolTip|s = "%s"
        match self.ui_strings {
            Some(s) => s.resolve("#template_game_FormationButton_ToolTip|s").replacen("%s", &text, 1),
            None => text,
        }
    }
    /// The (name, tooltip) of an action button's command (`0x5F6860`).
    fn tip(&self, ui: &Ui, cmd: &str) -> Option<(String, String)> {
        let rest = cmd.strip_prefix("AAPlay ")?;
        let mut it = rest.split_whitespace();
        let verb = it.next()?;
        let arg = it.next().and_then(|s| s.parse::<u32>().ok());
        let arg2 = it.next().and_then(|s| s.parse::<u32>().ok());
        let resolve = |k: &str| if k.is_empty() { String::new() } else { self.name(k) };
        let w = self.world()?;
        match verb {
            "recruit" => {
                // a recruit page item (0x5F64A0): the organization's name,
                // then the "Requires" row while a requirement is missing
                // (0x63BDF0), the "Cost" and "Upkeep" rows with the values
                // in a second column
                let (building, def) = (arg?, arg2?);
                let d = w.data.def(def);
                let cost = self.cost_of(def);
                let upkeep = self.econ_of(def);
                let up = upkeep_text(ui, self.resources, &upkeep);
                let mut rows = Vec::new();
                let missing = aa_play::production::recruit_requirements(w, building, def);
                rows.extend(self.requires_row(ui, def, &missing.structures, &missing.properties));
                rows.push(format!("{}<setx=100>{}", resolve("#ui_tooltip_item_title_cost"), cost_text(ui, self.resources, &cost)));
                if !up.is_empty() {
                    rows.push(format!("{}<setx=100>{}", resolve("#ui_tooltip_item_title_upkeep"), up));
                }
                Some((resolve(&d.name), rows.join("\n")))
            }
            "research" => {
                let (building, index) = (arg?, arg2? as usize);
                let a = w.actor(building)?;
                let r = w.data.def(a.def).research.get(index)?;
                let prop = w.data.property(&r.property);
                let name = prop.map(|p| resolve(&p.name)).unwrap_or_else(|| r.property.clone());
                let missing = aa_play::production::research_requirements(w, building, index);
                if missing.code == 1 {
                    // 0x5842F0: the button's whole tooltip is
                    // research_error_missing_requirements ("%s required:
                    // %s"), the missing structures then properties joined
                    // by grammatical_element_separator (0x4C5630 / 0x4C5560)
                    let sep = resolve("#grammatical_element_separator");
                    let list: Vec<String> = missing
                        .structures
                        .iter()
                        .map(|d| resolve(&w.data.def(*d).name))
                        .chain(missing.properties.iter().map(|p| w.data.property(p).map(|x| resolve(&x.name)).unwrap_or_else(|| p.clone())))
                        .collect();
                    let text = resolve("#research_error_missing_requirements|ss").replacen("%s", &name, 1).replacen("%s", &list.join(&sep), 1);
                    return Some((String::new(), text));
                }
                // 0x5F6790: the property's name, its research cost
                // (0x6401E0), "Affects" with the properties it requires of
                // an actor (0x63B8C0), its modifier lines after a blank line
                // (0x63BFD0, ui/modifiers.rs), then its description
                // (0x638D90).
                let cost: Vec<f32> = self.resources.iter().map(|hr| w.data.resource_index(&hr.ids).and_then(|j| r.cost.get(j).copied()).unwrap_or(0.0)).collect();
                let mut text = format!("{}<setx=100>{}", resolve("#ui_tooltip_item_title_cost"), cost_text(ui, self.resources, &cost));
                let affects: Vec<String> = prop.map(|p| p.required_properties.iter().map(|q| w.data.property(q).map(|x| resolve(&x.name)).unwrap_or_else(|| q.clone())).collect()).unwrap_or_default();
                if !affects.is_empty() {
                    text.push_str(&format!("\n{}<setx=100>{}", resolve("#ui_tooltip_item_title_required_properties"), affects.join(", ")));
                }
                let art = self.research_art.get(&r.property.to_ascii_lowercase());
                let lines: Vec<String> = prop
                    .map(|p| {
                        p.modifiers
                            .iter()
                            .enumerate()
                            .filter_map(|(i, m)| super::modifiers::describe(ui, &w.data, self.resources, m, art.and_then(|a| a.repair.get(i).copied()).unwrap_or(false)))
                            .collect()
                    })
                    .unwrap_or_default();
                if !lines.is_empty() {
                    text.push_str("\n\n");
                    text.push_str(&lines.join("\n"));
                }
                let description = art.map(|x| resolve(&x.description)).unwrap_or_default();
                if !description.is_empty() {
                    text.push_str("\n\n");
                    text.push_str(&description);
                }
                Some((name, text))
            }
            "research_page" => self.command("research").map(|c| (resolve(&c.name), resolve(&c.tooltip))),
            "construct_page" => self.command("construct").map(|c| (resolve(&c.name), resolve(&c.tooltip))),
            "construct" => {
                // 0x5F6590 (not an upgrade): the structure's name, the
                // "Cost" (0x640140) and "Upkeep" (0x640590) rows, then the
                // sections 0x639AB0 adds after one blank line before the
                // first of them (DAT_009F20AC "\n"):
                // - 0x63B270 "Recruits" (a StructureComponent's recruit
                //   list, ActorData +0x454): each organization whose
                //   property requirements the kingdom meets (0x4C5250 with
                //   the structure test off), followed by
                //   ui_tooltip_other_requirements|s with the other
                //   structures it requires (+0x2C8), joined by ", "
                //   (0x638A00);
                // - 0x63B5C0 "Upgrades" (ConstructionComponent [Upgrade],
                //   +0x4C0), 0x63AEF0 "Properties Provided" / "Technology"
                //   (Element / StructureComponent SupportProperty, Structure
                //   TechnologyProperty, +0x2D8 / +0x434 / +0x43C): no AA
                //   definition authors these lists;
                // - 0x63B740 "Researches": the research items 0x5842F0 lets
                //   the kingdom research; it needs a kingdom-level 0x5842F0
                //   (aa-play's takes the researching building), and no AA
                //   construct item has research items;
                // - 0x63A440: the description (ActorData +0x30), which no
                //   AA construct item has (aa-play does not load it).
                let def = arg?;
                let d = w.data.def(def);
                let cost = self.cost_of(def);
                let up = upkeep_text(ui, self.resources, &self.econ_of(def));
                let mut rows = vec![format!("{}<setx=100>{}", resolve("#ui_tooltip_item_title_cost"), cost_text(ui, self.resources, &cost))];
                if !up.is_empty() {
                    rows.push(format!("{}<setx=100>{}", resolve("#ui_tooltip_item_title_upkeep"), up));
                }
                let mut sections = Vec::new();
                if d.has_component("StructureComponent") {
                    let known = self.local().and_then(|k| w.books.kingdom(k.id)).map(|k| k.technologies.clone()).unwrap_or_default();
                    let has = |p: &String| known.iter().any(|t| t.eq_ignore_ascii_case(p));
                    let mut recruits = Vec::new();
                    for r in &d.recruits {
                        let rq = &w.data.def(*r).requirements;
                        let props_met = rq.properties.is_empty() || if rq.any_property { rq.properties.iter().any(has) } else { rq.properties.iter().all(has) };
                        if !props_met {
                            continue;
                        }
                        let others: Vec<String> = rq.structures.iter().filter_map(|s| w.data.find(s)).filter(|s| *s != def).map(|s| resolve(&w.data.def(s).name)).collect();
                        let mut item = resolve(&w.data.def(*r).name);
                        if !others.is_empty() {
                            item.push_str(&resolve("#ui_tooltip_other_requirements|s").replacen("%s", &others.join(", "), 1));
                        }
                        recruits.push(item);
                    }
                    if !recruits.is_empty() {
                        sections.push(format!("{}<setx=100>{}", resolve("#ui_tooltip_item_title_recruits"), recruits.join(", ")));
                    }
                }
                if !sections.is_empty() {
                    rows.push(String::new());
                    rows.extend(sections);
                }
                Some((resolve(&d.name), rows.join("\n")))
            }
            "recruit_back" => self.command("structure_cancel_queued").map(|c| (resolve(&c.name), resolve(&c.tooltip))),
            "command_all" => Some((resolve(&self.command_all.name), resolve(&self.command_all.tooltip))),
            "team_page" => Some((resolve(&self.team_go.name), resolve(&self.team_go.tooltip))),
            "team_back" => Some((resolve(&self.team_cancel.name), resolve(&self.team_cancel.tooltip))),
            "command_cancel" => Some((resolve(&self.command_cancel.name), resolve(&self.command_cancel.tooltip))),
            "team_command" => self.team_commands.get(arg? as usize).map(|t| (resolve(&t.name), resolve(&t.tooltip))),
            "recruit_page" => self.command("recruit").map(|c| (resolve(&c.name), resolve(&c.tooltip))),
            "sell" => {
                let c = self.command("sell")?;
                let a = w.actor(arg?)?;
                let value = aa_play::player_orders::sell_value(w, a.def);
                // registry order of the HUD resources
                let values: Vec<f32> = self.resources.iter().map(|r| w.data.resource_index(&r.ids).and_then(|i| value.get(i).copied()).unwrap_or(0.0)).collect();
                let text = resolve(&c.tooltip).replacen("%s", &cost_text(ui, self.resources, &values), 1);
                Some((resolve(&c.name), text))
            }
            ids => self.command(ids).map(|c| (resolve(&c.name), resolve(&c.tooltip))),
        }
    }
}

/// The local kingdom's idle units (`CycleSelection` kinds): 1 idle
/// builders (trucks that are not moving or unpacking), 2 idle detached
/// organizations (fighting regiments without an HQ, standing).
pub fn idle_units(b: &Battle, kind: u32) -> Vec<Unit> {
    let w = &b.world;
    let mut out = Vec::new();
    match kind {
        1 => {
            for a in w.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom == Some(b.local)) {
                let d = w.data.def(a.def);
                if d.class != Some(DefClass::Truck) || d.unpacks_to.is_none() {
                    continue;
                }
                let busy = w.books.actor(a.id).is_some_and(|s| s.deploy.is_some());
                let unit = match a.org {
                    Some(o) if w.orgs.get(o as usize).is_some_and(|x| x.alive) => {
                        if w.orgs[o as usize].order != aa_play::Order::Idle {
                            continue;
                        }
                        Unit::Org(o)
                    }
                    _ => Unit::Actor(a.id),
                };
                if a.order == aa_play::Order::Idle && !busy && !out.contains(&unit) {
                    out.push(unit);
                }
            }
        }
        _ => {
            for o in w.orgs.iter().filter(|o| o.alive && o.kingdom == b.local && o.hq.is_none() && o.order == aa_play::Order::Idle) {
                let d = w.data.def(o.def);
                let fights = o.members.iter().filter_map(|m| w.actor(*m)).any(|a| a.alive && !w.data.def(a.def).weapons.is_empty());
                if d.supply_used && d.max_attachments.is_none() && fights {
                    out.push(Unit::Org(o.id));
                }
            }
        }
    }
    out
}

/// Place actions at their preferred position, else the first free slot.
fn place(actions: Vec<(i32, HudAction)>) -> Vec<Option<HudAction>> {
    let mut v: Vec<Option<HudAction>> = vec![None; 12];
    let mut rest = Vec::new();
    for (p, a) in actions {
        if (0..12).contains(&p) && v[p as usize].is_none() {
            v[p as usize] = Some(a);
        } else {
            rest.push(a);
        }
    }
    for a in rest {
        if let Some(slot) = v.iter_mut().find(|s| s.is_none()) {
            *slot = Some(a);
        }
    }
    v
}

impl HudWorld for View<'_> {
    fn resources(&self) -> Vec<HudResource> {
        self.resources.to_vec()
    }

    /// The local kingdom's economy arrays (economy+0x1A0/+0x1B8/+0x1C4)
    /// from the aa-play kingdom: net per-minute income, stockpile, unit
    /// limit provided / used. aa-play keeps no building limit yet.
    fn economy(&self) -> Option<HudEconomy> {
        let k = self.local()?;
        let n = self.resources.len();
        let (mut production, mut upkeep, mut stock) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
        for (ids, i) in [("money", aa_play::data::MONEY), ("ammo", aa_play::data::AMMO), ("oil", aa_play::data::OIL)] {
            if let Some(j) = self.res_index(ids) {
                production[j] = k.income[i];
                stock[j] = k.stockpile[i];
            }
        }
        let _ = k;
        // All four limit resources as (provided, consumed) from the kingdom's
        // books; the bar shows upkeep[consumed] / production[provider].
        if let (Some(b), Some(w)) = (self.battle, self.world()) {
            for ids in ["unit_limit_provided", "unit_limit_consumed", "building_limit_provided", "building_limit_consumed"] {
                if let (Some(j), Some((p, c))) = (self.res_index(ids), aa_play::economy::rates(w, b.local, ids)) {
                    production[j] = p;
                    upkeep[j] = c;
                }
            }
        }
        let limited_max = self.world().and_then(|w| w.data.rule_ci("EconomyLimitedResourceMax")).unwrap_or(f32::MAX);
        Some(HudEconomy { preview_production: production.clone(), production, upkeep, stock, limited_max })
    }

    /// The hovered recruit item's impact (`0x602640`): its upkeep change
    /// (production minus upkeep, negative) and its cost.
    fn economic_impact(&self) -> Option<HudImpact> {
        let w = self.world()?;
        // the Unpack placement mode (0x603A00 -> 0x603620): the structure's
        // own production and upkeep, no cost (the original shows the second
        // bar while the ghost is out: "+30 +25" over "+25 +20" for a supply
        // depot)
        // a construct item (0x603B50 -> 0x603A00, a structure without an
        // organization layout): no upkeep change, the cost the first
        // selected builder is quoted (0x6025E0)
        if let Some((def, true)) = self.placing.filter(|_| self.hovered.is_none_or(|h| !h.starts_with("AAPlay recruit "))) {
            return Some(HudImpact { upkeep: vec![0.0; self.resources.len()], cost: self.cost_of(def) });
        }
        if let Some((def, false)) = self.placing.filter(|_| self.hovered.is_none_or(|h| !h.starts_with("AAPlay recruit "))) {
            let mut upkeep = self.econ_of(def);
            for (i, r) in self.resources.iter().enumerate() {
                if r.kind == 2 {
                    upkeep[i] = -upkeep[i].abs();
                }
            }
            return Some(HudImpact { upkeep, cost: vec![0.0; self.resources.len()] });
        }
        let rest = self.hovered?.strip_prefix("AAPlay recruit ")?;
        let def = rest.split_whitespace().nth(1)?.parse::<u32>().ok()?;
        let d = w.data.def(def);
        let mut upkeep = self.econ_of(def);
        // limited resources: the consumed amount as a negative change
        for (i, r) in self.resources.iter().enumerate() {
            if r.kind == 2 {
                upkeep[i] = -upkeep[i].abs();
            }
        }
        let _ = d;
        let cost = self.cost_of(def);
        Some(HudImpact { upkeep, cost })
    }

    /// The kingdom's experience (player +0x230): SVar
    /// ExperienceFactionGainPeriodic per second plus
    /// ExperienceFactionGainMultiplier of what its organizations gained,
    /// up to the faction's experience_max (Templates/template_faction_aa:
    /// 5000). Abilities spending it are not implemented.
    fn faction_experience(&self) -> Option<aa_ui::hud::FactionExperience> {
        let (b, w) = (self.battle?, self.world()?);
        // the commander's balance and maximum (special operations spend it)
        if let Some((value, max)) = aa_play::special_ops::experience(w, b.local) {
            return Some(aa_ui::hud::FactionExperience { value: value.floor(), max });
        }
        let periodic = w.data.rule_ci("ExperienceFactionGainPeriodic").unwrap_or(1.0);
        let mult = w.data.rule_ci("ExperienceFactionGainMultiplier").unwrap_or(1.0);
        let gained: f32 = w.orgs.iter().filter(|o| o.kingdom == b.local).map(|o| o.experience.max(0.0)).sum();
        let max = 5000.0;
        // the faction starts with the StartingExperience world rule
        Some(aa_ui::hud::FactionExperience { value: (self.starting_experience + periodic * w.time + mult * gained).floor().min(max), max })
    }

    fn faction_abilities(&self) -> Vec<aa_ui::hud::HudAbility> {
        // the local general's special operations (ui/abilities.rs)
        self.battle.map(|b| super::abilities::hud_items(&b.world, b.local)).unwrap_or_default()
    }

    fn group_textures(&self) -> Vec<Option<String>> {
        vec![None; 10]
    }

    fn idle_builder(&self) -> bool {
        self.battle.is_some_and(|b| !idle_units(b, 1).is_empty())
    }

    fn idle_detached_organization(&self) -> bool {
        self.battle.is_some_and(|b| !idle_units(b, 2).is_empty())
    }

    fn game_time(&self) -> f32 {
        self.world().map(|w| w.time).unwrap_or(0.0)
    }

    fn selection(&self) -> HudSelection {
        let (Some(b), Some(w)) = (self.battle, self.world()) else { return HudSelection::None };
        let Some(first) = b.selection.first() else { return HudSelection::None };
        match *first {
            Unit::Org(id) => {
                let Some(o) = w.orgs.get(id as usize).filter(|o| o.alive) else { return HudSelection::None };
                // a deployed division HQ: its structure's display (HQ slots,
                // recruit queue) with the organization's leader portrait
                let shown = match hq_structure(w, o).and_then(|s| w.actor(s)) {
                    Some(a) => self.company_for_building(a).map(|mut c| {
                        // the HQ's portrait is its kingdom's general (the
                        // Faction portrait_ids Thing, original-ui/06-corps)
                        c.portrait_model = self.general_portrait.map(str::to_string).or(c.portrait_model);
                        c
                    }),
                    None => self.company_for_org(o),
                };
                match shown {
                    Some(c) => HudSelection::Company(Box::new(c)),
                    None => HudSelection::None,
                }
            }
            Unit::Actor(id) => {
                let Some(a) = w.actor(id) else { return HudSelection::None };
                match w.data.def(a.def).class {
                    Some(DefClass::Building) => match self.company_for_building(a) {
                        Some(c) => HudSelection::Company(Box::new(c)),
                        None => HudSelection::None,
                    },
                    // trucks and other single actors (SimpleActor display)
                    _ => match self.company_for_actor(a) {
                        Some(c) => HudSelection::Company(Box::new(c)),
                        None => HudSelection::None,
                    },
                }
            }
        }
    }

    fn action_buttons(&self) -> Vec<Option<HudAction>> {
        // the page's buttons; a verification run can tint them all
        // (script step tint_actions: texture effect mode 2, or 3 when
        // disabled)
        let mut v = (|| -> Vec<Option<HudAction>> {
            let (Some(b), Some(w)) = (self.battle, self.world()) else { return vec![None; 12] };
            let own = |k: Option<aa_play::KingdomId>| k == Some(b.local);
            // placing a construct item (0x612A70 -> 0x5F85A0): the panel holds
            // only ActorCommandCancel, in the last slot, until the placement
            // ends
            if self.placing.is_some_and(|(_, construct)| construct) {
                let mut v: Vec<Option<HudAction>> = vec![None; 12];
                if !self.command_cancel.texture.is_empty() {
                    v[11] = Some(HudAction { texture: self.command_cancel.texture.clone(), command: "AAPlay command_cancel".into(), enabled: true, selected: false, targeted: false, tint: None });
                }
                return v;
            }
            match self.page {
                ActionPage::Recruit(id) => {
                    let id = *id;
                    let Some(a) = w.actor(id) else { return vec![None; 12] };
                    let def = w.data.def(a.def);
                    let mut v: Vec<Option<HudAction>> = vec![None; 12];
                    // 0x5F7350: a button per recruit, enabled while 0x581F70
                    // finds its requirements met (code 0); code 2 leaves its
                    // slot empty. The price is not part of it: an unaffordable
                    // recruit is refused when pressed (0x6157B0 -> 0x582950).
                    for (i, r) in def.recruits.iter().take(11).enumerate() {
                        let code = aa_play::production::recruit_requirements(w, id, *r).code;
                        if code == 2 {
                            continue;
                        }
                        let rd = w.data.def(*r);
                        v[i] = Some(HudAction {
                            texture: rd.icon.clone().unwrap_or_default(),
                            command: format!("AAPlay recruit {id} {r}"),
                            enabled: code == 0,
                            selected: false,
                            targeted: false,
                            tint: None,
                        });
                    }
                    if let Some(c) = self.command("structure_cancel_queued") {
                        v[11] = Some(HudAction { texture: c.icon.clone(), command: "AAPlay recruit_back".into(), enabled: true, selected: false, targeted: false, tint: None });
                    }
                    v
                }
                ActionPage::Construct => {
                    // 0x5F7350 (behavior flag 0x40000): a button per structure of
                    // the builders' list, up to 11 (0x5F6D30 with the
                    // definition's icon), the back button in the last slot
                    let own_orgs: Vec<&aa_play::Organization> = b
                        .selection
                        .iter()
                        .filter_map(|u| match u {
                            Unit::Org(o) => w.orgs.get(*o as usize).filter(|o| o.alive && own(Some(o.kingdom))),
                            _ => None,
                        })
                        .collect();
                    let mut v: Vec<Option<HudAction>> = vec![None; 12];
                    for (i, d) in construct_list(w, &own_orgs).into_iter().take(11).enumerate() {
                        v[i] = Some(HudAction { texture: w.data.def(d).icon.clone().unwrap_or_default(), command: format!("AAPlay construct {d}"), enabled: true, selected: false, targeted: false, tint: None });
                    }
                    if let Some(c) = self.command("structure_cancel_queued") {
                        v[11] = Some(HudAction { texture: c.icon.clone(), command: "AAPlay recruit_back".into(), enabled: true, selected: false, targeted: false, tint: None });
                    }
                    v
                }
                ActionPage::Research(id) => {
                    // 0x5F77B0: the structure's research items in order, up to
                    // 11 slots; 0x5844A0 code 2 shows nothing and takes no
                    // slot, code 4 (known, queued here or in progress in the
                    // kingdom) leaves its slot empty, code 1 (a structure
                    // missing) a disabled button, code 0 an enabled one
                    // (0x5F6EA0: PropertyDataSelectCommand, the property's
                    // action_button_icon); the back button as on the recruit
                    // page (0x5F6FE0 on the last slot)
                    let id = *id;
                    let Some(a) = w.actor(id) else { return vec![None; 12] };
                    let mut v: Vec<Option<HudAction>> = vec![None; 12];
                    let mut slot = 0;
                    for (index, r) in w.data.def(a.def).research.iter().enumerate() {
                        if slot > 10 {
                            break;
                        }
                        let code = aa_play::production::research_requirements(w, id, index).code;
                        match code {
                            2 => continue,
                            4 => {}
                            _ => {
                                let texture = self.research_art.get(&r.property.to_ascii_lowercase()).map(|x| x.icon.clone()).unwrap_or_default();
                                v[slot] = Some(HudAction { texture, command: format!("AAPlay research {id} {index}"), enabled: code == 0, selected: false, targeted: false, tint: None });
                            }
                        }
                        slot += 1;
                    }
                    if let Some(c) = self.command("structure_cancel_queued") {
                        v[11] = Some(HudAction { texture: c.icon.clone(), command: "AAPlay recruit_back".into(), enabled: true, selected: false, targeted: false, tint: None });
                    }
                    v
                }
                ActionPage::Team => {
                    // the team commands (Data/Game/team_commands.tgi); flares to
                    // allies are not implemented, so they stay disabled
                    let mut acts: Vec<(i32, HudAction)> = Vec::new();
                    for (i, t) in self.team_commands.iter().enumerate().filter(|(_, t)| !t.icon.is_empty()) {
                        if acts.iter().any(|(p, _)| *p == t.position) {
                            continue;
                        }
                        acts.push((t.position, HudAction { texture: t.icon.clone(), command: format!("AAPlay team_command {i}"), enabled: false, selected: false, targeted: false, tint: None }));
                    }
                    if !self.team_cancel.texture.is_empty() {
                        acts.push((self.team_cancel.index, HudAction { texture: self.team_cancel.texture.clone(), command: "AAPlay team_back".into(), enabled: true, selected: false, targeted: false, tint: None }));
                    }
                    place(acts)
                }
                ActionPage::Main => {
                    let mut acts = Vec::new();
                    let own_orgs: Vec<&aa_play::Organization> = b
                        .selection
                        .iter()
                        .filter_map(|u| match u {
                            Unit::Org(o) => w.orgs.get(*o as usize).filter(|o| o.alive && own(Some(o.kingdom)) && hq_structure(w, o).is_none()),
                            _ => None,
                        })
                        .collect();
                    let member_defs = |o: &aa_play::Organization| -> Vec<&aa_play::Def> { o.members.iter().filter_map(|m| w.actor(*m)).filter(|a| a.alive).map(|a| w.data.def(a.def)).collect() };
                    // a regiment that fights; an HQ truck organization only moves
                    let fighting = own_orgs.iter().any(|o| member_defs(o).iter().any(|d| !d.weapons.is_empty()));
                    let own_trucks: Vec<aa_play::ActorId> = b
                        .selection
                        .iter()
                        .filter_map(|u| match u {
                            Unit::Actor(a) => w.actor(*a).filter(|x| x.alive && own(x.kingdom) && w.data.def(x.def).class == Some(DefClass::Truck)).map(|x| x.id),
                            _ => None,
                        })
                        .collect();
                    // 0x530510 over the members' performer abilities (an
                    // organization's command mask is the OR of its members',
                    // 0x589A60): kill (0) for one with an ability (0x57E230),
                    // unless all its abilities bombard (0x57E2F0); bombard (1)
                    // for one with an ability flagged bombard (AbilityDefinition
                    // +0x54 bit 3, 0x57E280); repair (3) for one with a healing
                    // ability (0x57E1C0 -> 0x4BE970: a HEAL effect)
                    const BOMBARD: u32 = 1 << 3;
                    const HEAL: u32 = 3;
                    let performer = |d: &aa_play::Def| d.weapons.iter().filter(|x| !x.defender).cloned().collect::<Vec<_>>();
                    let caps: Vec<Vec<aa_play::data::Weapon>> = own_orgs.iter().flat_map(|o| member_defs(o)).map(performer).collect();
                    let can_kill = caps.iter().any(|ws| !ws.is_empty() && (!ws.iter().any(|x| x.flags & BOMBARD != 0) || ws.iter().any(|x| x.flags & BOMBARD == 0)));
                    let can_bombard = caps.iter().any(|ws| ws.iter().any(|x| x.flags & BOMBARD != 0));
                    let can_repair = caps.iter().any(|ws| ws.iter().any(|x| x.effects.iter().any(|e| e.kind == HEAL)));
                    if !own_orgs.is_empty() || !own_trucks.is_empty() {
                        acts.extend(self.act("move", "AAPlay move".into()));
                        if can_kill {
                            acts.extend(self.act("kill", "AAPlay kill".into()));
                        }
                        if fighting {
                            acts.extend(self.act("retreat", "AAPlay retreat".into()));
                        }
                        if can_bombard {
                            acts.extend(self.act("bombard", "AAPlay bombard".into()));
                        }
                        if can_repair {
                            acts.extend(self.act("repair", "AAPlay repair".into()));
                        }
                        // construct (4): a member with a mover and a build list
                        // (0x530510: mover and actor +0x108); the button opens
                        // the construct page (behavior flags 0x50030)
                        if !construct_list(w, &own_orgs).is_empty() {
                            acts.extend(self.act("construct", "AAPlay construct_page".into()));
                        }
                        // air_attack (0x19) / air_recon (0x1A) of a denizen
                        // member (a carrier in its organization): 0x5F88C0 adds
                        // them only through the selection's command mask (the
                        // organization's is its members' OR, 0x589A60), so only
                        // while one is ready, and enabled; the order goes to the
                        // first ready member
                        for (bit, ids) in [(AIR_OFFENSIVE, "air_attack"), (AIR_RECON, "air_recon")] {
                            let ready = own_orgs.iter().flat_map(|o| o.members.iter().copied()).find(|m| w.actor(*m).is_some_and(|a| a.alive) && air_ready(w, *m, bit));
                            if let Some(m) = ready {
                                acts.extend(self.act(ids, format!("AAPlay {ids} {m}")));
                            }
                        }
                        acts.extend(self.act("stop", "AAPlay stop".into()));
                        if fighting {
                            acts.extend(self.act("guard", "AAPlay guard".into()));
                            acts.extend(self.act("explore", "AAPlay explore".into()));
                        }
                    }
                    // the team commands page (ActionButtonInfo GoToTeamCommands)
                    if (!own_orgs.is_empty() && fighting) || selected_structure(b).is_some() && !self.team_go.texture.is_empty() {
                        let building = selected_structure(b).is_some_and(|id| w.actor(id).is_some_and(|a| own(a.kingdom) && w.data.def(a.def).class == Some(DefClass::Building)));
                        if (!own_orgs.is_empty() && fighting) || building {
                            acts.push((self.team_go.index, HudAction { texture: self.team_go.texture.clone(), command: "AAPlay team_page".into(), enabled: true, selected: false, targeted: false, tint: None }));
                        }
                    }
                    // one regiment that attaches to HQs: hq_attach / hq_detach
                    if let [Unit::Org(o)] = b.selection.as_slice() {
                        if let Some(org) = w.orgs.get(*o as usize).filter(|x| x.alive && own(Some(x.kingdom))) {
                            let od = w.data.def(org.def);
                            if od.max_attachments.is_none() && od.supply_used && fighting {
                                acts.extend(self.act_enabled("hq_attach", "AAPlay hq_attach".into(), org.hq.is_none()));
                                acts.extend(self.act_enabled("hq_detach", format!("AAPlay hq_detach {o}"), org.hq.is_some()));
                            }
                            // an HQ truck organization unpacks through its truck
                            if let Some(t) = org.members.iter().copied().find(|m| w.actor(*m).is_some_and(|a| a.alive && w.data.def(a.def).unpacks_to.is_some())) {
                                acts.extend(self.act("unpack", format!("AAPlay unpack {t}")));
                            }
                        }
                    }
                    // a structure: the selected building, or a deployed division
                    // HQ organization's structure (the recruit component is on
                    // the building, not on its organization)
                    if let Some(id) = &selected_structure(b) {
                        if let Some(a) = w.actor(*id).filter(|a| own(a.kingdom)) {
                            let def = w.data.def(a.def);
                            if !def.recruits.is_empty() {
                                acts.extend(self.act("recruit", format!("AAPlay recruit_page {id}")));
                                acts.extend(self.act("set_rally_point", format!("AAPlay set_rally_point {id}")));
                            }
                            // a ResearchComponent (actor +0xB4): research
                            // (actor_commands_aa.tgi, preferred_position 6)
                            if !def.research.is_empty() {
                                acts.extend(self.act("research", format!("AAPlay research_page {id}")));
                            }
                            // a denizen (0x5F88C0 for one own actor: an airfield,
                            // a carrier outside an organization): shown while it
                            // has the capability, +0x38 bits 0x10 / 0x40
                            // (0x567F80(0) / 0x567FA0(0)), enabled by the command
                            // mask (0x437810; 0x569C70: capability and readiness
                            // +0x3C)
                            if air_capable(w, *id, AIR_OFFENSIVE) {
                                acts.extend(self.act_enabled("air_attack", format!("AAPlay air_attack {id}"), air_ready(w, *id, AIR_OFFENSIVE)));
                            }
                            if air_capable(w, *id, AIR_RECON) {
                                acts.extend(self.act_enabled("air_recon", format!("AAPlay air_recon {id}"), air_ready(w, *id, AIR_RECON)));
                            }
                            // an HQ: CommandAll (ActionButtonInfo index 4)
                            let hq = aa_play::production::hq_org(w, *id).is_some() || def.has_component("HQComponent") || w.orgs.iter().any(|o| o.alive && o.hq == Some(*id));
                            if hq && !self.command_all.texture.is_empty() {
                                acts.push((self.command_all.index, HudAction { texture: self.command_all.texture.clone(), command: format!("AAPlay command_all {id}"), enabled: true, selected: false, targeted: false, tint: None }));
                            }
                            if def.unpacks_to.is_some() {
                                acts.extend(self.act("unpack", format!("AAPlay unpack {id}")));
                            }
                            if def.packs_to.is_some() {
                                acts.extend(self.act("pack", format!("AAPlay pack {id}")));
                            }
                            if aa_play::player_orders::sellable(w, *id) {
                                acts.extend(self.act_enabled("sell", format!("AAPlay sell {id}"), aa_play::player_orders::sell_refused(w, *id).is_none()));
                            }
                        }
                    }
                    // trucks and HQ truck organizations can be disbanded
                    if !own_trucks.is_empty() || (!own_orgs.is_empty() && !fighting) {
                        acts.extend(self.act("disband", "AAPlay disband".into()));
                    }
                    place(acts)
                }
            }
        })();
        if let Some(t) = self.debug_tint {
            for a in v.iter_mut().flatten() {
                a.tint = Some(t);
            }
        }
        v
    }
}
