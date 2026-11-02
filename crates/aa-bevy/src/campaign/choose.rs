//! ChooseDynamicCampaign (KKC_ChooseDynamicCampaignInterface,
//! ChooseDynamicCampaignInterface.cpp): the campaign (side) and general
//! chooser of the World War II campaign.
//!
//! Widgets in AddChild order (CreateChildren 942AE0):
//! - `MenuButtonPanel` (5D28B0) with `NewButton` (DynamicCampaignNew),
//!   `LoadButton` (DynamicCampaignLoad) and `ExitButton` (DynamicCampaignExit);
//! - `/DynamicCampaignButtons` (942A00 -> 942070): per campaign the toggle
//!   `DynamicCampaignButton%d` and the labelled toggle `DynamicCampaignLabel%d`,
//!   both "SetDynamicCampaign <ids>", each list a radio group (5F0420);
//!   LoadData 941B80 sets the button decals from the campaign's
//!   `ChooseButtonVE` list by `when` (9419D0 -> 95BE80) and the label text to
//!   the campaign name;
//! - `/DynamicCampaignGeneralButtons` (942A70 -> 942550): as many labelled
//!   toggles `DynamicCampaignGeneralButton%d` ("SetGeneral %d", hidden, one
//!   radio group) as the largest public faction count of any campaign's
//!   default-kingdom nation (473110), then the `DynamicCampaignGeneralPortrait`
//!   cinematic;
//! - `/GlobeBink` and `/NationSelectionBackground` cinematics (sort key
//!   -FLT_MAX, drawn first).
//!
//! SetDynamicCampaign (9432A0 -> 943200): the campaign becomes current
//! (+0xF0), its button and label are selected (941BF0), the general buttons
//! show its nation's public factions sorted by 473320 (wcsicmp of the
//! faction's staging name) with their names (942F80), and the nation video
//! plays. SetGeneral (941A60) picks a faction (+0xA0) and its portrait
//! video. The campaign's nation and general videos are not baked for the video
//! player (ui/video.rs): those cinematics are hidden.
use super::data::{CampaignData, FactionView};
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};

pub const FILE: &str = "MetaGame/UI/choose_dynamic_campaign.tgi";
const IFACE: &str = "ChooseDynamicCampaign";

/// What a chooser command asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    None,
    Exit,
    New { campaign: usize, faction: Option<String> },
    /// DynamicCampaignLoad 932A00: the saved campaigns' dialog.
    Load,
}

#[derive(Default)]
pub struct Chooser {
    /// +0xF0: the current campaign.
    pub campaign: usize,
    /// +0xA0 of the general buttons: index into `factions`.
    pub faction: usize,
    /// 942F80: the current campaign nation's public factions, sorted.
    pub factions: Vec<FactionView>,
    /// The page was built with the campaign data (else a rebuild follows).
    built_with_data: bool,
}

fn decal_textures(ve: &mut aa_ui::ve::ViewElement, buttons: &[super::data::ChooseVe]) {
    if let aa_ui::ve::VeKind::Decal(d) = &mut ve.kind {
        // 95BE80: the ChooseButtonVE whose `when` equals the decal's
        if let Some(b) = buttons.iter().find(|b| aa_ui::state::parse_when(&b.when).bits == ve.when) {
            d.texture = Some(format!("/MetaGame/{}", b.texture.trim_start_matches('/')));
        }
    }
    for c in &mut ve.children {
        decal_textures(c, buttons);
    }
}

impl Chooser {
    /// RequestDynamicCampaign: a fresh chooser on the first campaign.
    pub fn enter(&mut self, _ui: &mut Ui) {
        self.campaign = 0;
        self.faction = 0;
        self.factions.clear();
        self.built_with_data = false;
    }

    /// The page needs a rebuild now that the data is here.
    pub fn data_arrived(&mut self, _ui: &mut Ui, _data: &CampaignData) -> bool {
        !self.built_with_data
    }

    /// 473110 over every campaign: the largest public faction count of a
    /// campaign's default-kingdom nation.
    fn general_count(data: &CampaignData) -> usize {
        data.meta
            .campaigns
            .iter()
            .filter_map(|c| c.kingdoms.get(c.default_kingdom))
            .filter_map(|k| data.nation(&k.nation_ids))
            .map(|n| n.factions.iter().filter(|f| f.public).count())
            .max()
            .unwrap_or(0)
    }

