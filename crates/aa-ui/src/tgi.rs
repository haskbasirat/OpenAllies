//! Native TGI loading for UI data: lexer, scope stack, template registry and
//! the load-time merge of templates into nodes.
//!
//! Ported from AA.exe (research/phase8-ui/README.txt, "TGI semantics"):
//! * statement lexer `0x5BE9D0` (table 0x5BEC24): `;` or end of line ends a
//!   statement (the rest of the line is a comment), `/*` opens a block
//!   comment, `[` a header, `{`/`}` braces; key scan `0x5BE8A0` stops at the
//!   first `=`, `;` or `/`, then exactly one character is skipped
//!   (0x5BEE1C) and blanks; value scan `0x5BDF80` stops at an unquoted `;`
//!   or `/*`, a `"` quotes until the next `"`, the result is trimmed;
//!   optional type words `0x5BECB0`/`0x5BD990`;
//! * header handler `0x5BC0A0`: tokens split on blank/`=`, leading
//!   `Template` = template definition, `Optional`/`List` before the name,
//!   `Template T` (strict) / `Inherit T` (non-strict), `Language id`;
//!   implicit template = scope template's same-named child (`0x5B42C0`);
//!   explicit template from the registry (`0x5BF0A0`); same-template rule
//!   (0x5BC815); list-member flag (`0x5B37A0`); unbraced nodes close at the
//!   next header (0x5BC126) or at the parent's `}` (0x5BA9D0);
//! * on close only the explicit template is applied:
//!   `CopyValuesAndSubNodesTo` `0x5B57C0`, child merge `0x5B6190`, clone
//!   `0x5B5FC0`; AddChild / indexed groups `0x5B5C80`, `0x42BC40`;
//! * template registry: global, case-insensitive, duplicate definitions are
//!   errors (`0x5BCA42` -> `0x5BF0D0`); data nodes are found only through
//!   their file (`0x5B4410` paths, leading `/` = file root).
//!
//! Every error the original reports is recorded in `diagnostics` instead of
//! aborting, so a port can see data the original would have rejected.

use crate::data::{decode, expand_list};
use crate::fs::DataRoot;
use crate::node::{Field, Origin, UiNode};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Node flags (`node+0x20`).
pub mod flag {
    pub const IN_TEMPLATE: u32 = 0x1;
    pub const OPTIONAL: u32 = 0x2;
    pub const LIST: u32 = 0x4;
    pub const LIST_MEMBER: u32 = 0x10;
    pub const REGISTERED: u32 = 0x20;
    pub const HAD_TEMPLATE: u32 = 0x40;
    pub const HAD_INHERIT: u32 = 0x80;
    pub const CLONE: u32 = 0x4000;
}

const TYPE_WORDS: [&str; 15] = [
    "float", "int", "integer", "point3", "point2", "fixed", "kf32", "sfloat", "bool", "flag", "boolean", "string",
    "localized", "localizable", "templatename",
];

#[derive(Clone, Debug, PartialEq)]
enum Stmt {
    Header(Vec<String>, usize),
    Open(usize),
    Close(usize),
    Field { type_word: Option<String>, key: String, value: Option<String>, line: usize },
}

