//! The MetaGame interface (KKC_MetaGameInterface, MetaGameInterface.cpp) over
//! the strategic map: the widget tree of CreateChildren 935130 with the
//! control panel (94F6E0), the command buttons (965B10), the player display
//! (94FB60), the message log and the turn status label; its commands (9375C0)
//! and per-frame update (the player display 94FCA0, the command buttons
//! 965A10, the turn status 935760 and the battle block 93694D..936A07); the
//! game menu (94B100) and the Start Battle dialog (94D9D0).
//!
//! The politics display (932930, 950290) is [`MetaGameUi::refresh_politics`]. The saved-game
//! dialogs (68DB60 / 68DCE0 type 3) are campaign/saves.rs. The RTS battle launch is
//! campaign/battle.rs; the history dialog (94AD40) is the whole message log;
//! the purchasing dialog (94BFE0) lists the shop rows (94C7B0, 94CAE0).
//! The help popups (94A5C0) are campaign/help.rs. The tooltips of the army rows
//! (94DFE0, 94BC50), of the technology rows (94BE50) and of the quick battle (94E610)
//! are [`MetaGameUi::tooltip`], written as the tooltip builder 638690 does ([`TipBody`]).
//! The object display's portrait (96AD80, 96B1D0 -> 96ABF0: the army's own model in
//! the ModelWidget) is drawn by `ui::portrait`. The Play button's tooltip is 94E920
//! ([`rts_tip`]). Not built at all: CycleSelection (933DB0). The computer turn's camera
//! follow (933320) is campaign/map.rs.
//! The end of the campaign (938E10 -> 50D370) shows the mission box.
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
    /// MetaGamePurchasingDialog (94BFE0).
    Purchase,
    /// MetaGameHistoryDialog (94AD40).
    History,
}

/// What an interface command asks the front end for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    None,
    /// RequestFinished: leave the campaign.
    Finished,
    QuitConfirm,
    Options,
    /// LaunchRTSGame: the playbattle order was sent, the RTS battle follows.
    LaunchBattle,
    /// MetaGameLoadDialog / MetaGameSaveDialog (saves.rs).
    LoadDialog,
    SaveDialog,
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
    /// The purchase waiting for PurchaseConfirm (+0xF4, 94C410).
    purchase: Option<Command>,
    /// The purchase page on show: armies (true) or technologies.
    purchase_armies: bool,
    /// The upgrade buttons on show: (button, division). `None`: the page was just built.
    upgrade_shown: Option<Vec<(usize, usize)>>,
    /// The divisions the object display's list shows.
    divisions_shown: Option<Vec<usize>>,
    /// The help popups (94A5C0).
    pub help: super::help::Help,
    /// 94F2F0 on the way: its help popup (local player involved).
    battle_dialog: Option<bool>,
    /// The dynamic tooltips of the open dialog: (the widget's path, name, text).
    tips: Vec<(String, String, String)>,
    /// The game's folder (the data of the technologies, `crate::preload::game_data`).
    game_root: std::path::PathBuf,
    /// PoliticsDisplayToggle: the display is on, and what it shows.
    politics: bool,
    politics_rows: Vec<String>,
}

/// `+0x120` of the interface, made by 935130 (932930).
const POLITICS: &str = "MetaGame/UI/metagame_politics_display.tgi:MetaGamePoliticsDisplay";
const POLITICS_NAME: &str = "MetaGamePoliticsDisplay";
const TAG_POLITICS: u64 = 9500;

/// The tooltip builder 638690 after its name (6388F0: `ui::tooltip` writes the name and
/// the blank line after it): +0 the text, +4 the break owed to the next value.
#[derive(Default)]
struct TipBody {
    text: String,
    pending: bool,
}

impl TipBody {
    /// 638B70 with its callers' break (63C160, 63D3D0, 63B8C0, 63BFD0): `\n` when a value
    /// comes and a break is owed, `\n%s<SetX=%d>%s` of the label and the value (the first
    /// `\n` only after other text; without a label the value alone).
    fn row(&mut self, label: &str, value: &str, x: i32) {
        if !value.is_empty() && self.pending {
            self.text.push('\n');
            self.pending = false;
        }
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        if !label.is_empty() {
            self.text.push_str(&format!("{label}<SetX={x}>"));
        }
        self.text.push_str(value);
    }

    /// 638D90: after other text a blank line, then the description.
    fn description(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if !self.text.is_empty() {
            self.text.push_str("\n\n");
        }
        self.text.push_str(text);
        self.pending = false;
    }

    /// 63D3D0: `Cost` and `%lc %.0f` of the money's glyph, not for a cost of 0.
    fn cost(&mut self, ui: &Ui, cost: i32, x: i32) {
        if cost != 0 {
            let value = money_glyph(ui).map(|g| format!("{g} {cost}")).unwrap_or_else(|| cost.to_string());
            self.row(&ui.strings.resolve("#ui_tooltip_item_title_cost"), &value, x);
        }
    }
}

/// 638B70's column x: the AVar UITabWidth (index 0x56, `[B39990]+0x158`, registered at
/// 0x403B1E) times 5C1DB0 (the window's height over the reference height), truncated.
fn tab_x(ui: &Ui) -> i32 {
    let tab = ui.avars.get("UITabWidth").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(100.0);
    (tab * ui.screen.height as f32 / ui.reference.height.max(1.0)) as i32
}

/// The tooltip of an army row (94DFE0 without a cost, 94BC50 with one): the Thing's name, the divisions it
/// starts with (63C160), the MetaGameComponent's tooltip (638D90).
fn army_tip(ui: &Ui, game: &Game, d: usize, cost: Option<i32>) -> (String, String) {
    let data = &game.campaign.data;
    let x = tab_x(ui);
    let mut body = TipBody::default();
    if let Some(cost) = cost {
        body.cost(ui, cost, x);
    }
    // 63C160(list, 1, 0): a break before and after; the list is built head first
    // from +0x5B8's array, so the last division comes first
    let list = &data.armies[d].divisions;
    if !list.is_empty() {
        body.pending = true;
    }
    for i in list.iter().rev() {
        if let Some(div) = data.divisions.get(*i) {
            body.row("", &ui.strings.resolve(&div.name), x);
        }
    }
    if !list.is_empty() {
        body.pending = true;
    }
    body.description(&ui.strings.resolve(&data.armies[d].tooltip));
    (army_name(ui, game, d), body.text)
}

