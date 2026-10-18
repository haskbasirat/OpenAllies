//! ActorCommandData's bounded48CF0A..48CF97 control-field producer.
//!
//! The existing canonical mode2 manager and inherited typed nodes supply the
//! seven getters. Earlier localization, icons, cursors and VFX loading remain
//! separate required services; this is not the whole48CD00 loader or a UI.
use crate::catalog::Catalog;
use aa_numeric::Precision;
use aa_rules::{
    actors::ResolvedNode,
    native::{TypedValue, actor_command::ActorCommandDefinition},
    resources::Provenance,
};

pub type Result<T> = std::result::Result<T, String>;
pub const BOOLEAN_FIELDS: [&str; 6] = [
    "non_group_command",
    "apply_group_offset",
    "aggressive_command",
    "can_hot_command_actor",
    "can_hot_command_location",
    "high_importance",
];

/// Fallible reached node getters. Missing/invalid fields must remain errors.
pub trait Fields {
    fn boolean(&mut self, name: &'static str) -> Result<bool>;
    fn integer(&mut self, name: &'static str) -> Result<i32>;
}

/// Only this fragment's stores. None means the fragment has not written that
/// location; it makes no claim about the object's earlier constructor bytes.
/// Stores remain visible if a later getter fails, as in48CF0A..48CF97.
#[derive(Debug, Default)]
pub struct LoadState {
    bytes_3c_41: [Option<bool>; 6],
    position_44: Option<i32>,
}
impl LoadState {
    pub fn written_bytes(&self) -> &[Option<bool>; 6] {
        &self.bytes_3c_41
    }
    pub fn written_position(&self) -> Option<i32> {
        self.position_44
    }
    pub fn completed(&self) -> Result<Controls> {
        let mut bytes = [false; 6];
        for (i, byte) in self.bytes_3c_41.iter().enumerate() {
            bytes[i] = byte.ok_or("ActorCommand control fragment is incomplete")?;
        }
        Ok(Controls {
            bytes_3c_41: bytes,
            position_44: self
                .position_44
                .ok_or("ActorCommand preferred_position store is incomplete")?,
        })
    }
}

/// Immutable completed scalar projection. It does not represent an installed
/// command/UI object, mutable gameplay arena, or a completed presentation load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Controls {
    bytes_3c_41: [bool; 6],
    position_44: i32,
}
impl Controls {
    pub fn bytes_3c_41(self) -> [bool; 6] {
        self.bytes_3c_41
    }
    pub fn non_group_command(self) -> bool {
        self.bytes_3c_41[0]
    }
    pub fn apply_group_offset(self) -> bool {
        self.bytes_3c_41[1]
    }
    pub fn aggressive_command(self) -> bool {
        self.bytes_3c_41[2]
    }
    pub fn can_hot_command_actor(self) -> bool {
        self.bytes_3c_41[3]
    }
    pub fn can_hot_command_location(self) -> bool {
        self.bytes_3c_41[4]
    }
    pub fn high_importance(self) -> bool {
        self.bytes_3c_41[5]
    }
    pub fn preferred_position(self) -> i32 {
        self.position_44
    }
}

/// Exact native getter/store order, including the496F30(flags,0) branch after
/// aggressive_command. That predicate tests bits16..23, not behavior bit0.
pub fn load_fields(
    state: &mut LoadState,
    fields: &mut impl Fields,
    mind_type: u32,
    behavior_flags: u32,
) -> Result<()> {
    state.bytes_3c_41[0] = Some(fields.boolean(BOOLEAN_FIELDS[0])?);
    state.bytes_3c_41[1] = Some(fields.boolean(BOOLEAN_FIELDS[1])?);
    state.bytes_3c_41[2] = Some(fields.boolean(BOOLEAN_FIELDS[2])?);
    if behavior_flags & 0x00ff_0000 != 0 || matches!(mind_type, 11 | 12 | 34) {
        state.bytes_3c_41[0] = Some(true);
    }
    state.bytes_3c_41[3] = Some(fields.boolean(BOOLEAN_FIELDS[3])?);
    state.bytes_3c_41[4] = Some(fields.boolean(BOOLEAN_FIELDS[4])?);
    state.bytes_3c_41[5] = Some(fields.boolean(BOOLEAN_FIELDS[5])?);
    state.position_44 = Some(fields.integer("preferred_position")?);
    Ok(())
}

struct NodeFields<'a> {
    node: ResolvedNode<'a>,
    precision: Precision,
    inputs: Vec<TypedValue>,
}
impl NodeFields<'_> {
    fn value(&self, name: &str) -> Result<TypedValue> {
        self.node
            .field(name, self.precision)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("ActorCommand node does not contain {name}"))
    }
}
impl Fields for NodeFields<'_> {
    fn boolean(&mut self, name: &'static str) -> Result<bool> {
        let value = self.value(name)?;
        let boolean = value.boolean().map_err(|e| e.to_string())?;
        self.inputs.push(value);
        Ok(boolean)
    }
    fn integer(&mut self, name: &'static str) -> Result<i32> {
        let value = self.value(name)?;
        let integer = value.integer().map_err(|e| e.to_string())?;
        self.inputs.push(value);
        Ok(integer)
    }
}

