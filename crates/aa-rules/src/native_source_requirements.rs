//! Source-owned ActorData fields reached by creation placement.
//!
//! This is a guarded dependency slice of source completion, not a replacement
//! for 4BD800 or a claim that every unrelated deferred callback succeeded.
//! 4624B0 initializes +430/+444/+1A8/+290/+4E0 to zero. 4640F0 can replace
//! +430/+1A8, and 4BB480/45FBB0 can populate +444. Consequently the zero values
//! below require ALL registered standard/derived ActorData to have no resolved
//! SettlementComponent, and the complete Upgrade census to be empty. Nonempty
//! required_object_ids is also refused until its historical callback is owned.
use crate::{
    actors::{ActorCatalog, ConstructionUpgradeCensus, Diagnostic, LiteralReferenceCensus},
    native::{CATEGORIES, Error, Result, TypedValue},
    resources::{Provenance, SourceIdentity},
};
use aa_numeric::Precision;
use serde::Serialize;
use std::collections::BTreeMap;

fn invalid(message: impl Into<String>) -> Error {
    Error::Unsupported(message.into())
}
fn actor_kind(kind: u32) -> bool {
    matches!(kind, 2 | 19)
}
fn object_kind(kind: u32) -> bool {
    matches!(kind, 1 | 2 | 3 | 18 | 19 | 20 | 21)
}

/// Opaque identity of a registered source object, including its source PC.
/// An alias is never translated when this token is subsequently consumed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DefinitionToken {
    /// Portable arena slot only; never a native registration ordinal/address.
    arena_ordinal: usize,
    canonical_id: String,
    source: Provenance,
    source_precision: Precision,
}
impl DefinitionToken {
    pub fn canonical_id(&self) -> &str {
        &self.canonical_id
    }
    pub fn source(&self) -> &Provenance {
        &self.source
    }
}

#[derive(Debug, Serialize)]
struct AffiliationNodeToken {
    // Native +444 is a list-node pointer, not an ActorData definition pointer.
    // No nonempty node is certified by this bounded source producer.
    ordinal: usize,
}
#[derive(Debug, Serialize)]
pub struct ActorFields {
    settlement_owner: Option<DefinitionToken>,
    affiliation_head: Option<AffiliationNodeToken>,
    settlement_wall: u8,
    captureable: u8,
    required_object: Option<DefinitionToken>,
    /// Actual Body field or None only when constructor-zero remains in force.
    captureable_source: Option<TypedValue>,
    construction_sources: Vec<Provenance>,
}
#[derive(Debug, Serialize)]
pub struct Definition {
    token: DefinitionToken,
    native_kind: u32,
    registry_index: u32,
    sources: Vec<Provenance>,
    own_settlement_space: Option<TypedValue>,
    actor: Option<ActorFields>,
}
impl Definition {
    pub fn token(&self) -> &DefinitionToken {
        &self.token
    }
    pub fn native_kind(&self) -> u32 {
        self.native_kind
    }
    pub fn registry_index(&self) -> u32 {
        self.registry_index
    }
    pub fn sources(&self) -> &[Provenance] {
        &self.sources
    }
}

/// Proof inputs retained with the owner instead of accepting a caller's
/// boolean assertion that the graph was empty.
#[derive(Debug, Serialize)]
pub struct WriterReceipt {
    pub registered_things: usize,
    pub registered_actors: usize,
    pub registered_random_actors: usize,
    pub settlement_category_objects: usize,
    pub source_diagnostics: Vec<Diagnostic>,
    pub sources: Vec<SourceIdentity>,
    pub upgrade_census: ConstructionUpgradeCensus,
    /// Besides inspecting resolved roots, this checks hash-verified omitted
    /// source bytes cannot hide SettlementComponent/required_object_ids.
    pub writer_source_census: LiteralReferenceCensus,
}

