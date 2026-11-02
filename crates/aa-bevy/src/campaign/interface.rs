//! The MetaGame interface (KKC_MetaGameInterface, MetaGameInterface.cpp) over
//! the strategic map: the widget tree of CreateChildren 935130 with the
//! control panel (94F6E0), the command buttons (965B10), the player display
//! (94FB60), the message log and the turn status label; its commands (9375C0)
//! and per-frame update (the player display 94FCA0, the command buttons
//! 965A10, the turn status 935760 and the battle block 93694D..936A07); the
//! game menu (94B100) and the Start Battle dialog (94D9D0).
//!
//! Refused with a log line until they are built: the purchasing dialog
//! (94BFE0), the history dialog (94AD40), the politics display, the saved-game
//! dialogs (68DB60 / 68DCE0 type 3) and the RTS battle launch (LaunchRTSGame
//! 9336E0 -> 932210, `aa_metagame::launch`): a battle ends only by Quick
//! Battle or Cancel until then. Not built at all: the help popups (94A5C0),
//! the minimap (MetaGameMiniMap), the object display's portrait and division
//! list, the start dialog's army lists (94DFE0) and tooltips (94E610, 94E920),
//! the upgrade buttons (966650 hides them here), the drag move (957130), the
//! double click (957540), CycleSelection (933DB0), the computer turn's camera
//! follow (933320), the territory game events (0x3A / 0x3B of 953A20) and the
//! end of the campaign (938E10 -> 50D370, only logged).
use super::Game;
use aa_metagame::battle::Side;
use aa_metagame::commands::Command;
use aa_metagame::driver::{Campaign, Note};
use aa_metagame::mgai::controller::OrderEffect;
use aa_metagame::resolve::EngineCall;
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};

const IFACE_FILE: &str = "MetaGame/UI/metagame_interface.tgi";
const IFACE: &str = "MetaGame";
pub const CP: &str = "MetaGame/MetaGameControlPanel";
const BUTTONS: &str = "MetaGame/MetaGameControlPanel/MetaGameCommandButtons";
const PLAYER: &str = "MetaGame/MetaGamePlayerDisplay";
const LOG: &str = "Root/MetaGame/MetaGameMessageLog";
const STATUS: &str = "MetaGame/MetaGameTurnStatusLabel";
const OBJECT: &str = "MetaGame/MetaGameControlPanel/MetaGameObjectDisplay";

/// The dialogs of the interface (+0xF0..+0x100).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog {
    Menu,
    StartBattle,
}

/// What an interface command asks the front end for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    None,
    /// RequestFinished: leave the campaign.
    Finished,
    QuitConfirm,
    Options,
}

#[derive(Default)]
pub struct MetaGameUi {
    pub dialog: Option<Dialog>,
    /// The command waiting for ImportantMGCommandAccept (+0x134).
    important: Option<Command>,
    /// The kingdom the camera last centred on (CenterOnCapital at a turn change).
    pub centered_for: Option<usize>,
    /// A LookAt (6FF200) for the map camera, native ground point.
    pub look_at: Option<[f32; 2]>,
    /// The last texts written (labels are rewritten when they change).
    last: Vec<(String, String)>,
    /// `MGState+0x44` at the last update (957710 flag 0x80).
    battle_started: bool,
}

/// The kingdom's name (kingdom data +8): the labels.
fn kingdom_name(ui: &Ui, game: &Game, k: usize) -> String {
    let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
    let kd = game.campaign.state.kingdoms[k].data;
    ui.strings.resolve(&c.kingdoms[kd].name)
}

/// `0x95A8D0`, the kingdom in the messages: with a kingdom actor the glyph
/// in the kingdom colour (4E93C0, `<color=%g,%g,%g>%c`) and " <rc>" before
/// the name.
fn message_kingdom(ui: &Ui, game: &Game, k: usize) -> String {
    let name = kingdom_name(ui, game, k);
    if game.campaign.state.kingdoms[k].actor.is_none() {
        return name;
    }
    let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
    let color = game.data.color(&c.kingdoms[game.campaign.state.kingdoms[k].data].color_ids);
    let glyph = ui.avars.get("KingdomGlyphInfo").map(aa_ui::node::unquote).and_then(|g| ui.strings.glyphs.get(&g.to_ascii_lowercase()).copied());
    let g = |v: f32| aa_ui::staging::fmt_g(f64::from(v));
    match glyph {
        Some(glyph) => format!("<color={},{},{}>{glyph} <rc>{name}", g(color[0]), g(color[1]), g(color[2])),
        None => name,
    }
}

/// An army type's display name (actor virtual +0x24: the Thing's `name`).
fn army_name(ui: &Ui, game: &Game, definition: usize) -> String {
    let ids = &game.campaign.data.armies[definition].ids;
    match game.data.thing_name(ids) {
        Some(key) => ui.strings.resolve(key),
        None => ids.clone(),
    }
}

fn territory_name(ui: &Ui, game: &Game, t: usize) -> String {
    let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
    let map = c.map.unwrap_or(0);
    let d = game.campaign.state.territories[t].definition;
    ui.strings.resolve(&game.campaign.data.maps[map].territories[d].name)
}

/// `0x454DC0`: 0 green (own), 1 blue (ally), 2 grey, 3 red (enemy).
fn relation_table(index: u32) -> [f32; 4] {
    match index {
        0 => [0.0, 1.0, 0.0, 1.0],
        1 => [0.0, 0.0, 1.0, 1.0],
        2 => [0.8, 0.8, 0.8, 1.0],
        _ => [1.0, 0.0, 0.0, 1.0],
    }
}

