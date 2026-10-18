//! UI hotkeys from `Data/Localization/Hotkeys/hotkeys_ui.txt`
//! (`map <key> [<widget>] <target>`).
//!
//! The native hotkey dispatcher is not ported. INFERRED rules used here:
//! the top-level widgets are tried from the topmost one down; an entry
//! applies to a top-level when its context names it, or when it has no
//! context and its target is a shown, enabled button inside it (the
//! button's command fires); a context entry fires its target as a console
//! command. `MessageBoxAccept` / `MessageBoxCancel` (no context) act on the
//! modal message box: its first and its last button.

use crate::state::widget_flag as wf;
use crate::ui::{Ui, UiEvent, child_segments};
use crate::widget::Widget;

#[derive(Clone, Debug, PartialEq)]
pub struct Hotkey {
    pub key: String,
    pub context: Option<String>,
    pub target: String,
}

/// Parse a hotkeys file.
pub fn parse(text: &str) -> Vec<Hotkey> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut it = line.split_whitespace();
        if !it.next().is_some_and(|w| w.eq_ignore_ascii_case("map")) {
            continue;
        }
        let rest: Vec<String> = it.map(|t| t.trim_matches('"').to_string()).collect();
        match rest.len() {
            2 => out.push(Hotkey { key: rest[0].to_ascii_lowercase(), context: None, target: rest[1].clone() }),
            3 => out.push(Hotkey { key: rest[0].to_ascii_lowercase(), context: Some(rest[1].clone()), target: rest[2].clone() }),
            _ => {}
        }
    }
    out
}

/// Shown, enabled button named `name` below `w`: (path, command).
fn find_button(w: &Widget, path: &str, name: &str) -> Option<(String, Option<String>)> {
    if w.flags & wf::SHOWN == 0 {
        return None;
    }
    for (c, seg) in w.children.iter().zip(child_segments(w)) {
        let p = format!("{path}/{seg}");
        if c.class.is_button() && c.name.trim_start_matches('/').eq_ignore_ascii_case(name) {
            if c.flags & wf::SHOWN != 0 && c.flags & wf::ENABLED != 0 {
                return Some((p, c.command.clone()));
            }
            return None;
        }
        if let Some(hit) = find_button(c, &p, name) {
            return Some(hit);
        }
    }
    None
}

impl Ui {
    /// Load `hotkeys_ui.txt` (once).
    pub fn hotkeys(&mut self) -> &[Hotkey] {
        if self.hotkeys.is_none() {
            let text = self
                .store
                .root
                .read("Localization/Hotkeys/hotkeys_ui.txt")
                .map(|b| crate::data::decode(&b))
                .unwrap_or_default();
            self.hotkeys = Some(parse(&text));
        }
        self.hotkeys.as_deref().unwrap_or(&[])
    }

    /// A key press by hotkey name (`esc`, `enter`, `d` ...). Returns true
    /// when an entry fired.
    pub fn hotkey(&mut self, key: &str) -> bool {
        let key = key.to_ascii_lowercase();
        let entries: Vec<Hotkey> = self.hotkeys().iter().filter(|h| h.key == key).cloned().collect();
        if entries.is_empty() {
            return false;
        }
        // the modal message box
        if let Some(req) = self.message_box.clone() {
            for h in entries.iter().filter(|h| h.context.is_none()) {
                let i = match h.target.as_str() {
                    "MessageBoxAccept" => Some(0),
                    "MessageBoxCancel" => req.buttons.len().checked_sub(1),
                    _ => None,
                };
                if let Some(i) = i.filter(|&i| i < req.buttons.len().min(3)) {
                    self.close_message_box();
                    let cmd = req.buttons[i].1.clone();
                    self.events.push(UiEvent::Command { widget: "MessageBox".into(), command: cmd });
                    self.layout();
                    return true;
                }
            }
            return false;
        }
        // "editbox" context: the focused edit box's commands
        if self.capture.last().and_then(|c| self.widget(c)).is_some_and(|w| w.class == crate::widget::WidgetClass::Editbox) {
            for h in entries.iter().filter(|h| h.context.as_deref().is_some_and(|c| c.eq_ignore_ascii_case("editbox"))) {
                if self.editbox_command(&h.target) {
                    return true;
                }
            }
        }
        let root = self.root.name.trim_start_matches('/').to_string();
        let tops: Vec<(String, String)> = self
            .root
            .children
            .iter()
            .zip(child_segments(&self.root))
            .rev()
            .filter(|(c, _)| c.flags & wf::SHOWN != 0)
            .map(|(c, s)| (c.name.trim_start_matches('/').to_string(), s))
            .collect();
        for (name, seg) in tops {
            for h in &entries {
                match &h.context {
                    Some(ctx) if ctx.eq_ignore_ascii_case(&name) => {
                        self.events.push(UiEvent::Command { widget: format!("{root}/{seg}"), command: h.target.clone() });
                        return true;
                    }
                    Some(_) => {}
                    None => {
                        let top = &self.root.children.iter().find(|c| c.name.trim_start_matches('/') == name);
                        if let Some(t) = top {
                            if let Some((path, cmd)) = find_button(t, &format!("{root}/{seg}"), &h.target) {
                                if let Some(c) = cmd {
                                    self.events.push(UiEvent::Command { widget: path, command: c });
                                }
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lines() {
        let h = parse("# x\nmap esc MainMenuButtonQuit\nmap enter OptionsDialog OptionsOK\nmap esc CreateGameDialog \"CreateGameCancel\"\n");
        assert_eq!(h.len(), 3);
        assert_eq!(h[0], Hotkey { key: "esc".into(), context: None, target: "MainMenuButtonQuit".into() });
        assert_eq!(h[1].context.as_deref(), Some("OptionsDialog"));
        assert_eq!(h[2].target, "CreateGameCancel");
    }
}
