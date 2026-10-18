//! Resolved UI data nodes: a TGI node after template/inheritance resolution,
//! with typed field access in the style of the native node accessors
//! (`0x5B54D0` string, `0x5B5620` localized, `0x5B5380` bool/flag,
//! `0x5B5280` bool with default, `0x5B4ED0` int, `0x5B4AE0` float,
//! `0x5B4C30` point3, `0x5B4410` child by name, `0x5B4220` child count,
//! `0x42BE10` i-th child). Names compare case-insensitively
//! (`0x497220`/`0x4247C0`).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Where a value or node came from.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Origin {
    /// Data-relative source file (`/UI/Menus/main.tgi`).
    pub file: String,
    /// Byte offset in that file.
    pub offset: usize,
}

/// One field of a resolved node.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Field {
    /// Field name as first declared (most-derived spelling kept for display).
    pub name: String,
    /// Declared type from the defining template (`string`, `bool`, `flag`,
    /// `float`, `int`, `point2`, `point3`, `localized`, `fixed`), if any.
    pub decl: Option<String>,
    /// Raw authored value text (quotes kept), `None` for a declaration
    /// without a value.
    pub raw: Option<String>,
    pub origin: Origin,
}

/// Native node bookkeeping (`node+0x18` template, `+0x1C` uid, `+0x20`
/// flags, see `tgi::flag`).
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NodeMeta {
    pub flags: u32,
    pub uid: u64,
    pub template_uid: Option<u64>,
}

/// A resolved node.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct UiNode {
    /// Node name (`MainMenu`, `View`, `Child`).
    pub name: String,
    /// Template chain, most derived first (`MenuPushButtonWidget`,
    /// `PushButtonWidget`, `ButtonWidget`, `Widget`).
    pub templates: Vec<String>,
    pub fields: Vec<Field>,
    pub children: Vec<UiNode>,
    pub origin: Origin,
    #[serde(default)]
    pub meta: NodeMeta,
}

/// A field access failure (the original throws "Node doesn't contain
/// field." from the typed accessors).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FieldError {
    #[error("node '{node}' doesn't contain field '{field}'")]
    Missing { node: String, field: String },
    #[error("node '{node}' field '{field}' has no value")]
    NoValue { node: String, field: String },
    #[error("node '{node}' field '{field}' value '{raw}' is not a valid {kind}")]
    Invalid { node: String, field: String, raw: String, kind: &'static str },
}

