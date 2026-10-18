//! UI data sources: `.lst` load lists and TGI documents converted to raw
//! node trees (before template/inheritance resolution, see `resolve.rs`).
//!
//! `.lst` handling follows the comments in `Data/templates.lst` and the UI
//! lists (`Data/UI/menus.lst`, `shared.lst`, `game.lst`): one file spec per
//! line, `;` starts a comment, a leading `/` is relative to `Data`, otherwise
//! relative to the list's own folder, `*` loads every match in name order,
//! and a nested `.lst` is processed at that point.

use crate::fs::DataRoot;
use aa_rules::tgi::{Document, Item, Limits, Node, TokenKind};
use serde::{Deserialize, Serialize};

/// A property as written: left-hand tokens (`string`, `L`), the value text
/// (`None` for a declaration without `=`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawField {
    /// Tokens before `=`, e.g. `["point3", "color"]` or `["texture",
    /// "/UI/%s/x.tga"]` for the authored `texture /UI/%s/x.tga` form.
    pub left: Vec<String>,
    pub value: Option<String>,
    pub offset: usize,
}

/// A node header and body as written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawNode {
    /// Header tokens other than `=` (`["Template", "MenuPushButtonWidget",
    /// "Template", "PushButtonWidget"]`, `["list", "Child", "Template",
    /// "ViewElement"]`, `["MainMenu", "Inherit", "PanelWidget"]`).
    pub header: Vec<String>,
    pub fields: Vec<RawField>,
    pub children: Vec<RawNode>,
    pub braced: bool,
    pub offset: usize,
}

/// One parsed source file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceFile {
    /// Data-relative path as requested (`/UI/Menus/main.tgi`).
    pub path: String,
    pub nodes: Vec<RawNode>,
    /// Top-level properties outside any node (rare; kept for diagnostics).
    pub stray_fields: Vec<RawField>,
    pub notices: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("missing data file {0}")]
    Missing(String),
    #[error("{path}: {message}")]
    Syntax { path: String, message: String },
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

fn text(doc: &Document, span: aa_rules::tgi::Span) -> String {
    match doc.bytes(span) {
        Some(b) => decode(b),
        None => String::new(),
    }
}

/// TGI files are UTF-8 (localization, with BOM) or Windows-1252 (most data).
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| cp1252(b)).collect(),
    }
}

fn cp1252(b: u8) -> char {
    const HIGH: [u16; 32] = [
        0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x8D,
        0x017D, 0x8F, 0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A,
        0x0153, 0x9D, 0x017E, 0x0178,
    ];
    if (0x80..0xA0).contains(&b) {
        char::from_u32(HIGH[(b - 0x80) as usize] as u32).unwrap_or('?')
    } else {
        b as char
    }
}

fn convert_node(doc: &Document, node: &Node) -> RawNode {
    let header = node
        .header
        .tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Equals)
        .map(|t| text(doc, t.span))
        .collect();
    let mut fields = Vec::new();
    let mut children = Vec::new();
    for item in &node.items {
        match item {
            Item::Node(n) => children.push(convert_node(doc, n)),
            Item::Property(p) => fields.push(RawField {
                left: p.left.iter().map(|t| text(doc, t.span)).collect(),
                value: p.value.map(|s| text(doc, s).trim().to_string()),
                offset: p.span.start,
            }),
        }
    }
    RawNode { header, fields, children, braced: node.braced, offset: node.span.start }
}

/// Parse one TGI file with the native-tolerant grammar.
pub fn parse_source(path: &str, bytes: Vec<u8>) -> Result<SourceFile, DataError> {
    let doc = Document::parse_native(bytes, Limits::default())
        .map_err(|e| DataError::Syntax { path: path.to_string(), message: e.to_string() })?;
    let mut nodes = Vec::new();
    let mut stray = Vec::new();
    for item in &doc.items {
        match item {
            Item::Node(n) => nodes.push(convert_node(&doc, n)),
            Item::Property(p) => stray.push(RawField {
                left: p.left.iter().map(|t| text(&doc, t.span)).collect(),
                value: p.value.map(|s| text(&doc, s).trim().to_string()),
                offset: p.span.start,
            }),
        }
    }
    let notices = doc.notices.iter().map(|n| format!("{:?} at {}", n.kind, n.span.start)).collect();
    Ok(SourceFile { path: path.to_string(), nodes, stray_fields: stray, notices })
}

/// Join a list entry to the list's folder (`/UI/menus.lst` + `Shared/x.tgi`
/// = `/UI/Shared/x.tgi`; a leading `/` is Data-relative).
pub fn join(list_path: &str, entry: &str) -> String {
    let entry = entry.replace('\\', "/");
    if entry.starts_with('/') {
        return entry;
    }
    let list = list_path.replace('\\', "/");
    let dir = match list.rfind('/') {
        Some(i) => &list[..i],
        None => "",
    };
    let mut out = if dir.starts_with('/') || dir.is_empty() { dir.to_string() } else { format!("/{dir}") };
    out.push('/');
    out.push_str(&entry);
    out
}

/// Expand a `.lst` into the ordered list of TGI files (nested lists are
/// expanded in place, `*` matches in case-insensitive name order). Missing
/// entries are returned in `missing` (the shipped `ui_templates.lst` names
/// `template_ve.tgi`/`template_widget.tgi` beside `UI/`, which do not exist).
pub fn expand_list(root: &DataRoot, list_path: &str, files: &mut Vec<String>, missing: &mut Vec<String>) {
    let Ok(bytes) = root.read(list_path) else {
        missing.push(list_path.to_string());
        return;
    };
    let body = decode(&bytes);
    for line in body.lines() {
        let line = match line.find(';') {
            Some(i) => &line[..i],
            None => line,
        };
        let entry = line.trim().trim_start_matches('\u{feff}');
        if entry.is_empty() {
            continue;
        }
        let path = join(list_path, entry);
        if path.contains('*') {
            let (dir, pat) = path.rsplit_once('/').unwrap_or(("", &path));
            let Some(real) = root.find(dir) else {
                missing.push(path.clone());
                continue;
            };
            let mut names: Vec<String> = std::fs::read_dir(real)
                .map(|rd| rd.filter_map(|e| e.ok()).filter_map(|e| e.file_name().into_string().ok()).collect())
                .unwrap_or_default();
            names.sort_by_key(|n| n.to_ascii_lowercase());
            for n in names {
                if wildcard(pat, &n) {
                    files.push(format!("{dir}/{n}"));
                }
            }
            continue;
        }
        if path.to_ascii_lowercase().ends_with(".lst") {
            expand_list(root, &path, files, missing);
        } else if root.exists(&path) {
            files.push(path);
        } else {
            missing.push(path);
        }
    }
}

fn wildcard(pat: &str, name: &str) -> bool {
    let p = pat.to_ascii_lowercase();
    let n = name.to_ascii_lowercase();
    match p.split_once('*') {
        None => p == n,
        Some((a, b)) => n.len() >= a.len() + b.len() && n.starts_with(a) && n.ends_with(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn list_join() {
        assert_eq!(join("/UI/menus.lst", "Shared/x.tgi"), "/UI/Shared/x.tgi");
        assert_eq!(join("ui/menus.lst", "/Templates/ui_templates.lst"), "/Templates/ui_templates.lst");
        assert_eq!(join("/Templates/ui_templates.lst", "UI/template_ve.tgi"), "/Templates/UI/template_ve.tgi");
        assert!(wildcard("*.tgi", "a.TGI"));
    }
}
