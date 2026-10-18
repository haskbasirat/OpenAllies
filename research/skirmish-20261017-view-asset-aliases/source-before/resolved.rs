//! Native inherited node views, including ordered authored list instances.
use super::{ActorCatalog, Definition, FieldResolution};
#[path = "native_load.rs"]
pub mod native_load;
#[path = "native_field_order.rs"]
mod native_field_order;
pub use native_field_order::SVarField;
use crate::{
    native::{Error, Result, ScalarType, TypedScalar, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;

/// Stored native text of a `localized` field (see `ResolvedNode::localized`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct LocalizedText {
    /// Key such as `#name` or literal text; `null`/`NULL` is kept verbatim.
    pub text: String,
    /// False only for the unquoted `null` form.
    pub quoted: bool,
    pub source: Provenance,
    pub declaration_source: Provenance,
}
#[derive(Clone)]
pub struct ResolvedNode<'a> {
    pub(super) catalog: &'a ActorCatalog,
    pub(super) layers: Vec<&'a Definition>,
}
impl ResolvedNode<'_> {
    /// Bounded plain-string assignment for native string getters that also
    /// accept untyped identifier assignments (for example recruitable). This
    /// does not relax `field` or infer numeric/localized/expression values.
    pub fn plain_string_field(&self, name: &str) -> Result<Option<TypedValue>> {
        let mut selected = None;
        let mut declaration = None;
        let mut first_assignment = None;
        for layer in &self.layers {
            let fields: Vec<_> = layer
                .fields
                .iter()
                .filter(|field| field.name.eq_ignore_ascii_case(name))
                .collect();
            if fields.len() > 1 || fields.iter().any(|f| f.unsupported_declaration) {
                return Err(Error::Unsupported(format!(
                    "ambiguous plain-string field {name}"
                )));
            }
            for field in fields {
                if let Some(kind) = &field.declared_type {
                    if !kind.eq_ignore_ascii_case("string") {
                        return Err(Error::Unsupported(format!(
                            "plain-string field {name} declared as {kind}"
                        )));
                    }
                    declaration = Some(field.source.clone());
                }
                first_assignment = Some(field.source.clone());
                selected.get_or_insert(field);
            }
        }
        let Some(field) = selected else {
            return Ok(None);
        };
        let raw = field
            .value
            .as_deref()
            .ok_or_else(|| Error::Unsupported(format!("plain-string field {name} has no value")))?;
        let raw = std::str::from_utf8(raw)
            .map_err(|_| Error::Unsupported(format!("non-ASCII plain-string field {name}")))?;
        if !raw
            .bytes()
            .next()
            .is_some_and(|v| v.is_ascii_alphabetic() || v == b'_')
            || !raw.bytes().all(|v| v.is_ascii_alphanumeric() || v == b'_')
        {
            return Err(Error::Unsupported(format!(
                "plain-string field {name} requires one unquoted ASCII identifier"
            )));
        }
        Ok(Some(TypedValue::String {
            value: if raw.eq_ignore_ascii_case("null") {
                String::new()
            } else {
                raw.to_owned()
            },
            source: field.source.clone(),
            // For an implicit field the original base assignment creates the
            // field; keep that origin instead of inventing a declaration.
            declaration_source: declaration
                .or(first_assignment)
                .expect("selected field has an origin"),
        }))
    }
    pub fn sources(&self) -> Vec<Provenance> {
        self.layers.iter().map(|n| n.source.clone()).collect()
    }
    /// A `localized` field's stored native text: the string-table key (for
    /// example `#rus_corps_name`) or literal text, after the parse-time quote
    /// rule (5BEE60/5BDC00) and the single leading `@` removal of 5BA3C0.
    /// Text is NOT looked up here: native resolves it in its getter. A
    /// declaration-only field (no value anywhere) returns `Ok(None)`.
    pub fn localized(&self, name: &str) -> Result<Option<LocalizedText>> {
        let mut selected = None;
        let mut declaration = None;
        for layer in &self.layers {
            let fields: Vec<_> = layer
                .fields
                .iter()
                .filter(|f| f.name.eq_ignore_ascii_case(name))
                .collect();
            if fields.len() > 1 || fields.iter().any(|f| f.unsupported_declaration) {
                return Err(Error::Unsupported(format!(
                    "field {name} has duplicate or unsupported declaration syntax"
                )));
            }
            for field in fields {
                if let Some(kind) = &field.declared_type {
                    if !kind.eq_ignore_ascii_case("localized") {
                        return Err(Error::Unsupported(format!(
                            "field {name} is declared {kind}, not localized"
                        )));
                    }
                    declaration.get_or_insert_with(|| field.source.clone());
                }
                if selected.is_none() && field.value.is_some() {
                    selected = Some(field);
                }
            }
        }
        let Some(declaration_source) = declaration else {
            return if selected.is_some() {
                Err(Error::Unsupported(format!(
                    "field {name} has no verified localized declaration"
                )))
            } else {
                Ok(None)
            };
        };
        let Some(field) = selected else {
            return Ok(None);
        };
        let raw = std::str::from_utf8(field.value.as_deref().unwrap_or_default())
            .map_err(|_| Error::Unsupported(format!("non-UTF-8 localized field {name}")))?
            .trim();
        let text = crate::tgi::native_localized_text(raw).ok_or_else(|| {
            Error::Unsupported(format!(
                "localized field {name} value is neither quoted nor null (native rejects it)"
            ))
        })?;
        Ok(Some(LocalizedText {
            quoted: raw.starts_with('"'),
            text,
            source: field.source.clone(),
            declaration_source,
        }))
    }
    pub fn field(&self, name: &str, precision: Precision) -> Result<Option<TypedValue>> {
        let layers = &self.layers;
        let path = name;
        if layers.is_empty() {
            return Ok(None);
        }
        let field = layers
            .iter()
            .find_map(|layer| {
                layer
                    .fields
                    .iter()
                    .find(|f| f.name.eq_ignore_ascii_case(name))
            })
            .cloned();
        let Some(field) = field else {
            return Ok(None);
        };
        let raw = field.value.as_ref().ok_or_else(|| {
            Error::Unsupported(format!("required field {path} has no assigned value"))
        })?;
        let raw = std::str::from_utf8(raw)
            .map_err(|_| Error::Unsupported(format!("non-ASCII typed field {path}")))?;
        // A value override keeps the native declaration/type of its inherited
        // field. Multiple explicit incompatible declarations are not guessed.
        let mut declarations = Vec::new();
        for layer in layers {
            let matches: Vec<_> = layer
                .fields
                .iter()
                .filter(|f| f.name.eq_ignore_ascii_case(name))
                .collect();
            if matches.len() > 1 || matches.iter().any(|f| f.unsupported_declaration) {
                return Err(Error::Unsupported(format!(
                    "field {path} has duplicate or unsupported declaration syntax"
                )));
            }
            for f in &layer.fields {
                if f.name.eq_ignore_ascii_case(name)
                    && let Some(kind) = &f.declared_type
                {
                    declarations.push((kind.to_ascii_lowercase(), f.source.clone()));
                }
            }
        }
        let Some((kind, declaration_source)) = declarations.first() else {
            return Err(Error::Unsupported(format!(
                "field {path} has no verified inherited declaration"
            )));
        };
        fn canonical(s: &str) -> &str {
            match s {
                "fixed" | "kf32" | "sfloat" => "fixed",
                "bool" | "flag" | "boolean" => "bool",
                s => s,
            }
        }
        if declarations
            .iter()
            .any(|(v, _)| canonical(v) != canonical(kind))
        {
            return Err(Error::Unsupported(format!(
                "field {path} changes declared type across inheritance"
            )));
        }
        let scalar = |field_type| -> crate::native::Result<TypedValue> {
            Ok(TypedValue::Scalar(TypedScalar {
                bits: crate::native::scalar_literal(raw, precision)?,
                field_type,
                source: field.source.clone(),
                declaration_source: declaration_source.clone(),
            }))
        };
        Ok(Some(match canonical(kind) {
            "fixed" => scalar(ScalarType::Fixed)?,
            "float" => scalar(ScalarType::Float)?,
            "int" => TypedValue::Integer {
                value: {
                    let digits = raw.strip_prefix(['+', '-']).unwrap_or(raw);
                    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(Error::Unsupported(format!(
                            "unsupported integer literal {path}: {raw}"
                        )));
                    }
                    raw.parse().map_err(|_| {
                        Error::Unsupported(format!("integer outside i32 {path}: {raw}"))
                    })?
                },
                source: field.source,
                declaration_source: declaration_source.clone(),
            },
            "point2" | "point3" => TypedValue::Point {
                bits: {
                    let parts: Vec<_> = raw.split(',').collect();
                    let count = if kind == "point2" { 2 } else { 3 };
                    if parts.len() != count {
                        return Err(Error::Unsupported(format!(
                            "{kind} {path} requires {count} components"
                        )));
                    }
                    parts
                        .iter()
                        .map(|v| crate::native::scalar_literal(v.trim(), precision))
                        .collect::<Result<Vec<_>>>()?
                },
                source: field.source,
                declaration_source: declaration_source.clone(),
            },
            "bool" => TypedValue::Boolean {
                value: if raw.eq_ignore_ascii_case("true") {
                    true
                } else if raw.eq_ignore_ascii_case("false") {
                    false
                } else {
                    return Err(Error::Unsupported(format!("invalid boolean {path}: {raw}")));
                },
                source: field.source,
                declaration_source: declaration_source.clone(),
            },
            "string" if !raw.contains('"') => TypedValue::String {
                // Original5BB0B0 plain-string assignment converts case-insensitive
                // NULL to an empty native string before constructing the field.
                value: if raw.eq_ignore_ascii_case("null") {
                    String::new()
                } else {
                    raw.to_owned()
                },
                source: field.source,
                declaration_source: declaration_source.clone(),
            },
            other => {
                return Err(Error::Unsupported(format!(
                    "field {path} type {other} not implemented"
                )));
            }
        }))
    }
}

