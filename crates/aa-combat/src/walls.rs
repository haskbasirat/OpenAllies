//! Wall component (actor `+98`) siege/ownership helpers and the Denizen
//! component (actor `+88`) ownership transfer, ported from the original:
//! `0x585160` completeness predicate, `0x585AE0` actor flag 13, `0x586370`
//! under-siege propagation, `0x585A20` wall ownership transfer, `0x56ACB0`
//! component-88 transfer and `0x569540` intruder predicate (with the list
//! cleanup `0x569300`). Proven by research/phase8-combat/native_walls.py.
use crate::Result;
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// Native `0x585160`: the wall's `+24` segment list (count `+28`): an empty
/// list is false, a negative count true; otherwise every segment must be
/// non-null with actor `+8` bit 10.
pub fn wall_complete(count: i32, segment_flags8: &[Option<u32>]) -> bool {
    if count == 0 {
        return false;
    }
    (0..count.max(0) as usize).all(|i| {
        segment_flags8
            .get(i)
            .copied()
            .flatten()
            .is_some_and(|flags| flags & 0x400 != 0)
    })
}

/// Inputs of `0x585AE0` (wall `+4` actor flag 13 in `+F0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flag13Inputs {
    /// Wall actor `+100` bit 20 (state 20 under siege).
    pub actor_under_siege: bool,
    /// Wall `+14` owner present with its `+100` bit 30.
    pub owner_bit30: bool,
    /// Wall `+1C` segment count.
    pub segments: i32,
    /// `[[+4]+4]+464[+10 * 3]` (definition table entry, signed).
    pub required_segments: i32,
}
/// Native `0x585AE0`: the value passed to `0x517FA0(13, value)`.
pub fn flag13(inputs: &Flag13Inputs) -> bool {
    !inputs.actor_under_siege && !inputs.owner_bit30 && inputs.segments < inputs.required_segments
}

pub trait World {
    /// Wall `+4` actor.
    fn wall_actor(&self, wall: u32) -> Result<u32>;
    /// Wall `+14` owner actor (`None` = null).
    fn wall_owner(&self, wall: u32) -> Result<Option<u32>>;
    /// Wall `+18[i]` for `i < +1C` (`None` = null).
    fn wall_segments(&self, wall: u32) -> Result<Vec<Option<u32>>>;
    /// Actor `+100` low dword.
    fn flags100(&self, actor: u32) -> Result<u32>;
    /// `0x4D64F0(state, enabled)`.
    fn set_state(&mut self, actor: u32, state: u32, enabled: bool) -> Result<()>;
    /// `0x42D490([B39D8C], 7, virtual +108(actor, virtual +24 name))`.
    fn announce_under_siege(&mut self, actor: u32) -> Result<()>;
    /// `0x4D1350`.
    fn notify(&mut self, actor: u32) -> Result<()>;
    /// Remaining `0x585AE0` facts (definition table entry).
    fn required_segments(&self, wall: u32) -> Result<i32>;
    /// `0x517FA0(13, value)` on the wall actor.
    fn set_flag13(&mut self, actor: u32, value: bool) -> Result<()>;
    /// `0x5859C0`: per segment, actor `+100` bit 21 selects `0x5656B0` on its
    /// `+9C` construction component, else `0x564DA0` then `0x565A20`.
    fn refresh_segments(&mut self, wall: u32) -> Result<()>;
    /// Wall `+14` owner's `+88` component (`None` = null).
    fn owner_component_88(&self, wall: u32) -> Result<Option<u32>>;
}

/// Native `0x585AE0` through the world.
pub fn refresh_flag13(wall: u32, world: &mut impl World) -> Result<()> {
    let actor = world.wall_actor(wall)?;
    let actor_under_siege = world.flags100(actor)? & 0x10_0000 != 0;
    let owner_bit30 = match world.wall_owner(wall)? {
        Some(owner) => world.flags100(owner)? & 0x4000_0000 != 0,
        None => false,
    };
    let value = if actor_under_siege || owner_bit30 {
        false
    } else {
        let segments = world.wall_segments(wall)?.len() as i32;
        let required_segments = world.required_segments(wall)?;
        flag13(&Flag13Inputs {
            actor_under_siege,
            owner_bit30,
            segments,
            required_segments,
        })
    };
    world.set_flag13(actor, value)
}

