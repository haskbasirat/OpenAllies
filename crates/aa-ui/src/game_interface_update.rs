//! Game interface per-frame update `0x612010` (vtable `0xA5B0B0` +0x0C),
//! proven by research/development10/game-state-entry/native-game-update.json
//! (`native_game_update.py`, original = v14 bytes, PC24/PC53/PC64).
//!
//! The fields are the widgets Init `0x6182B0` stores in the interface:
//! TimeMessage +0x190 (attached only while UVar ViewShowElapsedGameTime
//! +0x8C), GroupFloater +0x144 / FactionAbilitiesFloater +0x148 (attached
//! while `0x5C0E30` has a local Kingdom), DeveloperModeLabel +0x170,
//! CameraInformationLabel +0x174 (AppState node 7 current and UVar
//! CameraInformation +0xB0), ControlPanel +0x138 ButtonSupplyBorder +0xA0
//! (selected as UVar ViewShowZoneFriendlySupply +0x270), DisplayBannerPanel
//! +0x178 with Background +0x17C / Banner +0x180 (fade over AVar
//! BannerFadeDurationMS), GameMenuDialog +0x154, DisplayPause +0x184 (blinks
//! every 1000 ms while KKC_Sim+0x28 paused), GameChatPanel +0x198 / ButtonChat
//! +0xA8 and UpkeepShortageIcon +0x128 (shown while the local Kingdom+0x1F0
//! shortage ratio > 0.0). Every child and world read is a required service.

use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub type Result<T> = std::result::Result<T, String>;

/// Widget flag bits read by 612010 (`widget+0x24`).
pub mod flag {
    /// Bit 0: the visible request (`0x5EFB00`).
    pub const VISIBLE: u32 = 1;
    /// Bit 5: active in the tree (AddChild / RemoveChild).
    pub const ACTIVE: u32 = 1 << 5;
    /// Bit 7: toggle selected (`0x5F01B0` / `0x5F0210`).
    pub const SELECTED: u32 = 1 << 7;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    TimeMessage,
    GroupFloater,
    FactionAbilitiesFloater,
    DeveloperModeLabel,
    CameraInformationLabel,
    ButtonSupplyBorder,
    ButtonChat,
    DisplayBannerPanel,
    GameMenuDialog,
    DisplayPause,
    GameChatPanel,
    UpkeepShortageIcon,
}
impl Field {
    pub fn name(self) -> &'static str {
        match self {
            Field::TimeMessage => "TimeMessage",
            Field::GroupFloater => "GroupFloater",
            Field::FactionAbilitiesFloater => "FactionAbilitiesFloater",
            Field::DeveloperModeLabel => "DeveloperModeLabel",
            Field::CameraInformationLabel => "CameraInformationLabel",
            Field::ButtonSupplyBorder => "ButtonSupplyBorder",
            Field::ButtonChat => "ButtonChat",
            Field::DisplayBannerPanel => "DisplayBannerPanel",
            Field::GameMenuDialog => "GameMenuDialog",
            Field::DisplayPause => "DisplayPause",
            Field::GameChatPanel => "GameChatPanel",
            Field::UpkeepShortageIcon => "UpkeepShortageIcon",
        }
    }
}

/// The interface's own banner timing words (+0x19C start, +0x1A0 last timer
/// value, +0x1A4 accumulated, +0x1A8 previous paused byte); all zero after
/// the constructor `0x61A650`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BannerTiming {
    pub start_19c: u32,
    pub last_1a0: u32,
    pub accumulated_1a4: u32,
    pub prev_paused_1a8: u8,
}

