//! Borrowed Organization leader/slot operations `579A00` / `5797E0`.
//!
//! Every field and service is required. Handles retain pointer identity, while
//! later reads observe current authoritative storage. Earlier writes survive
//! a missing dependency; this operation is not transactional or safe to retry.
use crate::formation::{SlotOutcome, random_direction, rotate_offset, unit_draw};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};

pub trait World {
    type Actor: Copy + Eq;
    type Positions: Copy;
    type Formation: Copy;
    type Mind: Copy;

    fn leader(&mut self) -> Result<Option<Self::Actor>>;
    /// Actual `+40` write, before reading either Element slot.
    fn store_leader(&mut self, leader: Option<Self::Actor>) -> Result<()>;
    /// Actor `+80` Element then Element `+18`.
    fn element_slot(&mut self, actor: Self::Actor) -> Result<i32>;
    /// Current Organization `+28` array and its indexed actor pointer.
    fn member(&mut self, slot: i32) -> Result<Option<Self::Actor>>;
    fn primary_slot(&mut self) -> Result<i32>;
    fn formation(&mut self) -> Result<Option<Self::Formation>>;
    fn formation_oriented(&mut self, formation: Self::Formation) -> Result<bool>;
    fn owner_facing(&mut self) -> Result<[f32; 2]>;
    /// `578250` rereads current `+74`; do not reuse the earlier check handle.
    fn formation_offset(&mut self, slot: i32) -> Result<[f32; 2]>;
    /// Capture current `+78` array identity, without copying its contents.
    fn positions(&mut self) -> Result<Self::Positions>;
    fn offset(&mut self, positions: Self::Positions, slot: i32, axis: usize) -> Result<f32>;
    fn store_offset(
        &mut self,
        positions: Self::Positions,
        slot: i32,
        axis: usize,
        value: f32,
    ) -> Result<()>;
    fn element_attached(&mut self, actor: Self::Actor) -> Result<bool>;
    fn actor_position(&mut self, actor: Self::Actor, axis: usize) -> Result<f32>;
    fn owner_id(&mut self) -> Result<u32>;
    fn actor_id(&mut self, actor: Self::Actor) -> Result<u32>;
    fn owner_mind(&mut self) -> Result<Option<Self::Mind>>;
    fn mind_leader_changed(&mut self, mind: Self::Mind) -> Result<()>;
    /// One `85D840` step through `4F02B0`, in the authoritative RNG stream.
    fn next_random_u32(&mut self) -> Result<u32>;
    fn detached_member(
        &mut self,
        organization_id: u32,
        member_id: u32,
        offset: [f32; 2],
    ) -> Result<()>;
}

fn b(value: f32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.to_bits())?)
}
fn f(value: &Binary) -> Result<f32> {
    Ok(f32::from_bits(value.store_f32()?))
}

fn zero<W: World>(world: &mut W, slot: i32) -> Result<SlotOutcome> {
    let positions = world.positions()?;
    world.store_offset(positions, slot, 0, 0.0)?;
    world.store_offset(positions, slot, 1, 0.0)?;
    Ok(SlotOutcome::Origin)
}

/// Entire native `5797E0`, with finite supported numeric inputs.
///
/// Negative slots read nothing. Missing arrays/components report the adapter's
/// explicit error at their first native use. Scattering captures its initial
/// output pointer before RNG, then recaptures `+78` before scaling the current
/// values; a service mutation cannot silently restore a stale snapshot.
pub fn compute_slot_position<W: World>(
    world: &mut W,
    slot: i32,
    p: Precision,
) -> Result<SlotOutcome> {
    if slot < 0 {
        return Ok(SlotOutcome::Ignored);
    }
    let member = world.member(slot)?;
    let leader = world.leader()?;
    if member.is_some() && member == leader {
        return zero(world, slot);
    }
    let primary = world.primary_slot()?;
    if slot == primary {
        return zero(world, slot);
    }
    if let Some(formation) = world.formation()?
        && world.formation_oriented(formation)?
    {
        let facing = world.owner_facing()?;
        let offset = world.formation_offset(slot)?;
        let rotated = rotate_offset(offset, facing, p)?;
        let positions = world.positions()?;
        world.store_offset(positions, slot, 0, rotated[0])?;
        world.store_offset(positions, slot, 1, rotated[1])?;
        return Ok(SlotOutcome::Rotated);
    }
    let Some(member) = member else {
        if leader.is_none() && primary < 0 {
            return zero(world, slot);
        }
        let target = world.positions()?;
        let heading = random_direction(world.next_random_u32()?);
        world.store_offset(target, slot, 0, heading[0])?;
        world.store_offset(target, slot, 1, heading[1])?;
        let scale = unit_draw(world.next_random_u32()?, p)?.mul(&b(4.0)?, p)?;
        let positions = world.positions()?;
        let scale = scale.add(&b(0.5)?, p)?;
        let x = f(&scale.mul(&b(world.offset(positions, slot, 0)?)?, p)?)?;
        world.store_offset(positions, slot, 0, x)?;
        let y = f(&scale.mul(&b(world.offset(positions, slot, 1)?)?, p)?)?;
        world.store_offset(positions, slot, 1, y)?;
        return Ok(SlotOutcome::Scattered);
    };
    if world.element_attached(member)? {
        let x = b(world.actor_position(member, 0)?)?;
        let leader = leader.ok_or_else(|| {
            Error::World("attached member offset requires a leader (native null read)".into())
        })?;
        let x = x.sub(&b(world.actor_position(leader, 0)?)?, p)?;
        let y = b(world.actor_position(member, 1)?)?;
        let positions = world.positions()?;
        let y = y.sub(&b(world.actor_position(leader, 1)?)?, p)?;
        world.store_offset(positions, slot, 0, f(&x)?)?;
        world.store_offset(positions, slot, 1, f(&y)?)?;
        return Ok(SlotOutcome::Relative);
    }
    let organization_id = world.owner_id()?;
    let positions = world.positions()?;
    let y = world.offset(positions, slot, 1)?;
    let member_id = world.actor_id(member)?;
    let x = world.offset(positions, slot, 0)?;
    world.detached_member(organization_id, member_id, [x, y])?;
    Ok(SlotOutcome::Detached)
}

/// Native `579A00`. Same-identity assignments still recompute twice and notify.
/// The new leader is stored before old Element lookup. The new leader and owner
/// Mind are reread after the old slot, preserving mutations and error prefixes.
pub fn set_leader<W: World>(world: &mut W, leader: Option<W::Actor>, p: Precision) -> Result<()> {
    let old = world.leader()?;
    world.store_leader(leader)?;
    if let Some(old) = old {
        let slot = world.element_slot(old)?;
        compute_slot_position(world, slot, p)?;
    }
    if let Some(leader) = world.leader()? {
        let slot = world.element_slot(leader)?;
        compute_slot_position(world, slot, p)?;
    }
    if let Some(mind) = world.owner_mind()? {
        world.mind_leader_changed(mind)?;
    }
    Ok(())
}
