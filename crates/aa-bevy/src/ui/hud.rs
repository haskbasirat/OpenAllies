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
//!   RegimentButtons icon) and `structure_cancel_queued`'s IconCancel in the
//!   last slot to go back (07-recruit.png).
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
    /// `pack`: a building back into its truck.
    Pack { building: aa_play::ActorId },
    /// Arm a targeted order: the next left click picks its target.
    Target(TargetOrder),
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum TargetOrder {
    /// `kill`: attack the clicked enemy.
    Attack,
    /// Truck `unpack`: deploy at the clicked ground point.
    Deploy(aa_play::ActorId),
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
}

#[derive(Clone, Debug, PartialEq)]
enum ActionPage {
    Main,
    Recruit(aa_play::ActorId),
}

pub struct GameHud {
    hud: Hud,
    resources: Vec<HudResource>,
    commands: Vec<ActorCommand>,
    page: ActionPage,
    /// Action slot commands of the current page (slot -> command string).
    slot_commands: Vec<Option<String>>,
    menu_open: bool,
    game_over: bool,
    last_events: usize,
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
            out.push(ActorCommand { ids, icon: n.get("icon").unwrap_or_default().to_string(), position: n.int("preferred_position").unwrap_or(-1) });
        }
    }
    if out.is_empty() {
        warn!("HUD: no actor commands read from Data/Game/actor_commands*.tgi");
    }
    out
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
        let view = View { battle, resources: &resources, commands: &commands, page: &ActionPage::Main, ui_strings: None };
        let hud = Hud::show(ui, &view);
        let mut g = GameHud {
            hud,
            resources,
            commands,
            page: ActionPage::Main,
            slot_commands: vec![None; 12],
            menu_open: false,
            game_over: false,
            last_events: 0,
        };
        if let Some(b) = battle {
            g.update(ui, b);
        }
        g
    }

    /// Per-frame update from the battle.
    pub fn update(&mut self, ui: &mut Ui, b: &Battle) {
        // a recruit page whose building left the selection closes
        if let ActionPage::Recruit(id) = self.page {
            if b.selection != [Unit::Actor(id)] || b.world.actor(id).is_none() {
                self.page = ActionPage::Main;
            }
        }
        self.messages(ui, b);
        self.game_over = b.world.winner.is_some();
        let strings = ui.strings.clone();
        let view = View { battle: Some(b), resources: &self.resources, commands: &self.commands, page: &self.page, ui_strings: Some(&strings) };
        self.slot_commands = view.action_buttons().into_iter().map(|a| a.map(|a| a.command)).collect();
        self.hud.update(ui, &view);
    }

    /// The game message log from this frame's battle events (message
    /// system send 0x40B5C0 with the original message types).
    fn messages(&mut self, ui: &mut Ui, b: &Battle) {
        let w = &b.world;
        let kingdom_name = |k: Option<aa_play::KingdomId>| w.kingdoms.iter().find(|x| Some(x.id) == k).map(|x| x.name.clone()).unwrap_or_default();
        let org_name = |ui: &Ui, o: aa_play::OrgId| w.orgs.get(o as usize).map(|o| name(ui, &w.data.def(o.def).name)).unwrap_or_default();
        // Data/Game/game_events.tgi: event -> message type, text
        // game_events_<event>_game_message (Localization strings)
        let fmt = |ui: &Ui, event: &str, args: &[&str]| -> String {
            let mut s = ui.strings.resolve(&format!("#game_events_{event}_game_message|*"));
            for a in args {
                s = s.replacen("%s", a, 1);
            }
            s
        };
        let mut out: Vec<(&str, String, Option<(f32, f32)>)> = Vec::new();
        for e in &b.frame_events {
            match e {
                aa_play::Event::Recruited { kingdom, org: Some(o), .. } if *kingdom == b.local => {
                    let at = w.orgs.get(*o as usize).map(|o| (o.anchor.x, o.anchor.y));
                    out.push(("COMPANY_GENERIC", fmt(ui, "company_recruited", &[&org_name(ui, *o)]), at));
                }
                aa_play::Event::OrgDestroyed { org } => {
                    let Some(o) = w.orgs.get(*org as usize) else { continue };
                    let at = Some((o.anchor.x, o.anchor.y));
                    if o.kingdom == b.local {
                        out.push(("COMPANY_DESTROYED", fmt(ui, "mobile_actor_lost", &[&org_name(ui, *org)]), at));
                    } else {
                        out.push(("COMPANY_POSITIVE", fmt(ui, "actor_destroyed", &[&kingdom_name(Some(o.kingdom)), &org_name(ui, *org)]), at));
                    }
                }
                aa_play::Event::Died { actor } => {
                    let Some(a) = w.actors.get(*actor as usize) else { continue };
                    let def = w.data.def(a.def);
                    if def.class != Some(DefClass::Building) {
                        continue;
                    }
                    let at = Some((a.pos.x, a.pos.y));
                    if a.kingdom == Some(b.local) {
                        out.push(("BUILDING_DESTROYED", fmt(ui, "static_actor_lost", &[&name(ui, &def.name)]), at));
                    } else if a.kingdom.is_some() {
                        out.push(("COMPANY_POSITIVE", fmt(ui, "actor_destroyed", &[&kingdom_name(a.kingdom), &name(ui, &def.name)]), at));
                    }
                }
                aa_play::Event::Routed { org } => {
                    if w.orgs.get(*org as usize).is_some_and(|o| o.kingdom == b.local) {
                        out.push(("COMPANY_ROUTED", fmt(ui, "company_routed", &[&org_name(ui, *org)]), None));
                    }
                }
                aa_play::Event::KingdomDefeated { kingdom } => {
                    out.push(("KINGDOM_DEFEATED", fmt(ui, "kingdom_defeated", &[&kingdom_name(Some(*kingdom))]), None));
                }
                // refusals play a sound in the original, no log line
                _ => {}
            }
        }
        for (kind, text, at) in out {
            let uv = ui.uvars.clone();
            ui.messages.send(&uv, kind, &text, at);
        }
        self.last_events = b.frame_events.len();
    }

    /// HUD commands: action buttons, control panel buttons, game menu.
    /// `None` = not a HUD command; `Some(actions)` = handled.
    pub fn command(&mut self, ui: &mut Ui, cmd: &str) -> Option<Vec<GameAction>> {
        if let Some(rest) = cmd.strip_prefix("AAPlay ") {
            return Some(self.action(rest).into_iter().collect());
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
            "recruit" => Some(GameAction::Recruit { building: arg?, def: arg2? }),
            "stop" => Some(GameAction::Stop),
            "kill" => Some(GameAction::Target(TargetOrder::Attack)),
            "unpack" => Some(GameAction::Target(TargetOrder::Deploy(arg?))),
            "pack" => Some(GameAction::Pack { building: arg? }),
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
        let verbs: &[&str] = match key {
            KeyCode::KeyS => &["stop"],
            KeyCode::KeyA => &["kill"],
            KeyCode::KeyP => &["pack", "unpack"],
            KeyCode::KeyD => &["recruit_page"],
            KeyCode::KeyC => &["recruit_back"],
            _ => return None,
        };
        let cmd = self.slot_commands.iter().flatten().find(|c| {
            let v = c.strip_prefix("AAPlay ").and_then(|r| r.split_whitespace().next()).unwrap_or("");
            verbs.contains(&v)
        })?;
        let cmd = cmd.clone();
        self.command(ui, &cmd)
    }
}

