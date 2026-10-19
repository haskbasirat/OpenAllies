//!Bindings of the ported Game interface update 612010 / HUD children and the
//!LoadingScreen OnConnected 6E8730 to the actual session, World and Ui.
//!Reads happen on the retained simulation thread (`LiveFrame::read`,
//!`LoadingFrame::read`) and travel as Send values through the UI bridge;
//!widgets are only touched on the UI thread. A source that is not bound is a
//!named error: for the HUD children it is recorded once and that widget keeps
//!its state (aa_ui::hud::LiveWorld), never filled with an invented value.
use aa_numeric::Precision;
use aa_ui::{
    game_interface_update::{self as gu, Field},
    hud::{FactionExperience, Hud, HudAbility, HudAction, HudEconomy, HudImpact, HudResource, HudSelection, LiveWorld},
    loading_refresh::{self as lr, Art, Widget as LoadingWidget},
    ui::Ui,
};
type Result<T> = std::result::Result<T, String>;

/// Resource registry entries (`0x48B800`) in native registry order: +0x18
/// type, +0x1C purchase costs, +0x28 icon IDS, +0x2C limit provider, +0x30
/// limited-by index, from the same source definitions the economy uses.
pub fn resource_entries(
    catalog: &aa_runtime::catalog::Catalog<'_>,
    source: &aa_rules::resources::ResourceCatalog,
) -> Result<Vec<HudResource>> {
    let pc = Precision::Double53;
    let registry = catalog.actors.native_resource_registry(pc).map_err(|e| e.to_string())?.ids;
    let economy = catalog.native_resource_economy(pc)?;
    let nodes = catalog.actors.resolved_definitions("Resource").map_err(|e| e.to_string())?;
    if nodes.len() != registry.len() || economy.catalog().resources.len() != registry.len() {
        return Err("HUD Resource registry, definitions and economy counts differ".into());
    }
    let mut out = Vec::with_capacity(registry.len());
    for ((ids, node), def) in registry.iter().zip(&nodes).zip(&economy.catalog().resources) {
        if !def.id.eq_ignore_ascii_case(ids) {
            return Err(format!("HUD Resource economy order differs at {ids}"));
        }
        let icon = match node.field("icon", pc).map_err(|e| e.to_string())? {
            Some(aa_rules::native::TypedValue::String { value, .. }) => value,
            None => String::new(),
            Some(_) => return Err(format!("Resource {ids} icon is not a string")),
        };
        let authored = source.find(ids).ok_or_else(|| format!("Resource {ids} source definition absent"))?;
        let limited_by = match &authored.limited_by {
            Some(other) => Some(
                registry
                    .iter()
                    .position(|r| r.eq_ignore_ascii_case(other))
                    .ok_or_else(|| format!("Resource {ids} limited_by {other} is not registered"))?,
            ),
            None => None,
        };
        out.push(HudResource {
            ids: ids.clone(),
            kind: match def.kind {
                aa_economy::ResourceKind::Stockpiled => 0,
                aa_economy::ResourceKind::Upkeep => 1,
                aa_economy::ResourceKind::LimitedConsumed => 2,
            },
            icon,
            limited_by,
            limit_provider: def.limited_provider,
            purchase_cost: def.purchase_cost.iter().map(|v| v.as_f32()).collect(),
        });
    }
    Ok(out)
}