/// Immutable current storage for the narrowly certified placement fields.
/// No Default/Deserialize constructor can manufacture an empty proof owner.
#[derive(Debug, Serialize)]
pub struct NativeSourceRequirements {
    source_precision: Precision,
    definitions: BTreeMap<String, Definition>,
    receipt: WriterReceipt,
}
impl NativeSourceRequirements {
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
    pub fn receipt(&self) -> &WriterReceipt {
        &self.receipt
    }
    pub fn definitions(&self) -> impl Iterator<Item = &Definition> {
        self.definitions.values()
    }
    /// Canonical registration identity only: no current Find or Translate.
    pub fn definition(&self, canonical_id: &str) -> Result<&Definition> {
        self.definitions
            .get(&canonical_id.to_ascii_lowercase())
            .ok_or_else(|| {
                invalid(format!(
                    "canonical source requirements identity unavailable: {canonical_id}"
                ))
            })
    }
    pub fn definition_by_token(&self, token: &DefinitionToken) -> Result<&Definition> {
        let entry = self.definition(&token.canonical_id)?;
        if &entry.token != token {
            return Err(invalid(
                "source requirements token belongs to different registration/source precision",
            ));
        }
        Ok(entry)
    }
    /// Only null/non-null predicates are portable. The returned value is a
    /// logical presence value (0/1), NEVER an emulated/native pointer address.
    pub fn word(&self, canonical_id: &str, offset: u32) -> Result<u32> {
        let entry = self.definition(canonical_id)?;
        let actor = entry
            .actor
            .as_ref()
            .ok_or_else(|| invalid("ActorData word requested on non-ActorData source"))?;
        match offset {
            0x430 => Ok(u32::from(actor.settlement_owner.is_some())),
            0x444 => Ok(u32::from(actor.affiliation_head.is_some())),
            _ => Err(invalid(format!(
                "source requirements word +{offset:X} is not certified"
            ))),
        }
    }
    pub fn byte(&self, canonical_id: &str, offset: u32) -> Result<u8> {
        let entry = self.definition(canonical_id)?;
        if offset == 0x16D {
            if !object_kind(entry.native_kind) {
                return Err(invalid(
                    "ObjectData +16D requested on non-ObjectData source",
                ));
            }
            //49C760 initializes this byte to zero.49C460 only sets it when
            //the type2 ActorData list contains a +4E0 reference to this object.
            //This owner's complete required-object/Upgrade writer receipt
            //excludes every such pointer, including45FAE0 inheritance.
            return Ok(0);
        }
        if offset == 0x34 {
            return entry
                .own_settlement_space
                .as_ref()
                .ok_or_else(|| invalid("ObjectData +34 requested on non-ObjectData source"))?
                .boolean()
                .map(u8::from);
        }
        let actor = entry
            .actor
            .as_ref()
            .ok_or_else(|| invalid("ActorData byte requested on non-ActorData source"))?;
        match offset {
            0x1A8 => Ok(actor.settlement_wall),
            0x290 => Ok(actor.captureable),
            _ => Err(invalid(format!(
                "source requirements byte +{offset:X} is not certified"
            ))),
        }
    }
    pub fn reference(&self, canonical_id: &str, offset: u32) -> Result<Option<&str>> {
        let entry = self.definition(canonical_id)?;
        let actor = entry
            .actor
            .as_ref()
            .ok_or_else(|| invalid("ActorData reference requested on non-ActorData source"))?;
        if offset != 0x4E0 {
            return Err(invalid(format!(
                "source requirements reference +{offset:X} is not certified"
            )));
        }
        Ok(actor
            .required_object
            .as_ref()
            .map(DefinitionToken::canonical_id))
    }
}

