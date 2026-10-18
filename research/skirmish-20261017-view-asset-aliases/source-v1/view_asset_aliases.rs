//!Same application-owned current CString fields read by VWorld410CB0.
//!These22 disjoint aliases are created by4B3C10 from actual rootAVars files;
//!401BD0 has125 registrations and contains NONE of these names. This selected
//!current owner is not a fullAVars manager or a successful application token.
use crate::{
    catalog::Catalog, combat_value_source_context::SourceFpContext, definitions::Bank,
    load_view_variables::NativeOverrides,
};
use aa_rules::{
    actors::native_fresh_field_order,
    native::index::native_enumerate,
    resources::{Provenance, SourceIdentity},
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};
use std::{collections::BTreeMap, sync::Arc};
type Result<T> = std::result::Result<T, String>;
struct Cell {
    current: Arc<[u16]>,
    assignments: Vec<Provenance>,
    raw_assignments: Vec<String>,
}
///Owned counted CString copy. Reassigning the current variable leaves an
///already obtained temporary readable, exactly as410CB0/CString references.
///This carries text only and certifies no resource lookup or renderer state.
pub struct Alias {
    text: Arc<[u16]>,
}
impl Alias {
    pub fn units(&self) -> &[u16] {
        &self.text
    }
}
pub struct State {
    cells: BTreeMap<String, Cell>,
    sources: Vec<SourceIdentity>,
    context: SourceFpContext,
    files: Vec<(String, String)>,
    complete: bool,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewAssetAliases")
            .field("complete", &self.complete)
            .field("cells", &self.cells.len())
            .finish()
    }
}
///The same selected current-cell allocation and completed source stores remain
///owned on a later source failure. An incomplete state cannot supply aliases.
pub struct Failure {
    pub error: String,
    pub state: Option<State>,
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewAssetAliasesFailure")
            .field("error", &self.error)
            .field("state", &self.state.is_some())
            .finish()
    }
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn literal(raw: &str) -> Result<Vec<u16>> {
    if !raw.is_ascii() || raw.contains(['\0', '\\', '&']) {
        return Err("VWorld alias source requires proved plain ASCII CString literal".into());
    }
    let text = if let Some(tail) = raw.strip_prefix('"') {
        let end = tail.find('"').ok_or("VWorld alias quote unclosed")?;
        if !tail[end + 1..].trim().is_empty() {
            return Err("VWorld alias source expression is unbound".into());
        }
        &tail[..end]
    } else {
        if raw.contains('"') || raw.chars().any(char::is_whitespace) {
            return Err("VWorld alias unquoted expression is unbound".into());
        }
        raw
    };
    Ok(text.encode_utf16().collect())
}
fn context(catalog: &Catalog<'_>, bank: &Bank) -> Result<SourceFpContext> {
    catalog.require_source_owner()?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources {
        return Err("VWorld alias actual Bank/source differs".into());
    }
    Ok(source.context().clone())
}
impl State {
    pub fn from_fresh_sources(
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> Result<Self> {
        Self::from_fresh_sources_retained(catalog, files, overrides).map_err(|f| f.error)
    }
    pub fn from_fresh_sources_retained(
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> std::result::Result<Self, Failure> {
        let initial = (|| -> Result<SourceFpContext> {
            if !matches!(overrides, NativeOverrides::ControlledNone) {
                return Err("VWorld alias current profile/console owner is unbound".into());
            }
            let bank = catalog.definition_bank.try_borrow().map_err(error)?;
            context(catalog, &bank)
        })();
        let source_context = match initial {
            Ok(v) => v,
            Err(error) => return Err(Failure { error, state: None }),
        };
        let mut state = Self {
            cells: BTreeMap::new(),
            sources: catalog.actors.sources.clone(),
            context: source_context,
            files: Vec::new(),
            complete: false,
        };
        if let Err(error) = state.apply_sources(catalog, files) {
            return Err(Failure {
                error,
                state: Some(state),
            });
        }
        state.complete = true;
        Ok(state)
    }
    fn apply_sources(&mut self, catalog: &Catalog<'_>, files: &mut V14Sources) -> Result<()> {
        let settings = catalog
            .settings
            .as_ref()
            .ok_or("VWorld alias public Settings owner absent")?;
        let paths = files
            .catalog()
            .into_iter()
            .map(|e| e.logical_path)
            .collect::<Vec<_>>();
        for path in native_enumerate("AVars*.tgi", &paths) {
            let source = files.read(&path).map_err(error)?;
            if aa_rules::sources::sha256(&source.bytes) != source.sha256 {
                return Err("VWorld alias source hash differs".into());
            }
            let doc = Document::parse_native(source.bytes, Limits::default()).map_err(error)?;
            self.files
                .push((source.logical_path.clone(), source.sha256.clone()));
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|n| n.eq_ignore_ascii_case("Vars"))
                {
                    continue;
                }
                if node.header.tokens.len() != 1 {
                    return Err(
                        "VWorld alias inherited/prefilled Vars field table is unbound".into(),
                    );
                }
                let properties = node
                    .items
                    .iter()
                    .filter_map(|v| {
                        if let Item::Property(p) = v {
                            Some(p)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let names = properties
                    .iter()
                    .map(|p| {
                        doc.property_name(p)
                            .ok_or_else(|| "VWorld alias field name absent".to_string())
                    })
                    .collect::<Result<Vec<_>>>()?;
                for i in native_fresh_field_order(&names).map_err(error)? {
                    let p = properties[i];
                    let name = names[i];
                    if !crate::view_init_assets::alias_keys()
                        .any(|key| key.eq_ignore_ascii_case(name))
                    {
                        continue;
                    }
                    if p.left.len() != 2
                        || !doc
                            .text(p.left[0].span)
                            .is_some_and(|v| v.eq_ignore_ascii_case("string"))
                    {
                        return Err(format!(
                            "VWorld alias {name} requires actual native String field"
                        ));
                    }
                    let raw = doc
                        .property_value(p)
                        .ok_or("VWorld alias current source assignment absent")?;
                    let provenance = Provenance {
                        path: source.logical_path.clone(),
                        sha256: source.sha256.clone(),
                        span: p.span,
                    };
                    let public = settings
                        .app_variables
                        .get(&name.to_ascii_lowercase())
                        .ok_or("VWorld alias public source row absent")?;
                    let matches = public
                        .iter()
                        .filter(|r| r.source == provenance)
                        .collect::<Vec<_>>();
                    let [row] = matches.as_slice() else {
                        return Err("VWorld alias source/span ambiguity".into());
                    };
                    if row.raw != raw
                        || !row
                            .field_type
                            .as_deref()
                            .is_some_and(|v| v.eq_ignore_ascii_case("string"))
                    {
                        return Err(
                            "VWorld alias public projection differs from actual source".into()
                        );
                    }
                    let value: Arc<[u16]> = literal(raw)?.into();
                    let cell = self
                        .cells
                        .entry(name.to_ascii_lowercase())
                        .or_insert_with(|| Cell {
                            current: Arc::from([]),
                            assignments: Vec::new(),
                            raw_assignments: Vec::new(),
                        });
                    cell.current = value;
                    cell.assignments.push(provenance);
                    cell.raw_assignments.push(raw.to_owned());
                }
            }
        }
        for key in crate::view_init_assets::alias_keys() {
            let cell = self
                .cells
                .get(&key.to_ascii_lowercase())
                .ok_or_else(|| format!("VWorld current alias {key} is absent"))?;
            if settings
                .app_variables
                .get(&key.to_ascii_lowercase())
                .map(Vec::len)
                != Some(cell.assignments.len())
            {
                return Err("VWorld alias has unmatched/foreign public source rows".into());
            }
        }
        Ok(())
    }
    pub fn validate_in_bank(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<()> {
        if !self.complete {
            return Err("VWorld alias source assignment pass is incomplete".into());
        }
        let actual = context(catalog, bank)?;
        if self.sources != catalog.actors.sources
            || actual.precision() != self.context.precision()
            || actual.reference_startup() != self.context.reference_startup()
        {
            return Err("VWorld alias same source Bank/context differs".into());
        }
        let settings = catalog
            .settings
            .as_ref()
            .ok_or("VWorld alias Settings owner absent")?;
        for (name, cell) in &self.cells {
            let rows = settings
                .app_variables
                .get(name)
                .ok_or("VWorld alias current source projection absent")?;
            if rows.len() != cell.assignments.len()
                || cell.raw_assignments.len() != cell.assignments.len()
            {
                return Err("VWorld alias current source projection count differs".into());
            }
            for (provenance, raw) in cell.assignments.iter().zip(&cell.raw_assignments) {
                let found = rows
                    .iter()
                    .filter(|r| r.source == *provenance)
                    .collect::<Vec<_>>();
                let [row] = found.as_slice() else {
                    return Err("VWorld alias current source span differs".into());
                };
                if row.raw != *raw
                    || !row
                        .field_type
                        .as_deref()
                        .is_some_and(|v| v.eq_ignore_ascii_case("string"))
                {
                    return Err("VWorld alias current source projection differs".into());
                }
            }
        }
        Ok(())
    }
    pub fn alias_410cb0(&self, catalog: &Catalog<'_>, bank: &Bank, key: &str) -> Result<Alias> {
        self.validate_in_bank(catalog, bank)?;
        let cell = self
            .cells
            .get(&key.to_ascii_lowercase())
            .ok_or_else(|| format!("410CB0 VWorld current alias {key} is absent"))?;
        Ok(Alias {
            text: Arc::clone(&cell.current),
        })
    }
    ///412070 on these actual source-created type4/flags0 current cells.
    pub fn assign(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        key: &str,
        value: &[u16],
    ) -> Result<()> {
        self.validate_in_bank(catalog, bank)?;
        let cell = self
            .cells
            .get_mut(&key.to_ascii_lowercase())
            .ok_or("VWorld alias setter current field absent")?;
        cell.current = Arc::from(value);
        Ok(())
    }
    ///Completed current-cell source assignments, available even on retained failure.
    pub fn applied_source_assignments(&self, key: &str) -> &[Provenance] {
        self.cells
            .get(&key.to_ascii_lowercase())
            .map(|c| c.assignments.as_slice())
            .unwrap_or(&[])
    }
    pub fn source_files(&self) -> &[(String, String)] {
        &self.files
    }
}

#[cfg(test)]
#[path = "view_asset_aliases_tests.rs"]
mod tests;
