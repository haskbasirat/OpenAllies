//! `DataNode` over the shared aa-rules inherited-node resolver (the native
//! data load: index order, parse-time template binding, list templates).
//! Getter semantics follow the native contract documented in `data.rs`;
//! field and child names are ASCII case-insensitive. A localized field is
//! returned as its stored key/literal text: the string-table lookup of
//! `5B5620` is not modelled (the SAI loaders only store the name).
use super::data::f32_of;
use super::data::{DataNode, NodeId};
use super::ego::{EgoServices, ThingSource};
use super::profile::{DataManagers, Manager};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use aa_rules::actors::{ActorCatalog, ResolvedNode};
use aa_rules::native::{ScalarType, TypedValue};

pub struct RulesData<'a> {
    pub catalog: &'a ActorCatalog,
    pub nodes: Vec<ResolvedNode<'a>>,
    /// Child handles are stable per (parent, lower-case name, index), as the
    /// native child node objects are.
    children: std::collections::BTreeMap<(NodeId, String, i32), NodeId>,
    /// Precision used by aa-rules for scalar literals (percent forms only).
    pub precision: Precision,
}

fn rules(e: aa_rules::native::Error) -> Error {
    Error::World(format!("aa-rules: {e}"))
}
fn missing(name: &str) -> Error {
    Error::World(format!("Node doesn't contain field. ({name})"))
}

impl<'a> RulesData<'a> {
    pub fn new(catalog: &'a ActorCatalog, precision: Precision) -> Self {
        RulesData {
            catalog,
            nodes: Vec::new(),
            children: Default::default(),
            precision,
        }
    }
    /// Registers a resolved root node and returns its handle.
    pub fn root(&mut self, node: ResolvedNode<'a>) -> NodeId {
        self.nodes.push(node);
        (self.nodes.len() - 1) as NodeId
    }
    fn node(&self, id: NodeId) -> Result<&ResolvedNode<'a>> {
        self.nodes
            .get(id as usize)
            .ok_or_else(|| Error::World(format!("unknown data node {id}")))
    }
    fn value(&self, node: NodeId, name: &str) -> Result<Option<TypedValue>> {
        self.node(node)?.field(name, self.precision).map_err(rules)
    }
    fn string_value(&self, node: NodeId, name: &str) -> Result<Option<String>> {
        let n = self.node(node)?;
        match n.field(name, self.precision) {
            Ok(Some(TypedValue::String { value, .. })) => Ok(Some(value)),
            Ok(Some(other)) => Err(Error::World(format!(
                "field {name} is not a string: {other:?}"
            ))),
            Ok(None) => Ok(None),
            Err(_) => match n.plain_string_field(name).map_err(rules)? {
                Some(TypedValue::String { value, .. }) => Ok(Some(value)),
                Some(other) => Err(Error::World(format!(
                    "field {name} is not a string: {other:?}"
                ))),
                None => Ok(None),
            },
        }
    }
}

