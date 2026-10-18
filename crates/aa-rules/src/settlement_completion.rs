//! Settlement4640F0 shared-definition prefix and WallSet source identities.
//!
//! These are gameplay ObjectData shapes, not Gamebryo bounds. The fitting
//! routine460940, and therefore completed ActorData+478/+1E4, are NOT supplied
//! here. Calling code must keep source-load order and current shared storage.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
    objects::Shape,
    resources::Provenance,
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::collections::BTreeMap;

fn invalid(message: impl Into<String>) -> Error {
    Error::Unsupported(message.into())
}

#[derive(Clone, Debug, Serialize)]
pub struct ActorReference {
    pub authored_ids: String,
    pub canonical_id: String,
    pub sources: Vec<Provenance>,
    pub component_mask: u32,
    pub flags: u32,
    /// Initial ObjectData49DC40 shape words. Later Settlement mutations are
    /// deliberately not represented as if these immutable inputs were final.
    pub initial_blocking: Shape,
    pub initial_soft: Shape,
}

#[derive(Clone, Debug, Serialize)]
pub struct WallSetInputs {
    pub canonical_id: String,
    pub source: Provenance,
    /// Native +18 segment,+1C corner,+20 gate, in that order.
    pub pieces: [ActorReference; 3],
}

#[derive(Clone, Debug, Serialize)]
pub struct SettlementRegistration {
    pub canonical_id: String,
    pub source: Provenance,
    pub component_sources: Vec<Provenance>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RegistrationCensus {
    pub actor_registrations: usize,
    pub wall_set_registrations: usize,
    pub registered_actors: Vec<SettlementRegistration>,
    pub settlement_actors: Vec<SettlementRegistration>,
    pub sources: Vec<crate::resources::SourceIdentity>,
    /// Retain source loader notices/failures for independent classification.
    /// An empty Settlement list alone is not a completed source-load proof.
    pub source_diagnostics: Vec<crate::actors::Diagnostic>,
}

impl ActorCatalog {
    /// Inspect every registered kind2 definition by REGISTRATION provenance,
    /// including canonical IDS subsequently shadowed by a Translate. These
    /// are inputs to a reached-dependency audit, not a completion certificate.
    pub fn native_settlement_registration_census(
        &self,
        pc: Precision,
    ) -> Result<RegistrationCensus> {
        let wall_sets = self
            .native_data_registry("WallSet")
            .ok_or_else(|| invalid("Settlement census requires native registry loading"))?;
        let mut nodes = Vec::new();
        for tag in crate::actors::native_load::THING_TAGS {
            nodes.extend(self.resolved_definitions(tag)?);
        }
        let mut actor_registrations = 0;
        let mut registered_actors = Vec::new();
        let mut settlement_actors = Vec::new();
        for actor in &self.actors {
            let mut matching = nodes
                .iter()
                .filter(|node| node.sources().first() == Some(&actor.source));
            let node = matching
                .next()
                .ok_or_else(|| invalid("registered Settlement census source missing"))?;
            if matching.next().is_some() {
                return Err(invalid("registered Settlement census source ambiguous"));
            }
            let Some(TypedValue::String { value, .. }) = node.field("category", pc)? else {
                return Err(invalid("registered Settlement census category absent"));
            };
            let category = crate::native::CATEGORIES
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(&value))
                .ok_or_else(|| invalid("registered Settlement census category unknown"))?;
            if category.native_kind != 2 {
                continue;
            }
            actor_registrations += 1;
            let entry = SettlementRegistration {
                canonical_id: actor.id.clone(),
                source: actor.source.clone(),
                component_sources: node
                    .child("SettlementComponent")?
                    .map(|c| c.sources())
                    .unwrap_or_default(),
            };
            if !entry.component_sources.is_empty() {
                settlement_actors.push(entry.clone());
            }
            registered_actors.push(entry);
        }
        Ok(RegistrationCensus {
            actor_registrations,
            wall_set_registrations: wall_sets.entries.len(),
            registered_actors,
            settlement_actors,
            sources: self.sources.clone(),
            source_diagnostics: self.diagnostics.clone(),
        })
    }

    /// Read immutable inputs of an actual registered WallSet using CURRENT
    /// registry identities. This is not the completed native WallSet object:
    ///47DFB9 resolves at load time, so a later Translate/registration may make
    /// this projection differ. A runtime binding MUST establish that historical
    /// identity separately. These inputs never supply completed shapes or CV.
    /// The three references use one-level Thing Find; canonical IDS shadowing
    /// remains unsupported by the immutable ObjectData helpers.
    pub fn native_wall_set_inputs(&self, ids: &str, pc: Precision) -> Result<WallSetInputs> {
        let registry = self
            .native_data_registry("WallSet")
            .ok_or_else(|| invalid("WallSet requires native registry loading"))?;
        let entry = registry
            .find(ids)
            .ok_or_else(|| invalid(format!("unknown WallSet {ids}")))?;
        if !entry.reloads.is_empty() {
            return Err(invalid("WallSet reload requires actual completed storage"));
        }
        let node = self
            .native_named_definition("WallSet", ids, pc)?
            .ok_or_else(|| invalid("registered WallSet source missing"))?;
        let reference = |field: &str| -> Result<ActorReference> {
            let Some(TypedValue::String { value, .. }) = node.field(field, pc)? else {
                return Err(invalid(format!(
                    "WallSet {field} must be an assigned string"
                )));
            };
            let actor = self
                .find(&value)
                .ok_or_else(|| invalid(format!("unknown WallSet actor {value}")))?;
            let canonical = self
                .find(&actor.id)
                .ok_or_else(|| invalid("WallSet canonical ActorData identity missing"))?;
            if canonical.source != actor.source || canonical.id != actor.id {
                return Err(invalid(
                    "WallSet canonical ActorData IDS is translated elsewhere",
                ));
            }
            if self.native_category(&value)?.native_kind != 2 {
                return Err(invalid("WallSet piece is not ActorData"));
            }
            let object = self.native_object_definition(&value, pc)?;
            Ok(ActorReference {
                authored_ids: value.clone(),
                canonical_id: actor.id.clone(),
                sources: self.resolved_actor(&value)?.sources(),
                component_mask: self.native_component_mask_at(&value, None, pc)?.bits,
                flags: self.native_final_flags(&value, pc)?,
                initial_blocking: object.blocking,
                initial_soft: object.soft,
            })
        };
        let pieces = [
            reference("wall_segment_ids")?,
            reference("wall_corner_ids")?,
            reference("gate_ids")?,
        ];
        //47DFF8..47E06D: null references are rejected before component flags,
        //then all three require bit12; only the CORNER checks targetable14.
        if pieces
            .iter()
            .any(|piece| piece.component_mask & (1 << 12) == 0)
        {
            return Err(invalid("WallSet pieces must have WallComponent"));
        }
        if pieces[1].flags & (1 << 14) != 0 {
            return Err(invalid("WallSet corner must not be targetable"));
        }
        Ok(WallSetInputs {
            canonical_id: entry.ids.clone(),
            source: entry.source.clone(),
            pieces,
        })
    }
}

