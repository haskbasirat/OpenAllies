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
                    let cmd = self.message_box_command(i).unwrap_or_default();
                    self.close_message_box();
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

// ---------------------------------------------------------------------
// The original's hotkey table (Hotkeys.cpp and the console `map`
// command 0x5CA4B0).
//
// - The console splits a line into arguments (whitespace, `"` groups,
//   `#` starts a comment outside quotes): `map <key> <context> <command..>`,
//   the command being the remaining arguments joined by one space
//   (0x9F5AF4).
// - Key 0x5C46A0: prefix 0x4098A0 (a spec longer than one character
//   starting with `-` is key-up, flag 2, and loses the `-`; one starting
//   with `+` loses the `+`; every other spec is key-down, flag 1), then the
//   modifiers 0x409CF0 (while the spec holds a `-`, the text before it is
//   looked up in the 0xB27AF0 names; a hit ORs its flag in and drops the
//   text and the `-`, a miss ends the loop), then the key name 0x409960
//   over the 0xB27B20 names (`key_` dropped, case-insensitive).
// - Context 0x5CA0B0: split at `|`; a part naming a flag of the 0xB3B26C
//   table (built by 0x499C50) ORs it in; a part starting with a digit or
//   `-` is the priority (`_wtol`); any other part is the widget, `Root`
//   when none is named.
// - The binding goes to its widget's list (`widget+0x88`, 0x40AC60 puts a
//   new one first). A spec that does not start with `+`, has the down flag
//   and no ACTION or FORMATION flag also gets a key-up binding whose
//   command is the `;` parts with `+x` made `-x` (0x5C9ED0 / 0x5C9D00), or
//   `DefaultKeyUpHandler` (0x9FBC2C) when no part starts with `+`.
// - Find 0x40A610 walks a widget's list for the key; a binding counts when
//   its context holds (0x40FCA0) and its widget exists and is active
//   (0x497220, 0x5EFAB0). A binding whose context is exactly EDITBOX
//   returns at once; otherwise the highest priority wins and, of equal
//   ones, the last in the list (the first registered); more than one at
//   the top priority is logged as duplicate.
// - The key event 0x5EFC70 ignores repeats and compares the flags under
//   0xFFFFF07F; an empty command presses the widget (vt+0x6C), the wheel
//   formats the command with the delta / 1200, any other command runs.
// - OptionsHotkeySelect 0x67B650: the configuration's file
//   (hotkey_configuration_data.tgi `hotkey_file_location`) after 0x5C9BA0
//   dropped every ACTION or FORMATION binding.
// - Root's ctrl-h runs `ToggleHotkeyDisplay`, a command AA.exe does not
//   register (the name is nowhere in the executable), so the key changes
//   nothing.

/// Key flags (0x4098A0, the event's repeat bit, the 0xB27AF0 modifiers).
pub mod key_flag {
    pub const DOWN: u32 = 1;
    pub const UP: u32 = 2;
    pub const REPEAT: u32 = 4;
    pub const ALT: u32 = 0x10;
    pub const SHIFT: u32 = 0x20;
    pub const CTRL: u32 = 0x40;
    /// 0x5EFC70's event mask.
    pub const EVENT_MASK: u32 = 0xFFFF_F07F;
}

/// Context flags (the 0xB3B26C name table).
pub mod context_flag {
    pub const DEV: u32 = 1;
    pub const EDITORDEV: u32 = 2;
    pub const EDITBOX: u32 = 4;
    pub const ALLOW_CHEATS: u32 = 8;
    pub const ACTION: u32 = 0x10;
    pub const FORMATION: u32 = 0x20;
    pub const VERIFY: u32 = 0x40;
    pub const TESTMODE: u32 = 0x80;
}

/// 0x499C50's names for the 0xB3B26C table, in registration order.
pub const CONTEXT_FLAG_NAMES: [(&str, u32); 9] = [
    ("NONE", 0),
    ("DEV", 1),
    ("EDITORDEV", 2),
    ("EDITBOX", 4),
    ("ALLOW_CHEATS", 8),
    ("ACTION", 0x10),
    ("FORMATION", 0x20),
    ("VERIFY", 0x40),
    ("TESTMODE", 0x80),
];

/// 0xB27AF0 modifier names (`key_` dropped) and flags.
pub const MODIFIER_NAMES: [(&str, u32); 5] = [("Alt", 0x10), ("Shift", 0x20), ("Ctrl", 0x40), ("Control", 0x40), ("Ctl", 0x40)];

