//! Bounded producer for the definition value read by ActorData::45D580(NULL).
//!
//! The standard ActorData constructor 4624B0 writes zero at +2C0 (46280C).
//! Its loader 464BD0 looks up ElementComponent at 466822; when present it
//! reads GetFixed("resupply_rate") into +2CC, then GetFixed("morale_value")
//! into +2C0 at 466896..4668A8. Getter 45D580 with a null modifier context
//! loads that binary32 value without arithmetic or clamping.
//!
//! This projects only those fields of a freshly constructed standard
//! ActorData, using the existing native typed/inherited resolver. It does
//! not claim to load ActorData or apply non-null Properties modifiers.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, ScalarType, TypedScalar, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ActorMoraleContribution {
    /// Definition +2C0, returned by 45D580(NULL), as binary32 bits.
    pub bits: u32,
    pub actor_source: Provenance,
    /// None means the ElementComponent is absent and 46280C's zero remains.
    /// It never means an absent or unassigned morale field was defaulted.
    pub morale_value: Option<TypedScalar>,
    /// The prior GetFixed read at 46688B is validated in native order.
    pub resupply_rate: Option<TypedScalar>,
}

/// Source side of the null-context ActorData morale getter, with provenance.
/// Unknown/non-ActorData kinds and incorrectly typed fields remain errors.
pub fn actor_morale_source(
    catalog: &ActorCatalog,
    id: &str,
    precision: Precision,
) -> Result<ActorMoraleContribution> {
    let actor = catalog
        .find(id)
        .ok_or_else(|| Error::Unsupported(format!("missing actor {id}")))?;
    // Callers already hold the resolved native ActorData identity. The public
    // catalog lookup includes Translate entries, so a canonical IDS can be
    // shadowed by another translation. Never follow that second translation.
    if !actor.id.eq_ignore_ascii_case(id) {
        return Err(Error::Unsupported(format!(
            "45D580 requires an unshadowed canonical ActorData identity: {id} resolves to {}",
            actor.id
        )));
    }
    if catalog.native_category(id)?.native_kind != 2 {
        return Err(Error::Unsupported(format!(
            "45D580 requires standard ActorData definition kind2: {id}"
        )));
    }
    let node = catalog.resolved_actor(id)?;
    let Some(element) = node.child("ElementComponent")? else {
        return Ok(ActorMoraleContribution {
            bits: 0,
            actor_source: actor.source.clone(),
            morale_value: None,
            resupply_rate: None,
        });
    };
    let fixed = |name: &str| -> Result<TypedScalar> {
        match element.field(name, precision)? {
            Some(TypedValue::Scalar(value)) if value.field_type == ScalarType::Fixed => Ok(value),
            _ => Err(Error::Unsupported(format!(
                "ElementComponent.{name} requires an assigned native fixed field for {id}"
            ))),
        }
    };
    let resupply_rate = fixed("resupply_rate")?;
    let morale_value = fixed("morale_value")?;
    Ok(ActorMoraleContribution {
        bits: morale_value.bits,
        actor_source: actor.source.clone(),
        morale_value: Some(morale_value),
        resupply_rate: Some(resupply_rate),
    })
}

/// Return exactly the binary32 definition value used by 45D580(NULL).
pub fn actor_morale_contribution(
    catalog: &ActorCatalog,
    id: &str,
    precision: Precision,
) -> Result<u32> {
    Ok(actor_morale_source(catalog, id, precision)?.bits)
}