impl DataNode for RulesData<'_> {
    fn fixed(&mut self, node: NodeId, name: &str) -> Result<Binary> {
        match self.value(node, name)? {
            Some(TypedValue::Scalar(s)) if s.field_type == ScalarType::Fixed => {
                crate::x87::n(s.bits)
            }
            Some(TypedValue::Integer { value, .. }) => Ok(Binary::from_i32(value)),
            Some(other) => Err(Error::World(format!(
                "native GetFixed on unsupported field {name}: {other:?}"
            ))),
            None => Err(missing(name)),
        }
    }
    fn int(&mut self, node: NodeId, name: &str) -> Result<i32> {
        match self.value(node, name)? {
            Some(v) => v.integer().map_err(rules),
            None => Err(missing(name)),
        }
    }
    fn boolean(&mut self, node: NodeId, name: &str) -> Result<bool> {
        match self.value(node, name)? {
            Some(v) => v.boolean().map_err(rules),
            None => Err(missing(name)),
        }
    }
    fn string(&mut self, node: NodeId, name: &str) -> Result<String> {
        self.string_value(node, name)?.ok_or_else(|| missing(name))
    }
    fn localized(&mut self, node: NodeId, name: &str) -> Result<String> {
        match self.node(node)?.localized(name).map_err(rules)? {
            Some(text) => Ok(text.text),
            None => Err(missing(name)),
        }
    }
    fn count(&mut self, node: NodeId, name: &str) -> Result<i32> {
        Ok(self.node(node)?.children(name).map_err(rules)?.len() as i32)
    }
    fn item(&mut self, node: NodeId, name: &str, index: i32, _count: i32) -> Result<NodeId> {
        let key = (node, name.to_ascii_lowercase(), index);
        if let Some(&id) = self.children.get(&key) {
            return Ok(id);
        }
        let mut children = self.node(node)?.children(name).map_err(rules)?;
        if index < 0 || index as usize >= children.len() {
            return Err(Error::World(format!(
                "{name}[{index}] outside the child list"
            )));
        }
        let child = children.swap_remove(index as usize);
        self.nodes.push(child);
        let id = (self.nodes.len() - 1) as NodeId;
        self.children.insert(key, id);
        Ok(id)
    }
    fn section(&mut self, node: NodeId, name: &str, required: bool) -> Result<Option<NodeId>> {
        let key = (node, name.to_ascii_lowercase(), -1);
        if let Some(&id) = self.children.get(&key) {
            return Ok(Some(id));
        }
        match self.node(node)?.child(name).map_err(rules)? {
            Some(child) => {
                self.nodes.push(child);
                let id = (self.nodes.len() - 1) as NodeId;
                self.children.insert(key, id);
                Ok(Some(id))
            }
            None if required => Err(Error::World(format!("SubNode not found. ({name})"))),
            None => Ok(None),
        }
    }
    fn data_error(&mut self, format: &str, args: &[String]) -> Error {
        Error::World(format!("data error: {format} {args:?}"))
    }
}

/// Opaque handles for definitions whose native pointers the ego loaders
/// store or hash: `base + 0x40 * registry slot` (never 0, distinct per
/// registry). The native values are heap addresses; only identity matters.
pub const THING_HANDLES: u32 = 0x3000_0000;
pub const PROPERTY_HANDLES: u32 = 0x3400_0000;
pub const ABILITY_HANDLES: u32 = 0x3800_0000;
pub const FORMATION_HANDLES: u32 = 0x3a00_0000;
pub const RESOURCE_HANDLES: u32 = 0x3c00_0000;
const HANDLE_STRIDE: u32 = 0x40;

/// Registry kinds of the data managers. Nations, factions and egos use the
/// authored root order of `ActorCatalog::load_v14` with the index as handle;
/// the generic managers use the native-order registry (`48D500`, translations
/// first) of aa-rules.
fn kind(manager: Manager) -> Result<&'static str> {
    match manager {
        Manager::Nations => Ok("Nation"),
        Manager::Factions => Ok("Faction"),
        Manager::Egos => Ok("Ego"),
        Manager::Abilities => Ok("AbilityDefinition"),
        Manager::Formations => Ok("Formation"),
        Manager::Resources => Ok("Resource"),
        Manager::Other(offset) => Err(Error::World(format!(
            "data manager at root+{offset:#x} is not mapped"
        ))),
    }
}
fn handle_base(manager: Manager) -> Option<u32> {
    match manager {
        Manager::Abilities => Some(ABILITY_HANDLES),
        Manager::Formations => Some(FORMATION_HANDLES),
        Manager::Resources => Some(RESOURCE_HANDLES),
        _ => None,
    }
}

impl<'a> RulesData<'a> {
    fn registry(
        &self,
        manager: Manager,
    ) -> Result<&'a aa_rules::actors::native_load::NativeDataRegistry> {
        let tag = kind(manager)?;
        self.catalog.native_data_registry(tag).ok_or_else(|| {
            Error::World(format!(
                "no native-order {tag} registry (catalog not loaded natively)"
            ))
        })
    }
}

