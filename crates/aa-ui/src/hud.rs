//! In-game HUD sub-widgets (`KKC_UIGame` Init `0x6182B0` and the classes it
//! builds), laid out from the TGI data with the native code-built trees and
//! filled from an explicit [`HudWorld`] the native world implements.
//!
//! Ported (research/phase8-ui/scratch-hud/lead/notes.txt):
//! * EconomyBar `0x603C40` / EconomicImpact `0x603E70`: one resource widget
//!   per registered resource (`0x603B70`), update `0x602D10` (net rates,
//!   visibility, the impact projection), texts `0x602BD0` -> `0x5C9830` /
//!   `0x5C8F10`, the hidden-offset slide (`0x603190`, `0x602480`, movers
//!   `0x5C3260`) driven by the impact (`0x602640`, `0x6025B0`).
//! * FactionExperience `0x604080` (update `0x604130`, text `0x4CE3A0`).
//! * Floaters `0x608600`: ShowButton ("ShowFloater %i"), EndDecal, the
//!   content added while open (`0x607A50`); Groups `0x607B30` (10 buttons,
//!   x = i * width * 0.1, `0x6065D0`); Abilities viewport (`0x607FC0`,
//!   items `0x6078D0`, end decal `0x606820`).
//! * Idle buttons `0x623520` ("CycleSelection %d", shown while the world
//!   has an idle builder / detached organization, `0x6233C0`).
//! * TimeMessage (`0x5D27B0` "%d:%02d:%02d", added only while the UVar
//!   ViewShowElapsedGameTime, `0x612010`).
//! * ObjectDisplay `0x62CBA0` with the CompanyDisplay `0x630320`
//!   (SetActor `0x62EF20`, Refresh `0x62E3B0`): name, owner (`0x4E9560` /
//!   `0x4E93C0` markup), portrait / banner models, health / morale /
//!   attack-efficiency bars (`0x5DF010` / `0x5DF060`), economy label
//!   (`0x5C4CB0`), state label, company units (`0x62F690` layout,
//!   `0x62D520` / `0x6259D0` content), HQ attachments (`0x6301C0`,
//!   `0x62D010` grid, `0x62E0F0`), formation buttons (`0x629AB0`,
//!   `0x628490`, `0x623D30`), properties (`0x629440`, `0x629620`).
//! * ActionButtons `0x5F9520`: 12 buttons on a 4 x 3 grid (`0x5F4540`).
//!
//! Not ported (explicit): SimpleActor / Settlement / Group displays
//! ([`HudSelection::Unsupported`]), the recruit item floater (AA disables
//! it: UIUseRecruitItemFloater = false), tooltips, the pinned impact modes
//! (+0xD9 / +0xDA), the unit border decals of company units, the action
//! button state machine (the world supplies each slot).

use crate::state::widget_flag as wf;
use crate::ui::Ui;
use crate::widget::{Widget, WidgetClass as C};
use crate::controls as k;
use std::sync::Arc;

pub const GAME: &str = "Root/Game";

/// Resource registry entry (`0x48B800`).
#[derive(Clone, Debug, PartialEq)]
pub struct HudResource {
    /// IDS (resource+8): also the resource widget's name.
    pub ids: String,
    /// +0x18: 0 stockpiled, 1 upkeep, 2 limited consumed.
    pub kind: u8,
    /// Special glyph IDS of `icon` (+0x28).
    pub icon: String,
    /// +0x30 (index of the limiting upkeep resource).
    pub limited_by: Option<usize>,
    /// +0x2C: some limited resource is limited by this one.
    pub limit_provider: bool,
    /// +0x1C: purchase cost per stockpiled resource (registry order).
    pub purchase_cost: Vec<f32>,
}

/// The local kingdom's economy (`0x602760` -> `0x5C1760`), per resource.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HudEconomy {
    /// economy+0x1A0.
    pub production: Vec<f32>,
    /// economy+0x1AC (used by the impact projection).
    pub preview_production: Vec<f32>,
    /// economy+0x1B8.
    pub upkeep: Vec<f32>,
    /// economy+0x1C4.
    pub stock: Vec<f32>,
    /// The limited resource maximum (SVar int `[0xB39AB0]+0x1B4`), the
    /// impact projection's cap for limit providers.
    pub limited_max: f32,
}