impl ActorCatalog {
    /// Build once from the actual source owner and its actual source precision.
    /// 4640F0/45FBB0 effects are excluded by resolved registered components plus
    /// an omitted-source guard; 45FAE0 inheritance is excluded by the complete
    /// Upgrade census. Postpasses 4BB130/4BB730, 45F7A0, 4BD440 and actual
    /// virtual+8C verifiers do not write these typed fields (see evidence).
    pub fn native_source_requirements(
        &self,
        source_precision: Precision,
    ) -> Result<NativeSourceRequirements> {
        if self.native_data_registry("NameList").is_none() {
            return Err(invalid(
                "source requirements need native registration order",
            ));
        }
        if self
            .diagnostics
            .iter()
            .any(|d| !d.message.starts_with("native syntax notice "))
        {
            return Err(invalid(
                "source requirements cannot certify unclassified source-load diagnostics",
            ));
        }
        let upgrade_census = self.construction_upgrade_census()?;
        if !upgrade_census.explicit_instances.is_empty() {
            return Err(invalid(
                "source requirements need completed nonempty Upgrade graph; +4E0 inheritance is not certified",
            ));
        }
        let writer_source_census =
            self.literal_reference_census(&["SettlementComponent", "required_object_ids"])?;
        // Missing Settlement/required-reference names alone does not exclude
        // an omitted file from reloading byte34/290 of an existing identity.
        // The current shipped receipt has no opaque files; broader omission
        // support needs its own native registration/reload ownership proof.
        if !writer_source_census
            .opaque_sources_without_field_names
            .is_empty()
        {
            return Err(invalid(
                "source requirements cannot certify canonical field/reload ownership with opaque source files",
            ));
        }
        let mut roots = Vec::new();
        for tag in crate::actors::native_load::THING_TAGS {
            roots.extend(self.resolved_definitions(tag)?);
        }
        // actors/sources are public projections. The complete private resolver
        // set must agree before its absence of shared writers can certify zero
        // fields; otherwise deleting a public entry could hide a Settlement.
        if roots.len() != self.actors.len() {
            return Err(invalid(
                "source requirements registered projection differs from the complete private Thing-root set",
            ));
        }
        // Indexes over this call's borrowed data; each lookup yields exactly
        // the matches (in order) of the linear filter it replaces.
        let mut actors_by_source: std::collections::HashMap<&Provenance, Vec<usize>> =
            std::collections::HashMap::new();
        for (index, actor) in self.actors.iter().enumerate() {
            actors_by_source.entry(&actor.source).or_default().push(index);
        }
        let mut identity_counts: std::collections::HashMap<(&str, &str), usize> =
            std::collections::HashMap::new();
        for identity in &self.sources {
            *identity_counts
                .entry((identity.path.as_str(), identity.sha256.as_str()))
                .or_default() += 1;
        }
        let mut roots_by_source: std::collections::HashMap<&Provenance, Vec<usize>> =
            std::collections::HashMap::new();
        for (index, node) in roots.iter().enumerate() {
            if let Some(source) = node.first_source() {
                roots_by_source.entry(source).or_default().push(index);
            }
        }
        for node in &roots {
            let sources = node.sources();
            let root = sources
                .first()
                .ok_or_else(|| invalid("private Thing root provenance missing"))?;
            let mut matching = actors_by_source
                .get(root)
                .map(Vec::as_slice)
                .unwrap_or(&[])
                .iter()
                .map(|&index| &self.actors[index]);
            let actor = matching
                .next()
                .ok_or_else(|| invalid("private Thing root omitted from registered projection"))?;
            if matching.next().is_some() {
                return Err(invalid(
                    "private Thing root has duplicate public registrations",
                ));
            }
            let Some(TypedValue::String { value: id, .. }) = node.field("IDS", source_precision)?
            else {
                return Err(invalid("private Thing root IDS must be an assigned string"));
            };
            if id != actor.id || sources != actor.template_chain {
                return Err(invalid(
                    "public registered IDS/template chain differs from its private Thing root",
                ));
            }
            for source in &sources {
                if identity_counts
                    .get(&(source.path.as_str(), source.sha256.as_str()))
                    .copied()
                    .unwrap_or(0)
                    != 1
                {
                    return Err(invalid(
                        "registered Thing/template source identity missing or duplicated in public source universe",
                    ));
                }
            }
        }
        let mut definitions = BTreeMap::new();
        let mut registered_actors = 0;
        let mut registered_random_actors = 0;
        let mut settlement_category_objects = 0;
        for (ordinal, registered) in self.actors.iter().enumerate() {
            let mut candidates = roots_by_source
                .get(&registered.source)
                .map(Vec::as_slice)
                .unwrap_or(&[])
                .iter()
                .map(|&index| &roots[index]);
            let node = candidates.next().ok_or_else(|| {
                invalid(format!(
                    "registered source requirements root unavailable: {}",
                    registered.id
                ))
            })?;
            if candidates.next().is_some() {
                return Err(invalid("registered source requirements root is ambiguous"));
            }
            let Some(TypedValue::String {
                value: category, ..
            }) = node.plain_string_field("category")?
            else {
                return Err(invalid(
                    "source requirements category must be an assigned string",
                ));
            };
            let category = CATEGORIES
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(&category))
                .ok_or_else(|| invalid("source requirements category unknown"))?;
            if category.registry_index == 21 {
                settlement_category_objects += 1;
            }
            let token = DefinitionToken {
                arena_ordinal: ordinal,
                canonical_id: registered.id.clone(),
                source: registered.source.clone(),
                source_precision,
            };
            let own_settlement_space = if object_kind(category.native_kind) {
                let field = node
                    .field("needs_space_for_settlement_zone", source_precision)?
                    .ok_or_else(|| {
                        invalid(format!(
                            "{} needs_space_for_settlement_zone missing",
                            registered.id
                        ))
                    })?;
                field.boolean()?;
                Some(field)
            } else {
                None
            };
            let actor = if actor_kind(category.native_kind) {
                registered_actors += 1;
                if category.native_kind == 19 {
                    registered_random_actors += 1;
                }
                if node.child("SettlementComponent")?.is_some() {
                    return Err(invalid(format!(
                        "{} has SettlementComponent: completed shared +430/+444/+1A8 writers required",
                        registered.id
                    )));
                }
                let captureable_source = node
                    .child("BodyComponent")?
                    .map(|body| {
                        body.field("captureable", source_precision)?.ok_or_else(|| {
                            invalid(format!("{} Body.captureable missing", registered.id))
                        })
                    })
                    .transpose()?;
                let captureable = captureable_source
                    .as_ref()
                    .map(|v| v.boolean().map(u8::from))
                    .transpose()?
                    .unwrap_or(0);
                let construction = node.child("ConstructionComponent")?;
                if let Some(construction) = &construction {
                    let Some(TypedValue::String { value, .. }) =
                        construction.field("required_object_ids", source_precision)?
                    else {
                        return Err(invalid(
                            "ConstructionComponent.required_object_ids must be an assigned string",
                        ));
                    };
                    if !value.is_empty() {
                        return Err(invalid(format!(
                            "{} has required_object_ids {value:?}: actual historical deferred reference ownership required",
                            registered.id
                        )));
                    }
                }
                Some(ActorFields {
                    settlement_owner: None,
                    affiliation_head: None,
                    settlement_wall: 0,
                    captureable,
                    required_object: None,
                    captureable_source,
                    construction_sources: construction.map(|n| n.sources()).unwrap_or_default(),
                })
            } else {
                None
            };
            let key = registered.id.to_ascii_lowercase();
            if definitions
                .insert(
                    key,
                    Definition {
                        token,
                        native_kind: category.native_kind,
                        registry_index: category.registry_index,
                        sources: node.sources(),
                        own_settlement_space,
                        actor,
                    },
                )
                .is_some()
            {
                return Err(invalid("duplicate canonical source requirements identity"));
            }
        }
        Ok(NativeSourceRequirements {
            source_precision,
            definitions,
            receipt: WriterReceipt {
                registered_things: self.actors.len(),
                registered_actors,
                registered_random_actors,
                settlement_category_objects,
                source_diagnostics: self.diagnostics.clone(),
                sources: self.sources.clone(),
                upgrade_census,
                writer_source_census,
            },
        })
    }
}

