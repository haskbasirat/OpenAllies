//! Native definition-registry counts for decree payload decoding.
//!
//! A payload definition reference is a 0x411FA0 read over `count + 1`
//! possibilities, so its bit width depends on the live registry count
//! (`[registry]+0x28`). An unknown count is an explicit decode error, never a
//! guessed zero.
//!
//! Proven counts:
//! * ActorCommand `[0xB3AA58]`: the 39 code-predefined commands
//!   (`aa_rules::native_actor_command::PREDEFINED_ACTOR_COMMANDS`, registered
//!   by 0x475870 before any source load). Always present.
//! * Thing DataManager `[0xB39F58]`: `ActorCatalog::actors` of the
//!   native-order load (`load_v14`), the 4BCA00 registration slots
//!   (aa-runtime `player_order_object_reference.rs`, proven by
//!   research/phase8-orders-input/recruit/native-object-reference.json).
//!
//! Inferred counts (opt-in, [`Registries::from_catalog`] with
//! `include_inferred`): the generic DataManagers Formation `[0xB3AB08]`,
//! OrganizationLayout `[0xB3AB00]` and AbilityDefinition `[0xB3AB10]`. The
//! number of entries registered by the native-order load is assumed to equal
//! the live count, which is not oracle-proven. SharedProperty `[0xB3AA2C]` has
//! no source here.
use aa_orders::payload::{Registry, SchemaContext};
use std::collections::BTreeMap;

pub const ACTOR_COMMANDS: Registry = Registry(0xb3aa58);
pub const OBJECT_DEFINITIONS: Registry = Registry(0xb39f58);
pub const FORMATIONS: Registry = Registry(0xb3ab08);
pub const ORGANIZATION_LAYOUTS: Registry = Registry(0xb3ab00);
pub const SHARED_PROPERTIES: Registry = Registry(0xb3aa2c);
pub const ABILITIES: Registry = Registry(0xb3ab10);

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
        0xb3aa70 => "Condition",
        0xb3aa44 => "Nation",
        0xb3aa4c => "Faction",
        0xb3aa60 => "KingdomColor",
        0xb3aa68 => "Handicap",
        _ => "unknown",
    }
}

/// Where a count came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Provenance {
    Proven,
    Inferred,
}

#[derive(Clone, Debug)]
pub struct Registries {
    counts: BTreeMap<u32, (usize, Provenance)>,
    /// Thing DataManager slot → definition IDS (native load order), when
    /// loaded from a catalog. Index = the wire value of an object reference.
    pub object_ids: Vec<String>,
    /// Generic DataManager slot → IDS for the inferred registries.
    pub formation_ids: Vec<String>,
    pub ability_ids: Vec<String>,
}

impl Default for Registries {
    fn default() -> Self {
        Self::builtin()
    }
}

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
        }
    }
    /// Set one count explicitly (tests, or a count measured elsewhere).
    pub fn with_count(mut self, registry: Registry, count: usize, provenance: Provenance) -> Self {
        self.counts.insert(registry.0, (count, provenance));
        self
    }
    /// Counts of a native-order catalog (`ActorCatalog::load_v14`).
    pub fn from_catalog(catalog: &aa_rules::actors::ActorCatalog, include_inferred: bool) -> Self {
        let mut out = Self::builtin();
        out.object_ids = catalog.actors.iter().map(|a| a.id.clone()).collect();
        out.counts.insert(
            OBJECT_DEFINITIONS.0,
            (out.object_ids.len(), Provenance::Proven),
        );
        if include_inferred {
            let ids = |tag: &str| -> Option<Vec<String>> {
                catalog
                    .native_data_registry(tag)
                    .map(|r| r.entries.iter().map(|e| e.ids.clone()).collect())
            };
            if let Some(f) = ids("Formation") {
                out.counts
                    .insert(FORMATIONS.0, (f.len(), Provenance::Inferred));
                out.formation_ids = f;
            }
            if let Some(l) = ids("OrganizationLayout") {
                out.counts
                    .insert(ORGANIZATION_LAYOUTS.0, (l.len(), Provenance::Inferred));
            }
            if let Some(a) = ids("AbilityDefinition") {
                out.counts
                    .insert(ABILITIES.0, (a.len(), Provenance::Inferred));
                out.ability_ids = a;
            }
        }
        out
    }
    /// Load the native-order v14 catalog from the game root (Data.rwd plus
    /// `reconstruction/native-game/runtime-expanded-v14`, the data the
    /// shipped QA films were recorded with: their AA.exe is the v14 build).
    pub fn load_v14(game_root: &std::path::Path, include_inferred: bool) -> anyhow::Result<Self> {
        let mut sources = aa_rules::sources::V14Sources::open(
            game_root.join("Data.rwd"),
            game_root
                .join("reconstruction")
                .join("native-game")
                .join("runtime-expanded-v14"),
            aa_rules::sources::SourceLimits {
                max_member_bytes: 64 << 20,
            },
        )
        .map_err(|e| {
            anyhow::anyhow!("opening the original data in {}: {e}", game_root.display())
        })?;
        let catalog = aa_rules::actors::ActorCatalog::load_v14(&mut sources)
            .map_err(|e| anyhow::anyhow!("native-order catalog load: {e}"))?;
        Ok(Self::from_catalog(&catalog, include_inferred))
    }
    pub fn count(&self, registry: Registry) -> Option<(usize, Provenance)> {
        self.counts.get(&registry.0).copied()
    }
    /// Every known count, for reports.
    pub fn counts(&self) -> impl Iterator<Item = (Registry, usize, Provenance)> + '_ {
        self.counts.iter().map(|(&r, &(n, p))| (Registry(r), n, p))
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
    fn team_command_flags(&self, _index: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext(
            "film decode: TeamCommand definition flags not loaded",
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
