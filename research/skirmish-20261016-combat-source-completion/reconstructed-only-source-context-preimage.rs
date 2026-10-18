//! Explicit source-phase CV settings and diagnostic UVar ownership.
//!
//! This module binds inputs, never completed ActorData+1E4/+30C/+36C. A supplied
//! FP context or FreshNoProfile scope is NOT an observed native startup receipt.
use crate::{catalog::Catalog, settings::Declaration};
use aa_body::F32;
use aa_numeric::{
    Precision,
    crt_pow::{CrtEnvironment, PowPath, select_path},
};
use aa_rules::{
    resources::{Provenance, SourceIdentity},
    sources::{ResolvedFile, V14Sources},
    tgi::{Document, Item, Limits},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub type Result<T> = std::result::Result<T, String>;
fn err(value: impl std::fmt::Display) -> String {
    value.to_string()
}

/// No Default: source loading precedes simulation/restore and owns its FP state.
#[derive(Clone, Debug)]
pub struct SourceFpContext {
    precision: Precision,
    environment: CrtEnvironment,
    status: u16,
    supplied_diagnostic_label: String,
}
impl SourceFpContext {
    /// Caller-supplied diagnostic assumption only. Actual original startup
    /// capture/binding is a separate requirement; labels cannot certify it.
    pub fn supplied_diagnostic(
        x87_control: u16,
        x87_status: u16,
        mxcsr: u32,
        sse2_available: bool,
        label: &str,
    ) -> Result<Self> {
        if label.trim().is_empty() {
            return Err("CV source FP context requires an explicit diagnostic label".into());
        }
        if x87_control & 0x0c00 != 0 {
            return Err("CV source arithmetic supports round-to-nearest only".into());
        }
        let precision = match x87_control & 0x0300 {
            0 => Precision::Single24,
            0x0200 => Precision::Double53,
            0x0300 => Precision::Extended64,
            _ => return Err("reserved x87 precision in CV source context".into()),
        };
        let environment = CrtEnvironment {
            sse2_available,
            mxcsr,
            x87_control,
        };
        if select_path(environment) != PowPath::Sse2 {
            return Err("CV source CRT pow requires its proven SSE2 path".into());
        }
        if mxcsr & !0x3f != 0x1f80 {
            return Err("CV source CRT pow MXCSR rounding/FTZ/DAZ is unsupported".into());
        }
        Ok(Self {
            precision,
            environment,
            status: x87_status,
            supplied_diagnostic_label: label.into(),
        })
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
    pub fn crt_environment(&self) -> CrtEnvironment {
        self.environment
    }
    pub fn x87_status(&self) -> u16 {
        self.status
    }
    pub fn diagnostic_label(&self) -> &str {
        &self.supplied_diagnostic_label
    }
    /// Until an actual application lifecycle receipt is implemented.
    pub fn is_observed_native_startup(&self) -> bool {
        false
    }
}

/// Cached Settings pointer-table offsets; None is an original lazy name lookup.
pub const NUMERIC_SETTINGS: [(&str, Option<u32>); 10] = [
    ("CombatDamageMinimum", Some(0xa4)),
    ("CombatValueRangePower", Some(0x144)),
    ("CombatValueAreaPower", Some(0x148)),
    ("CombatValueOffenseWeight", Some(0x14c)),
    ("CombatValueDefenseWeight", Some(0x150)),
    ("CombatValueMovementWeight", Some(0x154)),
    ("CombatValueMovementDefault", Some(0x158)),
    ("CombatValueMeanValuesDiscardPercent", Some(0x15c)),
    ("CombatValueRangedMinimum", None),
    ("CombatValueDefenseWeightRanged", None),
];
#[derive(Clone, Debug, Serialize)]
pub struct NumericValue {
    pub name: String,
    pub native_kind: u32,
    pub cached_pointer_offset: Option<u32>,
    pub bits: u32,
    pub declarations: Vec<Declaration>,
}

/// Borrowing the source Catalog prevents replacing its Settings owner while
/// these source values are borrowed. This does not adopt a live game's SVars.
pub struct NumericBindings<'a, 'source> {
    catalog: &'a Catalog<'source>,
    context: SourceFpContext,
    values: BTreeMap<String, NumericValue>,
}
impl NumericBindings<'_, '_> {
    pub fn context(&self) -> &SourceFpContext {
        &self.context
    }
    pub fn values(&self) -> &BTreeMap<String, NumericValue> {
        &self.values
    }
    pub fn value(&self, name: &str) -> Result<&NumericValue> {
        self.catalog.require_source_owner()?;
        self.values
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| format!("unbound CV numeric source {name}"))
    }
    pub fn formula_settings(&self) -> Result<aa_combat::combat_value::Settings> {
        let read = |name| self.value(name).map(|v| F32(v.bits));
        Ok(aa_combat::combat_value::Settings {
            damage_minimum: read("CombatDamageMinimum")?,
            range_power: read("CombatValueRangePower")?,
            area_power: read("CombatValueAreaPower")?,
            offense_weight: read("CombatValueOffenseWeight")?,
            defense_weight: read("CombatValueDefenseWeight")?,
            movement_weight: read("CombatValueMovementWeight")?,
            movement_default: read("CombatValueMovementDefault")?,
        })
    }
    pub fn mean_discard(&self) -> Result<F32> {
        Ok(F32(self.value("CombatValueMeanValuesDiscardPercent")?.bits))
    }
    pub fn ranged_minimum(&self) -> Result<F32> {
        Ok(F32(self.value("CombatValueRangedMinimum")?.bits))
    }
    pub fn defense_weight_ranged(&self) -> Result<F32> {
        Ok(F32(self.value("CombatValueDefenseWeightRanged")?.bits))
    }
}

/// Source SVars only: Game/SVars.tgi then Game/SVars_AA.tgi, one assigned
/// Fixed declaration per file. Native registered float2/flags0 values retain
/// their identity through4B3C10/4B2F80 overwrites. No constructor default fills
/// a missing source assignment. Registered WorldRule defaults run474C50 before
/// completion: any rule with one of these IDS is explicitly unsupported.
pub fn bind_numeric<'a, 'source>(
    catalog: &'a Catalog<'source>,
    context: &SourceFpContext,
) -> Result<NumericBindings<'a, 'source>> {
    catalog.require_source_owner()?;
    if catalog
        .actors
        .diagnostics
        .iter()
        .any(|d| !d.message.starts_with("native syntax notice "))
    {
        return Err("CV source loader diagnostics require classification before WorldRule absence can be used".into());
    }
    let registry = catalog
        .actors
        .native_data_registry("WorldRule")
        .ok_or("CV source WorldRule registry is unavailable")?;
    let source = catalog
        .settings
        .as_ref()
        .ok_or("CV source Settings owner is unavailable")?;
    let source_nodes = catalog.actors.resolved_definitions("SVars").map_err(err)?;
    let mut values = BTreeMap::new();
    for (name, offset) in NUMERIC_SETTINGS {
        if registry
            .entries
            .iter()
            .any(|entry| entry.ids.eq_ignore_ascii_case(name))
        {
            return Err(format!(
                "CV {name} WorldRule default application is not bound"
            ));
        }
        let rows = source
            .declarations
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| format!("CV {name} has no actual source assignment"))?;
        if rows.is_empty() || rows.len() > 2 {
            return Err(format!("CV {name} has ambiguous source assignments"));
        }
        let mut selected = None;
        let mut ordered = Vec::new();
        for path in ["Game/SVars.tgi", "Game/SVars_AA.tgi"] {
            let matches: Vec<_> = rows
                .iter()
                .filter(|r| r.source.path.eq_ignore_ascii_case(path))
                .collect();
            if matches.len() > 1 {
                return Err(format!("CV {name} repeats in {path}"));
            }
            let Some(row) = matches.first() else { continue };
            if !catalog
                .actors
                .sources
                .iter()
                .any(|s| s.path.eq_ignore_ascii_case(path) && s.sha256 == row.source.sha256)
            {
                return Err(format!(
                    "CV {name} source owner/hash differs from the registered catalog"
                ));
            }
            if !row
                .field_type
                .as_deref()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("fixed"))
            {
                return Err(format!("CV {name} requires its assigned Fixed declaration"));
            }
            let parsed =
                aa_rules::native::scalar_literal(&row.raw, context.precision).map_err(err)?;
            // Settings is a public legacy projection. Hash identity alone
            // must not bless a caller-mutated raw value or source span.
            let mut matching = Vec::new();
            for node in &source_nodes {
                if !node
                    .sources()
                    .iter()
                    .any(|s| s.path.eq_ignore_ascii_case(path) && s.sha256 == row.source.sha256)
                {
                    continue;
                }
                if let Some(aa_rules::native::TypedValue::Scalar(value)) =
                    node.field(name, context.precision).map_err(err)?
                    && value.source == row.source
                    && value.field_type == aa_rules::native::ScalarType::Fixed
                    && value.bits == parsed
                {
                    matching.push(value.bits);
                }
            }
            if matching.len() != 1 {
                return Err(format!(
                    "CV {name} Settings value/span differs from its actual registered source"
                ));
            }
            selected = Some(parsed);
            ordered.push((*row).clone());
        }
        if ordered.len() != rows.len() {
            return Err(format!("CV {name} has an unbound source file/override"));
        }
        let bits = selected.ok_or_else(|| format!("CV {name} has no proven source value"))?;
        values.insert(
            name.to_ascii_lowercase(),
            NumericValue {
                name: name.into(),
                native_kind: 2,
                cached_pointer_offset: offset,
                bits,
                declarations: ordered,
            },
        );
    }
    Ok(NumericBindings {
        catalog,
        context: context.clone(),
        values,
    })
}

