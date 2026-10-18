//! Native Organization layout definitions exactly as AA.exe builds them
//! while it loads the data files (OrganizationData.cpp), plus the ActorData
//! OrganizationComponent part that binds a definition to its layout.
//!
//! Native objects and functions ported here (image base 0x400000):
//! * DataManagers built by the GameData constructor 48A8E0: `+464` Property
//!   (47EFD0, 64 buckets, mode 1), `+470` OrganizationLayout (481740, mode 0,
//!   also `[B3AB00]`), `+474` Formation (481A50, mode 0), `+478`
//!   GroupFormation (4840A0, mode 0). vf0 483A40 -> vf1C (48B580 / 4818A0 /
//!   481BB0 / 484200): `GetString("IDS")`, then mode 1 reuses an existing
//!   definition, otherwise the create path (48B640 / 481960 / 481C70 /
//!   4842C0) throws "Data IDS already used: '%s'" for a registered IDS,
//!   creates the object with `+C` = registration ordinal, registers it and
//!   only then runs the type LoadData (so a definition whose LoadData throws
//!   stays registered with its partial state).
//! * Find (463FA0 = 48D500 = 48D490): empty IDS -> null; the translation map
//!   first, then the registry with the target only (case-insensitive).
//! * KKC_FormationData (4C2540 ctor, 0x28 bytes): LoadData 4C34A0.
//! * KKC_OrganizationLayoutData (4C2690 ctor, 0x54 bytes): LoadData 4C3610
//!   with the LayoutFormation objects (0x1C bytes, vtable A232A8, LoadData
//!   4C32B0), CheckFormationMatch 4C1BE0 and the formation index lookup
//!   4C3110; the globals `[B3B6B0]` (largest slot `number`) and `[B3B6B4]`
//!   (largest slot-group count).
//! * Property type LoadData 4C48C0 (`is_technology`); the embedded
//!   ModifierPropertyData objects (461ED0, 0x5C bytes) of a layout's
//!   `BrokenProperty` (`+50`) and a formation's `Property` (`+24`).
//! * 48D660 file processing: a native throw abandons the remaining roots of
//!   that file; 4763C0 icon path rule with GameData+4 = 85A550(file path).
//! * ActorData 464BD0: Properties list 4C4CF0 (`+1D4`) and the
//!   OrganizationComponent block 466C4F..46707C (layout `+2E8`, default
//!   layout `+2F0` = 4C2110, 4C2310), deferred element link 460000.
//!
//! NOT ported here (explicitly recorded, never defaulted):
//! * KKC_ModifierPropertyData::LoadData 4C42D0 (property modifiers, icons,
//!   FX, its 47D960 name step and its throws). This producer only records
//!   when and on which node it is called ([`LoadEvent::PropertyLoad`]).
//! * GroupFormation type LoadData (49A660 object): only its registration.
//! * Texture loading 5A77E0 (presentation): the requested path is recorded.
//! * String-table lookup 4B9F40 for `#` names: [`StoredName::resolve`] takes
//!   the lookup from the caller.
//!
//! Oracle: research/phase8-layouts/native-layout-data.json (original code
//! under Unicorn on the shipped files and on synthetic cases), replayed by
//! crates/aa-rules/tests/native_layout_data.rs.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::TypedValue,
    resources::Provenance,
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::collections::BTreeMap;

// ------------------------------------------------------------------ errors

/// One printf argument of a native message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Arg {
    Text(String),
    Int(i32),
}

/// A native C++ exception thrown while loading definitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum NativeThrow {
    /// 410C40 TGC_BadDataException: original format literal and arguments.
    Data { format: &'static str, args: Vec<Arg> },
    /// TGC_DataFileNodeException "Node doesn't contain field." from a typed
    /// node getter (5B54D0, 5B5380, 5B4ED0, 5B5130, 5B4D80, 5B5620).
    MissingField { field: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, thiserror::Error)]
pub enum LayoutError {
    #[error("native throw: {0:?}")]
    Native(NativeThrow),
    /// The input lies outside what this port can decide (never a guess).
    #[error("unsupported: {0}")]
    Unsupported(String),
}
pub type Result<T> = std::result::Result<T, LayoutError>;

impl From<crate::native::Error> for LayoutError {
    fn from(e: crate::native::Error) -> Self {
        Self::Unsupported(e.to_string())
    }
}
/// printf of a native message (`%s`, `%d`, `%%`; `h`/`l` size prefixes).
pub fn native_format(format: &str, args: &[Arg]) -> String {
    let mut out = String::new();
    let mut args = args.iter();
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        while chars.peek().is_some_and(|c| *c == 'h' || *c == 'l') {
            chars.next();
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some(_) => match args.next() {
                Some(Arg::Text(t)) => out.push_str(t),
                Some(Arg::Int(v)) => out.push_str(&v.to_string()),
                None => {}
            },
            None => out.push('%'),
        }
    }
    out
}
impl NativeThrow {
    /// The exception text (TGC_BadDataException: the formatted message).
    pub fn message(&self) -> String {
        match self {
            Self::Data { format, args } => native_format(format, args),
            Self::MissingField { .. } => "Node doesn't contain field.".into(),
        }
    }
}
fn throw(format: &'static str, args: Vec<Arg>) -> LayoutError {
    LayoutError::Native(NativeThrow::Data { format, args })
}
fn missing(field: &str) -> LayoutError {
    LayoutError::Native(NativeThrow::MissingField {
        field: field.to_owned(),
    })
}
fn unsupported(text: impl Into<String>) -> LayoutError {
    LayoutError::Unsupported(text.into())
}
fn text(s: &str) -> Arg {
    Arg::Text(s.to_owned())
}

/// Native message literals (strings.tsv addresses).
pub mod formats {
    pub const IDS_USED: &str = "Data IDS already used: '%s'"; // A0B9F8
    pub const UNKNOWN_SLOT_PROPERTY: &str = "Unknown property '%s' loading layout '%s'"; // A23BE0
    pub const TWO_PRIMARY: &str = "Layout '%s' cannot have more than one primary slot."; // A23B50
    pub const UNKNOWN_FORMATION: &str = "Unknown formation '%s' loading layout '%s'"; // A23850
    pub const UNKNOWN_POSITION_PROPERTY: &str = "Unknown property '%s' loading formation '%s'"; // A237F0
    pub const INCOMPATIBLE: &str = "Incompatible formation '%s' loading layout '%s'"; // A23AF0
    pub const NEEDS_FORMATION: &str = "Layout '%s' needs at least one formation"; // A23A98
    pub const MISSING_BROKEN: &str = "Layout '%s' missing broken formation property"; // A23A18
    pub const UNRECOGNIZED: &str = "Layout '%s' unrecognized formation: '%s'"; // A23730
    pub const NOT_USED: &str = "Layout '%s' does not use formation: '%s'"; // A236D8
    pub const UNKNOWN_GROUP: &str = "Unknown group formation '%s' loading formation '%s'"; // A238C8
    pub const SIZE_LOG: &str = "formation '%s' size %d does not match layout '%s' size %d"; // A230E0
    pub const PROPERTY_LOG: &str =
        "slot %d formation '%s' property '%s' does not match layout '%s' property '%s'"; // A22F88
    pub const UNKNOWN_PROPERTY: &str = "Unknown property '%s'"; // A24120
    pub const MORALE_BREAKPOINT: &str = "Morale breakpoint must be non-negative in actor '%s'"; // A07E30
    pub const CONTROL_RANGE: &str = "Control range must be positive in actor '%s'"; // A07DD0
    pub const OPERATIONAL_RANGE: &str = "Operational range must be positive in actor '%s'"; // A07D48
    pub const UNKNOWN_LAYOUT: &str = "Unknown layout '%s' loading organization actor '%s'"; // A07CC8
    pub const TOO_MANY_SLOTS: &str = "Too many slot elements defined loading organization actor '%s'"; // A07AA8
    pub const LOCKED_NOT_REQUIRED: &str =
        "Slot definition index %d is locked, therefore you also need to flag it as required."; // A07A00
    pub const REQUIRED_ELEMENT: &str =
        "Slot definition index %d requires defining an element for organization actor '%s'"; // A07940
    pub const UNKNOWN_ELEMENT: &str = "Unknown element '%s' loading organization actor '%s'";
    pub const NOT_ELEMENT: &str = "Actor '%s' is not an element (loading organization actor '%s')";
    pub const LACKS_PROPERTY: &str =
        "Actor '%s' does not have property '%s' (loading organization actor '%s')";
    pub const PACKABLE: &str = "Organization '%s' packable/unpackable element '%s' must be in own slot";
    pub const HQ_RESEARCH: &str = "HQ Organiazation '%s' element '%s' has a research component with techs";
}
use formats as f;