/// Verified source projection in the same Catalog/Bank source domain. It is
/// safe to share immutable scalar inputs; no mutable native manager is copied.
#[derive(Debug)]
pub struct Source {
    definition: ActorCommandDefinition,
    definition_source: Provenance,
    node_sources: Vec<Provenance>,
    precision: Precision,
    inputs: Vec<TypedValue>,
    controls: Controls,
}
impl Source {
    pub fn definition(&self) -> ActorCommandDefinition {
        self.definition
    }
    pub fn definition_source(&self) -> &Provenance {
        &self.definition_source
    }
    pub fn node_sources(&self) -> &[Provenance] {
        &self.node_sources
    }
    pub fn source_precision(&self) -> Precision {
        self.precision
    }
    /// Seven typed input values in native getter order, before forced+3C.
    pub fn inputs(&self) -> &[TypedValue] {
        &self.inputs
    }
    pub fn controls(&self) -> Controls {
        self.controls
    }
    pub fn bind(catalog: &Catalog<'_>, canonical_ids: &str) -> Result<Self> {
        catalog.require_source_owner()?;
        let definition = aa_rules::native::actor_command::predefined_actor_command(canonical_ids)
            .map_err(|e| e.to_string())?;
        let precision = {
            let bank = catalog
                .definition_bank
                .try_borrow()
                .map_err(|_| "nested ActorCommand UI source Bank")?;
            let state = bank.combat_source()?;
            if state.source_identities() != catalog.actors.sources.as_slice() {
                return Err("ActorCommand UI source FP owner differs".into());
            }
            state.source_precision()
        };
        let registry = catalog
            .actors
            .native_data_registry("ActorCommand")
            .ok_or("actual ActorCommand manager is absent")?;
        if registry.mode != Some(2) {
            return Err("ActorCommand manager mode differs from native47F7C0".into());
        }
        let entry = catalog
            .actors
            .native_data_find("ActorCommand", definition.id)
            .ok_or("ActorCommand authored UI root is absent; no constructor fallback")?;
        if !entry.ids.eq_ignore_ascii_case(definition.id) {
            return Err("canonical ActorCommand identity is shadowed by translation".into());
        }
        let node = catalog
            .actors
            .native_data_node(entry)
            .map_err(|e| e.to_string())?;
        match node.field("IDS", precision).map_err(|e| e.to_string())? {
            Some(TypedValue::String { value, .. }) if value.eq_ignore_ascii_case(definition.id) => {
            }
            _ => return Err("ActorCommand node IDS differs from canonical registration".into()),
        }
        let node_sources = node.sources();
        if node_sources.first() != Some(&entry.source) {
            return Err("ActorCommand registered source/node provenance differs".into());
        }
        let mut fields = NodeFields {
            node,
            precision,
            inputs: Vec::new(),
        };
        let mut state = LoadState::default();
        load_fields(
            &mut state,
            &mut fields,
            definition.mind_type,
            definition.behavior_flags,
        )?;
        Ok(Self {
            definition,
            definition_source: entry.source.clone(),
            node_sources,
            precision,
            inputs: fields.inputs,
            controls: state.completed()?,
        })
    }
}
