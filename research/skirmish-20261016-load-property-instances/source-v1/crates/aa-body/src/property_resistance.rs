//! Native RESIST_DAMAGE kind5 aggregation502440/5020A0/4FE7B0.
//! The ordered type entry persists after the last source property is removed.
use crate::{
    Error, F32, Result,
    credit::Object,
    properties::{DamageFactor, PropertiesState},
    property_effects::{Definitions, Instances},
};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResistDamage {
    pub damage_type: Object,
    pub amount: F32,
}
/// Matches kind5 plus exact loaded DamageType identity, accumulating every
/// active matching descriptor in native list order with a binary32 store after
/// each addition. The final factor is2-(1+sum), then minimum-multiplier clamped.
/// The extra native+8 field records(1+sum)-1 before that clamp, not the authored
/// amount copied directly. This preserves precision loss around the base1.
pub fn recompute<M>(
    modifier: &ResistDamage,
    state: &mut PropertiesState,
    instances: &Instances,
    definitions: &Definitions<M>,
    minimum: F32,
    pc: Precision,
    resistance: impl Fn(&M) -> Option<&ResistDamage>,
) -> Result<()> {
    recompute_with_lazy_minimum(
        modifier,
        state,
        instances,
        definitions,
        || Ok(minimum),
        pc,
        resistance,
    )
}
///50252F reads its clamp after entry allocation, aggregation and both stores.
pub fn recompute_with_lazy_minimum<M>(
    modifier: &ResistDamage,
    state: &mut PropertiesState,
    instances: &Instances,
    definitions: &Definitions<M>,
    minimum: impl FnOnce() -> Result<F32>,
    pc: Precision,
    resistance: impl Fn(&M) -> Option<&ResistDamage>,
) -> Result<()> {
    let index = if let Some(i) = state
        .modifier
        .damage_factors
        .iter()
        .position(|v| v.damage_type == modifier.damage_type)
    {
        i
    } else {
        state
            .modifier
            .damage_factors
            .try_reserve(1)
            .map_err(|e| Error::Callback(format!("resistance entry allocation failed: {e}")))?;
        state.modifier.damage_factors.push(DamageFactor {
            damage_type: modifier.damage_type,
            factor: F32::ZERO,
            resistance: Some(F32::ZERO),
        });
        state.modifier.damage_factors.len() - 1
    };
    let mut sum = F32::ONE;
    for handle in &state.effects {
        let instance = instances.get(handle).ok_or_else(|| {
            Error::Callback("missing property instance in resistance scan".into())
        })?;
        if instance.disabled {
            continue;
        }
        let definition = definitions.get(&instance.definition).ok_or_else(|| {
            Error::Callback("missing property definition in resistance scan".into())
        })?;
        for item in &definition.modifiers {
            if let Some(candidate) = resistance(item)
                && candidate.damage_type == modifier.damage_type
            {
                sum = F32(candidate
                    .amount
                    .number()?
                    .add(&sum.number()?, pc)?
                    .store_f32()?);
            }
        }
    }
    //5020A0 stores the temporary sum only when unequal, then502440 reads that
    //actual field for both subtracts. Preserve this even for signed-zero equality.
    let entry = &mut state.modifier.damage_factors[index];
    if entry.factor.compare(sum)? != Ordering::Equal {
        entry.factor = sum;
    }
    entry.resistance = Some(F32(entry
        .factor
        .number()?
        .sub(&F32::ONE.number()?, pc)?
        .store_f32()?));
    entry.factor = F32(aa_numeric::Binary::from_i32(2)
        .sub(&entry.factor.number()?, pc)?
        .store_f32()?);
    let minimum = minimum()?;
    if minimum.compare(entry.factor)? == Ordering::Greater {
        entry.factor = minimum;
    }
    Ok(())
}