/// The colour of a kingdom for the local player (94FE40..: grey without a
/// local player, red when the kingdom's virtual +0x140 says enemy, else own
/// or ally).
fn relation_color(game: &Game, k: Option<usize>) -> [f32; 4] {
    let local = game.campaign.local_actor();
    let other = k.and_then(|k| game.campaign.actor(k));
    relation_table(match (local, other) {
        (Some(l), Some(o)) => {
            use aa_metagame::battle::KingdomRelations;
            if game.campaign.relations.enemy(o, l) {
                3
            } else {
                u32::from(o != l)
            }
        }
        _ => 2,
    })
}

fn set_label_color(ui: &mut Ui, path: &str, color: [f32; 4]) {
    ui.with_widget(path, |w| {
        for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
            if let aa_ui::ve::VeKind::Label(l) = &mut ve.kind {
                l.color = color;
            }
        }
    });
}

/// `metagame_round_display_format` ("%b %y") of a round: month index
/// (initial month + round - 1) mod 12, year initial year + months / 12
/// (94FCA0 -> 6FC0A0).
pub fn round_text(ui: &Ui, game: &Game) -> String {
    let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
    let months = c.initial_month_index + game.campaign.state.round - 1;
    let (month, year) = (months.rem_euclid(12), c.initial_year + months.div_euclid(12));
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let format = ui.strings.resolve("#metagame_round_display_format");
    format.replace("%b", MONTHS[month as usize]).replace("%y", &format!("{:02}", year.rem_euclid(100)))
}

/// The income of the next round (959F90: the production of the owned
/// territories).
fn income(game: &Game, k: usize) -> i32 {
    let s = &game.campaign.state;
    let map = game.campaign.data.campaigns[s.campaign].map.unwrap_or(0);
    let defs = &game.campaign.data.maps[map].territories;
    s.kingdoms[k].territories.iter().fold(0i32, |a, &t| a.wrapping_add(defs[s.territories[t].definition].production))
}

impl MetaGameUi {
    fn tree() -> Widget {
        let mut iface = Widget::new(C::Interface, &format!("{IFACE_FILE}:{IFACE}"), false);
        // 94F6E0: the control panel's children
        let mut cp = Widget::new(C::Panel, "MetaGame/UI/metagame_cp.tgi:MetaGameControlPanel", true);
        cp.add_child(Widget::push_button("MetaGameButtonMenu", false, "OpenMetaGameMenuDialog"));
        cp.add_child(Widget::push_button("MetaGameButtonSpeedUp", false, "SpeedUp"));
        cp.add_child(Widget::push_button("MetaGameButtonSpeedDown", false, "SpeedDown"));
        cp.add_child(Widget::toggle_button("MetaGameButtonPause", false, "GamePause"));
        let mut object = Widget::new(C::Panel, "MetaGame/UI/metagame_object_display.tgi:MetaGameObjectDisplay", false);
        for display in ["ActorDisplay", "TerritoryDisplay"] {
            let mut d = Widget::new(C::Panel, display, false);
            d.add_child(Widget::new(C::Label, "Name", false));
            d.add_child(Widget::new(C::Label, "Owner", false));
            if display == "ActorDisplay" {
                d.add_child(Widget::new(C::Label, "MovesRemaining", false));
            } else {
                d.add_child(Widget::new(C::Label, "Money", false));
            }
            let mut k = Widget::new(C::Decal, "KingdomDecal", false);
            k.set_visible(true);
            d.add_child(k);
            d.set_visible(false);
            object.add_child(d);
        }
        cp.add_child(object);
        // 965B10: the command buttons
        let mut buttons = Widget::new(C::Panel, "MetaGame/UI/metagame_command_buttons.tgi:MetaGameCommandButtons", false);
        buttons.add_child(Widget::push_button("MetaGameButtonAttack", false, "MGCommand attack"));
        buttons.add_child(Widget::push_button("MetaGameButtonMove", false, "MGCommand move"));
        buttons.add_child(Widget::push_button("MetaGameButtonUpgradeAll", false, "MGCommand upgradeactorall"));
        for i in 0..4 {
            buttons.add_child(Widget::push_button(&format!("MetaGameButtonUpgrade{i}"), false, "MGCommand upgradeactor"));
        }
        buttons.add_child(Widget::push_button("MetaGameButtonHistory", false, "History"));
        buttons.add_child(Widget::push_button("MetaGameButtonEndTurn", false, "MGCommand endturn"));
        buttons.add_child(Widget::push_button("MetaGameButtonStartBattle", false, "MGCommand startbattle"));
        buttons.add_child(Widget::push_button("MetaGameButtonPurchase", false, "Purchase"));
        buttons.add_child(Widget::toggle_button("MetaGameButtonPolitics", false, "PoliticsDisplayToggle"));
        cp.add_child(buttons);
        iface.add_child(cp);
        // 94FB60: the player display
        let mut player = Widget::new(C::Panel, "MetaGame/UI/metagame_player_display.tgi:MetaGamePlayerDisplay", true);
        for l in ["MetaGameRoundLabel", "MetaGameTurnLabel", "MetaGameResources"] {
            player.add_child(Widget::new(C::Label, l, false));
        }
        iface.add_child(player);
        // 5FD6D0(name, 0, 0xCA MetaGameUIMessageLogMaximumLines, fades, no auto height)
        iface.add_child(aa_ui::message_log::message_log(
            "/MetaGame/UI/metagame_message_log.tgi:MetaGameMessageLog",
            "MetaGameUIMessageLogMaximumLines",
            true,
            false,
        ));
        iface.add_child(Widget::new(C::Label, "/MetaGameTurnStatusLabel", false));
        iface
    }

