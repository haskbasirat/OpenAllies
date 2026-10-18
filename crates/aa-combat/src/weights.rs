//! Native4BE6C0 local ability utility and57E670 other-mask maximum.
//! These are decision weights, not seconds or cooldown durations.
use crate::{Result, admission::Filter};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Weights {
    pub priority: F32,
    /// Move, use, wait: local offsets24/28/2C.
    pub factors: [F32; 3],
    /// Local offsets30/34/38.
    pub bonuses: [F32; 3],
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub enabled: bool,
    pub targets: Filter,
    pub weights: Weights,
}
fn n(f: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(f.0)?)
}
/// The use and move branches are stored to f32; wait remains extended.
pub fn maximum(weights: Weights, actor_bonuses: [F32; 3], p: Precision) -> Result<Binary> {
    let branch = |i: usize| -> Result<Binary> {
        Ok(n(weights.factors[i])?
            .mul(&n(weights.priority)?, p)?
            .add(&n(actor_bonuses[i])?, p)?
            .add(&n(weights.bonuses[i])?, p)?)
    };
    let use_weight = n(F32(branch(1)?.store_f32()?))?;
    let mut value = branch(2)?;
    let move_weight = n(F32(branch(0)?.store_f32()?))?;
    if value.compare(&move_weight) != Ordering::Greater {
        value = move_weight;
    }
    if use_weight.compare(&value) == Ordering::Greater {
        value = use_weight;
    }
    Ok(value)
}
/// Stable authored definition/list snapshots; no callback defaults or elapsed
/// time substitution. Same target masks are excluded only when supplied.
pub fn maximum_other(
    entries: &[Entry],
    excluded: Option<Filter>,
    actor_bonuses: [F32; 3],
    p: Precision,
) -> Result<Binary> {
    let mut value = F32::ZERO;
    for e in entries {
        if !e.enabled
            || excluded.is_some_and(|x| {
                x.required == e.targets.required && x.forbidden == e.targets.forbidden
            })
        {
            continue;
        }
        let candidate = maximum(e.weights, actor_bonuses, p)?;
        if n(value)?.compare(&candidate) != Ordering::Greater {
            value = F32(candidate.store_f32()?);
        }
    }
    n(value)
}