/// An economic impact being previewed (`0x602640`): upkeep changes
/// (EconomicImpact +0x84) and costs (+0x90).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HudImpact {
    pub upkeep: Vec<f32>,
    pub cost: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactionExperience {
    /// fe+0xC.
    pub value: f32,
    /// (fe+4)+0x3C; negative = no maximum shown.
    pub max: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HudAbility {
    /// (ability+4)+0x30.
    pub texture: Option<String>,
    pub command: String,
    pub recharge: f32,
}

/// The owner line (`0x4E9560`).
#[derive(Clone, Debug, PartialEq)]
pub struct OwnerInfo {
    pub name: String,
    /// The player's rank glyph (`0x7666C0`), if the owner has a profile;
    /// otherwise the AVar KingdomGlyphInfo glyph (`0x4E93C0`).
    pub rank_glyph: Option<u32>,
    /// Kingdom colour (kingdom+0x1E8 +0x18).
    pub kingdom_color: [f32; 3],
    /// Diplomatic colour (`0x457CD0`), if any.
    pub relation_color: Option<[f32; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitIcon {
    pub texture: Option<String>,
    /// org+0x50[i] / max health (`0x62D520`).
    pub health: f32,
    /// The AltBackground decal instead of Background (`0x6259D0`, info
    /// byte +0x20).
    pub alt_background: bool,
}

/// One slot of the organization layout (layout+0x28 / +0x1C).
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSlot {
    /// Position index (layout+0x28[i]).
    pub position: usize,
    /// Definition (front / support / flank) of the position (entry+4).
    pub definition: i32,
    /// The unit at the position (org+0x1C[position]).
    pub unit: Option<UnitIcon>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HqSlot {
    pub texture: Option<String>,
    pub health: f32,
    pub morale: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Formation {
    /// formation+0xC.
    pub id: i32,
    /// formation+0x1C.
    pub texture: String,
    pub selected: bool,
}

/// A company (regiment / HQ) selection (`0x62D660` accepts actors with an
/// organization).
#[derive(Clone, Debug, PartialEq)]
pub struct CompanyInfo {
    /// `0x4EBA80`.
    pub name: String,
    pub owner: Option<OwnerInfo>,
    /// Leader portrait model (`0x624270`), `None` hides the health bar.
    pub portrait_model: Option<String>,
    /// Banner model (`0x624DA0`).
    pub banner_model: Option<String>,
    /// `0x578BA0` (shown with a leader).
    pub health: f32,
    /// org+0x5C / org+0x60 and org+0xB8 / org+0x60.
    pub morale: (f32, f32),
    /// org+0x84, org+0x88.
    pub attack_efficiency: (f32, f32),
    /// Supply stockpile fraction (SupplyStockpile bar, with the AVar
    /// UIUseSupplyStockpile); `None` hides the bar (05-regiment shows
    /// none, 06-corps one: which code hides it was not traced).
    pub supply: Option<f32>,
    /// Upkeep per resource (`0x56FE00`), `None` when not shown.
    pub economy: Option<Vec<f32>>,
    /// State name (`0x4D1510` + 0x10).
    pub state: Option<String>,
    pub slots: Vec<LayoutSlot>,
    /// layout+0x18.
    pub leader_slot: i32,
    /// HQ attachments (actor+0xD8, own kingdom); `None` = a company.
    pub hq: Option<Vec<Option<HqSlot>>>,
    /// Up to 3 formations (only when the layout has more than one).
    pub formations: Vec<Formation>,
    /// Property icons (`0x627240`).
    pub properties: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HudSelection {
    None,
    Company(Box<CompanyInfo>),
    /// A selection whose display is not ported yet.
    Unsupported(&'static str),
}

/// One action button (ActionButtons slot, `0x5F5A90`).
#[derive(Clone, Debug, PartialEq)]
pub struct HudAction {
    pub texture: String,
    pub command: String,
    pub enabled: bool,
    pub selected: bool,
    pub targeted: bool,
}

/// The game data the HUD reads. The native world implements this.
pub trait HudWorld {
    /// Registered resources in registry order (`[0xB36530]+0x34`).
    fn resources(&self) -> Vec<HudResource>;
    fn economy(&self) -> Option<HudEconomy>;
    /// The impact of the hovered action, if any.
    fn economic_impact(&self) -> Option<HudImpact>;
    /// player+0x230.
    fn faction_experience(&self) -> Option<FactionExperience>;
    fn faction_abilities(&self) -> Vec<HudAbility>;
    /// Texture of the group of each `groups_shown` entry, `None` when the
    /// group is unassigned.
    fn group_textures(&self) -> Vec<Option<String>>;
    fn idle_builder(&self) -> bool;
    fn idle_detached_organization(&self) -> bool;
    /// world+0x8C seconds.
    fn game_time(&self) -> f32;
    fn selection(&self) -> HudSelection;
    /// The 12 action button slots.
    fn action_buttons(&self) -> Vec<Option<HudAction>>;
}

/// The same reads from a live world, where each source can still be
/// unbound: an error is recorded once in [`Hud::diagnostics`] and the widget
/// it would update keeps its current state (no value is invented).
pub trait LiveWorld {
    fn economy(&self) -> Result<Option<HudEconomy>, String>;
    fn economic_impact(&self) -> Result<Option<HudImpact>, String>;
    fn faction_experience(&self) -> Result<Option<FactionExperience>, String>;
    fn faction_abilities(&self) -> Result<Vec<HudAbility>, String>;
    fn group_textures(&self) -> Result<Vec<Option<String>>, String>;
    fn idle_builder(&self) -> Result<bool, String>;
    fn idle_detached_organization(&self) -> Result<bool, String>;
    /// TimeMessage `0x5D27B0`: VWorld+0x8C seconds.
    fn game_time(&self) -> Result<f32, String>;
    fn selection(&self) -> Result<HudSelection, String>;
    fn action_buttons(&self) -> Result<Vec<Option<HudAction>>, String>;
}

struct Infallible<'a>(&'a dyn HudWorld);
impl LiveWorld for Infallible<'_> {
    fn economy(&self) -> Result<Option<HudEconomy>, String> {
        Ok(self.0.economy())
    }
    fn economic_impact(&self) -> Result<Option<HudImpact>, String> {
        Ok(self.0.economic_impact())
    }
    fn faction_experience(&self) -> Result<Option<FactionExperience>, String> {
        Ok(self.0.faction_experience())
    }
    fn faction_abilities(&self) -> Result<Vec<HudAbility>, String> {
        Ok(self.0.faction_abilities())
    }
    fn group_textures(&self) -> Result<Vec<Option<String>>, String> {
        Ok(self.0.group_textures())
    }
    fn idle_builder(&self) -> Result<bool, String> {
        Ok(self.0.idle_builder())
    }
    fn idle_detached_organization(&self) -> Result<bool, String> {
        Ok(self.0.idle_detached_organization())
    }
    fn game_time(&self) -> Result<f32, String> {
        Ok(self.0.game_time())
    }
    fn selection(&self) -> Result<HudSelection, String> {
        Ok(self.0.selection())
    }
    fn action_buttons(&self) -> Result<Vec<Option<HudAction>>, String> {
        Ok(self.0.action_buttons())
    }
}

fn w(class: C, name: &str) -> Widget {
    Widget::new(class, name, false)
}

fn label(name: &str) -> Widget {
    w(C::Label, name)
}

/// `0x603B70`: a resource widget (name = resource IDS) and its label.
fn resource_widget(ids: &str) -> Widget {
    let mut r = w(C::Panel, ids);
    r.add_child(label("ResourceLabel"));
    r
}

/// Floater `0x608600`.
fn floater(ui: &Ui, file_node: &str, kind: i32) -> Widget {
    let mut f = w(C::Panel, file_node);
    f.add_child(Widget::toggle_button("ShowButton", false, &format!("ShowFloater {kind}")));
    if ui.avars.flag("UIFloatersDisplayEndCap").unwrap_or(false) {
        f.add_child(w(C::Decal, "EndDecal"));
    }
    match kind {
        1 => {
            let mut g = w(C::Panel, "Groups");
            for _ in 0..10 {
                // 0x6077B0: push button with the GroupDecal client
                let mut b = w(C::PushButton, "GroupButton");
                b.client_name = "GroupDecal".into();
                b.add_child(w(C::Decal, "GroupDecal"));
                g.add_child(b);
            }
            f.add_child(g);
        }
        2 => f.add_child_ret(Widget::composite(C::Viewport, "Abilities", false)),
        _ => {}
    }
    f
}

trait AddRet {
    fn add_child_ret(&mut self, c: Widget);
}

impl AddRet for Widget {
    fn add_child_ret(&mut self, c: Widget) {
        self.add_child(c);
    }
}

/// `0x6078D0`: an ability item.
fn ability_item() -> Widget {
    let mut it = w(C::Panel, "Ability");
    let mut b = w(C::PushButton, "AbilityButton");
    b.client_name = "AbilityButtonDecal".into();
    b.add_child(w(C::Decal, "AbilityButtonDecal"));
    it.add_child(b);
    it.add_child(w(C::StatusBar, "Recharge"));
    it
}

/// UnitDisplayStatic (company unit, `0x629D70`).
fn unit_static(name: &str) -> Widget {
    let mut u = w(C::Panel, name);
    let mut b = w(C::PushButton, "UnitButton");
    b.client_name = "UnitButtonDecal".into();
    b.add_child(w(C::Decal, "UnitButtonDecal"));
    u.add_child(b);
    u.add_child(w(C::StatusBar, "Health"));
    u
}

/// CompanyDisplay `0x630320` (children in AddChild order; UnitDisplay
/// CreateChildren `0x628940` first).
fn company_display(ui: &Ui) -> Widget {
    let mut d = w(C::Panel, "CompanyDisplay");
    // 0x628940: PortraitButton (client Portrait), DescriptionLabel
    let mut pb = w(C::PushButton, "PortraitButton");
    pb.client_name = "Portrait".into();
    pb.add_child(w(C::Model, "Portrait"));
    d.add_child(pb);
    // banner button with the Banner model client
    let mut bb = w(C::PushButton, "BannerButton");
    bb.client_name = "Banner".into();
    bb.add_child(w(C::Model, "Banner"));
    d.add_child(bb);
    d.add_child(w(C::StatusBar, "Morale"));
    d.add_child(w(C::StatusBar, "AttackEfficiency"));
    d.add_child(label("Name"));
    d.add_child(label("Owner"));
    d.add_child(w(C::StatusBar, "Health"));
    if ui.avars.flag("UIUseSupplyStockpile").unwrap_or(false) {
        d.add_child(w(C::StatusBar, "SupplyStockpile"));
    }
    d.add_child(label("Economy"));
    if ui.avars.flag("UIShouldUseStateLabel").unwrap_or(false) {
        d.add_child(label("State"));
    }
    d
}

/// CompanyUnits `0x62F550`: 9 CompanyUnit statics.
fn company_units() -> Widget {
    let mut p = w(C::Panel, "CompanyUnits");
    for _ in 0..9 {
        p.add_child(unit_static("CompanyUnit"));
    }
    p
}

/// HQAttached `0x6301C0`: 6 HQAttachedOrganization slots.
fn hq_attached() -> Widget {
    let mut p = w(C::Panel, "HQAttached");
    for _ in 0..6 {
        let mut s = w(C::Panel, "HQAttachedOrganization");
        let mut b = w(C::PushButton, "Button");
        b.client_name = "ButtonDecal".into();
        b.add_child(w(C::Decal, "ButtonDecal"));
        s.add_child(b);
        s.add_child(w(C::StatusBar, "Morale"));
        s.add_child(w(C::StatusBar, "Health"));
        p.add_child(s);
    }
    p
}

/// FormationButtons `0x629AB0`: 3 toggle buttons with decal clients.
fn formation_buttons() -> Widget {
    let mut p = w(C::Panel, "FormationButtons");
    for _ in 0..3 {
        let mut b = w(C::ToggleButton, "FormationButton");
        // 0x629AB0: +0xE8 = 0
        b.no_auto_toggle = true;
        b.client_name = "FormationButtonDecal".into();
        b.add_child(w(C::Decal, "FormationButtonDecal"));
        b.set_visible(false);
        p.add_child(b);
    }
    p
}

/// UnitProperties `0x629440`: PropertyMoreIndicator and 7 PropertyBars.
fn properties() -> Widget {
    let mut p = w(C::Panel, "Properties");
    let mut more = w(C::Decal, "PropertyMoreIndicator");
    more.set_visible(false);
    p.add_child(more);
    for _ in 0..7 {
        let mut b = w(C::Decal, "PropertyBar");
        b.set_visible(false);
        p.add_child(b);
    }
    p
}

/// ActionButtons `0x5F9520`: 12 "ActionButtonsButton" (`0x5F5A90`).
fn action_buttons() -> Widget {
    let mut p = w(C::Panel, "UI/Game/action_buttons.tgi:ActionButtons");
    for _ in 0..12 {
        let mut b = w(C::ToggleButton, "ActionButtonsButton");
        b.client_name = String::new();
        b.set_visible(false);
        p.add_child(b);
    }
    p
}

/// The economy bar slide state (+0xAC..+0xC4).
#[derive(Clone, Debug, Default)]
struct Slide {
    init: bool,
    shown: (f32, f32),
    hidden: (f32, f32),
    is_shown: bool,
    time: f32,
}

/// Values of a resource widget (+0x7C amount, +0x80 stock, +0x84 limit).
#[derive(Clone, Debug, Default, PartialEq)]
struct ResValues {
    amount: f32,
    stock: f32,
    limit: f32,
    dirty: bool,
}

/// The HUD controller.
pub struct Hud {
    resources: Vec<HudResource>,
    bar_values: Vec<ResValues>,
    impact_values: Vec<ResValues>,
    slide: Slide,
    impact_active: bool,
    last_experience_ms: Option<u64>,
    last_idle_ms: Option<u64>,
    /// Owned sub-displays not in the tree (ObjectDisplay +0x88..+0x94).
    stash: Vec<Widget>,
    selection: Option<&'static str>,
    groups_placed: bool,
    /// Registry IDS the EconomyBar / EconomicImpact children were built for.
    built_ids: Vec<String>,
    /// The VWorld widget `DAT_00B36538` (UI+0xD0), built by the UI
    /// (`0x5CAE30` -> `0x442C00`, `UI/Shared/VWorld.tgi:VWorld`) and held
    /// here until the Game interface connects it.
    vworld: Option<Widget>,
    pub diagnostics: Vec<String>,
}

impl Hud {
    /// Build the game interface (`0x61A650` / Init `0x6182B0`) with the
    /// code-built sub-widgets and connect it.
    pub fn show(ui: &mut Ui, world: &dyn HudWorld) -> Self {
        ui.clear();
        let resources = world.resources();
        let ids: Vec<String> = resources.iter().map(|r| r.ids.clone()).collect();
        let (mut hud, g) = Self::construct(ui, &ids);
        // the same registry entries, so the binding cannot fail here
        let _ = hud.bind_resources(resources);
        ui.connect(g);
        hud.on_connected(ui);
        hud
    }

    /// Resource registry entries (`0x48B800`) read by the update, in the
    /// SAME registry order the EconomyBar children were built from.
    pub fn bind_resources(&mut self, resources: Vec<HudResource>) -> Result<(), String> {
        if resources.len() != self.built_ids.len() || resources.iter().zip(&self.built_ids).any(|(r, ids)| &r.ids != ids) {
            return Err(format!(
                "HUD resource binding {:?} differs from the registry the EconomyBar was built for {:?}",
                resources.iter().map(|r| r.ids.as_str()).collect::<Vec<_>>(),
                self.built_ids
            ));
        }
        self.resources = resources;
        Ok(())
    }

    /// Init `0x6182B0`, which KKC_Application::Init (`0x41AA7A`) runs once on
    /// the Game interface right after the interfaces are allocated
    /// (`0x61A650`, Application+0xBC = `0xB3974C`): the code-built tree in
    /// AddChild order, loaded from the TGI data but NOT connected to the UI
    /// root. `resource_ids` is the resource registry (`[0xB36530]+0x34`) in
    /// registry order (EconomyBar ctor `0x603C40` -> `0x603B70`). Connect the
    /// returned widget, then call [`Hud::on_connected`].
    pub fn construct(ui: &mut Ui, resource_ids: &[String]) -> (Self, Widget) {
        let flag = |ui: &Ui, k: &str| ui.avars.flag(k).unwrap_or(false);
        let mut g = Widget::new(C::Interface, "UI/Game/game_interface.tgi:Game", false);
        // sort keys (vtable+0xA8): EconomyBar 1.0 (0x6E80C0), EconomicImpact
        // 0.5 (0x55D990), FactionExperience 1.0; everything else 0
        let mut bar = w(C::Panel, "UI/Game/economy_bar.tgi:EconomyBar");
        bar.key = 1.0;
        for ids in resource_ids {
            bar.add_child(resource_widget(ids));
        }
        g.add_child(bar);
        // 0x5FD6D0(name, 0, slot 0x47 UIMessageLogMaximumLines, fades 1,
        // auto height 1)
        g.add_child(crate::message_log::message_log("UI/Game/message_log.tgi:GameMessageLog", "UIMessageLogMaximumLines", true, true));
        if flag(ui, "UIShowFactionExperience") {
            let mut fe = label("/FactionExperience");
            fe.key = 1.0;
            g.add_child(fe);
        }
        let mut upkeep = w(C::Decal, "/UpkeepShortageIcon");
        upkeep.set_visible(false);
        g.add_child(upkeep);
        // 0x618502: the side panel only with UIUseSidePanel; the
        // objectives / technology / politics displays and the side message
        // log are owned, not added
        if flag(ui, "UIUseSidePanel") {
            g.add_child(w(C::Panel, "UI/Game/side_panel.tgi:SidePanel"));
        }
        if flag(ui, "UIUseEconomicImpact") {
            let mut imp = w(C::Panel, "UI/Game/economy_bar.tgi:EconomicImpact");
            imp.key = 0.5;
            for ids in resource_ids {
                imp.add_child(resource_widget(ids));
            }
            // 0x603E70: hidden at construction
            imp.set_visible(false);
            g.add_child(imp);
        }
        if flag(ui, "UIUseRecruitItemFloater") {
            g.add_child(floater(ui, "UI/Game/floaters.tgi:RecruitItemFloater", 0));
        }
        g.add_child(floater(ui, "UI/Game/floaters.tgi:GroupFloater", 1));
        g.add_child(floater(ui, "UI/Game/floaters.tgi:FactionAbilitiesFloater", 2));
        for (n, kind) in [("IdleButtonBuilder", 0), ("IdleButtonDetachedOrganization", 1)] {
            let mut ib = w(C::Panel, &format!("UI/Game/idle_buttons.tgi:{n}"));
            ib.add_child(Widget::push_button("IdleButton", false, &format!("CycleSelection {}", kind + 1)));
            g.add_child(ib);
        }
        let mut kd = w(C::DropDown, "/KingdomDropDown");
        kd.set_visible(false);
        g.add_child(kd);
        g.add_child(label("UI/Shared/dev_mode_label.tgi:DeveloperModeLabel"));
        g.add_child(label("UI/Game/time_message.tgi:TimeMessage"));
        g.add_child(label("UI/Shared/camera_info_label.tgi:CameraInformationLabel"));
        let mut banner = w(C::Panel, "UI/Shared/display_banner.tgi:DisplayBannerPanel");
        banner.add_child(w(C::Decal, "UI/Shared/display_banner.tgi:DisplayBannerPanel/DisplayBannerBackground"));
        banner.add_child(label("UI/Shared/display_banner.tgi:DisplayBannerPanel/DisplayBanner"));
        banner.set_visible(false);
        g.add_child(banner);
        let mut pause = label("UI/Shared/display_banner.tgi:DisplayPause");
        pause.set_visible(false);
        g.add_child(pause);
        let mut subs = w(C::Panel, "Subtitles");
        subs.set_visible(false);
        g.add_child(subs);
        // ControlPanel 0x615FB0
        let mut cp = w(C::Panel, "UI/Game/control_panel.tgi:ControlPanel");
        cp.add_child(w(C::Panel, "UI/Game/object_display.tgi:ObjectDisplay"));
        cp.add_child(action_buttons());
        cp.add_child(w(C::Panel, "SidePanelTabs"));
        for (n, c) in [
            ("ButtonMenu", "OpenGameMenuDialog"),
            ("ButtonObjectives", "ObjectivesDisplayToggle"),
            ("ButtonTechnology", "TechnologyDisplayToggle"),
            ("ButtonMessageLog", "MessageLogToggle"),
        ] {
            cp.add_child(Widget::push_button(n, false, c));
        }
        cp.add_child(Widget::toggle_button("ButtonPolitics", false, "PoliticsDisplayToggle"));
        cp.add_child(Widget::push_button("ButtonSpeedUp", false, "SpeedUp"));
        cp.add_child(Widget::push_button("ButtonSpeedDown", false, "SpeedDown"));
        cp.add_child(Widget::toggle_button("ButtonSupplyBorder", false, "ToggleShowZoneSupply"));
        let mut pause = Widget::toggle_button("ButtonPause", false, "GamePause");
        // 0x61687C: +0xE8 = 0
        pause.no_auto_toggle = true;
        cp.add_child(pause);
        cp.add_child(Widget::toggle_button("ButtonChat", false, "ChatActivate"));
        cp.add_child(w(C::MiniMap, "/UI/Game/minimap.tgi:GameMiniMap"));
        g.add_child(cp);
        g.add_child(w(C::Decal, "ControlPanelTop"));
        ui.load_widget(&mut g, None);
        // ActionButtonsButton LoadData 0x5F4820: the ActionButtonsButtonDecal
        // decal VE (type 2) joins the button's root view element
        if let Ok(dn) = ui.store.node("UI/Game/action_buttons.tgi:ActionButtons/ActionButtonsButton/ActionButtonsButtonDecal") {
            let strings = ui.strings.clone();
            let resolve = |v: &str| strings.resolve(v);
            if let Ok(ve) = crate::ve::ViewElement::load_localized(&dn, &resolve) {
                if let Some(ab) = g.find_mut("ControlPanel/ActionButtons") {
                    for b in &mut ab.children {
                        b.sub_ve = Some(ve.clone());
                    }
                }
            }
        }
        // the floater contents are owned by the floaters; they enter the
        // tree while open (0x607A50)
        // only the widget names are needed to build the tree; the update
        // reads the entries bound by `bind_resources`
        let built = resource_ids.len();
        let mut hud = Hud {
            resources: Vec::new(),
            bar_values: Vec::new(),
            impact_values: Vec::new(),
            slide: Slide::default(),
            impact_active: false,
            last_experience_ms: None,
            last_idle_ms: None,
            stash: Vec::new(),
            selection: None,
            groups_placed: false,
            built_ids: resource_ids.to_vec(),
            vworld: None,
            diagnostics: Vec::new(),
        };
        // KKC_UI 0x5CAE30: `new VWorld("UI/Shared/VWorld.tgi:VWorld", 1)`
        // (0x442C00 -> 0x5F1D00 registered); its data is a Widget with
        // render_threshold 15 and a DUMMY `[View]`, so the world can be picked
        let mut vworld = Widget::new(C::Static, "UI/Shared/VWorld.tgi:VWorld", true);
        ui.load_widget(&mut vworld, None);
        hud.vworld = Some(vworld);
        hud.bar_values = vec![ResValues { dirty: true, ..Default::default() }; built];
        hud.impact_values = hud.bar_values.clone();
        // ObjectDisplay sub displays (0x62CBA0): built and loaded against
        // the ObjectDisplay node, owned until a selection shows them
        let od_node = ui.store.node("UI/Game/object_display.tgi:ObjectDisplay").ok();
        if let Some(n) = od_node {
            let root = Arc::new(n.clone());
            let strings = ui.strings.clone();
            let resolve = |v: &str| strings.resolve(v);
            let mut cd = company_display(ui);
            cd.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
            // CompanyUnits / HQAttached / FormationButtons / Properties are
            // owned by the company display (added by SetActor)
            if let Some(cn) = n.child("CompanyDisplay") {
                let croot = root.clone();
                for mut sub in [company_units(), hq_attached(), formation_buttons(), properties()] {
                    sub.load(&mut ui.store, Some((cn, &croot)), Some(10), &resolve);
                    if sub.name == "CompanyUnits" {
                        // UnitDisplayStatic: the Background / AltBackground
                        // decal VEs join the root view element (0x5E37A0)
                        let unit = cn.child("CompanyUnits").and_then(|u| u.child("CompanyUnit"));
                        for name in ["Background", "AltBackground"] {
                            let ve = unit
                                .and_then(|u| u.child(name))
                                .and_then(|b| crate::ve::ViewElement::load_localized(b, &resolve).ok());
                            if let Some(mut ve) = ve {
                                if name == "AltBackground" {
                                    ve.layout.flags &= !crate::layout::ve_flag::VISIBLE;
                                }
                                for u in &mut sub.children {
                                    if let Some(v) = &mut u.view {
                                        v.children.push(ve.clone());
                                    }
                                }
                            }
                        }
                    }
                    hud.stash.push(sub);
                }
            }
            hud.stash.insert(0, cd);
        }
        (hud, g)
    }

    /// The Game interface was connected to the UI root (Show `0x5C0440`):
    /// OnConnected `0x60BAE0` and its children's OnConnected placements.
    pub fn on_connected(&mut self, ui: &mut Ui) {
        // 0x60BAE0 -> 0x43B490(VWorld, Game): Game vt+0xD4 AddChild(VWorld),
        // the head of its key group (behind every other child for drawing
        // and picking)
        if let Some(vworld) = self.vworld.take() {
            if ui.widget(crate::press_pick::VWORLD_PATH).is_none() {
                if ui.attach_front(GAME, vworld.clone()).is_none() {
                    self.vworld = Some(vworld);
                }
            }
        }
        self.place_static(ui);
        ui.layout();
    }

    fn stash_take(&mut self, name: &str) -> Option<Widget> {
        let i = self.stash.iter().position(|x| x.name == name)?;
        Some(self.stash.remove(i))
    }

    /// OnConnected placements that do not depend on world data.
    fn place_static(&mut self, ui: &mut Ui) {
        ui.layout();
        let g = ui.geo();
        // ActionButtons 0x5F4540: 4 x 3 cells of floor(W/4) x floor(H/3),
        // buttons centred with floor((cell - button) * 0.5)
        let ab = format!("{GAME}/ControlPanel/ActionButtons");
        if let Some(p) = ui.widget(&ab) {
            let r = p.root.outer.rect;
            let (pw, ph) = ((r.r - r.l) / g.sx, (r.b - r.t) / g.sy);
            let cw = (pw * 0.25).floor();
            let ch = (ph * (1.0f32 / 3.0)).floor();
            let b0 = p.children.first().map(|b| b.root.outer.rect);
            if let Some(b0) = b0 {
                let ox = ((cw - (b0.r - b0.l) / g.sx) * 0.5).floor();
                let oy = ((ch - (b0.b - b0.t) / g.sy) * 0.5).floor();
                for row in 0..3 {
                    for col in 0..4 {
                        let i = row * 4 + col;
                        let path = format!("{ab}/{}", seg("ActionButtonsButton", i));
                        ui.set_position(&path, col as f32 * cw + ox, row as f32 * ch + oy);
                    }
                }
            }
        }
        // economy bar OnConnected 0x603190: shown / hidden positions, taken
        // once (a later reconnect keeps the first positions)
        let bar = format!("{GAME}/EconomyBar");
        if self.slide.init {
            ui.layout();
            return;
        }
        if let (Some((x, y)), Ok(node)) = (ui.raw_position(&bar), ui.store.node("UI/Game/economy_bar.tgi:EconomyBar")) {
            let off = node.point2("hidden_offset").unwrap_or([0.0, 0.0]);
            let time = node.float("slide_time").unwrap_or(0.0);
            self.slide = Slide { init: true, shown: (x, y), hidden: (x + off[0], y + off[1]), is_shown: false, time };
            if off[0] != 0.0 || off[1] != 0.0 {
                ui.set_position(&bar, x + off[0], y + off[1]);
            }
        }
        ui.layout();
    }

    /// Per-frame update.
    pub fn update(&mut self, ui: &mut Ui, world: &dyn HudWorld) {
        let world = Infallible(world);
        self.update_children_inner(ui, &world, true);
    }

    /// The children's own per-frame updates for a live world, after the
    /// Game interface update `0x612010` (`crate::game_interface_update`)
    /// owns TimeMessage attachment and the label/icon visibilities.
    pub fn update_children(&mut self, ui: &mut Ui, world: &dyn LiveWorld) {
        self.update_children_inner(ui, world, false);
    }

    fn update_children_inner(&mut self, ui: &mut Ui, world: &dyn LiveWorld, placeholder_interface: bool) {
        let now = ui.now_ms;
        self.update_economy(ui, world);
        // FactionExperience 0x604130: every 1000 ms
        if self.last_experience_ms.is_none_or(|t| now.saturating_sub(t) >= 1000) {
            let path = format!("{GAME}/FactionExperience");
            match world.faction_experience() {
                Ok(Some(fe)) => {
                    self.last_experience_ms = Some(now);
                    let text = experience_text(ui, &fe);
                    ui.set_label(rel(&path), &text);
                    ui.with_widget(rel(&path), |x| x.set_visible(true));
                }
                Ok(None) => {
                    self.last_experience_ms = Some(now);
                    ui.with_widget(rel(&path), |x| x.set_visible(false));
                }
                Err(e) => self.diagnostics_once(&e),
            }
        }
        // idle buttons 0x6233C0: every 1000 ms
        if self.last_idle_ms.is_none_or(|t| now.saturating_sub(t) >= 1000) {
            match (world.idle_builder(), world.idle_detached_organization()) {
                (Ok(b), Ok(d)) => {
                    self.last_idle_ms = Some(now);
                    ui.with_widget("Game/IdleButtonBuilder", |x| x.set_visible(b));
                    ui.with_widget("Game/IdleButtonDetachedOrganization", |x| x.set_visible(d));
                }
                (Err(e), _) | (_, Err(e)) => self.diagnostics_once(&e),
            }
        }
        // TimeMessage 0x5D27B0: only while it is in the tree
        let time_path = format!("{GAME}/TimeMessage");
        if placeholder_interface || ui.widget(&time_path).is_some() {
            match world.game_time() {
                Ok(t) => {
                    let secs = t as i64;
                    let text = format!("{}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60);
                    ui.set_label(rel(&time_path), &text);
                }
                Err(e) => self.diagnostics_once(&e),
            }
        }
        if placeholder_interface {
            // TimeMessage: present only with ViewShowElapsedGameTime (0x612010)
            let show_time = ui.uvars.get_bool("ViewShowElapsedGameTime");
            ui.with_widget("Game/TimeMessage", |x| x.set_visible(show_time));
            // camera information (UVar CameraInformation)
            let cam = ui.uvars.get_bool("CameraInformation");
            ui.with_widget("Game/CameraInformationLabel", |x| x.set_visible(cam));
        }
        self.update_floaters(ui, world);
        self.update_selection(ui, world);
        self.update_actions(ui, world);
        ui.layout();
    }

    /// Interface vtable +0xD0 AddChild of a child the Game interface owns
    /// (Init `0x6182B0` keeps it in its field while detached).
    pub fn attach_interface_child(&mut self, ui: &mut Ui, name: &str) -> Result<(), String> {
        let child = self.stash_take(name).ok_or_else(|| format!("Game interface child {name} is not detached"))?;
        ui.attach(GAME, child).ok_or("Game interface is not in the UI root")?;
        Ok(())
    }
    /// `0x5EFB60(child, 0)`: RemoveChild, the Game interface keeps it.
    pub fn detach_interface_child(&mut self, ui: &mut Ui, name: &str) -> Result<(), String> {
        let child = take_child(ui, GAME, name).ok_or_else(|| format!("Game interface child {name} is not attached"))?;
        self.stash.push(child);
        Ok(())
    }
    /// `widget+0x24` of a Game interface child, attached or detached.
    pub fn interface_child_flags(&self, ui: &Ui, name: &str) -> Option<u32> {
        ui.widget(&format!("{GAME}/{name}"))
            .map(|w| w.flags)
            .or_else(|| self.stash.iter().find(|w| w.name == name).map(|w| w.flags))
    }
    /// Whether a Game interface child is attached (its +8 parent is Game).
    pub fn interface_child_attached(&self, ui: &Ui, name: &str) -> bool {
        ui.widget(&format!("{GAME}/{name}")).is_some()
    }
    /// Mutate a Game interface child, attached or detached.
    pub fn with_interface_child(&mut self, ui: &mut Ui, name: &str, f: impl FnOnce(&mut Widget)) -> bool {
        if ui.widget(&format!("{GAME}/{name}")).is_some() {
            return ui.with_widget(&format!("Game/{name}"), f);
        }
        match self.stash.iter_mut().find(|w| w.name == name) {
            Some(w) => {
                f(w);
                true
            }
            None => false,
        }
    }

    // ------------------------------------------------------------ economy

    /// `0x602D10` for one bar (`impact` = EconomicImpact with its arrays).
    fn bar_update(&mut self, ui: &mut Ui, path: &str, econ: Option<&HudEconomy>, impact: Option<&HudImpact>, show_all: bool, is_impact: bool) {
        let Some(econ) = econ else {
            ui.with_widget(rel(path), |x| x.set_visible(false));
            return;
        };
        let n = self.resources.len();
        let get = |v: &Vec<f32>, i: usize| v.get(i).copied().unwrap_or(0.0);
        let zero = HudImpact { upkeep: vec![0.0; n], cost: vec![0.0; n] };
        let imp = impact.unwrap_or(&zero);
        let upkeep_changes = imp.upkeep.iter().any(|v| *v != 0.0);
        let cost_changes = imp.cost.iter().any(|v| *v != 0.0);
        let mut net = vec![0.0f32; n];
        for i in 0..n {
            if !upkeep_changes {
                net[i] = get(&econ.production, i) - get(&econ.upkeep, i);
            } else {
                let r = self.resources[i].clone();
                let v = (get(&econ.preview_production, i) - get(&econ.upkeep, i)) + get(&imp.upkeep, i);
                net[i] = v;
                if r.limit_provider {
                    // capped by the limited resource maximum
                    net[i] = if econ.limited_max < v { econ.limited_max } else { v };
                }
                if r.kind == 1 {
                    let d = get(&imp.upkeep, i);
                    let shortfall = if d >= 0.0 || net[i] >= 0.0 {
                        let base = get(&econ.production, i) - get(&econ.upkeep, i);
                        if 0.0 < d && base < 0.0 {
                            if d < -base { d } else { -base }
                        } else {
                            0.0
                        }
                    } else if net[i] < d {
                        d
                    } else {
                        net[i]
                    };
                    for kk in 0..n {
                        if self.resources[kk].kind == 0 {
                            net[kk] += shortfall * r.purchase_cost.get(kk).copied().unwrap_or(0.0);
                        }
                    }
                }
            }
        }
        let mut vis = vec![false; n];
        for i in 0..n {
            let r = &self.resources[i];
            vis[i] = if show_all || r.limit_provider {
                true
            } else if !upkeep_changes && !cost_changes {
                false
            } else {
                let base = get(&econ.production, i) - get(&econ.upkeep, i);
                let unchanged = get(&imp.upkeep, i) == 0.0 && get(&imp.cost, i) == 0.0 && net[i] == base;
                if r.kind != 2 {
                    !unchanged
                } else {
                    let j = r.limited_by.unwrap_or(i);
                    !(unchanged && get(&imp.upkeep, j) == 0.0 && get(&imp.cost, j) == 0.0)
                }
            };
        }
        let invalid = ui.uvars.get_point3("UIColorOptionInvalid");
        for i in 0..n {
            let r = self.resources[i].clone();
            let wpath = format!("{path}/{}", r.ids);
            ui.with_widget(rel(&wpath), |x| x.set_visible(vis[i]));
            if !vis[i] || r.limit_provider {
                continue;
            }
            let vals = if is_impact { &mut self.impact_values[i] } else { &mut self.bar_values[i] };
            if r.kind == 2 {
                let j = r.limited_by.unwrap_or(i);
                let (amt, lim) = (-net[i], net[j]);
                if vals.amount != amt || vals.limit != lim {
                    vals.amount = amt;
                    vals.limit = lim;
                    vals.dirty = true;
                }
            } else {
                let stock = get(&econ.stock, i) - get(&imp.cost, i);
                if vals.amount != net[i] || vals.stock != stock {
                    vals.amount = net[i];
                    vals.stock = stock;
                    vals.dirty = true;
                }
            }
            if vals.dirty {
                vals.dirty = false;
                let icon = ui.strings.glyphs.get(r.icon.to_ascii_lowercase().as_str()).copied().map(|c| c as u32).unwrap_or(0);
                let text = if r.kind == 2 {
                    limit_text(icon, vals.amount, vals.limit, invalid)
                } else {
                    // ctor flag 1 (both bars) && type 0
                    resource_text(icon, vals.amount, invalid, r.kind == 0, vals.stock)
                };
                ui.set_label(rel(&format!("{wpath}/ResourceLabel")), &text);
            }
        }
    }

    fn diagnostics_once(&mut self, s: &str) {
        if !self.diagnostics.iter().any(|d| d == s) {
            self.diagnostics.push(s.to_string());
        }
    }

    fn update_economy(&mut self, ui: &mut Ui, world: &dyn LiveWorld) {
        let econ = match world.economy() {
            Ok(v) => v,
            Err(e) => {
                self.diagnostics_once(&e);
                return;
            }
        };
        let impact = match world.economic_impact() {
            Ok(v) => Some(v),
            Err(e) => {
                self.diagnostics_once(&e);
                None
            }
        };
        let bar = format!("{GAME}/EconomyBar");
        if ui.widget(&bar).is_some() {
            if econ.is_some() {
                ui.with_widget(rel(&bar), |x| x.set_visible(true));
            }
            self.bar_update(ui, &bar, econ.as_ref(), None, true, false);
        }
        let imp = format!("{GAME}/EconomicImpact");
        if ui.widget(&imp).is_none() {
            return;
        }
        let Some(impact) = impact else { return };
        match &impact {
            Some(i) => {
                // 0x602640 -> vt+0xE0(1): slide_economy_bar slides the bar up
                if !self.impact_active {
                    self.impact_active = true;
                    self.slide_bar(ui, true);
                }
                ui.with_widget(rel(&imp), |x| x.set_visible(true));
                self.bar_update(ui, &imp, econ.as_ref(), Some(i), false, true);
            }
            None => {
                if self.impact_active {
                    self.impact_active = false;
                    self.slide_bar(ui, false);
                }
                ui.with_widget(rel(&imp), |x| x.set_visible(false));
            }
        }
    }

    /// EconomyBar vt+0xE0 `0x602480`.
    fn slide_bar(&mut self, ui: &mut Ui, show: bool) {
        if !self.slide.init || show == self.slide.is_shown {
            return;
        }
        let bar = format!("{GAME}/EconomyBar");
        let mut time = self.slide.time;
        let mut from = if show { self.slide.hidden } else { self.slide.shown };
        if let Some(m) = ui.mover(&bar).cloned() {
            time = m.progress(ui.now_ms).min(1.0) * self.slide.time;
            from = ui.raw_position(&bar).unwrap_or(from);
            ui.remove_mover(&bar);
        }
        let to = if show { self.slide.shown } else { self.slide.hidden };
        ui.move_widget(&bar, from, to, time);
        self.slide.is_shown = show;
    }

    // ------------------------------------------------------------ floaters

    fn update_floaters(&mut self, ui: &mut Ui, world: &dyn LiveWorld) {
        let g = ui.geo();
        for (name, var, kind) in [("GroupFloater", "UIGroupFloaterIsOpen", 1), ("FactionAbilitiesFloater", "UIAbilityFloaterIsOpen", 2)] {
            let path = format!("{GAME}/{name}");
            if ui.widget(&path).is_none() {
                continue;
            }
            // ShowButton SELECTED from the UVar; 0x607A50
            let open = ui.uvars.get_bool(var);
            ui.set_flag(&format!("{path}/ShowButton"), wf::SELECTED, open);
            let content = if kind == 1 { "Groups" } else { "Abilities" };
            ui.with_widget(rel(&format!("{path}/{content}")), |x| x.set_visible(open));
            ui.with_widget(rel(&format!("{path}/EndDecal")), |x| x.set_visible(open));
        }
        // Groups 0x6065D0 (once): button i at x = i * width * 0.1
        let groups = format!("{GAME}/GroupFloater/Groups");
        if !self.groups_placed {
            if let Some(p) = ui.widget(&groups) {
                let r = p.root.outer.rect;
                let step = (r.r - r.l) / g.sx * 0.1f32;
                for i in 0..10 {
                    let path = format!("{groups}/{}", seg("GroupButton", i));
                    let y = ui.raw_position(&path).map(|p| p.1).unwrap_or(0.0);
                    ui.set_position(&path, i as f32 * step, y);
                }
                self.groups_placed = true;
            }
        }
        // group textures: assigned group art or empty_group_texture
        let empty = ui
            .store
            .node("UI/Game/floaters.tgi:GroupFloater/Groups")
            .ok()
            .and_then(|n| n.string("empty_group_texture").ok());
        match world.group_textures() {
            Ok(tex) => {
                for i in 0..10 {
                    let t = tex.get(i).cloned().flatten().or_else(|| empty.clone().map(|e| format!("/{}", e.trim_start_matches('/'))));
                    set_decal_texture(ui, &format!("{groups}/{}/GroupDecal", seg("GroupButton", i)), t);
                }
            }
            Err(e) => self.diagnostics_once(&e),
        }
        // Abilities 0x607FC0
        let ab = format!("{GAME}/FactionAbilitiesFloater/Abilities");
        if ui.widget(&ab).is_some() {
            let abilities = match world.faction_abilities() {
                Ok(v) => v,
                Err(e) => {
                    self.diagnostics_once(&e);
                    return;
                }
            };
            let have = ui.widget(&format!("{ab}/ItemPanel")).map(|p| p.children.len()).unwrap_or(0);
            if have != abilities.len() {
                ui.viewport_clear(&ab);
                if let Ok(n) = ui.store.node("UI/Game/floaters.tgi:FactionAbilitiesFloater/Abilities") {
                    let root = Arc::new(n.clone());
                    for _ in &abilities {
                        let mut it = ability_item();
                        let strings = ui.strings.clone();
                        let resolve = |v: &str| strings.resolve(v);
                        it.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
                        ui.viewport_add(&ab, it);
                    }
                }
            }
            for (i, a) in abilities.iter().enumerate() {
                let it = format!("{ab}/ItemPanel/{}", seg("Ability", i));
                set_decal_texture(ui, &format!("{it}/AbilityButton/AbilityButtonDecal"), a.texture.clone());
                ui.with_widget(rel(&format!("{it}/AbilityButton")), |x| x.command = Some(a.command.clone()));
                set_status(ui, &format!("{it}/Recharge"), a.recharge);
            }
            // 0x606820: EndDecal y = content top + end_decal_offset + item heights
            let open = ui.uvars.get_bool("UIAbilityFloaterIsOpen");
            if open && !abilities.is_empty() {
                ui.layout();
                let off = ui.store.node("UI/Game/floaters.tgi:FactionAbilitiesFloater").ok().and_then(|n| n.float("end_decal_offset").ok()).unwrap_or(0.0);
                let items: f32 = ui
                    .widget(&format!("{ab}/ItemPanel"))
                    .map(|p| p.children.iter().map(|c| (c.root.outer.rect.b - c.root.outer.rect.t) / g.sy).sum())
                    .unwrap_or(0.0);
                let top = ui.raw_position(&ab).map(|p| p.1).unwrap_or(0.0);
                let ed = format!("{GAME}/FactionAbilitiesFloater/EndDecal");
                if let Some(x) = ui.raw_position(&ed).map(|p| p.0) {
                    ui.set_position(&ed, x, top + off + items);
                }
            }
        }
    }

    // ------------------------------------------------------------ selection

    fn update_selection(&mut self, ui: &mut Ui, world: &dyn LiveWorld) {
        let od = format!("{GAME}/ControlPanel/ObjectDisplay");
        let sel = match world.selection() {
            Ok(v) => v,
            Err(e) => {
                self.diagnostics_once(&e);
                return;
            }
        };
        let want: Option<&'static str> = match &sel {
            HudSelection::Company(_) => Some("CompanyDisplay"),
            _ => None,
        };
        if want != self.selection {
            // Clear 0x624160: remove the shown display (and its owned subs)
            if let Some(cur) = self.selection.take() {
                let disp = format!("{od}/{cur}");
                for sub in ["CompanyUnits", "HQAttached", "FormationButtons", "Properties"] {
                    if let Some(x) = take_child(ui, &disp, sub) {
                        self.stash.push(x);
                    }
                }
                if let Some(x) = take_child(ui, &od, cur) {
                    self.stash.push(x);
                }
            }
            if let Some(name) = want {
                if let Some(d) = self.stash_take(name) {
                    ui.attach(&od, d);
                }
                self.selection = Some(name);
            }
        }
        match sel {
            HudSelection::Company(c) => self.company(ui, &format!("{od}/CompanyDisplay"), &c),
            HudSelection::Unsupported(what) => self.diagnostics_once(&format!("unsupported selection display: {what}")),
            HudSelection::None => {}
        }
    }

    fn ensure_sub(&mut self, ui: &mut Ui, disp: &str, name: &str, on: bool) {
        let present = ui.widget(&format!("{disp}/{name}")).is_some();
        if on && !present {
            if let Some(x) = self.stash_take(name) {
                ui.attach(disp, x);
            }
        } else if !on && present {
            if let Some(x) = take_child(ui, disp, name) {
                self.stash.push(x);
            }
        }
    }

    /// SetActor `0x62EF20` + Refresh `0x62E3B0`.
    fn company(&mut self, ui: &mut Ui, d: &str, c: &CompanyInfo) {
        let g = ui.geo();
        set_model(ui, &format!("{d}/BannerButton/Banner"), c.banner_model.clone());
        ui.with_widget(rel(&format!("{d}/BannerButton")), |x| x.command = Some("CommandAll".into()));
        // leader portrait; no leader -> health hidden (0x623910)
        set_model(ui, &format!("{d}/PortraitButton/Portrait"), c.portrait_model.clone());
        let has_leader = c.portrait_model.is_some();
        ui.with_widget(rel(&format!("{d}/Health")), |x| x.set_visible(has_leader));
        if has_leader {
            set_status(ui, &format!("{d}/Health"), c.health);
        }
        ui.set_label(rel(&format!("{d}/Name")), &c.name);
        let owner = c.owner.as_ref().map(|o| owner_text(ui, o)).unwrap_or_default();
        ui.set_label(rel(&format!("{d}/Owner")), &owner);
        set_status(ui, &format!("{d}/Morale"), c.morale.0);
        set_target(ui, &format!("{d}/Morale"), c.morale.1);
        let sp = c.supply;
        ui.with_widget(rel(&format!("{d}/SupplyStockpile")), |x| x.set_visible(sp.is_some()));
        set_status(ui, &format!("{d}/SupplyStockpile"), sp.unwrap_or(0.0));
        set_status(ui, &format!("{d}/AttackEfficiency"), c.attack_efficiency.0);
        set_target(ui, &format!("{d}/AttackEfficiency"), c.attack_efficiency.1);
        let econ = match &c.economy {
            Some(v) => economy_text(ui, &self.resources, v),
            None => String::new(),
        };
        ui.set_label(rel(&format!("{d}/Economy")), &econ);
        ui.set_label(rel(&format!("{d}/State")), c.state.as_deref().unwrap_or(""));
        // HQ attachments or company units
        let is_hq = c.hq.is_some();
        self.ensure_sub(ui, d, "HQAttached", is_hq);
        self.ensure_sub(ui, d, "CompanyUnits", !is_hq);
        self.ensure_sub(ui, d, "FormationButtons", !c.formations.is_empty());
        self.ensure_sub(ui, d, "Properties", true);
        ui.layout();
        if let Some(hq) = &c.hq {
            self.hq_layout(ui, &format!("{d}/HQAttached"), hq, g);
        } else {
            self.units_layout(ui, &format!("{d}/CompanyUnits"), c, g);
        }
        self.formations(ui, &format!("{d}/FormationButtons"), &c.formations, g);
        self.properties(ui, &format!("{d}/Properties"), &c.properties, g);
    }

    /// HQAttached OnConnected `0x62D010` grid (3 x 2, steps floor((W - w)
    /// * 0.5) and floor(H - h)) and content `0x62E0F0` (slots past the
    /// attachments hidden).
    fn hq_layout(&mut self, ui: &mut Ui, p: &str, hq: &[Option<HqSlot>], g: k::Geo) {
        let Some(panel) = ui.widget(p) else { return };
        let r = panel.root.outer.rect;
        let s0 = panel.children.first().map(|s| s.root.outer.rect).unwrap_or(r);
        let dx = (((r.r - r.l) / g.sx - (s0.r - s0.l) / g.sx) * 0.5).floor();
        let dy = ((r.b - r.t) / g.sy - (s0.b - s0.t) / g.sy).floor();
        for row in 0..2 {
            for col in 0..3 {
                let i = row * 3 + col;
                ui.set_position(&format!("{p}/{}", seg("HQAttachedOrganization", i)), col as f32 * dx, row as f32 * dy);
            }
        }
        for i in 0..6 {
            let sp = format!("{p}/{}", seg("HQAttachedOrganization", i));
            let shown = i < hq.len().min(6);
            ui.with_widget(rel(&sp), |x| x.set_visible(shown));
            let slot = hq.get(i).cloned().flatten();
            set_decal_texture(ui, &format!("{sp}/Button/ButtonDecal"), slot.as_ref().and_then(|s| s.texture.clone()));
            let has = slot.is_some();
            // 0x62D9D0(null) -> 0x62D440: an empty slot shows no content
            ui.with_widget(rel(&format!("{sp}/Button/ButtonDecal")), |x| x.set_visible(has));
            ui.with_widget(rel(&format!("{sp}/Morale")), |x| x.set_visible(has));
            ui.with_widget(rel(&format!("{sp}/Health")), |x| x.set_visible(has));
            if let Some(s) = slot {
                set_status(ui, &format!("{sp}/Health"), s.health);
                set_status(ui, &format!("{sp}/Morale"), s.morale);
            }
        }
    }

    /// CompanyUnits layout `0x62F690` with the UnitBorderDecal frames around
    /// each position group, and the unit content `0x62D520` / `0x6259D0`.
    fn units_layout(&mut self, ui: &mut Ui, p: &str, c: &CompanyInfo, g: k::Geo) {
        let Ok(node) = ui.store.node("UI/Game/object_display.tgi:ObjectDisplay/CompanyDisplay/CompanyUnits") else { return };
        let f = |k: &str| node.float(k).unwrap_or(0.0);
        let (hsep, vsep, dsep, bh, bv) = (
            f("horizontal_unit_separation"),
            f("vertical_unit_separation"),
            f("definition_separation"),
            f("border_horizontal_unit_separation"),
            f("border_vertical_unit_separation"),
        );
        let u0 = format!("{p}/{}", seg("CompanyUnit", 0));
        let Some(r0) = ui.widget(&u0).map(|x| x.root.outer.rect) else { return };
        let (uw, uh) = ((r0.r - r0.l) / g.sx, (r0.b - r0.t) / g.sy);
        let prec = ui.precision;
        // 0x5E28E0: rect * f32(1 / scale), relative to the parent's
        let rel_rect = |ui: &Ui, path: &str| -> Option<[f32; 4]> {
            let r = ui.widget(path)?.root.outer.rect;
            let pr = ui.widget(p)?.root.outer.rect;
            let ix = prec.div(1.0, g.sx as f64) as f32 as f64;
            let iy = prec.div(1.0, g.sy as f64) as f32 as f64;
            let m = |v: f32, s: f64| prec.mul(v as f64, s) as f32 as f64;
            Some([
                prec.sub(m(r.l, ix), m(pr.l, ix)) as f32,
                prec.sub(m(r.t, iy), m(pr.t, iy)) as f32,
                prec.sub(m(r.r, ix), m(pr.l, ix)) as f32,
                prec.sub(m(r.b, iy), m(pr.t, iy)) as f32,
            ])
        };
        let slots = &c.slots;
        let n = slots.len().min(9);
        let mut x = bh;
        let mut first_in_col = true;
        let mut new_col = true;
        let mut prev: usize = 0;
        let mut needed = 1usize;
        let mut used = 0usize;
        let (mut min_l, mut min_t, mut max_r, mut max_b) = (1.0e6f32, 1.0e6f32, 0.0f32, 0.0f32);
        let have = |ui: &Ui| ui.widget(p).map(|w| w.children.iter().filter(|c| c.name == "UnitBorderDecal").count()).unwrap_or(0);
        for i in 0..n {
            let up = format!("{p}/{}", seg("CompanyUnit", i));
            if slots[i].unit.is_none() {
                continue;
            }
            if i as i32 == c.leader_slot {
                ui.with_widget(rel(&up), |w| w.set_visible(false));
                continue;
            }
            if i == 0 || slots[i].position != slots[prev].position {
                if slots[i].position != slots[prev].position && slots[prev].definition != slots[i].definition {
                    x += dsep;
                    new_col = true;
                }
                // a border per position group, created on demand and
                // added at the head (vtable+0xD4: drawn behind the units)
                if have(ui) < needed {
                    let mut bd = Widget::new(C::Decal, "UnitBorderDecal", false);
                    let strings = ui.strings.clone();
                    let resolve = |v: &str| strings.resolve(v);
                    let root = Arc::new(node.clone());
                    bd.load(&mut ui.store, Some((&node, &root)), Some(10), &resolve);
                    if let Some(pw) = ui.widget_mut(p) {
                        pw.add_child_front(bd);
                    }
                    ui.layout();
                }
            }
            let (ul, ut, ur, ub);
            if new_col {
                ui.set_position(&up, x, bv);
                if i + 1 < slots.len() && slots[i].position != slots[i + 1].position && slots[i].definition != slots[i + 1].definition {
                    x = uw + hsep + x;
                }
                new_col = false;
                ui.layout();
                let r = rel_rect(ui, &up).unwrap_or_default();
                (ul, ut, ur, ub) = (r[0], r[1], r[2], r[3]);
                min_l = if min_l >= ul { ul } else { min_l };
                min_t = if min_t >= ut { ut } else { min_t };
            } else {
                ui.set_position(&up, x, vsep + bv + uh);
                let mut xt = x;
                if first_in_col {
                    xt -= bh;
                    first_in_col = false;
                }
                x = uw + hsep + xt;
                new_col = true;
                ui.layout();
                let r = rel_rect(ui, &up).unwrap_or_default();
                (ul, ut, ur, ub) = (r[0], r[1], r[2], r[3]);
                if slots[i].position != slots[prev].position {
                    min_l = if min_l >= ul { ul } else { min_l };
                    min_t = if min_t >= ut { ut } else { min_t };
                }
            }
            max_r = if max_r <= ur { ur } else { max_r };
            max_b = if max_b <= ub { ub } else { max_b };
            if i + 1 < slots.len() && slots[i + 1].position != slots[i].position {
                // the group's border: (minL - bh, minT - bv), size
                // (maxR - minL + 2bh, maxB - minT + 2bv)
                let bw = prec.add(prec.sub(max_r as f64, min_l as f64), prec.add(bh as f64, bh as f64)) as f32;
                let bhh = prec.add(prec.sub(max_b as f64, min_t as f64), prec.add(bv as f64, bv as f64)) as f32;
                let bx = prec.sub(min_l as f64, bh as f64) as f32;
                let by = prec.sub(min_t as f64, bv as f64) as f32;
                let bp = format!("{p}/{}", seg("UnitBorderDecal", used));
                ui.with_widget(rel(&bp), |w| {
                    k::set_bounds(&mut w.root.outer, [bx, by, bw, bhh], 0x300);
                    w.set_visible(true);
                });
                used += 1;
                needed += 1;
                if !first_in_col {
                    first_in_col = true;
                    x += bh + bh;
                }
                (min_l, min_t, max_r, max_b) = (1.0e6, 1.0e6, 0.0, 0.0);
            }
            // content 0x62D520 / 0x6259D0: Background, or AltBackground
            // when the slot asks for it
            let u = slots[i].unit.clone().unwrap();
            ui.with_widget(rel(&up), |w| {
                w.set_visible(true);
                if let Some(v) = &mut w.view {
                    for ch in &mut v.children {
                        let vis = crate::layout::ve_flag::VISIBLE;
                        if ch.name.eq_ignore_ascii_case("Background") {
                            if u.alt_background { ch.layout.flags &= !vis } else { ch.layout.flags |= vis }
                        } else if ch.name.eq_ignore_ascii_case("AltBackground") {
                            if u.alt_background { ch.layout.flags |= vis } else { ch.layout.flags &= !vis }
                        }
                    }
                }
            });
            set_decal_texture(ui, &format!("{up}/UnitButton/UnitButtonDecal"), u.texture.clone());
            set_status(ui, &format!("{up}/Health"), u.health);
            prev = i;
        }
        // borders not used by this layout (INFERRED: hidden)
        for j in used..have(ui) {
            ui.with_widget(rel(&format!("{p}/{}", seg("UnitBorderDecal", j))), |w| w.set_visible(false));
        }
        for i in n..9 {
            ui.with_widget(rel(&format!("{p}/{}", seg("CompanyUnit", i))), |w| w.set_visible(false));
        }
        for i in 0..n {
            if slots[i].unit.is_none() {
                ui.with_widget(rel(&format!("{p}/{}", seg("CompanyUnit", i))), |w| w.set_visible(false));
            }
        }
    }

    /// FormationButtons `0x628490` + OnConnected `0x623D30` (y = i *
    /// floor(H / 3)).
    fn formations(&mut self, ui: &mut Ui, p: &str, forms: &[Formation], g: k::Geo) {
        let Some(panel) = ui.widget(p) else { return };
        let r = panel.root.outer.rect;
        let step = ((r.b - r.t) / g.sy * (1.0f32 / 3.0)).floor();
        for (i, f) in forms.iter().take(3).enumerate() {
            let bp = format!("{p}/{}", seg("FormationButton", i));
            let x = ui.raw_position(&bp).map(|p| p.0).unwrap_or(0.0);
            ui.set_position(&bp, x, i as f32 * step);
            ui.with_widget(rel(&bp), |w| {
                w.set_visible(true);
                w.command = Some(format!("FormationChange {} ", f.id));
            });
            ui.set_flag(&bp, wf::SELECTED, f.selected);
            set_decal_texture(ui, &format!("{bp}/FormationButtonDecal"), Some(f.texture.clone()));
        }
        for i in forms.len().min(3)..3 {
            ui.with_widget(rel(&format!("{p}/{}", seg("FormationButton", i))), |w| w.set_visible(false));
        }
    }

    /// UnitProperties OnConnected `0x629620` (x = i * W * 0.125 + i *
    /// property_separation, separation capped at 2) and the bars shown for
    /// the actor's properties (`0x627240`).
    fn properties(&mut self, ui: &mut Ui, p: &str, props: &[String], g: k::Geo) {
        let Some(panel) = ui.widget(p) else { return };
        let r = panel.root.outer.rect;
        let sep = ui
            .store
            .node("UI/Game/object_display.tgi:ObjectDisplay/CompanyDisplay/Properties")
            .ok()
            .and_then(|n| n.float("property_separation").ok())
            .unwrap_or(0.0);
        let sep = if sep > 2.0 { 2.0 } else { sep };
        let step = (r.r - r.l) / g.sx * 0.125f32;
        for i in 0..7 {
            let bp = format!("{p}/{}", seg("PropertyBar", i));
            let y = ui.raw_position(&bp).map(|p| p.1).unwrap_or(0.0);
            ui.set_position(&bp, i as f32 * step + i as f32 * sep, y);
            let t = props.get(i).cloned();
            ui.with_widget(rel(&bp), |w| w.set_visible(t.is_some()));
            set_decal_texture(ui, &bp, t);
        }
        let more = props.len() > 7;
        ui.with_widget(rel(&format!("{p}/PropertyMoreIndicator")), |w| w.set_visible(more));
    }

    // ------------------------------------------------------------ actions

    fn update_actions(&mut self, ui: &mut Ui, world: &dyn LiveWorld) {
        let ab = format!("{GAME}/ControlPanel/ActionButtons");
        let acts = match world.action_buttons() {
            Ok(v) => v,
            Err(e) => {
                self.diagnostics_once(&e);
                return;
            }
        };
        for i in 0..12 {
            let bp = format!("{ab}/{}", seg("ActionButtonsButton", i));
            let a = acts.get(i).cloned().flatten();
            match a {
                Some(a) => {
                    ui.with_widget(rel(&bp), |w| {
                        w.set_visible(true);
                        w.set_enabled(a.enabled);
                        w.command = Some(a.command.clone());
                    });
                    ui.set_flag(&bp, wf::SELECTED, a.selected);
                    ui.set_flag(&bp, wf::TARGETED, a.targeted);
                    // decal inset in pixels: action_button_(un)pressed_client_*
                    // (+0x118.. / +0x14C..); pressed while selected
                    // (INFERRED: the state hook was not traced)
                    let key = if a.selected { "action_button_pressed_client_" } else { "action_button_unpressed_client_" };
                    let node = ui.store.node("UI/Game/action_buttons.tgi:ActionButtons/ActionButtonsButton").ok();
                    let px = |s: &str| node.as_ref().and_then(|n| n.int(&format!("{key}{s}")).ok()).unwrap_or(0) as f32;
                    let g = ui.geo();
                    let (l, t, r, b) = (px("L") / g.sx, px("T") / g.sy, px("R") / g.sx, px("B") / g.sy);
                    ui.with_widget(rel(&bp), |w| {
                        if let Some(d) = &mut w.sub_ve {
                            if let crate::ve::VeKind::Decal(dd) = &mut d.kind {
                                dd.texture = Some(a.texture.clone());
                            }
                            crate::layout::parse_bounds(&mut d.layout.outer, &format!("{l}"), &format!("{t}"), &format!("{r}r"), &format!("{b}b"));
                        }
                    });
                }
                None => {
                    ui.with_widget(rel(&bp), |w| w.set_visible(false));
                }
            }
        }
    }
}

fn rel(path: &str) -> &str {
    path.strip_prefix("Root/").unwrap_or(path)
}

/// Path segment of the i-th child named `name` (`child_segments`).
fn seg(name: &str, i: usize) -> String {
    if i == 0 { name.to_string() } else { format!("{name}#{i}") }
}

fn take_child(ui: &mut Ui, parent: &str, name: &str) -> Option<Widget> {
    let path = format!("{parent}/{name}");
    ui.widget(&path)?;
    ui.deactivate(&path);
    let p = ui.widget_mut(parent)?;
    let i = p.children.iter().position(|c| c.name.trim_start_matches('/').eq_ignore_ascii_case(name))?;
    Some(p.children.remove(i))
}

fn set_decal_texture(ui: &mut Ui, path: &str, tex: Option<String>) {
    ui.with_widget(rel(path), |w| {
        if let Some(ve) = &mut w.sub_ve {
            if let crate::ve::VeKind::Decal(d) = &mut ve.kind {
                d.texture = tex;
            }
        }
    });
}

fn set_model(ui: &mut Ui, path: &str, model: Option<String>) {
    ui.with_widget(rel(path), |w| {
        w.control = model.map(|m| Box::new(k::Control::Model(m)));
    });
}

/// `0x5DF010`.
pub fn set_status(ui: &mut Ui, path: &str, v: f32) {
    ui.with_widget(rel(path), |w| {
        if let Some(ve) = &mut w.sub_ve {
            if let crate::ve::VeKind::StatusBar(s) = &mut ve.kind {
                s.value = v;
                s.value = s.clamped();
            }
        }
    });
}

/// `0x5DF060`: the TargetPercentage decal at x = width * t - decal width.
pub fn set_target(ui: &mut Ui, path: &str, t: f32) {
    let g = ui.geo();
    let t = if t < 0.0 { 0.0 } else if t > 1.0 { 1.0 } else { t };
    let Some(wd) = ui.widget(path).map(|w| (w.root.outer.rect.r - w.root.outer.rect.l) / g.sx) else { return };
    let tp = format!("{path}/TargetPercentage");
    let Some(dw) = ui.widget(&tp).map(|w| (w.root.outer.rect.r - w.root.outer.rect.l) / g.sx) else { return };
    let y = ui.raw_position(&tp).map(|p| p.1).unwrap_or(0.0);
    ui.set_position(&tp, wd * t - dw, y);
}

/// `%g` of a double (MSVC: 6 significant digits).
fn g6(v: f64) -> String {
    crate::staging::fmt_g(v)
}

fn wrap(text: &str, rgb: [f32; 3]) -> String {
    crate::text::color_wrap(text, rgb)
}

/// `0x5C9830` (the flag = ctor arg && stockpiled).
pub fn resource_text(icon: u32, amount: f32, invalid: [f32; 3], with_stock: bool, stock: f32) -> String {
    let mut s = format!("{:+.0}", amount as f64);
    if amount < 0.0 {
        s = wrap(&s, invalid);
    }
    let ic = char::from_u32(icon).unwrap_or('\0');
    if !with_stock {
        return format!("{ic} {s}");
    }
    let st = (stock as f64).floor() as f32;
    let mut t = format!("{:.0}", st as f64);
    if st < 0.0 {
        t = wrap(&t, invalid);
    }
    format!("{ic} {t}({s})")
}

/// `0x5C8F10`.
pub fn limit_text(icon: u32, amount: f32, limit: f32, invalid: [f32; 3]) -> String {
    let ic = char::from_u32(icon).unwrap_or('\0');
    // no consumption gives -0 (the amount is -net): print 0
    let amount = amount + 0.0;
    let mut a = format!("{:.0}", amount as f64);
    if limit < amount {
        a = wrap(&a, invalid);
    }
    format!("{ic}{a}/{:.0} ", limit as f64)
}

/// `0x4CE3A0` with the glyph AVar FontFactionExperienceGlyphIndex.
fn experience_text(ui: &Ui, fe: &FactionExperience) -> String {
    let glyph = ui.avars.float("FontFactionExperienceGlyphIndex").map(|v| v as u32).unwrap_or(0);
    let invalid = ui.uvars.get_point3("UIColorOptionInvalid");
    let mut v = format!("{:.0}", fe.value as f64);
    if fe.value < 0.0 {
        v = wrap(&v, invalid);
    }
    let mut s = match char::from_u32(glyph).filter(|_| glyph != 0) {
        Some(c) => format!("{c} {v}"),
        None => v,
    };
    if fe.max >= 0.0 {
        s.push_str(&format!("/{:.0}", fe.max as f64));
    }
    s
}

/// `0x4E9560` with the `0x4E93C0` prefix.
pub fn owner_text(ui: &Ui, o: &OwnerInfo) -> String {
    let c = o.kingdom_color.map(|v| g6(v as f64));
    let default = ui
        .avars
        .get("KingdomGlyphInfo")
        .and_then(|ids| ui.strings.glyphs.get(&ids.to_ascii_lowercase()).copied())
        .map(|c| c as u32)
        .unwrap_or(0);
    let glyph = char::from_u32(o.rank_glyph.unwrap_or(default)).unwrap_or('\0');
    let prefix = format!("<color={},{},{}>{glyph}", c[0], c[1], c[2]);
    match o.relation_color {
        Some(r) => {
            let r = r.map(|v| g6(v as f64));
            format!("{prefix}<color={},{},{}> {}", r[0], r[1], r[2], o.name)
        }
        None => format!("{prefix} <rc>{}", o.name),
    }
}

/// `0x5C4CB0` / `0x5C4BD0`: every non-zero value, "%lc%+g%s" (type 2:
/// "%lc%.0f%s" of the magnitude) with the separator "  ".
fn economy_text(ui: &Ui, res: &[HudResource], values: &[f32]) -> String {
    let mut out = String::new();
    for (i, r) in res.iter().enumerate() {
        let v = values.get(i).copied().unwrap_or(0.0);
        if v == 0.0 {
            continue;
        }
        let icon = ui.strings.glyphs.get(r.icon.to_ascii_lowercase().as_str()).copied().unwrap_or('\0');
        if r.kind != 2 {
            let g = g6(v as f64);
            let g = if v > 0.0 { format!("+{g}") } else { g };
            out.push_str(&format!("{icon}{g}  "));
        } else {
            out.push_str(&format!("{icon}{:.0}  ", v.abs() as f64));
        }
    }
    out
}

// ------------------------------------------------------------ placeholder

/// Which original screenshot (qa-output/original-ui) a placeholder world
/// reproduces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceholderScene {
    /// 04-ingame: nothing selected.
    Ingame,
    /// 05-regiment: "1st Infantry Regiment".
    Regiment,
    /// 06-corps: "1st Infantry Division HQ".
    Corps,
    /// 07-recruit: the HQ recruit page with the first item hovered.
    Recruit,
    /// 09..13: the HQ recruit page under the game menu (no hover).
    RecruitMenu,
}

/// PLACEHOLDER game data for layout checks and screenshots only: the
/// values are read off the original screenshots, the textures and models
/// are stand-ins where the game would supply them. No rules are derived
/// from it.
pub struct PlaceholderWorld {
    pub scene: PlaceholderScene,
}

type ResRow = (&'static str, u8, &'static str, Option<usize>, bool, f32);

const RES: [ResRow; 7] = [
    ("money", 0, "ch_MONEY", None, false, 0.0),
    ("ammo", 1, "ch_AMMO", None, false, 2.0),
    ("oil", 1, "ch_FUEL", None, false, 3.0),
    ("unit_limit_provided", 1, "ch_UNITLIMIT", None, true, 0.0),
    ("unit_limit_consumed", 2, "ch_UNITLIMIT", Some(3), false, 0.0),
    ("building_limit_provided", 1, "ch_building", None, true, 0.0),
    ("building_limit_consumed", 2, "ch_building", Some(5), false, 0.0),
];

impl PlaceholderWorld {
    fn unit(tex: &str) -> UnitIcon {
        UnitIcon { texture: Some(format!("/UI/%s/Game/RegimentButtons/{tex}")), health: 1.0, alt_background: false }
    }

    fn act(tex: &str, cmd: &str) -> Option<HudAction> {
        Some(HudAction { texture: tex.to_string(), command: cmd.to_string(), enabled: true, selected: false, targeted: false })
    }

    fn owner() -> OwnerInfo {
        OwnerInfo { name: "Player".into(), rank_glyph: None, kingdom_color: [0.0, 0.2, 1.0], relation_color: Some([0.0, 1.0, 0.0]) }
    }

    fn hq() -> CompanyInfo {
        CompanyInfo {
            name: "1st Infantry Division HQ".into(),
            owner: Some(Self::owner()),
            portrait_model: Some("placeholder:general".into()),
            banner_model: Some("placeholder:hq_banner".into()),
            health: 1.0,
            morale: (1.0, 0.1),
            attack_efficiency: (1.0, 1.0),
            supply: Some(1.0),
            economy: Some(vec![30.0, 10.0, 5.0, 5.0, 0.0, 0.0, -1.0]),
            state: Some("Resting".into()),
            slots: Vec::new(),
            leader_slot: -1,
            hq: Some(vec![
                Some(HqSlot { texture: Some("/UI/%s/Game/ObjectDisplay/OrganizationRetreat.png".into()), health: 1.0, morale: 1.0 }),
                None,
                None,
                None,
                None,
            ]),
            formations: Vec::new(),
            properties: Vec::new(),
        }
    }
}

impl HudWorld for PlaceholderWorld {
    fn resources(&self) -> Vec<HudResource> {
        RES.iter()
            .map(|(ids, kind, icon, lim, prov, cost)| HudResource {
                ids: ids.to_string(),
                kind: *kind,
                icon: icon.to_string(),
                limited_by: *lim,
                limit_provider: *prov,
                purchase_cost: if *kind == 1 { vec![*cost, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] } else { vec![0.0; 7] },
            })
            .collect()
    }

    fn economy(&self) -> Option<HudEconomy> {
        let stock = match self.scene {
            PlaceholderScene::Ingame => 935.0,
            PlaceholderScene::Regiment => 957.0,
            PlaceholderScene::Corps => 959.0,
            PlaceholderScene::Recruit | PlaceholderScene::RecruitMenu => 989.0,
        };
        // net: money +60, ammo +20, oil +15, unit limit 5 / 1, building
        // limit 100 / 2
        let production = vec![60.0, 20.0, 15.0, 5.0, 0.0, 100.0, 0.0];
        let upkeep = vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0];
        Some(HudEconomy {
            preview_production: production.clone(),
            production,
            upkeep,
            stock: vec![stock, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            limited_max: 100.0,
        })
    }

    fn economic_impact(&self) -> Option<HudImpact> {
        // 07: recon infantry regiment, cost 25, upkeep ammo -5 and 1 unit
        (self.scene == PlaceholderScene::Recruit).then(|| HudImpact {
            upkeep: vec![0.0, -5.0, 0.0, 0.0, -1.0, 0.0, 0.0],
            cost: vec![25.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        })
    }

    fn faction_experience(&self) -> Option<FactionExperience> {
        let value = if self.scene == PlaceholderScene::RecruitMenu { 1093.0 } else { 1021.0 };
        Some(FactionExperience { value, max: 5000.0 })
    }

    fn faction_abilities(&self) -> Vec<HudAbility> {
        ["IconAtomicBomb.png", "IconResupply.png", "IconMechanizedAdvance.png", "IconAssemblyLine.png"]
            .iter()
            .enumerate()
            .map(|(i, t)| HudAbility {
                texture: Some(format!("/UI/%s/Game/SpecialOps/{t}")),
                command: format!("PLACEHOLDER ability {i}"),
                recharge: 1.0,
            })
            .collect()
    }

    fn group_textures(&self) -> Vec<Option<String>> {
        let mut v = vec![None; 10];
        v[0] = Some("/UI/%s/Game/RegimentButtons/IconInfantry.png".into());
        v
    }

    fn idle_builder(&self) -> bool {
        true
    }

    fn idle_detached_organization(&self) -> bool {
        false
    }

    fn game_time(&self) -> f32 {
        0.0
    }

    fn selection(&self) -> HudSelection {
        match self.scene {
            PlaceholderScene::Ingame => HudSelection::None,
            PlaceholderScene::Regiment => {
                let mut slots = Vec::new();
                // six riflemen (one definition) and two machine gunners
                // (a second definition): 3 + 1 columns of two
                for i in 0..8 {
                    let tex = if i >= 6 { "IconInfantryHeavy.png" } else { "IconInfantry.png" };
                    slots.push(LayoutSlot { position: i / 2, definition: if i >= 6 { 1 } else { 0 }, unit: Some(Self::unit(tex)) });
                }
                HudSelection::Company(Box::new(CompanyInfo {
                    name: "1st Infantry Regiment".into(),
                    owner: Some(Self::owner()),
                    portrait_model: Some("placeholder:rifleman".into()),
                    banner_model: Some("placeholder:regiment_banner".into()),
                    health: 1.0,
                    morale: (1.0, 0.1),
                    attack_efficiency: (1.0, 1.0),
                    supply: None,
                    economy: Some(vec![0.0, -10.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
                    state: Some("Entrenched".into()),
                    slots,
                    leader_slot: -1,
                    hq: None,
                    formations: (0..3)
                        .map(|i| Formation {
                            id: i,
                            texture: "/UI/%s/Game/SharedTextures/FormationButtonUnfocused.tga".into(),
                            selected: i == 1,
                        })
                        .collect(),
                    properties: vec![
                        "/UI/%s/Game/ActionButtons/IconGuard.png".into(),
                        "/UI/%s/Game/SpecialOps/IconSandBagging.png".into(),
                    ],
                }))
            }
            PlaceholderScene::Corps | PlaceholderScene::Recruit | PlaceholderScene::RecruitMenu => {
                HudSelection::Company(Box::new(Self::hq()))
            }
        }
    }

    fn action_buttons(&self) -> Vec<Option<HudAction>> {
        let a = |t: &str| Self::act(&format!("/UI/%s/Game/ActionButtons/{t}"), "PLACEHOLDER");
        let r = |t: &str| Self::act(&format!("/UI/%s/Game/RegimentButtons/{t}"), "PLACEHOLDER");
        let mut v: Vec<Option<HudAction>> = vec![None; 12];
        match self.scene {
            PlaceholderScene::Ingame => {
                v[8] = a("IconTeamCommands.png");
            }
            PlaceholderScene::Regiment => {
                for (i, t) in [
                    (0, "IconRecall.png"),
                    (1, "IconAttack.png"),
                    (2, "IconSallyForth.png"),
                    (3, "IconStop.png"),
                    (6, "IconGuard.png"),
                    (7, "IconExplore.png"),
                    (8, "IconTeamCommands.png"),
                    (10, "IconAttach.png"),
                    (11, "IconDetach.png"),
                ] {
                    v[i] = a(t);
                }
            }
            PlaceholderScene::Corps => {
                v[0] = Self::act("/UI/%s/Game/SpecialOps/IconCorps.png", "PLACEHOLDER");
                v[4] = a("IconCommandAll.png");
                v[5] = a("IconSetRallyPoint.png");
                v[8] = a("IconTeamCommands.png");
                v[9] = a("IconRecruitLocation.png");
                v[11] = a("IconSell.png");
            }
            PlaceholderScene::Recruit | PlaceholderScene::RecruitMenu => {
                let icons = [
                    "IconInfantryRecon.png",
                    "IconInfantry.png",
                    "IconInfantryAT.png",
                    "IconInfantryArtillery.png",
                    "IconInfantryMechanized.png",
                    "IconInfantryHeavy.png",
                    "IconInfantryAssault.png",
                ];
                for (i, t) in icons.iter().enumerate() {
                    v[i] = r(t);
                }
                if self.scene == PlaceholderScene::Recruit {
                    if let Some(x) = v[0].as_mut() {
                        x.targeted = true;
                    }
                }
                v[11] = a("IconCancel.png");
            }
        }
        v
    }
}
