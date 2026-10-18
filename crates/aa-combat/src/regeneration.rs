//! Organization regeneration `0x57B5A0` and regroup `0x57AEB0`.
//!
//! Slot layout is native: the organization's definition (+18) gives the
//! leader slot (+18) and the slot-type indices (+28) into the organization's
//! slot definitions (+1C); members are +28[slot] (count +2C), the primary
//! element is +44 and respawn progress is +50[slot]. Member spawning
//! (`0x57AD30`, an actor creation through `0x4FBF40`), plain healing
//! (`0x562BA0`, aa-body `Body::heal_plain`), health assignment (`0x561E50`,
//! aa-body `Body::set_health_from_source`) and the regeneration presentation
//! (`0x50C380`) are world services.
use crate::{Error, Result, morale::Organization};
use aa_body::{F32, properties::ModifierState};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}
fn st(value: &Binary) -> Result<F32> {
    Ok(F32(value.store_f32()?))
}
/// `[0xA3CED4]` (1e-4f): regroup minimum transferred health.
pub const REGROUP_MINIMUM: F32 = F32(0x38d1_b717);

/// Body health (+10) and maximum (+14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyHealth {
    pub health: F32,
    pub maximum: F32,
}
impl BodyHealth {
    /// `0x5617C0`: `!(health < maximum)`.
    pub fn full(&self) -> Result<bool> {
        Ok(n(self.health)?.compare(&n(self.maximum)?) != Ordering::Less)
    }
}

/// Slot definition fields (`+1C[type]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotDefinition {
    /// +174 (bit 0 regenerable).
    pub flags_174: u32,
    /// +2CC regeneration (`0x45D5B0`, modifier `0x5014E0` multiply +3C).
    pub regeneration_2cc: F32,
    /// +284 maximum health (`0x45D730`, no modifier context).
    pub health_284: F32,
}

pub trait World {
    /// Organization definition +18.
    fn leader_slot(&self, o: &Organization) -> Result<i32>;
    /// +2C.
    fn slot_count(&self, o: &Organization) -> Result<i32>;
    /// +28[slot].
    fn member(&self, o: &Organization, slot: i32) -> Result<Option<u32>>;
    /// Member +60 body.
    fn body(&self, member: u32) -> Result<Option<BodyHealth>>;
    /// Member +80 membership `+1C` byte.
    fn member_attached(&self, member: u32) -> Result<bool>;
    /// Member +80 membership `+20` regeneration rate.
    fn member_regeneration(&self, member: u32) -> Result<F32>;
    /// `+1C[[+18]+28[slot]]`; `None` for a null definition.
    fn slot_definition(&self, o: &Organization, slot: i32) -> Result<Option<SlotDefinition>>;
    /// Owner virtual +108 allegiance +1F0.
    fn allegiance_shortage(&mut self, o: &mut Organization) -> Result<F32>;
    /// `0x4CF110(owner)`.
    fn modifier(&mut self, o: &mut Organization) -> Result<Option<ModifierState>>;
    /// `0x562BA0(member body, {owner, amount})`.
    fn heal(&mut self, o: &mut Organization, member: u32, amount: F32) -> Result<()>;
    /// `0x50C380(world+130; owner, {owner, amount})`.
    fn present_regeneration(&mut self, o: &mut Organization, amount: F32) -> Result<()>;
    /// +50[slot].
    fn progress(&self, o: &Organization, slot: i32) -> Result<F32>;
    fn set_progress(&mut self, o: &mut Organization, slot: i32, value: F32) -> Result<()>;
    /// `0x57AD30(slot, 0)`: spawn the slot's member; true on success. Bind
    /// [`spawn_member`] with `flag = false`.
    fn spawn(&mut self, o: &mut Organization, slot: i32) -> Result<bool>;
    /// SVar "RegroupStealHealthRatio" (`0x4B2E60`, cached in `[0xB3D6B0]`).
    fn regroup_steal_ratio(&mut self) -> Result<F32>;
    /// `0x561E50(member body; health, 0, 0, 0)`.
    fn set_health(&mut self, o: &mut Organization, member: u32, health: F32) -> Result<()>;
}

fn damaged(world: &impl World, member: Option<u32>) -> Result<Option<u32>> {
    let Some(member) = member else {
        return Ok(None);
    };
    match world.body(member)? {
        Some(body) if !body.full()? => Ok(Some(member)),
        _ => Ok(None),
    }
}

