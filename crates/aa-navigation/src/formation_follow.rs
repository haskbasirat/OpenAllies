//! UAIStateMove formation following (Tai/UAI.cpp): the mode-3 speed controller
//! `0x52ff00` that ActionGo (`0x5300a0`) calls as its formation gate, slot lag
//! `0x52e400`, path-length difference `0x52e300`, path length `0x52ce70`,
//! vector inequality `0x46b000`, squared distance `0x45d880`, Mover catch-up
//! `0x577090` and Mover requested-speed clamp `0x576d20`.
//!
//! Organization and leader paths are the collections returned by Mind virtual
//! +0x44. The concrete implementations are pure accessors (Element UAI
//! `0x535e00`: `lea eax,[ecx+0xd4]`; Regiment CAI `0x5247a0`:
//! `lea eax,[ecx+0x9c]`), so callers pass those paths as an immutable view.
//! Every other effect (leader moving predicate, stop, timer, state finish) is
//! a required callback, called in native order.
use crate::{ActorState, Error, MoveState, OwnedPath, PathNode, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn store(value: &Binary) -> Result<u32> {
    Ok(value.store_f32()?)
}
/// Native binary32 0.5 (0x9fbdb8), 0.2 (0xa0145c), -0.25 (0xa34d3c),
/// 36.0 (0xa34aec), 6.0 (0xa32694), 1/6 (0xa34ae8) and 1.0 (0x9f32e0).
const HALF: u32 = 0x3f00_0000;
const MINIMUM_FORMATION_SPEED: u32 = 0x3e4c_cccd;
const PATH_DIFFERENCE_SCALE: u32 = 0xbe80_0000;
const CORNER_RADIUS_SQUARED: u32 = 0x4210_0000;
const CORNER_RADIUS: u32 = 0x40c0_0000;
const CORNER_SCALE: u32 = 0x3e2a_aaab;
const ONE: u32 = 0x3f80_0000;

/// Leader actor (organization component +0x40) and its Mind +0x44 path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowLeader {
    pub position_bits: [u32; 2],
    pub path: OwnedPath,
}

/// Organization actor (Element +0x10): position, facing (component +4 owner),
/// Mind +0x44 path, component +0x78 slot offsets and +0x40 leader.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowOrganization {
    pub position_bits: [u32; 2],
    pub facing_bits: [u32; 2],
    pub path: OwnedPath,
    pub slot_positions_bits: Vec<[u32; 2]>,
    pub leader: Option<FollowLeader>,
}

/// actor+0x80 Element: +0x18 slot and +0x10 organization actor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowElement {
    pub slot: i32,
    pub organization: Option<FollowOrganization>,
}