/// Simulation-thread reads for one Game interface update.
#[derive(Clone, Debug)]
pub struct LiveFrame {
    local_kingdom_present_5c0e30: Result<bool>,
    dev_mode_40eaa0: Result<bool>,
    node7_current: bool,
    sim_paused_28: u8,
    shortage_ratio_5c1760: Result<Option<u32>>,
    economy: Result<Option<HudEconomy>>,
    faction_experience: Result<Option<FactionExperience>>,
    view_time_8c: Result<f32>,
    selection_empty: Result<Option<bool>>,
}
impl LiveFrame {
    #[allow(clippy::too_many_arguments)]
    pub fn read<G>(
        session: &aa_netcode::local_bootstrap::world_ready::ActualActivatedSession,
        view: &aa_runtime::view_init_prefix::State<G>,
        application_state: u32,
        dev_mode: impl FnMut() -> Result<u8>,
    ) -> Self {
        let world = session.world();
        let parts = world.presentation_parts();
        let kingdom = session.local_player_kingdom_76ac70().map_err(|e| e.to_string());
        //5C1760: 40CEC0 (Session+E4 == 3) film branch 6AA370; otherwise the
        //Session 76AC70 Kingdom, then UI+1E0 and VWorld+13C fallbacks.
        let local_5c1760 = |kingdom: &Result<Option<aa_actors::ActorId>>| -> Result<aa_actors::ActorId> {
            if session.session_state_e4() == 3 {
                return Err("5C1760 film branch 6AA370 is not ported".into());
            }
            match kingdom {
                Ok(Some(k)) => Ok(*k),
                Ok(None) => Err("5C1760 UI+1E0 / VWorld+13C fallback Kingdom is not bound".into()),
                Err(e) => Err(e.clone()),
            }
        };
        let k = local_5c1760(&kingdom);
        let actors = world.actors();
        let economy = k.clone().and_then(|k| {
            let vectors = aa_runtime::hud_world_reads::kingdom_economy(actors, k)?;
            let variables = parts.as_ref().map_err(Clone::clone)?.variables;
            let max = aa_runtime::hud_world_reads::economy_limited_resource_max(variables)?;
            let f = |v: &[u32]| v.iter().map(|b| f32::from_bits(*b)).collect::<Vec<_>>();
            Ok(Some(HudEconomy {
                production: f(&vectors.production_1a0),
                preview_production: f(&vectors.preview_production_1ac),
                upkeep: f(&vectors.upkeep_1b8),
                stock: f(&vectors.stock_1c4),
                limited_max: max as f32,
            }))
        });
        //5C0E50 for FactionExperience 604130: 40CEC0 -> NULL.
        let faction_experience = if session.session_state_e4() == 3 {
            Ok(None)
        } else {
            match &kingdom {
                Ok(Some(k)) => aa_runtime::hud_world_reads::kingdom_commander_experience_230(actors, *k).map(|c| {
                    c.map(|(value, max)| FactionExperience { value: f32::from_bits(value), max: f32::from_bits(max) })
                }),
                Ok(None) => Err("5C0E50 UI+1E0 fallback Kingdom is not bound".into()),
                Err(e) => Err(e.clone()),
            }
        };
        let view_time_8c = parts
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|p| aa_runtime::hud_world_reads::view_time_8c(view, p.revision))
            .map(f32::from_bits);
        Self {
            local_kingdom_present_5c0e30: kingdom.as_ref().map(Option::is_some).map_err(Clone::clone),
            dev_mode_40eaa0: session.host().can_process_40eaa0(dev_mode),
            node7_current: application_state == 7,
            sim_paused_28: session.sim_paused_28(),
            shortage_ratio_5c1760: k
                .and_then(|k| aa_runtime::hud_world_reads::kingdom_shortage_ratio_1f0(actors, k))
                .map(Some),
            economy,
            faction_experience,
            view_time_8c,
            selection_empty: session.local_player_selection_empty().map_err(|e| e.to_string()),
        }
    }
    ///A laboratory frame for the ignored real-forest test only.
    #[cfg(test)]
    pub(crate) fn laboratory(shortage: Option<u32>, economy: Option<HudEconomy>, paused: u8, time: f32) -> Self {
        Self {
            local_kingdom_present_5c0e30: Ok(true),
            dev_mode_40eaa0: Ok(false),
            node7_current: false,
            sim_paused_28: paused,
            shortage_ratio_5c1760: Ok(shortage),
            economy: Ok(economy),
            faction_experience: Ok(Some(FactionExperience { value: 12.0, max: 5000.0 })),
            view_time_8c: Ok(time),
            selection_empty: Ok(Some(true)),
        }
    }
    fn no_selection(&self) -> Result<bool> {
        match &self.selection_empty {
            Ok(Some(empty)) => Ok(*empty),
            Ok(None) => Ok(true),
            Err(e) => Err(e.clone()),
        }
    }
}
impl LiveWorld for LiveFrame {
    fn economy(&self) -> Result<Option<HudEconomy>> {
        self.economy.clone()
    }
    ///The only impact source in A&A is a hovered action button (602640;
    ///AVar UIUseRecruitItemFloater is false). No selection, no buttons.
    fn economic_impact(&self) -> Result<Option<HudImpact>> {
        if self.no_selection()? {
            Ok(None)
        } else {
            Err("EconomicImpact hovered action source (602640) is not bound".into())
        }
    }
    fn faction_experience(&self) -> Result<Option<FactionExperience>> {
        self.faction_experience.clone()
    }
    fn faction_abilities(&self) -> Result<Vec<HudAbility>> {
        Err("FactionAbilities 607FC0 Commander ability item source is not bound".into())
    }
    fn group_textures(&self) -> Result<Vec<Option<String>>> {
        Err("GroupFloater 607B30 static group source is not bound".into())
    }
    fn idle_builder(&self) -> Result<bool> {
        Err("IdleButtonBuilder 6233C0 world scan is not bound".into())
    }
    fn idle_detached_organization(&self) -> Result<bool> {
        Err("IdleButtonDetachedOrganization 6233C0 world scan is not bound".into())
    }
    fn game_time(&self) -> Result<f32> {
        self.view_time_8c.clone()
    }
    fn selection(&self) -> Result<HudSelection> {
        if self.no_selection()? {
            Ok(HudSelection::None)
        } else {
            Err("ObjectDisplay 62CBA0 selection display binding is not ported".into())
        }
    }
    fn action_buttons(&self) -> Result<Vec<Option<HudAction>>> {
        Err("ActionButtons 5F9520 slot source is not bound".into())
    }
}

