//! Element Move537B90 and shared52FC50/52E0F0 lifecycle. The state owns native
//! decisions; required services perform real component, path and virtual work.
use crate::{Arena, Error, MindId, Result, StateId};
use aa_navigation::result_plan;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Payload {
    pub plan: result_plan::State,
    /// Last native collision result at+3C, including its generation-bearing ID.
    pub collision: [u32; 4],
    pub collision_vector: [u32; 2],
    pub detached_time_bits: u32,
    pub word78: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub flags: u32,
    pub position: [u32; 2],
    pub mover: Option<u32>,
    pub mind: Option<MindId>,
    pub component74: Option<u32>,
    pub organization: Option<u32>,
    pub element: Option<u32>,
    pub component88: Option<u32>,
    pub component_d0: bool,
    /// Exact Actor+148/+14C registry; index4 is read only when signed length>4.
    pub kingdom_slots: Vec<Option<u32>>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Element {
    pub organization_actor: Option<u32>,
    pub special: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Mover {
    pub last_step_speed: u32,
    pub speed: u32,
    pub maximum_speed: u32,
    pub locked: bool,
    pub turning: bool,
    pub falling: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MindEvent {
    DetachedFc,
    Failed124,
    TargetLost140(u32),
    TargetReachedF4(u32),
    Completed144,
}

/// All methods are required. Callbacks may mutate payload/Arena and component
/// state; subsequent reads follow native order. They must retain the active
/// state's identity/storage until return. Earlier effects survive errors.
pub trait World {
    fn actor(&self, actor: u32) -> Result<Actor>;
    fn element(&self, element: u32) -> Result<Element>;
    fn mover(&self, mover: u32) -> Result<Mover>;
    fn resolve(&self, generation_id: u32) -> Result<Option<u32>>;
    fn world_time_bits(&self) -> Result<u32>;
    fn world_extents_bits(&self) -> Result<[u32; 2]>;
    /// Source-bound MoverDetachedSeconds SVar, after real lazy registration.
    fn ensure_detached_setting(&mut self) -> Result<()>;
    fn detached_seconds_bits(&self) -> Result<u32>;
    ///529E60 one-time diagnostic state. Logging must not silently vanish.
    fn diagnostic_already_emitted(&self) -> bool;
    fn emit_status_diagnostic(
        &mut self,
        state: StateId,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    fn set_diagnostic_emitted(&mut self);
    /// Exact578E90 on leader+7C, with live Organization/Element behavior.
    fn organization_ready(
        &mut self,
        organization: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    ///54EF40 for the current tile's live list and the selected kingdom.
    fn destination_has_kingdom_object(
        &mut self,
        point: [u32; 2],
        kingdom: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    ///5693B0, then4D10A0; each only reached by its native gate.
    fn component88_blocks(
        &mut self,
        component: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    fn actor_can_move(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<bool>;
    fn mind_event(
        &mut self,
        mind: MindId,
        event: MindEvent,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    /// Full533C30 moving-target update, including target-relative prediction.
    fn update_target(
        &mut self,
        state: StateId,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<Option<u32>>;
    /// Actor virtual0 preserves its extended x87 result.
    fn radius(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary>;
    fn property(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<u32>;
    fn accepts_target_property(
        &mut self,
        actor: u32,
        property: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    ///4EA0F0 source descriptor first, target second, then46F100(target,range,
    ///source,false). This uses actual virtual+10 soft shapes, not range+14.
    fn soft_shapes_in_range(
        &mut self,
        source: u32,
        target: u32,
        range_bits: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    /// Mind virtual+4C,+50,+54,+6C, preserving repeated dispatches.
    fn mind_target(
        &mut self,
        mind: MindId,
        slot: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<Option<u32>>;
    fn mind_has_location(&mut self, mind: MindId, p: &mut Payload, a: &mut Arena) -> Result<bool>;
    fn mind_location(&mut self, mind: MindId, p: &mut Payload, a: &mut Arena) -> Result<[u32; 2]>;
    fn target_point(
        &mut self,
        target: u32,
        property: u32,
        point: &mut [u32; 2],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    ///57DD90/57DEA0 actual component74 state transitions.
    fn component74_target(
        &mut self,
        component: u32,
        mode: u32,
        target: Option<u32>,
        point: [u32; 2],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    fn clear_component74(&mut self, component: u32, p: &mut Payload, a: &mut Arena) -> Result<()>;
    /// Native7AA3D0 must resolve a real region; missing is invalid, not region0.
    fn region_at(&self, point: [u32; 2]) -> Result<u32>;
    fn region_neighbors(&self, region: u32) -> Result<Vec<u32>>;
    /// Actual5336B0 and5379C0 implementations, with their real path services.
    fn result_plan(
        &mut self,
        state: StateId,
        trigger: Option<u32>,
        argument: bool,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    fn continue_path(&mut self, state: StateId, p: &mut Payload, a: &mut Arena) -> Result<()>;
    ///5767C0(mover,false,false),4DB0F0,4CFC80, in that order when reached.
    fn stop_mover(&mut self, mover: u32, p: &mut Payload, a: &mut Arena) -> Result<()>;
    fn mover_active(&self, mover: u32) -> Result<bool>;
    fn stop_character(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<()>;
    fn schedule(
        &mut self,
        mind: MindId,
        delay_bits: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    /// Pure field writes for576D20. The initial request store precedes clamp.
    fn write_requested_speed(&mut self, mover: u32, bits: u32) -> Result<()>;
    fn write_speed_factor(&mut self, mover: u32, bits: u32) -> Result<()>;
}
fn err(s: &str) -> Error {
    Error::World(format!("ElementMove: {s}"))
}
fn num<T>(r: aa_numeric::Result<T>) -> Result<T> {
    r.map_err(|e| err(&e.to_string()))
}
fn b(bits: u32) -> Result<Binary> {
    num(Binary::from_f32_bits(bits))
}
pub fn parent(state: StateId, a: &Arena) -> Result<MindId> {
    Ok(a.state(state)?.parent)
}
pub fn owner(state: StateId, a: &Arena) -> Result<u32> {
    Ok(a.mind(parent(state, a)?)?.actor)
}
fn active(state: StateId, a: &Arena) -> Result<bool> {
    Ok(a.mind(parent(state, a)?)?.stack.front().copied().flatten() == Some(state))
}
fn mover(state: StateId, a: &Arena, w: &impl World) -> Result<u32> {
    w.actor(owner(state, a)?)?
        .mover
        .ok_or_else(|| err("required Mover is null"))
}
fn event(
    state: StateId,
    e: MindEvent,
    p: &mut Payload,
    a: &mut Arena,
    w: &mut impl World,
) -> Result<bool> {
    w.mind_event(parent(state, a)?, e, p, a)
}
///52CD90 clears nodes AND native metadata, unlike5189F0 clear_nodes alone.
pub fn clear_path(p: &mut Payload) {
    let path = &mut p.plan.movement.path;
    path.clear_nodes();
    path.kind1_count = 0;
    path.kind2_count = 0;
    path.word18 = u32::MAX;
    path.dirty = true;
}
///52A1F0/52A240: X delta stores to binary32; Y remains extended.
pub fn near_point(
    position: [u32; 2],
    point: [u32; 2],
    threshold: u32,
    precision: Precision,
) -> Result<bool> {
    let x = b(num(b(point[0])?.sub(&b(position[0])?, precision))?
        .store_f32()
        .map_err(|e| err(&e.to_string()))?)?;
    let y = num(b(point[1])?.sub(&b(position[1])?, precision))?;
    Ok(
        num(num(y.mul(&y, precision))?.add(&num(x.mul(&x, precision))?, precision))?
            .compare(&b(threshold)?)
            .is_lt(),
    )
}
///5525D0 preserves signed-zero for in-bounds coordinates; upper clamp stores
///extent minus source0.0001 only when the coordinate is strictly above it.
pub fn clamp_destination(
    point: &mut [u32; 2],
    extents: [u32; 2],
    precision: Precision,
) -> Result<()> {
    for i in 0..2 {
        let upper = num(b(extents[i])?.sub(&b(0x38d1b717)?, precision))?;
        if b(point[i])?.is_negative_nonzero() {
            point[i] = 0;
        } else if upper.compare(&b(point[i])?).is_lt() {
            point[i] = num(upper.store_f32())?;
        }
    }
    Ok(())
}
///52D660, including repeated live Mover lookup and Character fallback.
pub fn stop(state: StateId, p: &mut Payload, a: &mut Arena, w: &mut impl World) -> Result<()> {
    if w.actor(owner(state, a)?)?.mover.is_some() {
        let m = mover(state, a, w)?;
        w.stop_mover(m, p, a)?;
    }
    let active = if w.actor(owner(state, a)?)?.mover.is_some() {
        w.mover_active(mover(state, a, w)?)?
    } else {
        false
    };
    if !active {
        w.stop_character(owner(state, a)?, p, a)?;
    }
    Ok(())
}
///52FC50 OnEnter.
pub fn enter(
    state: StateId,
    p: &mut Payload,
    a: &mut Arena,
    precision: Precision,
    w: &mut impl World,
) -> Result<()> {
    let speed = p.plan.movement.requested_speed_bits;
    a.state_mut(state)?.status = 2;
    if b(speed)?.compare(&b(0x38d1b717)?).is_lt() {
        p.plan.movement.requested_speed_bits = w.mover(mover(state, a, w)?)?.maximum_speed;
    }
    p.plan.previous_destination_bits = [0, 0];
    if p.plan.movement.phase != 1 {
        clear_path(p);
    }
    clamp_destination(
        &mut p.plan.movement.destination_bits,
        w.world_extents_bits()?,
        precision,
    )?;
    p.plan.attempts = 0;
    p.word78 = 0;
    if p.plan.movement.mode == 0
        && near_point(
            w.actor(owner(state, a)?)?.position,
            p.plan.movement.destination_bits,
            0x3e800000,
            precision,
        )?
    {
        p.plan.movement.mode = 1;
    }
    let speed = p.plan.movement.requested_speed_bits;
    let m = mover(state, a, w)?;
    let maximum = w.mover(m)?.maximum_speed;
    w.write_requested_speed(m, speed)?;
    if b(maximum)?.compare(&b(speed)?).is_lt() {
        w.write_requested_speed(m, maximum)?;
    }
    w.write_speed_factor(m, 0x3f800000)?;
    p.detached_time_bits = w.world_time_bits()?;
    Ok(())
}
///52E0F0 OnLeave captures the actor before the component callback.
pub fn leave(state: StateId, p: &mut Payload, a: &mut Arena, w: &mut impl World) -> Result<()> {
    a.state_mut(state)?.status = 0;
    let actor = owner(state, a)?;
    if let Some(component) = w.actor(actor)?.component74 {
        w.clear_component74(component, p, a)?;
    }
    clear_path(p);
    let m = w
        .actor(actor)?
        .mover
        .ok_or_else(|| err("OnLeave Mover is null"))?;
    if w.mover_active(m)? {
        stop(state, p, a, w)?;
    }
    Ok(())
}

fn movement_blocked(
    state: StateId,
    p: &mut Payload,
    a: &mut Arena,
    w: &mut impl World,
) -> Result<bool> {
    let actor = owner(state, a)?;
    if let Some(m) = w.actor(actor)?.mover
        && w.mover(m)?.locked
    {
        return Ok(true);
    }
    if let Some(c) = w.actor(actor)?.component88
        && w.component88_blocks(c, p, a)?
    {
        return Ok(true);
    }
    if w.actor(actor)?.flags & (1 << 29) != 0 {
        return Ok(true);
    }
    Ok(!w.actor_can_move(actor, p, a)?)
}
fn moving(m: Mover) -> bool {
    m.turning || f32::from_bits(m.speed) != 0.0 || f32::from_bits(m.last_step_speed) != 0.0
}
fn moving_target_valid(state: StateId, p: &Payload, a: &Arena, w: &impl World) -> Result<bool> {
    let Some(target) = w.resolve(p.plan.movement.target_id)? else {
        return Ok(false);
    };
    let flags = w.actor(target)?.flags;
    Ok(flags & ((1 << 29) | 1) == 0 && target != owner(state, a)?)
}
fn arrived(
    state: StateId,
    p: &mut Payload,
    a: &mut Arena,
    precision: Precision,
    w: &mut impl World,
) -> Result<bool> {
    if near_point(
        w.actor(owner(state, a)?)?.position,
        p.plan.movement.destination_bits,
        0x3c23d70a,
        precision,
    )? {
        return Ok(true);
    }
    let Some(target) = w.resolve(p.plan.movement.target_id)? else {
        return Ok(false);
    };
    if w.actor(target)?.flags & (1 << 10) == 0 {
        return Ok(false);
    }
    w.soft_shapes_in_range(owner(state, a)?, target, 0x3dcccccd, p, a)
}
fn same_or_neighbor(p: &Payload, point: [u32; 2], w: &impl World) -> Result<bool> {
    let previous = p.plan.movement.path.word18;
    if (previous as i32) < 0 {
        return Ok(false);
    }
    let current = w.region_at(point)?;
    Ok(current == previous || w.region_neighbors(current)?.contains(&previous))
}
fn reachable_event(
    state: StateId,
    target: Option<u32>,
    p: &mut Payload,
    a: &mut Arena,
    w: &mut impl World,
) -> Result<()> {
    if let Some(target) = target {
        let property = w.property(target, p, a)?;
        if w.actor(target)?.component_d0
            && w.accepts_target_property(owner(state, a)?, property, p, a)?
        {
            event(state, MindEvent::TargetReachedF4(target), p, a, w)?;
            return Ok(());
        }
    }
    event(state, MindEvent::Completed144, p, a, w)?;
    Ok(())
}
fn update_component74(
    state: StateId,
    component: u32,
    p: &mut Payload,
    a: &mut Arena,
    w: &mut impl World,
) -> Result<()> {
    let actor = owner(state, a)?;
    let replacement = if w.actor(actor)?.organization.is_none()
        && let Some(element) = w.actor(actor)?.element
    {
        w.element(element)?.organization_actor.unwrap_or(actor)
    } else {
        actor
    };
    let Some(mind) = w.actor(replacement)?.mind else {
        return w.clear_component74(component, p, a);
    };
    if w.mind_target(mind, 0x4c, p, a)?.is_some() {
        let mind = w
            .actor(replacement)?
            .mind
            .ok_or_else(|| err("replacement Mind removed"))?;
        let target = w
            .mind_target(mind, 0x4c, p, a)?
            .ok_or_else(|| err("second target getter returned null"))?;
        let mut point = [0, 0];
        let property = w.property(owner(state, a)?, p, a)?;
        if w.target_point(target, property, &mut point, p, a)? {
            return w.component74_target(component, 1, Some(target), point, p, a);
        }
    } else if let Some(mind) = w.actor(replacement)?.mind
        && w.mind_has_location(mind, p, a)?
    {
        let mind = w
            .actor(replacement)?
            .mind
            .ok_or_else(|| err("replacement Mind removed"))?;
        let target = w.mind_target(mind, 0x50, p, a)?;
        if let Some(target) = target {
            let mut point = [0, 0];
            let property = w.property(owner(state, a)?, p, a)?;
            if w.target_point(target, property, &mut point, p, a)? {
                return w.component74_target(component, 3, Some(target), point, p, a);
            }
        } else {
            let mind = w
                .actor(replacement)?
                .mind
                .ok_or_else(|| err("replacement Mind removed"))?;
            let point = w.mind_location(mind, p, a)?;
            return w.component74_target(component, 3, None, point, p, a);
        }
    }
    w.clear_component74(component, p, a)
}
/// Complete537B90 outer decision order; no synthetic Mover-order shortcut.
pub fn think(
    state: StateId,
    p: &mut Payload,
    a: &mut Arena,
    precision: Precision,
    w: &mut impl World,
) -> Result<()> {
    if a.state(state)?.status != 2 {
        if !w.diagnostic_already_emitted() {
            w.emit_status_diagnostic(state, p, a)?;
            w.set_diagnostic_emitted();
        }
        a.state_mut(state)?.status = 2;
    }
    w.ensure_detached_setting()?;
    let mut attached = true;
    if let Some(element) = w.actor(owner(state, a)?)?.element
        && let Some(leader) = w.element(element)?.organization_actor
    {
        let element = w
            .actor(owner(state, a)?)?
            .element
            .ok_or_else(|| err("Element removed during snapshot"))?;
        if !w.element(element)?.special {
            let organization = w
                .actor(leader)?
                .organization
                .ok_or_else(|| err("leader Organization is null"))?;
            if !w.organization_ready(organization, p, a)? {
                attached = false;
            } else {
                let kingdom = w.actor(leader)?.kingdom_slots.get(4).copied().flatten();
                if active(state, a)?
                    && let Some(kingdom) = kingdom
                {
                    let point = p.plan.movement.destination_bits;
                    let extents = w.world_extents_bits()?;
                    for bits in point.into_iter().chain(extents) {
                        b(bits)?;
                    }
                    let inside = point.into_iter().zip(extents).all(|(v, e)| {
                        f32::from_bits(v) >= 0.0 && f32::from_bits(v) < f32::from_bits(e)
                    });
                    if inside
                        && !w.destination_has_kingdom_object(
                            p.plan.movement.destination_bits,
                            kingdom,
                            p,
                            a,
                        )?
                    {
                        attached = false;
                    }
                }
            }
        }
    }
    if attached {
        p.detached_time_bits = w.world_time_bits()?;
    } else if active(state, a)?
        && !num(b(w.world_time_bits()?)?.sub(&b(p.detached_time_bits)?, precision))?
            .compare(&b(w.detached_seconds_bits()?)?)
            .is_lt()
        && event(state, MindEvent::DetachedFc, p, a, w)?
    {
        return Ok(());
    }
    let Some(mover_id) = w.actor(owner(state, a)?)?.mover else {
        event(state, MindEvent::Failed124, p, a, w)?;
        return Ok(());
    };
    if w.mover(mover_id)?.falling {
        event(state, MindEvent::Failed124, p, a, w)?;
        return Ok(());
    }
    if movement_blocked(state, p, a, w)? {
        if moving(w.mover(mover_id)?) {
            stop(state, p, a, w)?;
        }
        return w.schedule(parent(state, a)?, 0x3f800000, p, a);
    }
    let mut trigger = None;
    if p.plan.movement.target_id != 0 && p.plan.movement.mode == 2 {
        trigger = w.update_target(state, p, a)?;
        let Some(target) = trigger else {
            event(
                state,
                MindEvent::TargetLost140(p.plan.movement.target_id),
                p,
                a,
                w,
            )?;
            return stop(state, p, a, w);
        };
        if moving_target_valid(state, p, a, w)? && active(state, a)? {
            let actor = owner(state, a)?;
            if target == actor {
                return reachable_event(state, None, p, a, w);
            }
            let origin = w.actor(owner(state, a)?)?.position;
            let destination = w.actor(target)?.position;
            let x = num(b(destination[0])?.sub(&b(origin[0])?, precision))?;
            let y = num(b(destination[1])?.sub(&b(origin[1])?, precision))?;
            let distance = b(num(num(
                num(x.mul(&x, precision))?.add(&num(y.mul(&y, precision))?, precision)
            )?
            .sqrt(precision))?
            .store_f32()
            .map_err(|e| err(&e.to_string()))?)?;
            let radius = w.radius(owner(state, a)?, p, a)?;
            let distance = b(num(distance.sub(&radius, precision))?
                .store_f32()
                .map_err(|e| err(&e.to_string()))?)?;
            let target_radius = w.radius(target, p, a)?;
            if num(distance.sub(&target_radius, precision))?
                .compare(&b(0x40a00000)?)
                .is_lt()
            {
                return reachable_event(state, Some(target), p, a, w);
            }
        }
    }
    if let Some(component) = w.actor(owner(state, a)?)?.component74 {
        update_component74(state, component, p, a, w)?;
    }
    if !p.plan.movement.path.nodes.is_empty() {
        if !same_or_neighbor(p, w.actor(owner(state, a)?)?.position, w)? {
            return w.continue_path(state, p, a);
        }
        p.plan.attempts = 0;
    }
    let mut replan = true;
    if p.plan.movement.phase == 2 {
        p.plan.movement.phase = 1;
        if p.collision[0] == 2
            && let Some(actor) = w.resolve(p.collision[1])?
            && w.actor(actor)?.flags & (1 << 29) == 0
        {
            replan = false;
        }
    }
    let first = p.plan.movement.mode == 3
        && near_point(
            w.actor(owner(state, a)?)?.position,
            p.plan.movement.destination_bits,
            0x3e800000,
            precision,
        )?;
    let done = if first {
        true
    } else {
        let mut value = false;
        if p.plan.movement.mode != 0 {
            value = arrived(state, p, a, precision, w)?;
        }
        if !value && p.plan.movement.mode == 0 {
            value = near_point(
                w.actor(owner(state, a)?)?.position,
                p.plan.movement.destination_bits,
                0x3e800000,
                precision,
            )?;
        }
        value
    };
    if done {
        let target = w.resolve(p.plan.movement.target_id)?;
        return reachable_event(state, target, p, a, w);
    }
    w.result_plan(state, trigger, replan, p, a)
}
