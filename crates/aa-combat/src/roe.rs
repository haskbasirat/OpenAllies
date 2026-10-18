//! Native rules-of-engagement (ROE) primary damage history, kept by an
//! element's membership component (`actor+80`, vector at `+28`).
//!
//! - `570F70`: called by Body damage (`5625C0`) for an actor with membership;
//!   only the organization's primary element (`organization+44`) records.
//! - `570A70`: prune records with time strictly older than
//!   `now - ROEPrimaryDamageHistoryLength` (binary32 cutoff), in place, stable.
//! - `570E30`: prune, then the in-order extended sum of record amounts divided
//!   by the history length. Its caller (`5340C0`, unit AI) is not ported here.
//! - `570EA0`/`5707E0`: append. Native capacity doubling is an allocation
//!   detail, replaced by a growable Vec (no fixed history cap is introduced).
use crate::{Error, Result};
use aa_body::{F32, damage::DamageRecord};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `570A70`. `now` is world `+E0`; `length` is settings-cache `+194`.
pub fn prune(history: &mut Vec<DamageRecord>, now: F32, length: F32, p: Precision) -> Result<()> {
    let cutoff = n(F32(n(now)?.sub(&n(length)?, p)?.store_f32()?))?;
    let mut index = 0;
    while index < history.len() {
        if n(history[index].time)?.compare(&cutoff) == Ordering::Less {
            history.remove(index);
        } else {
            index += 1;
        }
    }
    Ok(())
}

/// Native `570E30`: extended result, no binary32 store.
pub fn rate(
    history: &mut Vec<DamageRecord>,
    now: F32,
    length: F32,
    p: Precision,
) -> Result<Binary> {
    prune(history, now, length, p)?;
    let mut sum = Binary::from_u32(0);
    for record in history.iter() {
        sum = sum.add(&n(record.amount)?, p)?;
    }
    Ok(sum.div(&n(length)?, p)?)
}

/// Native `570F70` inputs: membership `+10` organization actor and, when
/// present, its OrganizationComponent `+44` primary element.
pub trait World {
    /// Membership component `+10`.
    fn membership_organization(&self, member: u32) -> Result<Option<u32>>;
    /// `organization_actor+7C` then `+44`. A missing component is an error:
    /// the original dereferences it unconditionally.
    fn primary_element(&self, organization_actor: u32) -> Result<Option<u32>>;
}

/// Native `570F70(record)` on `member`'s membership component.
pub fn record(
    member: u32,
    history: &mut Vec<DamageRecord>,
    damage: DamageRecord,
    now: F32,
    length: F32,
    world: &impl World,
    p: Precision,
) -> Result<bool> {
    let Some(organization) = world.membership_organization(member)? else {
        return Ok(false);
    };
    if world.primary_element(organization)? != Some(member) {
        return Ok(false);
    }
    prune(history, now, length, p)?;
    history
        .try_reserve(1)
        .map_err(|e| Error::World(format!("ROE history allocation failed: {e}")))?;
    history.push(damage);
    Ok(true)
}