/// The Game interface's own retained fields used by 612010.
pub struct GameRuntime {
    banner: gu::BannerTiming,
    /// +0x1B0 timer: 0x59AB60 restart in the constructor 0x61A650.
    pause_timer_start_ms: u64,
    notes: Vec<String>,
}
impl GameRuntime {
    pub fn constructed(now_ms: u64) -> Self {
        Self { banner: gu::BannerTiming::default(), pause_timer_start_ms: now_ms, notes: Vec::new() }
    }
}

fn path(field: Field) -> &'static str {
    match field {
        Field::TimeMessage => "Game/TimeMessage",
        Field::GroupFloater => "Game/GroupFloater",
        Field::FactionAbilitiesFloater => "Game/FactionAbilitiesFloater",
        Field::DeveloperModeLabel => "Game/DeveloperModeLabel",
        Field::CameraInformationLabel => "Game/CameraInformationLabel",
        Field::ButtonSupplyBorder => "Game/ControlPanel/ButtonSupplyBorder",
        Field::ButtonChat => "Game/ControlPanel/ButtonChat",
        Field::DisplayBannerPanel => "Game/DisplayBannerPanel",
        Field::GameMenuDialog => "Game/GameMenuDialog",
        Field::DisplayPause => "Game/DisplayPause",
        Field::GameChatPanel => "Game/GameChatPanel",
        Field::UpkeepShortageIcon => "Game/UpkeepShortageIcon",
    }
}
fn owned_child(field: Field) -> bool {
    matches!(field, Field::TimeMessage | Field::GroupFloater | Field::FactionAbilitiesFloater)
}