/// Reached children of 612010, in call order. No default success.
pub trait Services {
    /// UVar +0x8C ViewShowElapsedGameTime.
    fn uvar_view_show_elapsed_game_time(&mut self) -> Result<bool>;
    /// UVar +0xB0 CameraInformation.
    fn uvar_camera_information(&mut self) -> Result<bool>;
    /// UVar +0x270 ViewShowZoneFriendlySupply.
    fn uvar_view_show_zone_friendly_supply(&mut self) -> Result<bool>;
    /// TimeMessage+8 (its parent) is this interface.
    fn time_message_parent_is_interface(&mut self) -> Result<bool>;
    /// `widget+0x24` of a present field.
    fn flags(&mut self, field: Field) -> Result<u32>;
    /// `widget+0x24` store by `0x5F01B0` / `0x5F0210`.
    fn store_flags(&mut self, field: Field, flags: u32) -> Result<()>;
    /// Interface vtable +0xD0 AddChild.
    fn add_child_d0(&mut self, field: Field) -> Result<()>;
    /// `0x5EFB60(widget, 0)` RemoveChild.
    fn remove_child_5efb60(&mut self, field: Field) -> Result<()>;
    fn update_60ba50(&mut self) -> Result<()>;
    /// `0x5C0E30`: Session `0x76AC70` local Kingdom is non-null.
    fn local_kingdom_present_5c0e30(&mut self) -> Result<bool>;
    /// `0x5C06B0`: base interface update.
    fn base_update_5c06b0(&mut self) -> Result<()>;
    /// +0x170 is non-null.
    fn developer_label_present(&mut self) -> Result<bool>;
    /// `0x40EAA0` on the application.
    fn dev_mode_40eaa0(&mut self) -> Result<bool>;
    /// `0x5EFB00` (bit 0 store, then `0x5EF100`).
    fn set_visible_5efb00(&mut self, field: Field, visible: bool) -> Result<()>;
    /// `0x40CEB0(7)` then `0x40D8E0`: AppState node 7 is the current node.
    fn node7_current(&mut self) -> Result<bool>;
    /// The positive CameraInformationLabel text (`0x856370`, `0x4B9F40`
    /// "camera_info", `0x405D60`, label +0xC8).
    fn camera_information_text(&mut self) -> Result<()>;
    /// `widget+0xE4` (the 5F04C0 group selection owner) is non-null.
    fn selection_group_e4(&mut self, field: Field) -> Result<bool>;
    /// `widget+0xE0` (the functor) is non-null.
    fn functor_e0(&mut self, field: Field) -> Result<bool>;
    /// `0x5DC780(selected, 0)` on the functor, posted by `0x5C3EB0`.
    fn post_selection_event(&mut self, field: Field, selected: bool) -> Result<()>;
    fn update_427e60(&mut self) -> Result<()>;
    /// `0x59ABA0` on the application timer `0xB396E0`.
    fn timer_ms_b396e0(&mut self) -> Result<u32>;
    /// `[0xB36554]+0x28`.
    fn sim_paused_28(&mut self) -> Result<u8>;
    /// AVar +0x108 BannerFadeDurationMS (binary32 bits).
    fn banner_fade_duration_ms(&mut self) -> Result<u32>;
    /// `0x5EE520` on DisplayBannerBackground (binary32 RGBA bits).
    fn banner_background_color_5ee520(&mut self) -> Result<[u32; 4]>;
    fn set_banner_background_color_5ee600(&mut self, color: [u32; 4]) -> Result<()>;
    /// `0x5EDFF0` on DisplayBanner.
    fn banner_color_5edff0(&mut self) -> Result<[u32; 4]>;
    fn set_banner_color_5edfe0(&mut self, color: [u32; 4]) -> Result<()>;
    /// +0x154 GameMenuDialog is non-null.
    fn game_menu_present(&mut self) -> Result<bool>;
    /// `0x59ABA0` on the interface's +0x1B0 timer.
    fn pause_timer_elapsed(&mut self) -> Result<u32>;
    /// `0x59AB60` on +0x1B0.
    fn pause_timer_restart(&mut self) -> Result<()>;
    /// `0x5C1760` then Kingdom+0x1F0: None for a NULL Kingdom.
    fn local_kingdom_shortage_ratio_5c1760(&mut self) -> Result<Option<u32>>;
    fn update_60b9d0(&mut self) -> Result<()>;
}

/// `0x5F0910` / `0x5F0940`: select or deselect, with optional notification.
fn select<S: Services + ?Sized>(s: &mut S, field: Field, select: bool, notify: bool, change: bool) -> Result<()> {
    if s.selection_group_e4(field)? {
        return Err(format!("5F04C0 group selection of {} is not ported", field.name()));
    }
    let flags = s.flags(field)?;
    let selected = flags & flag::SELECTED != 0;
    if selected == select {
        return Ok(());
    }
    let mut now = flags;
    if change {
        now = if select { flags | flag::SELECTED } else { flags & !flag::SELECTED };
        s.store_flags(field, now)?;
    }
    if notify && s.functor_e0(field)? {
        s.post_selection_event(field, now & flag::SELECTED != 0)?;
    }
    Ok(())
}

fn num(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| format!("612010 binary32 operand {bits:#x}: {e}"))
}

