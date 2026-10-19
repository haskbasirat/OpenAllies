//! Original AA.exe definition load order for the Thing registry.
//!
//! This replaces "every TGI member, any order" with the native sequence:
//! - files: the index chain of 48DB60/5AB590 (`crate::native::index`), each
//!   parsed by 5BD080 while the global template registry ([B39BC8]) is live;
//! - parse-time template binding: a `Template=`/`inherit=` reference is
//!   resolved when its node is read (5BF0A0 -> 4247C0, case-insensitive on
//!   the node's CURRENT name); forward references and duplicate registrations
//!   are parse errors ("Template definition not found." /
//!   "Template was already defined.", 5BACE4/5BCA42);
//! - sibling name collisions rename nodes to `?%07d_<name>` (5B5C80/42BC40),
//!   which makes an already registered template unreachable by its name;
//! - root processing 48D660: non-template roots in order, dispatched by name
//!   (`?`-renamed roots by their label); the first TGC exception (caught at
//!   48D824/48D886/48D8F0) abandons the rest of that file and keeps it loaded;
//! - 48D970: a file that processed at least one non-template root without an
//!   exception is released (5AA550 -> 5BCDD0 -> 5B3FE0 -> 5BF080), which
//!   unregisters every template it declared; other files stay loaded;
//! - Thing/Unit/Projectile/VFX roots go to the Thing DataManager (vtable
//!   A21260): 4BDC50 reads IDS and category (48D500, "Unknown thing
//!   category"), 4BCA00 throws "ThingData name already used: %s" for an IDS
//!   already registered, so the FIRST declaration wins;
//! - Translate roots go to 48D2D0: the named DataManager must exist, a
//!   repeated `ids_old` throws, the pair is inserted, then the target is
//!   checked with the manager's Find (Thing: 427BD0) and a missing target
//!   throws AFTER insertion. 427BD0 consults the translation map first and
//!   then looks up the translated IDS without any further fallback.
//!
//! Per-definition LoadData validation (4BB310, 49DC40, 464BD0, ...) is not
//! re-run here. Native v14 runs log every 48D660 catch on channel B38B70
//! ("KKC_Data::LoadDataFile", level 0, the channel that records the level-10
//! "loading index" line); none appear in the recorded native logs, so no
//! LoadData exception truncates a shipped file. The model reports any
//! exception it predicts itself as a diagnostic.
use super::super::{
    ActorCatalog, ActorDefinition, Definition, Diagnostic, FIELDS, RawField, Resolver, definition,
    key,
};
use crate::{
    resources::SourceIdentity,
    sources::{SourceError, V14Sources, sha256},
    tgi::{Document, Item, Limits},
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Root node names the Thing DataManager handles (48DB60 registers "Thing",
/// "Unit", "Projectile" and "VFX" to GameData+C).
pub const THING_TAGS: &[&str] = &["Thing", "Unit", "Projectile", "VFX"];

/// Root tags in the GameData+834 handler map: 48DB60 registers these (static
/// scan of 48DC00..4901DC), then 49006C..4900BD merges the MetaGameData
/// handler map (GameData+820, built by 93E2B0) — the last seven tags.
pub const KNOWN_ROOT_TAGS: &[&str] = &[
    "Text", "ThingCategory", "Set", "Font", "TriggerElement", "Glyph", "TargetType",
    "AbilityType", "EnchantmentType", "DamageType", "Nation", "Faction", "WorldRule",
    "DifficultyLevel", "PlacementGroup", "Resource", "Equipment", "EquipmentType",
    "TeamCommand", "ActorCommand", "ActorState", "WallSet", "KingdomColor", "Handicap",
    "Property", "AbilityDefinition", "Thing", "Unit", "Projectile", "VFX",
    "SettlementLayout", "OrganizationLayout", "Formation", "GroupFormation", "TerrainGroup",
    "Biome", "Terrain", "GameType", "SVars", "TransitionSet", "MessageType", "Cursor",
    "Banner", "SAI", "Ego", "SettlementTemplate", "CompanyTemplate", "Statistic",
    "ScoringTab", "ScoringPanel", "Bone", "Audio", "Resolution", "GameEvent",
    "GraphicsPreset", "RMC", "Campaign", "Minimap", "ConnectionType", "DenizenType",
    "NameList", "Weather", "Sun", "Credits", "Description", "Translate",
    "HotkeyConfigurationData", "PlayerRank", "Variation",
    // 93E2B0 (MetaGameData) handler map, merged at 4900A7.
    "MGAI", "MGMap", "MGMapTable", "MGDivision", "MGCommand", "DynamicCampaign", "MGHelpPopup",
];

/// Root tags whose GameData+834 handler is a generic DataManager<T>: vf0
/// 483A40 (node -> vf1C type Load), vf8 6A71D0 (translations allowed) and a
/// 48D500-family Find at vf18 (static join in
/// research/phase8-definitions/tag-managers.json). "Variation" is also fed by
/// TerrainData::LoadData 4B87A0 for every [Variation] child of a Terrain.
/// EnchantmentType's vf18 is the48D4A0 JMP48D500 thunk; its486F00
/// constructor installs A0D410 with the same mode0/translation contract.
pub const GENERIC_DATA_TAGS: &[&str] = &[
    "Set", "Font", "TriggerElement", "Glyph", "EnchantmentType", "DamageType", "Nation", "Faction", "WorldRule",
    "DifficultyLevel", "PlacementGroup", "Resource", "Equipment", "EquipmentType",
    "ActorCommand", "ActorState", "WallSet", "KingdomColor", "Handicap", "AbilityDefinition",
    "SettlementLayout", "OrganizationLayout", "Formation", "GroupFormation", "TerrainGroup",
    "Biome", "Terrain", "GameType", "TransitionSet", "MessageType", "Cursor", "Banner", "Audio",
    "Resolution", "GameEvent", "GraphicsPreset", "RMC", "Campaign", "ConnectionType",
    "DenizenType", "NameList", "Weather", "Sun", "Credits", "Description", "Bone",
    "HotkeyConfigurationData", "PlayerRank", "Variation",
];

/// DataManager mode word (+8, the second argument of 47D880 as passed by the
/// GameData constructor 48A8E0; static scan in tag-managers.json /
/// research/phase8-definitions/README.txt). 0: vf1C never looks an IDS up,
/// so a repeat throws "Data IDS already used" in the create path (4BD270);
/// 1: an existing definition is reused and re-loaded; 2: only existing
/// (code-predefined) definitions may be loaded. `None`: not determined.
pub fn native_manager_mode(tag: &str) -> Option<u32> {
    const MODE_1: &[&str] = &["KingdomColor", "GameType", "Cursor"];
    const MODE_2: &[&str] = &["ActorCommand", "ActorState", "TriggerElement", "MessageType", "GameEvent"];
    const UNKNOWN: &[&str] = &["Glyph", "PlayerRank", "Description"];
    let is = |list: &[&str]| list.iter().any(|t| t.eq_ignore_ascii_case(tag));
    if generic_tag(tag).is_none() || is(UNKNOWN) {
        None
    } else if is(MODE_1) {
        Some(1)
    } else if is(MODE_2) {
        Some(2)
    } else {
        Some(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum NativeLoadEventKind {
    /// 5BD080 parse failure. Fatal for the whole original data load.
    ParseFailure,
    /// "Template definition not found." during parse (fatal in AA.exe).
    TemplateNotFound,
    /// "Template was already defined." during parse (fatal in AA.exe).
    TemplateAlreadyDefined,
    /// A registered template was renamed by a later same-name sibling.
    TemplateHiddenByRename,
    /// A TGC exception in 48D660: the remaining roots of the file are skipped.
    RootException,
    /// Root tag has no handler in the scanned 48DB60 map ("Unknown node type").
    UnknownRootTag,
    /// File processed at least one non-template root: its templates released.
    Released,
    /// File kept loaded (template-only or exception): templates stay live.
    Kept,
    /// A translation for a DataManager whose handler is not a verified
    /// generic DataManager (its registry is not modeled).
    UnmodeledTranslation,
    /// A generic DataManager root whose header/IDS the Rust definition model
    /// cannot read (e.g. `[DenizenType = Template=...]`); not registered.
    UnreadGenericRoot,
    /// Generic duplicate IDS whose first definition had an empty name: the
    /// native type LoadData runs again on the same object (overlay).
    GenericReload,
    /// A mode-2 manager loaded an IDS that is not authored earlier: the
    /// native accepts it only if code predefined it (not modeled here).
    PredefinedAssumed,
    /// A duplicate IDS in a manager whose mode word is not determined.
    UnmodeledDuplicate,
}

#[derive(Clone, Debug, Serialize)]
pub struct NativeLoadEvent {
    pub kind: NativeLoadEventKind,
    pub file: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct NativeTranslation {
    /// DataManager name exactly as authored in `node`.
    pub manager: String,
    pub ids_old: String,
    pub ids_new: String,
    pub source: crate::resources::Provenance,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct NativeLoadReport {
    pub index: crate::native::index::IndexExpansion,
    /// Files actually parsed, in load order.
    pub files: Vec<String>,
    pub events: Vec<NativeLoadEvent>,
    /// Thing DataManager registrations in registration order (IDS, file).
    pub things: Vec<(String, String)>,
    /// Translate roots read, in processing order, for every manager.
    pub translations: Vec<NativeTranslation>,
    /// Final Thing DataManager translation map (old, new) as authored.
    pub thing_translations: Vec<(String, String)>,
    /// Every other dispatched non-template root: (tag, IDS if authored, file).
    pub other_roots: Vec<(String, Option<String>, String)>,
    /// Final translation maps of the generic DataManagers (tag, old, new).
    pub data_translations: Vec<(String, String, String)>,
    /// Registered entries per generic DataManager tag.
    pub data_counts: BTreeMap<String, usize>,
}
impl NativeLoadReport {
    /// Events that the original executable treats as fatal or that truncate
    /// a file. Shipped v14 data produces none in native runs.
    pub fn exceptions(&self) -> impl Iterator<Item = &NativeLoadEvent> {
        self.events.iter().filter(|e| {
            matches!(
                e.kind,
                NativeLoadEventKind::ParseFailure
                    | NativeLoadEventKind::TemplateNotFound
                    | NativeLoadEventKind::TemplateAlreadyDefined
                    | NativeLoadEventKind::RootException
                    | NativeLoadEventKind::UnknownRootTag
            )
        })
    }
}

/// One live template registry entry: the file that declared it and the node
/// path (root index into `Resolver::roots`, then child indices).
#[derive(Clone, Debug)]
struct Registered {
    file: usize,
    path: Vec<usize>,
}

/// Global template registry keyed by the node's current name (lowercase).
#[derive(Default)]
struct TemplateRegistry {
    live: BTreeMap<String, Registered>,
}

fn renamed(ordinal: u32, name: &str) -> String {
    format!("?{ordinal:07}_{name}")
}

struct FileSim<'a> {
    file: usize,
    /// Index in `Resolver::roots` of this file's first root node.
    first_root: usize,
    file_name: &'a str,
    registry: &'a mut TemplateRegistry,
    events: &'a mut Vec<NativeLoadEvent>,
}
impl FileSim<'_> {
    /// Parse-order simulation over one parent's children: bind references,
    /// apply the 5B5C80 sibling renaming, then register templates.
    fn children(&mut self, nodes: &mut [Definition], parent_path: &[usize]) {
        // lowercase name -> (dup counter, index of the first sibling). Siblings
        // copied in from a bound template are not modeled here; that only
        // matters for a nested template named like an inherited child.
        let mut groups: BTreeMap<String, (u32, usize)> = BTreeMap::new();
        let mut current: Vec<String> = Vec::with_capacity(nodes.len());
        for k in 0..nodes.len() {
            let path = self.path(parent_path, k);
            // 1. Header: template reference resolved against the live registry.
            if let Some(base) = nodes[k].base.clone() {
                match self.registry.live.get(&key(&base)) {
                    Some(reg) => nodes[k].bound = Some(reg.path.clone()),
                    None => {
                        let message = format!(
                            "native parse error at {}: Template definition not found ({base})",
                            nodes[k].name
                        );
                        nodes[k].problem.get_or_insert(message.clone());
                        self.events.push(NativeLoadEvent {
                            kind: NativeLoadEventKind::TemplateNotFound,
                            file: self.file_name.into(),
                            detail: message,
                        });
                    }
                }
            }
            // 2. Child insertion with same-name renaming.
            let name_key = key(&nodes[k].name);
            let name = match groups.get_mut(&name_key) {
                None => {
                    groups.insert(name_key, (1, k));
                    nodes[k].name.clone()
                }
                Some((count, first)) => {
                    if *count == 1 {
                        let old = current[*first].clone();
                        let new = renamed(0, &nodes[*first].name);
                        let first_path = self.path(parent_path, *first);
                        self.hide(&old, &new, &first_path);
                        current[*first] = new;
                    }
                    let ordinal = *count;
                    *count += 1;
                    renamed(ordinal, &nodes[k].name)
                }
            };
            current.push(name.clone());
            // 3. Template registration by the node's current name.
            if nodes[k].template {
                let name_key = key(&name);
                if let Some(prior) = self.registry.live.get(&name_key) {
                    let message = format!(
                        "native parse error: Template was already defined ({name}, first in file #{})",
                        prior.file
                    );
                    nodes[k].problem.get_or_insert(message.clone());
                    self.events.push(NativeLoadEvent {
                        kind: NativeLoadEventKind::TemplateAlreadyDefined,
                        file: self.file_name.into(),
                        detail: message,
                    });
                } else {
                    self.registry.live.insert(
                        name_key,
                        Registered {
                            file: self.file,
                            path: path.clone(),
                        },
                    );
                }
            }
            // 4. Body.
            let mut children = std::mem::take(&mut nodes[k].children);
            self.children(&mut children, &path);
            nodes[k].children = children;
        }
    }
    /// Absolute node path: roots are numbered in `Resolver::roots`.
    fn path(&self, parent_path: &[usize], k: usize) -> Vec<usize> {
        if parent_path.is_empty() {
            vec![self.first_root + k]
        } else {
            let mut path = parent_path.to_vec();
            path.push(k);
            path
        }
    }
    /// A renamed template keeps its registry bucket under the old hash but no
    /// longer compares equal to any lookup name (4247C0 compares the current
    /// node name), so it is unreachable from then on.
    fn hide(&mut self, old: &str, new: &str, path: &[usize]) {
        let old_key = key(old);
        if self
            .registry
            .live
            .get(&old_key)
            .is_some_and(|r| r.file == self.file && r.path == path)
        {
            self.registry.live.remove(&old_key);
            self.events.push(NativeLoadEvent {
                kind: NativeLoadEventKind::TemplateHiddenByRename,
                file: self.file_name.into(),
                detail: format!("{old} renamed to {new}"),
            });
        }
    }
}

fn plain_text(field: Option<RawField>) -> Option<String> {
    let raw = field?.value?;
    let text = String::from_utf8(raw).ok()?;
    let text = text.trim();
    Some(if text.eq_ignore_ascii_case("null") {
        String::new()
    } else {
        text.to_owned()
    })
}

/// Thing DataManager registry (+30) plus its translation map (+C, count at
/// +14), with the original insertion/lookup rules. Slots are registration
/// order. Keys compare case-insensitively (_wcsicmp, hash 405030).
#[derive(Clone, Debug, Default)]
pub struct NativeThingRegistry {
    registry: BTreeMap<String, usize>,
    /// key(old) -> (old, new) as authored.
    translations: BTreeMap<String, (String, String)>,
    slots: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum NativeTranslateError {
    /// 48D3E4: "DataManager '%s' has duplicate IDS translation %s -> %s";
    /// thrown before insertion.
    DuplicateOld,
    /// 48D405: "DataManager '%s' does not contain target IDS - loading
    /// translation %s -> %s"; thrown after the pair was inserted.
    MissingTarget,
}
impl NativeThingRegistry {
    /// Original 4BCA00: an IDS already in the registry throws
    /// "ThingData name already used: %s" and nothing is created.
    pub fn register(&mut self, ids: &str) -> Result<usize, String> {
        if self.registry.contains_key(&key(ids)) {
            return Err(format!("ThingData name already used: {ids}"));
        }
        let slot = self.slots;
        self.slots += 1;
        self.registry.insert(key(ids), slot);
        Ok(slot)
    }
    /// Original 427BD0: empty IDS -> null; a translated IDS is looked up
    /// by its target only (no fallback to the old IDS).
    pub fn find(&self, id: &str) -> Option<usize> {
        if id.is_empty() {
            return None;
        }
        let k = key(id);
        if !self.translations.is_empty()
            && let Some((_, new)) = self.translations.get(&k)
        {
            return self.registry.get(&key(new)).copied();
        }
        self.registry.get(&k).copied()
    }
    /// Original 48D2D0 on the Thing DataManager (whose vf8 6A71D0 is true
    /// and vf18 4BDD90 is 427BD0).
    pub fn translate(&mut self, old: &str, new: &str) -> Result<(), NativeTranslateError> {
        if self.translations.contains_key(&key(old)) {
            return Err(NativeTranslateError::DuplicateOld);
        }
        self.translations
            .insert(key(old), (old.to_owned(), new.to_owned()));
        if self.find(new).is_none() {
            return Err(NativeTranslateError::MissingTarget);
        }
        Ok(())
    }
    /// Translation pairs as authored, ordered by lowercase old IDS.
    pub fn translations(&self) -> Vec<(String, String)> {
        self.translations.values().cloned().collect()
    }
}

/// One generic DataManager definition (registry entry +30 of its manager).
#[derive(Clone, Debug, Serialize)]
pub struct NativeDataEntry {
    /// IDS as authored by the registering node.
    pub ids: String,
    /// Node path (root index in the catalog, then child indices).
    pub path: Vec<usize>,
    pub source: crate::resources::Provenance,
    /// The latest LoadData stored a nonempty localized `name` (47D960 then
    /// rejects a later node with the same IDS in mode 1/2 managers).
    pub named: bool,
    /// Later nodes whose type LoadData ran on this same object (only when
    /// the earlier load left the name empty). Overlay field semantics are
    /// type-specific and not modeled; resolution reports them explicitly.
    pub reloads: Vec<Vec<usize>>,
}
/// Generic DataManager<T> registry with the vf1C type-Load rule (47D030,
/// 479080, ... same shape as 4BD1B0) and the 48D500 Find.
#[derive(Clone, Debug, Default, Serialize)]
pub struct NativeDataRegistry {
    pub manager: String,
    /// DataManager +8 mode word (see `native_manager_mode`).
    pub mode: Option<u32>,
    pub entries: Vec<NativeDataEntry>,
    registry: BTreeMap<String, usize>,
    translations: BTreeMap<String, (String, String)>,
}
impl NativeDataRegistry {
    /// vf1C (47D030/479080/... the 4BD1B0 shape) after GetString("IDS"):
    /// mode != 0 looks the IDS up and re-loads an existing definition (the
    /// KKC_Data base LoadData 47D960 throws "loading duplicate node with ids
    /// '%s'" when that definition's stored name is nonempty); otherwise the
    /// create path (4BD270 shape) throws "Data IDS already used: '%s'" for a
    /// registered IDS and, in mode 2, "Unknown data cannot be added to this
    /// data manager: '%s'". Returns (slot, outcome).
    pub fn load(
        &mut self,
        ids: &str,
        named: bool,
        path: Vec<usize>,
        source: crate::resources::Provenance,
    ) -> Result<(usize, LoadOutcome), String> {
        let existing = self.registry.get(&key(ids)).copied();
        match (self.mode, existing) {
            (Some(1 | 2), Some(slot)) => {
                let entry = &mut self.entries[slot];
                if entry.named {
                    return Err(format!("loading duplicate node with ids '{}'", entry.ids));
                }
                entry.reloads.push(path);
                entry.named = named;
                return Ok((slot, LoadOutcome::Reloaded));
            }
            (Some(0), Some(_)) => return Err(format!("Data IDS already used: '{ids}'")),
            (None, Some(slot)) => return Ok((slot, LoadOutcome::UnmodeledDuplicate)),
            _ => {}
        }
        let outcome = if self.mode == Some(2) {
            LoadOutcome::PredefinedAssumed
        } else {
            LoadOutcome::Created
        };
        let slot = self.entries.len();
        self.entries.push(NativeDataEntry {
            ids: ids.to_owned(),
            path,
            source,
            named,
            reloads: Vec::new(),
        });
        self.registry.insert(key(ids), slot);
        Ok((slot, outcome))
    }
    /// Original 48D500 (same algorithm as 427BD0): empty -> null; translation
    /// map first, then the registry with the target only.
    pub fn find(&self, id: &str) -> Option<&NativeDataEntry> {
        if id.is_empty() {
            return None;
        }
        let k = key(id);
        let k = match self.translations.get(&k) {
            Some((_, new)) => key(new),
            None => k,
        };
        self.registry.get(&k).map(|&slot| &self.entries[slot])
    }
    /// Original 48D2D0 on a generic DataManager (vf8 6A71D0, vf18 Find).
    pub fn translate(&mut self, old: &str, new: &str) -> Result<(), NativeTranslateError> {
        if self.translations.contains_key(&key(old)) {
            return Err(NativeTranslateError::DuplicateOld);
        }
        self.translations
            .insert(key(old), (old.to_owned(), new.to_owned()));
        if self.find(new).is_none() {
            return Err(NativeTranslateError::MissingTarget);
        }
        Ok(())
    }
    pub fn translations(&self) -> Vec<(String, String)> {
        self.translations.values().cloned().collect()
    }
    /// Empty registry for a manager with the given mode word.
    pub fn new(manager: &str, mode: Option<u32>) -> Self {
        Self {
            manager: manager.to_owned(),
            mode,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum LoadOutcome {
    Created,
    Reloaded,
    PredefinedAssumed,
    UnmodeledDuplicate,
}

/// The verified generic registry a root/translation tag addresses.
fn generic_tag(tag: &str) -> Option<&'static str> {
    GENERIC_DATA_TAGS
        .iter()
        .find(|t| t.eq_ignore_ascii_case(tag))
        .copied()
}

/// Native-order catalog: only index-listed files, parse-time template
/// binding, first-wins Thing registration and the Thing translation map.
pub fn load(sources: &mut V14Sources) -> Result<(ActorCatalog, NativeLoadReport), SourceError> {
    let resources: Vec<String> = sources
        .catalog()
        .into_iter()
        .map(|e| e.logical_path)
        .collect();
    let mut read_error = None;
    let index = crate::native::index::native_expand_index(&resources, &mut |path| {
        sources.read(path).map(|f| f.bytes).map_err(|e| {
            let text = e.to_string();
            read_error.get_or_insert(e);
            text
        })
    });
    let index = match index {
        Ok(index) => index,
        Err(message) => {
            return Err(read_error.unwrap_or(SourceError::Missing(message)));
        }
    };
    let mut files = Vec::new();
    for entry in &index.files {
        files.push(sources.read(&entry.path)?);
    }
    let mut report = NativeLoadReport {
        index,
        ..NativeLoadReport::default()
    };
    let catalog = load_files(files, Limits::default(), &mut report);
    Ok((catalog, report))
}

/// The native load over already-ordered files (index order). Exposed for
/// synthetic tests of the registry rules.
pub fn load_files(
    files: Vec<crate::sources::ResolvedFile>,
    limits: Limits,
    report: &mut NativeLoadReport,
) -> ActorCatalog {
    let mut catalog = ActorCatalog {
        sources: Vec::new(),
        diagnostics: Vec::new(),
        actors: Vec::new(),
        lookup: BTreeMap::new(),
        registered: BTreeMap::new(),
        resolution_cache: Default::default(),
        resolver: Resolver::default(),
        native_data: BTreeMap::new(),
    };
    let mut data: BTreeMap<String, NativeDataRegistry> = GENERIC_DATA_TAGS
        .iter()
        .map(|t| {
            (
                key(t),
                NativeDataRegistry {
                    manager: (*t).to_owned(),
                    mode: native_manager_mode(t),
                    ..NativeDataRegistry::default()
                },
            )
        })
        .collect();
    catalog.resolver.upgrade_census_complete = true;
    catalog.resolver.source_hashes_valid = true;
    let mut registry = TemplateRegistry::default();
    let mut things = NativeThingRegistry::default();
    let mut thing_owner: Vec<usize> = Vec::new(); // registry slot -> root index
    for (file_no, file) in files.into_iter().enumerate() {
        let identity = SourceIdentity {
            path: file.logical_path.clone(),
            sha256: file.sha256.clone(),
            origin: file.origin.clone(),
        };
        report.files.push(identity.path.clone());
        if sha256(&file.bytes) != identity.sha256 {
            catalog.resolver.upgrade_census_complete = false;
            catalog.resolver.source_hashes_valid = false;
            catalog.diagnostics.push(Diagnostic {
                source: identity.clone(),
                message: "source SHA256 does not match supplied bytes; omitted".into(),
            });
            catalog.sources.push(identity);
            continue;
        }
        if file.bytes.contains(&0) {
            catalog.resolver.upgrade_census_complete = false;
            catalog.resolver.source_hashes_valid = false;
        }
        let could_contain_upgrade = file.bytes.contains(&0)
            || file
                .bytes
                .windows(7)
                .any(|w| w.eq_ignore_ascii_case(b"Upgrade"));
        let doc = match Document::parse_native(file.bytes.clone(), limits) {
            Ok(doc) => doc,
            Err(error) => {
                // 5BD2C0 logs and rethrows; 48D970/48DB60 do not catch, so the
                // original load would stop here. Shipped data never does.
                report.events.push(NativeLoadEvent {
                    kind: NativeLoadEventKind::ParseFailure,
                    file: identity.path.clone(),
                    detail: error.to_string(),
                });
                catalog.resolver.opaque_sources.push((identity.clone(), file.bytes));
                if could_contain_upgrade {
                    catalog.resolver.upgrade_census_complete = false;
                } else {
                    catalog
                        .resolver
                        .opaque_sources_without_upgrade_bytes
                        .push(identity.clone());
                }
                catalog.diagnostics.push(Diagnostic {
                    source: identity.clone(),
                    message: format!("native parse failure (fatal in AA.exe): {error}"),
                });
                catalog.sources.push(identity);
                continue;
            }
        };
        for notice in &doc.notices {
            catalog.diagnostics.push(Diagnostic {
                source: identity.clone(),
                message: format!("native syntax notice {:?} at {}", notice.kind, notice.span.start),
            });
        }
        let first_root = catalog.resolver.roots.len();
        let mut roots: Vec<Definition> = doc
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Node(node) => Some(definition(&doc, node, &identity)),
                Item::Property(_) => None,
            })
            .collect();
        // Root-level properties: the native root node accepts them as fields;
        // they create no definitions and are not dispatched.
        {
            let mut sim = FileSim {
                file: file_no,
                first_root,
                file_name: &identity.path,
                registry: &mut registry,
                events: &mut report.events,
            };
            sim.children(&mut roots, &[]);
        }
        for root in &roots {
            if root.template {
                catalog
                    .resolver
                    .templates
                    .entry(key(&root.name))
                    .or_default()
                    .push(catalog.resolver.roots.len());
            }
            catalog.resolver.roots.push(root.clone());
        }
        drop(roots);
        // 48D660 root dispatch.
        let mut processed = 0usize;
        let mut aborted: Option<String> = None;
        for index in first_root..catalog.resolver.roots.len() {
            let node = &catalog.resolver.roots[index];
            if node.template {
                continue;
            }
            processed += 1;
            if aborted.is_some() {
                continue;
            }
            let tag = node.name.clone();
            let failure = if THING_TAGS.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
                thing_root(&catalog.resolver, index, &mut things, &mut thing_owner)
                    .map(|registered| {
                        if let Some(id) = registered {
                            report.things.push((id, identity.path.clone()));
                        }
                    })
                    .err()
            } else if tag.eq_ignore_ascii_case("Translate") {
                translate_root(&catalog.resolver, index, &mut things, &mut data, report).err()
            } else if let Some(manager) = generic_tag(&tag) {
                let ids = plain_text(
                    catalog.resolver.raw_field(node, &[], "IDS").ok().flatten(),
                );
                report.other_roots.push((tag.clone(), ids, identity.path.clone()));
                generic_root(&catalog.resolver, index, manager, &mut data, report).err()
            } else if KNOWN_ROOT_TAGS.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
                let ids = plain_text(
                    catalog.resolver.raw_field(node, &[], "IDS").ok().flatten(),
                );
                report.other_roots.push((tag.clone(), ids, identity.path.clone()));
                None
            } else {
                report.events.push(NativeLoadEvent {
                    kind: NativeLoadEventKind::UnknownRootTag,
                    file: identity.path.clone(),
                    detail: tag.clone(),
                });
                Some(format!("Unknown node type '{tag}'"))
            };
            if let Some(message) = failure {
                report.events.push(NativeLoadEvent {
                    kind: NativeLoadEventKind::RootException,
                    file: identity.path.clone(),
                    detail: message.clone(),
                });
                catalog.diagnostics.push(Diagnostic {
                    source: identity.clone(),
                    message: format!(
                        "native 48D660 exception, remaining roots of this file skipped: {message}"
                    ),
                });
                aborted = Some(message);
            }
        }
        // 48D970: release (unregister templates) or keep.
        if processed > 0 && aborted.is_none() {
            let before = registry.live.len();
            registry.live.retain(|_, r| r.file != file_no);
            report.events.push(NativeLoadEvent {
                kind: NativeLoadEventKind::Released,
                file: identity.path.clone(),
                detail: format!("{} template(s) unregistered", before - registry.live.len()),
            });
        } else {
            report.events.push(NativeLoadEvent {
                kind: NativeLoadEventKind::Kept,
                file: identity.path.clone(),
                detail: aborted.unwrap_or_else(|| "no non-template roots".into()),
            });
        }
        catalog.sources.push(identity);
    }
    // Project registered Things in registration order.
    let resolver = &catalog.resolver;
    for &root in &thing_owner {
        let node = &resolver.roots[root];
        let id = plain_text(resolver.raw_field(node, &[], "IDS").ok().flatten()).unwrap_or_default();
        let chain = resolver
            .layers(node, &mut Vec::new())
            .unwrap_or_default()
            .iter()
            .map(|n| n.source.clone())
            .collect();
        catalog.actors.push(ActorDefinition {
            id,
            source: node.source.clone(),
            template_chain: chain,
            fields: FIELDS
                .iter()
                .map(|(label, path, field)| (label.to_string(), resolver.field(node, path, field)))
                .collect(),
        });
    }
    report.thing_translations = things.translations();
    for registry in data.values() {
        for (old, new) in registry.translations() {
            report.data_translations.push((registry.manager.clone(), old, new));
        }
        if !registry.entries.is_empty() {
            report
                .data_counts
                .insert(registry.manager.clone(), registry.entries.len());
        }
    }
    catalog.native_data = data;
    // Registry slots follow registration order; then 427BD0 translations.
    for (id, slot) in &things.registry {
        catalog.lookup.insert(id.clone(), *slot);
        catalog.registered.insert(id.clone(), *slot);
    }
    for old in things.translations.keys() {
        match things.find(old) {
            Some(slot) => {
                catalog.lookup.insert(old.clone(), slot);
            }
            None => {
                catalog.lookup.remove(old);
            }
        }
    }
    catalog
}

/// 4BDC50 + 4BCA00 for one root. Ok(Some(id)) registered; Err = exception.
fn thing_root(
    resolver: &Resolver,
    root: usize,
    things: &mut NativeThingRegistry,
    owners: &mut Vec<usize>,
) -> Result<Option<String>, String> {
    let node = &resolver.roots[root];
    let ids = resolver
        .raw_field(node, &[], "IDS")
        .map_err(|e| format!("IDS unresolved: {e}"))?;
    let Some(ids_field) = ids else {
        return Err("Node doesn't contain field. (IDS)".into());
    };
    let id = plain_text(Some(ids_field)).unwrap_or_default();
    let category = resolver
        .raw_field(node, &[], "category")
        .map_err(|e| format!("category unresolved: {e}"))?
        .ok_or_else(|| "Node doesn't contain field. (category)".to_string())?;
    let category = plain_text(Some(category)).unwrap_or_default();
    if !crate::native::CATEGORIES
        .iter()
        .any(|c| c.name.eq_ignore_ascii_case(&category))
    {
        return Err(format!("Unknown thing category '{category}'"));
    }
    let slot = things.register(&id)?;
    debug_assert_eq!(slot, owners.len());
    owners.push(root);
    Ok(Some(id))
}

/// 48D2D0 for one Translate root.
fn translate_root(
    resolver: &Resolver,
    root: usize,
    things: &mut NativeThingRegistry,
    data: &mut BTreeMap<String, NativeDataRegistry>,
    report: &mut NativeLoadReport,
) -> Result<(), String> {
    let node = &resolver.roots[root];
    let get = |name: &str| -> Result<String, String> {
        match resolver.raw_field(node, &[], name) {
            Ok(Some(field)) => Ok(plain_text(Some(field)).unwrap_or_default()),
            Ok(None) => Err(format!("Node doesn't contain field. ({name})")),
            Err(e) => Err(e),
        }
    };
    let manager = get("node")?;
    let old = get("ids_old")?;
    let new = get("ids_new")?;
    report.translations.push(NativeTranslation {
        manager: manager.clone(),
        ids_old: old.clone(),
        ids_new: new.clone(),
        source: node.source.clone(),
    });
    let result = if THING_TAGS.iter().any(|t| t.eq_ignore_ascii_case(&manager)) {
        Some(things.translate(&old, &new))
    } else if let Some(tag) = generic_tag(&manager) {
        Some(
            data.get_mut(&key(tag))
                .expect("generic registry")
                .translate(&old, &new),
        )
    } else {
        None
    };
    if let Some(result) = result {
        match result {
            Ok(()) => Ok(()),
            Err(NativeTranslateError::DuplicateOld) => Err(format!(
                "DataManager '{manager}' has duplicate IDS translation {old} -> {new}"
            )),
            Err(NativeTranslateError::MissingTarget) => Err(format!(
                "DataManager '{manager}' does not contain target IDS - loading translation {old} -> {new}"
            )),
        }
    } else if KNOWN_ROOT_TAGS.iter().any(|t| t.eq_ignore_ascii_case(&manager)) {
        report.events.push(NativeLoadEvent {
            kind: NativeLoadEventKind::UnmodeledTranslation,
            file: node.source.path.clone(),
            detail: format!("{manager}: {old} -> {new}"),
        });
        Ok(())
    } else {
        Err(format!(
            "DataManager '{manager}' does not exist - loading translation {old} -> {new}"
        ))
    }
}

/// Nonempty stored name: 47D960 stores GetLocalized("name") (5B5620). The
/// parse-time text keeps unquoted `null` verbatim and drops the quotes
/// (`tgi::native_localized_text`), so `null` is a nonempty name and `""` is
/// empty (oracle section `generic`). A `#key` is taken as nonempty; its
/// string-table text is not consulted here.
fn named_fields(fields: &[RawField]) -> bool {
    fields.iter().any(|f| {
        f.name.eq_ignore_ascii_case("name")
            && f.value.as_ref().is_some_and(|v| {
                let t = String::from_utf8_lossy(v);
                crate::tgi::native_localized_text(t.trim()).is_some_and(|text| !text.is_empty())
            })
    })
}
fn named(resolver: &Resolver, node: &Definition) -> bool {
    resolver
        .raw_field(node, &[], "name")
        .ok()
        .flatten()
        .is_some_and(|field| named_fields(std::slice::from_ref(&field)))
}

/// vf0 483A40 -> vf1C for a generic DataManager root; a Terrain root also
/// feeds each [Variation] child to the Variation manager (4B87A0 ->
/// [GameData+658] vf1C), in child order, after its own registration.
fn generic_root(
    resolver: &Resolver,
    root: usize,
    manager: &str,
    data: &mut BTreeMap<String, NativeDataRegistry>,
    report: &mut NativeLoadReport,
) -> Result<(), String> {
    let node = &resolver.roots[root];
    if let Some(problem) = &node.problem {
        report.events.push(NativeLoadEvent {
            kind: NativeLoadEventKind::UnreadGenericRoot,
            file: node.source.path.clone(),
            detail: format!("{manager}: {problem}"),
        });
        return Ok(());
    }
    let ids = match resolver.raw_field(node, &[], "IDS") {
        Ok(Some(field)) => plain_text(Some(field)).unwrap_or_default(),
        // 5B54D0 throws TGC_DataFileNodeException for a missing field.
        Ok(None) => return Err("Node doesn't contain field. (IDS)".into()),
        Err(e) => {
            report.events.push(NativeLoadEvent {
                kind: NativeLoadEventKind::UnreadGenericRoot,
                file: node.source.path.clone(),
                detail: format!("{manager}: {e}"),
            });
            return Ok(());
        }
    };
    let registry = data.get_mut(&key(manager)).expect("generic registry");
    let (_, outcome) =
        registry.load(&ids, named(resolver, node), vec![root], node.source.clone())?;
    outcome_event(report, outcome, &node.source.path, manager, &ids);
    if manager == "Terrain" {
        for (k, child) in node.children.iter().enumerate() {
            if !child.name.eq_ignore_ascii_case("Variation") || child.template {
                continue;
            }
            if let Some(problem) = &child.problem {
                report.events.push(NativeLoadEvent {
                    kind: NativeLoadEventKind::UnreadGenericRoot,
                    file: child.source.path.clone(),
                    detail: format!("Variation: {problem}"),
                });
                continue;
            }
            let ids = child
                .fields
                .iter()
                .find(|f| f.name.eq_ignore_ascii_case("IDS"))
                .cloned();
            let Some(ids) = plain_text(ids) else {
                return Err("Node doesn't contain field. (IDS)".into());
            };
            let registry = data.get_mut("variation").expect("Variation registry");
            let (_, outcome) = registry.load(
                &ids,
                named_fields(&child.fields),
                vec![root, k],
                child.source.clone(),
            )?;
            outcome_event(report, outcome, &child.source.path, "Variation", &ids);
        }
    }
    Ok(())
}

impl ActorCatalog {
    /// Registry of one generic DataManager after the native-order load
    /// (`None` for catalogs built by `from_files` or an unverified tag).
    pub fn native_data_registry(&self, manager: &str) -> Option<&NativeDataRegistry> {
        self.native_data.get(&key(manager))
    }
    /// Original 48D500 on that manager (translation map first).
    pub fn native_data_find(&self, manager: &str, id: &str) -> Option<&NativeDataEntry> {
        self.native_data_registry(manager)?.find(id)
    }
    /// Inherited view of a registered generic definition. A definition that
    /// a later node re-loaded (empty first name) is an explicit error: the
    /// type-specific LoadData overlay is not modeled.
    pub fn native_data_node(
        &self,
        entry: &NativeDataEntry,
    ) -> crate::native::Result<super::ResolvedNode<'_>> {
        use crate::native::Error;
        if !entry.reloads.is_empty() {
            return Err(Error::Unsupported(format!(
                "generic definition {} was re-loaded by {} later node(s); overlay semantics unmodeled",
                entry.ids,
                entry.reloads.len()
            )));
        }
        let node = self
            .resolver
            .node_at(&entry.path)
            .ok_or_else(|| Error::Unsupported("native data node unavailable".into()))?;
        Ok(super::ResolvedNode {
            catalog: self,
            layers: self
                .resolver
                .layers(node, &mut Vec::new())
                .map_err(Error::Unsupported)?,
        })
    }
}

fn outcome_event(report: &mut NativeLoadReport, outcome: LoadOutcome, file: &str, manager: &str, ids: &str) {
    let kind = match outcome {
        LoadOutcome::Created => return,
        LoadOutcome::Reloaded => NativeLoadEventKind::GenericReload,
        LoadOutcome::PredefinedAssumed => NativeLoadEventKind::PredefinedAssumed,
        LoadOutcome::UnmodeledDuplicate => NativeLoadEventKind::UnmodeledDuplicate,
    };
    report.events.push(NativeLoadEvent {
        kind,
        file: file.to_owned(),
        detail: format!("{manager}: {ids}"),
    });
}