/// Statement lexer (`0x5BE9D0` and helpers).
fn lex(text: &str, diags: &mut Vec<String>, path: &str) -> Vec<Stmt> {
    let mut out = Vec::new();
    let mut in_block = false;
    for (ln, raw_line) in text.split('\n').enumerate() {
        let line: Vec<char> = raw_line.trim_end_matches('\r').chars().collect();
        let mut i = 0;
        let n = line.len();
        let line_no = ln + 1;
        while i < n {
            if in_block {
                if i + 1 < n && line[i] == '*' && line[i + 1] == '/' {
                    in_block = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            let c = line[i];
            if c == ' ' || c == '\t' || c == '\u{feff}' {
                i += 1;
                continue;
            }
            match c {
                ';' => break,
                '/' if i + 1 < n && line[i + 1] == '*' => {
                    in_block = true;
                    i += 2;
                }
                '/' => {
                    diags.push(format!("{path}:{line_no}: Unexpected '/' found."));
                    break;
                }
                '[' => {
                    let start = i + 1;
                    let mut j = start;
                    while j < n && line[j] != ']' {
                        j += 1;
                    }
                    let mut inner: String = line[start..j].iter().collect();
                    // Block comments inside a header (`[View Template =
                    // DecalVE/*TiledHorizontal*/]`, template_game.tgi:1613)
                    // are removed: the shipped in-game panels load, so the
                    // original cannot have read them as part of the name.
                    while let Some(a) = inner.find("/*") {
                        match inner[a..].find("*/") {
                            Some(b) => inner.replace_range(a..a + b + 2, ""),
                            None => inner.truncate(a),
                        }
                    }
                    let tokens = inner
                        .split([' ', '\t', '='])
                        .filter(|t| !t.is_empty())
                        .map(str::to_string)
                        .collect();
                    out.push(Stmt::Header(tokens, line_no));
                    i = if j < n { j + 1 } else { n };
                }
                '{' => {
                    out.push(Stmt::Open(line_no));
                    i += 1;
                }
                '}' => {
                    out.push(Stmt::Close(line_no));
                    i += 1;
                }
                ']' => {
                    diags.push(format!("{path}:{line_no}: unexpected ']'"));
                    i += 1;
                }
                _ => {
                    // key: up to '=', ';' or '/'
                    let start = i;
                    let mut j = i;
                    while j < n && !matches!(line[j], '=' | ';' | '/') {
                        j += 1;
                    }
                    let key_text: String = line[start..j].iter().collect::<String>().trim().to_string();
                    let mut value = None;
                    let mut k = j;
                    if j < n {
                        k = j + 1; // exactly one character skipped
                        while k < n && (line[k] == ' ' || line[k] == '\t') {
                            k += 1;
                        }
                        let vstart = k;
                        let mut quoted = false;
                        while k < n {
                            let ch = line[k];
                            if ch == '"' {
                                quoted = !quoted;
                            } else if !quoted && (ch == ';' || (ch == '/' && k + 1 < n && line[k + 1] == '*')) {
                                break;
                            }
                            k += 1;
                        }
                        let v: String = line[vstart..k].iter().collect::<String>().trim().to_string();
                        if !v.is_empty() {
                            value = Some(v);
                        }
                    }
                    let mut words: Vec<&str> = key_text.split_whitespace().collect();
                    let mut type_word = None;
                    if words.len() >= 2 && TYPE_WORDS.iter().any(|t| t.eq_ignore_ascii_case(words[0])) {
                        type_word = Some(words[0].to_ascii_lowercase());
                        words.remove(0);
                    }
                    let key = words.join(" ");
                    if !key.is_empty() {
                        out.push(Stmt::Field { type_word, key, value, line: line_no });
                    }
                    // continue after the value; a ';' ends the line, '/*'
                    // opens a block comment handled by the loop
                    if k < n && line[k] == ';' {
                        break;
                    }
                    i = k.max(i + 1);
                }
            }
        }
    }
    out
}

/// A reference to a node inside a resolved template tree.
#[derive(Clone, Debug)]
struct TRef {
    root: Arc<UiNode>,
    path: Vec<usize>,
}

impl TRef {
    fn get(&self) -> &UiNode {
        let mut n: &UiNode = &self.root;
        for &i in &self.path {
            n = &n.children[i];
        }
        n
    }
    fn find_child(&self, name: &str) -> Option<TRef> {
        let idx = self.get().children.iter().position(|c| c.name.eq_ignore_ascii_case(name))?;
        let mut path = self.path.clone();
        path.push(idx);
        Some(TRef { root: self.root.clone(), path })
    }
}

struct Scope {
    node: UiNode,
    explicit: Option<TRef>,
    implicit: Option<TRef>,
    strict: bool,
    braced: bool,
    tdef: bool,
}

impl Scope {
    fn scope_template(&self) -> Option<&TRef> {
        self.explicit.as_ref().or(self.implicit.as_ref())
    }
}

/// Loaded UI data: template registry and parsed files.
pub struct TgiStore {
    pub root: DataRoot,
    registry: HashMap<String, Arc<UiNode>>,
    files: HashMap<String, Arc<UiNode>>,
    loaded_lists: HashSet<String>,
    next_uid: u64,
    pub diagnostics: Vec<String>,
    /// Template names in registration order.
    pub template_order: Vec<String>,
}

fn key(s: &str) -> String {
    s.replace('\\', "/").trim_start_matches('/').to_ascii_lowercase()
}

impl TgiStore {
    pub fn new(root: DataRoot) -> Self {
        Self {
            root,
            registry: HashMap::new(),
            files: HashMap::new(),
            loaded_lists: HashSet::new(),
            next_uid: 1,
            diagnostics: Vec::new(),
            template_order: Vec::new(),
        }
    }

    pub fn template(&self, name: &str) -> Option<Arc<UiNode>> {
        self.registry.get(&name.to_ascii_lowercase()).cloned()
    }

    pub fn template_count(&self) -> usize {
        self.registry.len()
    }

    /// `0x5C5060`: the UI template lists in native order, from the AVars
    /// `InterfaceSharedTemplates`, `InterfaceEditorTemplates`,
    /// `InterfaceGameTemplates`, `InterfaceMenuTemplates` (Data/AVars.tgi).
    pub fn load_ui_templates(&mut self) {
        for l in ["ui/shared.lst", "ui/editor.lst", "ui/game.lst", "ui/menus.lst"] {
            self.load_list(l);
        }
    }

    /// Load every file of a `.lst` (once per file, reference counted in the
    /// original).
    pub fn load_list(&mut self, list_path: &str) {
        if !self.loaded_lists.insert(key(list_path)) {
            return;
        }
        let mut files = Vec::new();
        let mut missing = Vec::new();
        expand_list(&self.root, list_path, &mut files, &mut missing);
        for m in missing {
            self.diagnostics.push(format!("list {list_path}: missing {m}"));
        }
        for f in files {
            if let Err(e) = self.load_file(&f) {
                self.diagnostics.push(e);
            }
        }
    }

    /// Parse a file once and return its root node.
    pub fn load_file(&mut self, path: &str) -> Result<Arc<UiNode>, String> {
        let k = key(path);
        if let Some(f) = self.files.get(&k) {
            return Ok(f.clone());
        }
        let bytes = self.root.read(path).map_err(|e| format!("{path}: {e}"))?;
        let text = decode(&bytes);
        let root = self.parse(path, &text);
        let root = Arc::new(root);
        self.files.insert(k, root.clone());
        Ok(root)
    }

    /// `file.tgi:Node/Sub` lookup (`0x5F1D00` split on ':', `0x5B4410`).
    pub fn node(&mut self, spec: &str) -> Result<UiNode, String> {
        let (file, path) = spec.split_once(':').ok_or_else(|| format!("no ':' in {spec}"))?;
        let root = self.load_file(file)?;
        find_path(&root, path).cloned().ok_or_else(|| format!("SubNode not found: {spec}"))
    }

    fn uid(&mut self) -> u64 {
        self.next_uid += 1;
        self.next_uid
    }

    fn parse(&mut self, path: &str, text: &str) -> UiNode {
        let mut diags = Vec::new();
        let stmts = lex(text, &mut diags, path);
        let mut root = UiNode { name: path.to_string(), origin: Origin { file: path.into(), offset: 0 }, ..Default::default() };
        root.meta.uid = self.uid();
        let mut stack: Vec<Scope> = Vec::new();
        for st in stmts {
            match st {
                Stmt::Header(tokens, line) => {
                    if stack.last().is_some_and(|s| !s.braced) {
                        let s = stack.pop().unwrap();
                        self.close(s, &mut stack, &mut root, path, &mut diags);
                    }
                    self.open_header(tokens, line, path, &mut stack, &root, &mut diags);
                }
                Stmt::Open(line) => match stack.last_mut() {
                    Some(s) if !s.braced => s.braced = true,
                    Some(_) => diags.push(format!("{path}:{line}: Illegal nested opening brace.")),
                    None => diags.push(format!("{path}:{line}: opening brace without node")),
                },
                Stmt::Close(line) => {
                    let Some(s) = stack.pop() else {
                        diags.push(format!("{path}:{line}: unmatched closing brace"));
                        continue;
                    };
                    let was_braced = s.braced;
                    self.close(s, &mut stack, &mut root, path, &mut diags);
                    if !was_braced {
                        if let Some(s2) = stack.pop() {
                            self.close(s2, &mut stack, &mut root, path, &mut diags);
                        }
                    }
                }
                Stmt::Field { type_word, key, value, line } => {
                    let (target, tmpl, strict) = match stack.last_mut() {
                        Some(s) => {
                            let t = s.scope_template().cloned();
                            let strict = s.strict;
                            (&mut s.node, t, strict)
                        }
                        None => (&mut root, None, false),
                    };
                    let decl_from_template = tmpl.as_ref().and_then(|t| t.get().field(&key).and_then(|f| f.decl.clone()));
                    if strict && tmpl.as_ref().is_some_and(|t| t.get().field(&key).is_none()) && !target.is_template() {
                        diags.push(format!("{path}:{line}: Field not found in template. ({key})"));
                    }
                    if let (Some(tw), Some(dt)) = (&type_word, &decl_from_template) {
                        if !same_type(tw, dt) {
                            diags.push(format!("{path}:{line}: Illegal type change from template's type. ({key})"));
                        }
                    }
                    let decl = decl_from_template.or(type_word);
                    let value = match (&decl, value) {
                        (Some(d), Some(v)) if d == "string" && v.eq_ignore_ascii_case("null") => Some(String::new()),
                        (_, v) => v,
                    };
                    if value.is_none() && !target.is_template() {
                        diags.push(format!("{path}:{line}: Non-template node had a field with no value specified. ({key})"));
                    }
                    if target.field(&key).is_some() {
                        diags.push(format!("{path}:{line}: Node already contains field. ({key})"));
                        continue;
                    }
                    target.fields.push(Field {
                        name: key,
                        decl,
                        raw: value,
                        origin: Origin { file: path.into(), offset: line },
                    });
                }
            }
        }
        while let Some(s) = stack.pop() {
            self.close(s, &mut stack, &mut root, path, &mut diags);
        }
        self.diagnostics.extend(diags);
        root
    }

    fn open_header(
        &mut self,
        tokens: Vec<String>,
        line: usize,
        path: &str,
        stack: &mut Vec<Scope>,
        root: &UiNode,
        diags: &mut Vec<String>,
    ) {
        let mut it = tokens.into_iter().peekable();
        let mut tdef = false;
        let mut optional = false;
        let mut list = false;
        let mut name: Option<String> = None;
        let mut explicit_name: Option<(String, bool)> = None;
        if it.peek().is_some_and(|t| t.eq_ignore_ascii_case("template")) {
            tdef = true;
            it.next();
        }
        while let Some(t) = it.next() {
            if t.eq_ignore_ascii_case("language") {
                it.next();
                continue;
            }
            if name.is_none() && !tdef && t.eq_ignore_ascii_case("optional") {
                optional = true;
                continue;
            }
            if name.is_none() && !tdef && t.eq_ignore_ascii_case("list") {
                list = true;
                continue;
            }
            if name.is_some() && (t.eq_ignore_ascii_case("template") || t.eq_ignore_ascii_case("inherit")) {
                match it.next() {
                    Some(tn) => explicit_name = Some((tn, t.eq_ignore_ascii_case("template"))),
                    None => diags.push(format!("{path}:{line}: Could not determine template name.")),
                }
                continue;
            }
            if name.is_none() {
                name = Some(t);
            } else {
                diags.push(format!("{path}:{line}: Malformed section name. ({t})"));
            }
        }
        let Some(name) = name else {
            diags.push(format!("{path}:{line}: empty node header"));
            return;
        };
        let parent_in_template = stack.last().is_some_and(|s| s.node.is_template());
        let parent_scope = stack.last().and_then(|s| s.scope_template().cloned());
        if (optional || list) && !parent_in_template {
            diags.push(format!("{path}:{line}: Optional/List outside a template ({name})"));
        }
        let mut implicit = parent_scope.as_ref().and_then(|t| t.find_child(&name));
        let parent_strict = stack.last().is_some_and(|s| s.strict);
        let mut strict = parent_strict;
        let mut explicit = None;
        if let Some((tn, is_template)) = &explicit_name {
            strict = *is_template;
            match self.template(tn) {
                Some(t) => explicit = Some(TRef { root: t, path: vec![] }),
                None => diags.push(format!("{path}:{line}: Template definition not found. ({tn})")),
            }
        }
        if parent_strict && implicit.is_none() && !stack.is_empty() && explicit_name.is_none() {
            diags.push(format!("{path}:{line}: Subnode added that did not exist in template. ({name})"));
        }
        // same-template rule (0x5BC815)
        if let (Some(imp), Some(exp)) = (&implicit, &explicit) {
            if imp.get().meta.template_uid.is_some() && imp.get().meta.template_uid == Some(exp.get().meta.uid) {
                implicit = Some(exp.clone());
                explicit = None;
            }
        }
        let mut node = UiNode {
            name: name.clone(),
            origin: Origin { file: path.into(), offset: line },
            ..Default::default()
        };
        node.meta.uid = self.uid();
        node.meta.template_uid = explicit.as_ref().or(implicit.as_ref()).map(|t| t.get().meta.uid);
        let tchain_src = explicit.as_ref().or(implicit.as_ref());
        if let Some(t) = tchain_src {
            let tn = t.get();
            let mut chain = Vec::new();
            if tn.meta.flags & flag::REGISTERED != 0 {
                chain.push(tn.name.clone());
            }
            chain.extend(tn.templates.iter().cloned());
            node.templates = chain;
        }
        let mut f = 0;
        if tdef || parent_in_template {
            f |= flag::IN_TEMPLATE;
        }
        if optional {
            f |= flag::OPTIONAL;
        }
        if list {
            f |= flag::LIST;
        }
        if let Some((_, is_t)) = &explicit_name {
            f |= if *is_t { flag::HAD_TEMPLATE } else { flag::HAD_INHERIT };
        }
        // list member (0x5B37A0)
        let parent_node: &UiNode = stack.last().map(|s| &s.node).unwrap_or(root);
        let is_list_child = |n: &UiNode| {
            n.children.iter().any(|c| c.name.eq_ignore_ascii_case(&name) && c.meta.flags & (flag::LIST | flag::LIST_MEMBER) != 0)
        };
        if is_list_child(parent_node)
            || stack.last().and_then(|s| s.explicit.as_ref()).is_some_and(|t| is_list_child(t.get()))
            || stack.last().and_then(|s| s.implicit.as_ref()).is_some_and(|t| is_list_child(t.get()))
        {
            f |= flag::LIST_MEMBER;
        }
        if tdef {
            f |= flag::REGISTERED;
        }
        node.meta.flags = f;
        stack.push(Scope { node, explicit, implicit, strict, braced: false, tdef });
    }

    fn close(&mut self, s: Scope, stack: &mut [Scope], root: &mut UiNode, path: &str, diags: &mut Vec<String>) {
        let Scope { mut node, explicit, strict, tdef, .. } = s;
        if let Some(t) = &explicit {
            copy_into(t.get(), &mut node, strict, diags);
        }
        if tdef {
            let k = node.name.to_ascii_lowercase();
            if self.registry.contains_key(&k) {
                diags.push(format!("{path}:{}: Template was already defined. ({})", node.origin.offset, node.name));
            } else {
                self.template_order.push(node.name.clone());
                self.registry.insert(k, Arc::new(node.clone()));
            }
        }
        let parent = match stack.last_mut() {
            Some(p) => &mut p.node,
            None => root,
        };
        add_child(parent, node, false);
    }
}

fn same_type(a: &str, b: &str) -> bool {
    let canon = |t: &str| match t.to_ascii_lowercase().as_str() {
        "integer" => "int".to_string(),
        "flag" | "boolean" => "bool".to_string(),
        "kf32" | "sfloat" => "fixed".to_string(),
        "localizable" => "localized".to_string(),
        o => o.to_string(),
    };
    canon(a) == canon(b)
}

/// AddChild (`0x5B5C80`): same-named siblings form an ordered group; a
/// front insert becomes entry 0.
fn add_child(parent: &mut UiNode, child: UiNode, front: bool) {
    let first = parent.children.iter().position(|c| c.name.eq_ignore_ascii_case(&child.name));
    if front {
        parent.children.insert(first.unwrap_or(0), child);
    } else {
        parent.children.push(child);
    }
}

fn mark_list_members(d: &mut UiNode, name: &str) {
    for c in d.children.iter_mut().filter(|c| c.name.eq_ignore_ascii_case(name)) {
        if c.meta.flags & flag::LIST == 0 {
            c.meta.flags |= flag::LIST_MEMBER;
        }
    }
}

/// `CopyValuesAndSubNodesTo` (`0x5B57C0`).
fn copy_into(s: &UiNode, d: &mut UiNode, strict: bool, diags: &mut Vec<String>) {
    for f in &s.fields {
        if d.field(&f.name).is_none() {
            if strict && !d.is_template() && f.raw.is_none() {
                diags.push(format!(
                    "{}: Field '{}' missing from node '{}' using strict template",
                    d.origin.file, f.name, d.name
                ));
            }
            d.fields.push(f.clone());
        }
    }
    // distinct child names in source order
    let mut names: Vec<String> = Vec::new();
    for c in &s.children {
        if !names.iter().any(|n| n.eq_ignore_ascii_case(&c.name)) {
            names.push(c.name.clone());
        }
    }
    for name in names {
        let group: Vec<&UiNode> = s.children.iter().filter(|c| c.name.eq_ignore_ascii_case(&name)).collect();
        if group.len() >= 2 {
            for c in group.iter().rev() {
                merge_child(c, d, strict, true, diags);
            }
        } else {
            merge_child(group[0], d, strict, false, diags);
        }
    }
}

/// Child merge (`0x5B6190`).
fn merge_child(s: &UiNode, d: &mut UiNode, strict: bool, indexed: bool, diags: &mut Vec<String>) {
    let s_list = s.meta.flags & flag::LIST != 0;
    let s_member = s.meta.flags & flag::LIST_MEMBER != 0;
    if s_list {
        for e in d.children.iter_mut().filter(|c| c.name.eq_ignore_ascii_case(&s.name)) {
            copy_into(s, e, strict, diags);
        }
        if d.children.iter().any(|c| c.name.eq_ignore_ascii_case(&s.name) && c.meta.flags & flag::LIST != 0) {
            return;
        }
    }
    let target = if s_member {
        d.children.iter().position(|c| c.meta.uid == s.meta.uid)
    } else {
        d.children.iter().position(|c| c.name.eq_ignore_ascii_case(&s.name))
    };
    match target {
        Some(t) if !s_list => {
            let mut child = std::mem::take(&mut d.children[t]);
            copy_into(s, &mut child, strict, diags);
            d.children[t] = child;
        }
        _ => {
            if !d.is_template() && (s.meta.flags & flag::OPTIONAL != 0 || s_list) {
                return;
            }
            // clone (0x5B5FC0)
            let mut n = UiNode {
                name: s.name.clone(),
                templates: s.templates.clone(),
                origin: s.origin.clone(),
                ..Default::default()
            };
            n.meta.flags = (if d.is_template() { s.meta.flags } else { 0 }) | flag::CLONE;
            n.meta.uid = s.meta.uid;
            n.meta.template_uid = s.meta.template_uid;
            copy_into(s, &mut n, strict, diags);
            let front = indexed || n.meta.flags & flag::LIST != 0;
            add_child(d, n, front);
        }
    }
    if s_list || s_member {
        mark_list_members(d, &s.name);
    }
}

/// `0x5B4410`: `/`-separated path from `node` (a leading `/` is the root of
/// the same file, which callers pass as `node`).
pub fn find_path<'a>(node: &'a UiNode, path: &str) -> Option<&'a UiNode> {
    let mut at = node;
    for part in path.split('/').filter(|p| !p.is_empty()) {
        at = at.child(part)?;
    }
    Some(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(files: &[(&str, &str)]) -> (TgiStore, Vec<Arc<UiNode>>) {
        let dir = std::env::temp_dir().join(format!("aa-ui-tgi-{}-{}", std::process::id(), files.len()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, text) in files {
            std::fs::write(dir.join(name), text).unwrap();
        }
        let mut store = TgiStore::new(DataRoot::new(&dir));
        let roots = files.iter().map(|(n, _)| store.load_file(n).unwrap()).collect();
        (store, roots)
    }

    #[test]
    fn list_entries_merge_in_native_order() {
        let (store, roots) = store_with(&[(
            "t.tgi",
            "[Template VE]
{
 string type
 string L = 0
}
             [Template Decal Inherit=VE]
{
 type = DECAL
 string texture = NULL
}
             [Template Group Inherit=VE]
{
 type = GROUP
 [list Child Template=VE]
}
             [Template Two Template=Group]
{
 [Child Template=Decal]
 texture = a
 [Child Template=Decal]
 texture = b
}
             [Template One Template=Group]
{
 [Child Template=Decal]
 texture = a
}
             [X Template=Two]
{
 [Child Template=Decal]
 texture = mine
}
             [Y Template=One]
{
 [Child Template=Decal]
 texture = mine
}
",
        )]);
        assert!(store.diagnostics.is_empty(), "{:?}", store.diagnostics);
        let tex = |n: &UiNode| n.children_named("Child").map(|c| c.string_or_empty("texture")).collect::<Vec<_>>();
        // a template group of >= 2 entries is front-inserted in template order
        let x = roots[0].child("X").unwrap();
        assert_eq!(tex(x), vec!["a", "b", "mine"]);
        // a derived template also holds the cloned list declaration at index
        // 0, so even one template entry forms a group and comes first
        let y = roots[0].child("Y").unwrap();
        assert_eq!(tex(y), vec!["a", "mine"]);
        // list declarations fill entries and are not cloned into instances
        assert!(x.children_named("Child").all(|c| c.string_or_empty("L") == "0"));
        assert_eq!(x.string_or_empty("type"), "GROUP");
        // derived template keeps the declaration at index 0
        let two = store.template("Two").unwrap();
        assert_eq!(two.children_named("Child").count(), 3);
        assert!(two.children_named("Child").next().unwrap().meta.flags & flag::LIST != 0);
    }

    #[test]
    fn lexer_native_quirks() {
        let mut d = Vec::new();
        let s = lex(
            "texture /UI/%s/x.tga\nstring justify ; LEFT, CENTER\nint index ;; c\nL = 50 ; x\n[Child Template = DecalVE]\n",
            &mut d,
            "t",
        );
        assert_eq!(
            s[0],
            Stmt::Field { type_word: None, key: "texture".into(), value: Some("UI/%s/x.tga".into()), line: 1 }
        );
        assert_eq!(
            s[1],
            Stmt::Field {
                type_word: Some("string".into()),
                key: "justify".into(),
                value: Some("LEFT, CENTER".into()),
                line: 2
            }
        );
        assert_eq!(s[2], Stmt::Field { type_word: Some("int".into()), key: "index".into(), value: None, line: 3 });
        assert_eq!(s[3], Stmt::Field { type_word: None, key: "L".into(), value: Some("50".into()), line: 4 });
        assert_eq!(s[4], Stmt::Header(vec!["Child".into(), "Template".into(), "DecalVE".into()], 5));
    }
}
