//! Original path ownership, movement-state operations and concrete tile routing.
//! Authoritative map/world binding and region topology remain required boundaries.
#![forbid(unsafe_code)]

mod action_world;
pub use action_world::TryActionWorld;
pub mod ai_startup;
pub mod control;
pub mod cost;
pub mod company_move;
pub mod endpoint;
pub mod formation_follow;
pub mod low_path;
pub mod materialize;
pub mod preparation;
pub mod region;
pub mod region_search;
pub mod result_plan;
pub mod sai_state;
pub mod search;
pub mod solver;
pub mod startup;
pub mod topology;
pub mod topology_builder;

/// Exact native node payload; links are owned by the collection, not the payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PathNode {
    pub words: [u32; 8],
}

use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedPath {
    pub nodes: Vec<PathNode>,
    /// Native collection +0x0c and +0x10; deliberately retained by clear_nodes.
    pub kind1_count: i32,
    pub kind2_count: i32,
    pub dirty: bool,
    /// Native +0x18, uninterpreted in the recovered operations.
    pub word18: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("path allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error("positive type-2 count outlives the owned node chain")]
    InvalidPathCount,
    #[error("the native path event requires a last node")]
    EmptyEventPath,
    #[error("a required prepared path node was removed by a callback")]
    MissingPathNode,
    #[error("ActionGo requires the actor's Mover component")]
    MissingMover,
    #[error("navigation world operation failed: {0}")]
    World(String),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

impl OwnedPath {
    /// 0x5189f0 on the embedded list. Metadata is intentionally unchanged.
    pub fn clear_nodes(&mut self) {
        self.nodes.clear();
    }
    /// 0x52d0f0/0x52d000: copy all words in source order, then metadata.
    /// Rust's exclusive borrow prevents native self-aliasing. On allocation
    /// failure the destination is retained instead of native null dereference.
    pub fn assign_from(&mut self, source: &Self) -> Result<()> {
        let mut nodes = Vec::new();
        nodes.try_reserve_exact(source.nodes.len())?;
        nodes.extend_from_slice(&source.nodes);
        self.nodes = nodes;
        self.kind1_count = source.kind1_count;
        self.kind2_count = source.kind2_count;
        self.dirty = true;
        self.word18 = source.word18;
        Ok(())
    }
    /// 0x52ce00 removes from the tail until the signed kind-2 count is <=0.
    /// It is not merely a `while last.kind == 2` filter. Errors are not rollback.
    pub fn prune_kind2_tail(&mut self) -> Result<()> {
        while self.kind2_count > 0 {
            let node = self.nodes.pop().ok_or(Error::InvalidPathCount)?;
            match node.words[4] {
                1 => self.kind1_count = self.kind1_count.wrapping_sub(1),
                2 => self.kind2_count = self.kind2_count.wrapping_sub(1),
                _ => {}
            }
            self.dirty = true;
        }
        Ok(())
    }
}

/// UAIStateMove fields consumed by the recovered routines. Scalar floats remain
/// raw binary32 bits, including signed zero. No constructor defaults are inferred.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveState {
    pub target_id: u32,             // +10
    pub destination_bits: [u32; 2], // +14
    pub auxiliary_bits: [u32; 2],   // +1c
    pub requested_speed_bits: u32,  // +24
    pub phase: u32,                 // +2c
    pub mode: u32,                  // +30
    pub waypoint_offset_bits: u32,  // +54
    pub path: OwnedPath,            // +58
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorState {
    /// Original actor+8 flags, not the property word at +100.
    pub flags: u32,
    pub position_bits: [u32; 2],
    pub mover_instant_turn: bool,
    pub mover_requested_speed_bits: u32,
    pub mover_speed_factor_bits: u32,
    pub mover_maximum_speed_bits: u32,
    pub has_mover: bool,
    /// actor+80 must identify a valid organization component for ActionGo.
    pub organization_special: bool, // component +1c
    pub has_leader: bool, // component +10
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activation {
    pub activation_mode: u32,
    pub destination_bits: [u32; 2],
    pub movement_mode: u32,
    pub requested_speed_bits: u32,
    pub auxiliary_bits: [u32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoverRequest {
    pub direction_bits: [u32; 2],
    pub destination_bits: [u32; 2],
    pub requested_speed_bits: u32,
    pub speed_factor_bits: u32,
}

/// Required native seams. No callback defaults invent collision, formation,
/// UAI state activation, actor eligibility or scheduler behavior. All mutable
/// state is reread at the corresponding native boundary after callbacks.
pub trait NavigationWorld {
    /// 0x52a000 for the separate UAI+48 state; ordinary move events only.
    fn special_state_active(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool;
    /// MoveState virtual +4, invoked by 0x529c00 for modes other than 0/1.
    fn can_activate(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool;
    /// 0x52a810 state activation; bool result is preserved, including false
    /// after the fields written by 0x52df60 have already changed.
    fn activate(&mut self, mode: u32, state: &mut MoveState, actor: &mut ActorState) -> bool;
    fn node_reached(&mut self, state: &mut MoveState, actor: &mut ActorState);
    fn single_node_placement(
        &mut self,
        waypoint: [u32; 2],
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> bool;
    fn actor_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool;
    fn formation_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool;
    /// 0x4ce6c0 selects actor or leader; 0x44dea0 generation-checks its +164
    /// handle. The semantic identity of that referenced object is not inferred.
    fn actor_or_leader_handle_valid(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> bool;
    fn organization_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool;
    fn leader_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool;
    /// Entire 0x52d660 stop/character fallback, not just a speed assignment.
    fn stop_action(&mut self, state: &mut MoveState, actor: &mut ActorState);
    /// Exact 0x5773c0 boundary; its return is intentionally ignored by ActionGo.
    fn begin_mover(
        &mut self,
        request: MoverRequest,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> bool;
    /// One wide-output draw from the actual GWorld RNG stream (0x4f02b0 →
    /// 0x45d400 → 0x85d840). This is NOT an already rounded f32 random value.
    fn next_world_random_u32(&mut self, state: &mut MoveState, actor: &mut ActorState) -> u32;
    /// UAI timer virtual +4, with native second argument zero.
    fn schedule(&mut self, delay_bits: u32, state: &mut MoveState, actor: &mut ActorState);
}

/// Complete 0x52df60 with diagnostic logging disabled; all simulation fields
/// and the activation result/order are retained. Diagnostic logging is separate.
pub fn activate_move<W: NavigationWorld>(
    state: &mut MoveState,
    actor: &mut ActorState,
    request: Activation,
    world: &mut W,
) -> bool {
    if request.activation_mode > 1 && !world.can_activate(state, actor) {
        return false;
    }
    state.target_id = 0;
    state.destination_bits = request.destination_bits;
    state.mode = request.movement_mode;
    state.requested_speed_bits = request.requested_speed_bits;
    state.auxiliary_bits = request.auxiliary_bits;
    state.phase = 0;
    world.activate(request.activation_mode, state, actor)
}

/// 0x532170 / 0x532310. The extra two native argument words are unused in
/// non-diagnostic behavior; they do not become invented path controls.
pub fn move_path_event<W: NavigationWorld>(
    state: &mut MoveState,
    actor: &mut ActorState,
    source: &OwnedPath,
    speed_bits: u32,
    formation: bool,
    world: &mut W,
) -> Result<bool> {
    if actor.flags & (1 | (1 << 29)) != 0 {
        return Ok(false);
    }
    if !formation && world.special_state_active(state, actor) {
        return Ok(false);
    }
    let last = source.nodes.last().ok_or(Error::EmptyEventPath)?;
    if !activate_move(
        state,
        actor,
        Activation {
            activation_mode: 2,
            destination_bits: [last.words[0], last.words[1]],
            movement_mode: if formation { 3 } else { 0 },
            requested_speed_bits: speed_bits,
            auxiliary_bits: [0, 0],
        },
        world,
    ) {
        return Ok(false);
    }
    state.phase = 1;
    state.path.assign_from(source)?;
    state.path.prune_kind2_tail()?;
    state.path.word18 = u32::MAX; // State+70 / UAI+ec aliases collection+18.
    Ok(true)
}

#[derive(Clone, Copy, Debug)]
pub struct ActionContext {
    pub precision: Precision,
    /// GroupSpeedMinimum, dereferenced SVar cache+190 (registration ID100).
    pub group_speed_minimum_bits: u32,
    /// MoverCatchupSpeed, SVar cache+48 / registration ID18, used by 0x577090.
    pub catchup_speed_factor_bits: u32,
}

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn first(state: &MoveState) -> Result<PathNode> {
    state
        .path
        .nodes
        .first()
        .copied()
        .ok_or(Error::MissingPathNode)
}

/// Entire 0x5300a0 prepared-node operation, preserving all control/arithmetic
/// and required native callbacks. This is not path production. Errors can occur
/// after effects; the caller must not assume rollback or automatically retry.
pub fn action_go<W: NavigationWorld>(
    state: &mut MoveState,
    actor: &mut ActorState,
    speed_bits: u32,
    context: ActionContext,
    world: &mut W,
) -> Result<bool> {
    try_action_go(
        state,
        actor,
        speed_bits,
        context,
        &mut action_world::Legacy(world),
    )
}

/// Fallible live-world entry to the same original5300A0 kernel. Successful
/// callbacks preserve native behavior. An error stops immediately, retaining
/// earlier effects and consuming no later random value or scheduler operation.
pub fn try_action_go<W: TryActionWorld>(
    state: &mut MoveState,
    actor: &mut ActorState,
    speed_bits: u32,
    context: ActionContext,
    world: &mut W,
) -> Result<bool> {
    let p = context.precision;
    let epsilon = b(0x38d1b717)?;
    if state.path.nodes.is_empty() {
        world.stop_action(state, actor)?;
        world.schedule(0x3f800000, state, actor)?;
        return Ok(false);
    }
    if !actor.has_mover {
        return Err(Error::MissingMover);
    }
    let node = first(state)?;
    actor.mover_instant_turn = node.words[4] == 0;
    let dx = b(node.words[0])?.sub(&b(actor.position_bits[0])?, p)?;
    let dy = b(node.words[1])?.sub(&b(actor.position_bits[1])?, p)?;
    let distance = dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?.sqrt(p)?;
    if distance.compare(&epsilon) != Ordering::Greater {
        world.node_reached(state, actor)?;
        return Ok(false);
    }
    let projection = dx
        .mul(&b(node.words[2])?, p)?
        .add(&dy.mul(&b(node.words[3])?, p)?, p)?;
    if projection.compare(&Binary::from_u32(0)) != Ordering::Greater {
        world.node_reached(state, actor)?;
        return Ok(false);
    }
    if !actor.mover_instant_turn
        && state.path.nodes.len() == 1
        && world.single_node_placement([node.words[0], node.words[1]], state, actor)?
    {
        actor.mover_instant_turn = true;
    }
    if state.phase == 2 {
        state.phase = 1;
    }
    let mut speed = speed_bits;
    if b(speed)?.compare(&epsilon) == Ordering::Less {
        speed = if world.actor_is_moving(state, actor)? {
            actor.mover_requested_speed_bits
        } else {
            state.requested_speed_bits
        };
    }
    let mut factor = 0x3f800000;
    if state.mode == 3 {
        if !world.formation_ready(state, actor)? {
            return Ok(false);
        }
        factor = actor.mover_speed_factor_bits;
    }
    if world.actor_or_leader_handle_valid(state, actor)?
        && b(context.group_speed_minimum_bits)?.compare(&b(state.requested_speed_bits)?)
            == Ordering::Greater
    {
        world.stop_action(state, actor)?;
        world.schedule(0x3f800000, state, actor)?;
        return Ok(false);
    }
    if !actor.organization_special
        && world.organization_ready(state, actor)?
        && actor.has_leader
        && world.leader_is_moving(state, actor)?
        && actor.has_mover
    {
        speed = b(context.catchup_speed_factor_bits)?
            .mul(&b(actor.mover_maximum_speed_bits)?, p)?
            .store_f32()?;
    }
    let node = first(state)?;
    let offset = b(state.waypoint_offset_bits)?;
    // Native X product is stored to f32 before subtraction; Y stays extended.
    let offset_x = b(node.words[2])?.mul(&offset, p)?.store_f32()?;
    let y = b(node.words[1])?
        .sub(&b(node.words[3])?.mul(&offset, p)?, p)?
        .store_f32()?;
    let x = b(node.words[0])?.sub(&b(offset_x)?, p)?.store_f32()?;
    if !actor.has_mover {
        return Err(Error::MissingMover);
    }
    let _ = world.begin_mover(
        MoverRequest {
            direction_bits: [node.words[2], node.words[3]],
            destination_bits: [x, y],
            requested_speed_bits: speed,
            speed_factor_bits: factor,
        },
        state,
        actor,
    )?;
    let random = world.next_world_random_u32(state, actor)?;
    let mut value = Binary::from_i32(random as i32);
    if random & 0x80000000 != 0 {
        value = value.add(&Binary::from_f64_bits(0x41f0000000000000)?, p)?;
    }
    let random_unit = value.mul(&Binary::from_f64_bits(0x3df0000000100000)?, p)?;
    let jitter = random_unit.mul(&b(0x3e99999a)?, p)?;
    let base = if first(state)?.words[4] == 0 {
        0x3e99999a
    } else {
        0x3f800000
    };
    let delay = b(base)?.add(&jitter, p)?.store_f32()?;
    world.schedule(delay, state, actor)?;
    Ok(true)
}
pub mod route_cache;
pub mod uai_collision;
