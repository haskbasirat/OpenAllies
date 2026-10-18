//! Only the native DenizenData guard-range projection used by Detection.
//! Component/AI, Organization, Runway, Deployment and deferred references remain
//! separate loaders; this snapshot does not authorize Denizen construction/use.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, ScalarType, TypedScalar, TypedValue},
    resources::Provenance,
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::cmp::Ordering;
#[derive(Clone, Debug, Serialize)]
pub struct InventoryInput {
    pub actor_id: TypedValue,
    pub count: TypedValue,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct DenizenGuards {
    pub guard_range: TypedScalar,
    pub operational_range: TypedScalar,
    pub finalized_bits: [u32; 2],
    pub inventory: Vec<InventoryInput>,
    pub sources: Vec<Provenance>,
}
/// Original4670C6..467145 followed by4676AD..4676E6. The operational range
/// receives guard+2 only when authored zero and guard positive. Empty Inventory
/// then clears only guard_range; operational range retains the earlier result.
pub fn finalize_guards(
    guard: u32,
    operational: u32,
    inventory_present: bool,
    pc: Precision,
) -> Result<[u32; 2]> {
    let g = Binary::from_f32_bits(guard)?;
    let mut o = Binary::from_f32_bits(operational)?;
    if o.is_zero() && g.compare(&Binary::from_u32(0)) == Ordering::Greater {
        o = Binary::from_f32_bits(
            g.add(&Binary::from_f32_bits(0x40000000)?, pc)?
                .store_f32()?,
        )?;
    }
    if g.compare(&Binary::from_u32(0)) == Ordering::Greater && o.compare(&g) != Ordering::Greater {
        return Err(Error::Unsupported(
            "native Denizen guard operational range must exceed positive guard range".into(),
        ));
    }
    Ok([if inventory_present { guard } else { 0 }, o.store_f32()?])
}
impl ActorCatalog {
    pub fn native_denizen_guards(&self, id: &str, pc: Precision) -> Result<Option<DenizenGuards>> {
        let Some(node) = self.resolved_actor(id)?.child("DenizenComponent")? else {
            return Ok(None);
        };
        let fixed = |name: &str| -> Result<TypedScalar> {
            match node.field(name, pc)? {
                Some(TypedValue::Scalar(value)) if value.field_type == ScalarType::Fixed => {
                    Ok(value)
                }
                _ => Err(Error::Unsupported(format!(
                    "Denizen {name} must be assigned Fixed"
                ))),
            }
        };
        let guard_range = fixed("guard_range")?;
        let operational_range = fixed("guard_operational_range")?;
        let mut inventory = Vec::new();
        for item in node.children("Inventory")? {
            let actor_id = item
                .field("actor_ids", pc)?
                .ok_or_else(|| Error::Unsupported("Denizen Inventory actor_ids missing".into()))?;
            if !matches!(&actor_id, TypedValue::String { .. }) {
                return Err(Error::Unsupported(
                    "Denizen Inventory actor_ids must be String".into(),
                ));
            }
            let count = item
                .field("count", pc)?
                .ok_or_else(|| Error::Unsupported("Denizen Inventory count missing".into()))?;
            count.integer()?;
            inventory.push(InventoryInput {
                actor_id,
                count,
                sources: item.sources(),
            });
        }
        let finalized_bits = finalize_guards(
            guard_range.bits,
            operational_range.bits,
            !inventory.is_empty(),
            pc,
        )?;
        Ok(Some(DenizenGuards {
            guard_range,
            operational_range,
            finalized_bits,
            inventory,
            sources: node.sources(),
        }))
    }
}