impl DataManagers for RulesData<'_> {
    /// `48D500`.
    fn find(&mut self, manager: Manager, ids: &str) -> Result<Option<(u32, i32)>> {
        if let Some(base) = handle_base(manager) {
            let registry = self.registry(manager)?;
            let Some(entry) = registry.find(ids) else {
                return Ok(None);
            };
            let slot = registry
                .entries
                .iter()
                .position(|e| std::ptr::eq(e, entry))
                .expect("entry of this registry");
            return Ok(Some((base + HANDLE_STRIDE * slot as u32, slot as i32)));
        }
        let kind = kind(manager)?;
        let roots = self.catalog.resolved_definitions(kind).map_err(rules)?;
        for (index, root) in roots.iter().enumerate() {
            if let Some(TypedValue::String { value, .. }) =
                root.field("IDS", self.precision).map_err(rules)?
                && value.eq_ignore_ascii_case(ids)
            {
                return Ok(Some((index as u32, index as i32)));
            }
        }
        Ok(None)
    }
}

impl EgoServices for RulesData<'_> {
    /// `4BDBA0(ids, 0x2B)`: the Thing registry (`427BD0`, translations
    /// applied) and the actor check of the Thing virtual +38 (native kind 2,
    /// as aa-metagame models it). The Thing DataManager constructor `4BD850`
    /// publishes itself in `[B39F58]`, the same manager as `[[B36530]+0C]`.
    fn thing(&mut self, _source: ThingSource, ids: &str) -> Result<Option<u32>> {
        let Some(def) = self.catalog.find(ids) else {
            return Ok(None);
        };
        let slot = self
            .catalog
            .actors
            .iter()
            .position(|a| std::ptr::eq(a, def))
            .expect("catalog actor");
        let category = self.catalog.native_category(ids).map_err(rules)?;
        Ok((category.native_kind == 2).then(|| THING_HANDLES + HANDLE_STRIDE * slot as u32))
    }
    /// `463FA0`: Property definitions in authored root order.
    fn property(&mut self, ids: &str) -> Result<Option<u32>> {
        let roots = self
            .catalog
            .resolved_definitions("Property")
            .map_err(rules)?;
        for (index, root) in roots.iter().enumerate() {
            if let Some(TypedValue::String { value, .. }) =
                root.field("IDS", self.precision).map_err(rules)?
                && value.eq_ignore_ascii_case(ids)
            {
                return Ok(Some(PROPERTY_HANDLES + HANDLE_STRIDE * index as u32));
            }
        }
        Ok(None)
    }
    fn registry_count(&mut self, manager: Manager) -> Result<i32> {
        Ok(self.registry(manager)?.entries.len() as i32)
    }
    /// AbilityDefinition LoadData `4BF390` stores `area` (fixed) at +38.
    fn ability_area(&mut self, ability: u32) -> Result<u32> {
        let slot = ability
            .checked_sub(ABILITY_HANDLES)
            .filter(|v| v % HANDLE_STRIDE == 0)
            .map(|v| (v / HANDLE_STRIDE) as usize)
            .ok_or_else(|| Error::World(format!("{ability:#x} is not an ability handle")))?;
        let registry = self.registry(Manager::Abilities)?;
        let entry = registry
            .entries
            .get(slot)
            .ok_or_else(|| Error::World(format!("ability slot {slot} outside the registry")))?;
        let node = self.catalog.native_data_node(entry).map_err(rules)?;
        match node.field("area", self.precision).map_err(rules)? {
            Some(TypedValue::Scalar(s)) if s.field_type == ScalarType::Fixed => {
                f32_of(&crate::x87::n(s.bits)?)
            }
            Some(TypedValue::Integer { value, .. }) => f32_of(&Binary::from_i32(value)),
            other => Err(Error::World(format!(
                "AbilityDefinition {} area is not a fixed value: {other:?}",
                entry.ids
            ))),
        }
    }
}