/// One run of `0x612010`.
pub fn update_612010<S: Services + ?Sized>(banner: &mut BannerTiming, s: &mut S, precision: Precision) -> Result<()> {
    // TimeMessage +0x190
    let show_time = s.uvar_view_show_elapsed_game_time()?;
    let attached = s.time_message_parent_is_interface()?;
    if show_time {
        if !attached {
            s.add_child_d0(Field::TimeMessage)?;
        }
    } else if attached {
        s.remove_child_5efb60(Field::TimeMessage)?;
    }
    s.update_60ba50()?;
    // floaters +0x144 / +0x148
    if s.local_kingdom_present_5c0e30()? {
        for field in [Field::GroupFloater, Field::FactionAbilitiesFloater] {
            if s.flags(field)? & flag::ACTIVE == 0 {
                s.add_child_d0(field)?;
            }
        }
    } else {
        for field in [Field::GroupFloater, Field::FactionAbilitiesFloater] {
            if s.flags(field)? & flag::ACTIVE != 0 {
                s.remove_child_5efb60(field)?;
            }
        }
    }
    s.base_update_5c06b0()?;
    if s.developer_label_present()? {
        let dev = s.dev_mode_40eaa0()?;
        s.set_visible_5efb00(Field::DeveloperModeLabel, dev)?;
    }
    if s.node7_current()? && s.uvar_camera_information()? {
        s.set_visible_5efb00(Field::CameraInformationLabel, true)?;
        s.camera_information_text()?;
    } else {
        s.set_visible_5efb00(Field::CameraInformationLabel, false)?;
    }
    // ButtonSupplyBorder: 5F0910(1,1) / 5F0940(1,1)
    if s.uvar_view_show_zone_friendly_supply()? {
        select(s, Field::ButtonSupplyBorder, true, true, true)?;
    } else {
        select(s, Field::ButtonSupplyBorder, false, true, true)?;
    }
    s.update_427e60()?;
    // banner fade
    if s.flags(Field::DisplayBannerPanel)? & flag::VISIBLE != 0 {
        let now = s.timer_ms_b396e0()?;
        let paused = s.sim_paused_28()?;
        let delta = if banner.prev_paused_1a8 != 0 { 0 } else { now.wrapping_sub(banner.last_1a0) };
        let accumulated = banner.accumulated_1a4.wrapping_add(delta);
        banner.last_1a0 = now;
        banner.accumulated_1a4 = accumulated;
        banner.prev_paused_1a8 = paused;
        if accumulated > banner.start_19c {
            let elapsed = accumulated.wrapping_sub(banner.start_19c);
            let fade = s.banner_fade_duration_ms()?;
            // fild (signed) + 2^32 when negative, fstp dword
            let mut value = Binary::from_i32(elapsed as i32);
            if (elapsed as i32) < 0 {
                value = value
                    .add(&num(0x4f80_0000)?, precision)
                    .map_err(|e| e.to_string())?;
            }
            let elapsed_f32 = value.store_f32().map_err(|e| e.to_string())?;
            let fade_value = num(fade)?;
            if num(elapsed_f32)?.compare(&fade_value) == Ordering::Greater {
                s.set_visible_5efb00(Field::DisplayBannerPanel, false)?;
            } else {
                let alpha = num(0x3f80_0000)?
                    .sub(
                        &num(elapsed_f32)?.div(&fade_value, precision).map_err(|e| e.to_string())?,
                        precision,
                    )
                    .map_err(|e| e.to_string())?
                    .store_f32()
                    .map_err(|e| e.to_string())?;
                let mut color = s.banner_background_color_5ee520()?;
                color[3] = alpha;
                s.set_banner_background_color_5ee600(color)?;
                let mut color = s.banner_color_5edff0()?;
                color[3] = alpha;
                s.set_banner_color_5edfe0(color)?;
            }
        }
    }
    // DisplayPause blink
    let menu_active = if s.game_menu_present()? {
        s.flags(Field::GameMenuDialog)? & flag::ACTIVE != 0
    } else {
        false
    };
    if s.sim_paused_28()? != 0 && !menu_active {
        if s.pause_timer_elapsed()? >= 1000 {
            let visible = s.flags(Field::DisplayPause)? & flag::VISIBLE == 0;
            s.set_visible_5efb00(Field::DisplayPause, visible)?;
            s.pause_timer_restart()?;
        }
    } else {
        s.set_visible_5efb00(Field::DisplayPause, false)?;
    }
    // ButtonChat: 5F0910(0,1) / 5F0940(0,1)
    if s.flags(Field::GameChatPanel)? & flag::ACTIVE != 0 {
        select(s, Field::ButtonChat, true, false, true)?;
    } else {
        select(s, Field::ButtonChat, false, false, true)?;
    }
    // UpkeepShortageIcon: Kingdom+0x1F0 > 0.0
    let shortage = match s.local_kingdom_shortage_ratio_5c1760()? {
        Some(bits) => {
            let v = f32::from_bits(bits);
            v > 0.0
        }
        None => false,
    };
    s.set_visible_5efb00(Field::UpkeepShortageIcon, shortage)?;
    s.update_60b9d0()?;
    Ok(())
}