/// The battle as the HUD reads it.
struct View<'a> {
    battle: Option<&'a Battle>,
    resources: &'a [HudResource],
    commands: &'a [ActorCommand],
    page: &'a ActionPage,
    ui_strings: Option<&'a aa_ui::resources::StringTable>,
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
    /// Upkeep per resource of a definition (shown negative, unit limit
    /// consumed positive) in registry order.
    fn upkeep(&self, def: &aa_play::Def) -> Vec<f32> {
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
        Some(CompanyInfo {
            name: self.name(&def.name),
            owner: self.owner(Some(org.kingdom)),
            portrait_model: def.model.clone(),
            banner_model: None,
            health: if max > 0.0 { hp / max } else { 1.0 },
            morale: (org.morale.clamp(0.0, 1.0), breakpoint.clamp(0.0, 1.0)),
            attack_efficiency: (1.0, 1.0),
            supply: None,
            economy: Some(self.upkeep(def)),
            state: None,
            slots,
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
        let attached: Vec<Option<HqSlot>> = w
            .orgs
            .iter()
            .filter(|o| o.alive && o.hq == Some(a.id))
            .map(|o| {
                Some(HqSlot { texture: w.data.def(o.def).icon.clone(), health: 1.0, morale: o.morale.clamp(0.0, 1.0) })
            })
            .collect();
        let mut hq = attached;
        hq.resize(5.max(hq.len()), None);
        Some(CompanyInfo {
            name: self.name(&def.name),
            owner: self.owner(a.kingdom),
            portrait_model: def.model.clone(),
            banner_model: None,
            health: (a.health / def.health.max(1e-6)).clamp(0.0, 1.0),
            morale: (1.0, 0.0),
            attack_efficiency: (1.0, 1.0),
            // a supply relay's stockpile (SupplyComponent +18 / stockpile_max)
            supply: (def.supply_relay && def.supply_stockpile_max > 0.0)
                .then(|| (w.books.actor(a.id).map_or(0.0, |s| s.supply_stock) / def.supply_stockpile_max).clamp(0.0, 1.0)),
            economy: Some(self.upkeep(def)),
            state: None,
            slots: Vec::new(),
            leader_slot: -1,
            hq: Some(hq),
            formations: Vec::new(),
            properties: Vec::new(),
        })
    }
    fn act(&self, ids: &str, cmd: String) -> Option<(i32, HudAction)> {
        let c = self.command(ids)?;
        Some((c.position, HudAction { texture: c.icon.clone(), command: cmd, enabled: true, selected: false, targeted: false }))
    }
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

    fn economic_impact(&self) -> Option<HudImpact> {
        None
    }

    fn faction_experience(&self) -> Option<aa_ui::hud::FactionExperience> {
        None
    }

    fn faction_abilities(&self) -> Vec<aa_ui::hud::HudAbility> {
        Vec::new()
    }

    fn group_textures(&self) -> Vec<Option<String>> {
        vec![None; 10]
    }

    fn idle_builder(&self) -> bool {
        false
    }

    fn idle_detached_organization(&self) -> bool {
        false
    }

    fn game_time(&self) -> f32 {
        self.world().map(|w| w.time).unwrap_or(0.0)
    }

    fn selection(&self) -> HudSelection {
        let (Some(b), Some(w)) = (self.battle, self.world()) else { return HudSelection::None };
        let Some(first) = b.selection.first() else { return HudSelection::None };
        match *first {
            Unit::Org(id) => match w.orgs.get(id as usize).filter(|o| o.alive).and_then(|o| self.company_for_org(o)) {
                Some(c) => HudSelection::Company(Box::new(c)),
                None => HudSelection::None,
            },
            Unit::Actor(id) => {
                let Some(a) = w.actor(id) else { return HudSelection::None };
                match w.data.def(a.def).class {
                    Some(DefClass::Building) => match self.company_for_building(a) {
                        Some(c) => HudSelection::Company(Box::new(c)),
                        None => HudSelection::None,
                    },
                    // the SimpleActor display (trucks...) is not ported in aa-ui
                    _ => HudSelection::Unsupported("SimpleActor"),
                }
            }
        }
    }

    fn action_buttons(&self) -> Vec<Option<HudAction>> {
        let (Some(b), Some(w)) = (self.battle, self.world()) else { return vec![None; 12] };
        let own = |k: Option<aa_play::KingdomId>| k == Some(b.local);
        match self.page {
            ActionPage::Recruit(id) => {
                let id = *id;
                let Some(a) = w.actor(id) else { return vec![None; 12] };
                let def = w.data.def(a.def);
                let mut v: Vec<Option<HudAction>> = vec![None; 12];
                for (i, r) in def.recruits.iter().take(11).enumerate() {
                    let rd = w.data.def(*r);
                    let affordable = (0..3).all(|k| self.local().is_some_and(|l| l.stockpile[k] >= rd.cost[k]));
                    v[i] = Some(HudAction {
                        texture: rd.icon.clone().unwrap_or_default(),
                        command: format!("AAPlay recruit {id} {r}"),
                        enabled: affordable,
                        selected: false,
                        targeted: false,
                    });
                }
                if let Some(c) = self.command("structure_cancel_queued") {
                    v[11] = Some(HudAction { texture: c.icon.clone(), command: "AAPlay recruit_back".into(), enabled: true, selected: false, targeted: false });
                }
                v
            }
            ActionPage::Main => {
                let mut acts = Vec::new();
                let orgs = b.selection.iter().any(|u| matches!(u, Unit::Org(o) if w.orgs.get(*o as usize).is_some_and(|o| o.alive && own(Some(o.kingdom)))));
                if orgs {
                    acts.extend(self.act("kill", "AAPlay kill".into()));
                    acts.extend(self.act("stop", "AAPlay stop".into()));
                }
                if let [Unit::Actor(id)] = b.selection.as_slice() {
                    if let Some(a) = w.actor(*id).filter(|a| own(a.kingdom)) {
                        let def = w.data.def(a.def);
                        if !def.recruits.is_empty() {
                            acts.extend(self.act("recruit", format!("AAPlay recruit_page {id}")));
                        }
                        if def.unpacks_to.is_some() {
                            acts.extend(self.act("unpack", format!("AAPlay unpack {id}")));
                        }
                        if def.packs_to.is_some() {
                            acts.extend(self.act("pack", format!("AAPlay pack {id}")));
                        }
                    }
                }
                place(acts)
            }
        }
    }
}
