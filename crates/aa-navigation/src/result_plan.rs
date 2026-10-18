//! Native UAI ResultPlan (0x5336b0), with explicit world/controller boundaries.
use crate::MoveState;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("a type-2 collision result references a missing actor")]
    InvalidCollisionActor,
    #[error("ResultPlan world boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// A host-owned object key, not a dereferenceable original process address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Object(pub u32);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub movement: MoveState,
    pub attempts: i32,
    pub previous_destination_bits: [u32; 2],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub object: Object,
    pub position_bits: [u32; 2],
    pub facing_bits: [u32; 2],
    pub turn_radius_bits: u32,
}
/// Original 0x58-byte collision descriptor. Pointer-bearing words contain host
/// object keys supplied by World; callers must not reinterpret them as pointers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionContext {
    pub words: [u32; 22],
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CollisionHit {
    pub kind: u32,
    pub actor_id: u32,
}

pub trait World {
    /// Actor virtual+b4; its result is followed by a fresh +90 read.
    fn special_type(
        &mut self,
        object: Object,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<bool>;
    fn has_special_component(&self, object: Object) -> bool;
    /// GWorld4f2220, preserving two separate position queries.
    fn region_at(&mut self, point: [u32; 2], state: &mut State, actor: &mut Actor) -> Result<i32>;
    ///554350(actor,0), including constructor/shape semantics.
    fn collision_context(
        &mut self,
        object: Object,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<CollisionContext>;
    /// Generation-valid native GTID lookup. No implicit nearest-object fallback.
    fn resolve_actor(&self, id: u32) -> Option<Object>;
    ///4ce6e0 actor or its +7c/+40 replacement.
    fn replacement_actor(&self, object: Object) -> Object;
    fn object_definition(&self, object: Object) -> u32;
    ///553ce0 on the native world grid, returning all required collision identity.
    fn sweep(
        &mut self,
        context: &mut CollisionContext,
        goal: [u32; 2],
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<CollisionHit>;
    ///5313f0, with the original bool argument (0 near-goal,1 turning circle).
    fn continue_turn(&mut self, turn: bool, state: &mut State, actor: &mut Actor) -> Result<()>;
    ///531520 receives ResultPlan's unmodified second argument. The original
    /// body is ported as [`crate::low_path::continue_current`] (with
    /// `ContinuationInput::argument` = this argument); implementations call it
    /// with the UAIStateMove +0x78 last-solve time, the GWorld+0x23C solver and
    /// the shared [`crate::low_path::Environment`].
    fn continue_current(
        &mut self,
        argument: u32,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<()>;
    /// Actor virtual+108. The original invokes it twice when the first is nonzero.
    fn property(&mut self, object: Object, state: &mut State, actor: &mut Actor) -> Result<u32>;
    fn property_active(
        &mut self,
        property: u32,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<bool>;
    ///53e1f0; implementations should call this crate's concrete solver with the
    /// authoritative grid and collision context. No direct Mover shortcut.
    fn solve(
        &mut self,
        context: &mut CollisionContext,
        request: crate::solver::Request,
        state: &mut State,
        actor: &mut Actor,
    ) -> Result<i32>;
    ///52fd90 waypoint offset; its tangent/geometry operation remains separate.
    fn update_waypoint_offset(&mut self, state: &mut State, actor: &mut Actor) -> Result<()>;
    ///5300a0 receives zero speed. Its downstream world scheduler is required.
    fn action_go(&mut self, speed_bits: u32, state: &mut State, actor: &mut Actor) -> Result<()>;
    /// UAI virtual+124, on exhausted attempts or failed Solve.
    fn failed(&mut self, state: &mut State, actor: &mut Actor) -> Result<()>;
}
fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn near(state: &State, actor: &Actor, p: Precision) -> Result<bool> {
    let x = b(actor.position_bits[0])?.sub(&b(state.movement.destination_bits[0])?, p)?;
    let y = b(actor.position_bits[1])?.sub(&b(state.movement.destination_bits[1])?, p)?;
    Ok(x.mul(&x, p)?
        .add(&y.mul(&y, p)?, p)?
        .compare(&b(0x41100000)?)
        == Ordering::Less)
}
fn within_turn_circle(state: &State, actor: &Actor, p: Precision) -> Result<bool> {
    let facing = actor.facing_bits;
    let dy = b(b(state.movement.destination_bits[1])?
        .sub(&b(actor.position_bits[1])?, p)?
        .store_f32()?)?;
    let dx_extended = b(state.movement.destination_bits[0])?.sub(&b(actor.position_bits[0])?, p)?;
    let dx = b(dx_extended.store_f32()?)?;
    let projection = dx_extended
        .mul(&b(facing[0])?, p)?
        .add(&dy.mul(&b(facing[1])?, p)?, p)?;
    if projection.is_negative_nonzero() {
        return Ok(false);
    }
    let mut perpendicular = [b(facing[1] ^ 0x80000000)?, b(facing[0])?];
    let side = dx
        .mul(&perpendicular[0], p)?
        .add(&dy.mul(&b(facing[0])?, p)?, p)?;
    if side.is_negative_nonzero() {
        perpendicular[0] = perpendicular[0].mul(&b(0xbf800000)?, p)?;
        perpendicular[1] = perpendicular[1].mul(&b(0xbf800000)?, p)?;
    }
    let radius = b(actor.turn_radius_bits)?;
    let offset_x = b(perpendicular[0].mul(&radius, p)?.store_f32()?)?;
    let offset_y = b(perpendicular[1].mul(&radius, p)?.store_f32()?)?;
    let x = b(state.movement.destination_bits[0])?
        .sub(&offset_x.add(&b(actor.position_bits[0])?, p)?, p)?;
    let y = b(state.movement.destination_bits[1])?
        .sub(&offset_y.add(&b(actor.position_bits[1])?, p)?, p)?;
    Ok(x.mul(&x, p)?
        .add(&y.mul(&y, p)?, p)?
        .compare(&radius.mul(&radius, p)?)
        == Ordering::Less)
}

/// Complete ResultPlan's non-diagnostic behavior. All earlier writes/callbacks
/// remain on error; callers must not automatically retry. Diagnostic string
/// logging and original allocator addresses do not form part of this contract.
pub fn result_plan<W: World>(
    state: &mut State,
    actor: &mut Actor,
    trigger: Option<Object>,
    argument: u32,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    state.previous_destination_bits = state.movement.destination_bits;
    if state.attempts >= 3 {
        return world.failed(state, actor);
    }
    state.attempts = state.attempts.wrapping_add(1);
    let mut force_plan = false;
    if state.movement.mode != 0 && near(state, actor, p)? {
        if state.movement.phase != 0 {
            let mut special = false;
            if let Some(object) = trigger {
                special = world.special_type(object, state, actor)?
                    && world.has_special_component(object);
            }
            let current_region = world.region_at(actor.position_bits, state, actor)?;
            let goal_region = world.region_at(state.movement.destination_bits, state, actor)?;
            if current_region != goal_region && !special {
                let mut context = world.collision_context(actor.object, state, actor)?;
                context.words[14] |= 1 << 29;
                let target = world
                    .resolve_actor(state.movement.target_id)
                    .map(|o| world.replacement_actor(o));
                context.words[8] = target.map_or(0, |o| o.0);
                context.words[9] = target.map_or(0, |o| world.object_definition(o));
                let hit =
                    world.sweep(&mut context, state.movement.destination_bits, state, actor)?;
                if hit.kind == 2 {
                    let object = world
                        .resolve_actor(hit.actor_id)
                        .ok_or(Error::InvalidCollisionActor)?;
                    if world.special_type(object, state, actor)? {
                        // Original resolves the hit again after virtual+b4.
                        let object = world
                            .resolve_actor(hit.actor_id)
                            .ok_or(Error::InvalidCollisionActor)?;
                        force_plan = world.has_special_component(object);
                    }
                }
            }
        }
        if !force_plan {
            return if state.movement.phase == 0 {
                world.continue_turn(false, state, actor)
            } else {
                world.continue_current(argument, state, actor)
            };
        }
    }
    if state.movement.phase == 0 && within_turn_circle(state, actor, p)? {
        return world.continue_turn(true, state, actor);
    }
    state.movement.phase = 1;
    let object = actor.object;
    let mut context = world.collision_context(object, state, actor)?;
    context.words[14] |= 1 << 29;
    context.words[8] = world
        .resolve_actor(state.movement.target_id)
        .map(|o| world.replacement_actor(o))
        .map_or(0, |o| o.0);
    let mut option = 0;
    if world.property(object, state, actor)? != 0 {
        let property = world.property(object, state, actor)?;
        if world.property_active(property, state, actor)? {
            option = 1;
        }
    }
    if world.solve(
        &mut context,
        crate::solver::Request {
            start_bits: actor.position_bits,
            goal_bits: state.movement.destination_bits,
            option_word: option,
        },
        state,
        actor,
    )? != 0
    {
        world.update_waypoint_offset(state, actor)?;
        world.action_go(0, state, actor)
    } else {
        world.failed(state, actor)
    }
}
