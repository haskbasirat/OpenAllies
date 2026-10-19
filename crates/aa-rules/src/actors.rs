//! Bounded authored Thing-definition projection, not live actor state or a game evaluator.
//! Original missing-field copy and ordinary/optional child merge are the supported
//! subset. Lists, ambiguous definitions and unsupported header operations stay errors.
use crate::{
    resources::{Provenance, SourceIdentity},
    sources::{ResolvedFile, SourceError, V14Sources, sha256},
    tgi::{Document, Item, Limits, Node, TokenKind},
};
use serde::Serialize;
use std::collections::BTreeMap;
#[path = "resolved.rs"]
mod resolved;
pub use resolved::{ResolvedNode, SVarField, native_load, native_fresh_field_order};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum FieldResolution {
    Missing,
    Declared { source: Provenance },
    Value { raw: Vec<u8>, source: Provenance },
    Unsupported { reason: String },
}
impl FieldResolution {
    /// Exact expression text, not evaluated, translated, or quote-stripped.
    pub fn raw_text(&self) -> Option<&str> {
        match self {
            Self::Value { raw, .. } => std::str::from_utf8(raw).ok(),
            _ => None,
        }
    }
    pub fn source(&self) -> Option<&Provenance> {
        match self {
            Self::Declared { source } | Self::Value { source, .. } => Some(source),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct ActorDefinition {
    pub id: String,
    pub source: Provenance,
    /// Most-specific definition first. This is not a native registry ordinal.
    pub template_chain: Vec<Provenance>,
    pub fields: BTreeMap<String, FieldResolution>,
}
impl ActorDefinition {
    pub fn field(&self, path: &str) -> Option<&FieldResolution> {
        self.fields.get(path)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub source: SourceIdentity,
    pub message: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct ConstructionUpgradeCensus {
    pub sources: Vec<SourceIdentity>,
    pub declarations: Vec<Provenance>,
    pub explicit_instances: Vec<Provenance>,
    /// Hash-checked parse-omitted single-byte sources with no case-insensitive
    /// ASCII 'Upgrade' byte sequence anywhere. This certifies only absence of
    /// this node name, not successful parsing of their other definitions.
    pub opaque_sources_without_upgrade_bytes: Vec<SourceIdentity>,
}
#[derive(Clone, Debug, Serialize)]
pub struct LiteralReferenceInput {
    pub field: String,
    pub id: String,
    pub source: Provenance,
}
#[derive(Clone, Debug, Serialize)]
pub struct LiteralReferenceCensus {
    pub sources: Vec<SourceIdentity>,
    pub references: Vec<LiteralReferenceInput>,
    pub opaque_sources_without_field_names: Vec<SourceIdentity>,
}
/// Interior cache for [`ActorCatalog`]; cloning copies the computed entries.
#[derive(Debug, Default)]
pub(crate) struct ResolutionCache {
    /// Lowercase tag -> None when every root of that tag resolves, else the
    /// first Unsupported message `resolved_definitions` would return.
    pub(crate) tags: std::sync::Mutex<BTreeMap<String, Option<String>>>,
    /// Precision significand bits -> lowercase Thing IDS -> declaration count
    /// over every Thing-tag root, or the error that census returns.
    pub(crate) thing_ids: std::sync::Mutex<BTreeMap<u64, Result<BTreeMap<String, usize>, String>>>,
    /// Lowercase tag -> each resolved root's layers as resolver paths (root
    /// index, then child indices), or the Unsupported message. Paths are
    /// structural, so they stay valid in a clone.
    pub(crate) definitions: std::sync::Mutex<BTreeMap<String, std::sync::Arc<Result<Vec<Vec<Vec<usize>>>, String>>>>,
    /// Node address -> resolver path, for this instance only (never cloned).
    pub(crate) addresses: std::sync::OnceLock<std::collections::HashMap<usize, Vec<usize>>>,
    /// Successful per-definition query results (see `query_memo`).
    pub(crate) memo: crate::query_memo::QueryMemo,
}
impl Clone for ResolutionCache {
    fn clone(&self) -> Self {
        let tags = self.tags.lock().map(|t| t.clone()).unwrap_or_default();
        let thing_ids = self.thing_ids.lock().map(|t| t.clone()).unwrap_or_default();
        let definitions = self.definitions.lock().map(|t| t.clone()).unwrap_or_default();
        Self {
            tags: std::sync::Mutex::new(tags),
            thing_ids: std::sync::Mutex::new(thing_ids),
            definitions: std::sync::Mutex::new(definitions),
            addresses: std::sync::OnceLock::new(),
            memo: self.memo.clone(),
        }
    }
}
/// How a Thing IDS selects its ThingData allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThingLookup {
    /// Original 427BD0: the Thing translation map first, then the registry.
    Find,
    /// The registry only: the allocation registered under this IDS.
    Registered,
}
/// A Thing IDS plus the lookup that selects its allocation. Plain `&str`
/// arguments keep the original Find semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThingRef<'a> {
    pub ids: &'a str,
    pub lookup: ThingLookup,
}
impl<'a> ThingRef<'a> {
    pub fn find(ids: &'a str) -> Self {
        Self { ids, lookup: ThingLookup::Find }
    }
    pub fn registered(ids: &'a str) -> Self {
        Self { ids, lookup: ThingLookup::Registered }
    }
}
impl<'a> From<&'a str> for ThingRef<'a> {
    fn from(ids: &'a str) -> Self {
        Self::find(ids)
    }
}
impl<'a> From<&&'a str> for ThingRef<'a> {
    fn from(ids: &&'a str) -> Self {
        Self::find(ids)
    }
}
impl<'a> From<&&'a String> for ThingRef<'a> {
    fn from(ids: &&'a String) -> Self {
        Self::find(ids)
    }
}
impl<'a> From<&'a String> for ThingRef<'a> {
    fn from(ids: &'a String) -> Self {
        Self::find(ids)
    }
}
impl std::fmt::Display for ThingRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.ids)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct ActorCatalog {
    pub sources: Vec<SourceIdentity>,
    pub diagnostics: Vec<Diagnostic>,
    pub actors: Vec<ActorDefinition>,
    #[serde(skip)]
    lookup: BTreeMap<String, usize>,
    /// The Thing registry alone (4BCA00 slots), before any 427BD0 Thing
    /// translation is consulted. Keyed like `lookup`.
    #[serde(skip)]
    registered: BTreeMap<String, usize>,
    /// Memoized whole-tag resolution outcomes. The catalog is immutable after
    /// loading, so a cached outcome equals recomputing it.
    #[serde(skip)]
    pub(crate) resolution_cache: ResolutionCache,
    #[serde(skip)]
    resolver: Resolver,
    /// Generic DataManager registries of the native-order load, keyed by
    /// lowercase manager tag (empty for `from_files`).
    #[serde(skip)]
    native_data: BTreeMap<String, native_load::NativeDataRegistry>,
}
const FIELDS: &[(&str, &[&str], &str)] = &[
    ("category", &[], "category"),
    ("name", &[], "name"),
    ("View.resource_prefix", &["View"], "resource_prefix"),
    ("Flags.invisible", &["Flags"], "invisible"),
    ("BodyComponent.health", &["BodyComponent"], "health"),
    ("BodyComponent.defense", &["BodyComponent"], "defense"),
    ("MoverComponent.speed_max", &["MoverComponent"], "speed_max"),
];
#[derive(Clone, Debug)]
struct RawField {
    name: String,
    value: Option<Vec<u8>>,
    source: Provenance,
    declared_type: Option<String>,
    unsupported_declaration: bool,
}
#[derive(Clone, Debug)]
struct Definition {
    name: String,
    template: bool,
    optional: bool,
    list: bool,
    base: Option<String>,
    /// Template declaration bound when the original parser read this node
    /// (5BF0A0): root index in `Resolver::roots`, then child indices. Set
    /// only by the native-order loader; `None` keeps by-name lookup.
    bound: Option<Vec<usize>>,
    problem: Option<String>,
    fields: Vec<RawField>,
    children: Vec<Definition>,
    source: Provenance,
}
fn key(s: &str) -> String {
    s.to_ascii_lowercase()
}
fn provenance(identity: &SourceIdentity, span: crate::tgi::Span) -> Provenance {
    Provenance {
        path: identity.path.clone(),
        sha256: identity.sha256.clone(),
        span,
    }
}
fn definition(doc: &Document, node: &Node, identity: &SourceIdentity) -> Definition {
    let mut out = Definition {
        name: String::new(),
        template: false,
        optional: false,
        list: false,
        base: None,
        bound: None,
        problem: None,
        fields: Vec::new(),
        children: Vec::new(),
        source: provenance(identity, node.span),
    };
    let tokens = &node.header.tokens;
    let mut at = 0;
    while let Some(t) = tokens.get(at) {
        let text = doc.text(t.span).unwrap_or("");
        if text.eq_ignore_ascii_case("template") {
            out.template = true;
        } else if text.eq_ignore_ascii_case("optional") {
            out.optional = true;
        } else if text.eq_ignore_ascii_case("list") {
            out.list = true;
        } else {
            break;
        }
        at += 1;
    }
    if let Some(t) = tokens.get(at) {
        out.name = doc.text(t.span).unwrap_or("").to_owned();
        at += 1;
    }
    while at < tokens.len() {
        // The original header scan ignores '=' tokens that do not belong to
        // a key=value attribute ("[DenizenType = Template=DenizenType]",
        // "[A =]"); a bare word stays "Malformed section name." (oracle
        // section `headers`).
        if tokens[at].kind == TokenKind::Equals {
            at += 1;
            continue;
        }
        let attr = doc.text(tokens[at].span).unwrap_or("");
        if (attr.eq_ignore_ascii_case("template") || attr.eq_ignore_ascii_case("inherit"))
            && tokens
                .get(at + 1)
                .is_some_and(|t| t.kind == TokenKind::Equals)
            && tokens
                .get(at + 2)
                .is_some_and(|t| t.kind == TokenKind::Atom)
        {
            if out.base.is_some() {
                out.problem = Some("multiple template/inherit bases are unsupported".into());
            }
            out.base = doc.text(tokens[at + 2].span).map(str::to_owned);
            at += 3;
        } else {
            out.problem = Some(format!("unsupported node-header operation {attr:?}"));
            break;
        }
    }
    if out.name.is_empty() || !out.name.is_ascii() {
        out.problem = Some("missing/non-ASCII definition name".into());
    }
    for item in &node.items {
        match item {
            Item::Node(n) => out.children.push(definition(doc, n, identity)),
            Item::Property(p) => out.fields.push(RawField {
                name: doc.property_name(p).unwrap_or("").to_owned(),
                value: p.value.and_then(|s| doc.bytes(s)).map(|b| b.to_vec()),
                source: provenance(identity, p.span),
                declared_type: (p.left.len() == 2)
                    .then(|| doc.text(p.left[0].span).unwrap_or("").to_owned()),
                unsupported_declaration: p.left.len() > 2,
            }),
        }
    }
    out
}

