//! Widget state, `when` masks and label override attributes.
//!
//! * `0x5E4D00` builds the `when` keyword table (hash container at
//!   0xB3DEF4/0xB3DEF8); values read from its instruction stream by
//!   `research/phase8-ui/native_layout.py` (`when_table`).
//! * `0x5E54A0` parses a `when` string: tokenizer `0x424FA0`/`0x412480`
//!   with delimiters `"+ "` (0xA511D4), case-insensitive lookup (hash of
//!   upper-cased characters, `__wcsicmp`), OR of every keyword's bits.
//! * `0x5E5210` builds the label `attributes` table used by `0x5E5BA0`.
//! * `0x5E25C0` derives a view element's state bits from its widget's flags
//!   and decides visibility: `shown && (when & state) == when`, where
//!   `shown` is `0x5EE2C0` (widget flag bit 1 and `render_threshold` >= the
//!   current UI render level).

use serde::{Deserialize, Serialize};

/// `when` keyword bits (`0x5E4D00`).
pub mod when {
    pub const ALWAYS: u32 = 0;
    pub const ENABLED: u32 = 0x1;
    pub const DISABLED: u32 = 0x2;
    pub const FOCUSED: u32 = 0x4;
    pub const UNFOCUSED: u32 = 0x8;
    pub const PRESSED: u32 = 0x10;
    pub const UNPRESSED: u32 = 0x20;
    pub const SELECTED: u32 = 0x40;
    pub const UNSELECTED: u32 = 0x80;
    pub const OPEN: u32 = 0x100;
    pub const CLOSED: u32 = 0x200;
    pub const TARGETED: u32 = 0x400;
    pub const UNTARGETED: u32 = 0x800;
}

/// The `when` keyword table in native insertion order.
pub const WHEN_KEYWORDS: [(&str, u32); 13] = [
    ("ALWAYS", when::ALWAYS),
    ("ENABLED", when::ENABLED),
    ("DISABLED", when::DISABLED),
    ("FOCUSED", when::FOCUSED),
    ("UNFOCUSED", when::UNFOCUSED),
    ("PRESSED", when::PRESSED),
    ("UNPRESSED", when::UNPRESSED),
    ("SELECTED", when::SELECTED),
    ("UNSELECTED", when::UNSELECTED),
    ("OPEN", when::OPEN),
    ("CLOSED", when::CLOSED),
    ("TARGETED", when::TARGETED),
    ("UNTARGETED", when::UNTARGETED),
];

/// Label override `attributes` bits (`0x5E5210`).
pub mod attribute {
    pub const NONE: u32 = 0;
    pub const TEXT: u32 = 0x1;
    pub const FONT: u32 = 0x2;
    pub const JUSTIFY: u32 = 0x4;
    pub const COLOR: u32 = 0x8;
    pub const BACKGROUND_COLOR: u32 = 0x10;
}

/// The label attribute keyword table in native insertion order.
pub const ATTRIBUTE_KEYWORDS: [(&str, u32); 6] = [
    ("NONE", attribute::NONE),
    ("TEXT", attribute::TEXT),
    ("FONT", attribute::FONT),
    ("JUSTIFY", attribute::JUSTIFY),
    ("COLOR", attribute::COLOR),
    ("BACKGROUND_COLOR", attribute::BACKGROUND_COLOR),
];

/// Result of parsing a keyword mask string.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MaskParse {
    pub bits: u32,
    /// Tokens not in the table. The original logs "Unknown when value" /
    /// "Unknown attribute value" and then ORs in the value of the previously
    /// matched keyword (a stale local, uninitialised before the first match);
    /// the port reproduces the OR of the previous match and reports the token.
    pub unknown: Vec<String>,
}