#[cfg(test)]
mod memory_byte_tests {
    use super::*;

    #[test]
    #[ignore = "requires original/v14 assets in AA_GAME_ROOT"]
    fn actual_complete_source_receipt_certifies_object16d_all_precisions() {
        use crate::sources::{SourceLimits, V14Sources};
        let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut sources = V14Sources::open(
            game.join("Data.rwd"),
            game.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
        let native: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-source-memory-byte/native-source-memory.json"
        ))
        .unwrap();
        let mut captured = Vec::new();
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let fields = catalog.native_source_requirements(pc).unwrap();
            let receipt = fields.receipt();
            assert_eq!(receipt.registered_things, 1959);
            assert_eq!(receipt.registered_actors, 1084);
            assert_eq!(receipt.registered_random_actors, 56);
            let reference = native["actual_rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["precision"] == serde_json::to_value(pc).unwrap())
                .unwrap();
            assert_eq!(reference["actor_count"], receipt.registered_actors);
            let mut values = Vec::new();
            for entry in fields.definitions() {
                if object_kind(entry.native_kind()) {
                    let actual = fields.byte(entry.token().canonical_id(), 0x16d).unwrap();
                    assert_eq!(serde_json::json!(actual), reference["output"]["byte16d"]);
                    values.push(serde_json::json!({"id":entry.token().canonical_id(),"native_kind":entry.native_kind(),"source":entry.token().source(),"byte16d":actual}));
                } else {
                    assert!(fields.byte(entry.token().canonical_id(), 0x16d).is_err());
                }
            }
            eprintln!(
                "{pc:?}: {} actual ObjectData16D values,1084 null required-object references",
                values.len()
            );
            captured
                .push(serde_json::json!({"source_precision":pc,"receipt":receipt,"values":values}));
        }
        if let Some(path) = std::env::var_os("AA_SOURCE_MEMORY_CAPTURE") {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap();
            serde_json::to_writer_pretty(file, &serde_json::json!({"cases":captured})).unwrap();
        }
    }
}
