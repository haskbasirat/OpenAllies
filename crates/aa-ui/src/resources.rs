//! UI-side data tables: localized strings, AVars used by the UI and font
//! definitions, read with the native TGI loader (`tgi.rs`).
//!
//! * Localization (`KKC_TextDataManager`): string tables are the fields of
//!   `[Text]` nodes in `Data/Localization/**/*.tgi` (index_aa.lst lines
//!   29-40), keys `ids`/`name` skipped (0x4BA710); lookup is
//!   case-insensitive (hash 0x405030, `_wcsicmp` 0x4B9D40); with
//!   `DataCheckingStrict` (shipped default) a duplicate key logs an error and
//!   the later entry wins (0x4BA2A0). A localized value starting with `#` is
//!   looked up without the `#` (`0x5B5620` -> `GetDisplayString` 0x4B9F40);
//!   a missing key returns `error_missing_string` ("[Missing String]").
//! * AVars: `[Vars]` fields of `Data/AVars*.tgi` in name order (later
//!   wins), with the accepted v14 loose overrides.
//! * Fonts: `[Font]` nodes of `Data/Fonts/font_*.tgi` (`KKC_FontData::
//!   LoadData` 0x493B20).

use crate::fs::DataRoot;
use crate::node::UiNode;
use crate::text::{CharacterSet, FontDef, ftol255};
use crate::tgi::TgiStore;
use std::collections::HashMap;
#[path="model_variables.rs"]mod model_variables;

#[derive(Clone, Debug, Default)]
pub struct StringTable {
    map: HashMap<String, String>,
    pub duplicates: Vec<String>,
    /// Special glyphs (`Data/Fonts/special_glyphs*.tgi`): ids -> character.
    pub glyphs: HashMap<String, char>,
    /// The same definitions with their image names (loader `0x493500`).
    pub special: Vec<crate::fonts::SpecialGlyph>,
}

