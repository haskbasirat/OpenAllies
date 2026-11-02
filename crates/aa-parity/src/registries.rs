//! Native definition-registry counts for decree payload decoding.
//!
//! A payload definition reference is a 0x411FA0 read over `count + 1`
//! possibilities, so its bit width depends on the live registry count
//! (`[registry]+0x28`). An unknown count is an explicit decode error, never a
//! guessed zero. The counts and slot orders depend on the data a film was
//! recorded with, so they are computed from that data, the way the original
//! fills the registries, instead of being fixed numbers:
//!
//! * ActorCommand `[0xB3AA58]`: the 39 code-predefined commands
//!   (`aa_rules::native_actor_command::PREDEFINED_ACTOR_COMMANDS`, registered
//!   by 0x475870 before any source load). Always present.
//! * Thing DataManager `[0xB39F58]`: the 4BCA00 registration slots of the
//!   native-order load (aa-runtime `player_order_object_reference.rs`, proven
//!   by research/phase8-orders-input/recruit/native-object-reference.json).
//! * Generic DataManagers (Formation `[0xB3AB08]`, OrganizationLayout
//!   `[0xB3AB00]`, the setters in GameData::GameData 0x48A8E0): the entries of
//!   the native-order load.
//! * SharedProperty `[0xB3AA2C]`: slot 0 is the predefined `mobile` (0x47E9B0).
//!   `[Property]` roots go through a DataManager in mode 1 (constructor
//!   0x47EFD0, `0x47efd0(0x40, 1)` in 0x48A8E0; vf1C 0x48B580, create
//!   0x48B640): an IDS seen before re-loads its definition, a new one takes the
//!   next slot. Every ENCHANTMENT effect of an AbilityDefinition adds its
//!   `EnchantmentProperty` when the definition loads (0x4CBBF0 -> 0x4B4E60).
//! * AbilityDefinition `[0xB3AB10]`: the `[AbilityDefinition]` roots (vf1C
//!   0x481EC0, create 0x481F80) and, interleaved in load order, every inline
//!   definition of an actor's abilities: KKC_ActorData::LoadData 0x464BD0 runs
//!   KKC_AbilityData::LoadData 0x4BF990 on each `PerformerComponent/Ability`,
//!   each `DefenderComponent/Ability` and `BodyComponent/OnDeathAbility`, in
//!   that order, while the Thing root is being registered (4BDC50), and a
//!   Faction's `Ability` children (0x48BC9D). An ability with an
//!   `AbilityDefinition` child always adds one slot: named by its
//!   `ids`, else `<object>_bombard` / `<object>_ranged` when it has
//!   `projectile_ids`, else anonymous; a generated name that exists is logged
//!   and made anonymous too (0x4BF170).
//!
//! [`Provenance`] says where a count came from. The computed counts are proven
//! against the original by `tests/registry_counts.rs`: NativeSampler read every
//! registry's live `[registry]+0x28` from the v14 QA copy on 2026-10-25 (`REG`
//! lines of the Berlin film run, [`MEASURED_V14`]) and the 194 SharedProperty
//! slots in order (`REGIDS`).
use aa_orders::payload::{Registry, SchemaContext};
use aa_rules::actors::{ActorCatalog, ResolvedNode, ThingRef, native_load};
use aa_rules::native::TypedValue;
use aa_rules::resources::Provenance as SourceSpan;
use aa_rules::sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const ACTOR_COMMANDS: Registry = Registry(0xb3aa58);
pub const OBJECT_DEFINITIONS: Registry = Registry(0xb39f58);
pub const FORMATIONS: Registry = Registry(0xb3ab08);
pub const ORGANIZATION_LAYOUTS: Registry = Registry(0xb3ab00);
pub const SHARED_PROPERTIES: Registry = Registry(0xb3aa2c);
pub const ABILITIES: Registry = Registry(0xb3ab10);
pub const WORLD_RULES: Registry = Registry(0xb3aa88);
pub const GAME_TYPES: Registry = Registry(0xb3aa70);
pub const NATIONS: Registry = Registry(0xb3aa44);
pub const FACTIONS: Registry = Registry(0xb3aa4c);
pub const KINGDOM_COLORS: Registry = Registry(0xb3aa60);
pub const HANDICAPS: Registry = Registry(0xb3aa68);
pub const BIOMES: Registry = Registry(0xb3aa24);
pub const WEATHERS: Registry = Registry(0xb3aa34);

