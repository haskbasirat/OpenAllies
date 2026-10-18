//! Typed authored ActorState inputs. Native descriptor group/ordinal selection
//! remains in aa-actors; optional source fields do not invent those descriptors.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct MindStateInputs {
    pub resupply: TypedValue,
    pub morale_replenish: TypedValue,
    pub entrenchable: TypedValue,
    pub remove_from_group: TypedValue,
    pub formation_required: TypedValue,
    pub behavior_required: TypedValue,
}
#[derive(Clone, Debug, Serialize)]
pub struct ActorStateInputs {
    pub id: String,
    pub priority: TypedValue,
    pub state_type: TypedValue,
    pub mind: Option<MindStateInputs>,
    pub properties: Vec<TypedValue>,
    pub sources: Vec<Provenance>,
}
impl ActorCatalog {
    pub fn native_actor_state(&self, id: &str, pc: Precision) -> Result<ActorStateInputs> {
        let node = self.resolved_definition("ActorState", id, pc)?;
        let required = |name: &str| {
            node.field(name, pc)?
                .ok_or_else(|| Error::Unsupported(format!("ActorState {id}.{name} missing")))
        };
        let mind = if node.field("resupply", pc)?.is_some() {
            let resupply = required("resupply")?;
            let morale_replenish = required("morale_replenish")?;
            let entrenchable = required("entrenchable")?;
            let remove_from_group = required("remove_from_group")?;
            for value in [
                &resupply,
                &morale_replenish,
                &entrenchable,
                &remove_from_group,
            ] {
                value.boolean()?;
            }
            Some(MindStateInputs {
                resupply,
                morale_replenish,
                entrenchable,
                remove_from_group,
                formation_required: required("formation_required")?,
                behavior_required: required("behavior_required")?,
            })
        } else {
            None
        };
        let properties = node
            .children("Property")?
            .iter()
            .map(|n| {
                n.field("property_IDS", pc)?.ok_or_else(|| {
                    Error::Unsupported(format!("ActorState {id} Property ID missing"))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let TypedValue::String {
            value: canonical_id,
            ..
        } = required("IDS")?
        else {
            return Err(Error::Unsupported("ActorState IDS must be string".into()));
        };
        Ok(ActorStateInputs {
            id: canonical_id,
            priority: required("priority")?,
            state_type: required("type")?,
            mind,
            properties,
            sources: node.sources(),
        })
    }
}
