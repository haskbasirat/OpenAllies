//! Constructor/Create inputs resolved from authored ActorData. These identities
//! and parameters do not imply completed component or post-create lifecycles.
use crate::{
    abilities::DefinitionIdentity,
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct TrailerHitchInputs {
    pub bone: DefinitionIdentity,
    pub offset: TypedValue,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ActorInitInputs {
    pub actor_id: String,
    pub name_list_reference: TypedValue,
    pub name_list: Option<DefinitionIdentity>,
    pub numbered: TypedValue,
    pub trailer_reference: Option<TypedValue>,
    pub trailer: Option<DefinitionIdentity>,
    pub trailer_hitch: Option<TrailerHitchInputs>,
    /// Original4C4CF0 prepends each shared reference; this vector is native list
    /// traversal order, which is the reverse of the authored Property list.
    pub initial_properties: Vec<DefinitionIdentity>,
}
#[derive(Clone, Debug, Serialize)]
pub struct GroundPerformerConstructorInputs {
    pub energy_max: TypedValue,
    /// ActorData+1F8 turret +4 words (45DC30), from the native Turret loader.
    pub turret_yaw_bits: Vec<u32>,
    pub sources: Vec<Provenance>,
}
fn text(value: &TypedValue) -> Result<&str> {
    match value {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(Error::Unsupported(
            "definition reference must be string".into(),
        )),
    }
}
fn required(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<TypedValue> {
    node.field(name, pc)?
        .ok_or_else(|| Error::Unsupported(format!("Actor InitCreate field {name} missing")))
}
impl ActorCatalog {
    fn init_registry_identity(
        &self,
        kind: &str,
        id: &str,
        pc: Precision,
    ) -> Result<DefinitionIdentity> {
        if kind == "NameList" {
            let mut matches = Vec::new();
            for node in self.resolved_definitions(kind)? {
                let Some(value) = node.plain_string_field("IDS")? else {
                    continue;
                };
                if text(&value)?.eq_ignore_ascii_case(id) {
                    matches.push(DefinitionIdentity {
                        kind: kind.into(),
                        id: text(&value)?.into(),
                        sources: node.sources(),
                    });
                }
            }
            if matches.len() != 1 {
                return Err(Error::Unsupported(format!(
                    "NameList {id} has {} source declarations",
                    matches.len()
                )));
            }
            return Ok(matches.remove(0));
        }
        let node = self.resolved_definition(kind, id, pc)?;
        let id = required(&node, "IDS", pc)?;
        Ok(DefinitionIdentity {
            kind: kind.into(),
            id: text(&id)?.into(),
            sources: node.sources(),
        })
    }
    pub fn native_actor_init_inputs(&self, id: &str, pc: Precision) -> Result<ActorInitInputs> {
        if self.native_category(id)?.native_kind != 2 {
            return Err(Error::Unsupported(
                "Actor InitCreate inputs require standard ActorData".into(),
            ));
        }
        let actor = self.resolved_actor(id)?;
        let name_list_reference = required(&actor, "name_list_ids", pc)?;
        let name_list = if text(&name_list_reference)?.is_empty() {
            None
        } else {
            Some(self.init_registry_identity("NameList", text(&name_list_reference)?, pc)?)
        };
        let numbered = required(&actor, "numbered", pc)?;
        numbered.boolean()?;
        let mut trailer_reference = None;
        let mut trailer = None;
        let mut trailer_hitch = None;
        if let Some(mover) = actor.child("MoverComponent")? {
            let reference = required(&mover, "trailer_actor", pc)?;
            if !text(&reference)?.is_empty() {
                let id = text(&reference)?;
                let category = self.native_category(id)?;
                if category.registry_index != 0x27 {
                    return Err(Error::Unsupported(
                        "native trailer_actor must be TRAILER category39".into(),
                    ));
                }
                let node = self.resolved_actor(id)?;
                trailer = Some(DefinitionIdentity {
                    kind: "Thing".into(),
                    id: self
                        .find(id)
                        .ok_or_else(|| Error::Unsupported("trailer identity missing".into()))?
                        .id
                        .clone(),
                    sources: node.sources(),
                });
            }
            trailer_reference = Some(reference);
            if let Some(hitch) = mover.child("TrailerHitch")? {
                let bone_ref = required(&hitch, "node_ids", pc)?;
                let bone = self.init_registry_identity("Bone", text(&bone_ref)?, pc)?;
                let offset = required(&hitch, "node_offset", pc)?;
                if !matches!(&offset,TypedValue::Point{bits,..}if bits.len()==3) {
                    return Err(Error::Unsupported(
                        "native trailer hitch requires point3 offset".into(),
                    ));
                }
                trailer_hitch = Some(TrailerHitchInputs {
                    bone,
                    offset,
                    sources: hitch.sources(),
                });
            }
            if trailer.is_some() && trailer_hitch.is_none() {
                return Err(Error::Unsupported(
                    "native trailer_actor requires TrailerHitch".into(),
                ));
            }
        }
        let mut initial_properties = Vec::new();
        if let Some(properties) = actor.child("PropertiesComponent")? {
            for property in properties.children("Property")? {
                let reference = required(&property, "property_IDS", pc)?;
                initial_properties.push(self.init_registry_identity(
                    "Property",
                    text(&reference)?,
                    pc,
                )?);
            }
        }
        initial_properties.reverse();
        Ok(ActorInitInputs {
            actor_id: self
                .find(id)
                .ok_or_else(|| Error::Unsupported("Actor identity missing".into()))?
                .id
                .clone(),
            name_list_reference,
            name_list,
            numbered,
            trailer_reference,
            trailer,
            trailer_hitch,
            initial_properties,
        })
    }
    pub fn native_ground_performer_constructor(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<Option<GroundPerformerConstructorInputs>> {
        let Some(node) = self.resolved_actor(id)?.child("PerformerComponent")? else {
            return Ok(None);
        };
        let energy_max = required(&node, "mana_max", pc)?;
        energy_max.fixed_bits()?;
        let turret_yaw_bits = self
            .native_turrets(id, pc)?
            .iter()
            .map(|t| t.yaw_start_bits)
            .collect();
        Ok(Some(GroundPerformerConstructorInputs {
            energy_max,
            turret_yaw_bits,
            sources: node.sources(),
        }))
    }
}