/// Readable name of a registry address, for reports.
pub fn registry_name(registry: Registry) -> &'static str {
    match registry.0 {
        0xb3aa58 => "ActorCommand",
        0xb39f58 => "ThingData",
        0xb3ab08 => "Formation",
        0xb3ab00 => "OrganizationLayout",
        0xb3aa2c => "SharedProperty",
        0xb3ab10 => "AbilityDefinition",
        0xb3aa88 => "WorldRule",
        0xb3aa70 => "GameType",
        0xb3aa44 => "Nation",
        0xb3aa4c => "Faction",
        0xb3aa60 => "KingdomColor",
        0xb3aa68 => "Handicap",
        0xb3aa24 => "Biome",
        0xb3aa34 => "Weather",
        _ => "unknown",
    }
}

/// Where a count came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provenance {
    Proven,
    Inferred,
    /// Read live from the original process (see the module notes).
    Measured,
    /// Computed from a data set the way the original fills the registry.
    Computed,
}

/// The live counts the original v14 process reports (see the module notes).
/// Not used for decoding: they are what the computed counts are checked with.
pub const MEASURED_V14: [(Registry, usize); 6] = [
    (ACTOR_COMMANDS, 39),
    (OBJECT_DEFINITIONS, 1959),
    (FORMATIONS, 5),
    (ORGANIZATION_LAYOUTS, 10),
    (SHARED_PROPERTIES, 194),
    (ABILITIES, 409),
];

/// The ten TeamCommand definitions of GameData+0x270 in slot order: the code
/// registers them with their flag word (0x475590 -> 0x4B61C0, +0x24) and only
/// loads the rest of each from `Game/team_commands.tgi` (0x4B66A0 refuses any
/// other IDS), so these hold for every data set. The names are the 0x509790
/// enumeration. KKC_TeamCommand's reader 0x509090 indexes this list with its
/// first word and reads the fields the flags ask for.
pub const TEAM_COMMANDS: [(&str, u32); 10] = [
    ("team_attack", 0x103),
    ("team_defend", 0x403),
    ("team_explore", 0x2),
    ("team_give_resources", 0x120008),
    ("team_give_actor", 0x110801),
    ("team_request_resources", 0x140008),
    ("team_request_actor", 0x101001),
    ("team_build", 0x207),
    ("team_chat", 0x10),
    ("team_report", 0x20),
];