    /// Build the interface (CreateChildren 935130) and the open dialog.
    pub fn show(&mut self, ui: &mut Ui, game: &mut Game) {
        ui.clear();
        let mut iface = Self::tree();
        ui.load_widget(&mut iface, None);
        ui.connect(iface);
        self.last.clear();
        if let Some(d) = self.dialog {
            self.open(ui, game, d);
        }
        self.update(ui, game);
        ui.layout();
    }

    fn set_text(&mut self, ui: &mut Ui, path: &str, text: &str) {
        if self.last.iter().any(|(p, t)| p == path && t == text) {
            return;
        }
        self.last.retain(|(p, _)| p != path);
        self.last.push((path.to_string(), text.to_string()));
        ui.set_label(path, text);
    }

    /// The per-frame update.
    pub fn update(&mut self, ui: &mut Ui, game: &mut Game) {
        // the UVars are read when they are used (`[B39940]+0x338`, `+0x334`)
        game.campaign.options.computer_quick_battle = ui.uvars.get_bool("MetaGameUIComputerQuickBattle");
        game.campaign.options.fast_mgai = ui.uvars.get_bool("MetaGameUIFastMGAI");
        let s = &game.campaign.state;
        let Some(cur) = s.current else { return };
        let local_turn = game.campaign.local_turn();
        // 94FCA0: round, moving player, resources(+income)
        let round = round_text(ui, game);
        self.set_text(ui, &format!("{PLAYER}/MetaGameRoundLabel"), &round);
        let name = kingdom_name(ui, game, cur);
        self.set_text(ui, &format!("{PLAYER}/MetaGameTurnLabel"), &name);
        set_label_color(ui, &format!("{PLAYER}/MetaGameTurnLabel"), relation_color(game, Some(cur)));
        if let Some(l) = game.campaign.local {
            let money = game.campaign.state.kingdoms[l].resources;
            let text = money_glyph(ui).map(|g| resources_text(g, money, income(game, l))).unwrap_or_default();
            self.set_text(ui, &format!("{PLAYER}/MetaGameResources"), &text);
        }
        // 935760: the turn status label
        if local_turn {
            ui.with_widget(STATUS, |w| w.set_visible(false));
        } else {
            let text = ui.strings.format("metagame_turn_status_label|s", &[&name]);
            self.set_text(ui, STATUS, &text);
            ui.with_widget(STATUS, |w| w.set_visible(true));
        }
        // 965A10: purchase / end turn / start battle
        let pending = game.campaign.pending_battle().ok().flatten().is_some();
        let play = game.campaign.state.battle_play_requested;
        ui.with_widget(&format!("{BUTTONS}/MetaGameButtonEndTurn"), |w| w.set_visible(!pending && local_turn));
        ui.with_widget(&format!("{BUTTONS}/MetaGameButtonStartBattle"), |w| w.set_visible(!play && pending && local_turn));
        ui.with_widget(&format!("{BUTTONS}/MetaGameButtonPurchase"), |w| w.set_visible(local_turn));
        // 966650: the army buttons show for an own army of the local player
        let own_army = game.selected_army.filter(|&a| {
            game.campaign.state.armies.get(a.0 as usize).is_some_and(|x| x.as_ref().is_some_and(|x| Some(x.owner) == game.campaign.local_actor()))
        });
        for b in ["MetaGameButtonAttack", "MetaGameButtonMove"] {
            ui.with_widget(&format!("{BUTTONS}/{b}"), |w| w.set_visible(own_army.is_some() && local_turn));
        }
        for b in ["MetaGameButtonUpgradeAll", "MetaGameButtonUpgrade0", "MetaGameButtonUpgrade1", "MetaGameButtonUpgrade2", "MetaGameButtonUpgrade3"] {
            ui.with_widget(&format!("{BUTTONS}/{b}"), |w| w.set_visible(false));
        }
        self.object_display(ui, game);
        // 0x9367AD: a turn change centres on the selection during the local
        // player's turn (9335E0), otherwise on the capital of the kingdom
        // whose turn starts (933680)
        if self.centered_for != Some(cur) {
            self.centered_for = Some(cur);
            self.look_at = local_turn.then(|| selection_point(game)).flatten().or_else(|| game.campaign.state.kingdoms[cur].capital.map(|c| territory_point(game, c)));
        }
        // 0x936840: a change of `+0x44` centres on the pending battle
        if self.battle_started != game.campaign.state.battle_started {
            self.battle_started = game.campaign.state.battle_started;
            if let Some(t) = game.campaign.pending_battle().ok().flatten() {
                self.look_at = Some(territory_point(game, t));
            }
        }
        // the battle block of the interface update (93694D..936A07): a
        // started, unresolved battle opens the Start Battle dialog unless the
        // driver answers it (MetaGameUIComputerQuickBattle, not involved)
        let waiting = game.campaign.battle_waiting().unwrap_or(false);
        let auto = game.campaign.options.computer_quick_battle && !game.campaign.local_involved().unwrap_or(true);
        if waiting && !auto && self.dialog.is_none() {
            self.open(ui, game, Dialog::StartBattle);
        }
        // 94E560: the dialog closes once its conditions are gone
        if self.dialog == Some(Dialog::StartBattle) && !waiting {
            self.close(ui);
        }
        ui.message_log_update(LOG);
    }

