//! Camera subset of KKC_Hotkeys (Hotkeys.cpp) for the original binding file
//! `Data/Localization/Hotkeys/hotkeys_core.txt` (executed first by start.txt).
//!
//! - `map` command 5CA4B0: key spec 5C46A0 = prefix 4098A0 (`-x` key-up flag 2,
//!   `+x`/plain key-down flag 1, a one-character spec has no prefix), modifier
//!   list 409CF0, key name 409960 over the KKC_KeyNameList table B27B20
//!   (`key_` names, _wcsicmp). A binding whose spec does not start with `+`,
//!   has the down flag and no 0x30 context restriction also gets a key-up
//!   binding: `+cmd` tokens become `-cmd` (5C9ED0/5C9D00), otherwise
//!   `DefaultKeyUpHandler` (registered as 943800, `RET 4`).
//! - Events 5EFC70: repeat (flag bit 2) is ignored, lookup key is
//!   `{code, flags & 0xFFFFF07F}` (modifiers 0x10 alt, 0x20 shift, 0x40 ctrl
//!   from 59A5C0/59A6E0 stay significant). The wheel (code 0x110) command is
//!   vswprintf-formatted with `(float)(short)delta/1200` (see input module).
//! - `+name` / `-name` commands become `RegisterImpulse name` /
//!   `UnregisterImpulse name` (426950).
//!
//! Only lines whose context is exactly `vworld` or `game` and whose command is
//! one of the camera commands are retained; other bindings belong to their
//! own owners. Modifier-prefixed camera specs are refused (none exist in the
//! original file). Context validity and priority (40FCA0/497220/5EFAB0 widget
//! lookup) remain the frontend's explicit answer per context name.
type Result<T> = std::result::Result<T, String>;

pub const DOWN: u32 = 1;
pub const UP: u32 = 2;
pub const REPEAT: u32 = 4;
pub const ALT: u32 = 0x10;
pub const SHIFT: u32 = 0x20;
pub const CTRL: u32 = 0x40;
pub const WHEEL: u32 = 0x110;

///B27B20 KKC_KeyNameList source pairs (names without the `key_` prefix).
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
pub fn key_code_409960(name: &str) -> Option<u32> {
    KEY_NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, c)| *c)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: u32,
    pub flags: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub key: Key,
    pub context: String,
    pub command: String,
    pub line: usize,
}
#[derive(Clone, Debug, Default)]
pub struct Bindings {
    list: Vec<Binding>,
}

