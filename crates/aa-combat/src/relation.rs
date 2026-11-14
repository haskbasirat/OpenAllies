//! Native actor relation tests used by every targeting Think:
//!
//! * `4D4FF0(target, a, b)`: may this actor use an ability on the target.
//!   0 is allowed, 6 refused; an organization answers the smallest code of
//!   its members (stopping at 0), and a Performer facing an organization
//!   answers the smallest of its own `57E920` code and its codes for every
//!   member of that organization.
//! * `4D4420(target)`: is this actor (or, for an organization, one of its
//!   members) attacking the target or the organization the target belongs to.
//!
//! Both walk organization members with the `4364C0` iterator with the
//! attached-only byte clear (every non-null slot, count reread per step).
//! Oracle: research/phase8-combat/native_relation.py.
use crate::{Error, Result};

pub trait World {
    /// Target virtual `+B4`.
    fn targetable(&mut self, target: u32) -> Result<bool>;
    /// Actor virtual `+108`: the word handed to the target's `+84`.
    fn kingdom(&mut self, actor: u32) -> Result<u32>;
    /// Target virtual `+84(kingdom)`.
    fn accepts(&mut self, target: u32, kingdom: u32) -> Result<bool>;
    /// Actor `+7C` OrganizationComponent.
    fn organization(&mut self, actor: u32) -> Result<Option<u32>>;
    /// OrganizationComponent `+2C` (signed member count).
    fn member_count(&mut self, organization: u32) -> Result<i32>;
    /// OrganizationComponent `+28` slot `index`.
    fn member(&mut self, organization: u32, index: i32) -> Result<Option<u32>>;
    /// Actor `+74` Performer.
    fn performer(&mut self, actor: u32) -> Result<Option<u32>>;
    /// `57E920(target, a, b)` on a Performer.
    fn performer_relation(&mut self, performer: u32, target: u32, a: u32, b: u32) -> Result<i32>;
    /// Actor `+80` membership and its `+10` organization actor; None when
    /// either is null.
    fn membership_organization(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Actor `+70` Mind.
    fn mind(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Mind virtual `+4C` (its current target).
    fn mind_target(&mut self, mind: u32) -> Result<Option<u32>>;
}

/// `4364C0` with the attached-only byte clear, from position `index`
/// (-1 before the first): the next non-null slot, or None at the end. The
/// count is reread at every step.
fn next_member(world: &mut impl World, organization: u32, index: i32) -> Result<Option<(i32, u32)>> {
    let mut index = index;
    loop {
        index = index
            .checked_add(1)
            .ok_or(Error::Invalid("4364C0 member index overflow"))?;
        if index >= world.member_count(organization)? {
            return Ok(None);
        }
        if let Some(member) = world.member(organization, index)? {
            return Ok(Some((index, member)));
        }
    }
}

/// Native `4D4FF0(target, a, b)` on `actor`.
pub fn relation(world: &mut impl World, actor: u32, target: Option<u32>, a: u32, b: u32) -> Result<i32> {
    let Some(target) = target else {
        return Ok(6);
    };
    if !world.targetable(target)? {
        return Ok(6);
    }
    let kingdom = world.kingdom(actor)?;
    if !world.accepts(target, kingdom)? {
        return Ok(6);
    }
    if let Some(organization) = world.organization(actor)? {
        let mut best = 6;
        let mut cursor = next_member(world, organization, -1)?;
        while let Some((index, member)) = cursor {
            let code = relation(world, member, Some(target), a, b)?;
            if code < best {
                best = code;
            }
            if best == 0 {
                return Ok(0);
            }
            cursor = next_member(world, organization, index)?;
        }
        return Ok(best);
    }
    let Some(performer) = world.performer(actor)? else {
        return Ok(6);
    };
    if world.organization(target)?.is_none() {
        return world.performer_relation(performer, target, a, b);
    }
    let mut best = world.performer_relation(performer, target, a, b)?;
    // The target's +7C is read again after 57E920 for the member walk.
    let organization = world
        .organization(target)?
        .ok_or(Error::Invalid("4D4FF0 target organization vanished (native null member walk)"))?;
    let mut cursor = next_member(world, organization, -1)?;
    while let Some((index, member)) = cursor {
        let code = relation(world, actor, Some(member), a, b)?;
        if code < best {
            best = code;
        }
        if best == 0 {
            return Ok(0);
        }
        cursor = next_member(world, organization, index)?;
    }
    Ok(best)
}

/// The thing `4D4420` compares: an actor without an OrganizationComponent
/// that has a membership with an organization stands for that organization.
fn represented(world: &mut impl World, thing: u32) -> Result<u32> {
    if world.organization(thing)?.is_none()
        && let Some(organization) = world.membership_organization(thing)?
    {
        return Ok(organization);
    }
    Ok(thing)
}

/// Native `4D4420(target)` on `actor`.
pub fn attacking(world: &mut impl World, actor: u32, target: u32) -> Result<bool> {
    let target = represented(world, target)?;
    if let Some(mind) = world.mind(actor)?
        && let Some(current) = world.mind_target(mind)?
        && represented(world, current)? == target
    {
        return Ok(true);
    }
    if let Some(organization) = world.organization(actor)? {
        let mut cursor = next_member(world, organization, -1)?;
        while let Some((index, member)) = cursor {
            if attacking(world, member, target)? {
                return Ok(true);
            }
            cursor = next_member(world, organization, index)?;
        }
    }
    Ok(false)
}