    fn object_display(&mut self, ui: &mut Ui, game: &Game) {
        let army = game.selected_army.and_then(|a| game.campaign.state.armies.get(a.0 as usize).and_then(|x| x.as_ref()).map(|x| (a, x.clone())));
        let actor = format!("{OBJECT}/ActorDisplay");
        let territory = format!("{OBJECT}/TerritoryDisplay");
        match (army, game.selected_territory) {
            (Some((_, a)), _) => {
                ui.with_widget(&actor, |w| w.set_visible(true));
                ui.with_widget(&territory, |w| w.set_visible(false));
                let thing_name = army_name(ui, game, a.definition);
                self.set_text(ui, &format!("{actor}/Name"), &thing_name);
                let k = game.campaign.kingdom_of_actor(a.owner);
                let owner = k.map(|k| kingdom_name(ui, game, k)).unwrap_or_default();
                self.set_text(ui, &format!("{actor}/Owner"), &owner);
                set_label_color(ui, &format!("{actor}/Owner"), relation_color(game, k));
                set_kingdom_decal(ui, game, &format!("{actor}/KingdomDecal"), k);
            }
            (None, Some(t)) => {
                ui.with_widget(&actor, |w| w.set_visible(false));
                ui.with_widget(&territory, |w| w.set_visible(true));
                let name = territory_name(ui, game, t);
                self.set_text(ui, &format!("{territory}/Name"), &name);
                let owner = game.campaign.state.territories[t].owner;
                let text = match owner {
                    Some(k) => kingdom_name(ui, game, k),
                    None => ui.strings.resolve("#metagame_object_display_neutral"),
                };
                self.set_text(ui, &format!("{territory}/Owner"), &text);
                // 96B6DC: green by default, with a local player and an owner
                // red for hostile (954B30), green for its own, blue for an ally
                let color = match (game.campaign.local_actor(), owner.and_then(|k| game.campaign.actor(k))) {
                    (Some(l), Some(o)) => {
                        use aa_metagame::battle::KingdomRelations;
                        relation_table(if game.campaign.relations.enemy(o, l) {
                            3
                        } else {
                            u32::from(o != l)
                        })
                    }
                    _ => relation_table(0),
                };
                set_label_color(ui, &format!("{territory}/Owner"), color);
                set_kingdom_decal(ui, game, &format!("{territory}/KingdomDecal"), owner);
                let map = game.campaign.data.campaigns[game.campaign.state.campaign].map.unwrap_or(0);
                let p = game.campaign.data.maps[map].territories[game.campaign.state.territories[t].definition].production;
                let text = money_glyph(ui).map(|g| income_text(g, p)).unwrap_or_default();
                self.set_text(ui, &format!("{territory}/Money"), &text);
            }
            (None, None) => {
                ui.with_widget(&actor, |w| w.set_visible(false));
                ui.with_widget(&territory, |w| w.set_visible(false));
            }
        }
    }

    /// Show a dialog (ShowDialog 5C0FB0).
    fn open(&mut self, ui: &mut Ui, game: &Game, d: Dialog) {
        self.close(ui);
        self.dialog = Some(d);
        match d {
            // 94B100
            Dialog::Menu => {
                let mut w = Widget::new(C::TopLevel, "MetaGame/UI/metagame_menu.tgi:MetaGameMenuDialog", true);
                for (n, c) in [
                    ("MetaGameButtonOptions", "OptionsDialog"),
                    ("MetaGameButtonLoad", "MetaGameLoadDialog"),
                    ("MetaGameButtonSave", "MetaGameSaveDialog"),
                    ("MetaGameButtonReturnToGame", "ReturnToGame"),
                    ("MetaGameButtonFinished", "RequestFinishedConfirm"),
                    ("MetaGameButtonQuit", "QuitConfirm"),
                ] {
                    w.add_child(Widget::push_button(n, false, c));
                }
                ui.load_widget(&mut w, None);
                ui.connect(w);
            }
            // 94D9D0 + the army lists (94DFE0)
            Dialog::StartBattle => {
                let mut w = Widget::new(C::TopLevel, "MetaGame/UI/metagame_start_battle.tgi:MetaGameStartBattleDialog", true);
                w.add_child(Widget::push_button("MetaGameButtonStartBattlePlay", false, "LaunchRTSGame"));
                w.add_child(Widget::push_button("MetaGameButtonStartBattleWatch", false, "LaunchRTSGame"));
                w.add_child(Widget::push_button("MetaGameButtonStartBattleQuickBattle", false, "QuickBattle"));
                w.add_child(Widget::push_button("MetaGameButtonStartBattleCancel", false, "Cancel"));
                w.add_child(Widget::new(C::Label, "MetaGameContestedTerritoryLabel", false));
                for (panel, label) in [("MetaGameAttackerPanel", "AttackerLabel"), ("MetaGameDefenderPanel", "DefenderLabel")] {
                    let mut p = Widget::new(C::Panel, panel, false);
                    p.add_child(Widget::new(C::Label, label, false));
                    w.add_child(p);
                }
                ui.load_widget(&mut w, None);
                ui.connect(w);
                self.fill_battle(ui, game);
            }
        }
        ui.layout();
    }