impl<'a> ResolvedNode<'a> {
    /// Original NameList loader 4C19D0 uses 5B4220/42BE10 iteration even
    /// when repeated Name nodes have no list declaration. The parser's
    /// 5B5C80 renaming preserves their authored order. Keep this restricted
    /// to NameList inputs; generic children() intentionally remains strict.
    pub(crate) fn native_name_list_children(&self) -> Result<Vec<Self>> {
        if self.layers.len() == 1
            && self.layers[0].name.eq_ignore_ascii_case("NameList")
            && self.layers[0].base.is_none()
        {
            let mut out = Vec::new();
            for child in &self.layers[0].children {
                if !child.name.eq_ignore_ascii_case("Name") {
                    continue;
                }
                if child.template || child.optional || child.list || child.base.is_some() {
                    return Err(Error::Unsupported(
                        "NameList without a template has an unverified Name declaration".into(),
                    ));
                }
                out.push(Self {
                    catalog: self.catalog,
                    layers: self
                        .catalog
                        .resolver
                        .layers(child, &mut Vec::new())
                        .map_err(Error::Unsupported)?,
                });
            }
            return Ok(out);
        }
        self.children("Name")
    }

    /// A NameList Name.name can be an implicit quoted localized field. This
    /// is the field created by 5BA3C0 and read by 5B5620, not a plain string
    /// getter. Untyped null/plain identifiers create another native field
    /// type and fail this getter. Oracle: native-name-lists.json.
    pub(crate) fn native_name_list_text(&self) -> Result<String> {
        let mut selected = None;
        let mut declared = false;
        for layer in &self.layers {
            if !layer.name.eq_ignore_ascii_case("Name") {
                return Err(Error::Unsupported(
                    "NameList text requires a Name node".into(),
                ));
            }
            let fields: Vec<_> = layer
                .fields
                .iter()
                .filter(|f| f.name.eq_ignore_ascii_case("name"))
                .collect();
            if fields.len() > 1 || fields.iter().any(|f| f.unsupported_declaration) {
                return Err(Error::Unsupported(
                    "ambiguous NameList Name.name field".into(),
                ));
            }
            for field in fields {
                if let Some(kind) = &field.declared_type {
                    if !kind.eq_ignore_ascii_case("localized") {
                        return Err(Error::Unsupported(
                            "NameList Name.name is not localized (5B5620 type error)".into(),
                        ));
                    }
                    declared = true;
                }
                if selected.is_none() && field.value.is_some() {
                    selected = Some(field);
                }
            }
        }
        let field = selected.ok_or_else(|| {
            Error::Unsupported("NameList Name.name is missing (5B5620 throws)".into())
        })?;
        let raw = field.value.as_deref().expect("selected assigned field");
        // Native C-locale source reading widens bytes, not UTF-8 decoding.
        // The restricted text grammar below is ASCII, except literal bytes
        // which are widened individually after the quote/leading-@ rule.
        let quoted = raw.first() == Some(&b'"') && raw.last() == Some(&b'"');
        if !quoted && !(declared && raw.eq_ignore_ascii_case(b"null")) {
            return Err(Error::Unsupported(
                "NameList Name.name is not a quoted localized field".into(),
            ));
        }
        if raw.iter().any(|b| matches!(b, b'&' | b'\\')) {
            return Err(Error::Unsupported(
                "NameList text glyph/escape expansion is not bound".into(),
            ));
        }
        let mut bytes: Vec<u8> = raw.iter().copied().filter(|&b| b != b'"').collect();
        if quoted && bytes.first() == Some(&b'@') {
            bytes.remove(0);
        }
        Ok(bytes.into_iter().map(char::from).collect())
    }