/// Native `0x586370`: copy the wall actor's under-siege state (state 20)
/// to its segments, announcing (message 7) when under siege.
pub fn propagate_under_siege(wall: u32, world: &mut impl World) -> Result<()> {
    let owner = world.wall_owner(wall)?;
    let owner_actor = owner.ok_or(crate::Error::Invalid("wall without +14 owner"))?;
    let under_siege = world.flags100(owner_actor)? & 0x10_0000 != 0;
    let actor = world.wall_actor(wall)?;
    world.set_state(actor, 0x14, under_siege)?;
    if under_siege {
        world.announce_under_siege(actor)?;
    }
    for segment in world.wall_segments(wall)?.into_iter().flatten() {
        if Some(segment) == owner {
            continue;
        }
        world.set_state(segment, 0x14, under_siege)?;
        world.notify(segment)?;
    }
    refresh_flag13(wall, world)
}

pub trait TransferWorld: World {
    /// Denizen `+40` link list: each link's `[+0]+C` GTID list, resolved.
    /// `None` is an unresolved GTID.
    fn denizen_links(&self, component: u32) -> Result<Vec<Vec<Option<u32>>>>;
    /// Linked actor `+70` component present.
    fn has_component_70(&self, actor: u32) -> Result<bool>;
    /// `+70` virtual `+40(0)` then `+18C(new owner)`; true when it accepted.
    fn component_70_transfer(&mut self, actor: u32, owner: u32) -> Result<bool>;
    /// `0x4D7960`: bind [`crate::begin_death::begin_death`].
    fn begin_death(&mut self, actor: u32) -> Result<()>;
    /// Denizen component `+4` actor.
    fn denizen_actor(&self, component: u32) -> Result<u32>;
    /// After a link's actors: decrement the link object's `+4` refcount
    /// (virtual `0(1)` at zero) and clear the node's link pointer.
    fn release_link(&mut self, component: u32, index: usize) -> Result<()>;
    /// Free the `+40` nodes (`0x473300`) and clear `+0`/`+8` of every `+18`
    /// chain node (through `+10`).
    fn clear_denizen_links(&mut self, component: u32) -> Result<()>;
}

/// Native `0x56ACB0(component88)`: every linked actor either accepts the
/// denizen's owner through its `+70` component or begins dying; then the
/// links are released.
pub fn transfer_component_88(component: u32, world: &mut impl TransferWorld) -> Result<()> {
    let owner = world.denizen_actor(component)?;
    for (index, link) in world.denizen_links(component)?.into_iter().enumerate() {
        for actor in link.into_iter().flatten() {
            let accepted =
                world.has_component_70(actor)? && world.component_70_transfer(actor, owner)?;
            if !accepted {
                world.begin_death(actor)?;
            }
        }
        world.release_link(component, index)?;
    }
    world.clear_denizen_links(component)
}

/// Native `0x585A20(wall)`: `0x5859C0`, then the owner's `+88` transfer.
pub fn transfer_wall(wall: u32, world: &mut impl TransferWorld) -> Result<()> {
    world.refresh_segments(wall)?;
    if let Some(component) = world.owner_component_88(wall)? {
        transfer_component_88(component, world)?;
    }
    Ok(())
}

pub trait IntruderWorld {
    /// Component-88 `+24` GTID list in order: the resolved actor and its
    /// `+8` bit 0, `None` for an unresolved GTID.
    fn intruders(&self, component: u32) -> Result<Vec<Option<(u32, bool)>>>;
    /// `0x569300` pop of the list head (node freed).
    fn drop_front_intruder(&mut self, component: u32) -> Result<()>;
    /// `0x585280` on the wall actor's `+98` component: bind
    /// [`intruder_within_wall`] (radius from [`intruder_radius`]).
    fn within_wall(&mut self, wall_actor: u32, actor: u32) -> Result<bool>;
}