/// 0xB27B20 key names (`key_` dropped) and codes, in table order.
pub const KEY_NAMES: &[(&str, u32)] = &[
    ("space", 0x20), ("A", 0x41), ("B", 0x42), ("C", 0x43), ("D", 0x44), ("E", 0x45),
    ("F", 0x46), ("G", 0x47), ("H", 0x48), ("I", 0x49), ("J", 0x4a), ("K", 0x4b),
    ("L", 0x4c), ("M", 0x4d), ("N", 0x4e), ("O", 0x4f), ("P", 0x50), ("Q", 0x51),
    ("R", 0x52), ("S", 0x53), ("T", 0x54), ("U", 0x55), ("V", 0x56), ("W", 0x57),
    ("X", 0x58), ("Y", 0x59), ("Z", 0x5a), ("0", 0x30), ("1", 0x31), ("2", 0x32),
    ("3", 0x33), ("4", 0x34), ("5", 0x35), ("6", 0x36), ("7", 0x37), ("8", 0x38),
    ("9", 0x39), ("leftbracket", 0xdb), ("rightbracket", 0xdd), ("[", 0xdb), ("]", 0xdd),
    ("F1", 0x70), ("F2", 0x71), ("F3", 0x72), ("F4", 0x73), ("F5", 0x74), ("F6", 0x75),
    ("F7", 0x76), ("F8", 0x77), ("F9", 0x78), ("F10", 0x79), ("F11", 0x7a), ("F12", 0x7b),
    ("Enter", 0x0d), ("Return", 0x0d), ("Tab", 0x09), ("Escape", 0x1b), ("Esc", 0x1b),
    ("Backspace", 0x08), ("Insert", 0x2d), ("Home", 0x24), ("Pause", 0x13), ("PageUp", 0x21),
    ("PageDown", 0x22), ("Delete", 0x2e), ("End", 0x23), ("Left", 0x25), ("Right", 0x27),
    ("Up", 0x26), ("Down", 0x28), ("numpad-0", 0x60), ("numpad-1", 0x61), ("numpad-2", 0x62),
    ("numpad-3", 0x63), ("numpad-4", 0x64), ("numpad-5", 0x65), ("numpad-6", 0x66),
    ("numpad-7", 0x67), ("numpad-8", 0x68), ("numpad-9", 0x69), ("numpad-left", 0x64),
    ("numpad-right", 0x66), ("numpad-up", 0x68), ("numpad-down", 0x62), ("numpad-home", 0x67),
    ("numpad-pageup", 0x69), ("numpad-end", 0x61), ("numpad-pagedown", 0x63),
    ("numpad-Minus", 0x6d), ("numpad-Plus", 0x6b), ("Equals", 0xbb), ("EqualsSign", 0xbb),
    ("Plus", 0xbb), ("Minus", 0xbd), ("numpad-insert", 0x60), ("numpad-delete", 0x6e),
    ("Grave", 0xc0), ("Tilde", 0xc0), ("PrintScreen", 0x2c), ("Semicolon", 0xba),
    ("Comma", 0xbc), ("Period", 0xbe), ("Slash", 0xbf), ("Backslash", 0xdc), ("Quote", 0xde),
    ("Button1", 0x100), ("Button2", 0x101), ("Button3", 0x102), ("Button4", 0x103),
    ("Button5", 0x104), ("Button6", 0x105), ("Button7", 0x106), ("Button8", 0x107),
    ("MouseZ", 0x110), ("MouseWheel", 0x110), ("Wheel", 0x110), ("ZAxis", 0x110),
];

/// The mouse wheel's key code.
pub const WHEEL: u32 = 0x110;

/// The console's start script for the hotkey files (`exechere` lines).
pub const START_SCRIPT: &str = "Localization/Hotkeys/start.txt";
/// The hotkey configurations (`HotkeyConfigurationData`).
pub const CONFIGURATIONS: &str = "Localization/Hotkeys/Actions/hotkey_configuration_data.tgi";

/// One `map` binding as 0x5C9230 / 0x40AA70 store it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub code: u32,
    pub flags: u32,
    pub context: u32,
    pub widget: String,
    pub command: String,
    pub priority: i32,
}

/// The console's argument split of one line.
pub fn console_tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            '#' if !quoted => break,
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut current));
                    any = false;
                }
            }
            c => {
                current.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(current);
    }
    out
}