struct GameServices<'a> {
    ui: &'a mut Ui,
    hud: &'a mut Hud,
    frame: &'a LiveFrame,
    pause_timer_start_ms: &'a mut u64,
    notes: &'a mut Vec<String>,
}
impl GameServices<'_> {
    fn note(&mut self, text: String) {
        if !self.notes.contains(&text) {
            log::warn!("{text}");
            self.notes.push(text);
        }
    }
    fn widget(&self, field: Field) -> Result<&aa_ui::widget::Widget> {
        self.ui
            .widget_by(path(field))
            .ok_or_else(|| format!("612010 {} widget is absent", field.name()))
    }
}
impl gu::Services for GameServices<'_> {
    fn uvar_view_show_elapsed_game_time(&mut self) -> Result<bool> {
        Ok(self.ui.uvars.get_bool("ViewShowElapsedGameTime"))
    }
    fn uvar_camera_information(&mut self) -> Result<bool> {
        Ok(self.ui.uvars.get_bool("CameraInformation"))
    }
    fn uvar_view_show_zone_friendly_supply(&mut self) -> Result<bool> {
        Ok(self.ui.uvars.get_bool("ViewShowZoneFriendlySupply"))
    }
    fn time_message_parent_is_interface(&mut self) -> Result<bool> {
        Ok(self.hud.interface_child_attached(self.ui, "TimeMessage"))
    }
    fn flags(&mut self, field: Field) -> Result<u32> {
        match field {
            //Init 6182B0 creates +198 with 618090 and no AddChild; nothing
            //in this port attaches it (ChatActivate is not bound).
            Field::GameChatPanel => Ok(aa_ui::widget::INITIAL_FLAGS),
            Field::GameMenuDialog => Err("612010 +154 GameMenuDialog has no 613210 writer here".into()),
            f if owned_child(f) => {
                let name = f.name();
                self.hud.interface_child_flags(self.ui, name).ok_or_else(|| format!("612010 {name} child is absent"))
            }
            f => Ok(self.widget(f)?.flags),
        }
    }
    fn store_flags(&mut self, field: Field, flags: u32) -> Result<()> {
        if !self.ui.with_widget(path(field), |w| w.flags = flags) {
            return Err(format!("612010 {} widget is absent", field.name()));
        }
        Ok(())
    }
    fn add_child_d0(&mut self, field: Field) -> Result<()> {
        self.hud.attach_interface_child(self.ui, field.name())
    }
    fn remove_child_5efb60(&mut self, field: Field) -> Result<()> {
        self.hud.detach_interface_child(self.ui, field.name())
    }
    fn update_60ba50(&mut self) -> Result<()> {
        self.note("612010 child 60BA50 (tooltip/60B4C0/60B580) is not ported".into());
        Ok(())
    }
    fn local_kingdom_present_5c0e30(&mut self) -> Result<bool> {
        self.frame.local_kingdom_present_5c0e30.clone()
    }
    ///5EEC70 + 5C05C0 are the widget/interface animation timers; the aa-ui
    ///Ui::update of the same frame owns that presentation state.
    fn base_update_5c06b0(&mut self) -> Result<()> {
        Ok(())
    }
    fn developer_label_present(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn dev_mode_40eaa0(&mut self) -> Result<bool> {
        self.frame.dev_mode_40eaa0.clone()
    }
    fn set_visible_5efb00(&mut self, field: Field, visible: bool) -> Result<()> {
        let done = if owned_child(field) {
            self.hud.with_interface_child(self.ui, field.name(), |w| w.set_visible(visible))
        } else {
            self.ui.with_widget(path(field), |w| w.set_visible(visible))
        };
        if !done {
            return Err(format!("612010 {} widget is absent", field.name()));
        }
        Ok(())
    }
    fn node7_current(&mut self) -> Result<bool> {
        Ok(self.frame.node7_current)
    }
    fn camera_information_text(&mut self) -> Result<()> {
        Err("612010 CameraInformationLabel text (AppState 7 + CameraInformation) is not ported".into())
    }
    fn selection_group_e4(&mut self, field: Field) -> Result<bool> {
        Ok(self.widget(field)?.radio_group != 0)
    }
    fn functor_e0(&mut self, field: Field) -> Result<bool> {
        Ok(self.widget(field)?.command.is_some())
    }
    fn post_selection_event(&mut self, field: Field, selected: bool) -> Result<()> {
        let command = self.widget(field)?.command.clone().unwrap_or_default();
        self.note(format!("5DC780 selection event ({selected}) for {} functor {command} is not dispatched", field.name()));
        Ok(())
    }
    fn update_427e60(&mut self) -> Result<()> {
        self.note("612010 child 427E60 (B3971C) is not ported".into());
        Ok(())
    }
    fn timer_ms_b396e0(&mut self) -> Result<u32> {
        Ok(self.ui.now_ms as u32)
    }
    fn sim_paused_28(&mut self) -> Result<u8> {
        Ok(self.frame.sim_paused_28)
    }
    fn banner_fade_duration_ms(&mut self) -> Result<u32> {
        self.ui
            .avars
            .float("BannerFadeDurationMS")
            .map(f32::to_bits)
            .ok_or_else(|| "AVar BannerFadeDurationMS is absent".into())
    }
    fn banner_background_color_5ee520(&mut self) -> Result<[u32; 4]> {
        Err("DisplayBannerBackground colour 5EE520 is not bound".into())
    }
    fn set_banner_background_color_5ee600(&mut self, _: [u32; 4]) -> Result<()> {
        Err("DisplayBannerBackground colour 5EE600 is not bound".into())
    }
    fn banner_color_5edff0(&mut self) -> Result<[u32; 4]> {
        Err("DisplayBanner colour 5EDFF0 is not bound".into())
    }
    fn set_banner_color_5edfe0(&mut self, _: [u32; 4]) -> Result<()> {
        Err("DisplayBanner colour 5EDFE0 is not bound".into())
    }
    ///+154 is cleared by OnConnected 60BAE0 and written only by 613210 (the
    ///GameMenuDialog open path), which this port does not reach.
    fn game_menu_present(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn pause_timer_elapsed(&mut self) -> Result<u32> {
        Ok(self.ui.now_ms.saturating_sub(*self.pause_timer_start_ms) as u32)
    }
    fn pause_timer_restart(&mut self) -> Result<()> {
        *self.pause_timer_start_ms = self.ui.now_ms;
        Ok(())
    }
    fn local_kingdom_shortage_ratio_5c1760(&mut self) -> Result<Option<u32>> {
        self.frame.shortage_ratio_5c1760.clone()
    }
    fn update_60b9d0(&mut self) -> Result<()> {
        self.note("612010 child 60B9D0 (GWorld+27C Counter) is not ported".into());
        Ok(())
    }
}

/// One Game interface frame: 612010, then its children's own updates.
pub fn update(ui: &mut Ui, hud: &mut Hud, runtime: &mut GameRuntime, frame: &LiveFrame) -> Result<()> {
    let GameRuntime { banner, pause_timer_start_ms, notes } = runtime;
    let mut services = GameServices { ui, hud, frame, pause_timer_start_ms, notes };
    gu::update_612010(banner, &mut services, Precision::Double53)?;
    hud.update_children(ui, frame);
    for d in std::mem::take(&mut hud.diagnostics) {
        if !notes.contains(&d) {
            log::warn!("Game interface: {d}");
            notes.push(d);
        }
    }
    Ok(())
}

/// Simulation-thread reads for LoadingScreen OnConnected 6E8730.
#[derive(Clone, Debug)]
pub struct LoadingFrame {
    kind_c: u32,
    phase_f8: u32,
    launch_mode_58: u32,
    texts_68: Result<(String, String)>,
    local_player: bool,
    nation: Result<Option<String>>,
    status_148: String,
}
impl LoadingFrame {
    #[cfg(test)]
    pub(crate) fn laboratory(nation: Option<&str>) -> Self {
        Self {
            kind_c: 2,
            phase_f8: 2,
            launch_mode_58: 2,
            texts_68: Ok(("(2v2) Berlin".into(), "Description text".into())),
            local_player: true,
            nation: Ok(nation.map(str::to_string)),
            status_148: "Loading Resources".into(),
        }
    }
    pub fn read(session: &aa_netcode::local_bootstrap::world_ready::ActualActivatedSession) -> Self {
        let utf = |v: Vec<u16>| String::from_utf16_lossy(&v);
        Self {
            kind_c: session.services().application().mode(),
            phase_f8: session.session_phase_f8(),
            launch_mode_58: session.session_launch_mode_58(),
            texts_68: session.game_settings_68_texts().map(|(a, b)| (utf(a), utf(b))).map_err(|e| e.to_string()),
            local_player: session.local_player_present_76ac60(),
            nation: session.local_player_creator_nation_50dc00().map(|n| n.map(utf)).map_err(|e| e.to_string()),
            status_148: utf(session.session_status_148()),
        }
    }
}

struct LoadingServices<'a> {
    ui: &'a mut Ui,
    frame: &'a LoadingFrame,
    hold_745580: &'a mut u32,
}
const LS: &str = "LoadingScreen";
impl lr::Services for LoadingServices<'_> {
    type Texture = String;
    ///Interface show timing; Show is immediate in this presentation.
    fn base_on_connected_5c08e0(&mut self) -> Result<()> {
        Ok(())
    }
    fn game_app_kind_c(&mut self) -> Result<u32> {
        Ok(self.frame.kind_c)
    }
    fn preload_campaign_mission_art(&mut self) -> Result<()> {
        Err("6E8730 campaign mission art 411790 is not ported".into())
    }
    fn campaign_mission_art(&mut self) -> Result<Art<String>> {
        Err("6E83A0 campaign mission entry 411790 is not ported".into())
    }
    fn session_phase_f8(&mut self) -> Result<u32> {
        Ok(self.frame.phase_f8)
    }
    fn session_launch_mode_58(&mut self) -> Result<u32> {
        Ok(self.frame.launch_mode_58)
    }
    fn set_visible(&mut self, widget: LoadingWidget, visible: bool) -> Result<()> {
        if !self.ui.with_widget(&format!("{LS}/{}", widget.name()), |w| w.set_visible(visible)) {
            return Err(format!("LoadingScreen {} widget is absent", widget.name()));
        }
        Ok(())
    }
    fn set_map_name_text(&mut self) -> Result<()> {
        let text = self.frame.texts_68.as_ref().map_err(Clone::clone)?.0.clone();
        self.ui.set_label(&format!("{LS}/MapNameLabel"), &text).then_some(()).ok_or_else(|| "MapNameLabel is absent".into())
    }
    fn set_map_description_text(&mut self) -> Result<()> {
        let text = self.frame.texts_68.as_ref().map_err(Clone::clone)?.1.clone();
        self.ui
            .set_label(&format!("{LS}/MapDescriptionLabel"), &text)
            .then_some(())
            .ok_or_else(|| "MapDescriptionLabel is absent".into())
    }
    fn objectives_populate_673d40(&mut self) -> Result<()> {
        Err("6E83A0 campaign objectives 673D40 are not ported".into())
    }
    fn objectives_clear_673720(&mut self) -> Result<()> {
        self.ui.viewport_clear(&format!("Root/{LS}/ObjectivesList/SlotViewport"));
        Ok(())
    }
    fn ui_present(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn local_player_present_5c0e10(&mut self) -> Result<bool> {
        Ok(self.frame.local_player)
    }
    ///KingdomParameter+18 is the nation named by its nation string; its
    ///+5C/+60 loading art is Nations/<nation>.tgi loading_texture_left/right
    ///(48C050, aa_ui::screens::nation_loading_textures).
    fn local_player_nation_art(&mut self) -> Result<Option<Option<Art<String>>>> {
        let nation = self.frame.nation.clone()?;
        Ok(nation.map(|n| {
            if n.is_empty() {
                None
            } else {
                match aa_ui::screens::nation_loading_textures(self.ui, &n) {
                    Some((l, r)) => Some((Some(l), Some(r))),
                    None => Some((None, None)),
                }
            }
        }))
    }
    ///LoadData 6E8580 +EC/+F0.
    fn default_art(&mut self) -> Result<Art<String>> {
        let node = self.ui.store.node("UI/Loading/loading_screen.tgi:LoadingScreen").map_err(|e| e.to_string())?;
        let tex = |k: &str| Some(node.string_or_empty(k)).filter(|s| !s.is_empty());
        Ok((tex("LoadingTextureLeft"), tex("LoadingTextureRight")))
    }
    fn dynamic_art(&mut self) -> Result<Art<String>> {
        Err("6E8580 +F4/+F8 dynamic campaign art (40D0F0(0xB) menu) is not constructed".into())
    }
    fn set_texture_5ee580(&mut self, widget: LoadingWidget, texture: Option<String>) -> Result<()> {
        let done = self.ui.with_widget(&format!("{LS}/{}", widget.name()), |w| {
            if let Some(aa_ui::ve::ViewElement { kind: aa_ui::ve::VeKind::Decal(d), .. }) = &mut w.sub_ve {
                d.texture = texture;
            }
        });
        done.then_some(()).ok_or_else(|| format!("LoadingScreen {} widget is absent", widget.name()))
    }
    fn session_status_6e8300(&mut self) -> Result<()> {
        let text = self.frame.status_148.clone();
        self.ui.set_label(&format!("{LS}/StatusLabel"), &text).then_some(()).ok_or_else(|| "StatusLabel is absent".into())
    }
    ///B36548 +0x190 hold counter (lock +80); its reader is not ported.
    fn audio_hold_745580(&mut self) -> Result<()> {
        *self.hold_745580 += 1;
        Ok(())
    }
}

/// LoadingScreen OnConnected 6E8730 after its Show.
pub fn loading_on_connected(ui: &mut Ui, frame: &LoadingFrame, hold_745580: &mut u32) -> Result<()> {
    lr::on_connected_6e8730(&mut LoadingServices { ui, frame, hold_745580 })?;
    ui.layout();
    Ok(())
}