    /// `0x94F2F0`, the Start Battle dialog's Activate: the contested
    /// territory, the first attacker's kingdom (955420) and the defending
    /// kingdom (955EB0) by name, Play for a battle the local player is part of
    /// (94D620) and Watch otherwise, Cancel on the local player's turn (94D5D0).
    /// The army lists (94DFE0) and the odds tooltips are not built.
    fn fill_battle(&mut self, ui: &mut Ui, game: &Game) {
        let Some(t) = game.campaign.pending_battle().ok().flatten() else { return };
        let base = "MetaGameStartBattleDialog";
        let tname = territory_name(ui, game, t);
        ui.set_label(&format!("{base}/MetaGameContestedTerritoryLabel"), &tname);
        let attacker = game.campaign.side_armies(t, Side::Attacker).first().and_then(|&a| game.campaign.kingdom_of_actor(game.campaign.state.army(a).owner));
        let defender = game.campaign.defending_kingdom(t).ok().flatten();
        let a = attacker.map(|k| kingdom_name(ui, game, k)).unwrap_or_default();
        let d = defender.map(|k| kingdom_name(ui, game, k)).unwrap_or_default();
        ui.set_label(&format!("{base}/MetaGameAttackerPanel/AttackerLabel"), &a);
        ui.set_label(&format!("{base}/MetaGameDefenderPanel/DefenderLabel"), &d);
        let involved = game.campaign.local_involved().unwrap_or(false);
        let local_turn = game.campaign.local_turn();
        ui.with_widget(&format!("{base}/MetaGameButtonStartBattlePlay"), |w| w.set_visible(involved));
        ui.with_widget(&format!("{base}/MetaGameButtonStartBattleWatch"), |w| w.set_visible(!involved));
        ui.with_widget(&format!("{base}/MetaGameButtonStartBattleCancel"), |w| w.set_visible(local_turn));
    }

    fn close(&mut self, ui: &mut Ui) {
        if let Some(d) = self.dialog.take() {
            let name = match d {
                Dialog::Menu => "MetaGameMenuDialog",
                Dialog::StartBattle => "MetaGameStartBattleDialog",
            };
            ui.disconnect(name);
            ui.layout();
        }
    }

