//! Definition-registry counts a LAN session encodes its orders with.
//!
//! A definition reference on the wire is a 0x411FA0 read over `count + 1`
//! possibilities (writer 0x76A880 family: `def+0xC`, the count for null), so
//! every node of a session must use the original's live `[registry]+0x28`
//! counts. The registries are the GameData DataManagers whose global
//! pointers the payload schemas name; which manager each global holds is
//! read from the setters GameData::GameData 0x48A8E0 calls on the
//! constructed manager (research/phase8-definitions/tag-managers.json gives
//! the GameData field of each tag):
//!
//! | global | setter | GameData field | tag |
//! |---|---|---|---|
//! | 0xB3AA24 | 0x47EE80 | +0x5C0 | Biome (map settings +0x44) |
//! | 0xB3AA34 | 0x47F130 | +0x6F0 | Weather (map settings +0x48) |
//! | 0xB3AA44 | 0x47F3D0 | +0x5C | Nation |
//! | 0xB3AA4C | 0x47F520 | +0xA8 | Faction |
//! | 0xB3AA60 | 0x47F910 | +0x3CC | KingdomColor |
//! | 0xB3AA68 | 0x47FA60 | +0x418 | Handicap |
//! | 0xB3AA70 | 0x47FBB0 | +0x47C | GameType (world conditions +8) |
//! | 0xB3AA88 | 0x4801B0 | +0xF4 | WorldRule |
//! | 0xB3AA58 | 0x47F7C0 | +0x274 | ActorCommand (39 predefined) |
//! | 0xB3AB00 / 0xB3AB08 / 0xB3AB10 | | | OrganizationLayout / Formation / AbilityDefinition |
//! | 0xB39F58 | 0x4B4E60 | Thing manager | ThingData (registration slots) |
//!
//! The generic DataManager counts and slot orders come from the native-order
//! catalog load (aa_rules `ActorCatalog::load_v14`, `native_data_registry`).
//! SharedProperty 0xB3AA2C (a plain allocation in 0x48B180, not a generic
//! manager) and AbilityDefinition have no catalog source that matches the
//! live count; their counts are the values NativeSampler read from the
//! original v14 process (aa-parity `registries::MEASURED_V14`: 194 / 409).
use crate::{Error, Result};
use aa_orders::payload::Registry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const OBJECT_DEFINITIONS: Registry = Registry(0xb39f58);
pub const BIOMES: Registry = Registry(0xb3aa24);
pub const SHARED_PROPERTIES: Registry = Registry(0xb3aa2c);
pub const WEATHERS: Registry = Registry(0xb3aa34);
pub const NATIONS: Registry = Registry(0xb3aa44);
pub const FACTIONS: Registry = Registry(0xb3aa4c);
pub const ACTOR_COMMANDS: Registry = Registry(0xb3aa58);
pub const KINGDOM_COLORS: Registry = Registry(0xb3aa60);
pub const HANDICAPS: Registry = Registry(0xb3aa68);
pub const GAME_TYPES: Registry = Registry(0xb3aa70);
pub const WORLD_RULES: Registry = Registry(0xb3aa88);
pub const ORGANIZATION_LAYOUTS: Registry = Registry(0xb3ab00);
pub const FORMATIONS: Registry = Registry(0xb3ab08);
pub const ABILITIES: Registry = Registry(0xb3ab10);

/// Live counts of the original v14 process (NativeSampler REG lines,
/// 2026-10-25) for the registries without a catalog source.
pub const MEASURED_SHARED_PROPERTIES: usize = 194;
pub const MEASURED_ABILITIES: usize = 409;

/// Generic DataManager tag of each registry global (table above).
const GENERIC: [(Registry, &str); 11] = [
    (BIOMES, "Biome"),
    (WEATHERS, "Weather"),
    (NATIONS, "Nation"),
    (FACTIONS, "Faction"),
    (KINGDOM_COLORS, "KingdomColor"),
    (HANDICAPS, "Handicap"),
    (GAME_TYPES, "GameType"),
    (WORLD_RULES, "WorldRule"),
    (ORGANIZATION_LAYOUTS, "OrganizationLayout"),
    (FORMATIONS, "Formation"),
    (ABILITIES, "AbilityDefinition"),
];

/// Count and slot order (IDS by wire index) of every registry a session uses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRegistries {
    /// Registry global address -> IDS in slot order.
    pub slots: BTreeMap<u32, Vec<String>>,
    /// Registry global address -> live count when it differs from the slot
    /// list (measured registries: the slots are the catalog's partial list).
    pub counts: BTreeMap<u32, usize>,
    /// WorldRule value kind (`def+0x20`: 0 bool, 1 int, 2 float, 3 enum) by slot.
    pub world_rule_kinds: Vec<u32>,
    /// SHA256 of the source ledger (cache key).
    pub source_key: String,
}