impl UiNode {
    /// Inside a template definition (flag 0x1).
    pub fn is_template(&self) -> bool {
        self.meta.flags & 0x1 != 0
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name.eq_ignore_ascii_case(name))
    }

    pub fn has(&self, name: &str) -> bool {
        self.field(name).is_some()
    }

    /// First child with this name (`0x5B4410`).
    pub fn child(&self, name: &str) -> Option<&UiNode> {
        self.children.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    pub fn child_mut(&mut self, name: &str) -> Option<&mut UiNode> {
        self.children.iter_mut().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// All children with this name in order (`0x5B4220` / `0x42BE10`).
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a UiNode> + 'a {
        self.children.iter().filter(move |c| c.name.eq_ignore_ascii_case(name))
    }

    /// Whether this node's template chain contains `template`.
    pub fn is_a(&self, template: &str) -> bool {
        self.templates.iter().any(|t| t.eq_ignore_ascii_case(template))
    }

    fn raw_of(&self, name: &str) -> Result<&str, FieldError> {
        let f = self.field(name).ok_or_else(|| FieldError::Missing { node: self.name.clone(), field: name.into() })?;
        f.raw.as_deref().ok_or_else(|| FieldError::NoValue { node: self.name.clone(), field: name.into() })
    }

    /// String value with surrounding quotes removed.
    pub fn string(&self, name: &str) -> Result<String, FieldError> {
        Ok(unquote(self.raw_of(name)?))
    }

    /// String value or empty when absent / declared without value.
    pub fn string_or_empty(&self, name: &str) -> String {
        self.string(name).unwrap_or_default()
    }

    pub fn bool(&self, name: &str) -> Result<bool, FieldError> {
        let raw = self.raw_of(name)?;
        parse_bool(raw).ok_or_else(|| FieldError::Invalid {
            node: self.name.clone(),
            field: name.into(),
            raw: raw.into(),
            kind: "bool",
        })
    }

    /// `0x5B5280`: bool with a default when the field is absent.
    pub fn bool_or(&self, name: &str, default: bool) -> Result<bool, FieldError> {
        match self.field(name) {
            None => Ok(default),
            Some(_) => self.bool(name),
        }
    }

    pub fn float(&self, name: &str) -> Result<f32, FieldError> {
        let raw = self.raw_of(name)?;
        parse_float(raw).ok_or_else(|| FieldError::Invalid {
            node: self.name.clone(),
            field: name.into(),
            raw: raw.into(),
            kind: "float",
        })
    }

    pub fn int(&self, name: &str) -> Result<i32, FieldError> {
        let raw = self.raw_of(name)?;
        parse_int(raw).ok_or_else(|| FieldError::Invalid {
            node: self.name.clone(),
            field: name.into(),
            raw: raw.into(),
            kind: "int",
        })
    }

    pub fn point3(&self, name: &str) -> Result<[f32; 3], FieldError> {
        let raw = self.raw_of(name)?;
        parse_point::<3>(raw).ok_or_else(|| FieldError::Invalid {
            node: self.name.clone(),
            field: name.into(),
            raw: raw.into(),
            kind: "point3",
        })
    }

    pub fn point2(&self, name: &str) -> Result<[f32; 2], FieldError> {
        let raw = self.raw_of(name)?;
        parse_point::<2>(raw).ok_or_else(|| FieldError::Invalid {
            node: self.name.clone(),
            field: name.into(),
            raw: raw.into(),
            kind: "point2",
        })
    }

    /// Set or replace a field value (used by code that overrides data).
    pub fn set(&mut self, name: &str, raw: &str) {
        if let Some(f) = self.fields.iter_mut().find(|f| f.name.eq_ignore_ascii_case(name)) {
            f.raw = Some(raw.to_string());
        } else {
            self.fields.push(Field { name: name.into(), decl: None, raw: Some(raw.into()), origin: Origin::default() });
        }
    }
}

impl fmt::Display for UiNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn walk(n: &UiNode, depth: usize, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            writeln!(f, "{}[{} : {}]", "  ".repeat(depth), n.name, n.templates.join(" < "))?;
            for fl in &n.fields {
                writeln!(f, "{}  {} = {}", "  ".repeat(depth), fl.name, fl.raw.as_deref().unwrap_or("<decl>"))?;
            }
            for c in &n.children {
                walk(c, depth + 1, f)?;
            }
            Ok(())
        }
        walk(self, 0, f)
    }
}

/// Remove one pair of surrounding double quotes.
pub fn unquote(raw: &str) -> String {
    let t = raw.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

pub fn parse_bool(raw: &str) -> Option<bool> {
    let t = unquote(raw);
    if t.eq_ignore_ascii_case("true") || t == "1" {
        Some(true)
    } else if t.eq_ignore_ascii_case("false") || t == "0" {
        Some(false)
    } else {
        None
    }
}

pub fn parse_float(raw: &str) -> Option<f32> {
    let t = unquote(raw);
    let t = t.trim().trim_end_matches(['f', 'F']);
    if let Some(p) = t.strip_suffix('%') {
        return p.trim().parse::<f32>().ok().map(|v| v / 100.0);
    }
    t.parse::<f32>().ok()
}

pub fn parse_int(raw: &str) -> Option<i32> {
    let t = unquote(raw);
    let t = t.trim();
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return i64::from_str_radix(h, 16).ok().map(|v| v as i32);
    }
    t.parse::<i32>().ok()
}

pub fn parse_point<const N: usize>(raw: &str) -> Option<[f32; N]> {
    let t = unquote(raw);
    let parts: Vec<&str> = t.split(',').map(str::trim).collect();
    if parts.len() != N {
        return None;
    }
    let mut out = [0.0; N];
    for (o, p) in out.iter_mut().zip(parts) {
        *o = p.parse::<f32>().ok()?;
    }
    Some(out)
}
