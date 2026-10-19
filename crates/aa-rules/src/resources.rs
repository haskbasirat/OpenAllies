//! Ordered, typed projection of the verified resource schema only.
//! No general template inheritance or expression evaluation is performed.
use crate::{
    sources::{Origin, ResolvedFile, SourceError, V14Sources},
    tgi::{Document, Item, Node, ParseError, Property, Span, TokenKind},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error(transparent)]
    Syntax(#[from] ParseError),
    #[error("unsupported resource schema: {0}")]
    Schema(String),
}
type Result<T> = std::result::Result<T, CatalogError>;
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Provenance {
    pub path: String,
    pub sha256: String,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceIdentity {
    pub path: String,
    pub sha256: String,
    pub origin: Origin,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Scalar {
    /// Original spelling. Only single finite numeric literals are decoded.
    pub raw: String,
    pub f32_bits: u32,
    pub source: Provenance,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResourceRow {
    pub resource_id: String,
    pub value: Scalar,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResourceDeclaration {
    pub id: String,
    pub declared_type: String,
    pub default_value: Option<Scalar>,
    pub source: Provenance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum ResourceCategory {
    Stockpiled = 0,
    Upkeep = 1,
    LimitedConsumed = 2,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResourceDefinition {
    pub id: String,
    pub template: String,
    pub category: ResourceCategory,
    /// Author order in resources.tgi, NOT asserted to be a native registry index.
    pub declaration_ordinal: usize,
    /// Separate author order in ResourceList, which differs from definitions.
    pub resource_list_ordinal: usize,
    pub limited_by: Option<String>,
    pub limited_provider: bool,
    /// Explicit authored coefficients. Absent rows are not invented as zero.
    pub purchase_cost: Vec<ResourceRow>,
    pub source: Provenance,
    pub category_source: Provenance,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResourceCatalog {
    pub sources: Vec<SourceIdentity>,
    pub resource_list: Vec<ResourceDeclaration>,
    pub resources: Vec<ResourceDefinition>,
    pub cost_defaults: Vec<ResourceRow>,
    pub upkeep_defaults: Vec<ResourceRow>,
    pub production_defaults: Vec<ResourceRow>,
    pub limited_resource_max: i32,
    pub limited_resource_max_source: Provenance,
    pub stockpiled_resource_max: Scalar,
    pub start_location_production: Vec<ResourceRow>,
}
struct Input {
    identity: SourceIdentity,
    document: Document,
}
impl Input {
    fn new(file: ResolvedFile) -> Result<Self> {
        Ok(Self {
            identity: SourceIdentity {
                path: file.logical_path,
                sha256: file.sha256,
                origin: file.origin,
            },
            document: Document::parse(file.bytes)?,
        })
    }
    fn provenance(&self, span: Span) -> Provenance {
        Provenance {
            path: self.identity.path.clone(),
            sha256: self.identity.sha256.clone(),
            span,
        }
    }
}
impl ResourceCatalog {
    pub fn load_v14(sources: &mut V14Sources) -> Result<Self> {
        let list = Input::new(sources.read("Game/resource_list.tgi")?)?;
        let templates = Input::new(sources.read("Templates/template_resource.tgi")?)?;
        let definitions = Input::new(sources.read("Game/resources.tgi")?)?;
        let limits = Input::new(sources.read("Game/SVars_AA.tgi")?)?;
        let start = Input::new(sources.read("Special/Things/thing_start_location.tgi")?)?;
        let base_limits = Input::new(sources.read("Game/SVars.tgi")?)?;
        build(
            &list,
            &templates,
            &definitions,
            &limits,
            &start,
            &base_limits,
        )
    }
    pub fn find(&self, id: &str) -> Option<&ResourceDefinition> {
        self.resources.iter().find(|r| r.id == id)
    }
}
fn schema(message: impl Into<String>) -> CatalogError {
    CatalogError::Schema(message.into())
}
fn children(n: &Node) -> impl Iterator<Item = &Node> {
    n.items
        .iter()
        .filter_map(|i| if let Item::Node(n) = i { Some(n) } else { None })
}
fn properties(n: &Node) -> impl Iterator<Item = &Property> {
    n.items.iter().filter_map(|i| {
        if let Item::Property(p) = i {
            Some(p)
        } else {
            None
        }
    })
}
fn property<'a>(input: &Input, node: &'a Node, name: &str) -> Result<&'a Property> {
    let all: Vec<_> = properties(node)
        .filter(|p| input.document.property_name(p) == Some(name))
        .collect();
    if all.len() != 1 {
        return Err(schema(format!("expected exactly one {name}")));
    }
    Ok(all[0])
}
fn atom(input: &Input, p: &Property) -> Result<String> {
    if p.value_tokens.len() != 1 || p.value_tokens[0].kind != TokenKind::Atom {
        return Err(schema("expected one unquoted literal/identifier"));
    }
    input
        .document
        .property_value(p)
        .map(str::to_owned)
        .ok_or_else(|| schema("non-UTF8 identifier"))
}
fn scalar(input: &Input, p: &Property) -> Result<Scalar> {
    let raw = atom(input, p)?;
    // This schema's numbers are decimal integers. A wider numeric/expression
    // evaluator would need independent native conversion/rounding evidence.
    let integer: i32 = raw.parse().map_err(|_| {
        schema(format!(
            "non-integer scalar outside resource projection: {raw}"
        ))
    })?;
    if integer == 0 && raw.starts_with('-') {
        return Err(schema(
            "signed zero requires native floating conversion evidence",
        ));
    }
    if integer.unsigned_abs() > 16_777_216 {
        return Err(schema("integer not exactly representable in binary32"));
    }
    Ok(Scalar {
        raw,
        f32_bits: (integer as f32).to_bits(),
        source: input.provenance(p.span),
    })
}
fn header_attr(input: &Input, n: &Node, key: &str) -> Result<Option<String>> {
    let t = &n.header.tokens;
    let mut values = Vec::new();
    for w in t.windows(3) {
        if input
            .document
            .text(w[0].span)
            .is_some_and(|v| v.eq_ignore_ascii_case(key))
            && w[1].kind == TokenKind::Equals
        {
            if w[2].kind != TokenKind::Atom {
                return Err(schema("header attribute is not an atom"));
            }
            values.push(
                input
                    .document
                    .text(w[2].span)
                    .ok_or_else(|| schema("non-UTF8 header value"))?
                    .to_owned(),
            );
        }
    }
    if values.len() > 1 {
        return Err(schema(format!("duplicate header attribute {key}")));
    }
    Ok(values.pop())
}
fn named_template<'a>(input: &'a Input, name: &str) -> Result<&'a Node> {
    let all: Vec<_> = input
        .document
        .nodes()
        .into_iter()
        .filter(|n| {
            input
                .document
                .header_name(&n.header)
                .is_some_and(|v| v.eq_ignore_ascii_case("template"))
                && n.header
                    .tokens
                    .get(1)
                    .and_then(|t| input.document.text(t.span))
                    == Some(name)
        })
        .collect();
    if all.len() != 1 {
        return Err(schema(format!("expected one template {name}")));
    }
    Ok(all[0])
}
fn rows(input: &Input, n: &Node) -> Result<Vec<ResourceRow>> {
    if children(n).next().is_some() {
        return Err(schema("unexpected nested resource value node"));
    }
    let mut ids = BTreeSet::new();
    let mut out = Vec::new();
    for p in properties(n) {
        let id = input
            .document
            .property_name(p)
            .ok_or_else(|| schema("missing resource key"))?
            .to_owned();
        if !ids.insert(id.clone()) {
            return Err(schema("duplicate resource value"));
        }
        out.push(ResourceRow {
            resource_id: id,
            value: scalar(input, p)?,
        });
    }
    Ok(out)
}
fn build(
    list: &Input,
    templates: &Input,
    definitions: &Input,
    limits: &Input,
    start: &Input,
    base_limits: &Input,
) -> Result<ResourceCatalog> {
    let root = named_template(list, "ResourceList")?;
    let mut resource_list = Vec::new();
    let mut ids = BTreeMap::new();
    for p in properties(root) {
        if p.left.len() != 2 {
            return Err(schema("ResourceList declaration shape"));
        }
        let ty = list
            .document
            .text(p.left[0].span)
            .ok_or_else(|| schema("non-UTF8 declaration"))?;
        if ty != "fixed" {
            return Err(schema("unsupported ResourceList type"));
        }
        let id = list
            .document
            .property_name(p)
            .ok_or_else(|| schema("resource name"))?
            .to_owned();
        if ids.insert(id.clone(), resource_list.len()).is_some() {
            return Err(schema("duplicate ResourceList ID"));
        }
        resource_list.push(ResourceDeclaration {
            id,
            declared_type: ty.into(),
            default_value: if p.value.is_some() {
                Some(scalar(list, p)?)
            } else {
                None
            },
            source: list.provenance(p.span),
        });
    }
    let mut categories = BTreeMap::new();
    for (name, tag, category) in [
        (
            "PurchaseResource",
            "STOCKPILED",
            ResourceCategory::Stockpiled,
        ),
        ("UpkeepResource", "UPKEEP", ResourceCategory::Upkeep),
        (
            "LimitedConsumedResource",
            "LIMITED_CONSUMED",
            ResourceCategory::LimitedConsumed,
        ),
    ] {
        let n = named_template(templates, name)?;
        if header_attr(templates, n, "inherit")?.as_deref() != Some("Resource") {
            return Err(schema("category template must inherit known Resource base"));
        }
        let p = property(templates, n, "type")?;
        if atom(templates, p)? != tag {
            return Err(schema("resource category tag mismatch"));
        }
        categories.insert(name.to_owned(), (category, templates.provenance(p.span)));
    }
    let base = named_template(templates, "Resource")?;
    if atom(templates, property(templates, base, "limited_by")?)? != "null" {
        return Err(schema("Resource limited_by default changed"));
    }
    let upkeep = named_template(templates, "UpkeepResource")?;
    let cost_children: Vec<_> = children(upkeep)
        .filter(|n| templates.document.header_name(&n.header) == Some("PurchaseCost"))
        .collect();
    if cost_children.len() != 1
        || header_attr(templates, cost_children[0], "template")?.as_deref() != Some("CostList")
    {
        return Err(schema("UpkeepResource PurchaseCost template shape"));
    }
    let cost_defaults = rows(list, named_template(list, "CostList")?)?;
    let up = named_template(list, "UpkeepList")?;
    let production = named_template(list, "ProductionList")?;
    for n in [up, production] {
        if header_attr(list, n, "template")?.as_deref() != Some("ResourceList") {
            return Err(schema("resource default list parent changed"));
        }
    }
    let upkeep_defaults = rows(list, up)?;
    let production_defaults = rows(list, production)?;
    let mut resources = Vec::new();
    let mut defined = BTreeSet::new();
    for item in &definitions.document.items {
        let Item::Node(n) = item else {
            return Err(schema("root property in resource definitions"));
        };
        if definitions.document.header_name(&n.header) != Some("Resource") {
            return Err(schema("unknown resource root node"));
        }
        let id = atom(definitions, property(definitions, n, "IDS")?)?;
        if !defined.insert(id.clone()) {
            return Err(schema("duplicate resource definition"));
        }
        let ordinal = *ids
            .get(&id)
            .ok_or_else(|| schema("resource absent from ResourceList"))?;
        let template = header_attr(definitions, n, "template")?
            .ok_or_else(|| schema("resource template absent"))?;
        let (category, category_source) = categories
            .get(&template)
            .ok_or_else(|| schema("unknown resource template"))?;
        let bindings: Vec<_> = properties(n)
            .filter(|p| definitions.document.property_name(p) == Some("limited_by"))
            .collect();
        let limited_by = match (*category, bindings.as_slice()) {
            (ResourceCategory::LimitedConsumed, [p]) => Some(atom(definitions, p)?),
            (ResourceCategory::LimitedConsumed, _) => {
                return Err(schema("limited consumer needs one provider"));
            }
            (_, []) => None,
            _ => return Err(schema("unexpected limited_by override")),
        };
        let mut purchase_cost = Vec::new();
        let nested: Vec<_> = children(n).collect();
        if nested.len() > 1 {
            return Err(schema("duplicate/unexpected resource child"));
        }
        if let Some(c) = nested.first() {
            if definitions.document.header_name(&c.header) != Some("PurchaseCost")
                || *category != ResourceCategory::Upkeep
            {
                return Err(schema("unexpected PurchaseCost child"));
            }
            purchase_cost = rows(definitions, c)?;
        }
        resources.push(ResourceDefinition {
            id,
            template,
            category: *category,
            declaration_ordinal: resources.len(),
            resource_list_ordinal: ordinal,
            limited_by,
            limited_provider: false,
            purchase_cost,
            source: definitions.provenance(n.span),
            category_source: category_source.clone(),
        });
    }
    if defined.len() != resource_list.len() {
        return Err(schema("definition/list membership differs"));
    }
    for index in 0..resources.len() {
        if let Some(provider) = resources[index].limited_by.clone() {
            let i = resources
                .iter()
                .position(|r| r.id == provider)
                .ok_or_else(|| schema("unknown provider"))?;
            if resources[i].category != ResourceCategory::Upkeep || resources[i].limited_provider {
                return Err(schema("provider must be unique UPKEEP binding"));
            }
            resources[i].limited_provider = true;
        }
    }
    for r in &resources {
        for row in &r.purchase_cost {
            if !resources
                .iter()
                .any(|v| v.id == row.resource_id && v.category == ResourceCategory::Stockpiled)
            {
                return Err(schema("PurchaseCost destination is not stockpiled"));
            }
        }
    }
    for row in cost_defaults
        .iter()
        .chain(&upkeep_defaults)
        .chain(&production_defaults)
    {
        if !ids.contains_key(&row.resource_id) {
            return Err(schema("default row ID absent"));
        }
    }
    let max_fields: Vec<_> = limits
        .document
        .nodes()
        .into_iter()
        .flat_map(properties)
        .filter(|p| limits.document.property_name(p) == Some("EconomyLimitedResourceMax"))
        .collect();
    if max_fields.len() != 1 {
        return Err(schema("limited maximum must be a single v14 declaration"));
    }
    let max = atom(limits, max_fields[0])?
        .parse::<i32>()
        .map_err(|_| schema("limited maximum is not i32"))?;
    let stockpile_fields: Vec<_> = base_limits
        .document
        .nodes()
        .into_iter()
        .flat_map(properties)
        .filter(|p| base_limits.document.property_name(p) == Some("EconomyStockpileResourceMax"))
        .collect();
    if stockpile_fields.len() != 1
        || limits
            .document
            .nodes()
            .into_iter()
            .flat_map(properties)
            .any(|p| limits.document.property_name(p) == Some("EconomyStockpileResourceMax"))
    {
        return Err(schema(
            "stockpile maximum differs from verified base-only declaration schema",
        ));
    }
    let stockpiled_resource_max = scalar(base_limits, stockpile_fields[0])?;
    let start_nodes: Vec<_> = start
        .document
        .nodes()
        .into_iter()
        .filter(|n| start.document.header_name(&n.header) == Some("Production"))
        .collect();
    if start_nodes.len() != 1 {
        return Err(schema("start location production shape"));
    }
    let start_location_production = rows(start, start_nodes[0])?;
    Ok(ResourceCatalog {
        sources: [list, templates, definitions, limits, start, base_limits]
            .iter()
            .map(|i| i.identity.clone())
            .collect(),
        resource_list,
        resources,
        cost_defaults,
        upkeep_defaults,
        production_defaults,
        limited_resource_max: max,
        limited_resource_max_source: limits.provenance(max_fields[0].span),
        stockpiled_resource_max,
        start_location_production,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> Vec<Input> {
        [
            (
                "Game/resource_list.tgi",
                include_bytes!("../tests/fixtures/resource-schema/resource_list.tgi").as_slice(),
            ),
            (
                "Templates/template_resource.tgi",
                include_bytes!("../tests/fixtures/resource-schema/template_resource.tgi")
                    .as_slice(),
            ),
            (
                "Game/resources.tgi",
                include_bytes!("../tests/fixtures/resource-schema/resources.tgi").as_slice(),
            ),
            (
                "Game/SVars_AA.tgi",
                include_bytes!("../tests/fixtures/resource-schema/SVars_AA.tgi").as_slice(),
            ),
            (
                "Special/Things/thing_start_location.tgi",
                include_bytes!("../tests/fixtures/resource-schema/thing_start_location.tgi")
                    .as_slice(),
            ),
            (
                "Game/SVars.tgi",
                include_bytes!("../tests/fixtures/resource-schema/SVars.tgi").as_slice(),
            ),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (p, b))| {
            Input::new(ResolvedFile {
                logical_path: p.into(),
                origin: if i < 3 || i == 5 {
                    Origin::OriginalArchive { member_index: i }
                } else {
                    Origin::V14Loose
                },
                sha256: crate::sources::sha256(b),
                bytes: b.to_vec(),
            })
            .unwrap()
        })
        .collect()
    }
    fn catalog(i: &[Input]) -> Result<ResourceCatalog> {
        build(&i[0], &i[1], &i[2], &i[3], &i[4], &i[5])
    }
    fn replace(i: &mut Input, old: &[u8], new: &[u8]) {
        let b = i.document.original_bytes();
        let at = b.windows(old.len()).position(|w| w == old).unwrap();
        let mut out = b[..at].to_vec();
        out.extend_from_slice(new);
        out.extend_from_slice(&b[at + old.len()..]);
        i.document = Document::parse(out).unwrap();
    }
    #[test]
    fn exact_schema_keeps_orders_costs_and_absent_defaults() {
        let i = inputs();
        let c = catalog(&i).unwrap();
        assert_eq!(c.resources.len(), 7);
        assert_eq!(c.resource_list.len(), 7);
        assert_eq!(c.stockpiled_resource_max.raw, "99999");
        assert_eq!(c.stockpiled_resource_max.f32_bits, 99_999f32.to_bits());
        assert_eq!(c.stockpiled_resource_max.source.path, "Game/SVars.tgi");
        assert_eq!(
            c.resources
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            [
                "money",
                "ammo",
                "oil",
                "unit_limit_provided",
                "unit_limit_consumed",
                "building_limit_provided",
                "building_limit_consumed"
            ]
        );
        assert_eq!(
            c.resource_list
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            [
                "money",
                "oil",
                "ammo",
                "unit_limit_provided",
                "unit_limit_consumed",
                "building_limit_consumed",
                "building_limit_provided"
            ]
        );
        assert!(c.resource_list.iter().all(|r| r.default_value.is_none()));
        assert_eq!(
            c.find("ammo").unwrap().purchase_cost[0].value.f32_bits,
            2f32.to_bits()
        );
        assert_eq!(
            c.find("oil").unwrap().purchase_cost[0].value.f32_bits,
            3f32.to_bits()
        );
        assert_eq!(c.find("money").unwrap().category as u8, 0);
        assert_eq!(c.find("ammo").unwrap().category as u8, 1);
        assert_eq!(c.find("unit_limit_consumed").unwrap().category as u8, 2);
        assert_eq!(
            c.resources
                .iter()
                .filter(|r| r.limited_provider)
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            ["unit_limit_provided", "building_limit_provided"]
        );
        assert_eq!(
            c.find("building_limit_consumed")
                .unwrap()
                .limited_by
                .as_deref(),
            Some("building_limit_provided")
        );
        assert_eq!(c.limited_resource_max, 100);
        assert_eq!(
            c.start_location_production[0].value.f32_bits,
            100f32.to_bits()
        );
        assert!(
            c.upkeep_defaults
                .iter()
                .chain(&c.production_defaults)
                .all(|r| r.value.f32_bits == 0)
        );
        for row in &c.start_location_production {
            let span = row.value.source.span;
            assert!(
                std::str::from_utf8(&i[4].document.original_bytes()[span.start..span.end])
                    .unwrap()
                    .contains("= 100")
            );
        }
    }
    #[test]
    fn expressions_and_unknown_category_do_not_become_guessed_values() {
        let mut i = inputs();
        replace(&mut i[2], b"money = 2", b"money = 2+3");
        assert!(catalog(&i).is_err());
        let mut i = inputs();
        replace(&mut i[2], b"money = 2", b"money = -0");
        assert!(catalog(&i).is_err());
        let mut i = inputs();
        replace(&mut i[1], b"type = UPKEEP", b"type = MYSTERY");
        assert!(catalog(&i).is_err());
    }
    #[test]
    fn bad_provider_graph_and_duplicate_rows_fail() {
        let mut i = inputs();
        replace(
            &mut i[2],
            b"limited_by = unit_limit_provided",
            b"limited_by = money",
        );
        assert!(catalog(&i).is_err());
        let mut i = inputs();
        replace(&mut i[2], b"money = 2", b"money = 2\r\n money = 3");
        assert!(catalog(&i).is_err());
        let mut i = inputs();
        replace(&mut i[2], b"IDS = ammo", b"IDS = oil");
        assert!(catalog(&i).is_err());
    }
}