#[derive(Clone, Debug, Default)]
struct Resolver {
    roots: Vec<Definition>,
    templates: BTreeMap<String, Vec<usize>>,
    upgrade_census_complete: bool,
    opaque_sources_without_upgrade_bytes: Vec<SourceIdentity>,
    opaque_sources: Vec<(SourceIdentity, Vec<u8>)>,
    source_hashes_valid: bool,
}
impl Resolver {
    /// Every node's resolver path, keyed by its address in this resolver.
    pub(crate) fn node_paths(&self) -> std::collections::HashMap<usize, Vec<usize>> {
        fn walk(
            node: &Definition,
            path: &mut Vec<usize>,
            out: &mut std::collections::HashMap<usize, Vec<usize>>,
        ) {
            out.insert(node as *const Definition as usize, path.clone());
            for (index, child) in node.children.iter().enumerate() {
                path.push(index);
                walk(child, path, out);
                path.pop();
            }
        }
        let mut out = std::collections::HashMap::new();
        for (index, root) in self.roots.iter().enumerate() {
            walk(root, &mut vec![index], &mut out);
        }
        out
    }
    pub(crate) fn node_at(&self, path: &[usize]) -> Option<&Definition> {
        let (first, rest) = path.split_first()?;
        let mut node = self.roots.get(*first)?;
        for &child in rest {
            node = node.children.get(child)?;
        }
        Some(node)
    }
    fn layers<'a>(
        &'a self,
        node: &'a Definition,
        stack: &mut Vec<String>,
    ) -> Result<Vec<&'a Definition>, String> {
        self.layer_sequence(node, stack, false)
    }
    fn layer_sequence<'a>(
        &'a self,
        node: &'a Definition,
        stack: &mut Vec<String>,
        allow_list_declaration: bool,
    ) -> Result<Vec<&'a Definition>, String> {
        if let Some(e) = &node.problem {
            return Err(e.clone());
        }
        if node.list && !allow_list_declaration {
            return Err(format!("list node {} needs list semantics", node.name));
        }
        let mut out = vec![node];
        if let Some(name) = &node.base {
            let name_key = key(name);
            if stack.contains(&name_key) {
                return Err(format!("template cycle at {name}"));
            }
            if stack.len() >= 256 {
                return Err("template ancestry exceeds 256-node safety budget".into());
            }
            let target = if let Some(path) = &node.bound {
                self.node_at(path)
                    .ok_or_else(|| format!("bound template {name} unavailable"))?
            } else {
                let entries = self
                    .templates
                    .get(&name_key)
                    .ok_or_else(|| format!("missing template {name}"))?;
                if entries.len() != 1 {
                    return Err(format!(
                        "ambiguous template {name}: {} declarations",
                        entries.len()
                    ));
                }
                &self.roots[entries[0]]
            };
            stack.push(name_key);
            out.extend(self.layers(target, stack)?);
            stack.pop();
        }
        Ok(out)
    }
    fn path_layers<'a>(
        &'a self,
        node: &'a Definition,
        path: &[&str],
    ) -> Result<Vec<&'a Definition>, String> {
        let mut layers = self.layers(node, &mut Vec::new())?;
        for segment in path {
            let mut children = Vec::new();
            let mut ordinary = false;
            let mut bases = Vec::new();
            for layer in layers {
                let matches: Vec<_> = layer
                    .children
                    .iter()
                    .filter(|c| c.name.eq_ignore_ascii_case(segment))
                    .collect();
                if matches.len() > 1 {
                    return Err(format!("duplicate/list child {segment} is unsupported"));
                }
                if let Some(child) = matches.first() {
                    if child.list {
                        return Err(format!("list child {segment} is unsupported"));
                    }
                    ordinary |= !child.optional;
                    let child_layers = self.layers(child, &mut Vec::new())?;
                    if let Some(base) = &child.base {
                        bases.push((
                            base,
                            child_layers
                                .iter()
                                .skip(1)
                                .map(|n| &n.name)
                                .collect::<Vec<_>>(),
                        ));
                    }
                    children.extend(child_layers);
                }
            }
            // 5B6190 does not instantiate absent optional children on a Thing.
            if !ordinary {
                return Ok(Vec::new());
            }
            // Original5B57FC..5B5952 permits a more specific child template
            // only when its native ancestry includes the inherited template.
            // Sibling/unrelated template replacement remains an error.
            if let Some((_, most_specific)) = bases.first()
                && bases.iter().skip(1).any(|(base, _)| {
                    !most_specific
                        .iter()
                        .any(|name| name.eq_ignore_ascii_case(base))
                })
            {
                return Err(format!(
                    "child template at {segment} is not a descendant of its inherited template"
                ));
            }
            layers = children;
        }
        Ok(layers)
    }
    fn raw_field(
        &self,
        node: &Definition,
        path: &[&str],
        name: &str,
    ) -> Result<Option<RawField>, String> {
        let layers = self.path_layers(node, path)?;
        for layer in layers {
            let matches: Vec<_> = layer
                .fields
                .iter()
                .filter(|f| f.name.eq_ignore_ascii_case(name))
                .collect();
            if matches.len() > 1 {
                return Err(format!(
                    "duplicate field {name} needs assignment-order semantics"
                ));
            }
            if let Some(field) = matches.first() {
                return Ok(Some((*field).clone()));
            }
        }
        Ok(None)
    }
    fn field(&self, node: &Definition, path: &[&str], name: &str) -> FieldResolution {
        match self.raw_field(node, path, name) {
            Ok(None) => FieldResolution::Missing,
            Ok(Some(RawField {
                value: None,
                source,
                ..
            })) => FieldResolution::Declared { source },
            Ok(Some(RawField {
                value: Some(raw),
                source,
                ..
            })) => FieldResolution::Value { raw, source },
            Err(reason) => FieldResolution::Unsupported { reason },
        }
    }
}
impl ActorCatalog {
    /// The original data load (index files, native order, parse-time template
    /// binding, first-wins Thing registry, Thing translations). See
    /// `native_load` for the native addresses and the remaining scope.
    pub fn load_v14(sources: &mut V14Sources) -> Result<Self, SourceError> {
        native_load::load(sources).map(|(catalog, _)| catalog)
    }
    /// Every supplied TGI member in catalog order with by-name template
    /// lookup. A syntax census over all bytes; NOT the native registry.
    pub fn load_v14_all_sources(sources: &mut V14Sources) -> Result<Self, SourceError> {
        let paths: Vec<_> = sources
            .catalog()
            .into_iter()
            .filter(|r| r.logical_path.to_ascii_lowercase().ends_with(".tgi"))
            .map(|r| r.logical_path)
            .collect();
        let mut files = Vec::new();
        for path in paths {
            files.push(sources.read(&path)?);
        }
        Ok(Self::from_files(files, Limits::default()))
    }
    pub fn from_files(files: Vec<ResolvedFile>, limits: Limits) -> Self {
        let mut catalog = Self {
            sources: Vec::new(),
            diagnostics: Vec::new(),
            actors: Vec::new(),
            lookup: BTreeMap::new(),
            registered: BTreeMap::new(),
            resolution_cache: ResolutionCache::default(),
            resolver: Resolver::default(),
            native_data: BTreeMap::new(),
        };
        let mut resolver = Resolver {
            roots: Vec::new(),
            templates: BTreeMap::new(),
            upgrade_census_complete: true,
            opaque_sources_without_upgrade_bytes: Vec::new(),
            opaque_sources: Vec::new(),
            source_hashes_valid: true,
        };
        for file in files {
            let identity = SourceIdentity {
                path: file.logical_path,
                sha256: file.sha256,
                origin: file.origin,
            };
            if sha256(&file.bytes) != identity.sha256 {
                resolver.upgrade_census_complete = false;
                resolver.source_hashes_valid = false;
                catalog.diagnostics.push(Diagnostic {
                    source: identity.clone(),
                    message: "source SHA256 does not match supplied bytes; omitted".into(),
                });
                catalog.sources.push(identity);
                continue;
            }
            let contains_nul = file.bytes.contains(&0);
            if contains_nul {
                resolver.upgrade_census_complete = false;
                resolver.source_hashes_valid = false;
            }
            let could_contain_upgrade = contains_nul
                || file
                    .bytes
                    .windows(7)
                    .any(|w| w.eq_ignore_ascii_case(b"Upgrade"));
            match Document::parse_native(file.bytes.clone(), limits) {
                Ok(doc) => {
                    for notice in &doc.notices {
                        catalog.diagnostics.push(Diagnostic {
                            source: identity.clone(),
                            message: format!(
                                "native syntax notice {:?} at {}",
                                notice.kind, notice.span.start
                            ),
                        });
                    }
                    for item in &doc.items {
                        if let Item::Node(node) = item {
                            let entry = definition(&doc, node, &identity);
                            if entry.template {
                                resolver
                                    .templates
                                    .entry(key(&entry.name))
                                    .or_default()
                                    .push(resolver.roots.len());
                            }
                            resolver.roots.push(entry);
                        }
                    }
                }
                Err(error) => {
                    resolver.opaque_sources.push((identity.clone(), file.bytes));
                    if could_contain_upgrade {
                        resolver.upgrade_census_complete = false;
                    } else {
                        resolver
                            .opaque_sources_without_upgrade_bytes
                            .push(identity.clone());
                    }
                    catalog.diagnostics.push(Diagnostic {
                        source: identity.clone(),
                        message: error.to_string(),
                    });
                }
            }
            catalog.sources.push(identity);
        }
        for node in resolver
            .roots
            .iter()
            .filter(|n| !n.template && n.name.eq_ignore_ascii_case("Thing"))
        {
            let direct_ids: Vec<_> = node
                .fields
                .iter()
                .filter(|f| f.name.eq_ignore_ascii_case("IDS"))
                .collect();
            let id_field = if direct_ids.len() == 1 {
                Ok(Some(direct_ids[0].clone()))
            } else {
                resolver.raw_field(node, &[], "IDS")
            };
            let id = match id_field {
                Ok(Some(RawField {
                    value: Some(raw), ..
                })) => match std::str::from_utf8(&raw) {
                    Ok(s)
                        if !s.is_empty()
                            && s.bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b)) =>
                    {
                        s.to_owned()
                    }
                    _ => continue,
                },
                _ => continue,
            };
            let chain = resolver
                .layers(node, &mut Vec::new())
                .unwrap_or_default()
                .iter()
                .map(|n| n.source.clone())
                .collect();
            let mut entry = ActorDefinition {
                id: id.clone(),
                source: node.source.clone(),
                template_chain: chain,
                fields: FIELDS
                    .iter()
                    .map(|(label, path, field)| {
                        (label.to_string(), resolver.field(node, path, field))
                    })
                    .collect(),
            };
            if let Some(prior) = catalog.lookup.get(&key(&id)).copied() {
                let reason = format!("ambiguous Thing IDS {id}: multiple source declarations");
                for value in catalog.actors[prior].fields.values_mut() {
                    *value = FieldResolution::Unsupported {
                        reason: reason.clone(),
                    };
                }
                for value in entry.fields.values_mut() {
                    *value = FieldResolution::Unsupported {
                        reason: reason.clone(),
                    };
                }
            } else {
                catalog.lookup.insert(key(&id), catalog.actors.len());
                catalog.registered.insert(key(&id), catalog.actors.len());
            }
            catalog.actors.push(entry);
        }
        catalog.resolver = resolver;
        catalog
    }
    /// Original 427BD0 Find: a translated IDS selects its target only.
    pub fn find(&self, id: &str) -> Option<&ActorDefinition> {
        self.lookup.get(&key(id)).map(|&i| &self.actors[i])
    }
    /// The allocation registered under exactly this IDS: the pointer 4BCA00
    /// returns and 4BDC50 passes to vf0 LoadData. Later Thing translations do
    /// not change it (`chaparral_wall` stays its own allocation although Find
    /// returns `temperate_walls`). Use it to re-identify a definition that was
    /// already resolved; resolve authored references with [`Self::find`].
    pub fn registered(&self, id: &str) -> Option<&ActorDefinition> {
        self.registered.get(&key(id)).map(|&i| &self.actors[i])
    }
    pub fn select<'a>(&self, thing: impl Into<ThingRef<'a>>) -> Option<&ActorDefinition> {
        let thing = thing.into();
        match thing.lookup {
            ThingLookup::Find => self.find(thing.ids),
            ThingLookup::Registered => self.registered(thing.ids),
        }
    }
    /// A conservative whole-input syntax census, including otherwise ambiguous
    /// templates. Absence across this superset proves no Construction Upgrade
    /// callbacks can arise from the supplied sources. A parse omission is safe
    /// only if hash-checked non-NUL bytes cannot contain the node name at all.
    /// Other omissions or explicit instances prevent an empty-graph certificate.
    pub fn construction_upgrade_census(&self) -> crate::native::Result<ConstructionUpgradeCensus> {
        if !self.resolver.upgrade_census_complete {
            return Err(crate::native::Error::Unsupported("Construction upgrade census cannot exclude Upgrade nodes in omitted/changed source bytes".into()));
        }
        fn walk(node: &Definition, out: &mut ConstructionUpgradeCensus) {
            if node.name.eq_ignore_ascii_case("Upgrade") {
                if node.list || node.template {
                    out.declarations.push(node.source.clone());
                } else {
                    out.explicit_instances.push(node.source.clone());
                }
            }
            for child in &node.children {
                walk(child, out);
            }
        }
        let mut out = ConstructionUpgradeCensus {
            sources: self.sources.clone(),
            declarations: Vec::new(),
            explicit_instances: Vec::new(),
            opaque_sources_without_upgrade_bytes: self
                .resolver
                .opaque_sources_without_upgrade_bytes
                .clone(),
        };
        for root in &self.resolver.roots {
            walk(root, &mut out);
        }
        Ok(out)
    }

    /// Conservative reference superset across all nodes, including unused or
    /// ambiguous templates. Negative matches are authoritative; positive ones
    /// do not imply a template was instantiated by native index loading.
    pub(crate) fn literal_reference_census(
        &self,
        names: &[&str],
    ) -> crate::native::Result<LiteralReferenceCensus> {
        use crate::native::Error;
        if !self.resolver.source_hashes_valid {
            return Err(Error::Unsupported(
                "reference census has changed or unsupported encoded source bytes".into(),
            ));
        }
        let mut opaque = Vec::new();
        for (source, bytes) in &self.resolver.opaque_sources {
            if bytes.contains(&0)
                || names.iter().any(|n| {
                    bytes
                        .windows(n.len())
                        .any(|b| b.eq_ignore_ascii_case(n.as_bytes()))
                })
            {
                return Err(Error::Unsupported(format!(
                    "reference census cannot exclude requested fields from omitted {}",
                    source.path
                )));
            }
            opaque.push(source.clone());
        }
        fn walk(
            node: &Definition,
            names: &[&str],
            out: &mut Vec<LiteralReferenceInput>,
        ) -> crate::native::Result<()> {
            for field in &node.fields {
                if names.iter().any(|n| n.eq_ignore_ascii_case(&field.name)) {
                    let Some(raw) = &field.value else {
                        continue;
                    };
                    let value = std::str::from_utf8(raw)
                        .map_err(|_| {
                            Error::Unsupported(
                                "reference census contains non-UTF8 expression".into(),
                            )
                        })?
                        .trim();
                    if !value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
                    {
                        return Err(Error::Unsupported(format!(
                            "reference census requires literal IDs for {}",
                            field.name
                        )));
                    }
                    let id = if value.eq_ignore_ascii_case("null") {
                        String::new()
                    } else {
                        value.to_owned()
                    };
                    out.push(LiteralReferenceInput {
                        field: field.name.clone(),
                        id,
                        source: field.source.clone(),
                    });
                }
            }
            for child in &node.children {
                walk(child, names, out)?;
            }
            Ok(())
        }
        let mut references = Vec::new();
        for node in &self.resolver.roots {
            walk(node, names, &mut references)?;
        }
        Ok(LiteralReferenceCensus {
            sources: self.sources.clone(),
            references,
            opaque_sources_without_field_names: opaque,
        })
    }
    /// Every resolver root whose source is exactly `source`, ascending: the
    /// matches of a linear scan over the roots, in scan order. The resolver
    /// is private and immutable after loading, so this index never changes.
    pub(crate) fn roots_with_source(&self, source: &Provenance) -> &[usize] {
        self.resolution_cache
            .memo
            .roots_by_source
            .get_or_init(|| {
                let mut out: std::collections::HashMap<Provenance, Vec<usize>> =
                    std::collections::HashMap::new();
                for (index, node) in self.resolver.roots.iter().enumerate() {
                    out.entry(node.source.clone()).or_default().push(index);
                }
                out
            })
            .get(source)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    /// A shared copy of `sources`, equal to it at the time of this call (an
    /// unchanged list returns the same allocation again, so callers holding
    /// copies of one catalog's sources can recognise each other's copies).
    pub fn sources_snapshot(&self) -> std::sync::Arc<Vec<SourceIdentity>> {
        let memo = &self.resolution_cache.memo.sources_snapshot;
        let mut slot = match memo.lock() {
            Ok(slot) => slot,
            Err(_) => return std::sync::Arc::new(self.sources.clone()),
        };
        if let Some(snapshot) = slot.as_ref()
            && **snapshot == self.sources
        {
            return std::sync::Arc::clone(snapshot);
        }
        let snapshot = std::sync::Arc::new(self.sources.clone());
        *slot = Some(std::sync::Arc::clone(&snapshot));
        snapshot
    }
    /// First root with exactly this source (`roots.iter().position(..)`).
    pub(crate) fn root_with_source(&self, source: &Provenance) -> Option<usize> {
        self.roots_with_source(source).first().copied()
    }
    /// The selection and category check every per-definition query performs
    /// first (same errors), then the root of the selected entry's source.
    pub(crate) fn selected_root(&self, id: ThingRef<'_>) -> crate::native::Result<Option<usize>> {
        use crate::native::Error;
        let actor = self
            .select(id)
            .ok_or_else(|| Error::Unsupported(format!("missing actor {id}")))?;
        if let Some(FieldResolution::Unsupported { reason }) = actor.field("category") {
            return Err(Error::Unsupported(reason.clone()));
        }
        Ok(self.root_with_source(&actor.source))
    }
    /// Resolve one typed native field through the proved ancestry/child merge
    /// subset. Unknown source/load-order ambiguity remains an explicit error.
    pub fn native_field<'a>(
        &self,
        id: impl Into<ThingRef<'a>>,
        path: &str,
        precision: aa_numeric::Precision,
    ) -> crate::native::Result<Option<crate::native::TypedValue>> {
        use crate::native::Error;
        let id = id.into();
        let actor = self
            .select(id)
            .ok_or_else(|| Error::Unsupported(format!("missing actor {id}")))?;
        if let Some(FieldResolution::Unsupported { reason }) = actor.field("category") {
            return Err(Error::Unsupported(reason.clone()));
        }
        let root = self
            .root_with_source(&actor.source)
            .ok_or_else(|| Error::Unsupported("actor source node unavailable".into()))?;
        let key = (root, precision.significand_bits());
        let memo = &self.resolution_cache.memo.fields;
        if let Some(hit) = crate::query_memo::lock(memo)
            .and_then(|m| m.get(&key).and_then(|m| m.get(path)).cloned())
        {
            return Ok(hit);
        }
        let node = &self.resolver.roots[root];
        let parts: Vec<_> = path.split('.').collect();
        let (name, children) = parts
            .split_last()
            .ok_or_else(|| Error::Unsupported("empty field path".into()))?;
        let layers = self
            .resolver
            .path_layers(node, children)
            .map_err(Error::Unsupported)?;
        let result = ResolvedNode {
            catalog: self,
            layers,
        }
        .field(name, precision);
        if let (Ok(value), Some(mut memo)) = (&result, crate::query_memo::lock(memo)) {
            memo.entry(key).or_default().insert(path.to_owned(), value.clone());
        }
        result
    }

    /// Presence of an ordinary resolved child; optional declarations alone do
    /// not instantiate a component. This does not evaluate a component mask.
    pub fn native_node_sources<'a>(
        &self,
        id: impl Into<ThingRef<'a>>,
        path: &[&str],
    ) -> crate::native::Result<Vec<Provenance>> {
        use crate::native::Error;
        let id = id.into();
        let actor = self
            .select(id)
            .ok_or_else(|| Error::Unsupported(format!("missing actor {id}")))?;
        if let Some(FieldResolution::Unsupported { reason }) = actor.field("category") {
            return Err(Error::Unsupported(reason.clone()));
        }
        let root = self
            .root_with_source(&actor.source)
            .ok_or_else(|| Error::Unsupported("actor source unavailable".into()))?;
        let key = (root, crate::query_memo::path_key(path));
        let memo = &self.resolution_cache.memo.node_sources;
        if let Some(hit) = crate::query_memo::lock(memo).and_then(|m| m.get(&key).cloned()) {
            return Ok(hit);
        }
        let node = &self.resolver.roots[root];
        let sources: Vec<Provenance> = self
            .resolver
            .path_layers(node, path)
            .map_err(Error::Unsupported)?
            .iter()
            .map(|n| n.source.clone())
            .collect();
        if let Some(mut memo) = crate::query_memo::lock(memo) {
            memo.insert(key, sources.clone());
        }
        Ok(sources)
    }
}
#[cfg(test)]
#[path = "query_memo_tests.rs"]
mod query_memo_tests;
