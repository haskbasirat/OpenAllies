//! Native DenizenComponent (actor `+88`, `Denizen.cpp`) intruder handling:
//! `56AE70` OnDamaged (the Body-damage `Component88Damaged` action), `569A80`
//! intruder-in-range and `568D80` OnIntruder. This is the target acquisition
//! of neutral denizens: hostile attackers inside the definition `+368` range
//! are prepended to the `+24` intruder list (GTIDs; tail `+28`).
use crate::{Error, Result, reactions::Membership};
use aa_body::{F32, damage::DamageRecord};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub trait World {
    /// Generation-checked registry lookup.
    fn resolve(&self, gtid: u32) -> Result<Option<u32>>;
    /// Denizen owner definition `+364` and `+368`.
    fn ranges(&self, owner: u32) -> Result<[F32; 2]>;
    fn position(&self, actor: u32) -> Result<[F32; 2]>;
    /// Actor `+7C` OrganizationComponent: member count (`+2C`, reread per
    /// iterator step) and member slots (`+28`).
    fn organization_member_count(&self, actor: u32) -> Result<Option<i32>>;
    fn organization_member(&self, actor: u32, index: i32) -> Result<Option<u32>>;
    fn membership(&self, actor: u32) -> Result<Option<Membership>>;
    /// `4D0380` field part: actor has `+7C` or `+60`.
    fn has_organization_or_body(&self, actor: u32) -> Result<bool>;
    fn flags8(&self, actor: u32) -> Result<u32>;
    fn actor_id(&self, actor: u32) -> Result<u32>;
    /// Owner virtual `+108`.
    fn allegiance(&mut self, owner: u32) -> Result<Option<u32>>;
    /// Intruder virtuals `+84(allegiance)` (inside `4D0380`) and `+140`.
    fn relation84(&mut self, actor: u32, allegiance: Option<u32>) -> Result<bool>;
    fn hostile(&mut self, actor: u32, allegiance: Option<u32>) -> Result<bool>;
    /// Denizen.cpp line 0xAAA debug log ("denizen %d adding intruder %d").
    fn adding_intruder(&mut self, owner: u32, intruder_id: u32) -> Result<()>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `569A80(source)` for the denizen owned by `owner`.
pub fn intruder_in_range(
    owner: u32,
    source: Option<u32>,
    world: &impl World,
    p: Precision,
) -> Result<bool> {
    let Some(source) = source else {
        return Ok(false);
    };
    if n(world.ranges(owner)?[0])?.is_zero() || n(world.ranges(owner)?[1])?.is_zero() {
        return Ok(false);
    }
    if let Some(mut count) = world.organization_member_count(source)? {
        // 4364C0 with the attached-only flag: null slots and detached members
        // are skipped; the count is reread at every step.
        let mut index = 0;
        while index < count {
            if let Some(member) = world.organization_member(source, index)? {
                let attached = world
                    .membership(member)?
                    .ok_or(Error::Invalid(
                        "organization member without membership (native null dereference)",
                    ))?
                    .attached;
                if attached && intruder_in_range(owner, Some(member), world, p)? {
                    return Ok(true);
                }
            }
            index += 1;
            count = world
                .organization_member_count(source)?
                .ok_or(Error::Invalid("organization component vanished"))?;
        }
        return Ok(false);
    }
    let s = world.position(source)?;
    let o = world.position(owner)?;
    let dy = n(s[1])?.sub(&n(o[1])?, p)?;
    let dx = n(s[0])?.sub(&n(o[0])?, p)?;
    let distance = n(F32(dx
        .mul(&dx, p)?
        .add(&dy.mul(&dy, p)?, p)?
        .store_f32()?))?;
    let range = n(world.ranges(owner)?[1])?;
    Ok(distance.compare(&range.mul(&range, p)?) != Ordering::Greater)
}

/// Native `568D80(intruder)`. `intruders` is the `+24` list (front = head):
/// membership test `517F30`, GTID handle `4CF1B0`, head insertion `4D1970`.
pub fn on_intruder(
    owner: u32,
    intruders: &mut Vec<u32>,
    intruder: u32,
    world: &mut impl World,
) -> Result<()> {
    let mut intruder = intruder;
    if let Some(Membership {
        organization: Some(organization),
        attached: true,
    }) = world.membership(intruder)?
    {
        intruder = organization;
    }
    let allegiance = world.allegiance(owner)?;
    if !world.has_organization_or_body(intruder)?
        || world.flags8(intruder)? & 0x800 != 0
        || !world.relation84(intruder, allegiance)?
    {
        return Ok(());
    }
    let allegiance = world.allegiance(owner)?;
    if !world.hostile(intruder, allegiance)? || world.flags8(intruder)? & 0x4000 == 0 {
        return Ok(());
    }
    let id = world.actor_id(intruder)?;
    if intruders.contains(&id) {
        return Ok(());
    }
    world.adding_intruder(owner, id)?;
    intruders
        .try_reserve(1)
        .map_err(|e| Error::World(format!("intruder list allocation failed: {e}")))?;
    intruders.insert(0, id);
    Ok(())
}

/// Native `56AE70(record)` on the denizen owned by `owner`.
pub fn on_damaged(
    owner: u32,
    intruders: &mut Vec<u32>,
    record: &DamageRecord,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let Some(source) = world.resolve(record.source_id)? else {
        return Ok(());
    };
    if intruder_in_range(owner, Some(source), world, p)? {
        on_intruder(owner, intruders, source, world)?;
    }
    Ok(())
}
