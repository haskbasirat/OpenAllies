//! Native57C080 organization position interpolation with required553BA0 probes.
use crate::{
    ActorId,
    element_position::{Error, Result, World},
};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;
pub trait InterpolationWorld: World {
    type Query;
    /// Native SVar at cache+48 multiplied by Organization+D8.
    fn speed_multiplier(&self) -> Result<F32>;
    /// Native registered numeric SVar4BE0F0; may remain extended.
    fn minimum_speed(&mut self) -> Result<Binary>;
    fn timestep(&self) -> Result<F32>;
    /// Actual4CEDE0, implemented by aa-spatial::moving_state.
    fn member_moving(&mut self, member: Option<ActorId>) -> Result<bool>;
    /// Selected member's current Mind virtual+80 point, after moving-state check.
    fn mind_point(&mut self, mind: u32) -> Result<[F32; 2]>;
    ///552DD0→5531F0(member,true), then OR0x400 into query words38/3C.
    fn prepare_probe(&mut self, member: Option<ActorId>, extra_flags: u32) -> Result<Self::Query>;
    ///552250 sets query position, then553BA0 returns resultword0!=0.
    fn blocked(&mut self, query: &mut Self::Query, point: [F32; 2]) -> Result<bool>;
}
fn num<T>(value: aa_numeric::Result<T>) -> Result<T> {
    value.map_err(|e| Error::World(format!("Organization interpolation arithmetic: {e}")))
}
fn n(v: F32) -> Result<Binary> {
    num(Binary::from_f32_bits(v.0))
}
fn store(v: Binary) -> Result<F32> {
    Ok(F32(num(v.store_f32())?))
}
fn target(org: ActorId, w: &impl World) -> Result<[F32; 2]> {
    Ok(w.organization(org)?.position_xy)
}
fn relocate(org: ActorId, point: [F32; 2], w: &mut impl World) -> Result<()> {
    let owner = w.organization(org)?.header.owner;
    w.relocate_owner(owner, point)
}
pub fn interpolate(org: ActorId, w: &mut impl InterpolationWorld, p: Precision) -> Result<bool> {
    let owner = w.organization(org)?.header.owner;
    let mut point = w.actor(owner)?.position;
    let goal = target(org, w)?;
    let dy = store(num(n(goal[1])?.sub(&n(point[1])?, p))?)?;
    let dx_ext = num(n(goal[0])?.sub(&n(point[0])?, p))?;
    let dx = store(dx_ext.clone())?;
    // Native FST, not FSTP: the first multiplicand retains the extended delta.
    let square = num(dx_ext.mul(&n(dx)?, p))?;
    let square = num(square.add(&num(n(dy)?.mul(&n(dy)?, p))?, p))?;
    let square_stored = store(square.clone())?;
    if square.compare(&Binary::from_i32(0)) != Ordering::Greater {
        return Ok(true);
    }
    let local_speed = F32(*w
        .organization(org)?
        .words
        .get(&0xd8)
        .ok_or_else(|| Error::World("uninitialized Organization+D8 speed".into()))?);
    let mut speed = store(num(n(w.speed_multiplier()?)?.mul(&n(local_speed)?, p))?)?;
    let minimum = w.minimum_speed()?;
    if n(speed)?.compare(&minimum) == Ordering::Less {
        speed = store(minimum)?;
    }
    let step = store(num(n(speed)?.mul(&n(w.timestep()?)?, p))?)?;
    let mut distance = store(num(n(square_stored)?.sqrt(p))?)?;
    let ratio = num(n(step)?.div(&n(distance)?, p))?;
    let delta = [
        store(num(n(dx)?.mul(&ratio, p))?)?,
        store(num(n(dy)?.mul(&ratio, p))?)?,
    ];
    let near = num(n(step)?.add(&n(F32(0x38d1b717))?, p))?;
    if near.compare(&n(distance)?) != Ordering::Less {
        relocate(org, target(org, w)?, w)?;
        return Ok(true);
    }
    if w.member_moving(w.selected_member(org)?)? {
        let selected = w
            .selected_member(org)?
            .ok_or_else(|| Error::World("moving selected member became null".into()))?;
        let mind = w.actor(selected)?.mind.ok_or(Error::MissingMind)?;
        let destination = w.mind_point(mind)?;
        let a = num(n(point[0])?.sub(&n(destination[0])?, p))?;
        let b = num(n(point[1])?.sub(&n(destination[1])?, p))?;
        let a = if a.is_negative_nonzero() {
            a.negated()
        } else {
            a
        };
        let b = if b.is_negative_nonzero() {
            b.negated()
        } else {
            b
        };
        let gap = num(a.add(&b, p))?;
        if gap.compare(&n(F32(0x3951b717))?) == Ordering::Less {
            return Ok(false);
        }
    }
    // The native loop presumes a positive time/speed step. Invalid paused or
    // negative step inputs are rejected explicitly instead of inventing motion
    // or introducing an arbitrary number-of-probes cap.
    if n(step)?.compare(&Binary::from_i32(0)) != Ordering::Greater {
        return Err(Error::World(
            "nonpositive Organization interpolation step".into(),
        ));
    }
    let mut query = w.prepare_probe(w.selected_member(org)?, 0x400)?;
    loop {
        point = [
            store(num(n(point[0])?.add(&n(delta[0])?, p))?)?,
            store(num(n(point[1])?.add(&n(delta[1])?, p))?)?,
        ];
        let remaining = num(n(distance)?.sub(&n(step)?, p))?;
        distance = store(remaining.clone())?;
        if remaining.compare(&Binary::from_i32(0)) != Ordering::Greater {
            point = target(org, w)?;
        }
        if !w.blocked(&mut query, point)? {
            relocate(org, point, w)?;
            return Ok(n(distance)?.compare(&Binary::from_i32(0)) != Ordering::Greater);
        }
        if n(distance)?.compare(&Binary::from_i32(0)) == Ordering::Greater {
            continue;
        }
        let goal = target(org, w)?;
        if w.blocked(&mut query, goal)? {
            return Ok(false);
        }
        relocate(org, target(org, w)?, w)?;
        return Ok(true);
    }
}