// ----------------------------------------------------------- node getters

fn value(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<TypedValue> {
    node.field(name, pc)?.ok_or_else(|| missing(name))
}
/// 5B54D0 GetString (case-insensitive `NULL` is the empty string).
fn get_string(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<String> {
    match value(node, name, pc)? {
        TypedValue::String { value, .. } => Ok(value),
        other => Err(unsupported(format!("{name} is not a native string field: {other:?}"))),
    }
}
/// 5B5380 GetFlag.
fn get_flag(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<bool> {
    match value(node, name, pc)? {
        TypedValue::Boolean { value, .. } => Ok(value),
        other => Err(unsupported(format!("{name} is not a native flag field: {other:?}"))),
    }
}
/// 5B4ED0 GetInt.
fn get_int(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<i32> {
    match value(node, name, pc)? {
        TypedValue::Integer { value, .. } => Ok(value),
        other => Err(unsupported(format!("{name} is not a native int field: {other:?}"))),
    }
}
/// 5B5130 GetFixed (binary32 bits).
fn get_fixed(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<u32> {
    Ok(value(node, name, pc)?.fixed_bits()?)
}
/// 5B4D80 GetPoint2.
fn get_point(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<[u32; 2]> {
    match value(node, name, pc)? {
        TypedValue::Point { bits, .. } if bits.len() == 2 => Ok([bits[0], bits[1]]),
        other => Err(unsupported(format!("{name} is not a native point2 field: {other:?}"))),
    }
}

/// A `localized` field as 5B5620 stores it before the string-table step.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct StoredName {
    /// Parser text widened byte by byte (857210), quotes and one leading `@`
    /// removed; `#key` is kept as authored.
    pub stored: Vec<u16>,
}
impl StoredName {
    /// 5B5620: a `#key` goes through the string table 4B9F40 (`lookup`
    /// returns None for its static empty default B43B48); anything else is
    /// the stored text.
    pub fn resolve(&self, lookup: &mut dyn FnMut(&[u16]) -> Option<Vec<u16>>) -> Vec<u16> {
        if self.stored.first() == Some(&u16::from(b'#')) {
            lookup(&self.stored[1..]).unwrap_or_default()
        } else {
            self.stored.clone()
        }
    }
    fn is_key(&self) -> bool {
        self.stored.first() == Some(&u16::from(b'#'))
    }
}
/// 5B5620(node, field).
fn get_localized(node: &ResolvedNode<'_>, field: &str) -> Result<StoredName> {
    match node.localized(field)? {
        Some(t) => Ok(StoredName {
            stored: t.text.bytes().map(u16::from).collect(),
        }),
        None => match node.field(field, Precision::Double53) {
            Ok(None) => Err(missing(field)),
            _ => Err(unsupported(format!(
                "localized field {field} is declared without a value (strict-template parse failure)"
            ))),
        },
    }
}

// --------------------------------------------------------------- registry

/// Load position of a root: (index in `ActorCatalog::sources`, byte offset).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RootPosition {
    pub file: usize,
    pub offset: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
struct Registry {
    /// (lowercase key, slot, registration position)
    entries: Vec<(String, u32, RootPosition)>,
    /// (lowercase old, new text, position)
    translations: Vec<(String, String, RootPosition)>,
}
impl Registry {
    /// 463FA0 / 48D500 as of `at` (None: after the whole load).
    fn find(&self, id: &str, at: Option<RootPosition>) -> Option<u32> {
        if id.is_empty() {
            return None;
        }
        let visible = |p: &RootPosition| at.is_none_or(|a| *p < a);
        let mut k = id.to_ascii_lowercase();
        if let Some((_, new, _)) = self
            .translations
            .iter()
            .find(|(old, _, p)| *old == k && visible(p))
        {
            k = new.to_ascii_lowercase();
        }
        self.entries
            .iter()
            .find(|(key, _, p)| *key == k && visible(p))
            .map(|(_, slot, _)| *slot)
    }
    fn registered(&self, id: &str) -> Option<u32> {
        let k = id.to_ascii_lowercase();
        self.entries.iter().find(|(key, ..)| *key == k).map(|(_, s, _)| *s)
    }
    fn insert(&mut self, id: &str, slot: u32, at: RootPosition) {
        self.entries.push((id.to_ascii_lowercase(), slot, at));
    }
}

fn ordinal(len: usize) -> Result<u32> {
    u32::try_from(len).map_err(|_| unsupported("definition ordinal exceeds the native 32-bit +C word"))
}

// -------------------------------------------------------------- the data

/// A Property reference stored in a layout or formation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum PropertyRef {
    /// A Property DataManager definition (`+C` registration ordinal).
    Registered(u32),
    /// An embedded ModifierPropertyData (index in `embedded_properties`).
    Embedded(u32),
}
/// Handle bit marking an embedded property (registered ones are ordinal+1).
pub const EMBEDDED_PROPERTY_HANDLE: u32 = 0x8000_0000;
impl PropertyRef {
    /// Nonzero 32-bit handle standing for the native object pointer:
    /// registered = ordinal + 1; embedded = 0x8000_0000 | (index + 1).
    pub fn handle(self) -> Result<u32> {
        let (index, tag) = match self {
            Self::Registered(i) => (i, 0),
            Self::Embedded(i) => (i, EMBEDDED_PROPERTY_HANDLE),
        };
        if index >= EMBEDDED_PROPERTY_HANDLE - 1 {
            return Err(unsupported("property index outside the 31-bit handle space"));
        }
        Ok(tag | (index + 1))
    }
    pub fn from_handle(handle: u32) -> Option<Self> {
        let index = (handle & !EMBEDDED_PROPERTY_HANDLE).checked_sub(1)?;
        Some(if handle & EMBEDDED_PROPERTY_HANDLE != 0 {
            Self::Embedded(index)
        } else {
            Self::Registered(index)
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RegisteredProperty {
    pub ordinal: u32,
    /// `+8`: IDS of the first registration.
    pub ids: String,
    pub sources: Vec<Provenance>,
    pub position: RootPosition,
    /// `+5C` (4C48C0 `is_technology`) of the latest load; None if not reached.
    pub is_technology: Option<bool>,
    /// Mode-1 re-loads of this definition by later roots.
    pub reloads: u32,
    /// Stored name of the latest load (47D960 inside 4C42D0, not executed
    /// here; used for the mode-1 duplicate rule).
    stored_name: Option<StoredName>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum EmbeddedOwner {
    /// FormationData `+24` (`[Property]` child).
    Formation(u32),
    /// OrganizationLayoutData `+50` (`[BrokenProperty]` child).
    LayoutBroken(u32),
}
#[derive(Clone, Debug, Serialize)]
pub struct EmbeddedProperty {
    pub owner: EmbeddedOwner,
    /// The node passed to 4C42D0 (its layers, most specific first).
    pub sources: Vec<Provenance>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GroupFormationEntry {
    pub ordinal: u32,
    pub ids: String,
    pub sources: Vec<Provenance>,
    pub position: RootPosition,
}

/// KKC_FormationData (`+18` organized is 1 from the constructor 4C2540).
#[derive(Clone, Debug, Serialize)]
pub struct FormationData {
    pub ordinal: u32,
    pub ids: String,
    pub sources: Vec<Provenance>,
    pub position: RootPosition,
    /// `+10` stored name and `+14` (set by 47D960).
    pub name: StoredName,
    pub named: bool,
    pub organized: bool,
    pub herding: bool,
    pub aggressive: bool,
    /// `+1C` texture: the path handed to 5A77E0 (None: not reached, word 0).
    pub icon: Option<Vec<u16>>,
    /// `+20` GroupFormation ordinal.
    pub group_formation: Option<u32>,
    /// `+24` embedded property.
    pub property: Option<PropertyRef>,
    /// The LoadData exception, if it threw (the object stays registered).
    pub error: Option<LayoutError>,
}

/// One `+1C` slot group (16 bytes; 4C2250 defaults: name "", property null,
/// number 1, required 1, locked 0, morale_line 0).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LayoutSlotEntry {
    pub name: StoredName,
    pub property: Option<PropertyRef>,
    pub number: i32,
    pub required: bool,
    pub locked: bool,
    pub morale_line: bool,
}
impl Default for LayoutSlotEntry {
    fn default() -> Self {
        Self {
            name: StoredName::default(),
            property: None,
            number: 1,
            required: true,
            locked: false,
            morale_line: false,
        }
    }
}
/// One Position (12 bytes: property, offset x, y; offsets zeroed at growth).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FormationPosition {
    pub property: Option<PropertyRef>,
    pub offset_bits: [u32; 2],
}
/// KKC_OrganizationLayoutFormation (vtable A232A8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LayoutFormation {
    /// Nonzero handle for the native object pointer (see [`NativeLayoutData::formation_facts`]).
    pub handle: u32,
    /// `+C` FormationData ordinal.
    pub formation: Option<u32>,
    /// `+10/+14` positions.
    pub positions: Vec<FormationPosition>,
}

/// KKC_OrganizationLayoutData (ctor 4C2690 initial state; LoadData 4C3610).
#[derive(Clone, Debug, Serialize)]
pub struct OrganizationLayoutData {
    pub ordinal: u32,
    /// `+8` IDS.
    pub ids: String,
    pub sources: Vec<Provenance>,
    pub position: RootPosition,
    pub name: StoredName,
    pub named: bool,
    /// `+18` (-1: none).
    pub primary_slot: i32,
    /// `+1C/+20`.
    pub entries: Vec<LayoutSlotEntry>,
    /// `+28/+2C`: entry index of every slot.
    pub slot_groups: Vec<u32>,
    /// `+34/+38`.
    pub formations: Vec<LayoutFormation>,
    /// `+40` default, `+44` combat, `+48` retreat formation indices.
    pub default_formation: i32,
    pub combat_formation: i32,
    pub retreat_formation: i32,
    /// `+4C` sum of `number` over morale-line entries (wrapping add).
    pub morale_count: i32,
    /// `+50` BrokenProperty.
    pub broken_property: Option<PropertyRef>,
    pub error: Option<LayoutError>,
}

/// Recorded native calls/messages in load order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum LoadEvent {
    /// 48D660 started a file containing roots of these managers.
    File { path: String },
    /// 4C42D0 KKC_ModifierPropertyData::LoadData (not executed here).
    PropertyLoad { property: PropertyRef },
    /// 5A77E0 texture request (presentation).
    Texture { path: Vec<u16> },
    /// 404ED0 log (channel B38B70, OrganizationData.cpp, CheckFormationMatch).
    Log { line: u32, format: &'static str, args: Vec<Arg> },
    /// 410C40 throw (Data exceptions only).
    Throw { format: &'static str, args: Vec<Arg> },
}

/// 48D660 outcome of one file holding roots of these managers.
#[derive(Clone, Debug, Serialize)]
pub struct FileOutcome {
    pub path: String,
    pub source_index: usize,
    /// The exception that abandoned the rest of the file.
    pub abandoned: Option<LayoutError>,
}

/// The whole native producer state after the data load.
#[derive(Clone, Debug, Default, Serialize)]
pub struct NativeLayoutData {
    pub properties: Vec<RegisteredProperty>,
    pub embedded_properties: Vec<EmbeddedProperty>,
    pub group_formations: Vec<GroupFormationEntry>,
    pub formations: Vec<FormationData>,
    pub layouts: Vec<OrganizationLayoutData>,
    /// Owner (layout ordinal, formation index) of every LayoutFormation
    /// handle (handle = index + 1), in creation order.
    pub layout_formations: Vec<(u32, u32)>,
    /// `[B3B6B0]` and `[B3B6B4]` (.bss, initially 0).
    pub max_slot_number: i32,
    pub max_slot_groups: i32,
    pub files: Vec<FileOutcome>,
    pub events: Vec<LoadEvent>,
    property_registry: Registry,
    group_registry: Registry,
    formation_registry: Registry,
    layout_registry: Registry,
}

// --------------------------------------------------------------- the load

const MANAGERS: [&str; 4] = ["Property", "GroupFormation", "Formation", "OrganizationLayout"];

/// 85A550: the directory of a resource path (last '/' or '\\'; else up to
/// and including ':'; else empty).
pub fn native_directory(path: &[u16]) -> Vec<u16> {
    if let Some(i) = path.iter().rposition(|&c| c == u16::from(b'/') || c == u16::from(b'\\')) {
        return path[..i].to_vec();
    }
    if let Some(i) = path.iter().position(|&c| c == u16::from(b':')) {
        return path[..=i].to_vec();
    }
    Vec::new()
}
/// 4763C0: icon path relative to the current data file directory.
pub fn native_icon_path(directory: &[u16], path: &[u16]) -> Vec<u16> {
    if path.is_empty() {
        return path.to_vec();
    }
    if path[0] == u16::from(b'/') {
        let start = path.iter().position(|&c| c != u16::from(b'/')).unwrap_or(path.len());
        return path[start..].to_vec();
    }
    if directory.is_empty() {
        return path.to_vec();
    }
    let mut out = directory.to_vec();
    out.push(u16::from(b'/'));
    out.extend_from_slice(path);
    out
}
fn wide(s: &str) -> Vec<u16> {
    s.bytes().map(u16::from).collect()
}

struct Root<'a> {
    kind: usize, // index into MANAGERS, 4 = Translate
    node: ResolvedNode<'a>,
    position: RootPosition,
}

impl NativeLayoutData {
    /// The native load of the Property, GroupFormation, Formation and
    /// OrganizationLayout roots of `catalog` (and Translate roots naming
    /// those managers), in native file/root order.
    pub fn load(catalog: &ActorCatalog, pc: Precision) -> Result<Self> {
        let files: BTreeMap<String, usize> = catalog
            .sources
            .iter()
            .enumerate()
            .map(|(i, s)| (s.path.to_ascii_lowercase(), i))
            .collect();
        let position = |node: &ResolvedNode<'_>| -> Result<RootPosition> {
            let src = node
                .sources()
                .into_iter()
                .next()
                .ok_or_else(|| unsupported("root without a source"))?;
            let file = *files
                .get(&src.path.to_ascii_lowercase())
                .ok_or_else(|| unsupported(format!("root source {} is not a catalog file", src.path)))?;
            Ok(RootPosition {
                file,
                offset: src.span.start,
            })
        };
        let mut roots = Vec::new();
        for (kind, tag) in MANAGERS.iter().enumerate() {
            for node in catalog.resolved_definitions(tag)? {
                let position = position(&node)?;
                roots.push(Root { kind, node, position });
            }
        }
        for node in catalog.resolved_definitions("Translate")? {
            let manager = match node.plain_string_field("node")? {
                Some(TypedValue::String { value, .. }) => value,
                _ => continue,
            };
            if MANAGERS.iter().any(|m| m.eq_ignore_ascii_case(&manager)) {
                let position = position(&node)?;
                roots.push(Root { kind: 4, node, position });
            }
        }
        roots.sort_by_key(|r| r.position);
        let mut out = Self::default();
        let mut index = 0;
        while index < roots.len() {
            let file = roots[index].position.file;
            let path = catalog.sources[file].path.clone();
            out.events.push(LoadEvent::File { path: path.clone() });
            let directory = native_directory(&wide(&path));
            let mut abandoned = None;
            while index < roots.len() && roots[index].position.file == file {
                if abandoned.is_none() {
                    let root = &roots[index];
                    if let Err(e) = out.load_root(root, &directory, pc) {
                        if let LayoutError::Unsupported(_) = e {
                            return Err(e);
                        }
                        abandoned = Some(e);
                    }
                }
                index += 1;
            }
            out.files.push(FileOutcome {
                path,
                source_index: file,
                abandoned,
            });
        }
        // The native-order catalog records the first 48D660 exception of a
        // file (registration rules of every manager, but no type LoadData).
        // One that is not this producer's own first exception came from
        // another manager's root, whose position (and so the set of roots it
        // hid here) is not identifiable: refused instead of guessed.
        const PREFIX: &str = "native 48D660 exception, remaining roots of this file skipped: ";
        for outcome in &out.files {
            for d in &catalog.diagnostics {
                if !d.source.path.eq_ignore_ascii_case(&outcome.path) {
                    continue;
                }
                let Some(message) = d.message.strip_prefix(PREFIX) else {
                    continue;
                };
                let own = match &outcome.abandoned {
                    Some(LayoutError::Native(t)) => t.message() == message,
                    _ => false,
                };
                if !own {
                    return Err(unsupported(format!(
                        "{}: another manager's root threw during 48D660 ({message}); the roots it hid are not identifiable here",
                        outcome.path
                    )));
                }
            }
        }
        Ok(out)
    }

    fn load_root(&mut self, root: &Root<'_>, directory: &[u16], pc: Precision) -> Result<()> {
        let node = &root.node;
        if root.kind == 4 {
            return self.translate(node, root.position, pc);
        }
        // vf1C: GetString("IDS") before any registry step.
        let ids = get_string(node, "IDS", pc)?;
        match root.kind {
            0 => self.load_property(node, ids, root.position, pc),
            1 => {
                let slot = self.create(1, &ids, root.position)?;
                self.group_formations.push(GroupFormationEntry {
                    ordinal: slot,
                    ids,
                    sources: node.sources(),
                    position: root.position,
                });
                Ok(())
            }
            2 => {
                let slot = self.create(2, &ids, root.position)?;
                self.formations.push(FormationData {
                    ordinal: slot,
                    ids,
                    sources: node.sources(),
                    position: root.position,
                    name: StoredName::default(),
                    named: false,
                    organized: true,
                    herding: false,
                    aggressive: false,
                    icon: None,
                    group_formation: None,
                    property: None,
                    error: None,
                });
                let result = self.load_formation(slot as usize, node, root.position, directory, pc);
                self.record_error(&result, |s, e| s.formations[slot as usize].error = Some(e));
                result
            }
            _ => {
                let slot = self.create(3, &ids, root.position)?;
                self.layouts.push(OrganizationLayoutData {
                    ordinal: slot,
                    ids,
                    sources: node.sources(),
                    position: root.position,
                    name: StoredName::default(),
                    named: false,
                    primary_slot: -1,
                    entries: Vec::new(),
                    slot_groups: Vec::new(),
                    formations: Vec::new(),
                    default_formation: 0,
                    combat_formation: 0,
                    retreat_formation: 0,
                    morale_count: 0,
                    broken_property: None,
                    error: None,
                });
                let result = self.load_layout(slot as usize, node, root.position, pc);
                self.record_error(&result, |s, e| s.layouts[slot as usize].error = Some(e));
                result
            }
        }
    }

    fn record_error(&mut self, result: &Result<()>, set: impl FnOnce(&mut Self, LayoutError)) {
        if let Err(e) = result {
            set(self, e.clone());
        }
    }

    fn registry(&mut self, kind: usize) -> &mut Registry {
        match kind {
            0 => &mut self.property_registry,
            1 => &mut self.group_registry,
            2 => &mut self.formation_registry,
            _ => &mut self.layout_registry,
        }
    }

    /// Mode-0 create path (481960 shape): duplicate IDS throws, otherwise
    /// the new definition gets `+C` = the manager count and is registered.
    fn create(&mut self, kind: usize, ids: &str, at: RootPosition) -> Result<u32> {
        let count = match kind {
            0 => self.properties.len(),
            1 => self.group_formations.len(),
            2 => self.formations.len(),
            _ => self.layouts.len(),
        };
        let registry = self.registry(kind);
        if registry.registered(ids).is_some() {
            let e = throw(f::IDS_USED, vec![text(ids)]);
            self.events.push(throw_event(&e));
            return Err(e);
        }
        let slot = ordinal(count)?;
        self.registry(kind).insert(ids, slot, at);
        Ok(slot)
    }

    /// Property manager (mode 1) vf1C 48B580 then type LoadData 4C48C0.
    fn load_property(&mut self, node: &ResolvedNode<'_>, ids: String, at: RootPosition, pc: Precision) -> Result<()> {
        let name = get_localized(node, "name").ok();
        let slot = match self.property_registry.registered(&ids) {
            Some(slot) => {
                // 47D960 (inside 4C42D0) throws "loading duplicate node with
                // ids" when the earlier load stored a nonempty name.
                let previous = &self.properties[slot as usize];
                match &previous.stored_name {
                    Some(n) if n.is_key() => {
                        return Err(unsupported(format!(
                            "Property {ids} re-loaded after a '#key' name: needs the string table (47D960)"
                        )));
                    }
                    Some(n) if !n.stored.is_empty() => {
                        return Err(unsupported(format!(
                            "Property {ids} re-loaded after a named load: 47D960 inside the unported 4C42D0 throws"
                        )));
                    }
                    None => {
                        return Err(unsupported(format!(
                            "Property {ids} re-loaded after a load without a name field"
                        )));
                    }
                    _ => {}
                }
                self.properties[slot as usize].reloads += 1;
                slot
            }
            None => {
                let slot = ordinal(self.properties.len())?;
                self.property_registry.insert(&ids, slot, at);
                self.properties.push(RegisteredProperty {
                    ordinal: slot,
                    ids,
                    sources: node.sources(),
                    position: at,
                    is_technology: None,
                    reloads: 0,
                    stored_name: None,
                });
                slot
            }
        };
        self.properties[slot as usize].stored_name = name;
        // 4C48C0: 4C42D0 (recorded seam) then is_technology.
        self.events.push(LoadEvent::PropertyLoad {
            property: PropertyRef::Registered(slot),
        });
        let flag = get_flag(node, "is_technology", pc)?;
        self.properties[slot as usize].is_technology = Some(flag);
        Ok(())
    }

    /// 48D2D0 for a translation naming one of these managers.
    fn translate(&mut self, node: &ResolvedNode<'_>, at: RootPosition, pc: Precision) -> Result<()> {
        let manager = get_string(node, "node", pc)?;
        let old = get_string(node, "ids_old", pc)?;
        let new = get_string(node, "ids_new", pc)?;
        let kind = MANAGERS
            .iter()
            .position(|m| m.eq_ignore_ascii_case(&manager))
            .ok_or_else(|| unsupported("translation manager"))?;
        let registry = self.registry(kind);
        let k = old.to_ascii_lowercase();
        if registry.translations.iter().any(|(o, ..)| *o == k) {
            return Err(unsupported(format!(
                "duplicate {manager} translation {old}: 48D2D0 message format not ported"
            )));
        }
        registry.translations.push((k, new.clone(), at));
        if registry.find(&new, None).is_none() {
            return Err(unsupported(format!(
                "{manager} translation target {new} missing: 48D2D0 message format not ported"
            )));
        }
        Ok(())
    }

    fn new_embedded(&mut self, owner: EmbeddedOwner, node: &ResolvedNode<'_>) -> Result<PropertyRef> {
        let index = ordinal(self.embedded_properties.len())?;
        self.embedded_properties.push(EmbeddedProperty {
            owner,
            sources: node.sources(),
        });
        let property = PropertyRef::Embedded(index);
        self.events.push(LoadEvent::PropertyLoad { property });
        Ok(property)
    }

    /// 4C34A0 KKC_FormationData::LoadData.
    fn load_formation(
        &mut self,
        slot: usize,
        node: &ResolvedNode<'_>,
        at: RootPosition,
        directory: &[u16],
        pc: Precision,
    ) -> Result<()> {
        // 47D960.
        self.formations[slot].name = get_localized(node, "name")?;
        self.formations[slot].named = true;
        self.formations[slot].organized = get_flag(node, "organized", pc)?;
        self.formations[slot].herding = get_flag(node, "herding", pc)?;
        self.formations[slot].aggressive = get_flag(node, "aggressive", pc)?;
        let icon = get_string(node, "icon", pc)?;
        let path = native_icon_path(directory, &wide(&icon));
        self.events.push(LoadEvent::Texture { path: path.clone() });
        self.formations[slot].icon = Some(path);
        let group = get_string(node, "group_formation_ids", pc)?;
        if !group.is_empty() {
            match self.group_registry.find(&group, Some(at)) {
                Some(g) => self.formations[slot].group_formation = Some(g),
                None => {
                    let e = throw(f::UNKNOWN_GROUP, vec![text(&group), text(&self.formations[slot].ids)]);
                    self.events.push(throw_event(&e));
                    return Err(e);
                }
            }
        }
        // 5B4410("Property", 0).
        if let Some(child) = node.child("Property")? {
            let property = self.new_embedded(EmbeddedOwner::Formation(slot as u32), &child)?;
            self.formations[slot].property = Some(property);
        }
        Ok(())
    }

    fn property_ids(&self, property: PropertyRef) -> Result<String> {
        match property {
            PropertyRef::Registered(i) => Ok(self.properties[i as usize].ids.clone()),
            PropertyRef::Embedded(_) => Err(unsupported("embedded property has no registry IDS")),
        }
    }

    fn raise(&mut self, format: &'static str, args: Vec<Arg>) -> LayoutError {
        let e = throw(format, args);
        self.events.push(throw_event(&e));
        e
    }

    /// 4C3610 KKC_OrganizationLayoutData::LoadData.
    fn load_layout(&mut self, slot: usize, node: &ResolvedNode<'_>, at: RootPosition, pc: Precision) -> Result<()> {
        let layout_ids = self.layouts[slot].ids.clone();
        self.layouts[slot].name = get_localized(node, "name")?;
        self.layouts[slot].named = true;
        self.layouts[slot].primary_slot = -1;
        self.layouts[slot].morale_count = 0;
        for child in node.children("Slot")? {
            self.layouts[slot].entries.push(LayoutSlotEntry::default());
            let e = self.layouts[slot].entries.len() - 1;
            let property_ids = get_string(&child, "property_IDS", pc)?;
            let property = self.property_registry.find(&property_ids, Some(at)).map(PropertyRef::Registered);
            self.layouts[slot].entries[e].property = property;
            if property.is_none() {
                return Err(self.raise(f::UNKNOWN_SLOT_PROPERTY, vec![text(&property_ids), text(&layout_ids)]));
            }
            self.layouts[slot].entries[e].name = get_localized(&child, "name")?;
            self.layouts[slot].entries[e].required = get_flag(&child, "required", pc)?;
            self.layouts[slot].entries[e].locked = get_flag(&child, "locked", pc)?;
            let number = get_int(&child, "number", pc)?;
            self.layouts[slot].entries[e].number = number;
            let morale = get_flag(&child, "morale_line", pc)?;
            self.layouts[slot].entries[e].morale_line = morale;
            if morale {
                let l = &mut self.layouts[slot];
                l.morale_count = l.morale_count.wrapping_add(number);
            }
            if number > self.max_slot_number {
                self.max_slot_number = number;
            }
            if number > 0 {
                let count = number as usize;
                let groups = &mut self.layouts[slot].slot_groups;
                groups
                    .try_reserve(count)
                    .map_err(|_| unsupported("slot group allocation failed"))?;
                let group = u32::try_from(e).map_err(|_| unsupported("entry index"))?;
                groups.extend(std::iter::repeat_n(group, count));
            }
            if get_flag(&child, "primary", pc)? {
                if self.layouts[slot].primary_slot >= 0 {
                    return Err(self.raise(f::TWO_PRIMARY, vec![text(&layout_ids)]));
                }
                let count = i32::try_from(self.layouts[slot].slot_groups.len())
                    .map_err(|_| unsupported("slot count outside the native signed word"))?;
                self.layouts[slot].primary_slot = count - 1;
            }
        }
        let entry_count = i32::try_from(self.layouts[slot].entries.len())
            .map_err(|_| unsupported("entry count outside the native signed word"))?;
        if entry_count > self.max_slot_groups {
            self.max_slot_groups = entry_count;
        }
        for child in node.children("Formation")? {
            let handle = ordinal(self.layout_formations.len())?
                .checked_add(1)
                .ok_or_else(|| unsupported("formation handle space exhausted"))?;
            let index = ordinal(self.layouts[slot].formations.len())?;
            self.layout_formations.push((slot as u32, index));
            self.layouts[slot].formations.push(LayoutFormation {
                handle,
                formation: None,
                positions: Vec::new(),
            });
            self.load_layout_formation(slot, index as usize, &child, at, pc)?;
            if !self.check_formation_match(slot, index as usize)? {
                let fd = self.layouts[slot].formations[index as usize]
                    .formation
                    .ok_or_else(|| unsupported("formation without data"))?;
                let fids = self.formations[fd as usize].ids.clone();
                return Err(self.raise(f::INCOMPATIBLE, vec![text(&fids), text(&layout_ids)]));
            }
        }
        if self.layouts[slot].formations.is_empty() {
            return Err(self.raise(f::NEEDS_FORMATION, vec![text(&layout_ids)]));
        }
        let Some(broken) = node.child("BrokenProperty")? else {
            return Err(self.raise(f::MISSING_BROKEN, vec![text(&layout_ids)]));
        };
        let property = self.new_embedded(EmbeddedOwner::LayoutBroken(slot as u32), &broken)?;
        self.layouts[slot].broken_property = Some(property);
        self.layouts[slot].default_formation = 0;
        let default = get_string(node, "default_formation_ids", pc)?;
        if !default.is_empty() {
            self.layouts[slot].default_formation = self.formation_index(slot, &default, at)?;
        }
        self.layouts[slot].combat_formation = 0;
        let combat = get_string(node, "combat_formation_IDS", pc)?;
        if !combat.is_empty() {
            self.layouts[slot].combat_formation = self.formation_index(slot, &combat, at)?;
        }
        let retreat = get_string(node, "retreat_formation_IDS", pc)?;
        if !retreat.is_empty() {
            self.layouts[slot].retreat_formation = self.formation_index(slot, &retreat, at)?;
        } else {
            let organized = self.layouts[slot].formations.iter().rposition(|lf| {
                lf.formation
                    .is_some_and(|fd| self.formations[fd as usize].organized)
            });
            if let Some(i) = organized {
                self.layouts[slot].retreat_formation = i as i32;
            }
        }
        Ok(())
    }

    /// 4C32B0 LayoutFormation load.
    fn load_layout_formation(
        &mut self,
        slot: usize,
        index: usize,
        node: &ResolvedNode<'_>,
        at: RootPosition,
        pc: Precision,
    ) -> Result<()> {
        let layout_ids = self.layouts[slot].ids.clone();
        let ids = get_string(node, "formation_IDS", pc)?;
        let fd = self.formation_registry.find(&ids, Some(at));
        self.layouts[slot].formations[index].formation = fd;
        if fd.is_none() {
            return Err(self.raise(f::UNKNOWN_FORMATION, vec![text(&ids), text(&layout_ids)]));
        }
        for child in node.children("Position")? {
            self.layouts[slot].formations[index]
                .positions
                .push(FormationPosition::default());
            let p = self.layouts[slot].formations[index].positions.len() - 1;
            let property_ids = get_string(&child, "property_IDS", pc)?;
            let property = self.property_registry.find(&property_ids, Some(at)).map(PropertyRef::Registered);
            self.layouts[slot].formations[index].positions[p].property = property;
            if property.is_none() {
                return Err(self.raise(f::UNKNOWN_POSITION_PROPERTY, vec![text(&property_ids), text(&layout_ids)]));
            }
            self.layouts[slot].formations[index].positions[p].offset_bits = get_point(&child, "offset", pc)?;
        }
        Ok(())
    }

    /// 4C1BE0 CheckFormationMatch (logs through 404ED0, OrganizationData.cpp).
    fn check_formation_match(&mut self, slot: usize, index: usize) -> Result<bool> {
        let log = self.formation_mismatch(slot, index)?;
        let matched = log.is_none();
        if let Some(event) = log {
            self.events.push(event);
        }
        Ok(matched)
    }
    fn formation_mismatch(&self, slot: usize, index: usize) -> Result<Option<LoadEvent>> {
        let layout = &self.layouts[slot];
        let lf = &layout.formations[index];
        let fd = lf.formation.ok_or_else(|| unsupported("formation without data"))?;
        let fids = &self.formations[fd as usize].ids;
        let slot_count = layout.slot_groups.len();
        if lf.positions.len() != slot_count {
            return Ok(Some(LoadEvent::Log {
                line: 0xcb,
                format: f::SIZE_LOG,
                args: vec![
                    text(fids),
                    Arg::Int(lf.positions.len() as i32),
                    text(&layout.ids),
                    Arg::Int(slot_count as i32),
                ],
            }));
        }
        for i in 0..slot_count {
            let group = layout.slot_groups[i] as usize;
            let expected = layout.entries[group].property;
            let actual = lf.positions[i].property;
            if expected != actual {
                let (Some(expected), Some(actual)) = (expected, actual) else {
                    return Err(unsupported("null property compared in 4C1BE0"));
                };
                return Ok(Some(LoadEvent::Log {
                    line: 0xd5,
                    format: f::PROPERTY_LOG,
                    args: vec![
                        Arg::Int(i as i32),
                        text(fids),
                        text(&self.property_ids(actual)?),
                        text(&layout.ids),
                        text(&self.property_ids(expected)?),
                    ],
                }));
            }
        }
        Ok(None)
    }

    /// 4C3110: index of a formation in the layout's `+34` list.
    fn formation_index(&mut self, slot: usize, name: &str, at: RootPosition) -> Result<i32> {
        let layout_ids = self.layouts[slot].ids.clone();
        let Some(fd) = self.formation_registry.find(name, Some(at)) else {
            return Err(self.raise(f::UNRECOGNIZED, vec![text(&layout_ids), text(name)]));
        };
        match self.layouts[slot]
            .formations
            .iter()
            .position(|lf| lf.formation == Some(fd))
        {
            Some(i) => Ok(i as i32),
            None => Err(self.raise(f::NOT_USED, vec![text(&layout_ids), text(name)])),
        }
    }

    // ------------------------------------------------------------ lookups

    /// 463FA0 on `[B3AB00]` after the whole load (translations applied).
    pub fn find_layout(&self, ids: &str) -> Option<&OrganizationLayoutData> {
        self.layout_registry
            .find(ids, None)
            .map(|s| &self.layouts[s as usize])
    }
    /// 463FA0 on `[B3AB00]` as seen by a root loading at `at`.
    pub fn find_layout_at(&self, ids: &str, at: RootPosition) -> Option<&OrganizationLayoutData> {
        self.layout_registry
            .find(ids, Some(at))
            .map(|s| &self.layouts[s as usize])
    }
    /// 48D500 on GameData+474.
    pub fn find_formation(&self, ids: &str) -> Option<&FormationData> {
        self.formation_registry
            .find(ids, None)
            .map(|s| &self.formations[s as usize])
    }
    /// 463FA0 on GameData+464 (registered Property definitions).
    pub fn find_property(&self, ids: &str, at: Option<RootPosition>) -> Option<PropertyRef> {
        self.property_registry.find(ids, at).map(PropertyRef::Registered)
    }
    pub fn layout_formation(&self, handle: u32) -> Option<&LayoutFormation> {
        let (layout, index) = *self.layout_formations.get(handle.checked_sub(1)? as usize)?;
        self.layouts.get(layout as usize)?.formations.get(index as usize)
    }
    /// What `formation_facts(handle)` reads natively: LayoutFormation `+C`
    /// FormationData `+1A` (aggressive) and `+24` (property handle, 0 = none).
    pub fn formation_facts(&self, handle: u32) -> Result<(u8, u32)> {
        let lf = self
            .layout_formation(handle)
            .ok_or_else(|| unsupported(format!("unknown LayoutFormation handle {handle}")))?;
        let fd = lf
            .formation
            .ok_or_else(|| unsupported("LayoutFormation +C is null (its load threw)"))?;
        let data = &self.formations[fd as usize];
        let property = match data.property {
            Some(p) => p.handle()?,
            None => 0,
        };
        Ok((u8::from(data.aggressive), property))
    }

    /// The fields the Organization/Transport loaders read from a
    /// KKC_OrganizationLayoutData (aa_actors::organization_state::LayoutInfo).
    /// A layout whose LoadData threw is an explicit error (its partial
    /// object exists natively but nothing downstream is proven on it).
    pub fn layout_info(&self, layout: &OrganizationLayoutData) -> Result<LayoutInfoFields> {
        if let Some(e) = &layout.error {
            return Err(unsupported(format!(
                "layout {} was only partially loaded natively: {e}",
                layout.ids
            )));
        }
        let mut entries = Vec::with_capacity(layout.entries.len());
        for e in &layout.entries {
            let property = e
                .property
                .ok_or_else(|| unsupported("layout entry without property"))?;
            entries.push(LayoutEntryFields {
                property: property.handle()?,
                property_name: self.property_ids(property)?,
                number: e.number,
                required: e.required,
                locked: e.locked,
                morale_line: e.morale_line,
            });
        }
        Ok(LayoutInfoFields {
            name: layout.ids.clone(),
            primary_slot: layout.primary_slot,
            entries,
            slot_groups: layout.slot_groups.clone(),
            formations: layout.formations.iter().map(|lf| lf.handle).collect(),
            formation_40: layout.default_formation,
            formation_44: layout.combat_formation,
            formation_48: layout.retreat_formation,
            morale_count_4c: layout.morale_count,
            property_50: layout
                .broken_property
                .ok_or_else(|| unsupported("layout without BrokenProperty"))?
                .handle()?,
        })
    }
}

fn throw_event(e: &LayoutError) -> LoadEvent {
    match e {
        LayoutError::Native(NativeThrow::Data { format, args }) => LoadEvent::Throw {
            format,
            args: args.clone(),
        },
        _ => unreachable!("only Data throws are recorded as 410C40 events"),
    }
}

/// One `+1C` entry as LayoutInfo reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LayoutEntryFields {
    /// `+4` Property handle ([`PropertyRef::handle`]) and its `+8` IDS.
    pub property: u32,
    pub property_name: String,
    pub number: i32,
    pub required: bool,
    pub locked: bool,
    pub morale_line: bool,
}
/// `aa_actors::organization_state::LayoutInfo` field for field, plus `+40`
/// and `+4C`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LayoutInfoFields {
    /// `+8` IDS.
    pub name: String,
    /// `+18`.
    pub primary_slot: i32,
    pub entries: Vec<LayoutEntryFields>,
    /// `+28` entry index of every slot.
    pub slot_groups: Vec<u32>,
    /// `+34` LayoutFormation handles.
    pub formations: Vec<u32>,
    pub formation_40: i32,
    pub formation_44: i32,
    pub formation_48: i32,
    pub morale_count_4c: i32,
    /// `+50` BrokenProperty handle.
    pub property_50: u32,
}

// ------------------------------------------------- ActorData organization

/// ActorData `+1D4` (4C4CF0 over `PropertiesComponent` `Property`
/// children at the actor's load position): registered properties, head
/// first (every lookup is prepended).
pub fn native_actor_properties(
    catalog: &ActorCatalog,
    data: &NativeLayoutData,
    actor_id: &str,
    pc: Precision,
) -> Result<Vec<PropertyRef>> {
    let actor = catalog.resolved_actor(actor_id)?;
    let at = actor_position(catalog, &actor)?;
    let mut list = Vec::new();
    if let Some(properties) = actor.child("PropertiesComponent")? {
        for child in properties.children("Property")? {
            let ids = get_string(&child, "property_IDS", pc)?;
            match data.find_property(&ids, Some(at)) {
                Some(p) => list.insert(0, p),
                None => return Err(throw(f::UNKNOWN_PROPERTY, vec![text(&ids)])),
            }
        }
    }
    Ok(list)
}

fn actor_position(catalog: &ActorCatalog, actor: &ResolvedNode<'_>) -> Result<RootPosition> {
    let src = actor
        .sources()
        .into_iter()
        .next()
        .ok_or_else(|| unsupported("actor without source"))?;
    let file = catalog
        .sources
        .iter()
        .position(|s| s.path.eq_ignore_ascii_case(&src.path))
        .ok_or_else(|| unsupported("actor source is not a catalog file"))?;
    Ok(RootPosition {
        file,
        offset: src.span.start,
    })
}

/// ActorData OrganizationComponent fields (464BD0, 466C4F..46707C) and the
/// default layout `+2F0` (4C2110; parameters after 4C2310 and the deferred
/// element links 460000).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OrganizationComponentData {
    /// ActorData `+8` IDS.
    pub actor: String,
    /// `+2F4` supply_used.
    pub supply_used: bool,
    /// `+2F8` shortfall_damage_rate.
    pub shortfall_damage_rate: u32,
    /// `+320` company_group_priority.
    pub company_group_priority: i32,
    /// `+308` morale_breakpoint.
    pub morale_breakpoint: u32,
    /// `+2FC` control_range.
    pub control_range: u32,
    /// `+300` operational_range and `+304` its square (x87 at `pc`).
    pub operational_range: u32,
    pub operational_range_squared: u32,
    /// `+2E8` OrganizationLayoutData ordinal and IDS.
    pub layout: u32,
    pub layout_ids: String,
    /// `+2EC` entrenchment_time.
    pub entrenchment_time: u32,
    /// `+310` rampages.
    pub rampages: bool,
    /// `[+314]` +0 wait chance, +4 points to try, +8/+C direction (degrees
    /// times binary32 0x3C8EFA35 at `pc`), +10/+14 distance, +18 maximum
    /// detached ratio, +1C aggressive.
    pub rampage_wait_chance: u32,
    pub rampage_points_to_try: i32,
    pub rampage_direction: [u32; 2],
    pub rampage_distance: [u32; 2],
    pub rampage_max_detached_ratio: u32,
    pub rampage_aggressive: bool,
    /// `+318` recruit_time, `+31C` recruit_full, `+31D` in_supply_detached.
    pub recruit_time: u32,
    pub recruit_full: bool,
    pub in_supply_detached: bool,
    /// `+2F0` KKC_OrganizationLayout `+C`: one ActorData per layout entry
    /// (canonical Thing IDS; None = null).
    pub parameters: Vec<Option<String>>,
    /// Deferred element references (473AD0, GameData+824) in creation
    /// order: (slot, authored IDS). 475230 walks them head first, i.e. in
    /// reverse creation order, after every data file is loaded.
    pub elements: Vec<(u32, String)>,
}

fn store(value: &Binary) -> Result<u32> {
    value
        .store_f32()
        .map_err(|e| unsupported(format!("binary32 store outside the finite domain: {e}")))
}
fn number(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| unsupported(format!("non-finite x87 operand: {e}")))
}
fn x87(e: aa_numeric::Error) -> LayoutError {
    unsupported(format!("x87 arithmetic outside the finite domain: {e}"))
}
/// `fcomp` against +0.0: (less, equal); a NaN is unordered (neither).
fn sign_of(bits: u32) -> (bool, bool) {
    if bits & 0x7fff_ffff > 0x7f80_0000 {
        return (false, false);
    }
    let zero = bits & 0x7fff_ffff == 0;
    (!zero && bits & 0x8000_0000 != 0, zero)
}
/// binary32 at 0x9FC064 (pi/180).
pub const DEGREES_TO_RADIANS_9FC064: u32 = 0x3c8e_fa35;

/// 460000 tail: every owner `+17C` entry missing from the element's `+17C`
/// list is prepended to it in owner order (the element `+180` tail is set
/// by the first insert into an empty list). Lists are head first.
pub fn merge_list_17c(owner: &[u32], element: &[u32]) -> Vec<u32> {
    let mut list = element.to_vec();
    for &item in owner {
        if !list.contains(&item) {
            list.insert(0, item);
        }
    }
    list
}

/// Facts of an element ActorData read by the link 460000.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ElementFacts {
    /// Canonical IDS of the definition 4BDBA0 returned.
    pub definition: String,
    /// `+174` bit 8 (ElementComponent).
    pub is_element: bool,
    /// `+1D4` property list.
    pub properties: Vec<PropertyRef>,
    /// `+174` bit 19 (MobileDeploymentComponent).
    pub mobile_deployment: bool,
    /// `+174` bit 21 (ResearchComponent) and `+558` nonnull (ResearchItem list).
    pub research: bool,
    pub research_items: bool,
}

impl NativeLayoutData {
    /// 466C4F..46707C on an `OrganizationComponent` node: fields, checks,
    /// layout lookup (as of `at`; None = after the whole load) and the
    /// default layout before its element links.
    pub fn organization_component_fields(
        &self,
        org: &ResolvedNode<'_>,
        actor_ids: &str,
        at: Option<RootPosition>,
        pc: Precision,
    ) -> Result<OrganizationComponentData> {
        let ids = actor_ids;
        let supply_used = get_flag(org, "supply_used", pc)?;
        let shortfall_damage_rate = get_fixed(org, "shortfall_damage_rate", pc)?;
        let company_group_priority = get_int(org, "company_group_priority", pc)?;
        let morale_breakpoint = get_fixed(org, "morale_breakpoint", pc)?;
        if sign_of(morale_breakpoint).0 {
            return Err(throw(f::MORALE_BREAKPOINT, vec![text(ids)]));
        }
        let control_range = get_fixed(org, "control_range", pc)?;
        let (less, equal) = sign_of(control_range);
        if less || equal {
            return Err(throw(f::CONTROL_RANGE, vec![text(ids)]));
        }
        let operational_range = get_fixed(org, "operational_range", pc)?;
        let (less, equal) = sign_of(operational_range);
        if less || equal {
            return Err(throw(f::OPERATIONAL_RANGE, vec![text(ids)]));
        }
        // 466D3D: fld; fld st0; fmul st(1); fstp dword +304.
        let op = number(operational_range)?;
        let operational_range_squared = store(&op.mul(&op, pc).map_err(x87)?)?;
        let layout_text = get_string(org, "layout_IDS", pc)?;
        let found = match at {
            Some(at) => self.find_layout_at(&layout_text, at),
            None => self.find_layout(&layout_text),
        };
        let layout = found.ok_or_else(|| throw(f::UNKNOWN_LAYOUT, vec![text(&layout_text), text(ids)]))?;
        let entrenchment_time = get_fixed(org, "entrenchment_time", pc)?;
        let rampages = get_flag(org, "rampages", pc)?;
        let rampage_wait_chance = get_fixed(org, "rampage_wait_chance", pc)?;
        let rampage_points_to_try = get_int(org, "rampage_points_to_try", pc)?;
        let direction = get_point(org, "rampage_direction", pc)?;
        let rampage_distance = get_point(org, "rampage_distance", pc)?;
        let rampage_max_detached_ratio = get_fixed(org, "rampage_max_detached_ratio", pc)?;
        let rampage_aggressive = get_flag(org, "rampage_aggressive", pc)?;
        // 466E55..466E7D: fld; fmul dword [9FC064]; fstp (x then y).
        let scale = number(DEGREES_TO_RADIANS_9FC064)?;
        let mut rampage_direction = [0; 2];
        for (out, bits) in rampage_direction.iter_mut().zip(direction) {
            *out = store(&number(bits)?.mul(&scale, pc).map_err(x87)?)?;
        }
        let recruit_time = get_fixed(org, "recruit_time", pc)?;
        let recruit_full = get_flag(org, "recruit_full", pc)?;
        let in_supply_detached = get_flag(org, "in_supply_detached", pc)?;
        // 4C2110: one null parameter per LayoutData entry (+20).
        let entry_count = layout.entries.len();
        let parameters: Vec<Option<String>> = vec![None; entry_count];
        let mut elements = Vec::new();
        let mut i = 0usize;
        for child in org.children("Slot")? {
            if i >= entry_count {
                return Err(throw(f::TOO_MANY_SLOTS, vec![text(ids)]));
            }
            let entry = &layout.entries[i];
            if entry.locked && !entry.required {
                return Err(throw(f::LOCKED_NOT_REQUIRED, vec![Arg::Int(i as i32)]));
            }
            let element = get_string(&child, "element_IDS", pc)?;
            if element.is_empty() {
                // 4C2310(i, null) stores null.
                if entry.required {
                    return Err(throw(f::REQUIRED_ELEMENT, vec![Arg::Int(i as i32), text(ids)]));
                }
            } else {
                elements.push((i as u32, element));
            }
            i += 1;
        }
        while i < entry_count {
            if layout.entries[i].required {
                return Err(throw(f::REQUIRED_ELEMENT, vec![Arg::Int(i as i32), text(ids)]));
            }
            i += 1;
        }
        Ok(OrganizationComponentData {
            actor: ids.to_owned(),
            supply_used,
            shortfall_damage_rate,
            company_group_priority,
            morale_breakpoint,
            control_range,
            operational_range,
            operational_range_squared,
            layout: layout.ordinal,
            layout_ids: layout.ids.clone(),
            entrenchment_time,
            rampages,
            rampage_wait_chance,
            rampage_points_to_try,
            rampage_direction,
            rampage_distance,
            rampage_max_detached_ratio,
            rampage_aggressive,
            recruit_time,
            recruit_full,
            in_supply_detached,
            parameters,
            elements,
        })
    }

    /// 460000 for one deferred element reference of `owner`
    /// (`element` = what 4BDBA0(ids, 0x2B) returned).
    pub fn link_element(
        &self,
        owner: &mut OrganizationComponentData,
        owner_hq: bool,
        slot: u32,
        element_ids: &str,
        element: Option<&ElementFacts>,
    ) -> Result<()> {
        let layout = &self.layouts[owner.layout as usize];
        let index = slot as usize;
        let entry = layout
            .entries
            .get(index)
            .ok_or_else(|| unsupported("element slot outside the layout entries"))?;
        let property = entry
            .property
            .ok_or_else(|| unsupported("layout entry without property"))?;
        // 4C2310(slot, def): a definition without the slot property (45EEF0)
        // is not stored; null is stored.
        let has_property = element.is_some_and(|e| e.properties.contains(&property));
        if element.is_none() || has_property {
            owner.parameters[index] = element.map(|e| e.definition.clone());
        }
        let Some(element) = element else {
            return Err(throw(f::UNKNOWN_ELEMENT, vec![text(element_ids), text(&owner.actor)]));
        };
        if !element.is_element {
            return Err(throw(f::NOT_ELEMENT, vec![text(element_ids), text(&owner.actor)]));
        }
        if !has_property {
            return Err(throw(
                f::LACKS_PROPERTY,
                vec![text(element_ids), text(&self.property_ids(property)?), text(&owner.actor)],
            ));
        }
        if element.mobile_deployment && entry.number > 1 {
            return Err(throw(f::PACKABLE, vec![text(&owner.actor), text(element_ids)]));
        }
        // (The owner +17C list merge into the element is not modeled here.)
        if owner_hq && element.research && element.research_items {
            return Err(throw(f::HQ_RESEARCH, vec![text(&owner.actor), text(element_ids)]));
        }
        Ok(())
    }

    /// The element facts 460000 reads, from the catalog: 4BDBA0(ids, 0x2B)
    /// (427BD0 with translations, then virtual +38: 1 for KKC_ActorData; any
    /// other found category is unported).
    pub fn element_facts(
        &self,
        catalog: &ActorCatalog,
        element_ids: &str,
        pc: Precision,
    ) -> Result<Option<ElementFacts>> {
        let Some(found) = catalog.find(element_ids) else {
            return Ok(None);
        };
        let definition = found.id.clone();
        if catalog.native_category(&definition)?.native_kind != 2 {
            return Err(unsupported(format!(
                "4BDBA0 virtual +38 for non-ACTOR definition {definition} is unported"
            )));
        }
        let node = catalog.resolved_actor(&definition)?;
        let research = node.child("ResearchComponent")?;
        Ok(Some(ElementFacts {
            properties: native_actor_properties(catalog, self, &definition, pc)?,
            is_element: node.child("ElementComponent")?.is_some(),
            mobile_deployment: node.child("MobileDeploymentComponent")?.is_some(),
            research_items: match &research {
                Some(r) => !r.children("ResearchItem")?.is_empty(),
                None => false,
            },
            research: research.is_some(),
            definition,
        }))
    }

    /// The whole OrganizationComponent of an actor definition (None: no
    /// such component): fields at the actor's load position, then its
    /// element links in the native walk order (reverse creation).
    pub fn organization_component(
        &self,
        catalog: &ActorCatalog,
        actor_id: &str,
        pc: Precision,
    ) -> Result<Option<OrganizationComponentData>> {
        let actor = catalog.resolved_actor(actor_id)?;
        let Some(org) = actor.child("OrganizationComponent")? else {
            return Ok(None);
        };
        let at = actor_position(catalog, &actor)?;
        let ids = catalog
            .find(actor_id)
            .map(|a| a.id.clone())
            .ok_or_else(|| unsupported(format!("missing actor {actor_id}")))?;
        let mut data = self.organization_component_fields(&org, &ids, Some(at), pc)?;
        let owner_hq = actor.child("HQComponent")?.is_some();
        for (slot, element) in data.elements.clone().iter().rev() {
            let facts = self.element_facts(catalog, element, pc)?;
            self.link_element(&mut data, owner_hq, *slot, element, facts.as_ref())?;
        }
        Ok(Some(data))
    }
}

impl NativeLayoutData {
    /// 45EEF0 (`ActorData::HasProperty`): the actor definition's `+1D4`
    /// list holds the property with this handle.
    pub fn actor_has_property(
        &self,
        catalog: &ActorCatalog,
        actor_id: &str,
        property_handle: u32,
        pc: Precision,
    ) -> Result<bool> {
        let wanted = PropertyRef::from_handle(property_handle)
            .ok_or_else(|| unsupported("45EEF0 on a null property handle"))?;
        Ok(native_actor_properties(catalog, self, actor_id, pc)?.contains(&wanted))
    }
}

impl ActorCatalog {
    /// [`NativeLayoutData::load`] over this catalog.
    pub fn native_layout_data(&self, pc: Precision) -> Result<NativeLayoutData> {
        NativeLayoutData::load(self, pc)
    }
}
