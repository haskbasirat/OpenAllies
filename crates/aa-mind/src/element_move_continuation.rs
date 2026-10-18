//! Original5379C0 continuation,52FD90 turn offset,537150 forward check and
//!536F40 collision result. Shared payload remains owned by ElementMove.
//! Collision recovery/sweeps are required services, never clear-world defaults.
use crate::{
    Arena, Error, MindId, Result, StateId,
    element_move::{Payload, owner, parent},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub flags: u32,
    pub position: [u32; 2],
    pub facing: [u32; 2],
    pub mover: Option<Mover>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Mover {
    pub turn_radius: u32,
    pub requested_direction: [u32; 2],
}
/// Descriptor source selected by536F40. The host must construct the original
///4CF210 actor or52F3E0 tile descriptor before534A40; it must not use bounds
/// inferred from rendering geometry.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Obstacle {
    Actor(u32),
    Tile([u32; 2]),
}

/// Captured532B50 stack state at532C64, after its exact early decisions. This
/// is a continuation contract for the remaining turning recovery algorithm,
/// not a generated route. All bits are the original stored binary32 values.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnRecovery {
    pub head: aa_navigation::PathNode,
    pub facing: [u32; 2],
    pub requested_direction: [u32; 2],
    pub position: [u32; 2],
    pub perpendicular_squared_bits: u32,
}
pub trait TurnRecoveryWorld: World {
    /// Remaining532C64..533128 native turn geometry, placed-shape query,
    /// forward sweep and node insertion. A missing implementation is an error.
    fn turn_recovery(
        &mut self,
        state: StateId,
        input: TurnRecovery,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
}

/// Concrete remaining query services used by532B50. Each service must build
/// the original actor descriptor and preserve its ignore-target rules.
pub trait RecoveryGeometryWorld: World {
    ///554350(actor,false), flags|0x20000000,552250(endpoint),5522B0(direction),
    /// current target generation lookup→4CE6E0 ignore actor, then553BA0.
    fn recovery_placement_blocked(
        &mut self,
        state: StateId,
        endpoint: [u32; 2],
        direction: [u32; 2],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    ///5312C0 explicit-origin forward sweep. A zero collision result is supplied
    /// by the caller; the boolean return, not that result, controls532B50.
    #[allow(clippy::too_many_arguments)] // Original explicit-origin sweep ABI.
    fn recovery_probe(
        &mut self,
        state: StateId,
        origin: [u32; 2],
        direction: [u32; 2],
        distance: u32,
        result: &mut [u32; 4],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    fn recovery_diagnostic(
        &mut self,
        actor: u32,
        distance: u32,
        offset: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
}

fn prepend(p: &mut Payload, xy: [u32; 2], direction: [u32; 2], kind: u32) -> Result<()> {
    let path = &mut p.plan.movement.path;
    path.nodes.try_reserve(1).map_err(|_| Error::Allocation)?;
    path.nodes.insert(
        0,
        aa_navigation::PathNode {
            words: [
                xy[0],
                xy[1],
                direction[0],
                direction[1],
                kind,
                0,
                xy[0],
                xy[1],
            ],
        },
    );
    if kind == 1 {
        path.kind1_count = path.kind1_count.wrapping_add(1);
    } else if kind == 2 {
        path.kind2_count = path.kind2_count.wrapping_add(1);
    }
    path.dirty = true;
    Ok(())
}
/// Remaining532C64..533128 geometry and node ownership. This is the concrete
/// implementation for TurnRecoveryWorld::turn_recovery, not an approximate
/// straight-line substitute. Query callbacks may change the live state.
pub fn recover_turn<W: RecoveryGeometryWorld>(
    state: StateId,
    input: TurnRecovery,
    p: &mut Payload,
    a: &mut Arena,
    precision: Precision,
    w: &mut W,
) -> Result<bool> {
    let node = input.head;
    let origin = input.position;
    let request = input.requested_direction;
    let facing = input.facing;
    let x = num(b(node.words[0])?.sub(&b(origin[0])?, precision))?;
    let y = num(b(node.words[1])?.sub(&b(origin[1])?, precision))?;
    let cross = num(num(x.mul(&b(request[1])?, precision))?
        .sub(&num(y.mul(&b(request[0])?, precision))?, precision))?;
    let left = cross.is_negative_nonzero();
    let radius = w
        .actor(owner(state, a)?)?
        .mover
        .ok_or_else(|| err("required Mover is null"))?
        .turn_radius;
    let angle = aa_trig::turning::recovery_angle(
        radius,
        input.perpendicular_squared_bits,
        precision,
        w.precision_exception_pending(),
    )
    .map_err(err)?;
    // The first original table rotation is overwritten, but still executes.
    aa_trig::rotate(request, angle, !left, precision).map_err(err)?;
    let (sin, cos) = aa_trig::turning::small_sin_cos(angle, precision).map_err(err)?;
    let radial = b(radius)?;
    let fx = b(facing[0])?;
    let fy = b(facing[1])?;
    let xr = b(num(num(fx.mul(&radial, precision))?.store_f32())?)?;
    let yr = num(fy.mul(&radial, precision))?;
    let base_x = b(num(num(
        num(xr.mul(&sin, precision))?.add(&b(origin[0])?, precision)
    )?
    .store_f32())?)?;
    let base_y = b(num(num(
        b(origin[1])?.add(&num(yr.mul(&sin, precision))?, precision)
    )?
    .store_f32())?)?;
    let one_minus = b(num(
        num(Binary::from_u32(1).sub(&cos, precision))?.store_f32()
    )?)?;
    let (px, py) = if left {
        (fy.negated(), fx)
    } else {
        (fy, fx.negated())
    };
    let pxr = b(num(num(px.mul(&radial, precision))?.store_f32())?)?;
    let pyr = num(py.mul(&radial, precision))?;
    let mut arc = [
        num(num(num(pxr.mul(&one_minus, precision))?.add(&base_x, precision))?.store_f32())?,
        num(num(base_y.add(&num(pyr.mul(&one_minus, precision))?, precision))?.store_f32())?,
    ];
    let mut direction = aa_trig::rotate(facing, angle, !left, precision).map_err(err)?;
    //52DDB0 preserves the initialized output (angle) when the lines are parallel.
    let denominator = num(num(b(request[1])?.mul(&b(direction[0])?, precision))?.sub(
        &num(b(direction[1])?.mul(&b(request[0])?, precision))?,
        precision,
    ))?;
    let mut t = angle;
    if !denominator.is_zero() {
        let x = num(b(node.words[0])?.sub(&b(arc[0])?, precision))?;
        let y = num(b(node.words[1])?.sub(&b(arc[1])?, precision))?;
        let numerator = num(num(x.mul(&b(direction[1])?, precision))?
            .sub(&num(y.mul(&b(direction[0])?, precision))?, precision))?;
        t = num(num(numerator.div(&denominator, precision))?.store_f32())?;
    }
    let straight = b(t)?.compare(&Binary::from_u32(0)).is_gt();
    let endpoint;
    let distance;
    if straight {
        arc = origin;
        endpoint = [node.words[0], node.words[1]];
        let x = num(b(endpoint[0])?.sub(&b(origin[0])?, precision))?;
        let y = num(b(endpoint[1])?.sub(&b(origin[1])?, precision))?;
        distance = num(num(
            num(x.mul(&x, precision))?.add(&num(y.mul(&y, precision))?, precision)
        )?
        .sqrt(precision))?
        .store_f32()
        .map_err(err)?;
        let inverse = b(num(
            num(Binary::from_u32(1).div(&b(distance)?, precision))?.store_f32(),
        )?)?;
        direction = [
            num(num(x.mul(&inverse, precision))?.store_f32())?,
            num(num(y.mul(&inverse, precision))?.store_f32())?,
        ];
    } else {
        let x =
            num(b(node.words[0])?.add(&num(b(request[0])?.mul(&b(t)?, precision))?, precision))?;
        let y = num(num(
            b(node.words[1])?.add(&num(b(request[1])?.mul(&b(t)?, precision))?, precision)
        )?
        .store_f32())?;
        endpoint = [num(x.store_f32())?, y];
        let dx = num(x.sub(&b(arc[0])?, precision))?;
        let dy = num(b(y)?.sub(&b(arc[1])?, precision))?;
        distance = num(num(
            num(dx.mul(&dx, precision))?.add(&num(dy.mul(&dy, precision))?, precision)
        )?
        .sqrt(precision))?
        .store_f32()
        .map_err(err)?;
    }
    if w.recovery_placement_blocked(state, endpoint, direction, p, a)? {
        return Ok(false);
    }
    let radius = w.movement_radius(owner(state, a)?, p, a)?;
    let maximum = num(radius.mul(&b(0x3fc00000)?, precision))?;
    let lead = if b(distance)?.compare(&maximum).is_lt() {
        distance
    } else {
        num(maximum.store_f32())?
    };
    let mut collision = [0; 4];
    if w.recovery_probe(state, arc, direction, lead, &mut collision, p, a)? {
        return Ok(false);
    }
    let mut add_kind1 = true;
    if straight {
        let xp = b(num(
            num(b(direction[0])?.mul(&b(lead)?, precision))?.store_f32()
        )?)?;
        let yp = num(b(direction[1])?.mul(&b(lead)?, precision))?;
        let point = [
            num(num(xp.add(&b(arc[0])?, precision))?.store_f32())?,
            num(num(yp.add(&b(arc[1])?, precision))?.store_f32())?,
        ];
        prepend(p, point, direction, 0)?;
        add_kind1 = num(b(lead)?.add(&b(0x38d1b717)?, precision))?
            .compare(&b(distance)?)
            .is_lt();
    }
    if add_kind1 {
        prepend(p, endpoint, direction, 1)?;
    }
    let radius = w
        .actor(owner(state, a)?)?
        .mover
        .ok_or_else(|| err("required Mover removed"))?
        .turn_radius;
    waypoint_offset(p, radius, precision, w.precision_exception_pending())?;
    if !w.action_go(state, 0, p, a)? {
        w.recovery_diagnostic(
            owner(state, a)?,
            distance,
            p.plan.movement.waypoint_offset_bits,
            p,
            a,
        )?;
    }
    Ok(true)
}

///532B50 common path with original intermediate stores. Ordinary movement on
/// the current segment returns false here, without a collision approximation.
/// Kind0 drift drops the entire leading kind0 chain before restarting ActionGo.
pub fn get_back_to_path<W: TurnRecoveryWorld>(
    state: StateId,
    p: &mut Payload,
    a: &mut Arena,
    precision: Precision,
    w: &mut W,
) -> Result<bool> {
    let node = head(p)?;
    let facing = w.actor(owner(state, a)?)?.facing;
    let direction = w
        .actor(owner(state, a)?)?
        .mover
        .ok_or_else(|| err("required Mover is null"))?
        .requested_direction;
    let position = w.actor(owner(state, a)?)?.position;
    if !equal(facing, [node.words[2], node.words[3]])? {
        return Ok(false);
    }
    let x = b(num(num(
        b(node.words[0])?.sub(&b(position[0])?, precision)
    )?
    .store_f32())?)?;
    let y = num(b(node.words[1])?.sub(&b(position[1])?, precision))?;
    let dot = num(num(b(direction[1])?.mul(&y, precision))?
        .add(&num(x.mul(&b(direction[0])?, precision))?, precision))?;
    let projected_y = b(num(num(b(direction[1])?.mul(&dot, precision))?.store_f32())?)?;
    let projected_x = b(num(num(b(direction[0])?.mul(&dot, precision))?.store_f32())?)?;
    let y = num(y.sub(&projected_y, precision))?;
    let x = num(x.sub(&projected_x, precision))?;
    let length = num(
        num(num(x.mul(&x, precision))?.add(&num(y.mul(&y, precision))?, precision))?
            .sqrt(precision),
    )?;
    let squared = num(length.mul(&length, precision))?;
    // Compare the extended squared value, not the separately stored argument.
    let perpendicular_squared_bits = num(squared.store_f32())?;
    if squared.compare(&Binary::from_u32(1)).is_lt() {
        return Ok(false);
    }
    if node.words[4] == 0 {
        while p
            .plan
            .movement
            .path
            .nodes
            .first()
            .is_some_and(|n| n.words[4] == 0)
        {
            p.plan.movement.path.nodes.remove(0);
            p.plan.movement.path.dirty = true;
        }
        w.action_go(state, 0, p, a)?;
        return Ok(true);
    }
    w.turn_recovery(
        state,
        TurnRecovery {
            head: node,
            facing,
            requested_direction: direction,
            position,
            perpendicular_squared_bits,
        },
        p,
        a,
    )
}

pub trait World {
    fn actor(&self, actor: u32) -> Result<Actor>;
    fn resolve(&self, id: u32) -> Result<Option<u32>>;
    fn world_step_bits(&self) -> Result<u32>;
    /// Actual incoming x87 precision flag, needed by the original CRT angle
    /// leaf. This is explicit even though ordinary gameplay usually has PE set.
    fn precision_exception_pending(&self) -> bool;
    fn actor_is_moving(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<bool>;
    /// Bind actual aa_navigation::try_action_go with live component callbacks.
    fn action_go(
        &mut self,
        state: StateId,
        speed: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    fn schedule(&mut self, mind: MindId, delay: u32, p: &mut Payload, a: &mut Arena) -> Result<()>;
    ///532B50, including native node insertion and collision queries.
    fn get_back_to_path(&mut self, state: StateId, p: &mut Payload, a: &mut Arena) -> Result<bool>;
    /// Actor virtual18 extended result, not a rounded f32 substitute.
    fn movement_radius(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary>;
    ///531170→553CE0. The input result is zeroed by537150 before this callback.
    fn forward_probe(
        &mut self,
        state: StateId,
        direction: [u32; 2],
        distance: u32,
        result: &mut [u32; 4],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    /// Mind virtual160, distinct from ElementMove Completed144.
    fn node_reached(&mut self, mind: MindId, p: &mut Payload, a: &mut Arena) -> Result<()>;
    ///536840 actual actor avoidance/state transitions (ResultCollisionThing
    /// through 0x5358A0/0x5351C0; it never reaches 0x536AD0).
    fn avoid_actor(
        &mut self,
        state: StateId,
        target: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    fn refresh_visibility(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<()>;
    fn overlaps_actor(
        &mut self,
        actor: u32,
        target: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool>;
    fn overlap_diagnostic(
        &mut self,
        actor: u32,
        target: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    /// Actual virtual+C shape then444420; extended result retained.
    fn bounds_radius(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary>;
    /// 0x536AD0 (collision with a thing at a clearance distance); bind
    /// with [`stop_distance`] over `aa_navigation::uai_collision`.
    fn stop_distance(
        &mut self,
        state: StateId,
        target: u32,
        distance: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    ///530F40(true), not ordinary ResultPlan5336B0.
    fn result_path(&mut self, state: StateId, p: &mut Payload, a: &mut Arena) -> Result<u32>;
    fn wait(&mut self, state: StateId, p: &mut Payload, a: &mut Arena) -> Result<()>;
    /// 0x534A40 ResultAvoidStatic with the 0x536F40 descriptor; bind with
    /// [`collision_plan`] over `aa_navigation::uai_collision`.
    fn collision_plan(
        &mut self,
        state: StateId,
        obstacle: Obstacle,
        direction: [u32; 2],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
}
fn err(s: impl std::fmt::Display) -> Error {
    Error::World(format!("ElementMove continuation: {s}"))
}
fn num<T>(v: aa_numeric::Result<T>) -> Result<T> {
    v.map_err(err)
}
fn b(v: u32) -> Result<Binary> {
    num(Binary::from_f32_bits(v))
}
fn head(p: &Payload) -> Result<aa_navigation::PathNode> {
    p.plan
        .movement
        .path
        .nodes
        .first()
        .copied()
        .ok_or_else(|| err("required path head removed"))
}
fn equal(a: [u32; 2], z: [u32; 2]) -> Result<bool> {
    Ok(b(a[0])?.compare(&b(z[0])?).is_eq() && b(a[1])?.compare(&b(z[1])?).is_eq())
}
///52FD90. This leaf has no callbacks after the source Mover radius is read.
pub fn waypoint_offset(
    p: &mut Payload,
    radius: u32,
    precision: Precision,
    pending: bool,
) -> Result<()> {
    if b(radius)?.is_zero() {
        p.plan.movement.waypoint_offset_bits = 0;
        return Ok(());
    }
    let nodes = &p.plan.movement.path.nodes;
    let Some(first) = nodes.first() else {
        p.plan.movement.waypoint_offset_bits = 0;
        return Ok(());
    };
    if first.words[4] == 0 || nodes.len() == 1 {
        p.plan.movement.waypoint_offset_bits = 0;
        return Ok(());
    }
    let second = nodes[1];
    let (near, raised) = aa_trig::turning::near_waypoint(
        [first.words[0], first.words[1]],
        [second.words[0], second.words[1]],
        precision,
    )
    .map_err(err)?;
    let next = if near {
        if nodes.len() == 2 {
            p.plan.movement.waypoint_offset_bits = 0;
            return Ok(());
        }
        nodes[2]
    } else {
        second
    };
    p.plan.movement.waypoint_offset_bits = aa_trig::turning::waypoint_turn_offset(
        radius,
        [first.words[2], first.words[3]],
        [next.words[2], next.words[3]],
        precision,
        pending || raised,
    )
    .map_err(err)?;
    Ok(())
}
///52FD50→409230 writes collision fields before normalizing the direction.
pub fn save_collision(
    p: &mut Payload,
    result: [u32; 4],
    direction: [u32; 2],
    precision: Precision,
) -> Result<()> {
    p.collision = result;
    p.collision_vector = direction;
    let x = b(direction[0])?;
    let y = b(direction[1])?;
    let length = num(num(x.mul(&x, precision))?.add(&num(y.mul(&y, precision))?, precision))?;
    let length = num(length.sqrt(precision))?;
    if length.compare(&b(0x38d1b717)?).is_gt() {
        let inverse = num(Binary::from_u32(1).div(&length, precision))?;
        p.collision_vector[0] = num(num(inverse.mul(&x, precision))?.store_f32())?;
        p.collision_vector[1] = num(num(inverse.mul(&y, precision))?.store_f32())?;
    } else {
        p.collision_vector = [0, 0];
    }
    Ok(())
}
pub fn continue_path(
    state: StateId,
    p: &mut Payload,
    a: &mut Arena,
    precision: Precision,
    w: &mut impl World,
) -> Result<()> {
    if p.plan.movement.phase == 2 {
        return collision_result(state, p, a, precision, w);
    }
    if !w.actor_is_moving(owner(state, a)?, p, a)? {
        let mover = w
            .actor(owner(state, a)?)?
            .mover
            .ok_or_else(|| err("required Mover is null"))?;
        waypoint_offset(
            p,
            mover.turn_radius,
            precision,
            w.precision_exception_pending(),
        )?;
        w.action_go(state, p.plan.movement.requested_speed_bits, p, a)?;
        if let Some(first) = p.plan.movement.path.nodes.first() {
            let delay = if first.words[4] == 0 {
                0x3f800000
            } else {
                0x38d1b717
            };
            w.schedule(parent(state, a)?, delay, p, a)?;
        }
        return Ok(());
    }
    let mover = w
        .actor(owner(state, a)?)?
        .mover
        .ok_or_else(|| err("required Mover is null"))?;
    let node = head(p)?;
    if !equal(mover.requested_direction, [node.words[2], node.words[3]])? {
        if w.action_go(state, 0, p, a)? && p.plan.movement.phase != 0 {
            w.schedule(parent(state, a)?, 0x38d1b717, p, a)?;
        }
    } else if !w.get_back_to_path(state, p, a)? {
        if p.plan.movement.phase == 0 {
            w.action_go(state, 0, p, a)?;
            w.schedule(parent(state, a)?, 0x3f800000, p, a)?;
        } else {
            let radius = w.movement_radius(owner(state, a)?, p, a)?;
            let distance = num(num(radius.mul(&b(0x3fc00000)?, precision))?.store_f32())?;
            forward_check(state, p, a, distance, precision, w)?;
        }
    }
    Ok(())
}
///537150. The Y delta stores before the dot product; X stays extended.
pub fn forward_check(
    state: StateId,
    p: &mut Payload,
    a: &mut Arena,
    mut distance: u32,
    precision: Precision,
    w: &mut impl World,
) -> Result<()> {
    let node = head(p)?;
    if !equal(
        w.actor(owner(state, a)?)?.facing,
        [node.words[2], node.words[3]],
    )? || w.actor(owner(state, a)?)?.flags & (1 << 18) != 0
    {
        w.action_go(state, 0, p, a)?;
        return Ok(());
    }
    let origin = w.actor(owner(state, a)?)?.position;
    let node = head(p)?;
    let y = b(num(
        num(b(node.words[1])?.sub(&b(origin[1])?, precision))?.store_f32()
    )?)?;
    let x = num(b(node.words[0])?.sub(&b(origin[0])?, precision))?;
    let projection = num(num(y.mul(&b(node.words[3])?, precision))?
        .add(&num(x.mul(&b(node.words[2])?, precision))?, precision))?;
    if b(distance)?.compare(&projection).is_gt() {
        distance = num(projection.store_f32())?;
    }
    if projection.compare(&b(0x38d1b717)?).is_lt() {
        return w.node_reached(parent(state, a)?, p, a);
    }
    let step =
        num(b(w.world_step_bits()?)?.mul(&b(p.plan.movement.requested_speed_bits)?, precision))?;
    if b(distance)?.compare(&step).is_lt() {
        distance = num(step.store_f32())?;
    }
    let mut result = [0; 4];
    w.forward_probe(
        state,
        w.actor(owner(state, a)?)?.facing,
        distance,
        &mut result,
        p,
        a,
    )?;
    if result[0] == 0 {
        w.action_go(state, 0, p, a)?;
        return Ok(());
    }
    let facing = w.actor(owner(state, a)?)?.facing;
    save_collision(p, result, facing, precision)?;
    collision_result(state, p, a, precision, w)
}
pub fn collision_result(
    state: StateId,
    p: &mut Payload,
    a: &mut Arena,
    precision: Precision,
    w: &mut impl World,
) -> Result<()> {
    let mut target = None;
    if p.collision[0] == 2 {
        target = w.resolve(p.collision[1])?;
        let Some(actor) = target else {
            w.action_go(state, 0, p, a)?;
            return Ok(());
        };
        if w.avoid_actor(state, actor, p, a)? {
            return Ok(());
        }
        if w.actor(actor)?.flags & (1 << 29) == 0 {
            w.refresh_visibility(owner(state, a)?, p, a)?;
            if w.overlaps_actor(owner(state, a)?, actor, p, a)? {
                w.overlap_diagnostic(owner(state, a)?, actor, p, a)?;
                w.action_go(state, 0, p, a)?;
                return Ok(());
            }
            let minimum =
                num(num(b(w.world_step_bits()?)?
                    .mul(&b(p.plan.movement.requested_speed_bits)?, precision))?
                .store_f32())?;
            let origin = w.actor(owner(state, a)?)?.position;
            let destination = w.actor(actor)?.position;
            let x = num(b(destination[0])?.sub(&b(origin[0])?, precision))?;
            let y = num(b(destination[1])?.sub(&b(origin[1])?, precision))?;
            let mut distance = num(num(
                num(x.mul(&x, precision))?.add(&num(y.mul(&y, precision))?, precision)
            )?
            .sqrt(precision))?
            .store_f32()
            .map_err(err)?;
            if b(distance)?.compare(&b(minimum)?).is_lt() {
                distance = minimum;
            }
            let own = owner(state, a)?; // cached before target virtual+C
            let other_radius = b(num(w.bounds_radius(actor, p, a)?.store_f32())?)?;
            let own_radius = w.bounds_radius(own, p, a)?;
            let clearance = num(
                num(other_radius.sub(&own_radius, precision))?.add(&b(0x38d1b717)?, precision)
            )?;
            let distance = num(num(b(distance)?.sub(&clearance, precision))?.store_f32())?;
            return w.stop_distance(state, actor, distance, p, a);
        }
        w.refresh_visibility(owner(state, a)?, p, a)?;
    }
    match w.result_path(state, p, a)? {
        1 => w.wait(state, p, a),
        2 => {
            w.action_go(state, 0, p, a)?;
            Ok(())
        }
        _ => {
            let obstacle = target.map_or(
                Obstacle::Tile([p.collision[2], p.collision[3]]),
                Obstacle::Actor,
            );
            w.collision_plan(state, obstacle, p.collision_vector, p, a)
        }
    }
}

/// GWorld inputs of the UAI collision results 0x534A40 / 0x536AD0
/// (`aa_navigation::uai_collision`), apart from UAIStateMove +0x78, which is
/// [`Payload::word78`].
pub struct CollisionEnvironment<'a, 'g> {
    /// GWorld+0xE8.
    pub world_step_bits: u32,
    /// GWorld+0xE0.
    pub world_time_bits: u32,
    /// `LowPathingWaitMinimum`.
    pub wait_minimum_bits: u32,
    /// GWorld+0x23C solver.
    pub solver: &'a mut aa_navigation::low_path::LowPathSolver,
    pub environment: &'a mut aa_navigation::low_path::Environment<'g>,
    /// GMap +4 (tile shift), +8 (cell size) and the embedded +0x30 tile
    /// shape, read by the 0x52F3E0 tile descriptor.
    pub tile_shift: u32,
    pub cell_size_bits: u32,
    pub tile_shape: aa_navigation::uai_collision::Shape,
}

fn collision_context<'a, 'g>(
    env: &'a mut CollisionEnvironment<'_, 'g>,
    word78: &'a mut u32,
) -> aa_navigation::uai_collision::Context<'a, 'g> {
    aa_navigation::uai_collision::Context {
        world_step_bits: env.world_step_bits,
        world_time_bits: env.world_time_bits,
        wait_minimum_bits: env.wait_minimum_bits,
        last_solve_time_bits: word78,
        solver: &mut *env.solver,
        environment: &mut *env.environment,
    }
}

/// [`RecoveryGeometryWorld::collision_plan`] = 0x534A40 ResultAvoidStatic
/// with the descriptor 0x536F40 builds: 0x4CF210 for an actor, 0x52F3E0 for
/// a tile, and a null thing.
pub fn collision_plan<W: aa_navigation::uai_collision::UaiCollisionWorld>(
    obstacle: Obstacle,
    direction: [u32; 2],
    p: &mut Payload,
    env: &mut CollisionEnvironment<'_, '_>,
    nav: &mut W,
) -> Result<aa_navigation::uai_collision::StaticOutcome> {
    use aa_navigation::{result_plan::Object, uai_collision as uc};
    let precision = env.environment.options.precision;
    let mut descriptor = match obstacle {
        Obstacle::Actor(id) => uc::actor_descriptor(Object(id), nav).map_err(err)?,
        Obstacle::Tile([x, y]) => uc::tile_descriptor(
            env.tile_shift,
            env.cell_size_bits,
            env.tile_shape,
            [x as i32, y as i32],
            precision,
        )
        .map_err(err)?,
    };
    let mut ctx = collision_context(env, &mut p.word78);
    uc::avoid_static(
        &mut descriptor,
        direction,
        None,
        &mut p.plan.movement,
        &mut ctx,
        nav,
    )
    .map_err(err)
}

/// [`RecoveryGeometryWorld::stop_distance`] = 0x536AD0 (called directly from
/// 0x536F40 with the clearance distance) on the saved collision vector.
pub fn stop_distance<W: aa_navigation::uai_collision::UaiCollisionWorld>(
    target: u32,
    distance: u32,
    p: &mut Payload,
    env: &mut CollisionEnvironment<'_, '_>,
    nav: &mut W,
) -> Result<aa_navigation::uai_collision::ThingOutcome> {
    use aa_navigation::{result_plan::Object, uai_collision as uc};
    let collision = p.collision_vector;
    let mut ctx = collision_context(env, &mut p.word78);
    uc::avoid_thing(
        Object(target),
        distance,
        collision,
        &mut p.plan.movement,
        &mut ctx,
        nav,
    )
    .map_err(err)
}