    fn tree(data: Option<&CampaignData>) -> Widget {
        let mut iface = Widget::new(C::Interface, &format!("{FILE}:{IFACE}"), false);
        let mut menu = Widget::new(C::Panel, "MenuButtonPanel", false);
        menu.add_child(Widget::push_button("NewButton", false, "DynamicCampaignNew"));
        menu.add_child(Widget::push_button("LoadButton", false, "DynamicCampaignLoad"));
        menu.add_child(Widget::push_button("ExitButton", false, "DynamicCampaignExit"));
        iface.add_child(menu);
        let mut buttons = Widget::new(C::Panel, "/DynamicCampaignButtons", false);
        if let Some(d) = data {
            for (i, c) in d.meta.campaigns.iter().enumerate() {
                let cmd = format!("SetDynamicCampaign {}", c.ids);
                let mut b = Widget::toggle_button(&format!("DynamicCampaignButton{i}"), false, &cmd);
                b.radio_group = 1;
                buttons.add_child(b);
                let mut l = Widget::new(C::ToggleButton, &format!("DynamicCampaignLabel{i}"), false);
                l.command = Some(cmd);
                l.add_child(Widget::new(C::Label, "Client", false));
                l.radio_group = 2;
                buttons.add_child(l);
            }
        }
        iface.add_child(buttons);
        let mut generals = Widget::new(C::Panel, "/DynamicCampaignGeneralButtons", false);
        let count = data.map(Self::general_count).unwrap_or(0);
        for i in 0..count {
            let mut g = Widget::new(C::ToggleButton, &format!("DynamicCampaignGeneralButton{i}"), false);
            g.command = Some(format!("SetGeneral {i}"));
            g.add_child(Widget::new(C::Label, "Client", false));
            g.radio_group = 1;
            generals.add_child(g);
        }
        let mut portrait = Widget::new(C::Cinematic, "DynamicCampaignGeneralPortrait", true);
        // 942550: +0x88 = 0xFF666665
        portrait.key = f32::from_bits(0xff66_6665);
        generals.add_child(portrait);
        iface.add_child(generals);
        let mut globe = Widget::new(C::Cinematic, "/GlobeBink", true);
        globe.key = -f32::MAX;
        iface.add_child(globe);
        let mut nation = Widget::new(C::Cinematic, "/NationSelectionBackground", true);
        nation.key = -f32::MAX;
        iface.add_child(nation);
        iface
    }

    /// Build the page (the interface's CreateChildren and LoadData).
    pub fn show(&mut self, ui: &mut Ui, data: Option<&CampaignData>) {
        ui.clear();
        let mut iface = Self::tree(data);
        ui.load_widget(&mut iface, None);
        if let Some(d) = data {
            // 941B80: button decals and campaign names
            for (i, c) in d.meta.campaigns.iter().enumerate() {
                let view = d.view.campaigns.iter().find(|v| v.ids.eq_ignore_ascii_case(&c.ids));
                if let Some(b) = iface.find_mut(&format!("DynamicCampaignButtons/DynamicCampaignButton{i}")) {
                    if let (Some(v), Some(ve)) = (view, b.view.as_mut()) {
                        decal_textures(ve, &v.buttons);
                    }
                }
                let name = ui.strings.resolve(&c.name);
                if let Some(l) = iface.find_mut(&format!("DynamicCampaignButtons/DynamicCampaignLabel{i}/Client")) {
                    for ve in l.sub_ve.iter_mut().chain(l.view.iter_mut()) {
                        if let aa_ui::ve::VeKind::Label(lab) = &mut ve.kind {
                            lab.text = name.clone();
                        }
                    }
                }
            }
        }
        // The nation and general videos (5CE150, nation_selection_screen_background_video,
        // video_resource_ids) are not baked, and ui/render.rs plays the main
        // menu's parade for any other cinematic: hidden until they are.
        for p in ["DynamicCampaignGeneralButtons/DynamicCampaignGeneralPortrait", "NationSelectionBackground"] {
            if let Some(w) = iface.find_mut(p) {
                w.videos.clear();
                w.set_visible(false);
            }
        }
        ui.connect(iface);
        self.built_with_data = data.is_some();
        if let Some(d) = data {
            let c = self.campaign.min(d.meta.campaigns.len().saturating_sub(1));
            self.set_campaign(ui, d, c);
        }
        ui.layout();
    }

