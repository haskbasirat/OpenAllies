//! Original OrganizationComponent formation slot positions (Organization.cpp):
//! ComputeSlotPosition `0x5797e0` with facing rotation `0x578250`, all-slot
//! refresh `0x57bef0`, slot reassignment `0x57ba80`, leader change `0x579a00`
//! and slot world position `0x4cf290`.
//!
//! Slot positions (organization +0x78) are offsets from the organization
//! center (+0xBC/+0xC0). Element movement/following consumes them through
//! `slot_world_position`; this module does not move members.
//!
//! Removed engine cap: `0x57ba80` tracks pending slots in a 32-bit mask
//! (`1 << (slot & 31)`, assertion "e<32" at Organization.cpp:0xAEC). Here the
//! pending set is an exact growable per-slot set. For slot indices below 32 the
//! behavior is identical; the native aliasing for larger indices is not
//! reproduced. The swap budget keeps the native wrapping signed factorial.
use crate::{Error, Result};
use aa_numeric::{Binary, Fast, Precision};

// The slot arithmetic runs on `aa_numeric::Fast`, the exact x87 values of
// `Binary` without big-integer cost (formations are recomputed for every
// regiment order); `unit_draw` keeps its `Binary` result for other callers.
fn bin(value: f32) -> Result<Fast> {
    Ok(Fast::from_f32_bits(value.to_bits())?)
}
fn store(value: &Fast) -> Result<f32> {
    Ok(f32::from_bits(value.store_f32()?))
}
fn epsilon() -> Result<Fast> {
    Ok(Fast::from_f32_bits(crate::NATIVE_EPSILON_BITS)?)
}
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Native static 0xB39A90 (global 0xB39AF8): 2048 normalized (cos, sin)
/// binary32 pairs built by static initializer 0x9C8680 / 0x408ED0. The CRT
/// sets PC53 (`_controlfp(0x10000, 0x30000)` at 0x972C6B, from 0x96BDB7)
/// before C++ initializers run, so the PC53 output is the authoritative table.
const DIRECTION_TABLE: &[u8; 16384] =
    include_bytes!("../../../research/phase8-movement/direction-table.bin");
pub const DIRECTION_COUNT: u32 = 2048;

/// Entry of the native random-direction table.
pub fn direction(index: u32) -> Option<[f32; 2]> {
    if index >= DIRECTION_COUNT {
        return None;
    }
    let at = index as usize * 8;
    let word = |o: usize| {
        f32::from_bits(u32::from_le_bytes(
            DIRECTION_TABLE[o..o + 4].try_into().expect("table word"),
        ))
    };
    Some([word(at), word(at + 4)])
}

/// Original `0x444a90`: one draw, unsigned remainder by the table count.
pub fn random_direction(draw: u32) -> [f32; 2] {
    direction(draw % DIRECTION_COUNT).expect("index below count")
}

/// Original `0x45d400`: FILD of the signed draw, +2^32 (binary64) when
/// negative, times binary64 0x3DF0000000100000; extended, never stored.
pub fn unit_draw(draw: u32, precision: Precision) -> Result<Binary> {
    let mut value = Binary::from_i32(draw as i32);
    if draw & 0x8000_0000 != 0 {
        value = value.add(&Binary::from_f64_bits(0x41f0_0000_0000_0000)?, precision)?;
    }
    Ok(value.mul(&Binary::from_f64_bits(0x3df0_0000_0010_0000)?, precision)?)
}
/// [`unit_draw`] as a `Fast` value.
fn unit_draw_fast(draw: u32, precision: Precision) -> Result<Fast> {
    let mut value = Fast::from_i32(draw as i32);
    if draw & 0x8000_0000 != 0 {
        value = value.add(&Fast::from_f64_bits(0x41f0_0000_0000_0000)?, precision)?;
    }
    Ok(value.mul(&Fast::from_f64_bits(0x3df0_0000_0010_0000)?, precision)?)
}