/// Generic DataManager tag of each registry global (GameData::GameData 0x48A8E0
/// setters; aa-netcode `lan::registries` has the table of addresses).
const GENERIC: [(Registry, &str); 10] = [
    (FORMATIONS, "Formation"),
    (ORGANIZATION_LAYOUTS, "OrganizationLayout"),
    (WORLD_RULES, "WorldRule"),
    (GAME_TYPES, "GameType"),
    (NATIONS, "Nation"),
    (FACTIONS, "Faction"),
    (KINGDOM_COLORS, "KingdomColor"),
    (HANDICAPS, "Handicap"),
    (BIOMES, "Biome"),
    (WEATHERS, "Weather"),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Registries {
    counts: BTreeMap<u32, (usize, Provenance)>,
    /// Thing DataManager slot → definition IDS (native load order), when
    /// loaded from a catalog. Index = the wire value of an object reference.
    pub object_ids: Vec<String>,
    /// Generic DataManager slot → IDS.
    pub formation_ids: Vec<String>,
    /// AbilityDefinition slot → IDS. An anonymous definition (0x4BF170) has an
    /// empty IDS; [`Registries::ability_label`] names it by its object.
    pub ability_ids: Vec<String>,
    /// The object whose LoadData created the slot; empty for a root.
    pub ability_owners: Vec<String>,
    /// SharedProperty slot → IDS.
    pub property_ids: Vec<String>,
    pub layout_ids: Vec<String>,
    pub nation_ids: Vec<String>,
    pub faction_ids: Vec<String>,
    pub biome_ids: Vec<String>,
    pub weather_ids: Vec<String>,
    pub rmc_ids: Vec<String>,
    /// Lowercase faction IDS → the lowercase names of its abilities (0x48BC9D,
    /// an `ability_IDS` or an inline definition). A film's 70 FactionUseAbility
    /// must name one of the ordering player's, which checks the slot order.
    #[serde(default)]
    pub faction_abilities: BTreeMap<String, Vec<String>>,
    /// Findings of the computation (children the model could not read). Empty
    /// on the shipped v14 data.
    pub notes: Vec<String>,
}

impl Default for Registries {
    fn default() -> Self {
        Self::builtin()
    }
}

/// Bytes of one registered data resource, by its registered path.
type DataRead<'a> = dyn FnMut(&str) -> Result<Vec<u8>, String> + 'a;

impl Registries {
    /// Only the code-predefined ActorCommand registry.
    pub fn builtin() -> Self {
        let mut counts = BTreeMap::new();
        counts.insert(
            ACTOR_COMMANDS.0,
            (
                aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.len(),
                Provenance::Proven,
            ),
        );
        Self {
            counts,
            object_ids: Vec::new(),
            formation_ids: Vec::new(),
            ability_ids: Vec::new(),
            ability_owners: Vec::new(),
            property_ids: Vec::new(),
            layout_ids: Vec::new(),
            nation_ids: Vec::new(),
            faction_ids: Vec::new(),
            biome_ids: Vec::new(),
            weather_ids: Vec::new(),
            rmc_ids: Vec::new(),
            faction_abilities: BTreeMap::new(),
            notes: Vec::new(),
        }
    }
    /// Set one count explicitly (tests, or a count measured elsewhere).
    pub fn with_count(mut self, registry: Registry, count: usize, provenance: Provenance) -> Self {
        self.counts.insert(registry.0, (count, provenance));
        self
    }
    /// Only the ThingData slots of a native-order catalog
    /// (`ActorCatalog::load_v14`), with the built-in ActorCommand registry.
    pub fn from_catalog(catalog: &ActorCatalog) -> Self {
        let mut out = Self::builtin();
        out.object_ids = catalog.actors.iter().map(|a| a.id.clone()).collect();
        out.counts.insert(
            OBJECT_DEFINITIONS.0,
            (out.object_ids.len(), Provenance::Proven),
        );
        out
    }
    /// Every payload registry, computed from a native-order load (see the
    /// module notes for the original's rules).
    pub fn from_native_load(catalog: &ActorCatalog, report: &native_load::NativeLoadReport) -> Self {
        let mut out = Self::from_catalog(catalog);
        for (registry, tag) in GENERIC {
            if let Some(r) = catalog.native_data_registry(tag) {
                out.counts
                    .insert(registry.0, (r.entries.len(), Provenance::Computed));
            }
        }
        let ids = |tag: &str| -> Vec<String> {
            catalog.native_data_registry(tag).map_or_else(Vec::new, |r| {
                r.entries.iter().map(|e| e.ids.clone()).collect()
            })
        };
        out.formation_ids = ids("Formation");
        out.layout_ids = ids("OrganizationLayout");
        out.nation_ids = ids("Nation");
        out.faction_ids = ids("Faction");
        out.biome_ids = ids("Biome");
        out.weather_ids = ids("Weather");
        out.rmc_ids = ids("RMC");
        let slots = DefinitionSlots::compute(catalog, report);
        out.counts.insert(
            SHARED_PROPERTIES.0,
            (slots.property_ids.len(), Provenance::Computed),
        );
        out.counts.insert(
            ABILITIES.0,
            (slots.ability_ids.len(), Provenance::Computed),
        );
        out.property_ids = slots.property_ids;
        out.ability_ids = slots.ability_ids;
        out.ability_owners = slots.ability_owners;
        out.faction_abilities = slots.faction_abilities;
        out.notes = slots.notes;
        out
    }
    /// Load the native-order v14 catalog from the game root (Data.rwd plus
    /// `reconstruction/native-game/runtime-expanded-v14`, the data the
    /// shipped QA films were recorded with: their AA.exe is the v14 build).
    /// `computed` false keeps only the ThingData and ActorCommand counts.
    pub fn load_v14(game_root: &Path, computed: bool) -> anyhow::Result<Self> {
        let mut sources = open_v14(game_root)?;
        let (catalog, report) = native_load::load(&mut sources)
            .map_err(|e| anyhow::anyhow!("native-order catalog load: {e}"))?;
        Ok(if computed {
            Self::from_native_load(&catalog, &report)
        } else {
            Self::from_catalog(&catalog)
        })
    }
    /// The registries of a data set the way an install resolves it: the loose
    /// files of `overlay` (when given) over the stock archive `Data.rwd` of
    /// `game_root`. The v14 overlay is not part of it.
    pub fn load_overlay(game_root: &Path, overlay: Option<&Path>) -> anyhow::Result<Self> {
        let mut sources = open_v14(game_root)?;
        let mut stock: BTreeMap<String, String> = BTreeMap::new();
        for row in sources.catalog() {
            if matches!(row.origin, Origin::OriginalArchive { .. })
                || sources.read_original(&row.logical_path).is_ok()
            {
                stock.insert(row.logical_path.to_ascii_lowercase(), row.logical_path);
            }
        }
        let mut loose: BTreeMap<String, (String, PathBuf)> = BTreeMap::new();
        if let Some(dir) = overlay {
            walk_data(dir, "", &mut loose)?;
        }
        let mut resources: Vec<String> = Vec::with_capacity(stock.len() + loose.len());
        for (key, path) in &stock {
            if !loose.contains_key(key) {
                resources.push(path.clone());
            }
        }
        resources.extend(loose.values().map(|(path, _)| path.clone()));
        let mut read = |path: &str| -> Result<Vec<u8>, String> {
            let key = path.replace('\\', "/").to_ascii_lowercase();
            if let Some((_, disk)) = loose.get(&key) {
                return std::fs::read(disk).map_err(|e| format!("{}: {e}", disk.display()));
            }
            sources
                .read_original(path)
                .map(|f| f.bytes)
                .map_err(|e| format!("{path}: {e}"))
        };
        Self::load_resources(&resources, &mut read)
    }
    /// The 48DB60 load over registered resources.
    fn load_resources(resources: &[String], read: &mut DataRead<'_>) -> anyhow::Result<Self> {
        let index = aa_rules::native::index::native_expand_index(resources, read)
            .map_err(|e| anyhow::anyhow!("index chain: {e}"))?;
        let mut files = Vec::with_capacity(index.files.len());
        for entry in &index.files {
            // An index line naming a file no depot has adds no definition.
            let Ok(bytes) = read(&entry.path) else {
                continue;
            };
            files.push(ResolvedFile {
                logical_path: entry.path.clone(),
                origin: Origin::V14Loose,
                sha256: sha256(&bytes),
                bytes,
            });
        }
        let mut report = native_load::NativeLoadReport {
            index,
            ..native_load::NativeLoadReport::default()
        };
        let catalog =
            native_load::load_files(files, aa_rules::tgi::Limits::default(), &mut report);
        Ok(Self::from_native_load(&catalog, &report))
    }
    pub fn count(&self, registry: Registry) -> Option<(usize, Provenance)> {
        self.counts.get(&registry.0).copied()
    }
    /// Every known count, for reports.
    pub fn counts(&self) -> impl Iterator<Item = (Registry, usize, Provenance)> + '_ {
        self.counts.iter().map(|(&r, &(n, p))| (Registry(r), n, p))
    }
    /// The name of an AbilityDefinition slot for reports: its IDS, or the
    /// object an anonymous definition belongs to.
    pub fn ability_label(&self, slot: u32) -> Option<String> {
        let id = self.ability_ids.get(slot as usize)?;
        if !id.is_empty() {
            return Some(id.clone());
        }
        let owner = self.ability_owners.get(slot as usize)?;
        Some(format!("(anonymous, {owner})"))
    }
}