/// 0x409960 over the 0xB27B20 names.
pub fn key_code(name: &str) -> Option<u32> {
    KEY_NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, c)| *c)
}

/// 0x5C46A0: the key code and flags of a key spec.
pub fn key_spec(spec: &str) -> Result<(u32, u32), String> {
    let mut flags = 0;
    let mut s: &str = spec;
    // 0x4098A0
    if s.chars().count() != 1 {
        if let Some(rest) = s.strip_prefix('-') {
            flags |= key_flag::UP;
            s = rest;
        } else {
            if let Some(rest) = s.strip_prefix('+') {
                s = rest;
            }
            flags |= key_flag::DOWN;
        }
    } else {
        flags |= key_flag::DOWN;
    }
    // 0x409CF0
    while let Some(i) = s.find('-') {
        match MODIFIER_NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(&s[..i])) {
            Some((_, f)) => {
                flags |= f;
                s = &s[i + 1..];
            }
            None => break,
        }
    }
    let code = key_code(s).ok_or_else(|| format!("key `{s}` of `{spec}` is not in the 0xB27B20 names"))?;
    Ok((code, flags))
}

/// 0x5CA0B0: the context flags, widget and priority of a context.
pub fn context_spec(context: &str) -> (u32, String, i32) {
    let mut flags = 0;
    let mut widget = "Root".to_string();
    let mut priority = 0;
    for part in context.split('|').filter(|p| !p.is_empty()) {
        if let Some((_, f)) = CONTEXT_FLAG_NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(part)) {
            flags |= f;
        } else if part.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
            priority = wtol(part);
        } else {
            widget = part.to_string();
        }
    }
    (flags, widget, priority)
}