/// Escape processing of localized values (`0x494000`, via `0x5BA3C0`):
/// `TextEscapeCharacterStart` (`&`, AVars) starts `&Name.` (ended by
/// `TextEscapeCharacterEnd`, `.`), replaced by the special glyph `Name`;
/// `&&` and `TextOneCharEscapeCharacter` (default `\\`, registered at
/// 0x402CE8) escape the next character when it is one of the escape
/// characters. Unknown sequences are kept (the original logs them).
pub fn unescape(text: &str, glyphs: &HashMap<String, char>) -> String {
    const START: char = '&';
    const END: char = '.';
    const ONE: char = '\\';
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c == START || c == ONE) && i + 1 < chars.len() {
            let next = chars[i + 1];
            if c == ONE || next == START {
                if next == START || next == ONE {
                    out.push(next);
                    i += 2;
                    continue;
                }
                out.push(c);
                i += 1;
                continue;
            }
            if let Some(len) = chars[i + 1..].iter().position(|&x| x == END) {
                let name: String = chars[i + 1..i + 1 + len].iter().collect();
                if let Some(&g) = glyphs.get(&name.to_ascii_lowercase()) {
                    out.push(g);
                    i += len + 2;
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn unquote_localized(raw: &str) -> String {
    let t = raw.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') { t[1..t.len() - 1].to_string() } else { t.to_string() }
}

impl StringTable {
    /// Load every localization table in `index_aa.lst` order.
    pub fn load(store: &mut TgiStore) -> Self {
        let mut glyphs = HashMap::new();
        let mut special: Vec<crate::fonts::SpecialGlyph> = Vec::new();
        let _ = store.load_file("/Templates/template_font.tgi");
        for f in list_tgi(&store.root.clone(), "Fonts") {
            if !f.to_ascii_lowercase().contains("special_glyphs") {
                continue;
            }
            let Ok(root) = store.load_file(&f) else { continue };
            for g in root.children.iter().filter(|c| c.name.eq_ignore_ascii_case("Glyph")) {
                if let (Ok(ids), Ok(index)) = (g.string("ids"), g.int("index")) {
                    if let Some(ch) = char::from_u32(index as u32) {
                        glyphs.insert(ids.to_ascii_lowercase(), ch);
                    }
                    // an index defined twice: the later one replaces it (the
                    // original logs a warning)
                    special.retain(|s| s.index != index as u32);
                    special.push(crate::fonts::SpecialGlyph {
                        ids: ids.clone(),
                        display_name: g.string_or_empty("display_name"),
                        index: index as u32,
                        use_own_color: g.bool_or("use_own_color", true).unwrap_or(true),
                    });
                }
            }
        }
        let mut files = Vec::new();
        for dir in ["Localization", "Localization/SAI", "Localization/RandomMap", "Localization/Hotkeys/Actions"] {
            files.extend(list_tgi(&store.root, dir));
        }
        let mut table = StringTable { glyphs, special, ..Default::default() };
        for f in files {
            let Ok(root) = store.load_file(&f) else { continue };
            for text in root.children.iter().filter(|c| c.name.eq_ignore_ascii_case("Text")) {
                for field in &text.fields {
                    if field.name.eq_ignore_ascii_case("ids") || field.name.eq_ignore_ascii_case("name") {
                        continue;
                    }
                    let Some(raw) = &field.raw else { continue };
                    let k = field.name.to_ascii_lowercase();
                    let value = unescape(&unquote_localized(raw), &table.glyphs);
                    if table.map.insert(k, value).is_some() {
                        table.duplicates.push(field.name.clone());
                    }
                }
            }
        }
        table
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(&key.to_ascii_lowercase()).map(String::as_str)
    }

    /// `GetDisplayString` with printf-style `%s`/`%d` arguments (keys such
    /// as `quit_confirm|s`).
    pub fn format(&self, key: &str, args: &[&str]) -> String {
        let mut out = String::new();
        let template = self.resolve(&format!("#{key}"));
        let mut it = args.iter();
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '%' {
                match chars.peek() {
                    Some('s') | Some('d') | Some('i') => {
                        chars.next();
                        out.push_str(it.next().copied().unwrap_or(""));
                        continue;
                    }
                    Some('%') => {
                        chars.next();
                        out.push('%');
                        continue;
                    }
                    _ => {}
                }
            }
            out.push(c);
        }
        out
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Resolve a localized field value (`#key` or literal text).
    pub fn resolve(&self, value: &str) -> String {
        match value.strip_prefix('#') {
            Some(k) => match self.get(k) {
                Some(v) => v.to_string(),
                None => self.get("error_missing_string").unwrap_or("[Missing String]").to_string(),
            },
            None => unescape(value, &self.glyphs),
        }
    }
}

/// `*.tgi` files of a data directory, names in case-insensitive order.
pub fn list_tgi(root: &DataRoot, dir: &str) -> Vec<String> {
    let mut names: Vec<String> = root.list_dir(dir).into_iter().filter(|n| n.to_ascii_lowercase().ends_with(".tgi")).collect();
    names.sort_by_key(|n| n.to_ascii_lowercase());
    names.into_iter().map(|n| format!("/{dir}/{n}")).collect()
}

/// AVars values the UI reads.
#[derive(Clone, Debug, Default)]
pub struct AVars {
    pub values: HashMap<String, String>,
    model_source:model_variables::Receipt,
}

impl AVars {
    /// `AVars*.tgi` in name order (KKC_Application init 0x4196A0).
    pub fn load(store: &mut TgiStore) -> Self {
        let mut v = AVars::default();
        let mut files: Vec<String> = store
            .root
            .list_dir("/")
            .into_iter()
            .filter(|n| {
                let l = n.to_ascii_lowercase();
                l.starts_with("avars") && l.ends_with(".tgi")
            })
            .collect();
        files.sort_by_key(|n| n.to_ascii_lowercase());
        for f in files {
            let Ok(root) = store.load_file(&format!("/{f}")) else { continue };
            for vars in root.children.iter().filter(|c| c.name.eq_ignore_ascii_case("Vars")) {
                collect(vars, &mut v.values);
            }
        }
        if let Err(error)=v.model_source.bind(&store.root,crate::view_variables::SourceOverrides::ControlledNone,&mut v.values){v.model_source.fail(error)}
        v
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        self.values.get(&k.to_ascii_lowercase()).map(String::as_str)
    }
    pub fn flag(&self, k: &str) -> Option<bool> {
        crate::node::parse_bool(self.get(k)?)
    }
    pub fn float(&self, k: &str) -> Option<f32> {
        crate::node::parse_float(self.get(k)?)
    }
    pub fn preprocessed_data_missing_warning_current(&self)->Result<u8,String>{self.model_source.current(&self.values,0)}
    pub fn preprocessed_data_wrong_version_warning_current(&self)->Result<u8,String>{self.model_source.current(&self.values,1)}
}

fn collect(n: &UiNode, out: &mut HashMap<String, String>) {
    for f in &n.fields {
        if let Some(r) = &f.raw {
            out.insert(f.name.to_ascii_lowercase(), r.clone());
        }
    }
    for c in &n.children {
        collect(c, out);
    }
}

/// Load every `[Font]` from `Data/Fonts/*.tgi`.
pub fn load_fonts(store: &mut TgiStore) -> HashMap<String, FontDef> {
    let mut out = HashMap::new();
    let _ = store.load_file("/Templates/template_font.tgi");
    for f in list_tgi(&store.root.clone(), "Fonts") {
        let Ok(root) = store.load_file(&f) else { continue };
        for font in root.children.iter().filter(|c| c.name.eq_ignore_ascii_case("Font")) {
            let ids = font.string_or_empty("ids");
            let sets = font
                .children_named("CharacterSet")
                .map(|cs| CharacterSet {
                    ids: cs.string_or_empty("ids"),
                    font: cs.string_or_empty("font"),
                    backup_font: cs.string("backup_font").ok().filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null")),
                    backup_font_size: cs.int("backup_font_size").unwrap_or(0),
                    use_backup_font_when_smaller_than: cs.int("use_backup_font_when_smaller_than").unwrap_or(0),
                    antialias: cs.bool_or("antialias", true).unwrap_or(true),
                    bold: cs.bool_or("bold", false).unwrap_or(false),
                    italic: cs.bool_or("italic", false).unwrap_or(false),
                    outline: cs.bool_or("outline", false).unwrap_or(false),
                    shadow: cs.bool_or("shadow", false).unwrap_or(false),
                    shadow_opacity255: ftol255(cs.float("shadow_opacity").unwrap_or(0.5)),
                    shadow_brightness255: ftol255(cs.float("shadow_brightness").unwrap_or(0.1)),
                })
                .collect();
            out.insert(ids.to_ascii_lowercase(), FontDef { ids, size: font.int("size").unwrap_or(0), sets });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes() {
        let mut g = HashMap::new();
        g.insert("ch_money".to_string(), '\u{80}');
        assert_eq!(unescape("Quit Axis && Allies?", &g), "Quit Axis & Allies?");
        assert_eq!(unescape("&ch_MONEY.100", &g), "\u{80}100");
        assert_eq!(unescape("a & b", &g), "a & b");
    }
}