/// `0x94E920`: the Play button's tooltip: "RTS Battle", then per side the row builder
/// 63C6B0: its kingdom, Units (the additional actors of its participants, 9488B0, 639FE0),
/// Money (the side's stockpile bonus, 9459F0, on the default stockpile ResourceStockpileDefault
/// `[B39AB0]+0x110`, each stockpiled resource `%lc%.0f` of its floor, 5C4BD0), Capabilities
/// (the technologies of the side's divisions, 945D10), Technologies (the kingdom's, 945C90,
/// 63A0F0: `%d (%s)` of the first three and an ellipsis).
fn rts_tip(ui: &Ui, game: &Game, data: &aa_play::GameData) -> Option<(String, String)> {
    let t = game.campaign.pending_battle().ok().flatten()?;
    let resources: Vec<String> = data.resources.iter().map(|r| r.id.clone()).collect();
    let env = super::battle::env(game, resources);
    let review = aa_metagame::launch::battle_review(&game.campaign.state, &game.campaign.data, &game.campaign.relations, t, &env, aa_numeric::Precision::Double53).map_err(|e| bevy::log::warn!("the battle's tooltip: {e}")).ok()?;
    let x = tab_x(ui);
    let base = data.rule_ci("ResourceStockpileDefault").unwrap_or(0.0);
    let glyph = money_glyph(ui);
    let mut body = TipBody::default();
    for (side, label, kingdom, stock) in [
        (Side::Attacker, "#metagame_tooltip_rtsbattle_attacker", game.campaign.side_armies(t, Side::Attacker).first().and_then(|&a| game.campaign.kingdom_of_actor(game.campaign.state.army(a).owner)), &review.hostile),
        (Side::Defender, "#metagame_tooltip_rtsbattle_defender", game.campaign.defending_kingdom(t).ok().flatten(), &review.friendly),
    ] {
        let Some(k) = kingdom else { continue };
        let name = kingdom_name(ui, game, k);
        body.row(&ui.strings.resolve(label), &name, x);
        let actor_data = game.campaign.state.kingdoms[k].data;
        let units: Vec<String> = review.actors.iter().filter(|a| a.0 == Some(actor_data)).filter_map(|a| data.find(&a.1)).map(|d| ui.strings.resolve(&data.def(d).name)).collect();
        body.row(&ui.strings.resolve("#metagame_tooltip_rtsbattle_units"), &units.join(", "), x);
        let money: String = data
            .resources
            .iter()
            .zip(stock)
            .filter(|(r, _)| r.kind == aa_play::data::ResourceKind::Stockpiled)
            .map(|(_, bits)| format!("{}{:.0}", glyph.map(String::from).unwrap_or_default(), (f32::from_bits(*bits) + base).floor()))
            .collect();
        body.row(&ui.strings.resolve("#metagame_tooltip_rtsbattle_money"), &money, x);
        // 945D10: each technology once, in the order of the side's armies and divisions
        let mut capabilities: Vec<String> = Vec::new();
        for a in game.campaign.side_armies(t, side) {
            for d in &game.campaign.state.army(a).divisions {
                for tech in game.campaign.data.divisions[d.0 as usize].technologies.iter().flatten() {
                    if !capabilities.iter().any(|c| c.eq_ignore_ascii_case(tech)) {
                        capabilities.push(tech.clone());
                    }
                }
            }
        }
        let capabilities: Vec<String> = capabilities.iter().map(|c| technology_name(ui, Some(data), c)).collect();
        body.row(&ui.strings.resolve("#metagame_tooltip_rtsbattle_capabilities"), &capabilities.join(", "), x);
        let techs = &game.campaign.state.kingdoms[k].technologies;
        let techs_text = if techs.is_empty() {
            "0".to_string()
        } else {
            let mut names: Vec<String> = techs.iter().take(3).map(|c| technology_name(ui, Some(data), c)).collect();
            if techs.len() > 3 {
                names.push(ui.strings.resolve("#grammar_ellipsis"));
            }
            format!("{} ({})", techs.len(), names.join(", "))
        };
        body.row(&ui.strings.resolve("#metagame_tooltip_rtsbattle_techs"), &techs_text, x);
        body.pending = true;
    }
    Some((ui.strings.resolve("#metagame_tooltip_rtsbattle_title"), body.text))
}

/// The technology's name (the Property's +0x10), as the rows and the messages write it.
fn technology_name(ui: &Ui, data: Option<&aa_play::GameData>, ids: &str) -> String {
    data.and_then(|d| d.property(ids)).map(|p| ui.strings.resolve(&p.name)).unwrap_or_else(|| ids.to_string())
}

/// `0x94E610`: the Quick Battle button's tooltip: per side (63C2A0) its name, its
/// armies, its kingdom's technologies and its win chance (95F370; the defender's the
/// rest of 1). The row's column is 638B70's ([`tab_x`]).
fn quick_tip(ui: &Ui, game: &Game) -> Option<(String, String)> {
    let t = game.campaign.pending_battle().ok().flatten()?;
    let odds = f32::from_bits(game.campaign.attack_odds_bits(t, aa_numeric::Precision::Double53).ok()?);
    let x = tab_x(ui);
    let row = |label: &str, value: String| format!("{}<SetX={x}>{value}", ui.strings.resolve(label));
    let mut lines = Vec::new();
    for (side, label, chance) in [(Side::Attacker, "#metagame_tooltip_quickbattle_attacker", odds), (Side::Defender, "#metagame_tooltip_quickbattle_defender", 1.0 - odds)] {
        let armies = game.campaign.side_armies(t, side);
        let kingdom = match side {
            Side::Attacker => armies.first().and_then(|&a| game.campaign.kingdom_of_actor(game.campaign.state.army(a).owner)),
            Side::Defender => game.campaign.defending_kingdom(t).ok().flatten(),
        };
        let name = kingdom.map(|k| kingdom_name(ui, game, k)).unwrap_or_default();
        let techs = kingdom.map_or(0, |k| game.campaign.state.kingdoms[k].technologies.len());
        lines.push(row(label, name));
        lines.push(row("#metagame_tooltip_quickbattle_armies", ui.strings.format("metagame_tooltip_quickbattle_armies_format|d", &[&armies.len().to_string()])));
        lines.push(row("#metagame_tooltip_quickbattle_techs", ui.strings.format("metagame_tooltip_quickbattle_techs_format|d", &[&techs.to_string()])));
        lines.push(row("#metagame_tooltip_quickbattle_win_chance", format!("{:.1}%", chance * 100.0)));
    }
    Some((ui.strings.resolve("#metagame_tooltip_quickbattle_title"), lines.join("\n")))
}

