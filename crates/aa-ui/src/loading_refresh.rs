//! LoadingScreen OnConnected `0x6E8730` (vtable `0xA7EDD0` +0x48), its
//! refresh `0x6E83A0` and status `0x6E8300`, proven by
//! research/development10/game-state-entry/native-loading-refresh.json
//! (`native_loading_refresh.py`, original = v14 bytes, PC24/PC53/PC64,
//! precision independent).
//!
//! Fields of Init `0x6E88E0`: +0xD8 MapNameLabel, +0xDC MapDescriptionLabel,
//! +0xE0 ObjectivesList, +0xE4/+0xE8 LoadingTextureLeft/Right, +0xEC/+0xF0
//! default art and +0xF4/+0xF8 dynamic-campaign art (LoadData `0x6E8580`),
//! +0xFC StatusLabel, +0x100 ReadyButton.

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widget {
    MapNameLabel,
    MapDescriptionLabel,
    ObjectivesList,
    LoadingTextureLeft,
    LoadingTextureRight,
    ReadyButton,
}
impl Widget {
    pub fn name(self) -> &'static str {
        match self {
            Widget::MapNameLabel => "MapNameLabel",
            Widget::MapDescriptionLabel => "MapDescriptionLabel",
            Widget::ObjectivesList => "ObjectivesList",
            Widget::LoadingTextureLeft => "LoadingTextureLeft",
            Widget::LoadingTextureRight => "LoadingTextureRight",
            Widget::ReadyButton => "ReadyButton",
        }
    }
}

/// A texture pair; either word may be the native NULL.
pub type Art<T> = (Option<T>, Option<T>);

/// Reached children, in native call order. No default success.
pub trait Services {
    type Texture: Clone;
    /// `0x5C08E0` base interface OnConnected.
    fn base_on_connected_5c08e0(&mut self) -> Result<()>;
    /// `0x40CEB0(0xC)` GameAppState +0xC (read by 6E8730 and again by 6E83A0).
    fn game_app_kind_c(&mut self) -> Result<u32>;
    /// Kind 0: `0x411790(B39D04+0x50, 1)` current campaign mission entry art
    /// +0x10/+0x14/+0x18 preload through texture +0x0C (6E8730), returning
    /// the +0x14/+0x18 pair for 6E83A0.
    fn preload_campaign_mission_art(&mut self) -> Result<()>;
    fn campaign_mission_art(&mut self) -> Result<Art<Self::Texture>>;
    /// Session `0x76AC90` (+0xF8).
    fn session_phase_f8(&mut self) -> Result<u32>;
    /// Session +0x58 launch mode.
    fn session_launch_mode_58(&mut self) -> Result<u32>;
    /// `0x5EFB00`.
    fn set_visible(&mut self, widget: Widget, visible: bool) -> Result<()>;
    /// MapNameLabel +0xC8 with Session+0x68 +0x24.
    fn set_map_name_text(&mut self) -> Result<()>;
    /// MapDescriptionLabel +0xC8 with Session+0x68 +0x2C.
    fn set_map_description_text(&mut self) -> Result<()>;
    /// `0x673D40` on ObjectivesList.
    fn objectives_populate_673d40(&mut self) -> Result<()>;
    /// `0x673720`: ObjectivesList+0x88 (SlotViewport) +0xC8 ClearItems.
    fn objectives_clear_673720(&mut self) -> Result<()>;
    /// `0xB3653C` (the UI) is non-null.
    fn ui_present(&mut self) -> Result<bool>;
    /// `0x5C0E10`: Session present and `[B36558]+8` (local player) non-null.
    fn local_player_present_5c0e10(&mut self) -> Result<bool>;
    /// `0x76C700(player+0x24)` then `0x50DC00`: the local player's Creator
    /// KingdomParameter. None for no entry; Some(None) for a NULL +0x18
    /// nation; Some(Some(art)) with the nation's +0x5C/+0x60 art.
    fn local_player_nation_art(&mut self) -> Result<Option<Option<Art<Self::Texture>>>>;
    /// LoadData +0xEC/+0xF0.
    fn default_art(&mut self) -> Result<Art<Self::Texture>>;
    /// LoadData +0xF4/+0xF8.
    fn dynamic_art(&mut self) -> Result<Art<Self::Texture>>;
    /// `0x5EE580(texture, 0)`.
    fn set_texture_5ee580(&mut self, widget: Widget, texture: Option<Self::Texture>) -> Result<()>;
    /// `0x6E8300`: StatusLabel +0xC8 with `0x76E4C0` ("%s", Session+0x148).
    fn session_status_6e8300(&mut self) -> Result<()>;
    /// `0x745580` on `0xB36548` (+0x190 hold counter).
    fn audio_hold_745580(&mut self) -> Result<()>;
}

/// `0x6E8730`.
pub fn on_connected_6e8730<S: Services + ?Sized>(s: &mut S) -> Result<()> {
    s.base_on_connected_5c08e0()?;
    if s.game_app_kind_c()? == 0 {
        s.preload_campaign_mission_art()?;
    }
    refresh_6e83a0(s)?;
    s.audio_hold_745580()
}

/// `0x6E83A0`.
pub fn refresh_6e83a0<S: Services + ?Sized>(s: &mut S) -> Result<()> {
    let kind_reader = |s: &mut S| s.game_app_kind_c();
    if s.session_phase_f8()? == 5 || s.session_launch_mode_58()? == 5 {
        s.set_visible(Widget::MapNameLabel, false)?;
        s.set_visible(Widget::MapDescriptionLabel, false)?;
    } else {
        s.set_visible(Widget::MapNameLabel, true)?;
        s.set_visible(Widget::MapDescriptionLabel, true)?;
        s.set_map_name_text()?;
        s.set_map_description_text()?;
    }
    let kind = kind_reader(s)?;
    let mission = if kind == 0 {
        s.set_visible(Widget::ObjectivesList, true)?;
        s.objectives_populate_673d40()?;
        Some(s.campaign_mission_art()?)
    } else {
        s.objectives_clear_673720()?;
        s.set_visible(Widget::ObjectivesList, false)?;
        None
    };
    let (mut left, mut right) = s.default_art()?;
    if s.ui_present()? && s.local_player_present_5c0e10()? {
        let nation = s.local_player_nation_art()?;
        match mission {
            Some((Some(l), Some(r))) => {
                left = Some(l);
                right = Some(r);
            }
            _ => {
                let nation_pair = match nation {
                    Some(Some((Some(l), Some(r)))) => Some((l, r)),
                    _ => None,
                };
                if kind == 4 {
                    (left, right) = s.dynamic_art()?;
                } else if let Some((l, r)) = nation_pair {
                    left = Some(l);
                    right = Some(r);
                }
            }
        }
    }
    s.set_texture_5ee580(Widget::LoadingTextureLeft, left)?;
    s.set_texture_5ee580(Widget::LoadingTextureRight, right)?;
    s.set_visible(Widget::LoadingTextureLeft, true)?;
    s.set_visible(Widget::LoadingTextureRight, true)?;
    s.set_visible(Widget::ReadyButton, false)?;
    s.session_status_6e8300()
}