impl NativeRegistries {
    /// `[registry]+0x28`.
    pub fn count(&self, registry: Registry) -> Option<usize> {
        if let Some(&n) = self.counts.get(&registry.0) {
            return Some(n);
        }
        if registry == ACTOR_COMMANDS {
            return Some(aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.len());
        }
        self.slots.get(&registry.0).map(Vec::len)
    }
    /// The wire index of a definition IDS (case-insensitive, as 48D500 finds).
    pub fn index(&self, registry: Registry, ids: &str) -> Option<u32> {
        self.slots.get(&registry.0)?.iter().position(|s| s.eq_ignore_ascii_case(ids)).map(|i| i as u32)
    }
    /// The IDS at a wire index.
    pub fn ids(&self, registry: Registry, index: u32) -> Option<&str> {
        self.slots.get(&registry.0)?.get(index as usize).map(String::as_str)
    }

    /// The native-order catalog of the installed original data (Data.rwd and
    /// the v14 overlay `reconstruction/native-game/runtime-expanded-v14`).
    pub fn from_catalog(catalog: &aa_rules::actors::ActorCatalog) -> Result<Self> {
        let mut out = Self::default();
        out.slots.insert(OBJECT_DEFINITIONS.0, catalog.actors.iter().map(|a| a.id.clone()).collect());
        for (registry, tag) in GENERIC {
            if let Some(r) = catalog.native_data_registry(tag) {
                out.slots.insert(registry.0, r.entries.iter().map(|e| e.ids.clone()).collect());
            }
        }
        out.counts.insert(SHARED_PROPERTIES.0, MEASURED_SHARED_PROPERTIES);
        out.counts.insert(ABILITIES.0, MEASURED_ABILITIES);
        // WorldRule +0x20 value kind: the definition's `type` (template
        // world rule, read by the manager's LoadData 0x490xxx: "bool",
        // integer, "fixed", and "enum" / "enumeration" / "enum_specified",
        // which all store 3).
        if let Some(rules) = out.slots.get(&WORLD_RULES.0).cloned() {
            let pc = aa_numeric::Precision::Double53;
            for ids in rules {
                let ty = catalog.resolved_definition("WorldRule", &ids, pc).ok().and_then(|n| match n.field("type", pc).ok().flatten() {
                    Some(aa_rules::native::TypedValue::String { value, .. }) => Some(value),
                    _ => match n.plain_string_field("type").ok().flatten() {
                        Some(aa_rules::native::TypedValue::String { value, .. }) => Some(value),
                        _ => None,
                    },
                });
                let kind = match ty.as_deref().map(str::to_ascii_lowercase).as_deref() {
                    Some("bool") => 0,
                    Some("int") => 1,
                    Some("fixed") => 2,
                    Some("enum" | "enumeration" | "enum_specified") => 3,
                    other => return Err(Error::Protocol(format!("WorldRule {ids}: type {other:?} is not one the original loads"))),
                };
                out.world_rule_kinds.push(kind);
            }
        }
        Ok(out)
    }

    /// Load from the original install, cached as JSON at `cache` (keyed by
    /// the source ledger) because the native-order catalog load is slow.
    pub fn load_v14(game_root: &std::path::Path, cache: Option<&std::path::Path>) -> Result<Self> {
        let open = || {
            aa_rules::sources::V14Sources::open(
                game_root.join("Data.rwd"),
                game_root.join("reconstruction").join("native-game").join("runtime-expanded-v14"),
                aa_rules::sources::SourceLimits { max_member_bytes: 64 << 20 },
            )
            .map_err(|e| Error::Protocol(format!("opening the original data in {}: {e}", game_root.display())))
        };
        let key = format!("lan-registries-2:{}", aa_rules::sources::BASELINE_LEDGER_SHA256);
        if let Some(path) = cache
            && let Ok(bytes) = std::fs::read(path)
            && let Ok(cached) = serde_json::from_slice::<Self>(&bytes)
            && cached.source_key == key
        {
            return Ok(cached);
        }
        let mut sources = open()?;
        let catalog = aa_rules::actors::ActorCatalog::load_v14(&mut sources)
            .map_err(|e| Error::Protocol(format!("native-order catalog load: {e}")))?;
        let mut out = Self::from_catalog(&catalog)?;
        out.source_key = key;
        if let Some(path) = cache {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(bytes) = serde_json::to_vec(&out) {
                let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
                if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, path).is_err() {
                    let _ = std::fs::remove_file(&tmp);
                }
            }
        }
        Ok(out)
    }
}