/// Slot choice of `0x57B5A0`: a damaged primary at the leader slot, else the
/// first damaged attached member, else an absent primary's leader slot, else
/// the first empty slot with a regenerable definition.
fn choose(o: &Organization, world: &impl World) -> Result<Option<i32>> {
    let leader = world.leader_slot(o)?;
    if leader >= 0 && damaged(world, o.primary)?.is_some() {
        return Ok(Some(leader));
    }
    let count = world.slot_count(o)?;
    for slot in 0..count.max(0) {
        if let Some(member) = damaged(world, world.member(o, slot)?)?
            && world.member_attached(member)?
        {
            return Ok(Some(slot));
        }
    }
    if leader >= 0 && o.primary.is_none() {
        return Ok(Some(leader));
    }
    for slot in 0..count.max(0) {
        if world.member(o, slot)?.is_none()
            && let Some(def) = world.slot_definition(o, slot)?
            && def.flags_174 & 1 != 0
        {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

/// Native `0x57B5A0(delta)`.
pub fn regenerate(
    o: &mut Organization,
    delta: F32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let Some(slot) = choose(o, world)? else {
        return Ok(());
    };
    let mut factor = F32::ONE;
    let shortage = n(world.allegiance_shortage(o)?)?;
    if shortage.compare(&Binary::from_i32(0)) == Ordering::Greater {
        factor = st(&Binary::from_i32(1).sub(&shortage, p)?)?;
    }
    if let Some(member) = world.member(o, slot)? {
        let body = world.body(member)?.ok_or(Error::Invalid(
            "regenerating member without a body (native null)",
        ))?;
        let amount = st(&n(world.member_regeneration(member)?)?
            .mul(&n(body.maximum)?, p)?
            .mul(&n(factor)?, p)?
            .mul(&n(delta)?, p)?)?;
        return world.heal(o, member, amount);
    }
    let def = world.slot_definition(o, slot)?.ok_or(Error::Invalid(
        "empty slot without a definition (native null)",
    ))?;
    let rate = match world.modifier(o)? {
        None => def.regeneration_2cc,
        Some(m) => st(&m
            .multiply(0x3c, def.regeneration_2cc, p)
            .map_err(|e| Error::World(e.to_string()))?)?,
    };
    let amount = st(&n(def.health_284)?
        .mul(&n(rate)?, p)?
        .mul(&n(factor)?, p)?
        .mul(&n(delta)?, p)?)?;
    world.present_regeneration(o, amount)?;
    let progress = st(&n(amount)?.add(&n(world.progress(o, slot)?)?, p)?)?;
    world.set_progress(o, slot, progress)?;
    if n(def.health_284)?.compare(&n(progress)?) != Ordering::Greater {
        world.spawn(o, slot)?;
    }
    Ok(())
}

/// Native `0x57AEB0`: respawn the leader slot, then move
/// `RegroupStealHealthRatio` of every other member's health to it (at least
/// [`REGROUP_MINIMUM`]). Returns the spawn result.
pub fn regroup(o: &mut Organization, world: &mut impl World, p: Precision) -> Result<bool> {
    let leader = world.leader_slot(o)?;
    if !world.spawn(o, leader)? {
        return Ok(false);
    }
    let ratio = n(world.regroup_steal_ratio()?)?;
    let mut stolen = F32::ZERO;
    for slot in 0..world.slot_count(o)?.max(0) {
        if slot == leader {
            continue;
        }
        let Some(member) = world.member(o, slot)? else {
            continue;
        };
        let Some(body) = world.body(member)? else {
            continue;
        };
        let health = n(body.health)?;
        let take = st(&ratio.mul(&health, p)?)?;
        let rest = st(&health.sub(&n(take)?, p)?)?;
        world.set_health(o, member, rest)?;
        stolen = st(&n(take)?.add(&n(stolen)?, p)?)?;
    }
    if n(stolen)?.compare(&n(REGROUP_MINIMUM)?) == Ordering::Less {
        stolen = REGROUP_MINIMUM;
    }
    let respawned = world.member(o, leader)?.ok_or(Error::Invalid(
        "regroup spawned no leader member (native null)",
    ))?;
    world.set_health(o, respawned, stolen)?;
    Ok(true)
}

/// The stack creation descriptor `0x57AD30` passes to `0x4FBF40` (vtable
/// `0xA25B28`, 0x54 bytes). Words not listed are zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreationDescriptor {
    /// +04: the slot's definition (`+1C[[+18]+28[slot]]`).
    pub definition: u32,
    /// +18: `-1.0f`.
    pub word_18: u32,
    /// +1C: empty CString (name).
    pub name: String,
    /// +20 and +34: `-1`.
    pub word_20: u32,
    pub word_34: u32,
    /// +28: owner virtual `+108` (allegiance).
    pub allegiance: u32,
    /// +51: the caller's flag byte.
    pub flag: bool,
}
impl CreationDescriptor {
    pub fn for_member(definition: u32, allegiance: u32, flag: bool) -> Self {
        Self {
            definition,
            word_18: 0xbf80_0000,
            name: String::new(),
            word_20: u32::MAX,
            word_34: u32::MAX,
            allegiance,
            flag,
        }
    }
}

pub trait SpawnWorld {
    /// +28[slot] occupied.
    fn slot_occupied(&self, o: &Organization, slot: i32) -> Result<bool>;
    /// `+1C[[+18]+28[slot]]` (the native pointer is passed through).
    fn slot_definition_handle(&self, o: &Organization, slot: i32) -> Result<u32>;
    /// Owner virtual `+108`.
    fn allegiance(&mut self, o: &mut Organization) -> Result<u32>;
    /// `world.0x4FBF40(descriptor, owner, slot)`; true when created.
    fn create_actor(
        &mut self,
        descriptor: &CreationDescriptor,
        owner: u32,
        slot: i32,
    ) -> Result<bool>;
}

/// Native `0x57AD30(slot, flag)`: create the member of an empty slot through
/// the actor-creation seam `0x4FBF40`; false when the slot is occupied or the
/// creation fails.
pub fn spawn_member(
    o: &mut Organization,
    slot: i32,
    flag: bool,
    world: &mut impl SpawnWorld,
) -> Result<bool> {
    if world.slot_occupied(o, slot)? {
        return Ok(false);
    }
    let definition = world.slot_definition_handle(o, slot)?;
    let allegiance = world.allegiance(o)?;
    let descriptor = CreationDescriptor::for_member(definition, allegiance, flag);
    world.create_actor(&descriptor, o.owner, slot)
}