    /// Native template list items precede the items authored by descendants.
    /// The list declaration supplies defaults and never becomes a live item.
    pub fn children(&self, name: &str) -> Result<Vec<Self>> {
        let mut prototypes = Vec::new();
        let mut instances = Vec::new();
        let mut layers: Vec<&Definition> = Vec::new();
        for &layer in &self.layers {
            if !layers.iter().any(|&prior| std::ptr::eq(prior, layer)) {
                layers.push(layer);
            }
        }
        for layer in &layers {
            for child in &layer.children {
                if child.name.eq_ignore_ascii_case(name) && child.list {
                    prototypes.extend(
                        self.catalog
                            .resolver
                            .layer_sequence(child, &mut Vec::new(), true)
                            .map_err(Error::Unsupported)?,
                    );
                }
            }
        }
        if !prototypes.is_empty() {
            for layer in layers.iter().rev() {
                for child in &layer.children {
                    if child.name.eq_ignore_ascii_case(name) && !child.list && !child.optional {
                        let mut instance_layers = self
                            .catalog
                            .resolver
                            .layers(child, &mut Vec::new())
                            .map_err(Error::Unsupported)?;
                        if let Some(base) = &child.base {
                            let ancestry: Vec<_> = instance_layers
                                .iter()
                                .skip(1)
                                .map(|n| n.name.as_str())
                                .collect();
                            for prototype in prototypes.iter().filter(|n| n.list) {
                                if let Some(required) = &prototype.base
                                    && !ancestry
                                        .iter()
                                        .any(|name| name.eq_ignore_ascii_case(required))
                                {
                                    return Err(Error::Unsupported(format!(
                                        "list item template {base} is not a descendant of {required}"
                                    )));
                                }
                            }
                        }
                        instance_layers.extend(prototypes.iter().copied());
                        instances.push(Self {
                            catalog: self.catalog,
                            layers: instance_layers,
                        });
                    }
                }
            }
            return Ok(instances);
        }
        let mut children = Vec::new();
        let mut ordinary = false;
        let mut bases = Vec::new();
        for layer in layers {
            let matches: Vec<_> = layer
                .children
                .iter()
                .filter(|c| c.name.eq_ignore_ascii_case(name))
                .collect();
            if matches.len() > 1 {
                return Err(Error::Unsupported(format!(
                    "multiple children {name} lack a native list declaration"
                )));
            }
            if let Some(child) = matches.first() {
                ordinary |= !child.optional;
                let child_layers = self
                    .catalog
                    .resolver
                    .layers(child, &mut Vec::new())
                    .map_err(Error::Unsupported)?;
                if let Some(base) = &child.base {
                    bases.push((
                        base,
                        child_layers
                            .iter()
                            .skip(1)
                            .map(|n| n.name.as_str())
                            .collect::<Vec<_>>(),
                    ));
                }
                children.extend(child_layers);
            }
        }
        if !ordinary {
            return Ok(Vec::new());
        }
        if let Some((_, ancestry)) = bases.first()
            && bases
                .iter()
                .skip(1)
                .any(|(base, _)| !ancestry.iter().any(|name| name.eq_ignore_ascii_case(base)))
        {
            return Err(Error::Unsupported(format!(
                "child template at {name} is not a descendant of its inherited template"
            )));
        }
        Ok(vec![Self {
            catalog: self.catalog,
            layers: children,
        }])
    }
    pub fn child(&self, name: &str) -> Result<Option<Self>> {
        let mut items = self.children(name)?;
        if items.len() > 1 {
            return Err(Error::Unsupported(format!(
                "child {name} is an ordered list, use children"
            )));
        }
        Ok(items.pop())
    }
}
impl ActorCatalog {
    /// Authored root instances with inherited fields, in supplied source order.
    /// This does not imply native registry order or successful runtime creation.
    pub fn resolved_definitions(&self, kind: &str) -> Result<Vec<ResolvedNode<'_>>> {
        self.resolver
            .roots
            .iter()
            .filter(|n| !n.template && n.name.eq_ignore_ascii_case(kind))
            .map(|node| {
                Ok(ResolvedNode {
                    catalog: self,
                    layers: self
                        .resolver
                        .layers(node, &mut Vec::new())
                        .map_err(Error::Unsupported)?,
                })
            })
            .collect()
    }
    /// Find an unambiguous authored registry ID. Duplicate IDs remain errors;
    /// native registration order and overwrite behavior are not inferred.
    pub fn resolved_definition(
        &self,
        kind: &str,
        id: &str,
        precision: Precision,
    ) -> Result<ResolvedNode<'_>> {
        let mut matches = Vec::new();
        for node in self.resolved_definitions(kind)? {
            if let Some(TypedValue::String { value, .. }) = node.field("IDS", precision)?
                && value.eq_ignore_ascii_case(id)
            {
                matches.push(node);
            }
        }
        if matches.len() != 1 {
            return Err(Error::Unsupported(format!(
                "{kind} IDS {id} has {} matching declarations",
                matches.len()
            )));
        }
        Ok(matches.remove(0))
    }
    /// Original DataManager lookup by IDS for non-Thing managers (for example
    /// 4CDBB0 -> 48D500 for Resource): an IDS with no declaration yields
    /// `Ok(None)` exactly as the native Find returns null. The caller owns the
    /// native consequence (4CDC70 skips the saved Kingdom resource record and
    /// logs "unrecognized resource IDS loading kingdom"). More than one root
    /// declaration remains an explicit error: generic DataManager reload
    /// (4BD1B0 -> 47D960) semantics are not modeled for that case.
    pub fn native_named_definition(
        &self,
        kind: &str,
        id: &str,
        precision: Precision,
    ) -> Result<Option<ResolvedNode<'_>>> {
        // Native-order catalog: the manager's own registry and 48D500 Find
        // (translations applied, first registration kept).
        if let Some(registry) = self.native_data_registry(kind) {
            return match registry.find(id) {
                Some(entry) => self.native_data_node(entry).map(Some),
                None => Ok(None),
            };
        }
        let mut matches = Vec::new();
        for node in self.resolved_definitions(kind)? {
            if let Some(TypedValue::String { value, .. }) = node.field("IDS", precision)?
                && value.eq_ignore_ascii_case(id)
            {
                matches.push(node);
            }
        }
        match matches.len() {
            0 => Ok(None),
            1 => Ok(matches.pop()),
            n => Err(Error::Unsupported(format!(
                "{kind} IDS {id} has {n} matching declarations"
            ))),
        }
    }
    pub fn resolved_actor(&self, id: &str) -> Result<ResolvedNode<'_>> {
        let actor = self
            .find(id)
            .ok_or_else(|| Error::Unsupported(format!("missing actor {id}")))?;
        if let Some(FieldResolution::Unsupported { reason }) = actor.field("category") {
            return Err(Error::Unsupported(reason.clone()));
        }
        let node = self
            .resolver
            .roots
            .iter()
            .find(|n| n.source == actor.source)
            .ok_or_else(|| Error::Unsupported("actor source unavailable".into()))?;
        Ok(ResolvedNode {
            catalog: self,
            layers: self
                .resolver
                .layers(node, &mut Vec::new())
                .map_err(Error::Unsupported)?,
        })
    }
}