    /// `0x935A70`, a metagame command of the interface:
    /// - flags 0x1000 / 0x10000 without an actor definition (bit 1) open the
    ///   purchasing dialog (not built, refused);
    /// - flags 0x100000 without a division list (bit 5) do nothing;
    /// - flags 0xF00 need the selected army (bit 0) with the virtuals +0x134
    ///   (0x800), +0x138 (0x200) and +0x140 (0x400) true for the local player;
    /// - flags 0x70 without a territory (bit 2) wait for the territory: the
    ///   hot mode of +0xE4 (956B30);
    /// - the important ones (951260: flags bit 0, end turn; or a startbattle on
    ///   a pending territory with no defenders while the current kingdom has not
    ///   fought yet) ask "Are you sure?" with UVar 0x46 UIMessageBoxConfirmation
    ///   (a command already asking is replaced);
    /// - the rest is validated for the local player (951D70) and sent, or its
    ///   error is shown (952EC0).
    fn dispatch(&mut self, ui: &mut Ui, game: &mut Game, cmd: Command) {
        let flags = cmd.data.map(|d| d.flags).unwrap_or(0);
        let index = cmd.data.map(|d| d.index).unwrap_or(u32::MAX);
        if flags & (0x1000 | 0x1_0000) != 0 && cmd.actor_definition.is_none() {
            bevy::log::warn!("MetaGame command {index} refused: the purchasing dialog (94BFE0) is not built");
            return;
        }
        if flags & 0x10_0000 != 0 && cmd.divisions.is_none() {
            return;
        }
        if flags & 0xf00 != 0 {
            let Some(owner) = cmd.army.and_then(|a| game.campaign.state.armies.get(a.0 as usize)).and_then(|a| a.as_ref()).map(|a| a.owner) else { return };
            let Some(local) = game.campaign.local_actor() else { return };
            use aa_metagame::battle::KingdomRelations;
            let relations = &game.campaign.relations;
            if (flags & 0x800 != 0 && owner != local) || (flags & 0x200 != 0 && !relations.relation_138(owner, local)) || (flags & 0x400 != 0 && !relations.enemy(owner, local)) {
                return;
            }
        }
        if flags & 0x70 != 0 && cmd.territory.is_none() {
            game.targeting = Some(cmd);
            return;
        }
        let undefended_start = index == 2
            && game.campaign.pending_battle().ok().flatten().is_some_and(|t| {
                let tt = &game.campaign.state.territories[t];
                tt.armies.len() as i32 == tt.enemy_army_count
            })
            && game.campaign.state.current.is_some_and(|c| game.campaign.state.kingdoms[c].battles < 1);
        if (flags & 1 != 0 || undefended_start) && ui.uvars.get_bool("UIMessageBoxConfirmation") {
            self.important = Some(cmd);
            let text = ui.strings.resolve("#important_command_confirm");
            let ok = ui.strings.resolve("#button_ok");
            let cancel = ui.strings.resolve("#button_cancel");
            ui.show_message_box(
                aa_ui::message_box::GAME,
                aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, "ImportantMGCommandAccept".into()), (cancel, "ImportantMGCommandCancel".into())] },
            );
            return;
        }
        self.send_now(ui, game, cmd);
    }

    fn send_now(&mut self, ui: &mut Ui, game: &mut Game, cmd: Command) {
        match game.campaign.validate_local(&cmd) {
            Ok(0) => {
                let _ = game.campaign.send_local(cmd);
            }
            Ok(code) => error_message(ui, game, code, &cmd),
            Err(e) => bevy::log::warn!("command {cmd:?}: {e}"),
        }
    }

    /// The interface's commands (9375C0) and the dialogs' (94B100, 94D780).
    pub fn command(&mut self, ui: &mut Ui, game: &mut Game, command: &str) -> Option<Out> {
        let mut words = command.split_whitespace();
        let head = words.next()?.to_ascii_lowercase();
        match head.as_str() {
            // 936F40
            "mgcommand" => {
                let name = words.next()?;
                // 936CF0 lookup; "No such metagame command" otherwise
                let Some(mut cmd) = Campaign::command(name) else {
                    bevy::log::warn!("No such metagame command: {name}");
                    return Some(Out::None);
                };
                if words.next().is_some() {
                    bevy::log::warn!("MGCommand {name} refused: its arguments (9498E0 ..) are not ported");
                    return Some(Out::None);
                }
                // hot commands (flags & 0x0F000000) only on the local turn
                let flags = cmd.data.map(|d| d.flags).unwrap_or(0);
                if flags & 0x0f00_0000 != 0 && !game.campaign.local_turn() {
                    return Some(Out::None);
                }
                // 950EB0: the first selected thing
                cmd.army = game.selected_army;
                self.dispatch(ui, game, cmd);
                Some(Out::None)
            }
            "importantmgcommandaccept" => {
                if let Some(cmd) = self.important.take() {
                    self.send_now(ui, game, cmd);
                }
                Some(Out::None)
            }
            "importantmgcommandcancel" => {
                self.important = None;
                Some(Out::None)
            }
            "openmetagamemenudialog" => {
                if self.dialog == Some(Dialog::Menu) {
                    self.close(ui);
                } else {
                    self.open(ui, game, Dialog::Menu);
                }
                Some(Out::None)
            }
            "returntogame" => {
                self.close(ui);
                Some(Out::None)
            }
            "optionsdialog" => Some(Out::Options),
            "quitconfirm" => Some(Out::QuitConfirm),
            // 9359B0: "metagame_end_game_confirm" -> RequestFinished
            "requestfinishedconfirm" => {
                let text = ui.strings.resolve("#metagame_end_game_confirm");
                let ok = ui.strings.resolve("#button_ok");
                let cancel = ui.strings.resolve("#button_cancel");
                ui.show_message_box(aa_ui::message_box::GAME, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, "RequestFinished".into()), (cancel, String::new())] });
                Some(Out::None)
            }
            "requestfinished" => {
                self.close(ui);
                Some(Out::Finished)
            }
            // 932A60: the Start Battle dialog
            "startbattle" => {
                if game.campaign.battle_waiting().unwrap_or(false) {
                    self.open(ui, game, Dialog::StartBattle);
                }
                Some(Out::None)
            }
            // 94D4F0 / 94D440: the local player's quickbattle / cancelbattle orders
            "quickbattle" => {
                self.close(ui);
                if let Some(cmd) = Campaign::command("quickbattle") {
                    self.send_now(ui, game, cmd);
                }
                Some(Out::None)
            }
            "cancel" => {
                self.close(ui);
                if let Some(cmd) = Campaign::command("cancelbattle") {
                    self.send_now(ui, game, cmd);
                }
                Some(Out::None)
            }
            // 9336E0: the playbattle order, then the RTS launch 932210. Refused
            // until the launch (948EF0) is wired to the battle client; the
            // dialog stays open with Quick Battle and Cancel.
            "launchrtsgame" => {
                bevy::log::warn!("LaunchRTSGame refused: the RTS battle launch (9336E0 -> 932210) is not wired to the battle client");
                Some(Out::None)
            }
            // 933F20 (938BC0: the kingdom by its campaign IDS) -> 933680
            "centeroncapital" => {
                let ids = words.next()?;
                let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
                let capital = game.campaign.state.kingdoms.iter().find(|k| c.kingdoms[k.data].ids.eq_ignore_ascii_case(ids)).and_then(|k| k.capital);
                if let Some(t) = capital {
                    self.look_at = Some(territory_point(game, t));
                }
                Some(Out::None)
            }
            // 934290 -> 9335E0
            "centeronselection" => {
                if let Some(p) = selection_point(game) {
                    self.look_at = Some(p);
                }
                Some(Out::None)
            }
            // the Game interface's pause and speed commands (60C680 / 60C730:
            // half a step, as crate::input)
            "gamepause" => {
                game.paused = !game.paused;
                let paused = game.paused;
                ui.with_widget(&format!("{CP}/MetaGameButtonPause"), |w| {
                    if paused {
                        w.flags |= aa_ui::state::widget_flag::SELECTED;
                    } else {
                        w.flags &= !aa_ui::state::widget_flag::SELECTED;
                    }
                });
                Some(Out::None)
            }
            "speedup" => {
                game.speed = speed_step(game.speed, 1.0);
                Some(Out::None)
            }
            "speeddown" => {
                game.speed = speed_step(game.speed, -1.0);
                Some(Out::None)
            }
            // 933550: on the local player's turn; refused until it is built
            "purchase" => {
                if game.campaign.local_turn() {
                    bevy::log::warn!("Purchase refused: the purchasing dialog (94BFE0) is not built");
                }
                Some(Out::None)
            }
            "history" | "politicsdisplaytoggle" | "metagameloaddialog" | "metagamesavedialog" | "gamesave" | "cycleselection" => {
                bevy::log::warn!("MetaGame command {command} refused: its screen is not built");
                Some(Out::None)
            }
            _ => None,
        }
    }

    /// `0x956E40`: the left click of the hot mode on a territory: the
    /// waiting command gets it, validated for the local player and sent.
    pub fn hot_territory(&mut self, ui: &mut Ui, game: &mut Game, mut cmd: Command, t: usize) {
        let map = game.campaign.data.campaigns[game.campaign.state.campaign].map.unwrap_or(0);
        let definition = game.campaign.state.territories[t].definition;
        cmd.set_territory(&game.campaign.data, map, definition);
        self.send_now(ui, game, cmd);
    }

    /// `0x956C20`: the right click on a territory with an army selected:
    /// the command 9521A0 chooses for the filled fields, validated and sent;
    /// when there is none, an army the local player does not control reports
    /// error 5.
    pub fn right_click(&mut self, ui: &mut Ui, game: &mut Game, t: usize) {
        let Some(army) = game.selected_army else { return };
        let Some(local) = game.campaign.local_actor() else { return };
        let Some(owner) = game.campaign.state.armies.get(army.0 as usize).and_then(|a| a.as_ref()).map(|a| a.owner) else { return };
        let map = game.campaign.data.campaigns[game.campaign.state.campaign].map.unwrap_or(0);
        let definition = game.campaign.state.territories[t].definition;
        let mut cmd = Command::default();
        cmd.set_territory(&game.campaign.data, map, definition);
        cmd.army = Some(army);
        match game.campaign.hot_command_for(&cmd, local) {
            Some(data) => {
                cmd.data = Some(data);
                self.send_now(ui, game, cmd);
            }
            None if owner != local => error_message(ui, game, aa_metagame::commands::code::ARMY_NOT_OWNED, &cmd),
            None => {}
        }
    }

    /// The messages of what happened (953A20 battle texts, 9537B0 moves,
    /// 9528B0 / 952A00 / 952B90 purchases, 952790 end turn).
    pub fn notes(&mut self, ui: &mut Ui, game: &mut Game, notes: &[Note]) {
        let uv = ui.uvars.clone();
        for n in notes {
            match n {
                // 953A20: the attacker's name in every text, the defender's in
                // the captured capital's
                Note::Engine(EngineCall::BattleMessages { attacker, defender, territory, attacker_won }) => {
                    let a = message_kingdom(ui, game, *attacker);
                    let t = territory_name(ui, game, *territory);
                    let text = match defender {
                        None => {
                            let key = if *attacker_won { "metagame_territory_captured_neutral|ss" } else { "metagame_territory_defended_neutral|ss" };
                            ui.strings.format(key, &[&a, &t])
                        }
                        Some(d) => {
                            let capital = game.campaign.state.kingdoms[*d].capital == Some(*territory);
                            match (capital, *attacker_won) {
                                (true, true) => {
                                    let dn = message_kingdom(ui, game, *d);
                                    ui.strings.format("metagame_capital_captured_enemy|ss", &[&a, &dn])
                                }
                                (true, false) => ui.strings.format("metagame_capital_defended_enemy|ss", &[&a, &t]),
                                (false, true) => ui.strings.format("metagame_territory_captured_enemy|ss", &[&a, &t]),
                                (false, false) => ui.strings.format("metagame_territory_defended_enemy|ss", &[&a, &t]),
                            }
                        }
                    };
                    ui.messages.send(&uv, "METAGAME_BATTLE", &text, None);
                }
                // 952790
                Note::Engine(EngineCall::EndTurnMessage { kingdom }) => {
                    let name = message_kingdom(ui, game, *kingdom);
                    let text = ui.strings.format("metagame_end_turn|s", &[&name]);
                    ui.messages.send(&uv, "METAGAME_EVENT", &text, None);
                }
                Note::Order { kingdom: Some(k), command, result, move_key } if result.code == 0 => {
                    let index = command.data.map(|d| d.index).unwrap_or(u32::MAX);
                    let name = message_kingdom(ui, game, *k);
                    match (&result.effect, index) {
                        // 952A00
                        (Some(OrderEffect::Process(out)), 8) if out.created.is_some() => {
                            let army = command.actor_definition.map(|d| army_name(ui, game, d)).unwrap_or_default();
                            let text = ui.strings.format("metagame_purchased_army|ss", &[&name, &army]);
                            ui.messages.send(&uv, "METAGAME_PURCHASE", &text, None);
                        }
                        // 9528B0
                        (Some(OrderEffect::Process(_)), 7) => {
                            let tech = command.technology.clone().unwrap_or_default();
                            let tech = ui.strings.resolve(&format!("#{tech}_name"));
                            let text = ui.strings.format("metagame_purchased_technology|ss", &[&name, &tech]);
                            ui.messages.send(&uv, "METAGAME_PURCHASE", &text, None);
                        }
                        // 9537B0: the key was chosen before the order ran
                        (Some(OrderEffect::Process(out)), 0 | 1) if out.messages.contains(&aa_metagame::commands::category::MOVEMENT) => {
                            let t = command
                                .territory
                                .and_then(|d| game.campaign.state.territories.iter().position(|x| x.definition == d))
                                .map(|t| territory_name(ui, game, t))
                                .unwrap_or_default();
                            if let Some(key) = move_key {
                                let text = ui.strings.format(key, &[&name, &t]);
                                ui.messages.send(&uv, "METAGAME_MOVEMENT", &text, None);
                            }
                        }
                        _ => {}
                    }
                }
                Note::Engine(EngineCall::GameOver) => {
                    bevy::log::info!("the campaign is over (938E10)");
                }
                Note::AiFailed { kingdom, error } => {
                    bevy::log::error!("the computer player of kingdom {kingdom} stopped the campaign: {error}");
                }
                _ => {}
            }
        }
    }
}