/// Explicit input scope; this module never discovers or reads user profiles.
pub enum CsvScope<'a> {
    Unknown,
    /// Diagnostic application deliberately created without an original profile.
    FreshNoProfile {
        label: &'a str,
    },
    /// Caller supplies COMPLETE order and already authorized bytes. This is
    /// not a claim that the original app's profile discovery/order was run.
    SuppliedOrderedOverrides {
        files: &'a [OverrideFile<'a>],
        label: &'a str,
    },
}
///4B43B0 flags0 is ordinary source loading; actual UserUVarsFilename uses3.
/// Its low bit restricts changes to existing variables with native flag16.
pub struct OverrideFile<'a> {
    pub file: &'a ResolvedFile,
    pub native_load_flags: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct CsvAssignment {
    pub value_utf16: Vec<u16>,
    pub source: Provenance,
}
#[derive(Clone, Debug, Serialize)]
pub struct CsvBinding {
    value_utf16: Vec<u16>,
    assignments: Vec<CsvAssignment>,
    inputs: Vec<SourceIdentity>,
    input_load_flags: Vec<u32>,
    ignored_restricted_assignments: Vec<Provenance>,
    diagnostic_scope: String,
}
impl CsvBinding {
    pub fn value_utf16(&self) -> &[u16] {
        &self.value_utf16
    }
    pub fn assignments(&self) -> &[CsvAssignment] {
        &self.assignments
    }
    pub fn inputs(&self) -> &[SourceIdentity] {
        &self.inputs
    }
    pub fn ignored_restricted_assignments(&self) -> &[Provenance] {
        &self.ignored_restricted_assignments
    }
    pub fn diagnostic_scope(&self) -> &str {
        &self.diagnostic_scope
    }
    pub fn is_observed_native_startup(&self) -> bool {
        false
    }
    ///4BB590 checks CString length, not C-string prefix. Nonempty debug output
    /// needs the actual stream/CSV operations; it is never silently omitted.
    pub fn require_empty_debug_stream(&self) -> Result<()> {
        if self.value_utf16.is_empty() {
            Ok(())
        } else {
            Err("4BB590 nonempty CombatValueCSV debug stream is unbound".into())
        }
    }
}
fn csv_assignment(file: &ResolvedFile) -> Result<Option<CsvAssignment>> {
    if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
        return Err(format!("CSV source hash differs: {}", file.logical_path));
    }
    let doc = Document::parse_native(file.bytes.clone(), Limits::default()).map_err(err)?;
    let mut selected = None;
    for item in &doc.items {
        let Item::Node(node) = item else { continue };
        if !doc
            .header_name(&node.header)
            .is_some_and(|name| name.eq_ignore_ascii_case("Vars"))
        {
            continue;
        }
        if node.header.tokens.len() != 1 {
            return Err("CSV Vars inheritance requires its actual native source owner".into());
        }
        for item in &node.items {
            let Item::Property(property) = item else {
                continue;
            };
            if !doc
                .property_name(property)
                .is_some_and(|name| name.eq_ignore_ascii_case("CombatValueCSV"))
            {
                continue;
            }
            if selected.is_some() {
                return Err("CombatValueCSV duplicate field/node assignment is unsupported".into());
            }
            if property.left.len() != 2
                || !doc
                    .text(property.left[0].span)
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("string"))
            {
                return Err("CombatValueCSV requires an assigned typed String source".into());
            }
            let raw = doc
                .property_value(property)
                .ok_or("CombatValueCSV source has no assignment")?;
            if raw.is_empty() {
                return Err(
                    "CombatValueCSV unquoted empty assignment is outside the proven source scope"
                        .into(),
                );
            }
            let value = if raw.eq_ignore_ascii_case("null") || raw == "\"\"" {
                ""
            } else {
                // Native plain-string assignment is proven separately. Quoted
                // nonempty/expression/escape expansion is deliberately refused.
                if !raw.is_ascii()
                    || raw
                        .chars()
                        .any(|c| c == '"' || c == '\\' || c == '&' || c == '\0')
                {
                    return Err("CombatValueCSV nonempty quoted/expanded text is outside the proven source scope".into());
                }
                raw
            };
            selected = Some(CsvAssignment {
                value_utf16: value.encode_utf16().collect(),
                source: Provenance {
                    path: file.logical_path.clone(),
                    sha256: file.sha256.clone(),
                    span: property.span,
                },
            });
        }
    }
    Ok(selected)
}

