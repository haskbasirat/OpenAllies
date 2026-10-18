//! The single mutable standalone WorldRules owner (511CE0/512420/511640).
//! MAP_HEADER rules are separate serialized inputs and cannot replace it.
use crate::{
    catalog::Catalog, combat_value_source_context::SourceFpContext, definitions::Bank,
    saved_action_settings,
};
use aa_mapworld::{
    WorldInventory,
    header::{Rule, WorldRules},
};
use aa_numeric::{Binary, Precision};
use aa_rules::{actors::ResolvedNode, native::TypedValue, resources::SourceIdentity};

pub struct Record {
    ids: String,
    kind: u32,
    default: u32,
    editable: bool,
    enum_ids: Vec<String>,
    source: aa_rules::resources::Provenance,
    path: Vec<usize>,
    locked: u8,
    value: u32,
}
impl Record {
    pub fn ids(&self) -> &str {
        &self.ids
    }
    pub fn kind(&self) -> u32 {
        self.kind
    }
    pub fn default_value(&self) -> u32 {
        self.default
    }
    pub fn always_editable(&self) -> bool {
        self.editable
    }
    pub fn locked(&self) -> u8 {
        self.locked
    }
    pub fn value(&self) -> u32 {
        self.value
    }
    pub fn source(&self) -> &aa_rules::resources::Provenance {
        &self.source
    }
    pub fn enum_values(&self) -> &[String] {
        &self.enum_ids
    }
    pub fn enum_ids(&self, value: u32) -> Result<&str, String> {
        if self.kind != 3 {
            return Err("WorldRule is not an enum".into());
        }
        self.enum_ids
            .get(value as usize)
            .map(String::as_str)
            .ok_or_else(|| {
                format!(
                    "WorldRule {} enum value {value} is outside its actual domain",
                    self.ids
                )
            })
    }
}