/// Current shared ActorData fields touched by46450B..464909. A caller must
/// initialize these from the actual definition owner and pass the same store
/// across all Settlement loads. A fresh per-Settlement snapshot is incorrect.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SharedActor {
    pub component_mask: u32,
    pub blocking: Shape,
    pub soft: Shape,
    pub settlement_owner: Option<String>,
    pub settlement_wall: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WallSet {
    pub id: String,
    /// Canonical ActorData identities: segment,corner,gate.
    pub pieces: [String; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Level {
    pub maximum_structures: i32,
    pub center: Option<String>,
    pub wall_set: Option<WallSet>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SharedDefinitions {
    /// Canonical lowercase ASCII IDS keys; values remain shared mutable data.
    pub actors: BTreeMap<String, SharedActor>,
}

impl SharedDefinitions {
    fn actor(&self, id: &str) -> Result<&SharedActor> {
        self.actors
            .get(&id.to_ascii_lowercase())
            .ok_or_else(|| invalid(format!("current Settlement ActorData unavailable: {id}")))
    }
    fn actor_mut(&mut self, id: &str) -> Result<&mut SharedActor> {
        self.actors
            .get_mut(&id.to_ascii_lowercase())
            .ok_or_else(|| invalid(format!("current Settlement ActorData unavailable: {id}")))
    }
    fn copy_shapes(&mut self, source: &[String; 3], target: &[String; 3]) -> Result<()> {
        // Native copies all three blocking shapes before any soft shape.
        // Read each source at the instant its copy is reached: cross-piece
        // aliases can make an earlier destination a later source.
        for soft in [false, true] {
            for piece in 0..3 {
                let actor = self.actor(&source[piece])?;
                let shape = if soft { &actor.soft } else { &actor.blocking }.clone();
                let target = self.actor_mut(&target[piece])?;
                if soft {
                    target.soft = shape;
                } else {
                    target.blocking = shape;
                }
            }
        }
        Ok(())
    }

    ///4644DD..46469A after Max/Find calls: append the loaded Level words,
    /// validate Structure bit13 and existing owner, store center+430, validate
    /// WallSet, then mutate current shared shapes relative to Level0.
    /// Errors retain every preceding mutation. This is a bounded prefix;
    /// allocations, parsing and later fitting are separate required services.
    pub fn append_level(
        &mut self,
        owner: &str,
        levels: &mut Vec<Level>,
        level: Level,
    ) -> Result<()> {
        levels.push(level);
        let level = levels.last().unwrap();
        let center = level
            .center
            .as_deref()
            .ok_or_else(|| invalid("unknown Settlement town center"))?;
        let center_state = self.actor(center)?;
        if center_state.component_mask & (1 << 13) == 0 {
            return Err(invalid(
                "Settlement town center requires StructureComponent",
            ));
        }
        if center_state.settlement_owner.is_some() {
            return Err(invalid("Settlement town center already has an owner"));
        }
        self.actor_mut(center)?.settlement_owner = Some(owner.to_owned());
        let current = level
            .wall_set
            .as_ref()
            .ok_or_else(|| invalid("unknown Settlement WallSet"))?;
        let first = levels[0]
            .wall_set
            .as_ref()
            .ok_or_else(|| invalid("first Settlement WallSet unavailable"))?;
        if !current.id.eq_ignore_ascii_case(&first.id) {
            self.copy_shapes(&first.pieces, &current.pieces)?;
        }
        Ok(())
    }

    ///4647B1..464909: first all six shape copies, then the three +1A8 bytes.
    /// Target shapes at+68, flags and wall radius are intentionally untouched.
    pub fn initialize_placeholders(
        &mut self,
        first: &WallSet,
        placeholders: &[String; 3],
    ) -> Result<()> {
        self.copy_shapes(&first.pieces, placeholders)?;
        for id in placeholders {
            self.actor_mut(id)?.settlement_wall = 1;
        }
        Ok(())
    }
}

///46491C..46492F: initial authored radius, before the REQUIRED460940 fit.
/// This is never a completed radius and must not be published as ActorData+478.
pub fn initial_radius(wall_diameter_bits: u32, pc: Precision) -> Result<u32> {
    Ok(Binary::from_f32_bits(wall_diameter_bits)?
        .mul(&Binary::from_f32_bits(0x3f000000)?, pc)?
        .store_f32()?)
}