/// The lowercase resource paths of the v14 data (Data.rwd and its overlay): what
/// a `KKC_WorldCreatorLoad` film can find on the v14 target besides the player's
/// own `Save` folder.
pub fn v14_resources(game_root: &Path) -> anyhow::Result<BTreeSet<String>> {
    Ok(open_v14(game_root)?
        .catalog()
        .into_iter()
        .map(|row| row.logical_path.replace('\\', "/").to_ascii_lowercase())
        .collect())
}

fn open_v14(game_root: &Path) -> anyhow::Result<V14Sources> {
    V14Sources::open(
        game_root.join("Data.rwd"),
        game_root
            .join("reconstruction")
            .join("native-game")
            .join("runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 << 20,
        },
    )
    .map_err(|e| anyhow::anyhow!("opening the original data in {}: {e}", game_root.display()))
}

/// Every file below a loose data folder, by lowercase registered path.
fn walk_data(
    dir: &Path,
    prefix: &str,
    out: &mut BTreeMap<String, (String, PathBuf)>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let logical = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if entry.file_type()?.is_dir() {
            walk_data(&path, &logical, out)?;
        } else {
            out.insert(logical.to_ascii_lowercase(), (logical, path));
        }
    }
    Ok(())
}

/// `(position of a file in the load, byte offset)`: the order 48D660 dispatched
/// two roots of a data set in.
fn load_position(files: &BTreeMap<String, usize>, source: &SourceSpan) -> (usize, usize) {
    (
        files
            .get(&source.path.to_ascii_lowercase())
            .copied()
            .unwrap_or(usize::MAX),
        source.span.start,
    )
}

