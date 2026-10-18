//! Source inputs for ordinary Actor collision policy. Positive Settlement wall
//! marks require its full side-effect loader; this API fails closed for them.
use crate::{
    abilities::DefinitionIdentity,
    actors::{ActorCatalog, ConstructionUpgradeCensus, LiteralReferenceCensus},
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct CollisionDefinitionInputs {
    pub id: String,
    pub required_object: Option<DefinitionIdentity>,
    pub gate: bool,
    pub gate_source: Option<TypedValue>,
    pub settlement_wall: bool,
    pub sources: Vec<Provenance>,
    pub wall_reference_census: LiteralReferenceCensus,
    pub upgrade_census: ConstructionUpgradeCensus,
}
impl ActorCatalog {
    pub fn native_collision_definition(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<CollisionDefinitionInputs> {
        if self.native_category(id)?.native_kind != 2 {
            return Err(Error::Unsupported(
                "ordinary collision policy requires ActorData".into(),
            ));
        }
        let node = self.resolved_actor(id)?;
        let canonical = self
            .find(id)
            .ok_or_else(|| Error::Unsupported("collision actor identity missing".into()))?
            .id
            .clone();
        let wall_reference_census = self.literal_reference_census(&[
            "wall_segment_placeholder_ids",
            "wall_corner_placeholder_ids",
            "gate_placeholder_ids",
        ])?;
        if wall_reference_census
            .references
            .iter()
            .any(|r| r.id.eq_ignore_ascii_case(&canonical))
        {
            return Err(Error::Unsupported("possible Settlement placeholder reference requires full positive wall side-effect binding".into()));
        }
        let upgrade_census = self.construction_upgrade_census()?;
        if !upgrade_census.explicit_instances.is_empty() {
            return Err(Error::Unsupported(
                "required_object inheritance requires resolved nonempty Upgrade graph".into(),
            ));
        }
        let gate_source = node
            .child("WallComponent")?
            .map(|n| {
                n.field("gate", pc)?
                    .ok_or_else(|| Error::Unsupported("WallComponent.gate missing".into()))
            })
            .transpose()?;
        // ActorData constructor462B18 explicitly initializes absent Wall.gate0.
        let gate = gate_source
            .as_ref()
            .map(TypedValue::boolean)
            .transpose()?
            .unwrap_or(false);
        let mut required_object = None;
        if let Some(construction) = node.child("ConstructionComponent")? {
            let Some(TypedValue::String { value, .. }) =
                construction.field("required_object_ids", pc)?
            else {
                return Err(Error::Unsupported(
                    "Construction required_object_ids missing/string required".into(),
                ));
            };
            if !value.is_empty() {
                let category = self.native_category(&value)?;
                if category.registry_index != 0x20 {
                    return Err(Error::Unsupported(
                        "native required object must have MARKER category32".into(),
                    ));
                }
                let shape = self.native_object_definition(&canonical, pc)?;
                let marker_shape = self.native_object_definition(&value, pc)?;
                if Binary::from_f32_bits(shape.soft.words[1])?
                    .compare(&Binary::from_f32_bits(marker_shape.soft.words[1])?)
                    == std::cmp::Ordering::Less
                {
                    return Err(Error::Unsupported(
                        "required object soft shape exceeds actor soft shape".into(),
                    ));
                }
                if (self.native_final_flags(&canonical, pc)?
                    ^ self.native_final_flags(&value, pc)?)
                    & (1 << 16)
                    != 0
                {
                    return Err(Error::Unsupported(
                        "required object tile_locked flag differs".into(),
                    ));
                }
                required_object = Some(DefinitionIdentity {
                    kind: "Thing".into(),
                    id: self
                        .find(&value)
                        .ok_or_else(|| {
                            Error::Unsupported("required object identity missing".into())
                        })?
                        .id
                        .clone(),
                    sources: self.resolved_actor(&value)?.sources(),
                });
            }
        }
        // ActorData ctor46257A initializes this byte0. Complete conservative
        // reference absence proves no Settlement4640F0 write can set it to1.
        Ok(CollisionDefinitionInputs {
            id: canonical,
            required_object,
            gate,
            gate_source,
            settlement_wall: false,
            sources: node.sources(),
            wall_reference_census,
            upgrade_census,
        })
    }
}
