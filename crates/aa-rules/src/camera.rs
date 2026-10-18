//! Source CameraData parameters; the VFX reference is not a rendered resource.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, ScalarType, TypedScalar, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct CameraDefinitionInputs {
    pub id: String,
    pub sources: Vec<Provenance>,
    pub camera_vfx_reference: TypedValue,
    pub camera_distance: TypedScalar,
}
impl ActorCatalog {
    pub fn native_camera_inputs(&self, id: &str, pc: Precision) -> Result<CameraDefinitionInputs> {
        if self.native_category(id)?.native_kind != 20 {
            return Err(Error::Unsupported(
                "CameraData requires native kind20".into(),
            ));
        }
        let node = self.resolved_actor(id)?;
        let reference = node
            .field("camera_vfx_ids", pc)?
            .ok_or_else(|| Error::Unsupported("Camera VFX reference missing".into()))?;
        if !matches!(&reference,TypedValue::String{value,..}if !value.is_empty()) {
            return Err(Error::Unsupported(
                "Camera VFX must be a nonempty string reference".into(),
            ));
        }
        let Some(TypedValue::Scalar(distance)) = node.field("camera_distance", pc)? else {
            return Err(Error::Unsupported("camera_distance missing".into()));
        };
        if distance.field_type != ScalarType::Float {
            return Err(Error::Unsupported(
                "native Camera distance requires Float".into(),
            ));
        }
        Ok(CameraDefinitionInputs {
            id: self
                .find(id)
                .ok_or_else(|| Error::Unsupported("Camera identity missing".into()))?
                .id
                .clone(),
            sources: node.sources(),
            camera_vfx_reference: reference,
            camera_distance: distance,
        })
    }
}
