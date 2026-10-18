//! Source-bound Nation badge order. The native array is not a technology list.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct NationBadges {
    pub nation_id: String,
    pub sources: Vec<Provenance>,
    /// Authored resource expressions, before native formatting/resource lookup.
    /// Count/order are native48C9D9; existence and texture decoding are separate.
    pub textures: Vec<TypedValue>,
}
impl NationBadges {
    pub fn count(&self) -> Result<u32> {
        u32::try_from(self.textures.len())
            .map_err(|_| Error::Unsupported("Nation Badge count overflow".into()))
    }
}
impl ActorCatalog {
    pub fn native_nation_badges(&self, id: &str, pc: Precision) -> Result<NationBadges> {
        let node = self.resolved_definition("Nation", id, pc)?;
        let Some(TypedValue::String {
            value: nation_id, ..
        }) = node.field("IDS", pc)?
        else {
            return Err(Error::Unsupported(
                "Nation IDS missing/string type required".into(),
            ));
        };
        let mut textures = Vec::new();
        for badge in node.children("Badge")? {
            let field = badge
                .field("badge_texture", pc)?
                .ok_or_else(|| Error::Unsupported("Nation Badge texture missing".into()))?;
            if !matches!(field, TypedValue::String { .. }) {
                return Err(Error::Unsupported(
                    "Nation Badge texture must be string".into(),
                ));
            }
            textures.push(field);
        }
        Ok(NationBadges {
            nation_id,
            sources: node.sources(),
            textures,
        })
    }
}
