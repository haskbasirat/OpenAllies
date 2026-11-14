//! Native actor health fraction `4D0230(members)` (ST0), read by AI view and
//! goal scoring.
//!
//! * A Body (`+60`): extended `+10 / +14`.
//! * Otherwise, with `members` set and a `+98` component or an
//!   OrganizationComponent `+7C`: the mean of the parts' own fractions
//!   (each part asked with `members` clear), summed into a binary32 total:
//!   the `+98` leader (`+14`, its fraction stored), then the `+18` list
//!   (`+1C` entries, last first, no null check), then the non-null `+24`
//!   entries (`+28`, last first), then every organization slot (last first)
//!   whose layout slot is enabled (`[+1C][[+18]+28[i]]`), counted even when
//!   empty. Each addition is `binary32(part + total)`; the mean divides the
//!   binary32 total by the integer count.
//! * Otherwise binary32 zero.
//!
//! Oracle: research/phase8-combat/native_health_fraction.py.
use aa_body::F32;
use aa_numeric::{Binary, Precision};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("health fraction input unavailable: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub trait World {
    /// Actor `+60` Body `+10` health and `+14` maximum.
    fn body(&mut self, actor: u32) -> Result<Option<[F32; 2]>>;
    /// Actor `+98` component.
    fn component98(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Component `+14` (`4CEDA0` with `+98` present).
    fn component98_leader(&mut self, component: u32) -> Result<Option<u32>>;
    /// Component `+1C` (read once).
    fn component98_list_count(&mut self, component: u32) -> Result<i32>;
    /// Component `+18[index]` (the native does not test it for null).
    fn component98_list(&mut self, component: u32, index: i32) -> Result<Option<u32>>;
    /// Component `+28` (read once).
    fn component98_array_count(&mut self, component: u32) -> Result<i32>;
    /// Component `+24[index]`.
    fn component98_array(&mut self, component: u32, index: i32) -> Result<Option<u32>>;
    /// Actor `+7C` OrganizationComponent.
    fn organization(&mut self, actor: u32) -> Result<Option<u32>>;
    /// OrganizationComponent `+2C` (read once).
    fn member_count(&mut self, organization: u32) -> Result<i32>;
    /// `[+1C][[+18]+28[index]]` is nonzero.
    fn slot_enabled(&mut self, organization: u32, index: i32) -> Result<bool>;
    /// OrganizationComponent `+28[index]`.
    fn member(&mut self, organization: u32, index: i32) -> Result<Option<u32>>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// One part: `binary32(4D0230(part, false) + total)`.
fn part(world: &mut impl World, thing: u32, total: F32, precision: Precision) -> Result<F32> {
    let value = health_fraction(world, thing, false, precision)?
        .ok_or_else(|| Error::World("member health fraction is indefinite".into()))?;
    Ok(F32(value.add(&n(total)?, precision)?.store_f32()?))
}

/// Native `4D0230(members)` on `actor`: the ST0 result, or None for the x87
/// indefinite of `0 / 0` (a mean over no parts).
pub fn health_fraction(world: &mut impl World, actor: u32, members: bool, precision: Precision) -> Result<Option<Binary>> {
    if let Some([health, maximum]) = world.body(actor)? {
        return Ok(Some(n(health)?.div(&n(maximum)?, precision)?));
    }
    let component = world.component98(actor)?;
    let organization = world.organization(actor)?;
    if !members || (component.is_none() && organization.is_none()) {
        return Ok(Some(Binary::from_i32(0)));
    }
    let mut total = F32::ZERO;
    let mut count: i32 = 0;
    if let Some(component) = component {
        if let Some(leader) = world.component98_leader(component)? {
            let value = health_fraction(world, leader, false, precision)?
                .ok_or_else(|| Error::World("leader health fraction is indefinite".into()))?;
            total = F32(value.store_f32()?);
            count = 1;
        }
        let mut index = world.component98_list_count(component)?.wrapping_sub(1);
        if index >= 0 {
            count = count.wrapping_add(index).wrapping_add(1);
        }
        while index >= 0 {
            let entry = world
                .component98_list(component, index)?
                .ok_or_else(|| Error::World("4D0230 null +18 entry (native null dereference)".into()))?;
            total = part(world, entry, total, precision)?;
            index -= 1;
        }
        let mut index = world.component98_array_count(component)?.wrapping_sub(1);
        while index >= 0 {
            if let Some(entry) = world.component98_array(component, index)? {
                total = part(world, entry, total, precision)?;
                count = count.wrapping_add(1);
            }
            index -= 1;
        }
    }
    if let Some(organization) = organization {
        let mut index = world.member_count(organization)?.wrapping_sub(1);
        while index >= 0 {
            if world.slot_enabled(organization, index)? {
                if let Some(member) = world.member(organization, index)? {
                    total = part(world, member, total, precision)?;
                }
                count = count.wrapping_add(1);
            }
            index -= 1;
        }
    }
    if count == 0 {
        return Ok(None);
    }
    Ok(Some(n(total)?.div(&Binary::from_i32(count), precision)?))
}