const SCROLL: [&str; 8] = [
    "beginscrollleft", "beginscrollright", "beginscrollup", "beginscrolldown",
    "endscrollleft", "endscrollright", "endscrollup", "endscrolldown",
];
const IMPULSES: [&str; 6] = ["yawleft", "yawright", "pitchup", "pitchdown", "zoomin", "zoomout"];
fn camera_command(command: &str) -> bool {
    let head = command.split_whitespace().next().unwrap_or("");
    let lower = head.to_ascii_lowercase();
    if SCROLL.contains(&lower.as_str())
        || matches!(lower.as_str(), "deltazoom" | "zoommin" | "zoommax" | "begincamerarotate" | "endcamerarotate" | "defaultangles")
    {
        return true;
    }
    lower
        .strip_prefix(['+', '-'])
        .is_some_and(|rest| IMPULSES.contains(&rest))
}
///Console tokenizer subset: whitespace separation, double quotes group,
///`#` starts a comment outside quotes.
fn tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true
            }
            '#' if !quoted => break,
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut current));
                    any = false
                }
            }
            c => {
                current.push(c);
                any = true
            }
        }
    }
    if any {
        out.push(current)
    }
    out
}
///5C9ED0/5C9D00 over the `;` separated command tokens.
fn release_command(command: &str) -> String {
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
impl Bindings {
    ///Parse the camera bindings of the actual hotkeys_core.txt text.
    pub fn parse_core(text: &str) -> Result<Self> {
        let mut list = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let t = tokens(line);
            if t.len() != 4 || !t[0].eq_ignore_ascii_case("map") {
                continue;
            }
            let (spec, context, command) = (&t[1], &t[2], &t[3]);
            if !(context.eq_ignore_ascii_case("vworld") || context.eq_ignore_ascii_case("game"))
                || !camera_command(command)
            {
                continue;
            }
            let mut name = spec.as_str();
            let mut flags = 0;
            let plus = spec.starts_with('+');
            if spec.chars().count() != 1 && spec.starts_with('-') {
                flags |= UP;
                name = &spec[1..];
            } else {
                if spec.chars().count() != 1 && plus {
                    name = &spec[1..];
                }
                flags |= DOWN;
            }
            let lower = name.to_ascii_lowercase();
            if ["ctrl-", "alt-", "shift-"].iter().any(|m| lower.starts_with(m)) {
                return Err(format!("hotkeys_core.txt line {}: modifier camera hotkey needs 409CF0", index + 1));
            }
            let code = key_code_409960(name)
                .ok_or_else(|| format!("hotkeys_core.txt line {}: key `{name}` not in B27B20", index + 1))?;
            let key = Key { code, flags };
            list.push(Binding { key, context: context.clone(), command: command.clone(), line: index + 1 });
            if !plus && flags & DOWN != 0 {
                list.push(Binding {
                    key: Key { code, flags: (flags & !DOWN) | UP },
                    context: context.clone(),
                    command: release_command(command),
                    line: index + 1,
                });
            }
        }
        Ok(Self { list })
    }
    pub fn list(&self) -> &[Binding] {
        &self.list
    }
    ///5EFC70 key selection (repeat ignored; flags masked by 0xFFFFF07F).
    ///Among valid contexts the first binding is returned; the actual widget
    ///priority (40A610 +14) is the frontend's `context_active` decision.
    pub fn lookup(
        &self,
        code: u32,
        flags: u32,
        mut context_active: impl FnMut(&str) -> Result<bool>,
    ) -> Result<Option<&Binding>> {
        if flags & REPEAT != 0 {
            return Ok(None);
        }
        let key = Key { code, flags: flags & 0xffff_f07f };
        for binding in &self.list {
            if binding.key == key && context_active(&binding.context)? {
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }
}

///Executable camera commands (426C20 name dispatch, case-insensitive).
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    BeginScroll(u32),
    EndScroll(u32),
    RegisterImpulse(String),
    UnregisterImpulse(String),
    DeltaZoom(String),
    ZoomMin,
    ZoomMax,
    BeginCameraRotate,
    EndCameraRotate,
    DefaultKeyUpHandler,
}
impl Command {
    ///Translate one command string (after 426950 `+`/`-` rewriting).
    pub fn parse(command: &str) -> Result<Self> {
        use super::input::{SCROLL_DOWN, SCROLL_LEFT, SCROLL_RIGHT, SCROLL_UP};
        let command = command.trim();
        if let Some(rest) = command.strip_prefix('+') {
            return Ok(Self::RegisterImpulse(rest.trim().to_owned()));
        }
        if let Some(rest) = command.strip_prefix('-') {
            return Ok(Self::UnregisterImpulse(rest.trim().to_owned()));
        }
        let mut parts = command.splitn(2, char::is_whitespace);
        let head = parts.next().unwrap_or("").to_ascii_lowercase();
        let arg = parts.next().unwrap_or("").trim().to_owned();
        Ok(match head.as_str() {
            "beginscrollleft" => Self::BeginScroll(SCROLL_LEFT),
            "beginscrollright" => Self::BeginScroll(SCROLL_RIGHT),
            "beginscrollup" => Self::BeginScroll(SCROLL_UP),
            "beginscrolldown" => Self::BeginScroll(SCROLL_DOWN),
            "endscrollleft" => Self::EndScroll(SCROLL_LEFT),
            "endscrollright" => Self::EndScroll(SCROLL_RIGHT),
            "endscrollup" => Self::EndScroll(SCROLL_UP),
            "endscrolldown" => Self::EndScroll(SCROLL_DOWN),
            "registerimpulse" => Self::RegisterImpulse(arg),
            "unregisterimpulse" => Self::UnregisterImpulse(arg),
            "deltazoom" => Self::DeltaZoom(arg),
            "zoommin" => Self::ZoomMin,
            "zoommax" => Self::ZoomMax,
            "begincamerarotate" => Self::BeginCameraRotate,
            "endcamerarotate" => Self::EndCameraRotate,
            "defaultkeyuphandler" => Self::DefaultKeyUpHandler,
            _ => return Err(format!("camera command `{command}` has no ported handler")),
        })
    }
}
///vswprintf `%f` of the wheel argument then `_wtof` and the binary32 store of
///4257D0. Rust's correctly rounded decimal conversion is used for both steps
///(the MSVC CRT conversion of these short fixed-point strings is assumed to
///round correctly; recorded as a domain limit in the findings).
pub fn wheel_command_value(argument: u64) -> Result<(String, u32)> {
    let value = f64::from_bits(argument);
    if !value.is_finite() {
        return Err("wheel argument is not finite".into());
    }
    let text = format!("{value:.6}");
    let parsed: f64 = text.parse().map_err(|_| "wheel %f text did not parse")?;
    Ok((text, (parsed as f32).to_bits()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_core_camera_bindings() {
        let text = include_str!("../../../../reconstruction/assets/original/Data/Localization/Hotkeys/hotkeys_core.txt");
        let b = Bindings::parse_core(text).unwrap();
        let find = |code, flags| b.lookup(code, flags, |_| Ok(true)).unwrap().map(|b| b.command.clone());
        assert_eq!(find(0x25, DOWN).as_deref(), Some("beginscrollleft"));
        assert_eq!(find(0x25, UP).as_deref(), Some("endscrollleft"));
        assert_eq!(find(0x26, DOWN).as_deref(), Some("beginscrollup"));
        assert_eq!(find(0x28, UP).as_deref(), Some("endscrolldown"));
        assert_eq!(find(0x25, DOWN | REPEAT), None);
        assert_eq!(find(0x25, DOWN | SHIFT), None);
        assert_eq!(find(0x22, DOWN).as_deref(), Some("+zoomout"));
        assert_eq!(find(0x22, UP).as_deref(), Some("-zoomout"));
        assert_eq!(find(0x21, DOWN).as_deref(), Some("+zoomin"));
        assert_eq!(find(0x64, DOWN).as_deref(), Some("+yawleft"));
        assert_eq!(find(0x64, UP).as_deref(), Some("-yawleft"));
        assert_eq!(find(0x24, DOWN).as_deref(), Some("ZoomMin"));
        assert_eq!(find(0x24, UP).as_deref(), Some("DefaultKeyUpHandler"));
        assert_eq!(find(0x102, DOWN).as_deref(), Some("begincamerarotate"));
        assert_eq!(find(0x102, UP).as_deref(), Some("endcamerarotate"));
        assert_eq!(find(WHEEL, DOWN).as_deref(), Some("deltazoom %f"));
        assert_eq!(Command::parse("+zoomin").unwrap(), Command::RegisterImpulse("zoomin".into()));
        assert_eq!(Command::parse("deltazoom 0.100000").unwrap(), Command::DeltaZoom("0.100000".into()));
        let (text, value) = wheel_command_value(0.1f64.to_bits()).unwrap();
        assert_eq!(text, "0.100000");
        assert_eq!(value, 0.1f32.to_bits());
    }
}