/// One root of the load that touches the AbilityDefinition or SharedProperty
/// registry.
enum Root<'a> {
    Property(String),
    Ability(&'a str, ResolvedNode<'a>),
    Faction(&'a str, ResolvedNode<'a>),
    Actor(&'a str),
}

/// The AbilityDefinition and SharedProperty registries, in the order the
/// original fills them (see the module notes).
#[derive(Default)]
struct DefinitionSlots {
    ability_ids: Vec<String>,
    ability_owners: Vec<String>,
    property_ids: Vec<String>,
    notes: Vec<String>,
    /// Registered ability names: 48D500 finds by IDS, case-insensitive.
    abilities: BTreeSet<String>,
    properties: BTreeSet<String>,
    faction_abilities: BTreeMap<String, Vec<String>>,
}
impl DefinitionSlots {
    fn compute(catalog: &ActorCatalog, report: &native_load::NativeLoadReport) -> Self {
        let mut slots = Self::default();
        // KKC_Data::CreatePredefinedProperties 0x47E9B0, before any file.
        slots.property("mobile");
        let mut files = BTreeMap::new();
        for (position, path) in report.files.iter().enumerate() {
            files.entry(path.to_ascii_lowercase()).or_insert(position);
        }
        let mut roots: Vec<((usize, usize), Root<'_>)> = Vec::new();
        // The `[Property]` roots 48D660 dispatched (a root after an exception
        // in its file is not one of them).
        let mut dispatched: BTreeMap<(String, String), usize> = BTreeMap::new();
        for (tag, ids, file) in &report.other_roots {
            if tag.eq_ignore_ascii_case("Property")
                && let Some(ids) = ids
            {
                *dispatched
                    .entry((file.to_ascii_lowercase(), ids.to_ascii_lowercase()))
                    .or_default() += 1;
            }
        }
        match catalog.resolved_definitions("Property") {
            Ok(nodes) => {
                for node in nodes {
                    let (Some(source), Some(ids)) = (node.first_source(), plain(&node, "IDS"))
                    else {
                        continue;
                    };
                    let key = (source.path.to_ascii_lowercase(), ids.to_ascii_lowercase());
                    if let Some(left) = dispatched.get_mut(&key)
                        && *left > 0
                    {
                        *left -= 1;
                        roots.push((load_position(&files, source), Root::Property(ids)));
                    }
                }
            }
            Err(e) => slots.notes.push(format!("Property roots: {e}")),
        }
        for (key, left) in &dispatched {
            if *left > 0 {
                slots
                    .notes
                    .push(format!("Property root {} of {} not resolved", key.1, key.0));
            }
        }
        for tag in ["AbilityDefinition", "Faction"] {
            let Some(registry) = catalog.native_data_registry(tag) else {
                continue;
            };
            for entry in &registry.entries {
                match catalog.native_data_node(entry) {
                    Ok(node) => {
                        let root = if tag == "Faction" {
                            Root::Faction(&entry.ids, node)
                        } else {
                            Root::Ability(&entry.ids, node)
                        };
                        roots.push((load_position(&files, &entry.source), root));
                    }
                    Err(e) => slots.notes.push(format!("{tag} {}: {e}", entry.ids)),
                }
            }
        }
        for actor in &catalog.actors {
            roots.push((load_position(&files, &actor.source), Root::Actor(&actor.id)));
        }
        roots.sort_by_key(|r| r.0);
        for (_, root) in roots {
            match root {
                Root::Property(ids) => slots.property(&ids),
                Root::Ability(ids, node) => {
                    slots.abilities.insert(ids.to_ascii_lowercase());
                    slots.ability_ids.push(ids.to_owned());
                    slots.ability_owners.push(String::new());
                    slots.effects(ids, &node);
                }
                // FactionData LoadData reaches 0x4BF990 with no actor (0x48BC9D).
                Root::Faction(ids, node) => match node.children("Ability") {
                    Ok(list) => {
                        for ability in list {
                            let before = slots.ability_ids.len();
                            slots.ability(ids, &ability);
                            let name = slots
                                .ability_ids
                                .get(before)
                                .cloned()
                                .or_else(|| plain(&ability, "ability_IDS"))
                                .unwrap_or_default();
                            slots
                                .faction_abilities
                                .entry(ids.to_ascii_lowercase())
                                .or_default()
                                .push(name.to_ascii_lowercase());
                        }
                    }
                    Err(e) => slots.notes.push(format!("{ids}: Ability: {e}")),
                },
                Root::Actor(ids) => slots.actor(catalog, ids),
            }
        }
        slots
    }
    /// The mode 1 create of 0x48B580: a repeated IDS takes no slot.
    fn property(&mut self, ids: &str) {
        if self.properties.insert(ids.to_ascii_lowercase()) {
            self.property_ids.push(ids.to_owned());
        }
    }
    /// The inline AbilityDefinition slots of one registered Thing: 0x464BD0
    /// runs only for ActorData (category kind 2, 4BCA00 case 2).
    fn actor(&mut self, catalog: &ActorCatalog, id: &str) {
        let thing = ThingRef::registered(id);
        match catalog.native_category(thing) {
            Ok(category) if category.native_kind == 2 => {}
            Ok(_) => return,
            Err(e) => {
                self.notes.push(format!("{id}: category: {e}"));
                return;
            }
        }
        let node = match catalog.resolved_actor(thing) {
            Ok(node) => node,
            Err(e) => {
                self.notes.push(format!("{id}: unresolved: {e}"));
                return;
            }
        };
        for (component, child, list) in [
            ("PerformerComponent", "Ability", true),
            ("DefenderComponent", "Ability", true),
            ("BodyComponent", "OnDeathAbility", false),
        ] {
            let found = node.child(component).and_then(|c| match c {
                None => Ok(Vec::new()),
                Some(c) if list => c.children(child),
                Some(c) => c.child(child).map(|one| one.into_iter().collect()),
            });
            match found {
                Ok(found) => {
                    for ability in found {
                        self.ability(id, &ability);
                    }
                }
                Err(e) => self.notes.push(format!("{id}: {component}/{child}: {e}")),
            }
        }
    }
    /// 0x4BF990 on one ability of `owner`.
    fn ability(&mut self, owner: &str, ability: &ResolvedNode<'_>) {
        let definition = match ability.child("AbilityDefinition") {
            Ok(Some(definition)) => definition,
            // `ability_IDS`: a reference, no new definition.
            Ok(None) => return,
            // A child the node model cannot read (a mod's `Template==X`): it is
            // still a definition to 0x5B4410, so it keeps its slot, unnamed.
            Err(e) => {
                self.notes.push(format!("{owner}: AbilityDefinition: {e}"));
                self.ability_ids.push(String::new());
                self.ability_owners.push(owner.to_owned());
                return;
            }
        };
        let mut name = plain(&definition, "ids").unwrap_or_default();
        if name.is_empty() && !plain(&definition, "projectile_ids").unwrap_or_default().is_empty() {
            let bombard = definition
                .child("Flags")
                .ok()
                .flatten()
                .and_then(|flags| plain(&flags, "bombard"))
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            name = format!("{owner}_{}", if bombard { "bombard" } else { "ranged" });
        }
        // 0x4BFEB0: a name already registered is logged and made anonymous.
        if !name.is_empty() && !self.abilities.insert(name.to_ascii_lowercase()) {
            name.clear();
        }
        self.ability_ids.push(name);
        self.ability_owners.push(owner.to_owned());
        self.effects(owner, &definition);
    }
    /// The AbilityDefinition load of its `Effect` list: an ENCHANTMENT effect
    /// (kind 6, 0x4CBBF0) builds its `EnchantmentProperty` and adds it to the
    /// SharedProperty manager through 0x4B4E60.
    fn effects(&mut self, owner: &str, definition: &ResolvedNode<'_>) {
        let list = match definition.children("Effect") {
            Ok(list) => list,
            Err(e) => {
                self.notes.push(format!("{owner}: Effect: {e}"));
                return;
            }
        };
        for effect in list {
            if !plain(&effect, "type").is_some_and(|t| t.eq_ignore_ascii_case("ENCHANTMENT")) {
                continue;
            }
            let id = effect
                .child("EnchantmentProperty")
                .ok()
                .flatten()
                .and_then(|node| plain(&node, "ids"));
            match id {
                // 0x4B4E60 throws "Data IDS already used" on a repeat, which
                // 48D660 catches; the slot stays with the first one.
                Some(id) if self.properties.insert(id.to_ascii_lowercase()) => {
                    self.property_ids.push(id);
                }
                Some(id) => self
                    .notes
                    .push(format!("{owner}: enchantment property {id} repeats an IDS")),
                None => self.notes.push(format!("{owner}: enchantment without ids")),
            }
        }
    }
}

/// A string field the way GetString 0x5B54D0 reads it. The strict identifier form
/// covers the undeclared fields; a mod's `IDS = at_stun FRAArt1` or `2SP_recon`
/// is not an identifier but is a string to the original, so the typed getter
/// comes next. `None` when absent or of another shape.
fn plain(node: &ResolvedNode<'_>, name: &str) -> Option<String> {
    if let Ok(Some(TypedValue::String { value, .. })) = node.plain_string_field(name) {
        return Some(value);
    }
    // The getter's precision only matters for numbers. `aa_numeric` is a
    // dev-dependency of this crate, so its `Precision` is inferred from the call.
    let precision = serde_json::from_str("\"Double53\"").ok()?;
    match node.field(name, precision) {
        Ok(Some(TypedValue::String { value, .. })) => Some(value),
        _ => None,
    }
}

impl SchemaContext for Registries {
    fn registry_count(&self, registry: Registry) -> aa_orders::Result<usize> {
        self.count(registry).map(|(n, _)| n).ok_or_else(|| {
            aa_orders::Error::SchemaSource(format!(
                "film decode: count of definition registry {:#x} ({}) not loaded",
                registry.0,
                registry_name(registry)
            ))
        })
    }
    fn team_command_flags(&self, index: u32) -> aa_orders::Result<u32> {
        // 0x509090 does no bounds check; an index past the list is not a film.
        TEAM_COMMANDS
            .get(index as usize)
            .map(|(_, flags)| *flags)
            .ok_or(aa_orders::Error::SchemaContext(
                "film decode: TeamCommand index outside the ten definitions",
            ))
    }
    fn definition_kind(&self, _registry: Registry, _index: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext(
            "film decode: definition kinds not loaded",
        ))
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}