/// The display point of an MG territory (9552A0 -> 957DD0, z of the
/// territory visual 2.0 from 955A40).
pub fn territory_point(game: &Game, t: usize) -> [f32; 2] {
    match aa_metagame::positions::territory_position(&game.campaign.state, &game.campaign.data, t, Some(2.0f32.to_bits()), aa_metagame::GAME_PRECISION) {
        Ok(p) => [f32::from_bits(p[0]), f32::from_bits(p[1])],
        Err(_) => [0.0, 0.0],
    }
}

/// `0x937290`: the MetaGameMoneyGlyphInfo glyph (`None`: "Bad metagame money
/// glyph", the texts are empty).
fn money_glyph(ui: &Ui) -> Option<char> {
    ui.avars.get("MetaGameMoneyGlyphInfo").map(aa_ui::node::unquote).and_then(|g| ui.strings.glyphs.get(&g.to_ascii_lowercase()).copied())
}

/// `0x9374F0` -> 5C9830 with a total: `%lc %s(%s)` of the glyph, the money
/// `%.0f` and the income `%+.0f`.
fn resources_text(glyph: char, money: i32, income: i32) -> String {
    format!("{glyph} {money}({income:+})")
}

/// `0x937430` -> 5C9830 without a total: `%lc %s` of the glyph and the value
/// `%+.0f`.
fn income_text(glyph: char, value: i32) -> String {
    format!("{glyph} {value:+}")
}