pub trait FollowWorld {
    /// `0x4cede0` on the organization leader.
    fn leader_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool>;
    /// `0x52d660` stop orchestration.
    fn stop_action(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()>;
    /// UAI timer virtual (UAI+8)+4 with native second argument zero.
    fn schedule(&mut self, delay_bits: u32, state: &mut MoveState, actor: &mut ActorState)
    -> Result<()>;
    /// `0x52a3c0` on the UAI: release the state stack and finish.
    fn finish_state(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()>;
}

fn first(path: &OwnedPath) -> Result<&PathNode> {
    path.nodes.first().ok_or(Error::MissingPathNode)
}

/// Original `0x52ce70`: extended sum, over every node, of
/// `sqrt(dx*dx + dy*dy)` from the previous point (start = `from`).
pub fn path_length(nodes: &[PathNode], from: [u32; 2], p: Precision) -> Result<Binary> {
    let mut total = b(0)?;
    let mut previous_y = b(from[1])?;
    let mut previous_x = from[0];
    for node in nodes {
        let dy = b(node.words[1])?.sub(&previous_y, p)?;
        let dx = b(node.words[0])?.sub(&b(previous_x)?, p)?;
        previous_x = node.words[0];
        let length = dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?.sqrt(p)?;
        total = total.add(&length, p)?;
        previous_y = b(node.words[1])?;
    }
    Ok(total)
}

/// Original `0x52e300`: organization remaining path length minus the member's,
/// both stopping at the first kind-2 node. The member sum is stored to
/// binary32 after every node; the organization sum stays extended.
pub fn path_length_difference(
    state: &MoveState,
    actor: &ActorState,
    organization: &FollowOrganization,
    p: Precision,
) -> Result<Binary> {
    let leg = |dx: &Binary, dy: &Binary| -> Result<Binary> {
        Ok(dy.mul(dy, p)?.add(&dx.mul(dx, p)?, p)?.sqrt(p)?)
    };
    let mut member_total = 0u32;
    let mut previous_y = b(actor.position_bits[1])?;
    let mut previous_x = actor.position_bits[0];
    for node in state.path.nodes.iter().take_while(|n| n.words[4] != 2) {
        let dx = b(store(&b(node.words[0])?.sub(&b(previous_x)?, p)?)?)?;
        let dy = b(node.words[1])?.sub(&previous_y, p)?;
        member_total = store(&leg(&dx, &dy)?.add(&b(member_total)?, p)?)?;
        previous_y = b(node.words[1])?;
        previous_x = node.words[0];
    }
    let mut organization_total = b(0)?;
    let [mut ox, mut oy] = organization.position_bits;
    for node in organization.path.nodes.iter().take_while(|n| n.words[4] != 2) {
        let dx = b(store(&b(node.words[0])?.sub(&b(ox)?, p)?)?)?;
        let dy = b(node.words[1])?.sub(&b(oy)?, p)?;
        organization_total = organization_total.add(&leg(&dx, &dy)?, p)?;
        ox = node.words[0];
        oy = node.words[1];
    }
    Ok(organization_total.sub(&b(member_total)?, p)?)
}

/// Original `0x46b000`: true unless both components compare equal (x87
/// ordered equality; signed zeros equal, NaN unequal).
pub fn vectors_differ(a: [u32; 2], c: [u32; 2]) -> bool {
    f32::from_bits(a[0]) != f32::from_bits(c[0]) || f32::from_bits(a[1]) != f32::from_bits(c[1])
}

/// Original `0x45d880`: `(c.y-a.y)^2 + (c.x-a.x)^2`, extended.
pub fn squared_distance(a: [u32; 2], c: [u32; 2], p: Precision) -> Result<Binary> {
    let dy = b(c[1])?.sub(&b(a[1])?, p)?;
    let dx = b(c[0])?.sub(&b(a[0])?, p)?;
    Ok(dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?)
}

/// Original `0x52e400`: how far the member trails its slot along the path.
pub fn slot_lag(
    state: &MoveState,
    actor: &ActorState,
    element: &FollowElement,
    p: Precision,
) -> Result<Binary> {
    let organization = element
        .organization
        .as_ref()
        .ok_or_else(|| Error::World("slot lag requires the Element organization".into()))?;
    let count_difference = state.path.kind1_count.wrapping_sub(organization.path.kind1_count);
    if count_difference != 0 {
        let leader = organization
            .leader
            .as_ref()
            .ok_or_else(|| Error::World("slot lag requires the organization leader (native null read)".into()))?;
        let leader_length = store(&path_length(&leader.path.nodes, leader.position_bits, p)?)?;
        let member_length = path_length(&state.path.nodes, actor.position_bits, p)?;
        let difference = store(&member_length.sub(&b(leader_length)?, p)?)?;
        let leader_count = Binary::from_i32(state.path.kind1_count.wrapping_sub(leader.path.kind1_count));
        if b(difference)?.mul(&leader_count, p)?.compare(&b(0)?) == Ordering::Less {
            return Ok(leader_count.mul(&b(HALF)?, p)?);
        }
        return b(difference);
    }
    let slot = usize::try_from(element.slot)
        .ok()
        .and_then(|s| organization.slot_positions_bits.get(s))
        .copied()
        .ok_or_else(|| Error::World(format!("slot {} has no organization position", element.slot)))?;
    let [fx, fy] = organization.facing_bits.map(b);
    let (fx, fy) = (fx?, fy?);
    let lateral = store(&fy.mul(&b(slot[0])?, p)?.sub(&fx.mul(&b(slot[1])?, p)?, p)?)?;
    let forward = store(&fy.mul(&b(slot[1])?, p)?.add(&fx.mul(&b(slot[0])?, p)?, p)?)?;
    let head = *first(&state.path)?;
    let direction = [head.words[2], head.words[3]];
    let ahead_x = b(organization.position_bits[0])?.sub(&b(actor.position_bits[0])?, p)?;
    let ahead_y = b(organization.position_bits[1])?.sub(&b(actor.position_bits[1])?, p)?;
    let value = ahead_y
        .mul(&b(direction[1])?, p)?
        .add(&ahead_x.mul(&b(direction[0])?, p)?, p)?
        .add(&b(forward)?, p)?;
    let stored = store(&value)?;
    if state.path.nodes.len() <= 1 {
        return Ok(value);
    }
    let second = state.path.nodes[1];
    if second.words[4] == 0 {
        return Ok(value);
    }
    if !vectors_differ(direction, [second.words[2], second.words[3]]) {
        return b(stored);
    }
    let distance = store(&squared_distance(actor.position_bits, [head.words[0], head.words[1]], p)?)?;
    if b(CORNER_RADIUS_SQUARED)?.compare(&b(distance)?) != Ordering::Greater {
        return b(stored);
    }
    let cross = b(head.words[3])?
        .mul(&b(second.words[2])?, p)?
        .sub(&b(second.words[3])?.mul(&b(head.words[2])?, p)?, p)?;
    let mut side = b(lateral)?;
    if cross.compare(&b(0)?) != Ordering::Less {
        side = side.negated();
    }
    let taper = b(CORNER_RADIUS)?.sub(&b(distance)?.sqrt(p)?, p)?;
    Ok(side
        .mul(&taper, p)?
        .mul(&b(CORNER_SCALE)?, p)?
        .add(&b(stored)?, p)?)
}

/// Original `0x576d20`: write the requested speed, clamp it to the Mover
/// maximum (+0x28) when that is ordered below, then write the factor.
pub fn set_mover_speed(actor: &mut ActorState, speed_bits: u32, factor_bits: u32) {
    actor.mover_requested_speed_bits = speed_bits;
    if f32::from_bits(actor.mover_maximum_speed_bits) < f32::from_bits(speed_bits) {
        actor.mover_requested_speed_bits = actor.mover_maximum_speed_bits;
    }
    actor.mover_speed_factor_bits = factor_bits;
}

/// Original `0x577090`: catch-up SVar (`MoverCatchupSpeed`, cache+0x48) times
/// the Mover maximum speed, extended.
pub fn catchup_speed(actor: &ActorState, catchup_factor_bits: u32, p: Precision) -> Result<Binary> {
    Ok(b(catchup_factor_bits)?.mul(&b(actor.mover_maximum_speed_bits)?, p)?)
}

/// Original `0x52ff00` (ActionGo's mode-3 formation gate). Returns the native
/// AL: true continues ActionGo; false after the too-slow stop/finish branch.
pub fn formation_speed<W: FollowWorld>(
    state: &mut MoveState,
    actor: &mut ActorState,
    element: Option<&FollowElement>,
    catchup_factor_bits: u32,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    if state.mode != 3 {
        return Ok(true);
    }
    let Some((element, organization)) =
        element.and_then(|e| e.organization.as_ref().map(|o| (e, o)))
    else {
        let maximum = actor.mover_maximum_speed_bits;
        state.mode = 0;
        state.requested_speed_bits = maximum;
        return finish_speed(state, actor, maximum, p);
    };
    let head = *first(&state.path)?;
    let speed = if head.words[4] != 0 {
        let mut speed = store(
            &slot_lag(state, actor, element, p)?
                .mul(&b(HALF)?, p)?
                .add(&b(state.requested_speed_bits)?, p)?,
        )?;
        let catchup = catchup_speed(actor, catchup_factor_bits, p)?;
        let too_slow = if b(speed)?.compare(&b(MINIMUM_FORMATION_SPEED)?) == Ordering::Less {
            true
        } else {
            if b(speed)?.compare(&catchup) == Ordering::Greater {
                speed = store(&catchup)?;
            }
            b(speed)?.compare(&b(MINIMUM_FORMATION_SPEED)?) != Ordering::Greater
        };
        if too_slow {
            if organization.leader.is_none() {
                return Err(Error::World("0x4cede0 requires the organization leader (native null read)".into()));
            }
            if world.leader_is_moving(state, actor)? {
                world.stop_action(state, actor)?;
                world.schedule(ONE, state, actor)?;
            } else {
                world.finish_state(state, actor)?;
            }
            return Ok(false);
        }
        speed
    } else {
        let blend = path_length_difference(state, actor, organization, p)?
            .mul(&b(PATH_DIFFERENCE_SCALE)?, p)?;
        let mut weight = store(&blend)?;
        if blend.compare(&b(0)?) == Ordering::Less {
            weight = 0;
        } else if b(weight)?.compare(&b(ONE)?) == Ordering::Greater {
            weight = ONE;
        }
        let catchup = catchup_speed(actor, catchup_factor_bits, p)?;
        store(
            &catchup.mul(&b(weight)?, p)?.add(
                &b(ONE)?
                    .sub(&b(weight)?, p)?
                    .mul(&b(state.requested_speed_bits)?, p)?,
                p,
            )?,
        )?
    };
    finish_speed(state, actor, speed, p)
}

fn finish_speed(state: &MoveState, actor: &mut ActorState, speed: u32, p: Precision) -> Result<bool> {
    let factor = store(&b(speed)?.div(&b(state.requested_speed_bits)?, p)?)?;
    set_mover_speed(actor, speed, factor);
    Ok(true)
}
