//! Typed field access over the shared aa-rules inherited-node resolver,
//! following the native TGI getter contract (`5B4ED0` int, `5B5130`/`5B5020`
//! fixed, `5B5380`/`5B5280` bool, `5B54D0` string, `5B4C30`/`5B4D80` points,
//! `5B4410` sub-node): field and child names are ASCII case-insensitive, a
//! required getter on an absent field throws "Node doesn't contain field.",
//! a required absent sub-node throws "SubNode not found.", and defaults come
//! only from the TGI templates.
//!
//! Only declared TGI types are decoded (aa-rules `ResolvedNode::field`);
//! undeclared identifier assignments (for example `type = NORMAL`) go through
//! `plain_string_field`. Localized fields go through aa-rules
//! `ResolvedNode::localized` ([`localized`], native getter 5B5620).
use crate::{Error, Result};
use aa_numeric::Precision;
use aa_rules::actors::ResolvedNode;
use aa_rules::native::TypedValue;

pub(crate) fn path_of(node: &ResolvedNode<'_>) -> String {
    node.sources()
        .first()
        .map(|p| format!("{}@{}", p.path, p.span.start))
        .unwrap_or_default()
}

fn missing_field(node: &ResolvedNode<'_>, name: &str) -> Error {
    Error::NativeData(format!("Node doesn't contain field. ({name} at {})", path_of(node)))
}

/// String field; `None` when absent. aa-rules maps case-insensitive `NULL`
/// to the empty native string (parser 5BB0B0).
pub(crate) fn opt_string(node: &ResolvedNode<'_>, name: &str) -> Result<Option<String>> {
    match node.field(name, Precision::Extended64) {
        Ok(Some(TypedValue::String { value, .. })) => Ok(Some(value)),
        Ok(Some(other)) => Err(Error::Unsupported(format!(
            "{} field {name} is not a string: {other:?}",
            path_of(node)
        ))),
        Ok(None) => Ok(None),
        // Undeclared identifier assignment (no inherited type declaration).
        Err(_) => match node.plain_string_field(name)? {
            Some(TypedValue::String { value, .. }) => Ok(Some(value)),
            Some(other) => Err(Error::Unsupported(format!(
                "{} field {name} is not a string: {other:?}",
                path_of(node)
            ))),
            None => Ok(None),
        },
    }
}

/// `5B5620`: required localized field (also the `name` read of the data
/// base loader 47D960). Native returns the stored text and, when it starts
/// with `#`, replaces it at load time by the string-table entry
/// `4B9F40(text[1..])`. The string tables are not part of this crate: the
/// authored key or literal is kept as aa-rules decodes it (quote rule
/// 5BEE60/5BDC00, one leading `@` removed by 5BA3C0), so a `#key` value
/// stands for its looked-up text. A field absent from every layer throws
/// "Node doesn't contain field."; a declaration without any value is
/// Unsupported (the native default text of such a field is unverified).
pub(crate) fn localized(node: &ResolvedNode<'_>, name: &str) -> Result<String> {
    if let Some(text) = node.localized(name)? {
        return Ok(text.text);
    }
    match node.field(name, Precision::Extended64) {
        Ok(None) => Err(missing_field(node, name)),
        _ => Err(Error::Unsupported(format!(
            "{} localized field {name} is declared without a value (native default unverified)",
            path_of(node)
        ))),
    }
}

/// `5B54D0`: required string.
pub(crate) fn string(node: &ResolvedNode<'_>, name: &str) -> Result<String> {
    opt_string(node, name)?.ok_or_else(|| missing_field(node, name))
}

/// `5B4ED0`: required int.
pub(crate) fn int(node: &ResolvedNode<'_>, name: &str) -> Result<i32> {
    match node.field(name, Precision::Extended64)? {
        Some(v) => Ok(v.integer()?),
        None => Err(missing_field(node, name)),
    }
}

/// Required fixed (`5B5130`), binary32 bits. Native int fields answer the
/// fixed getter with an exact `fild` (used for MGMap width/height).
pub(crate) fn fixed(node: &ResolvedNode<'_>, name: &str, precision: Precision) -> Result<u32> {
    match node.field(name, precision)? {
        Some(TypedValue::Integer { value, .. }) => Ok(aa_numeric::Binary::from_i32(value).store_f32()?),
        Some(v) => Ok(v.fixed_bits()?),
        None => Err(missing_field(node, name)),
    }
}

/// Optional fixed with an explicit default (`5B5020`).
///
/// A template declaration without a value (`fixed ammo` in `ResourceList`)
/// is answered with `default` as well. INFERENCE (README gap): native either
/// does not list such a field (default returned) or holds a zero-constructed
/// value; both equal the only default used here (0.0).
pub(crate) fn fixed_or(node: &ResolvedNode<'_>, name: &str, default: u32, precision: Precision) -> Result<u32> {
    match node.field(name, precision) {
        Ok(Some(v)) => Ok(v.fixed_bits()?),
        Ok(None) => Ok(default),
        Err(aa_rules::native::Error::Unsupported(m))
            if default == 0 && m.ends_with("has no assigned value") =>
        {
            Ok(default)
        }
        Err(e) => Err(e.into()),
    }
}

/// Required bool (`5B5380`).
pub(crate) fn boolean(node: &ResolvedNode<'_>, name: &str) -> Result<bool> {
    match node.field(name, Precision::Extended64)? {
        Some(v) => Ok(v.boolean()?),
        None => Err(missing_field(node, name)),
    }
}

/// Optional bool, `None` when absent (field-presence test then `5B5380`).
pub(crate) fn opt_boolean(node: &ResolvedNode<'_>, name: &str) -> Result<Option<bool>> {
    match node.field(name, Precision::Extended64)? {
        Some(v) => Ok(Some(v.boolean()?)),
        None => Ok(None),
    }
}

/// Point field components as binary32 bits; `None` when absent.
pub(crate) fn opt_point(node: &ResolvedNode<'_>, name: &str, precision: Precision) -> Result<Option<Vec<u32>>> {
    match node.field(name, precision)? {
        Some(TypedValue::Point { bits, .. }) => Ok(Some(bits)),
        Some(other) => Err(Error::Unsupported(format!(
            "{} field {name} is not a point: {other:?}",
            path_of(node)
        ))),
        None => Ok(None),
    }
}

/// Required point2 (`5B4D80`), widened to (x, y, 0) as the territory/link code stores it.
pub(crate) fn point2_as3(node: &ResolvedNode<'_>, name: &str, precision: Precision) -> Result<[u32; 3]> {
    let p = opt_point(node, name, precision)?.ok_or_else(|| missing_field(node, name))?;
    match p.as_slice() {
        [x, y] => Ok([*x, *y, 0]),
        _ => Err(Error::Unsupported(format!(
            "{} field {name} is not a point2",
            path_of(node)
        ))),
    }
}

/// `5B4410(name, required=1)`: exactly one sub-node.
pub(crate) fn subnode<'a>(node: &ResolvedNode<'a>, name: &str) -> Result<ResolvedNode<'a>> {
    match node.children(name)?.len() {
        0 => Err(Error::NativeData(format!(
            "SubNode not found. ({name} at {})",
            path_of(node)
        ))),
        1 => Ok(node.children(name)?.remove(0)),
        n => Err(Error::Unsupported(format!(
            "{} has {n} {name} children; native single sub-node lookup of a list is unproven",
            path_of(node)
        ))),
    }
}