/// Root UVars wildcard set in native ordinal filename order, then the supplied
/// override order. Source identities/hashes and effective assignment values
/// are retained; ignored restricted assignments retain their provenance.
pub fn bind_csv_files(root_files: &[ResolvedFile], scope: CsvScope<'_>) -> Result<CsvBinding> {
    let (overrides, label) = match scope {
        CsvScope::Unknown => {
            return Err("CombatValueCSV profile/application scope is unknown".into());
        }
        CsvScope::FreshNoProfile { label } => (None, label),
        CsvScope::SuppliedOrderedOverrides { files, label } => (Some(files), label),
    };
    if label.trim().is_empty() {
        return Err("CombatValueCSV requires an explicit diagnostic scope label".into());
    }
    let mut roots: Vec<_> = root_files.iter().collect();
    for file in &roots {
        let name = &file.logical_path;
        let lower = name.to_ascii_lowercase();
        if !name.is_ascii()
            || lower.contains(['/', '\\'])
            || !lower.starts_with("uvars")
            || !lower.ends_with(".tgi")
        {
            return Err("CSV root wildcard source set contains a non-root UVars input".into());
        }
    }
    roots.sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
    if roots
        .iter()
        .map(|file| file.logical_path.to_ascii_lowercase())
        .collect::<BTreeSet<_>>()
        .len()
        != roots.len()
    {
        return Err("CSV wildcard filenames collide".into());
    }
    if !roots
        .iter()
        .any(|f| f.logical_path.eq_ignore_ascii_case("UVars.tgi"))
    {
        return Err("CSV source set lacks UVars.tgi".into());
    }
    let mut out = CsvBinding {
        value_utf16: Vec::new(),
        assignments: Vec::new(),
        inputs: Vec::new(),
        input_load_flags: Vec::new(),
        ignored_restricted_assignments: Vec::new(),
        diagnostic_scope: label.into(),
    };
    let root_count = roots.len();
    let mut files: Vec<_> = roots.into_iter().map(|file| (file, 0)).collect();
    for input in overrides.into_iter().flatten() {
        if !matches!(input.native_load_flags, 0 | 3) {
            return Err("CV CSV override native load flags outside proven0/3 scope".into());
        }
        files.push((input.file, input.native_load_flags));
    }
    for (ordinal, (file, load_flags)) in files.into_iter().enumerate() {
        if ordinal == root_count && out.assignments.is_empty() {
            return Err("CombatValueCSV requires its actual source-created root String variable before profile loading".into());
        }
        let assignment = csv_assignment(file)?;
        out.inputs.push(SourceIdentity {
            path: file.logical_path.clone(),
            sha256: file.sha256.clone(),
            origin: file.origin.clone(),
        });
        out.input_load_flags.push(load_flags);
        if let Some(assignment) = assignment {
            //419F72/419F82 passes3 for UserUVarsFilename.4B43B0 sets
            // Settings+24=1;4B3C10 skips this existing source-created
            // String variable because its native flags are0 (not16).
            if load_flags & 1 != 0 {
                out.ignored_restricted_assignments.push(assignment.source);
                continue;
            }
            out.value_utf16 = assignment.value_utf16.clone();
            out.assignments.push(assignment);
        }
    }
    if out.assignments.is_empty() {
        return Err("CombatValueCSV has no actual String source assignment".into());
    }
    Ok(out)
}
pub fn bind_csv_from_sources(sources: &mut V14Sources, scope: CsvScope<'_>) -> Result<CsvBinding> {
    let names: Vec<_> = sources
        .catalog()
        .into_iter()
        .map(|e| e.logical_path)
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            !lower.contains(['/', '\\']) && lower.starts_with("uvars") && lower.ends_with(".tgi")
        })
        .collect();
    let files = names
        .iter()
        .map(|name| sources.read(name).map_err(err))
        .collect::<Result<Vec<_>>>()?;
    bind_csv_files(&files, scope)
}