/// Native `0x569540(component88; wall actor)`: `0x569300` drops leading
/// unresolved or dying entries, then any resolved entry within the wall.
pub fn intruder_near_wall(
    component: u32,
    wall_actor: u32,
    world: &mut impl IntruderWorld,
) -> Result<bool> {
    while let Some(front) = world.intruders(component)?.first().copied() {
        if matches!(front, Some((_, false))) {
            break;
        }
        world.drop_front_intruder(component)?;
    }
    for (actor, _) in world.intruders(component)?.into_iter().flatten() {
        if world.within_wall(wall_actor, actor)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn number(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}
fn binary32(value: Binary) -> Result<F32> {
    Ok(F32(value.store_f32()?))
}

/// Native `0x5851A0(x, y)` on the wall component: the point relative to the
/// wall actor (`+4`, position `+20/+24`; binary32 differences) must lie
/// strictly on the inner side of every edge of the wall definition's
/// `+47C` polygon (vertex `i` at record `i * 0x14 + 4/+8`, the
/// component's `+28` vertices, edge `i -> (i + 1) % n`): the extended cross
/// product `(px - xi)(yj - yi) - (py - yi)(xj - xi)` below 0.01 (A03078;
/// unordered also continues). An empty polygon contains every point.
pub fn point_within_wall(
    point: [F32; 2],
    wall_position: [F32; 2],
    polygon: &[[F32; 2]],
    p: Precision,
) -> Result<bool> {
    let px = binary32(number(point[0])?.sub(&number(wall_position[0])?, p)?)?;
    let py = binary32(number(point[1])?.sub(&number(wall_position[1])?, p)?)?;
    let threshold = number(F32(0x3c23_d70a))?;
    for (i, &[xi, yi]) in polygon.iter().enumerate() {
        let [xj, yj] = polygon[if i + 1 < polygon.len() { i + 1 } else { 0 }];
        let a = number(px)?
            .sub(&number(xi)?, p)?
            .mul(&number(yj)?.sub(&number(yi)?, p)?, p)?;
        let b = number(py)?
            .sub(&number(yi)?, p)?
            .mul(&number(xj)?.sub(&number(xi)?, p)?, p)?;
        if a.sub(&b, p)?.compare(&threshold) != Ordering::Less {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Intruder radius read by `0x585280`: with an Organization (`+7C`), the
/// map `+8` value plus the Organization `+AC` (binary32 store); otherwise
/// the actor virtual `+10` data's `+4`.
pub fn intruder_radius(
    map_tile: F32,
    organization_radius: Option<F32>,
    data_radius: F32,
    p: Precision,
) -> Result<F32> {
    match organization_radius {
        Some(radius) => binary32(number(map_tile)?.add(&number(radius)?, p)?),
        None => Ok(data_radius),
    }
}

/// Native `0x585280(actor)` on the wall component: the actor position moved
/// toward the wall actor by `radius` (only when the extended distance is
/// above 1e-4; the y difference is also stored binary32 and reused), then
/// [`point_within_wall`]. Binds `IntruderWorld::within_wall`.
pub fn intruder_within_wall(
    wall_position: [F32; 2],
    actor_position: [F32; 2],
    radius: F32,
    polygon: &[[F32; 2]],
    p: Precision,
) -> Result<bool> {
    let dx = number(wall_position[0])?.sub(&number(actor_position[0])?, p)?;
    let dy = number(wall_position[1])?.sub(&number(actor_position[1])?, p)?;
    let dy32 = binary32(dy.clone())?;
    let length = dy
        .mul(&number(dy32)?, p)?
        .add(&dx.mul(&dx, p)?, p)?
        .sqrt(p)?;
    let length32 = binary32(length.clone())?;
    let point = if length.compare(&number(F32(0x38d1_b717))?) == Ordering::Greater {
        let r = number(radius)?;
        let rx = dx.mul(&r, p)?;
        let ry = number(dy32)?.mul(&r, p)?;
        let inverse = Binary::from_u32(1).div(&number(length32)?, p)?;
        let offset_x = binary32(inverse.mul(&rx, p)?)?;
        let offset_y = inverse.mul(&ry, p)?;
        [
            binary32(number(actor_position[0])?.add(&number(offset_x)?, p)?)?,
            binary32(number(actor_position[1])?.add(&offset_y, p)?)?,
        ]
    } else {
        actor_position
    };
    point_within_wall(point, wall_position, polygon, p)
}