/// No Clone, Default, raw-record constructor, or mutable record projection.
/// The actual loaded instance must move from LoadWorld into the live battle.
pub struct Owner {
    records: Vec<Record>,
    mode: u32,
    game_type: Vec<u16>,
    changes: u32,
    rules_source: aa_mapworld::SourceIdentity,
    sources: Vec<SourceIdentity>,
    context: SourceFpContext,
}
/// Creator+20 owns a separate native WorldRules instance. The wrapper cannot
/// be passed as the standalone Owner to Current SVar publication accidentally.
pub struct CreatorRules {
    records: Vec<Record>,
    mode: u32,
    game_type: Vec<u16>,
    changes: u32,
    source: Option<aa_mapworld::SourceIdentity>,
    sources: Vec<SourceIdentity>,
    context: SourceFpContext,
}
fn assign_bool_record(records: &mut [Record], changes: &mut u32, name: &str, value: u8) -> bool {
    let Some(record) = records
        .iter_mut()
        .find(|r| r.ids.eq_ignore_ascii_case(name))
    else {
        return false;
    };
    if record.kind != 0 {
        return false;
    }
    record.value = (record.value & !255) | u32::from(value);
    *changes = changes.wrapping_add(1);
    true
}
impl CreatorRules {
    fn from_loaded(owner: Owner) -> Self {
        Self {
            records: owner.records,
            mode: owner.mode,
            game_type: owner.game_type,
            changes: owner.changes,
            source: Some(owner.rules_source),
            sources: owner.sources,
            context: owner.context,
        }
    }
    pub(crate) fn from_native_constructor(
        catalog: &Catalog<'_>,
        bank: &Bank,
    ) -> Result<Self, String> {
        let context = source_context(catalog, bank)?;
        crate::combat_value_source_context::bind_numeric(catalog, &context)?;
        //511D9B..511DB2 stores the FIRST GameType manager entry (+47C+24).
        let game_types = catalog
            .actors
            .native_data_registry("GameType")
            .ok_or("Creator GameType registry absent")?;
        let first = game_types
            .entries
            .first()
            .ok_or("Creator default GameType array is empty")?;
        catalog.actors.native_data_node(first).map_err(err)?;
        Ok(Self {
            records: descriptors(catalog, context.precision())?,
            mode: 0,
            game_type: first.ids.encode_utf16().collect(),
            changes: 0,
            source: None,
            sources: catalog.actors.sources.clone(),
            context,
        })
    }
    pub fn record(&self, name: &str) -> Result<&Record, String> {
        self.find_record(name)
            .ok_or_else(|| format!("Creator WorldRule {name} absent"))
    }
    ///511A60 compares the retained definition IDS; it does not re-Translate.
    pub fn find_record(&self, name: &str) -> Option<&Record> {
        self.records
            .iter()
            .find(|r| r.ids.eq_ignore_ascii_case(name))
    }
    ///511AF0: absent/wrong-type is a no-op; even an identical byte increments
    ///the actual change counter once. Callers own any earlier equality guard.
    pub(crate) fn assign_bool(&mut self, name: &str, value: u8) -> bool {
        assign_bool_record(&mut self.records, &mut self.changes, name, value)
    }
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }
    ///Current Creator+20/+24/+28 values; diagnostics borrow the same owner.
    pub fn mode(&self) -> u32 { self.mode }
    pub fn game_type(&self) -> &[u16] { &self.game_type }
    ///Native Rules+18 is retained even though the film rules writer omits it.
    pub fn changes(&self) -> u32 {
        self.changes
    }
    pub fn to_serialized(&self) -> WorldRules {
        WorldRules {
            version: 4,
            mode: self.mode,
            game_type: self.game_type.clone(),
            rules: self
                .records
                .iter()
                .map(|r| Rule {
                    key: r.ids.encode_utf16().collect(),
                    enabled: r.locked,
                    kind: r.kind,
                    value: r.value,
                })
                .collect(),
        }
    }
    pub fn validate_in_bank(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<(), String> {
        validate_values(catalog, bank, &self.records, &self.sources, &self.context)
    }
    pub(crate) fn copy_for_creator(&self) -> Self {
        Self {
            records: copy_records(&self.records),
            mode: self.mode,
            game_type: self.game_type.clone(),
            changes: self.changes,
            source: self.source.clone(),
            sources: self.sources.clone(),
            context: self.context.clone(),
        }
    }
    ///Immediate native pointer/scalar copies at5A26E0. The caller supplies
    ///the current nullable canonical GameType IDS; this performs no lookup or
    ///localization and does not replace the separate rule record vector.
    pub(crate) fn set_mode_from_current(&mut self, mode: u32) {
        self.mode = mode;
    }
    pub(crate) fn set_game_type_from_current(&mut self, ids: Vec<u16>) {
        self.game_type = ids;
    }
    pub(crate) fn set_changes_from_current(&mut self, changes: u32) {
        self.changes = changes;
    }
}
fn copy_records(records: &[Record]) -> Vec<Record> {
    records
        .iter()
        .map(|r| Record {
            ids: r.ids.clone(),
            kind: r.kind,
            default: r.default,
            editable: r.editable,
            enum_ids: r.enum_ids.clone(),
            source: r.source.clone(),
            path: r.path.clone(),
            locked: r.locked,
            value: r.value,
        })
        .collect()
}
fn validate_values(
    catalog: &Catalog<'_>,
    bank: &Bank,
    records: &[Record],
    sources: &[SourceIdentity],
    context: &SourceFpContext,
) -> Result<(), String> {
    let actual = source_context(catalog, bank)?;
    if sources != catalog.actors.sources
        || actual.precision() != context.precision()
        || actual.reference_startup() != context.reference_startup()
    {
        return Err("WorldRules actual source/precision/startup context differs".into());
    }
    let registry = catalog
        .actors
        .native_data_registry("WorldRule")
        .ok_or("WorldRule registry absent")?;
    if registry.entries.len() != records.len()
        || registry.entries.iter().zip(records).any(|(a, b)| {
            a.ids != b.ids || a.path != b.path || a.source != b.source || !a.reloads.is_empty()
        })
    {
        return Err("WorldRules canonical registration identity/order differs".into());
    }
    crate::combat_value_source_context::bind_numeric(catalog, context).map(|_| ())
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn text(node: &ResolvedNode<'_>, name: &str, p: Precision) -> Result<String, String> {
    // Generic registry IDS also permits the original parser's untyped ASCII
    // identifier assignment. Keep that existing bounded getter separate from
    // typed numeric/default fields; it does not invent an IDS declaration.
    let value = if name.eq_ignore_ascii_case("IDS") {
        node.plain_string_field(name).map_err(err)?
    } else {
        node.field(name, p).map_err(err)?
    };
    match value {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(format!("WorldRule source {name} String is absent")),
    }
}
fn c_string(units: &[u16]) -> Result<String, String> {
    let end = units.iter().position(|&v| v == 0).unwrap_or(units.len());
    String::from_utf16(&units[..end]).map_err(err)
}
fn source_context(catalog: &Catalog<'_>, bank: &Bank) -> Result<SourceFpContext, String> {
    catalog.require_source_owner()?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources {
        return Err("WorldRules Bank/source identities differ".into());
    }
    Ok(source.context().clone())
}
pub(crate) fn descriptors(catalog: &Catalog<'_>, p: Precision) -> Result<Vec<Record>, String> {
    let registry = catalog
        .actors
        .native_data_registry("WorldRule")
        .ok_or("WorldRule registry absent")?;
    let mut result = Vec::with_capacity(registry.entries.len());
    for entry in &registry.entries {
        if !entry.reloads.is_empty() {
            return Err(format!(
                "WorldRule {} reloaded descriptor semantics are unbound",
                entry.ids
            ));
        }
        let node = catalog.actors.native_data_node(entry).map_err(err)?;
        if text(&node, "IDS", p)? != entry.ids {
            return Err("WorldRule registered/source identity differs".into());
        }
        let ty = text(&node, "type", p)?;
        let default_text = text(&node, "default", p)?;
        let mut enum_ids = Vec::new();
        let (kind, default) = match ty.to_ascii_lowercase().as_str() {
            "bool" => (
                0,
                match default_text.to_ascii_lowercase().as_str() {
                    "true" => 1,
                    "false" => 0,
                    _ => {
                        return Err(
                            "WorldRule Bool default requires the native true/false scope".into(),
                        );
                    }
                },
            ),
            "int" => (1, default_text.trim().parse::<i32>().map_err(err)? as u32),
            // Current source defaults are integral. Do not silently substitute
            // Rust decimal parsing for the native CRT's general string parser.
            "fixed" => (
                2,
                Binary::from_i32(default_text.trim().parse::<i32>().map_err(
                    |_| "WorldRule Fixed default requires proven signed-integer literal scope",
                )?)
                .store_f32()
                .map_err(err)?,
            ),
            "enum" | "enum_specified" => {
                if ty.eq_ignore_ascii_case("enum") {
                    for child in node.children("EnumValue").map_err(err)? {
                        enum_ids.push(text(&child, "IDS", p)?)
                    }
                } else {
                    //4909DE..490A97: only DifficultyLevel is recognized by
                    // enum_specified; it copies GameData+164/+168 in order.
                    if !entry.ids.eq_ignore_ascii_case("DifficultyLevel") {
                        return Err(format!(
                            "WorldRule {} enum_specified producer is unbound",
                            entry.ids
                        ));
                    }
                    let difficulty = catalog
                        .actors
                        .native_data_registry("DifficultyLevel")
                        .ok_or("WorldRule DifficultyLevel registry absent")?;
                    for value in &difficulty.entries {
                        if !value.reloads.is_empty() {
                            return Err("DifficultyLevel reloaded descriptor is unbound".into());
                        }
                        if value
                            .path
                            .first()
                            .zip(entry.path.first())
                            .is_none_or(|(value, rule)| value >= rule)
                        {
                            return Err("DifficultyLevel descriptor was not yet registered at WorldRule LoadData".into());
                        }
                        let loaded = catalog.actors.native_data_node(value).map_err(err)?;
                        if text(&loaded, "IDS", p)? != value.ids {
                            return Err("DifficultyLevel identity differs".into());
                        }
                        enum_ids.push(value.ids.clone());
                    }
                }
                //473440 returns the first case-insensitive enum IDS match.
                let value = enum_ids
                    .iter()
                    .position(|v| v.eq_ignore_ascii_case(&default_text))
                    .ok_or_else(|| {
                        format!(
                            "WorldRule {} default is absent from its native enum domain",
                            entry.ids
                        )
                    })?;
                (3, u32::try_from(value).map_err(err)?)
            }
            _ => {
                return Err(format!(
                    "WorldRule {} source type {ty} is unbound",
                    entry.ids
                ));
            }
        };
        let editable = node
            .field("always_editable", p)
            .map_err(err)?
            .ok_or("WorldRule always_editable source absent")?
            .boolean()
            .map_err(err)?;
        result.push(Record {
            ids: entry.ids.clone(),
            source: entry.source.clone(),
            path: entry.path.clone(),
            kind,
            default,
            editable,
            enum_ids,
            locked: 0,
            value: default,
        });
    }
    Ok(result)
}
fn overlay(record: &mut Record, kind: u32, locked: u8, value: u32) {
    if kind == record.kind {
        record.locked = locked;
        record.value = if kind == 0 { value & 255 } else { value };
    }
}
fn reset_editable(records: &mut [Record], load_mode: u32) {
    if load_mode & 2 != 0 {
        for row in records {
            if row.editable {
                row.locked = 0;
                row.value = row.default;
            }
        }
    }
}
impl Owner {
    ///4FC801 loads RULES first.4FC814..4FC83D then copies the actual current
    ///MapResource+88 Rules value when+A8 bit1 is clear; otherwise4FC852 calls
    ///511640 on this same loaded vector. Publication is a later caller effect.
    pub(crate) fn load_for_resource(
        catalog: &Catalog<'_>,
        inventory: &WorldInventory,
        resource: &crate::map_resource_load::Owner,
    ) -> Result<Self, String> {
        let saved = saved_action_settings::rules(inventory)?;
        let sections: Vec<_> = inventory
            .opaque_initialization
            .iter()
            .filter(|s| s.name.eq_ignore_ascii_case("RULES"))
            .collect();
        let [section] = sections.as_slice() else {
            return Err("WorldRules requires one actual standalone RULES source".into());
        };
        let mut owner = Self::from_saved(catalog, &saved, section.source.clone(), 0)?;
        if resource.load_mode() & 2 != 0 {
            reset_editable(&mut owner.records, resource.load_mode());
        } else {
            let bank = catalog.definition_bank.try_borrow().map_err(err)?;
            resource.validate_creator(catalog, &bank)?;
            let creator = resource.creator()?;
            let current = creator.rules();
            current.validate_in_bank(catalog, &bank)?;
            if owner.sources != current.sources
                || owner.context.precision() != current.context.precision()
                || owner.context.reference_startup() != current.context.reference_startup()
            {
                return Err("GWorld and Creator Rules current source contexts differ".into());
            }
            owner.mode = current.mode;
            owner.game_type = current.game_type.clone();
            owner.records = copy_records(&current.records);
            owner.changes = current.changes;
        }
        Ok(owner)
    }
    pub fn load(catalog: &Catalog<'_>, inventory: &WorldInventory) -> Result<Self, String> {
        let saved = saved_action_settings::rules(inventory)?;
        let sections: Vec<_> = inventory
            .opaque_initialization
            .iter()
            .filter(|s| s.name.eq_ignore_ascii_case("RULES"))
            .collect();
        let [section] = sections.as_slice() else {
            return Err("WorldRules requires one actual standalone RULES source".into());
        };
        let load_mode = catalog
            .native_load_mode
            .ok_or("WorldRules actual load mode absent")?;
        if !matches!(load_mode, 1 | 2 | 4) {
            return Err("WorldRules actual load mode is unbound".into());
        }
        Self::from_saved(catalog, &saved, section.source.clone(), load_mode)
    }
    ///513910 invokes512420 on this Creator's embedded WorldRules. GWorld's
    ///later511640 editable reset is deliberately NOT applied to this instance.
    pub(crate) fn load_embedded_creator(
        catalog: &Catalog<'_>,
        map: &aa_mapworld::MapArchive,
    ) -> Result<CreatorRules, String> {
        let chunk = map.decode_chunk("MAP_HEADER", u64::MAX).map_err(err)?;
        let header = aa_mapworld::header::MapHeader::decode(&chunk).map_err(err)?;
        Self::from_saved(catalog, &header.creator.rules, header.source, 0)
            .map(CreatorRules::from_loaded)
    }
    pub(crate) fn load_header_rules(
        catalog: &Catalog<'_>,
        header: &aa_mapworld::header::MapHeader,
    ) -> Result<CreatorRules, String> {
        Self::from_saved(catalog, &header.rules, header.source.clone(), 0)
            .map(CreatorRules::from_loaded)
    }
    fn from_saved(
        catalog: &Catalog<'_>,
        saved: &WorldRules,
        source: aa_mapworld::SourceIdentity,
        reset_mode: u32,
    ) -> Result<Self, String> {
        let bank = catalog.definition_bank.try_borrow().map_err(err)?;
        let context = source_context(catalog, &bank)?;
        crate::combat_value_source_context::bind_numeric(catalog, &context)?;
        let mut records = descriptors(catalog, context.precision())?;
        saved_action_settings::game_type(catalog, saved)?;
        let registry = catalog.actors.native_data_registry("WorldRule").unwrap();
        for row in &saved.rules {
            let name = c_string(&row.key)?;
            let Some(found) = registry.find(&name) else {
                continue;
            };
            let ordinal = registry
                .entries
                .iter()
                .position(|d| d.path == found.path && d.source == found.source)
                .ok_or("WorldRule Find result is not its registered descriptor")?;
            let record = records
                .get_mut(ordinal)
                .ok_or("WorldRule registered ordinal absent")?;
            //512420 ignores unknown IDS and changed types; later accepted rows
            //overwrite the same native record, including its raw locked byte.
            overlay(record, row.kind, row.enabled, row.value);
        }
        reset_editable(&mut records, reset_mode);
        let game_type = if saved.game_type.is_empty() {
            Vec::new()
        } else {
            let registry = catalog
                .actors
                .native_data_registry("GameType")
                .ok_or("WorldRules GameType registry absent")?;
            registry
                .find(&c_string(&saved.game_type)?)
                .ok_or("WorldRules GameType absent")?
                .ids
                .encode_utf16()
                .collect()
        };
        Ok(Self {
            records,
            mode: saved.mode,
            game_type,
            changes: 0,
            rules_source: source,
            sources: catalog.actors.sources.clone(),
            context,
        })
    }
    fn copy_value(&self) -> Self {
        //5145E0/41D130 copy the current record vector independently while
        //retaining the same immutable canonical definition identities.
        Self {
            records: copy_records(&self.records),
            mode: self.mode,
            game_type: self.game_type.clone(),
            changes: self.changes,
            rules_source: self.rules_source.clone(),
            sources: self.sources.clone(),
            context: self.context.clone(),
        }
    }
    pub fn validate_in_bank(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<(), String> {
        validate_values(catalog, bank, &self.records, &self.sources, &self.context)
    }
    pub fn source_precision(&self) -> Precision {
        self.context.precision()
    }
    pub fn changes(&self) -> u32 {
        self.changes
    }
    pub fn mode(&self) -> u32 {
        self.mode
    }
    pub fn game_type(&self) -> &[u16] {
        &self.game_type
    }
    ///Native41D130 value-copies only Rules+C, after5A26E0 has separately
    ///stored mode/GameType. Both actual owners retain their independent arrays.
    pub fn copy_records_into(
        &self,
        target: &mut CreatorRules,
        catalog: &Catalog<'_>,
        bank: &Bank,
    ) -> Result<(), String> {
        self.validate_in_bank(catalog, bank)?;
        target.validate_in_bank(catalog, bank)?;
        if self.sources != target.sources
            || self.context.precision() != target.context.precision()
            || self.context.reference_startup() != target.context.reference_startup()
        {
            return Err("film Rules current source/precision/startup contexts differ".into());
        }
        target.records = copy_records(&self.records);
        Ok(())
    }
    pub fn rules_source(&self) -> &aa_mapworld::SourceIdentity {
        &self.rules_source
    }
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }
    ///511A60 compares the current descriptor IDS, without manager translation.
    pub fn record(&self, name: &str) -> Result<&Record, String> {
        self.records
            .iter()
            .find(|v| v.ids.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("WorldRule {name} absent from actual owner"))
    }
    pub(crate) fn assign_word(&mut self, name: &str, kind: u32, value: u32) -> Result<(), String> {
        let record = self
            .records
            .iter_mut()
            .find(|v| v.ids.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("WorldRule {name} absent from actual owner"))?;
        if record.kind != kind {
            return Err("WorldRule assignment kind differs from actual descriptor".into());
        }
        record.value = if kind == 0 { value & 255 } else { value };
        self.changes = self.changes.wrapping_add(1);
        Ok(())
    }
    ///A serialization projection of current records, never another mutable owner.
    pub fn to_serialized(&self) -> WorldRules {
        WorldRules {
            version: 4,
            mode: self.mode,
            game_type: self.game_type.clone(),
            rules: self
                .records
                .iter()
                .map(|v| Rule {
                    key: v.ids.encode_utf16().collect(),
                    enabled: v.locked,
                    kind: v.kind,
                    value: v.value,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "load_world_rules_tests.rs"]
mod tests;