/// Half a step of the game speed exponent, clamped to SVars GameSpeedMin /
/// GameSpeedMax (-2 / +2), as `crate::input::speed_step` for a battle.
fn speed_step(speed: f32, dir: f32) -> f32 {
    let v = (speed.max(1e-3).log2() * 2.0).round() / 2.0 + 0.5 * dir;
    2f32.powf(v.clamp(-2.0, 2.0))
}

/// `0x9335E0`'s point for the first selected thing: its position.
fn selection_point(game: &Game) -> Option<[f32; 2]> {
    if let Some(a) = game.selected_army {
        return super::map::rest_position(game, a).map(|p| [p.x, p.y]);
    }
    game.selected_territory.map(|t| territory_point(game, t))
}

/// 96B1D0 / the territory display: the kingdom's `icon` on the KingdomDecal,
/// hidden without one.
fn set_kingdom_decal(ui: &mut Ui, game: &Game, path: &str, k: Option<usize>) {
    let campaign = game.campaign.state.campaign;
    let ids = &game.campaign.data.campaigns[campaign].ids;
    let view = game.data.view.campaigns.iter().find(|v| v.ids.eq_ignore_ascii_case(ids));
    let icon = k.and_then(|k| view.and_then(|v| v.kingdom_icons.get(game.campaign.state.kingdoms[k].data))).filter(|i| !i.is_empty());
    let texture = icon.map(|i| format!("/MetaGame/{}", i.trim_start_matches('/')));
    ui.with_widget(path, |w| {
        w.set_visible(texture.is_some());
        for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
            if let aa_ui::ve::VeKind::Decal(d) = &mut ve.kind {
                d.texture = texture.clone();
            }
        }
    });
}

/// 952EC0: the validation error message of a refused command.
fn error_message(ui: &mut Ui, game: &Game, code: u32, cmd: &Command) {
    use aa_metagame::commands::code::*;
    let army = cmd.army.and_then(|a| game.campaign.state.armies.get(a.0 as usize).and_then(|x| x.as_ref())).map(|a| army_name(ui, game, a.definition)).unwrap_or_default();
    let territory = cmd
        .territory
        .and_then(|d| game.campaign.state.territories.iter().position(|t| t.definition == d))
        .map(|t| territory_name(ui, game, t))
        .unwrap_or_default();
    let text = match code {
        ARMY_NOT_OWNED => {
            let Some(local) = game.campaign.local else { return };
            let kingdom = message_kingdom(ui, game, local);
            ui.strings.format("metagame_error_army_not_owned|ss", &[&army, &kingdom])
        }
        ARMY_CANNOT_MOVE => ui.strings.format("metagame_error_army_cannot_move|s", &[&army]),
        ARMY_CANNOT_MOVE_TO => ui.strings.format("metagame_error_army_cannot_move_to|ss", &[&army, &territory]),
        ENEMY_TERRITORY_REQUIRED => ui.strings.format("metagame_error_enemy_territory_required|s", &[&territory]),
        FRIENDLY_TERRITORY_REQUIRED => ui.strings.format("metagame_error_friendly_territory_required|s", &[&territory]),
        EXISTING_PENDING_BATTLE => ui.strings.resolve("#metagame_error_existing_pending_battle"),
        NO_ATTACKS_LEFT => ui.strings.resolve("#metagame_error_no_attacks_left"),
        NO_ATTACKS_LEFT_MUST_PULLOUT => ui.strings.format("metagame_error_no_attacks_left_must_pullout|s", &[&territory]),
        INSUFFICIENT_RESOURCES => ui.strings.resolve("#metagame_error_insufficient_resources"),
        PURCHASE_UNOWNED_CAPITAL => ui.strings.resolve("#metagame_error_purchase_unowned_capital"),
        _ => return,
    };
    let uv = ui.uvars.clone();
    ui.messages.send(&uv, "METAGAME_ERROR", &text, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_texts_are_5c9830() {
        assert_eq!(resources_text('$', 500, 30), "$ 500(+30)");
        assert_eq!(resources_text('$', 500, 0), "$ 500(+0)");
        assert_eq!(resources_text('$', 0, -5), "$ 0(-5)");
        assert_eq!(income_text('$', 12), "$ +12");
    }

    #[test]
    fn relation_colours_are_454dc0() {
        assert_eq!(relation_table(0), [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(relation_table(1), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(relation_table(2), [0.8, 0.8, 0.8, 1.0]);
        assert_eq!(relation_table(3), [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn speed_steps_by_half_an_exponent() {
        assert!((speed_step(1.0, 1.0) - 2f32.sqrt()).abs() < 1e-5);
        assert!((speed_step(1.0, -1.0) - 0.5f32.sqrt()).abs() < 1e-5);
        assert_eq!(speed_step(4.0, 1.0), 4.0);
        assert_eq!(speed_step(0.25, -1.0), 0.25);
    }
}
