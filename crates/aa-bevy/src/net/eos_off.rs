//! The Internet button of a build without the `eos` feature: it says so. LAN and
//! a direct address need nothing of Epic's. See `eos.rs` for the feature.
use super::{box_ok, Action, Lan};
use aa_ui::ui::Ui;

#[derive(Default)]
pub struct Online;

impl Lan {
    pub fn internet_lobby(&mut self, _ui: &mut Ui, _first_time: bool) -> bool {
        false
    }

    pub fn online_busy(&self) -> bool {
        false
    }

    pub(super) fn online_command(&mut self, ui: &mut Ui, cmd: &str) -> Option<Action> {
        if cmd != "RequestLobbyInternet" {
            return None;
        }
        let head = ui.strings.resolve("#ui_lobby_cant_connect");
        box_ok(ui, format!("{head}\nThis build was made without online play. LAN and a direct address work."));
        Some(Action::None)
    }

    pub(super) fn online_frame(&mut self, _ui: &mut Ui, _actions: &mut Vec<Action>) {}
}