/// Organization member (actor pointer in native slot array +0x28).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    /// Actor identity (native pointer equality); actor+0x14 for diagnostics.
    pub id: u32,
    /// Actor +0x20/+0x24.
    pub position: [f32; 2],
    /// Element (actor+0x80) +0x18.
    pub slot: i32,
    /// Element +0x1C byte: attached to the organization.
    pub attached: bool,
}

/// Current formation (organization +0x74).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormationShape {
    /// Formation definition (+0xC) byte +0x18.
    pub oriented: bool,
    /// Formation +0x10 12-byte records, X at +4 and Y at +8.
    pub offsets: Vec<[f32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    /// Owner actor +0x14 (diagnostics only).
    pub owner_id: u32,
    /// Owner actor facing +0x2C/+0x30.
    pub facing: [f32; 2],
    /// Owner actor has a Mind (+0x70) for the +0x114 leader event.
    pub owner_has_mind: bool,
    /// Organization +0xBC/+0xC0.
    pub center: [f32; 2],
    /// Layout +0x18 primary slot (signed).
    pub primary_slot: i32,
    /// Layout +0x1C group records (+8 slot count each), count +0x20.
    pub group_sizes: Vec<i32>,
    /// Layout +0x2C slot count.
    pub slot_count: i32,
    /// Organization +0x28 per-slot members.
    pub members: Vec<Option<Member>>,
    /// Organization +0x40.
    pub leader: Option<Member>,
    /// Organization +0x74.
    pub formation: Option<FormationShape>,
    /// Organization +0x78 per-slot offsets.
    pub slot_positions: Vec<[f32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotOutcome {
    /// Negative slot: no write.
    Ignored,
    /// Leader's slot or primary slot (or an empty slot with neither): zero.
    Origin,
    /// Oriented formation offset rotated by the owner facing.
    Rotated,
    /// Empty slot: random table direction scaled by 4*unit + 0.5.
    Scattered,
    /// Attached member: member minus leader position.
    Relative,
    /// Detached member: unchanged; native logs organization/member ids and
    /// the retained offset (Organization.cpp:0xAB0).
    Detached,
}

/// Formation boundaries.
pub trait FormationWorld {
    /// One `0x85d840` step of the GWorld+0xD4 stream (through `0x4f02b0`).
    fn next_random_u32(&mut self) -> Result<u32>;
    /// Owner Mind virtual +0x114 after a leader change.
    fn mind_leader_changed(&mut self) -> Result<()>;
    /// Diagnostic `0x404f90` "org %d detached elem %d - leaving slot position
    /// as %f,%f" (owner +0x14, member +0x14, retained offset). No state change.
    fn detached_member(&mut self, organization_id: u32, member_id: u32, offset: [f32; 2]) -> Result<()>;
}

fn slot_index(organization: &Organization, slot: i32, end: usize) -> Result<usize> {
    let index = usize::try_from(slot).map_err(|_| Error::World(format!("slot {slot} is negative")))?;
    if index >= end
        || index >= organization.members.len()
        || index >= organization.slot_positions.len()
    {
        return Err(Error::World(format!("slot {slot} is outside the organization arrays")));
    }
    Ok(index)
}

/// Original `0x578250`: `(fy*X + fx*Y, fy*Y - fx*X)` for formation offset
/// (X, Y) and owner facing (fx, fy), products and sums at `precision`.
pub fn rotate_offset(offset: [f32; 2], facing: [f32; 2], precision: Precision) -> Result<[f32; 2]> {
    let [x, y] = offset.map(bin);
    let [fx, fy] = facing.map(bin);
    let (x, y, fx, fy) = (x?, y?, fx?, fy?);
    Ok([
        store(&fx.mul(&y, precision)?.add(&fy.mul(&x, precision)?, precision)?)?,
        store(&fy.mul(&y, precision)?.sub(&fx.mul(&x, precision)?, precision)?)?,
    ])
}

/// Original `0x4cf290`: organization center plus slot offset (binary32 adds).
pub fn slot_world_position(organization: &Organization, slot: usize, precision: Precision) -> Result<[f32; 2]> {
    let offset = *organization
        .slot_positions
        .get(slot)
        .ok_or_else(|| Error::World(format!("slot {slot} has no position")))?;
    Ok([
        store(&bin(organization.center[0])?.add(&bin(offset[0])?, precision)?)?,
        store(&bin(organization.center[1])?.add(&bin(offset[1])?, precision)?)?,
    ])
}

/// Complete ComputeSlotPosition `0x5797e0`.
pub fn compute_slot_position<W: FormationWorld>(
    organization: &mut Organization,
    slot: i32,
    precision: Precision,
    world: &mut W,
) -> Result<SlotOutcome> {
    if slot < 0 {
        return Ok(SlotOutcome::Ignored);
    }
    let index = slot_index(organization, slot, usize::MAX)?;
    let leader_id = organization.leader.as_ref().map(|leader| leader.id);
    let member = organization.members[index].clone();
    if (member.is_some() && member.as_ref().map(|m| m.id) == leader_id)
        || slot == organization.primary_slot
    {
        organization.slot_positions[index] = [0.0, 0.0];
        return Ok(SlotOutcome::Origin);
    }
    if let Some(formation) = organization.formation.as_ref().filter(|f| f.oriented) {
        let offset = *formation
            .offsets
            .get(index)
            .ok_or_else(|| Error::World(format!("formation has no offset for slot {slot}")))?;
        organization.slot_positions[index] = rotate_offset(offset, organization.facing, precision)?;
        return Ok(SlotOutcome::Rotated);
    }
    let Some(member) = member else {
        if leader_id.is_none() && organization.primary_slot < 0 {
            organization.slot_positions[index] = [0.0, 0.0];
            return Ok(SlotOutcome::Origin);
        }
        let heading = random_direction(world.next_random_u32()?);
        organization.slot_positions[index] = heading;
        let scale = unit_draw_fast(world.next_random_u32()?, precision)?
            .mul(&bin(4.0)?, precision)?
            .add(&bin(0.5)?, precision)?;
        organization.slot_positions[index] = [
            store(&scale.mul(&bin(heading[0])?, precision)?)?,
            store(&scale.mul(&bin(heading[1])?, precision)?)?,
        ];
        return Ok(SlotOutcome::Scattered);
    };
    if member.attached {
        let leader = organization
            .leader
            .as_ref()
            .ok_or_else(|| Error::World("attached member offset requires a leader (native null read)".into()))?;
        organization.slot_positions[index] = [
            store(&bin(member.position[0])?.sub(&bin(leader.position[0])?, precision)?)?,
            store(&bin(member.position[1])?.sub(&bin(leader.position[1])?, precision)?)?,
        ];
        return Ok(SlotOutcome::Relative);
    }
    world.detached_member(organization.owner_id, member.id, organization.slot_positions[index])?;
    Ok(SlotOutcome::Detached)
}

/// Original `0x57bef0`: every layout slot, then reassignment when the current
/// formation is oriented.
pub fn compute_all_slot_positions<W: FormationWorld>(
    organization: &mut Organization,
    precision: Precision,
    world: &mut W,
) -> Result<Vec<SlotOutcome>> {
    let mut outcomes = Vec::new();
    let mut slot = 0;
    while slot < organization.slot_count {
        outcomes.push(compute_slot_position(organization, slot, precision, world)?);
        slot += 1;
    }
    if organization.formation.as_ref().is_some_and(|f| f.oriented) {
        reassign_slots(organization, precision)?;
    }
    Ok(outcomes)
}

/// Original `0x579a00`: replace the leader, recompute the old then the new
/// leader's slot, then notify the owner Mind (+0x114) when present.
pub fn set_leader<W: FormationWorld>(
    organization: &mut Organization,
    leader: Option<Member>,
    precision: Precision,
    world: &mut W,
) -> Result<()> {
    let old = std::mem::replace(&mut organization.leader, leader);
    if let Some(old) = old {
        compute_slot_position(organization, old.slot, precision, world)?;
    }
    if let Some(slot) = organization.leader.as_ref().map(|leader| leader.slot) {
        compute_slot_position(organization, slot, precision, world)?;
    }
    if organization.owner_has_mind {
        world.mind_leader_changed()?;
    }
    Ok(())
}

fn wrapping_factorial(count: i32) -> i32 {
    let mut value: i32 = 1;
    let mut factor = count;
    while factor > 1 {
        value = value.wrapping_mul(factor);
        factor -= 1;
    }
    value
}

/// Complete slot reassignment `0x57ba80`. Within each layout group of more than
/// one slot it builds the binary32 squared-distance matrix (row = slot, column
/// = member), then repeatedly takes the highest pending slot and swaps its
/// position with the last non-leader slot whose swap total plus binary32 1e-4
/// is below the running total. A swap clears the partner; no improvement
/// clears the slot. The running total starts as the extended diagonal sum and
/// becomes the stored binary32 candidate after each swap.
pub fn reassign_slots(organization: &mut Organization, precision: Precision) -> Result<()> {
    let Some(leader) = organization.leader.clone() else {
        return Ok(());
    };
    if organization.slot_count < 0 {
        return Err(Error::World("negative layout slot count".into()));
    }
    let zero = bin(0.0)?;
    let tolerance = epsilon()?;
    let mut start: i32 = 0;
    for &count in &organization.group_sizes.clone() {
        let end = start.wrapping_add(count);
        if count > 1 {
            let size = count as usize;
            let first = slot_index(organization, start, usize::MAX)?;
            slot_index(organization, end - 1, usize::MAX)?;
            let max_swaps = wrapping_factorial(count);
            let mut matrix = vec![0f32; size * size];
            let mut pending = vec![true; size];
            let mut total = zero.clone();
            let center_x = bin(organization.center[0])?;
            let center_y = bin(organization.center[1])?;
            for i in 0..size {
                let member = organization.members[first + i].clone();
                for j in 0..size {
                    let distance = match &member {
                        Some(member) => {
                            let slot = organization.slot_positions[first + j];
                            let world_x = store(&center_x.add(&bin(slot[0])?, precision)?)?;
                            let dy = center_y
                                .add(&bin(slot[1])?, precision)?
                                .sub(&bin(member.position[1])?, precision)?;
                            let dx = bin(world_x)?.sub(&bin(member.position[0])?, precision)?;
                            dx.mul(&dx, precision)?.add(&dy.mul(&dy, precision)?, precision)?
                        }
                        None => zero.clone(),
                    };
                    matrix[j * size + i] = store(&distance)?;
                    if i == j {
                        total = total.add(&distance, precision)?;
                    }
                }
            }
            if let Some(leader_relative) = leader
                .slot
                .checked_sub(start)
                .filter(|r| (0..count).contains(r))
            {
                pending[leader_relative as usize] = false;
            }
            let mut swaps: i32 = 0;
            while let Some(i) = pending.iter().rposition(|p| *p) {
                let mut best: Option<usize> = None;
                let mut best_value = f32::MAX;
                for c in 0..size {
                    if start + c as i32 == leader.slot {
                        continue;
                    }
                    let candidate = total
                        .sub(&bin(matrix[i * size + i])?, precision)?
                        .sub(&bin(matrix[c * size + c])?, precision)?
                        .add(&bin(matrix[i * size + c])?, precision)?
                        .add(&bin(matrix[c * size + i])?, precision)?;
                    if tolerance.add(&candidate, precision)?.compare(&total) == Ordering::Less {
                        best_value = store(&candidate)?;
                        best = Some(c);
                    }
                }
                match best {
                    Some(c) if c != i => {
                        total = bin(best_value)?;
                        organization.slot_positions.swap(first + c, first + i);
                        for k in 0..size {
                            matrix.swap(c * size + k, i * size + k);
                        }
                        pending[c] = false;
                        let previous = swaps;
                        swaps = swaps.wrapping_add(1);
                        if previous >= max_swaps {
                            break;
                        }
                    }
                    _ => pending[i] = false,
                }
            }
        }
        start = end;
    }
    Ok(())
}