/// `_wtol`: leading whitespace, an optional sign, then digits.
fn wtol(s: &str) -> i32 {
    let s = s.trim_start();
    let (neg, digits) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let mut v: i64 = 0;
    for c in digits.chars().take_while(|c| c.is_ascii_digit()) {
        v = (v * 10 + i64::from(c as u8 - b'0')).min(i64::from(i32::MAX) + 1);
    }
    let v = if neg { -v } else { v };
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// 0x5C9ED0 / 0x5C9D00: a key-down command's key-up command.
pub fn key_up_command(command: &str) -> String {
    let parts: Vec<&str> = command.split(';').collect();
    if !parts.iter().any(|p| p.starts_with('+')) {
        return "DefaultKeyUpHandler".into();
    }
    parts
        .iter()
        .map(|p| match p.strip_prefix('+') {
            Some(rest) => format!("-{rest}"),
            None => (*p).to_owned(),
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// The `map` command 0x5CA4B0 over its console arguments (after `map`):
/// the binding and, when it gets one, its key-up binding.
pub fn map_command(args: &[String]) -> Result<Vec<Binding>, String> {
    let spec = args.first().map(String::as_str).unwrap_or("");
    let context = args.get(1).map(String::as_str).unwrap_or("");
    let command = args.get(2..).map(|a| a.join(" ")).unwrap_or_default();
    let (code, flags) = key_spec(spec)?;
    let (context, widget, priority) = context_spec(context);
    let mut out = vec![Binding { code, flags, context, widget: widget.clone(), command: command.clone(), priority }];
    if !spec.starts_with('+') && flags & key_flag::DOWN != 0 && context & (context_flag::ACTION | context_flag::FORMATION) == 0 {
        out.push(Binding { code, flags: (flags & !key_flag::DOWN) | key_flag::UP, context, widget, command: key_up_command(&command), priority });
    }
    Ok(out)
}

/// What a key event of 0x5EFC70 does with the binding Find chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fire {
    /// An empty command: the binding's widget is pressed (vt+0x6C).
    Press(String),
    /// The wheel: the command formatted with the delta / 1200.
    Wheel(String),
    /// The command runs on the console.
    Command(String),
}

/// Find's answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub binding: Option<Binding>,
    /// More than one binding at the top priority (logged as duplicate).
    pub duplicate: bool,
    /// A binding of the key failed its context (0x40FCA0).
    pub invalid_context: bool,
    /// A binding of the key named a missing or inactive widget.
    pub widget_missing: bool,
}

/// The hotkey table: each widget's list by name.
#[derive(Clone, Debug, Default)]
pub struct HotkeyTable {
    lists: Vec<(String, Vec<Binding>)>,
    /// The `map` lines the console ran, as it logs them (`map` and its
    /// arguments joined by one space), in order.
    pub executed: Vec<String>,
    /// Console lines this table does not run (neither `map` nor `exechere`).
    pub skipped: Vec<String>,
    /// Lines whose `map` failed.
    pub errors: Vec<String>,
}

impl HotkeyTable {
    /// 0x40AC60: the binding goes first in its widget's list.
    pub fn add(&mut self, b: Binding) {
        match self.lists.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(&b.widget)) {
            Some((_, list)) => list.insert(0, b),
            None => self.lists.push((b.widget.clone(), vec![b])),
        }
    }

    /// A widget's list, newest first.
    pub fn list(&self, widget: &str) -> &[Binding] {
        self.lists.iter().find(|(n, _)| n.eq_ignore_ascii_case(widget)).map_or(&[], |(_, l)| l.as_slice())
    }

    /// Every widget that has a list, in first-binding order.
    pub fn widgets(&self) -> impl Iterator<Item = &str> {
        self.lists.iter().map(|(n, _)| n.as_str())
    }

    /// Every binding, by widget, newest first.
    pub fn all(&self) -> impl Iterator<Item = &Binding> {
        self.lists.iter().flat_map(|(_, l)| l.iter())
    }

    /// 0x5C9BA0: drop every ACTION or FORMATION binding.
    pub fn clear_action_bindings(&mut self) {
        for (_, list) in &mut self.lists {
            list.retain(|b| b.context & (context_flag::ACTION | context_flag::FORMATION) == 0);
        }
    }

    /// Run one console line: `map` binds, `exechere` runs the named file
    /// at once (the start script's order: each file fully before the next).
    pub fn exec_line(&mut self, line: &str, read: &dyn Fn(&str) -> Option<String>) {
        let t = console_tokens(line);
        let Some(head) = t.first() else { return };
        if head.eq_ignore_ascii_case("map") {
            self.executed.push(t.join(" "));
            match map_command(&t[1..]) {
                Ok(bs) => {
                    for b in bs {
                        self.add(b);
                    }
                }
                Err(e) => self.errors.push(format!("{line}: {e}")),
            }
        } else if head.eq_ignore_ascii_case("exechere") {
            let path = t.get(1).map(String::as_str).unwrap_or("");
            match read(path) {
                Some(text) => self.exec_text(&text, read),
                None => self.errors.push(format!("{line}: file not found")),
            }
        } else {
            self.skipped.push(t.join(" "));
        }
    }

    /// Run every line of a console file.
    pub fn exec_text(&mut self, text: &str, read: &dyn Fn(&str) -> Option<String>) {
        for line in text.lines() {
            self.exec_line(line, read);
        }
    }

    /// OptionsHotkeySelect 0x67B650 with the configuration's file.
    pub fn select(&mut self, file: &str, read: &dyn Fn(&str) -> Option<String>) -> Result<(), String> {
        self.clear_action_bindings();
        let text = read(file).ok_or_else(|| format!("load_error_missing_map: {file}"))?;
        self.exec_text(&text, read);
        Ok(())
    }

    /// The start script, then the configuration file.
    pub fn load(read: &dyn Fn(&str) -> Option<String>, configuration_file: Option<&str>) -> Self {
        let mut t = Self::default();
        match read(START_SCRIPT) {
            Some(text) => t.exec_text(&text, read),
            None => t.errors.push(format!("{START_SCRIPT}: file not found")),
        }
        if let Some(f) = configuration_file {
            if let Err(e) = t.select(f, read) {
                t.errors.push(e);
            }
        }
        t
    }

    /// Find 0x40A610 over `widget`'s list. `context(flags, command)` is
    /// 0x40FCA0; `active(widget)` is 0x497220 + 0x5EFAB0. `skip_checks`
    /// is the second argument (the checks are not made).
    pub fn find(
        &self,
        widget: &str,
        code: u32,
        flags: u32,
        skip_checks: bool,
        mut context: impl FnMut(u32, &str) -> bool,
        mut active: impl FnMut(&str) -> bool,
    ) -> Found {
        let mut found = Found::default();
        let mut best = i32::MIN;
        let mut ties = 0;
        for b in self.list(widget) {
            if b.code != code || b.flags != flags {
                continue;
            }
            if !skip_checks {
                if !context(b.context, &b.command) {
                    found.invalid_context = true;
                    continue;
                }
                if !active(&b.widget) {
                    found.widget_missing = true;
                    continue;
                }
            }
            if b.context == context_flag::EDITBOX {
                found.binding = Some(b.clone());
                found.duplicate = false;
                return found;
            }
            if b.priority >= best {
                found.binding = Some(b.clone());
                if b.priority > best {
                    ties = 0;
                } else {
                    ties += 1;
                }
                best = b.priority;
            }
        }
        found.duplicate = !skip_checks && ties > 0;
        found
    }

    /// The key event 0x5EFC70 on `widget`'s list.
    pub fn key_event(
        &self,
        widget: &str,
        code: u32,
        flags: u32,
        context: impl FnMut(u32, &str) -> bool,
        active: impl FnMut(&str) -> bool,
    ) -> Option<Fire> {
        if flags & key_flag::REPEAT != 0 {
            return None;
        }
        let b = self.find(widget, code, flags & key_flag::EVENT_MASK, false, context, active).binding?;
        Some(if b.command.is_empty() {
            Fire::Press(b.widget)
        } else if code == WHEEL {
            Fire::Wheel(b.command)
        } else {
            Fire::Command(b.command)
        })
    }
}

impl Ui {
    /// The hotkey table of the start script and the `HotkeyConfiguration`
    /// UVar's configuration file.
    pub fn hotkey_table(&mut self) -> HotkeyTable {
        let ids = self.uvars.get_str("HotkeyConfiguration");
        let file = self.store.load_file(CONFIGURATIONS).ok().and_then(|root| {
            root.children
                .iter()
                .find(|n| n.string_or_empty("IDS").eq_ignore_ascii_case(&ids))
                .map(|n| n.string_or_empty("hotkey_file_location"))
        });
        let root = &self.store.root;
        let read = |p: &str| root.read(p).ok().map(|b| crate::data::decode(&b));
        HotkeyTable::load(&read, file.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_command_decodes_the_original_forms() {
        let m = |line: &str| map_command(&console_tokens(line)[1..]).unwrap();
        // ACTION | VERIFY on the game widget: no key-up binding
        let b = m("map a game|action|verify \"ActorDataSelectCommand regiment_usa_paratrooper\"");
        assert_eq!(b, vec![Binding { code: 0x41, flags: 1, context: 0x50, widget: "game".into(), command: "ActorDataSelectCommand regiment_usa_paratrooper".into(), priority: 0 }]);
        // a priority part
        let b = m("map 1 game|action|verify|1 \"SelectTeamKingdom 1\"");
        assert_eq!((b[0].code, b[0].context, b[0].priority, b.len()), (0x31, 0x50, 1, 1));
        // a widget press: empty command, the widget from the context
        let b = m("map q CompanyFormationButtons_0|formation ");
        assert_eq!((b[0].widget.as_str(), b[0].context, b[0].command.as_str(), b.len()), ("CompanyFormationButtons_0", 0x20, "", 1));
        let b = m("map f1 ButtonObjectives");
        assert_eq!((b[0].widget.as_str(), b[0].command.as_str(), b[1].flags, b[1].command.as_str()), ("ButtonObjectives", "", 2, "DefaultKeyUpHandler"));
        // modifiers, numpad names, key-up specs
        let b = m("map ctrl-shift-alt-f10 SAIDebug \"SAIDisplayKingdomData 10\"");
        assert_eq!((b[0].code, b[0].flags), (0x79, 0x71));
        assert_eq!(key_spec("numpad-left").unwrap(), (0x64, 1));
        assert_eq!(key_spec("shift-minus").unwrap(), (0xbd, 0x21));
        assert_eq!(key_spec("-left").unwrap(), (0x25, 2));
        assert_eq!(key_spec("+button3").unwrap(), (0x102, 1));
        let b = m("map numpad-left vworld +yawleft");
        assert_eq!((b[1].flags, b[1].command.as_str()), (2, "-yawleft"));
        let b = m("map +left vworld beginscrollleft");
        assert_eq!(b.len(), 1);
        // dev | root: a flag and the widget
        let b = m("map ctrl-k dev|root ToggleSAIDebug");
        assert_eq!((b[0].context, b[0].widget.as_str()), (1, "root"));
        // no widget named: Root
        let b = m("map up console|editbox ConsolePreviousCommand");
        assert_eq!((b[0].context, b[0].widget.as_str()), (4, "console"));
        assert_eq!(context_spec("editbox").1, "Root");
    }

    #[test]
    fn find_takes_the_top_priority_then_the_first_registered() {
        let mut t = HotkeyTable::default();
        let read = |_: &str| None;
        t.exec_line("map 1 game|action|verify \"SelectTeamKingdom\"", &read);
        t.exec_line("map 1 game|action|verify|1 \"SelectTeamKingdom 1\"", &read);
        t.exec_line("map 1 game|action|verify|1 \"SelectObservingKingdom 0\"", &read);
        let f = t.find("game", 0x31, 1, false, |_, _| true, |_| true);
        assert_eq!(f.binding.unwrap().command, "SelectTeamKingdom 1");
        assert!(f.duplicate);
        // only the observer's button is shown
        let f = t.find("game", 0x31, 1, false, |_, c| c.starts_with("SelectObserving"), |_| true);
        assert_eq!(f.binding.unwrap().command, "SelectObservingKingdom 0");
        assert!(f.invalid_context && !f.duplicate);
        // a repeat is ignored, the modifiers count
        assert_eq!(t.key_event("game", 0x31, 1 | key_flag::REPEAT, |_, _| true, |_| true), None);
        assert_eq!(t.key_event("game", 0x31, 1 | key_flag::CTRL, |_, _| true, |_| true), None);
        // 0x5C9BA0 drops the action bindings
        t.clear_action_bindings();
        assert!(t.list("game").is_empty());
    }

    /// The shipped hotkey files (stock data, no overlay).
    const FILES: [&str; 11] = [
        "Localization/Hotkeys/start.txt",
        "Localization/Hotkeys/hotkeys_core.txt",
        "Localization/Hotkeys/hotkeys_ui.txt",
        "Localization/Hotkeys/hotkeys_groups.txt",
        "Localization/Hotkeys/hotkeys_favorites.txt",
        "Localization/Hotkeys/hotkeys_glyphs.txt",
        "Localization/Hotkeys/hotkeys_custom.txt",
        "Localization/Hotkeys/Actions/hotkeys_standard_aa.txt",
        "Localization/Hotkeys/Actions/hotkeys_visual_aa.txt",
        "Localization/Hotkeys/Actions/hotkeys_visual_dvorak_aa.txt",
        "Localization/Hotkeys/Actions/hotkeys_visual_left_handed_aa.txt",
    ];

    fn stock() -> crate::fs::DataRoot {
        crate::fs::DataRoot::new(crate::fs::DataRoot::reconstruction_dir().join("assets/original/Data"))
    }

    fn game_root() -> std::path::PathBuf {
        std::env::var_os("AA_GAME_ROOT").map(Into::into).unwrap_or_else(|| crate::fs::DataRoot::reconstruction_dir().join(".."))
    }

    /// Every line of every shipped file binds (each key name resolves), and
    /// the start script plus the classic configuration runs the same `map`
    /// lines, in the same order and with the same arguments, as the
    /// original's own log of its start (`executing "map ..."`).
    #[test]
    fn every_shipped_line_matches_the_original() {
        let root = stock();
        if !root.exists(START_SCRIPT) {
            eprintln!("skipped: no stock data");
            return;
        }
        let read = |p: &str| root.read(p).ok().map(|b| crate::data::decode(&b));
        let mut lines = 0;
        let (mut actor_select, mut property_select) = (0, 0);
        for f in FILES {
            let text = read(f).unwrap_or_else(|| panic!("{f}"));
            for line in text.lines() {
                let t = console_tokens(line);
                if !t.first().is_some_and(|h| h.eq_ignore_ascii_case("map")) {
                    continue;
                }
                lines += 1;
                let b = map_command(&t[1..]).unwrap_or_else(|e| panic!("{f}: {line}: {e}"));
                if b[0].command.starts_with("ActorDataSelectCommand ") {
                    actor_select += 1;
                    assert_eq!((b[0].widget.as_str(), b[0].context, b.len()), ("game", 0x50, 1), "{line}");
                }
                if b[0].command.starts_with("PropertyDataSelectCommand ") {
                    property_select += 1;
                    assert_eq!((b[0].widget.as_str(), b[0].context, b.len()), ("game", 0x50, 1), "{line}");
                }
            }
        }
        // 239 ActorDataSelectCommand lines, 5 of them commented out
        assert_eq!((actor_select, property_select), (234, 46));
        assert!(lines > 800, "{lines}");
        let classic = HotkeyTable::load(&read, Some("Localization/Hotkeys/Actions/hotkeys_standard_aa.txt"));
        assert!(classic.errors.is_empty(), "{:?}", classic.errors);
        assert!(classic.skipped.is_empty(), "{:?}", classic.skipped);
        assert_eq!(classic.executed.len(), 763);
        // the original's logs of its start
        let logs = std::fs::read_dir(game_root().join("Logs")).map(|d| d.flatten().map(|e| e.path()).collect::<Vec<_>>()).unwrap_or_default();
        let mut matched = Vec::new();
        for p in logs.iter().filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("log"))) {
            let Ok(bytes) = std::fs::read(p) else { continue };
            let text = String::from_utf8_lossy(&bytes);
            let mut seen: Option<Vec<String>> = None;
            for line in text.lines() {
                let Some(at) = line.find("executing \"") else { continue };
                let cmd = line[at + "executing \"".len()..].trim_end().strip_suffix('"').unwrap_or("");
                match &mut seen {
                    None if cmd.eq_ignore_ascii_case(&format!("ExecHere {START_SCRIPT}")) => seen = Some(Vec::new()),
                    None => {}
                    Some(v) if cmd.to_ascii_lowercase().starts_with("map ") => v.push(cmd.to_string()),
                    Some(_) if cmd.to_ascii_lowercase().starts_with("exechere ") => {}
                    Some(_) => break,
                }
            }
            if let Some(v) = seen {
                if v == classic.executed {
                    matched.push(p.clone());
                } else {
                    let first = v.iter().zip(&classic.executed).position(|(a, b)| a != b);
                    eprintln!("{}: {} lines, first difference at {first:?}", p.display(), v.len());
                }
            }
        }
        if logs.is_empty() {
            eprintln!("skipped the log comparison: no Logs folder");
        } else {
            assert!(!matched.is_empty(), "no log of the original's start matches");
            eprintln!("matched {} logs", matched.len());
        }
    }

    /// The classic configuration on the Game interface's list: a VERIFY key
    /// fires the command of a shown button; the observer's priority 1 keys
    /// win over the group keys; of equal priorities the first registered.
    #[test]
    fn classic_game_keys() {
        let root = stock();
        if !root.exists(START_SCRIPT) {
            return;
        }
        let read = |p: &str| root.read(p).ok().map(|b| crate::data::decode(&b));
        let t = HotkeyTable::load(&read, Some("Localization/Hotkeys/Actions/hotkeys_standard_aa.txt"));
        let shown = |buttons: &'static [&'static str]| move |flags: u32, cmd: &str| flags & context_flag::VERIFY == 0 || buttons.iter().any(|b| b.eq_ignore_ascii_case(cmd));
        let fire = |code: u32, flags: u32, buttons: &'static [&'static str]| t.key_event("game", code, flags, shown(buttons), |_| true);
        let cmd = |s: &str| Some(Fire::Command(s.to_string()));
        // an HQ's recruit page
        assert_eq!(fire(0x45, 1, &["ActorDataSelectCommand regiment_usa_tank_medium"]), cmd("ActorDataSelectCommand regiment_usa_tank_medium"));
        assert_eq!(fire(0x45, 1, &[]), None);
        // a research page
        assert_eq!(fire(0x56, 1, &["PropertyDataSelectCommand tech_heavy_tank"]), cmd("PropertyDataSelectCommand tech_heavy_tank"));
        // kill and CommandAll on `a`: the first registered
        assert_eq!(fire(0x41, 1, &["CommandAll", "ActorCommand kill"]), cmd("ActorCommand kill"));
        // the observer's 1 over SelectGroup group_1, which holds without a button
        assert_eq!(fire(0x31, 1, &["SelectObservingKingdom 0"]), cmd("SelectObservingKingdom 0"));
        assert_eq!(fire(0x31, 1, &[]), cmd("SelectGroup group_1"));
        // hotkeys_custom.txt on the Game interface
        assert_eq!(fire(0x74, 1, &[]), cmd("CycleSelection 2"));
        assert_eq!(fire(0x48, 1, &[]), cmd("UseFactionAbility 1"));
        // ctrl-h is Root's (ToggleHotkeyDisplay, a command AA.exe does not have)
        assert_eq!(fire(0x48, 1 | key_flag::CTRL, &[]), None);
        // Root's list: in an edit box the glyph (EDITBOX returns at once),
        // elsewhere ToggleHotkeyDisplay
        let editing = |editbox: bool| move |flags: u32, _: &str| flags & context_flag::EDITBOX == 0 || editbox;
        assert_eq!(t.key_event("Root", 0x48, 0x41, editing(true), |_| true), cmd("EditboxAddGlyph ch_HALFTRACK"));
        assert_eq!(t.key_event("root", 0x48, 0x41, editing(false), |_| true), cmd("ToggleHotkeyDisplay"));
        // the formation buttons' own lists press the widget
        assert_eq!(t.key_event("CompanyFormationButtons_0", 0x51, 1, |_, _| true, |_| true), Some(Fire::Press("CompanyFormationButtons_0".into())));
        // the visual configuration replaces them: q presses action button 0
        let mut v = t.clone();
        v.select("Localization/Hotkeys/Actions/hotkeys_visual_aa.txt", &read).unwrap();
        assert_eq!(v.key_event("ActionButtonsButton_0", 0x51, 1, |_, _| true, |_| true), Some(Fire::Press("ActionButtonsButton_0".into())));
        assert_eq!(v.key_event("CompanyFormationButtons_0", 0x51, 1, |_, _| true, |_| true), None);
        assert_eq!(v.key_event("game", 0x45, 1, shown(&["ActorDataSelectCommand regiment_usa_tank_medium"]), |_| true), None);
    }

    /// The key and modifier names are the original's tables (0xB27B20,
    /// 0xB27AF0 in AA.exe: pairs of a `key_` name and its value).
    #[test]
    fn key_tables_are_the_originals() {
        let Ok(exe) = std::fs::read(game_root().join("AA.exe")) else {
            eprintln!("skipped: no AA.exe");
            return;
        };
        let u16le = |o: usize| u16::from_le_bytes([exe[o], exe[o + 1]]);
        let u32le = |o: usize| u32::from_le_bytes([exe[o], exe[o + 1], exe[o + 2], exe[o + 3]]);
        let pe = u32le(0x3c) as usize;
        let sections = u16le(pe + 6) as usize;
        let base = u32le(pe + 24 + 28);
        let first = pe + 24 + u16le(pe + 20) as usize;
        let file_offset = |va: u32| -> Option<usize> {
            (0..sections).find_map(|i| {
                let s = first + i * 40;
                let (vsize, rva, rsize, raw) = (u32le(s + 8), u32le(s + 12), u32le(s + 16), u32le(s + 20));
                let r = va.checked_sub(base)?;
                (r >= rva && r < rva + vsize.max(rsize)).then(|| (raw + r - rva) as usize)
            })
        };
        let wide = |va: u32| -> Option<String> {
            let mut o = file_offset(va)?;
            let mut out = Vec::new();
            while o + 1 < exe.len() && u16le(o) != 0 {
                out.push(u16le(o));
                o += 2;
            }
            String::from_utf16(&out).ok()
        };
        let table = |va: u32| -> Vec<(String, u32)> {
            let mut out = Vec::new();
            let mut o = file_offset(va).expect("table");
            while let Some(name) = wide(u32le(o)).and_then(|n| n.strip_prefix("key_").map(str::to_string)) {
                out.push((name, u32le(o + 4)));
                o += 8;
            }
            out
        };
        let keys = table(0xB27B20);
        let ours: Vec<(String, u32)> = KEY_NAMES.iter().map(|(n, c)| (n.to_string(), *c)).collect();
        assert_eq!(keys, ours);
        let mods = table(0xB27AF0);
        let ours: Vec<(String, u32)> = MODIFIER_NAMES.iter().map(|(n, c)| (n.to_string(), *c)).collect();
        assert_eq!(mods, ours);
    }

    #[test]
    fn parse_lines() {
        let h = parse("# x\nmap esc MainMenuButtonQuit\nmap enter OptionsDialog OptionsOK\nmap esc CreateGameDialog \"CreateGameCancel\"\n");
        assert_eq!(h.len(), 3);
        assert_eq!(h[0], Hotkey { key: "esc".into(), context: None, target: "MainMenuButtonQuit".into() });
        assert_eq!(h[1].context.as_deref(), Some("OptionsDialog"));
        assert_eq!(h[2].target, "CreateGameCancel");
    }
}