/// Tokenize like `0x412480` with delimiter set `"+ "`: skip leading
/// whitespace, split on any of `+`/space; quoted tokens keep their content.
pub fn tokenize_plus(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() || c == '+' {
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            let q = c;
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != q {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !(chars[i] == '+' || chars[i] == ' ') && !chars[i].is_whitespace() {
            i += 1;
        }
        out.push(chars[start..i].iter().collect());
    }
    out
}

fn parse_mask(text: &str, table: &[(&str, u32)]) -> MaskParse {
    let mut out = MaskParse::default();
    let mut previous: Option<u32> = None;
    for token in tokenize_plus(text) {
        match table.iter().find(|(k, _)| k.eq_ignore_ascii_case(&token)) {
            Some(&(_, v)) => {
                previous = Some(v);
                out.bits |= v;
            }
            None => {
                if let Some(v) = previous {
                    out.bits |= v;
                }
                out.unknown.push(token);
            }
        }
    }
    out
}

/// `0x5E54A0`: parse a `when` value (`ENABLED + FOCUSED + UNPRESSED`).
pub fn parse_when(text: &str) -> MaskParse {
    parse_mask(text, &WHEN_KEYWORDS)
}

/// Label `[Override] attributes` (`0x5E5BA0` with the `0x5E5210` table).
pub fn parse_attributes(text: &str) -> MaskParse {
    parse_mask(text, &ATTRIBUTE_KEYWORDS)
}

/// Widget flag bits (widget+0x24) read by the UI state logic.
pub mod widget_flag {
    /// Widget is shown (`0x5EE2C0` tests bit 1).
    pub const SHOWN: u32 = 1 << 1;
    /// Keyboard/mouse focus.
    pub const FOCUSED: u32 = 1 << 3;
    /// Active (`0x5EE1D0`: set by Activate, cleared by Deactivate).
    pub const ACTIVE: u32 = 1 << 5;
    /// Drop-down internal open flag (`0x5D3740` / `0x5D2DD0`).
    pub const DROPDOWN_OPEN: u32 = 1 << 11;
    /// Slider is vertical (`0x5DE360`).
    pub const VERTICAL: u32 = 1 << 12;
    /// List box sorted.
    pub const SORTED: u32 = 1 << 13;
    /// Edit box overwrite mode.
    pub const OVERWRITE: u32 = 1 << 15;
    /// Edit box has the edit focus.
    pub const EDIT_FOCUS: u32 = 1 << 16;
    /// Toggle button selected.
    pub const SELECTED: u32 = 1 << 7;
    /// Push button pressed.
    pub const PRESSED: u32 = 1 << 8;
    /// Drop-down / menu open.
    pub const OPEN: u32 = 1 << 9;
    /// Enabled.
    pub const ENABLED: u32 = 1 << 19;
    /// Targeted (tutorial highlight).
    pub const TARGETED: u32 = 1 << 22;
}

/// `0x5E25C0`: state bits of a view element owned by a widget with `flags`.
pub fn state_bits(flags: u32) -> u32 {
    let pick = |bit: u32, yes: u32, no: u32| if flags & bit != 0 { yes } else { no };
    pick(widget_flag::ENABLED, when::ENABLED, when::DISABLED)
        | pick(widget_flag::FOCUSED, when::FOCUSED, when::UNFOCUSED)
        | pick(widget_flag::PRESSED, when::PRESSED, when::UNPRESSED)
        | pick(widget_flag::SELECTED, when::SELECTED, when::UNSELECTED)
        | pick(widget_flag::OPEN, when::OPEN, when::CLOSED)
        | pick(widget_flag::TARGETED, when::TARGETED, when::UNTARGETED)
}

/// `0x5EE2C0`: the widget is shown and its render threshold admits the
/// current UI render level.
pub fn widget_shown(flags: u32, render_threshold: i32, render_level: i32) -> bool {
    flags & widget_flag::SHOWN != 0 && render_threshold >= render_level
}

/// `0x5E25C0`: whether a view element with `when_mask` is visible.
pub fn ve_visible(when_mask: u32, flags: u32, render_threshold: i32, render_level: i32) -> bool {
    let state = state_bits(flags);
    widget_shown(flags, render_threshold, render_level) && (when_mask & state) == when_mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_strings() {
        assert_eq!(parse_when("ENABLED + FOCUSED + UNPRESSED").bits, 1 | 4 | 0x20);
        assert_eq!(parse_when("disabled").bits, 2);
        assert_eq!(parse_when("").bits, 0);
        let p = parse_when("ENABLED + BOGUS");
        assert_eq!(p.bits, 1);
        assert_eq!(p.unknown, vec!["BOGUS".to_string()]);
        assert_eq!(parse_attributes("TEXT + COLOR").bits, 9);
    }
}
