//! Native reactive engagement: OrganizationComponent `579490`
//! (OnElementDamaged), called by Body damage (`5625C0`) for hostile damage on
//! a member element. It is the engine's damage-driven target acquisition: a
//! company whose element was hit by an attacker inside the owner's operating
//! radius (`578470`) adds that attacker to its CAI engage target list
//! (`524C10`). Helpers `578E90` (Mind free), `4CE640` (element attached),
//! `432ED0` (`+100` bit4) and `5790F0` (`+100` bit15) are ported inline.
use crate::{Result, morale::Organization};
use aa_body::{F32, damage::DamageRecord};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub trait World {
    /// Generation-checked registry lookup of the record source GTID.
    fn resolve(&self, gtid: u32) -> Result<Option<u32>>;
    /// `actor+70` Mind present.
    fn has_mind(&self, actor: u32) -> Result<bool>;
    /// Owner actor virtual `+16C(attacker)`.
    fn can_engage(&mut self, owner: u32, attacker: u32) -> Result<bool>;
    /// Owner Mind virtuals `+64` and `+7C` (each true means busy).
    fn mind_busy64(&mut self, owner: u32) -> Result<bool>;
    fn mind_busy7c(&mut self, owner: u32) -> Result<bool>;
    /// Actor `+20/+24`.
    fn position(&self, actor: u32) -> Result<[F32; 2]>;
    /// Owner definition `+304` (squared operating radius).
    fn operating_radius_squared(&self, owner: u32) -> Result<F32>;
    /// `4CE640`: element `+80` membership present with byte `+1C` set.
    fn element_attached(&self, element: u32) -> Result<bool>;
    /// Actor `+100`.
    fn flags100(&self, actor: u32) -> Result<u32>;
    /// `524C10(attacker)` on the owner's Mind (CAI engage AddTarget).
    fn add_engage_target(&mut self, owner: u32, attacker: u32) -> Result<()>;
    /// The native diagnostic (Organization.cpp line 0x358) emitted when an
    /// attached element is hit from outside the operating radius.
    fn outside_operating_radius(&mut self, owner: u32, element: u32) -> Result<()>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `578470`: extended `dx*dx + dy*dy <= radius_squared`, with the
/// owner minus attacker differences kept unrounded.
pub fn within_operating_radius(
    owner: u32,
    attacker: u32,
    world: &impl World,
    p: Precision,
) -> Result<bool> {
    let radius = world.operating_radius_squared(owner)?;
    let o = world.position(owner)?;
    let a = world.position(attacker)?;
    let dy = n(o[1])?.sub(&n(a[1])?, p)?;
    let dx = n(o[0])?.sub(&n(a[0])?, p)?;
    let distance = dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?;
    Ok(distance.compare(&n(radius)?) != Ordering::Greater)
}

/// Native `578E90`: no Mind counts as free.
fn mind_free(owner: u32, world: &mut impl World) -> Result<bool> {
    if !world.has_mind(owner)? {
        return Ok(true);
    }
    if world.mind_busy64(owner)? {
        return Ok(false);
    }
    Ok(!world.mind_busy7c(owner)?)
}

/// Native `579490(element, record)`. Returns the native bool result.
pub fn element_damaged(
    organization: &mut Organization,
    element: u32,
    record: DamageRecord,
    world: &mut impl World,
    p: Precision,
) -> Result<bool> {
    organization.last_damage = record;
    let owner = organization.owner;
    if record.event_kind != 1 || !world.has_mind(owner)? {
        return Ok(false);
    }
    let Some(attacker) = world.resolve(record.source_id)? else {
        return Ok(false);
    };
    if !world.can_engage(owner, attacker)? {
        return Ok(false);
    }
    if !organization.byte8d && !mind_free(owner, world)? {
        return Ok(false);
    }
    if within_operating_radius(owner, attacker, world, p)? {
        if !world.element_attached(element)?
            && world.flags100(owner)? & 0x10 == 0
            && world.flags100(owner)? & 0x8000 == 0
        {
            return Ok(false);
        }
        world.add_engage_target(owner, attacker)?;
        return Ok(true);
    }
    if world.element_attached(element)? {
        world.outside_operating_radius(owner, element)?;
    }
    Ok(false)
}