    fn path(rest: &str) -> String {
        format!("{IFACE}/{rest}")
    }

    /// 943200 SetDynamicCampaign.
    fn set_campaign(&mut self, ui: &mut Ui, data: &CampaignData, campaign: usize) {
        self.campaign = campaign;
        // 941BF0: select the campaign's button and label
        for (i, _) in data.meta.campaigns.iter().enumerate() {
            for name in ["DynamicCampaignButton", "DynamicCampaignLabel"] {
                ui.with_widget(&Self::path(&format!("DynamicCampaignButtons/{name}{i}")), |w| {
                    if i == campaign {
                        w.flags |= aa_ui::state::widget_flag::SELECTED;
                    } else {
                        w.flags &= !aa_ui::state::widget_flag::SELECTED;
                    }
                });
            }
        }
        // 942F80: the nation's public factions, sorted by staging name
        let nation = data.meta.campaigns.get(campaign).and_then(|c| c.kingdoms.get(c.default_kingdom)).and_then(|k| data.nation(&k.nation_ids));
        let mut factions: Vec<FactionView> = nation.map(|n| n.factions.iter().filter(|f| f.public).cloned().collect()).unwrap_or_default();
        let key = |f: &FactionView| ui.strings.resolve(&f.staging_name).to_lowercase();
        // 869D30 bisection insert with the 473320 comparator: stable ascending
        factions.sort_by(|a, b| key(a).cmp(&key(b)));
        let count = Self::general_count(data);
        for i in 0..count {
            let path = Self::path(&format!("DynamicCampaignGeneralButtons/DynamicCampaignGeneralButton{i}"));
            let name = factions.get(i).map(|f| ui.strings.resolve(&f.name));
            ui.with_widget(&path, |w| w.set_visible(name.is_some()));
            if let Some(n) = name {
                ui.set_label(&path, &n);
            }
        }
        self.factions = factions;
        self.set_general(ui, 0);
        ui.layout();
    }

    /// 941A60 SetGeneral.
    fn set_general(&mut self, ui: &mut Ui, index: usize) {
        if index >= self.factions.len() {
            return;
        }
        self.faction = index;
        for i in 0..self.factions.len() {
            ui.with_widget(&Self::path(&format!("DynamicCampaignGeneralButtons/DynamicCampaignGeneralButton{i}")), |w| {
                if i == index {
                    w.flags |= aa_ui::state::widget_flag::SELECTED;
                } else {
                    w.flags &= !aa_ui::state::widget_flag::SELECTED;
                }
            });
        }
    }

    /// The chooser's commands (943330).
    pub fn command(&mut self, ui: &mut Ui, command: &str, data: Option<&CampaignData>) -> Option<Out> {
        let mut words = command.split_whitespace();
        let head = words.next()?;
        match head.to_ascii_lowercase().as_str() {
            "dynamiccampaignnew" => {
                data?;
                let faction = self.factions.get(self.faction).map(|f| f.ids.clone());
                Some(Out::New { campaign: self.campaign, faction })
            }
            // 932A00: the load dialog of type 3 (dynamic campaign saves, 68DB60)
            "dynamiccampaignload" => Some(Out::Load),
            "dynamiccampaignexit" => Some(Out::Exit),
            "setdynamiccampaign" => {
                let ids = words.next()?;
                let d = data?;
                // 939A20: the campaign by IDS
                let c = d.meta.campaigns.iter().position(|c| c.ids.eq_ignore_ascii_case(ids))?;
                self.set_campaign(ui, d, c);
                Some(Out::None)
            }
            "setgeneral" => {
                let i: usize = words.next()?.parse().ok()?;
                self.set_general(ui, i);
                Some(Out::None)
            }
            _ => None,
        }
    }
}