/// metagame_object_display.tgi's MGDivisionItem0..5.
const DIVISION_ITEMS: usize = 6;
/// metagame_command_buttons.tgi's MetaGameButtonUpgrade0..3.
const UPGRADE_BUTTONS: usize = 4;

/// `0x965950`: every decal of the view whose `when` a list entry names takes
/// that entry's texture (`5E7B20`).
fn set_ve_textures(ves: &mut [aa_ui::ve::ViewElement], list: &[super::data::ChooseVe]) {
    for ve in ves {
        if let aa_ui::ve::VeKind::Decal(d) = &mut ve.kind
            && let Some(b) = list.iter().find(|b| aa_ui::state::parse_when(&b.when).bits == ve.when)
        {
            d.texture = Some(format!("/{}", b.texture.trim_start_matches('/')));
        }
        set_ve_textures(&mut ve.children, list);
    }
}

/// A decal's texture, also inside a group.
fn set_decal_texture(ve: &mut aa_ui::ve::ViewElement, texture: &Option<String>) {
    if let aa_ui::ve::VeKind::Decal(d) = &mut ve.kind {
        d.texture = texture.clone();
    }
    for c in &mut ve.children {
        set_decal_texture(c, texture);
    }
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
    pub fn with_root(game_root: &std::path::Path) -> Self {
        Self { game_root: game_root.to_path_buf(), ..Default::default() }
    }

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
                // 96B890: the list's MGDivisionItem%d, all hidden until 96BA60
                let mut list = Widget::new(C::Panel, "MGDivisionList", false);
                for i in 0..DIVISION_ITEMS {
                    let mut item = Widget::new(C::Decal, &format!("MGDivisionItem{i}"), false);
                    item.set_visible(false);
                    list.add_child(item);
                }
                d.add_child(list);
            } else {
                d.add_child(Widget::new(C::Label, "Money", false));
            }
            // 96AD80: PortraitButton, its client the Portrait ModelWidget
            let mut pb = Widget::new(C::PushButton, "PortraitButton", false);
            pb.client_name = "Portrait".into();
            pb.add_child(Widget::new(C::Model, "Portrait", false));
            d.add_child(pb);
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
        // 935130: the minimap (campaign/map.rs draws into it)
        iface.add_child(Widget::new(C::MiniMap, "/MetaGame/UI/metagame_minimap.tgi:MetaGameMiniMap", true));
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
        self.upgrade_shown = None;
        self.divisions_shown = None;
        self.politics_rows.clear();
        if let Some(d) = self.dialog {
            self.open(ui, game, d);
        }
        self.update(ui, game);
        self.help.rebuilt(ui);
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
        self.help.update(ui, game);
        if self.politics {
            self.refresh_politics(ui, game);
        }
        if let Some(involved) = self.battle_dialog.take() {
            self.help.ask(ui, game, if involved { super::help::START_BATTLE_SELECTED } else { super::help::WATCH_BATTLE_SELECTED }, None);
        }
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
        self.army_buttons(ui, game);
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
        if self.dialog == Some(Dialog::History) {
            ui.message_log_update("Root/MetaGameHistoryDialog/MetaGameMessageLog");
        }
    }

    /// `0x966650`: the buttons of the selected army. An army of the kingdom
    /// whose turn it is, when that is the local player's: Move while it has moves
    /// (93EB40: the type's `movement` above 0, 93E900: it has not fought), Attack
    /// the same and while the kingdom has fought no battle (+3C); UpgradeAll stays
    /// hidden; an upgradable type (+5B8 +1D) shows the upgrade button at
    /// `upgrade_button_location` of each division the kingdom's shop sells (961A20),
    /// the army lacks (93E9E0) and whose prerequisites it has (93EA70):
    /// `MGCommand upgradeactor <ids>` with the division's UpgradeButtonVE textures
    /// (965950) and the tooltip of its name, cost and `upgrade_tooltip`.
    fn army_buttons(&mut self, ui: &mut Ui, game: &Game) {
        let path = |b: &str| format!("{BUTTONS}/{b}");
        let local = game.campaign.local_actor();
        let cur = game.campaign.state.current.and_then(|k| game.campaign.actor(k));
        let army = game.selected_army.and_then(|a| game.campaign.state.armies.get(a.0 as usize).and_then(|x| x.as_ref())).filter(|a| local.is_some() && Some(a.owner) == local && cur == local);
        let mut shown: Vec<(usize, usize, i32)> = Vec::new();
        if let (Some(a), Some(l)) = (army, game.campaign.local) {
            let def = &game.campaign.data.armies[a.definition];
            if def.upgradable {
                let kingdom = game.campaign.state.kingdoms[l].data;
                let shop = &game.campaign.data.campaigns[game.campaign.state.campaign].kingdoms[kingdom].shop;
                for (i, d) in game.campaign.data.divisions.iter().enumerate() {
                    let cost = shop.divisions.iter().find(|r| r.ids.as_deref() == Some(d.ids.as_str())).map(|r| r.cost);
                    let has = |x: usize| a.divisions.contains(&aa_metagame::battle::DivisionRef(x as u32));
                    if let Some(cost) = cost
                        && !has(i)
                        && d.prerequisites.iter().all(|p| has(*p))
                    {
                        shown.push((d.upgrade_button_location.max(0) as usize, i, cost));
                    }
                }
            }
        }
        let key: Vec<(usize, usize)> = shown.iter().map(|s| (s.0, s.1)).collect();
        if self.upgrade_shown.as_ref() != Some(&key) {
            self.upgrade_shown = Some(key);
            for i in 0..UPGRADE_BUTTONS {
                ui.with_widget(&path(&format!("MetaGameButtonUpgrade{i}")), |w| w.set_visible(false));
            }
            let glyph = money_glyph(ui);
            for (slot, d, cost) in shown {
                let def = &game.campaign.data.divisions[d];
                let view = game.data.view.divisions.get(&def.ids.to_ascii_lowercase()).cloned().unwrap_or_default();
                let cost_title = ui.strings.resolve("#ui_tooltip_item_title_cost");
                let cost_text = glyph.map(|g| format!("{g} {cost}")).unwrap_or_else(|| cost.to_string());
                let name = ui.strings.resolve(&def.name);
                let body = format!("{cost_title} {cost_text}\n\n{}", ui.strings.resolve(&def.upgrade_tooltip));
                let command = format!("MGCommand upgradeactor {}", def.ids);
                ui.with_widget(&path(&format!("MetaGameButtonUpgrade{slot}")), |w| {
                    w.set_visible(true);
                    w.command = Some(command.clone());
                    w.tooltip_name = name.clone();
                    w.tooltip = body.clone();
                    for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
                        set_ve_textures(std::slice::from_mut(ve), &view.buttons);
                    }
                });
            }
        }
        // 93EB40 > 0 and 93E900 > 0
        let moves = army.is_some_and(|a| game.campaign.data.armies[a.definition].movement > 0 && 1 - a.fought > 0);
        let attack = moves && game.campaign.local.is_some_and(|l| game.campaign.state.kingdoms[l].battles < 1);
        ui.with_widget(&path("MetaGameButtonUpgradeAll"), |w| w.set_visible(false));
        ui.with_widget(&path("MetaGameButtonMove"), |w| w.set_visible(moves));
        ui.with_widget(&path("MetaGameButtonAttack"), |w| w.set_visible(attack));
    }

    /// `0x96BA60`: the army's divisions on the items of their `icon_location`,
    /// with the division's name and tooltip.
    fn division_list(&mut self, ui: &mut Ui, game: &Game, army: &aa_metagame::state::MgArmy) {
        let list: Vec<usize> = army.divisions.iter().map(|d| d.0 as usize).collect();
        if self.divisions_shown.as_ref() == Some(&list) {
            return;
        }
        self.divisions_shown = Some(list.clone());
        let base = format!("{OBJECT}/ActorDisplay/MGDivisionList");
        for i in 0..DIVISION_ITEMS {
            ui.with_widget(&format!("{base}/MGDivisionItem{i}"), |w| w.set_visible(false));
        }
        for d in list {
            let Some(def) = game.campaign.data.divisions.get(d) else { continue };
            let texture = game.data.view.divisions.get(&def.ids.to_ascii_lowercase()).map(|v| format!("/MetaGame/{}", v.icon.trim_start_matches('/')));
            let name = ui.strings.resolve(&def.name);
            let tip = ui.strings.resolve(&def.tooltip);
            ui.with_widget(&format!("{base}/MGDivisionItem{}", def.icon_location), |w| {
                w.set_visible(true);
                w.tooltip_name = name.clone();
                w.tooltip = tip.clone();
                for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
                    set_decal_texture(ve, &texture);
                }
            });
        }
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
                self.division_list(ui, game, &a);
                // 96ABF0: the army's model
                let model = game.data.model(&game.campaign.data.armies[a.definition].ids).map(str::to_string);
                ui.with_widget(&format!("{actor}/PortraitButton/Portrait"), |w| {
                    w.control = model.map(|m| Box::new(aa_ui::controls::Control::Model(m)));
                });
            }
            (None, Some(t)) => {
                self.divisions_shown = None;
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
                self.divisions_shown = None;
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
                for (panel, label, list) in [("MetaGameAttackerPanel", "AttackerLabel", "AttackerArmyList"), ("MetaGameDefenderPanel", "DefenderLabel", "DefenderArmyList")] {
                    let mut p = Widget::new(C::Panel, panel, false);
                    p.add_child(Widget::new(C::Label, label, false));
                    p.add_child(Widget::composite(C::Viewport, list, false));
                    w.add_child(p);
                }
                ui.load_widget(&mut w, None);
                ui.connect(w);
                self.fill_battle(ui, game);
            }
            // 94AD40: the close button and the whole message log (5FD6D0 with
            // UVar 0x43 MessageLogMaximumLines, no fading)
            Dialog::History => {
                let mut w = Widget::new(C::TopLevel, "MetaGame/UI/metagame_history.tgi:MetaGameHistoryDialog", true);
                w.add_child(Widget::push_button("MetaGameButtonHistoryClose", false, "Close"));
                w.add_child(aa_ui::message_log::message_log("MetaGameMessageLog", "MessageLogMaximumLines", false, false));
                ui.load_widget(&mut w, None);
                ui.connect(w);
            }
            // 94BFE0, Init 94CFB0's commands, the pages 94C7B0 / 94CAE0
            Dialog::Purchase => {
                let mut w = Widget::new(C::TopLevel, "MetaGame/UI/metagame_purchasing.tgi:MetaGamePurchasingDialog", true);
                w.add_child(Widget::push_button("MetaGameButtonPurchaseClose", false, "Close"));
                let mut armies = Widget::new(C::Panel, "MetaGamePurchasingArmyPage", false);
                armies.add_child(Widget::composite(C::Viewport, "MetaGamePurchaseArmyList", false));
                w.add_child(armies);
                let mut technologies = Widget::new(C::Panel, "MetaGamePurchasingTechnologyPage", false);
                technologies.add_child(Widget::composite(C::Viewport, "MetaGamePurchaseTechnologyList", false));
                w.add_child(technologies);
                let mut a = Widget::toggle_button("MetaGameButtonPurchaseArmy", false, "PurchaseArmyPage");
                a.radio_group = 1;
                w.add_child(a);
                let mut t = Widget::toggle_button("MetaGameButtonPurchaseTechnology", false, "PurchaseTechnologyPage");
                t.radio_group = 1;
                w.add_child(t);
                let mut tip = Widget::new(C::Panel, "MetaGamePurchasingToolTipPanel", false);
                tip.add_child(Widget::new(C::Label, "Label", false));
                w.add_child(tip);
                ui.load_widget(&mut w, None);
                ui.connect(w);
                self.fill_purchase(ui, game);
                self.purchase_page(ui, true);
            }
        }
        ui.layout();
    }

    /// `0x94C7B0` / `0x94CAE0`: one row per shop entry of the local
    /// kingdom (kingdom data +0xDC): `PurchaseArmy <ids>` / `PurchaseTechnology
    /// <ids>`, the icon on the PushItem's Decal (964F90: the Thing's +0xF0, the
    /// Property's action_button_icon +0x30), the name and the cost (937350). A
    /// technology the kingdom owns (959FF0) has its row disabled. The rows' tooltips
    /// are 94BC50 and 94BE50.
    fn fill_purchase(&mut self, ui: &mut Ui, game: &Game) {
        let Some(l) = game.campaign.local else { return };
        let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
        let shop = c.kingdoms[game.campaign.state.kingdoms[l].data].shop.clone();
        let glyph = money_glyph(ui);
        let base = "MetaGamePurchasingDialog";
        let file = "MetaGame/UI/metagame_purchasing.tgi:MetaGamePurchasingDialog";
        let add = |ui: &mut Ui, page: &str, list: &str, command: String, name: String, cost: i32, tag: u64| {
            let vp = format!("Root/{base}/{page}/{list}");
            let mut row = Widget::new(C::Panel, "Client", false);
            let mut push = Widget::new(C::PushButton, "PushItem", false);
            push.command = Some(command);
            push.add_child(Widget::new(C::Decal, "Decal", false));
            row.add_child(push);
            row.add_child(Widget::new(C::Label, "Column0", false));
            row.add_child(Widget::new(C::Label, "Column1", false));
            if let Ok(n) = ui.store.node(&format!("{file}/{page}/{list}")) {
                let strings = ui.strings.clone();
                let resolve = |v: &str| strings.resolve(v);
                let root = std::sync::Arc::new(n.clone());
                row.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
            }
            row.tag = tag;
            ui.viewport_add(&vp, row);
            if let Some(path) = ui.find_tag(tag) {
                let rel = path.strip_prefix("Root/").unwrap_or(&path).to_string();
                ui.set_label(&format!("{rel}/Column0"), &name);
                let text = glyph.map(|g| format!("{g} {cost}")).unwrap_or_else(|| cost.to_string());
                ui.set_label(&format!("{rel}/Column1"), &text);
            }
        };
        for (i, r) in shop.actors.iter().enumerate() {
            let Some(ids) = r.ids.clone() else { continue };
            let name = game.data.thing_name(&ids).map(|k| ui.strings.resolve(k)).unwrap_or_else(|| ids.clone());
            add(ui, "MetaGamePurchasingArmyPage", "MetaGamePurchaseArmyList", format!("PurchaseArmy {ids}"), name, r.cost, 9600 + i as u64);
            // 94C7B0: the Thing's icon (+0xF0) on the Decal; 94BC50 the tooltip
            if let Some(path) = ui.find_tag(9600 + i as u64) {
                let icon = game.data.thing_icon(&ids).map(|t| format!("/{}", t.trim_start_matches('/')));
                let rel = path.strip_prefix("Root/").unwrap_or(&path).to_string();
                ui.with_widget(&format!("{rel}/PushItem/Decal"), |w| {
                    for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
                        set_decal_texture(ve, &icon);
                    }
                });
                if let Some(d) = game.campaign.data.armies.iter().position(|a| a.ids.eq_ignore_ascii_case(&ids)) {
                    let (name, body) = army_tip(ui, game, d, Some(r.cost));
                    self.tips.push((path, name, body));
                }
            }
        }
        // the Property of a technology: GameData for its name, requirements and
        // modifier lines, the data file for its button icon and description
        let data = crate::preload::game_data(&self.game_root).ok();
        let resources = crate::ui::hud::load_resources(ui);
        let mut art = std::collections::HashMap::new();
        for n in crate::ui::datafile::load(&ui.store.root, "Properties/properties_technologies.tgi").iter() {
            let Some(id) = n.get("IDS") else { continue };
            let clean = |s: Option<&str>| s.unwrap_or_default().trim_matches('"').trim().to_string();
            let repair: Vec<bool> = n.children_named("Modifier").map(|m| m.get("repair").is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))).collect();
            art.insert(id.to_ascii_lowercase(), (clean(n.get("action_button_icon")), clean(n.get("description")), repair));
        }
        let owned = game.campaign.state.kingdoms[l].technologies.clone();
        let x = tab_x(ui);
        for (i, r) in shop.technologies.iter().enumerate() {
            let Some(ids) = r.ids.clone() else { continue };
            let name = technology_name(ui, data.as_deref(), &ids);
            let tag = 9700 + i as u64;
            add(ui, "MetaGamePurchasingTechnologyPage", "MetaGamePurchaseTechnologyList", format!("PurchaseTechnology {ids}"), name.clone(), r.cost, tag);
            let Some(path) = ui.find_tag(tag) else { continue };
            let rel = path.strip_prefix("Root/").unwrap_or(&path).to_string();
            let (icon, description, repair) = art.get(&ids.to_ascii_lowercase()).cloned().unwrap_or_default();
            // a relative icon is below Properties/
            let icon = (!icon.is_empty() && !icon.eq_ignore_ascii_case("null")).then(|| if icon.starts_with('/') { icon } else { format!("/Properties/{icon}") });
            let enabled = !owned.iter().any(|t| t.eq_ignore_ascii_case(&ids));
            ui.with_widget(&format!("{rel}/PushItem/Decal"), |w| {
                for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
                    set_decal_texture(ve, &icon);
                }
            });
            ui.with_widget(&format!("{rel}/PushItem"), |w| w.set_enabled(enabled));
            // 94BE50: the name, the cost, 63B8C0 "Affects" with what the property
            // requires of an actor, 63BFD0(1, 0) the modifier lines, 638D90 the description
            let mut body = TipBody::default();
            body.cost(ui, r.cost, x);
            if let Some(p) = data.as_ref().and_then(|d| d.property(&ids)) {
                let list: Vec<String> = p.required_properties.iter().map(|q| technology_name(ui, data.as_deref(), q)).collect();
                if !list.is_empty() {
                    body.row(&ui.strings.resolve("#ui_tooltip_item_title_required_properties"), &list.join(", "), x);
                }
                let lines: Vec<String> = p
                    .modifiers
                    .iter()
                    .enumerate()
                    .filter_map(|(k, m)| crate::ui::modifiers::describe(ui, data.as_ref()?, &resources, m, repair.get(k).copied().unwrap_or(false)))
                    .collect();
                if !lines.is_empty() {
                    body.pending = true;
                    for l in &lines {
                        body.row("", l, x);
                    }
                    body.pending = true;
                }
            }
            body.description(&ui.strings.resolve(&description));
            self.tips.push((path, name, body.text));
        }
        ui.viewport_refresh(&format!("Root/{base}/MetaGamePurchasingArmyPage/MetaGamePurchaseArmyList"));
        ui.viewport_refresh(&format!("Root/{base}/MetaGamePurchasingTechnologyPage/MetaGamePurchaseTechnologyList"));
    }

    /// `0x94B780` / `0x94B800`: the pages' toggles and which page shows.
    fn purchase_page(&mut self, ui: &mut Ui, armies: bool) {
        self.purchase_armies = armies;
        let base = "MetaGamePurchasingDialog";
        ui.with_widget(&format!("{base}/MetaGamePurchasingArmyPage"), |w| w.set_visible(armies));
        ui.with_widget(&format!("{base}/MetaGamePurchasingTechnologyPage"), |w| w.set_visible(!armies));
        for (name, on) in [("MetaGameButtonPurchaseArmy", armies), ("MetaGameButtonPurchaseTechnology", !armies)] {
            ui.with_widget(&format!("{base}/{name}"), |w| {
                if on {
                    w.flags |= aa_ui::state::widget_flag::SELECTED;
                } else {
                    w.flags &= !aa_ui::state::widget_flag::SELECTED;
                }
            });
        }
        ui.layout();
    }

    /// `0x94C410`: validated for the local player (951D70); 0x14 and 0x15
    /// answer with their message boxes, another error by 952EC0; with the UVar
    /// (`[B39940]+0x33C`, MetaGameUIPurchaseConfirm) the purchase waits on
    /// `metagame_confirm_purchase|s`, otherwise it is sent and the dialog closes.
    fn purchase_request(&mut self, ui: &mut Ui, game: &mut Game, cmd: Command, name: &str) {
        use aa_metagame::commands::code::{INSUFFICIENT_RESOURCES, PURCHASE_UNOWNED_CAPITAL};
        match game.campaign.validate_local(&cmd) {
            Ok(0) => {
                if ui.uvars.get_bool("MetaGameUIPurchaseConfirm") {
                    self.purchase = Some(cmd);
                    let text = ui.strings.format("metagame_confirm_purchase|s", &[name]);
                    let ok = ui.strings.resolve("#button_ok");
                    let cancel = ui.strings.resolve("#button_cancel");
                    ui.show_message_box(aa_ui::message_box::GAME, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, "PurchaseConfirm".into()), (cancel, "PurchaseCancel".into())] });
                } else {
                    let _ = game.campaign.send_local(cmd);
                    self.close(ui);
                }
            }
            Ok(code) if code == INSUFFICIENT_RESOURCES || code == PURCHASE_UNOWNED_CAPITAL => {
                let key = if code == INSUFFICIENT_RESOURCES { "#metagame_error_insufficient_resources" } else { "#metagame_error_purchase_unowned_capital" };
                let text = ui.strings.resolve(key);
                let ok = ui.strings.resolve("#button_ok");
                ui.show_message_box(aa_ui::message_box::GAME, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, String::new())] });
            }
            Ok(code) => error_message(ui, game, code, &cmd),
            Err(e) => bevy::log::warn!("purchase {cmd:?}: {e}"),
        }
    }

    /// `0x94F2F0`, the Start Battle dialog's Activate: the contested
    /// territory, the first attacker's kingdom (955420) and the defending
    /// kingdom (955EB0) by name, Play for a battle the local player is part of
    /// (94D620) and Watch otherwise, Cancel on the local player's turn (94D5D0).
    /// The army lists are 94DFE0; the tooltip of the Play button is 94E920 ([`rts_tip`]).
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
        self.battle_dialog = Some(involved);
        self.tips.clear();
        if let Some((name, text)) = quick_tip(ui, game) {
            self.tips.push((format!("Root/{base}/MetaGameButtonStartBattleQuickBattle"), name, text));
        }
        if let Some((name, text)) = crate::preload::game_data(&self.game_root).ok().and_then(|data| rts_tip(ui, game, &data)) {
            self.tips.push((format!("Root/{base}/MetaGameButtonStartBattlePlay"), name.clone(), text.clone()));
            self.tips.push((format!("Root/{base}/MetaGameButtonStartBattleWatch"), name, text));
        }
        self.fill_army_list(ui, game, t, Side::Attacker, "MetaGameAttackerPanel", "AttackerArmyList", 9800);
        self.fill_army_list(ui, game, t, Side::Defender, "MetaGameDefenderPanel", "DefenderArmyList", 9900);
        ui.with_widget(&format!("{base}/MetaGameButtonStartBattlePlay"), |w| w.set_visible(involved));
        ui.with_widget(&format!("{base}/MetaGameButtonStartBattleWatch"), |w| w.set_visible(!involved));
        ui.with_widget(&format!("{base}/MetaGameButtonStartBattleCancel"), |w| w.set_visible(local_turn));
    }

    /// `0x94DFE0`: one row per army type of the side, its count kept by a hash
    /// and the rows ordered by 93AFE0 (+0x5B8 +0x18, the damage with the divisions,
    /// ascending; equal ones in the order they were met): the Thing's icon (+0xF0)
    /// on the Decal and `metagame_battle_dialog_army_item_format` of the count in
    /// Column0, and the row's tooltip ([`army_tip`]).
    fn fill_army_list(&mut self, ui: &mut Ui, game: &Game, t: usize, side: Side, panel: &str, list: &str, first_tag: u64) {
        let file = "MetaGame/UI/metagame_start_battle.tgi:MetaGameStartBattleDialog";
        let vp = format!("Root/MetaGameStartBattleDialog/{panel}/{list}");
        ui.viewport_clear(&vp);
        let mut rows: Vec<(usize, i32)> = Vec::new();
        for a in game.campaign.side_armies(t, side) {
            let d = game.campaign.state.army(a).definition;
            match rows.iter_mut().find(|r| r.0 == d) {
                Some(r) => r.1 += 1,
                None => rows.push((d, 1)),
            }
        }
        let damage = |d: usize| f32::from_bits(game.campaign.data.armies[d].damage_with_divisions_bits);
        rows.sort_by(|a, b| damage(a.0).total_cmp(&damage(b.0)));
        let format = ui.strings.resolve("#metagame_battle_dialog_army_item_format");
        for (i, (d, count)) in rows.into_iter().enumerate() {
            let mut row = Widget::new(C::Panel, "Client", false);
            row.add_child(Widget::new(C::Decal, "Decal", false));
            row.add_child(Widget::new(C::Label, "Column0", false));
            if let Ok(n) = ui.store.node(&format!("{file}/{panel}/{list}")) {
                let strings = ui.strings.clone();
                let resolve = |v: &str| strings.resolve(v);
                let root = std::sync::Arc::new(n.clone());
                row.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
            }
            let tag = first_tag + i as u64;
            row.tag = tag;
            ui.viewport_add(&vp, row);
            let Some(path) = ui.find_tag(tag) else { continue };
            let (name, body) = army_tip(ui, game, d, None);
            self.tips.push((path.clone(), name, body));
            let rel = path.strip_prefix("Root/").unwrap_or(&path).to_string();
            ui.set_label(&format!("{rel}/Column0"), &format.replace("%d", &count.to_string()));
            let icon = game.data.thing_icon(&game.campaign.data.armies[d].ids).map(|i| format!("/{}", i.trim_start_matches('/')));
            ui.with_widget(&format!("{rel}/Decal"), |w| {
                for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
                    set_decal_texture(ve, &icon);
                }
            });
        }
        ui.viewport_refresh(&vp);
    }

    /// The dynamic tooltip of a widget (the path up to the row).
    pub fn tooltip(&self, path: &str) -> Option<(String, String)> {
        self.tips.iter().find(|t| path == t.0 || path.strip_prefix(t.0.as_str()).is_some_and(|r| r.starts_with('/'))).map(|t| (t.1.clone(), t.2.clone()))
    }

    fn close(&mut self, ui: &mut Ui) {
        self.tips.clear();
        if let Some(d) = self.dialog.take() {
            let name = match d {
                Dialog::Menu => "MetaGameMenuDialog",
                Dialog::StartBattle => "MetaGameStartBattleDialog",
                Dialog::Purchase => "MetaGamePurchasingDialog",
                Dialog::History => "MetaGameHistoryDialog",
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
            self.open(ui, game, Dialog::Purchase);
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
        if self.help.command(ui, game, command) {
            return Some(Out::None);
        }
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
                // hot commands (flags & 0x0F000000) only on the local turn
                let flags = cmd.data.map(|d| d.flags).unwrap_or(0);
                if flags & 0x0f00_0000 != 0 && !game.campaign.local_turn() {
                    return Some(Out::None);
                }
                // 950EB0: the first selected thing
                cmd.army = game.selected_army;
                // the argument by what the command takes: a division (9498E0 ->
                // 952530), a territory (949850 -> 939B60, 950CF0), a technology
                // (9498B0 -> 463FA0, 950D20), an army type (949890 -> 427BD0, 950CD0)
                if let Some(arg) = words.next() {
                    let data = &game.campaign.data;
                    if flags & 0xf0_0000 != 0 {
                        if let Some(d) = data.divisions.iter().position(|d| d.ids.eq_ignore_ascii_case(arg)) {
                            cmd.divisions = Some(vec![d]);
                        }
                    } else if flags & 0xf0 != 0 {
                        let map = data.campaigns[game.campaign.state.campaign].map.unwrap_or(0);
                        if let Some(t) = data.maps[map].territories.iter().position(|t| t.object_ids.eq_ignore_ascii_case(arg) || t.ids.eq_ignore_ascii_case(arg)) {
                            cmd.set_territory(data, map, t);
                        }
                    } else if flags & 0xf_0000 != 0 {
                        cmd.technology = Some(arg.to_string());
                    } else if flags & 0xf000 != 0
                        && let Some(a) = data.armies.iter().position(|a| a.ids.eq_ignore_ascii_case(arg))
                    {
                        cmd.actor_definition = Some(a);
                    }
                }
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
            // 9336E0: the playbattle order, then the RTS launch 932210 (campaign/battle.rs)
            "launchrtsgame" => {
                self.close(ui);
                if let Some(cmd) = Campaign::command("playbattle") {
                    self.send_now(ui, game, cmd);
                }
                Some(Out::LaunchBattle)
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
            // 933550: on the local player's turn
            "purchase" => {
                if game.campaign.local_turn() {
                    self.open(ui, game, Dialog::Purchase);
                }
                Some(Out::None)
            }
            // 94CFB0: the purchasing dialog's commands
            "close" => {
                self.close(ui);
                Some(Out::None)
            }
            "purchasearmypage" => {
                self.purchase_page(ui, true);
                Some(Out::None)
            }
            "purchasetechnologypage" => {
                self.purchase_page(ui, false);
                Some(Out::None)
            }
            // 94CD20: [army_ids] (950CD0 the army type, command 8)
            "purchasearmy" => {
                let ids = words.next()?;
                let definition = game.campaign.data.armies.iter().position(|a| a.ids.eq_ignore_ascii_case(ids));
                if let (Some(d), Some(mut cmd)) = (definition, Campaign::command("purchaseactor")) {
                    cmd.actor_definition = Some(d);
                    let name = army_name(ui, game, d);
                    self.purchase_request(ui, game, cmd, &name);
                }
                Some(Out::None)
            }
            // 94CE70: [technology_ids] (command 7)
            "purchasetechnology" => {
                let ids = words.next()?;
                if let Some(mut cmd) = Campaign::command("purchasetech") {
                    cmd.technology = Some(ids.to_string());
                    let name = technology_name(ui, crate::preload::game_data(&self.game_root).ok().as_deref(), ids);
                    self.purchase_request(ui, game, cmd, &name);
                }
                Some(Out::None)
            }
            "purchaseconfirm" => {
                if let Some(cmd) = self.purchase.take() {
                    let _ = game.campaign.send_local(cmd);
                }
                self.close(ui);
                Some(Out::None)
            }
            "purchasecancel" => {
                self.purchase = None;
                Some(Out::None)
            }
            // 9375C0: History -> 94AD40
            "history" => {
                self.open(ui, game, Dialog::History);
                Some(Out::None)
            }
            "metagameloaddialog" => {
                self.close(ui);
                Some(Out::LoadDialog)
            }
            "metagamesavedialog" => {
                self.close(ui);
                Some(Out::SaveDialog)
            }
            // 9375C0: the display on or off (the toggle button flips itself)
            "politicsdisplaytoggle" => {
                self.politics = !self.politics;
                self.politics_rows.clear();
                if self.politics {
                    self.refresh_politics(ui, game);
                } else {
                    ui.disconnect(POLITICS_NAME);
                    ui.layout();
                }
                Some(Out::None)
            }
            "gamesave" | "cycleselection" => {
                bevy::log::warn!("MetaGame command {command} refused: its screen is not built");
                Some(Out::None)
            }
            _ => None,
        }
    }

    /// The politics display: per kingdom with a kingdom actor a label (634D70 the
    /// kingdom's glyph and name, `<SetX=%d>%lc` the relation's glyph at offset_relation
    /// for the others; 950290 `<SetX=%d>%s` the money and income 9374F0 at
    /// offset_resources; SetX in window pixels, 5C1DB0), laid out as the game's display:
    /// offset_top, `separation` apart, the panel's height after offset_bottom.
    fn refresh_politics(&mut self, ui: &mut Ui, game: &Game) {
        use aa_metagame::battle::KingdomRelations;
        let node = ui.store.node(POLITICS).ok();
        let num = |k: &str, d: f32| node.as_ref().and_then(|n| n.float(k).ok()).unwrap_or(d);
        let glyph = |ui: &Ui, k: &str| node.as_ref().map(|n| n.string_or_empty(k)).and_then(|n| ui.strings.glyphs.get(&n.to_ascii_lowercase()).copied());
        let scale = ui.screen.height as f32 / ui.reference.height.max(1.0);
        let (relation_x, resources_x) = ((num("offset_relation", 190.0) * scale) as i32, (num("offset_resources", 230.0) * scale) as i32);
        let money = money_glyph(ui);
        let local = game.campaign.local_actor();
        let mut rows = Vec::new();
        for (k, kingdom) in game.campaign.state.kingdoms.iter().enumerate() {
            let Some(actor) = game.campaign.actor(k) else { continue };
            let mut text = message_kingdom(ui, game, k);
            if let Some(l) = local.filter(|l| *l != actor) {
                let key = if game.campaign.relations.enemy(actor, l) { "glyph_enemy" } else { "glyph_ally" };
                if let Some(g) = glyph(ui, key) {
                    text.push_str(&format!("<SetX={relation_x}>{g}"));
                }
            }
            if let Some(g) = money {
                text.push_str(&format!("<SetX={resources_x}>{}", resources_text(g, kingdom.resources, income(game, k))));
            }
            rows.push(text);
        }
        if rows == self.politics_rows && ui.widget(&format!("Root/{POLITICS_NAME}")).is_some() {
            return;
        }
        self.politics_rows = rows.clone();
        ui.disconnect(POLITICS_NAME);
        let mut d = Widget::new(C::Panel, POLITICS, false);
        d.add_child(Widget::new(C::Decal, "Background", false));
        for i in 0..rows.len() {
            let mut l = Widget::new(C::Label, "Label", false);
            l.tag = TAG_POLITICS + i as u64;
            d.add_child(l);
        }
        aa_ui::screens::show(ui, d);
        let alpha = num("transparency", 0.0).clamp(0.0, 1.0);
        ui.with_widget(&format!("{POLITICS_NAME}/Background"), |w| {
            if let Some(aa_ui::ve::ViewElement { kind: aa_ui::ve::VeKind::Decal(dc), .. }) = &mut w.sub_ve {
                dc.color[3] = alpha;
            }
        });
        let (top, bottom, sep) = (num("offset_top", 20.0), num("offset_bottom", 20.0), num("separation", 3.0));
        let label_h = 15.0;
        let n = rows.len() as f32;
        let h = top + n * label_h + (n - 1.0).max(0.0) * sep + bottom;
        ui.with_widget(POLITICS_NAME, |w| aa_ui::geometry::set_height(w, h));
        for (i, text) in rows.iter().enumerate() {
            let Some(p) = ui.find_tag(TAG_POLITICS + i as u64) else { continue };
            let rel = p.strip_prefix("Root/").unwrap_or(&p).to_string();
            let y = top + i as f32 * (label_h + sep);
            ui.with_widget(&rel, |w| aa_ui::controls::set_bounds(&mut w.root.outer, [23.0, y, 340.0, label_h], 0x300));
            ui.set_label(&rel, text);
        }
        ui.layout();
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
                    // 953A20: game events 0x3A territory_positive / 0x3B territory_negative
                    // of the winner and the loser, their FXC_GameEventSFX
                    let local = game.campaign.local;
                    let good = if Some(*attacker) == local { Some(*attacker_won) } else if defender.is_some() && *defender == local { Some(!*attacker_won) } else { None };
                    if let Some(good) = good {
                        ui.events.push(aa_ui::ui::UiEvent::Sound { name: if good { "territory_captured" } else { "territory_lost" }.to_string() });
                    }
                    self.help.battle_result(ui, game, *defender, *attacker_won);
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
                            let tech = technology_name(ui, crate::preload::game_data(&self.game_root).ok().as_deref(), &tech);
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
                // 938E10 -> 50D370: the mission boxes, as a battle's end (0x50C600
                // case 2 -> 0x440190): the local kingdom won or lost the campaign
                Note::Engine(EngineCall::GameOver) => {
                    let won = game.campaign.local_actor().is_some_and(|l| game.campaign.won.contains(&l));
                    bevy::log::info!("the campaign is over (938E10), local won: {won}");
                    let key = if won { "#ui_mission_message_won" } else { "#ui_mission_message_lost" };
                    let text = ui.strings.resolve(key);
                    let end = ui.strings.resolve("#ui_mission_label_end_game");
                    ui.show_message_box(aa_ui::message_box::GAME, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(end, "RequestFinished".into())] });
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
pub(super) fn selection_point(game: &Game) -> Option<[f32; 2]> {
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
