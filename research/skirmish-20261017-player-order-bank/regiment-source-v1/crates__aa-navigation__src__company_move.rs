//! Original regiment (company) movement: `KKC_CAIStateMove` (vtable 0xA33804,
//! embedded at Regiment Mind +0x68) and the formation path helpers it uses,
//! plus the Element return-to-slot move.
//!
//! - activation `0x520320` (move to actor) / `0x520440` (move to location),
//!   destination `0x51e9b0` with group offset `0x51d840` and clamp `0x5525d0`;
//! - Enter `0x523080` (vtable +0x18), Leave `0x51d7b0` (+0x1c), speed
//!   broadcast `0x5205e0`, move-try accounting `0x526940`;
//! - formation path geometry: slot path `0x52cbd0` with line intersection
//!   `0x445280`, translated path `0x52d230`, path reset `0x52cd90`;
//! - Element return to slot `0x52e9d0` / `0x52e900`.
//!
//! The Mind state framework (`0x529c00` eligibility, `0x52a810` activation),
//! group objects (actor +0x164), member Minds and timers are explicit world
//! callbacks, called in native order with the live state.
use crate::{ActorState, Activation, Error, MoveState, NavigationWorld, OwnedPath, PathNode, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
#[path="company_activation.rs"]
mod activation;
#[path="company_lifecycle.rs"]
mod lifecycle;
pub use lifecycle::{CompanySpeedFields,CompanyExitFields,CompanyLifecycleFields,
    set_speed_on_fields,enter_on_fields,leave_on_fields};
pub use activation::{CompanyActivationFields,LocationActivation,set_destination_on_fields,
    activate_to_actor_on_fields,activate_to_location_on_fields};

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn store(value: &Binary) -> Result<u32> {
    Ok(value.store_f32()?)
}
/// Native binary32 constants: 1e-4 (0xa3aa10 / 0xa329e4), 1.0.
const EPSILON: u32 = 0x38d1_b717;
const ONE: u32 = 0x3f80_0000;

/// `KKC_CAIStateMove` fields (state +0x0C..+0x54).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyMoveState {
    /// +0x0C state status (Enter writes 2, Leave 0).
    pub status: u32,
    /// +0x10 target GTID (0 = location move).
    pub target_id: u32,
    /// +0x14/+0x18 path goal: destination plus group offset, clamped.
    pub path_destination_bits: [u32; 2],
    /// +0x1C/+0x20 requested destination, clamped.
    pub destination_bits: [u32; 2],
    /// +0x24/+0x28 auxiliary vector (copied to Mind +0x2B4/+0x2B8 on Enter).
    pub auxiliary_bits: [u32; 2],
    /// +0x2C regiment speed passed to members.
    pub speed_bits: u32,
    /// +0x30 signed move tries.
    pub tries: i32,
    /// +0x34 owned path collection.
    pub path: OwnedPath,
    /// +0x50 formation word compared with organization +0x74.
    pub formation_word: u32,
    /// +0x54 world time recorded on Enter (binary32 bits).
    pub start_time_bits: u32,
}

/// Map extents (GMap +0x0C/+0x10) used by `0x5525d0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extents {
    pub extent_bits: [u32; 2],
}

/// Original `0x5525d0`: negative (ordered) components become +0.0; values
/// above `extent - 1e-4` (extended, not stored) become its binary32 store.
pub fn clamp_to_extent(extents: Extents, point: &mut [u32; 2], p: Precision) -> Result<()> {
    for (axis, value) in point.iter_mut().enumerate() {
        *value=clamp_coordinate(extents.extent_bits[axis],*value,p)?;
    }
    Ok(())
}
///One axis of the SAME5525D0 algorithm, for owners that must commit the
///first coordinate before the next operation without a copied move payload.
pub fn clamp_coordinate(extent:u32,value:u32,p:Precision)->Result<u32>{
    let limit=b(extent)?.sub(&b(EPSILON)?,p)?;
    let current=b(value)?;
    if current.compare(&b(0)?)==Ordering::Less{Ok(0)}
    else if limit.compare(&current)==Ordering::Less{store(&limit)}else{Ok(value)}
}

/// Original `0x52cd90` path reset: clear nodes, both counts, dirty set and
/// word +0x18 = -1.
pub fn reset_path(path: &mut OwnedPath) {
    path.clear_nodes();
    path.kind2_count = 0;
    path.kind1_count = 0;
    path.word18 = u32::MAX;
    path.dirty = true;
}

/// Original `0x445280`: intersection of line p1->p2 with line p3->p4. Returns
/// None when the extended denominator compares equal to zero.
pub fn line_intersection(
    p1: [u32; 2],
    p2: [u32; 2],
    p3: [u32; 2],
    p4: [u32; 2],
    p: Precision,
) -> Result<Option<[u32; 2]>> {
    let d1x = b(p2[0])?.sub(&b(p1[0])?, p)?;
    let d2x = b(p4[0])?.sub(&b(p3[0])?, p)?;
    let d2y = b(p4[1])?.sub(&b(p3[1])?, p)?;
    let d1y = b(p2[1])?.sub(&b(p1[1])?, p)?;
    let denominator = d2y.mul(&d1x, p)?.sub(&d1y.mul(&d2x, p)?, p)?;
    if denominator.is_zero() {
        return Ok(None);
    }
    let e = b(p1[1])?.sub(&b(p3[1])?, p)?;
    let g = b(p4[1])?
        .sub(&b(p3[1])?, p)?
        .mul(&b(p1[0])?.sub(&b(p3[0])?, p)?, p)?;
    let t = d2x.mul(&e, p)?.sub(&g, p)?.div(&denominator, p)?;
    let x = store(&d1x.mul(&t, p)?.add(&b(p1[0])?, p)?)?;
    let y = store(&b(p2[1])?.sub(&b(p1[1])?, p)?.mul(&t, p)?.add(&b(p1[1])?, p)?)?;
    Ok(Some([x, y]))
}

/// Original `0x52d230`: copy the source path (`0x52d0f0`), then add the offset
/// to every node waypoint with binary32 stores; dirty is set.
pub fn translate_path(target: &mut OwnedPath, source: &OwnedPath, offset: [u32; 2], p: Precision) -> Result<()> {
    target.assign_from(source)?;
    for node in &mut target.nodes {
        node.words[0] = store(&b(offset[0])?.add(&b(node.words[0])?, p)?)?;
        node.words[1] = store(&b(offset[1])?.add(&b(node.words[1])?, p)?)?;
    }
    target.dirty = true;
    Ok(())
}

/// Original `0x52cbd0`: offset an already copied regiment path for one slot.
/// Walking from the last node backwards, each waypoint becomes the source
/// waypoint plus the slot offset (rotated by owner facing) projected onto the
/// node direction; interior waypoints are then replaced by the intersection
/// of the new segment with the previous one when the lines are not parallel.
pub fn offset_path_for_slot(
    target: &mut OwnedPath,
    slot_offset: [u32; 2],
    facing: [u32; 2],
    source: &OwnedPath,
    p: Precision,
) -> Result<()> {
    let count = target.nodes.len();
    if count == 0 || source.nodes.len() < count {
        return Err(Error::World("slot path needs a copied, nonempty source path (native null read)".into()));
    }
    let [sx, sy] = slot_offset.map(b);
    let [fx, fy] = facing.map(b);
    let (sx, sy, fx, fy) = (sx?, sy?, fx?, fy?);
    // A = sx*fy - sy*fx and B = sy*fy + sx*fx (extended, recomputed per node).
    let across = |p: Precision| -> Result<(Binary, Binary)> {
        Ok((
            sx.mul(&fy, p)?.sub(&sy.mul(&fx, p)?, p)?,
            sy.mul(&fy, p)?.add(&sx.mul(&fx, p)?, p)?,
        ))
    };
    let mut target_index = count - 1;
    let mut source_index = source.nodes.len() - 1;
    let last = source.nodes[source_index];
    let mut previous_dx = last.words[2];
    let mut previous_dy = last.words[3];
    let (a, bb) = across(p)?;
    let px = store(&b(previous_dy)?.mul(&a, p)?.add(&bb.mul(&b(previous_dx)?, p)?, p)?)?;
    let qy = store(&bb.mul(&b(previous_dy)?, p)?.sub(&a.mul(&b(previous_dx)?, p)?, p)?)?;
    target.nodes[target_index].words[0] = store(&b(px)?.add(&b(last.words[0])?, p)?)?;
    target.nodes[target_index].words[1] = store(&b(qy)?.add(&b(last.words[1])?, p)?)?;
    for _ in 1..count {
        let previous_point = [target.nodes[target_index].words[0], target.nodes[target_index].words[1]];
        target_index -= 1;
        source_index -= 1;
        let node: PathNode = target.nodes[target_index];
        let (dx, dy) = (node.words[2], node.words[3]);
        let (a, bb) = across(p)?;
        let px = store(&b(dy)?.mul(&a, p)?.add(&bb.mul(&b(dx)?, p)?, p)?)?;
        let qy = bb.mul(&b(dy)?, p)?.sub(&a.mul(&b(dx)?, p)?, p)?;
        let source_node = source.nodes[source_index];
        let nx = store(&b(px)?.add(&b(source_node.words[0])?, p)?)?;
        let ny = store(&qy.add(&b(source_node.words[1])?, p)?)?;
        target.nodes[target_index].words[0] = nx;
        target.nodes[target_index].words[1] = ny;
        let r1 = [
            store(&b(previous_point[0])?.add(&b(previous_dx)?, p)?)?,
            store(&b(previous_point[1])?.add(&b(previous_dy)?, p)?)?,
        ];
        let r2 = [store(&b(nx)?.add(&b(dx)?, p)?)?, store(&b(ny)?.add(&b(dy)?, p)?)?];
        if let Some(point) = line_intersection([nx, ny], r2, previous_point, r1, p)? {
            target.nodes[target_index].words[0] = point[0];
            target.nodes[target_index].words[1] = point[1];
        }
        previous_dx = dx;
        previous_dy = dy;
    }
    target.dirty = true;
    Ok(())
}

/// Regiment move services, in native call order.
pub trait CompanyWorld {
    /// `0x529c00`: Mind-state eligibility; native consults state virtual +4
    /// (`0x51d730`) only for activation modes other than 0/1.
    fn can_activate(&mut self, mode: u32, state: &mut CompanyMoveState) -> Result<bool>;
    /// `0x52a810` Mind state activation; its byte result is returned by 0x520320.
    fn activate(&mut self, mode: u32, state: &mut CompanyMoveState) -> Result<bool>;
    /// `0x51d840` group offset: actor +0x164 group handle valid, `0x4ddee0`
    /// true, then `0x4de6c0` offset. None when any test fails.
    fn group_offset(&mut self, state: &mut CompanyMoveState) -> Result<Option<[u32; 2]>>;
    /// Live GTID lookup: target actor position (+0x20/+0x24) when valid.
    fn actor_position(&mut self, id: u32) -> Result<Option<[u32; 2]>>;
    /// Mind +0x2B4/+0x2B8 auxiliary vector write (Enter).
    fn set_mind_auxiliary(&mut self, auxiliary: [u32; 2]) -> Result<()>;
    /// Organization component +0xD8 (regiment speed cap).
    fn organization_speed_cap(&mut self) -> Result<u32>;
    /// Organization members in slot order (`0x4364c0`, all members): count of
    /// nonnull member slots, visited by index.
    fn member_slots(&mut self) -> Result<Vec<usize>>;
    /// Member Mind virtual +0x64 and +0x88(speed).
    fn member_accepts_speed(&mut self, slot: usize) -> Result<bool>;
    fn member_set_speed(&mut self, slot: usize, speed_bits: u32) -> Result<()>;
    /// GWorld +0xE0 world time.
    fn world_time(&mut self) -> Result<u32>;
    /// Mind Updatable (+8) virtual +4 (delay, 0).
    fn schedule(&mut self, delay_bits: u32) -> Result<()>;
    /// Mind virtual +0x124 (move failure).
    fn move_failed(&mut self) -> Result<()>;
}

/// Original `0x51e9b0`: store and clamp the destination, then derive and clamp
/// the path destination from the unclamped arguments plus the group offset.
pub fn set_destination<W: CompanyWorld>(
    state: &mut CompanyMoveState,
    destination: [u32; 2],
    extents: Extents,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    set_destination_on_fields(&mut activation::Legacy{state,world,target:None},destination,extents,p)
}

/// Original `0x520320`: move to an actor (`None` target returns false first).
#[allow(clippy::too_many_arguments)]
pub fn activate_to_actor<W: CompanyWorld>(
    state: &mut CompanyMoveState,
    mode: u32,
    target: Option<(u32, [u32; 2])>,
    speed_bits: u32,
    auxiliary_bits: [u32; 2],
    extents: Extents,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    activate_to_actor_on_fields(&mut activation::Legacy{state,world,target},mode,target.map(|t|t.0),speed_bits,auxiliary_bits,extents,p)
}

///Historical location helper returns whether eligibility passed and activation
///was requested.527E30 consumes the exact native AL via the result API below.
#[allow(clippy::too_many_arguments)]
pub fn activate_to_location<W: CompanyWorld>(
    state: &mut CompanyMoveState,
    mode: u32,
    destination: [u32; 2],
    speed_bits: u32,
    auxiliary_bits: [u32; 2],
    extents: Extents,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    Ok(activate_to_location_on_fields(&mut activation::Legacy{state,world,target:None},mode,destination,speed_bits,auxiliary_bits,extents,p)?.requested)
}
///Actual520440 AL consumed by527E30. Existing activate_to_location retains
///its documented request-admission result; both use one coordinator above.
#[allow(clippy::too_many_arguments)]
pub fn activate_to_location_result<W:CompanyWorld>(state:&mut CompanyMoveState,mode:u32,
    destination:[u32;2],speed_bits:u32,auxiliary_bits:[u32;2],extents:Extents,p:Precision,world:&mut W)->Result<bool>{
    Ok(activate_to_location_on_fields(&mut activation::Legacy{state,world,target:None},mode,destination,speed_bits,auxiliary_bits,extents,p)?.result)
}

/// Original `0x5205e0`: cap the speed at organization +0xD8, replace values
/// below 1e-4 with the cap, store, then offer it to every member Mind.
pub fn set_speed<W: CompanyWorld>(state: &mut CompanyMoveState, speed_bits: u32, world: &mut W) -> Result<()> {
    set_speed_on_fields(&mut activation::Legacy{state,world,target:None},speed_bits)
}

/// Original Enter `0x523080` (state vtable +0x18).
pub fn enter<W: CompanyWorld>(
    state: &mut CompanyMoveState,
    extents: Extents,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    enter_on_fields(&mut activation::Legacy{state,world,target:None},extents,p)
}

/// Original Leave `0x51d7b0` (state vtable +0x1c).
pub fn leave(state: &mut CompanyMoveState) {
    leave_on_fields(&mut lifecycle::StateOnly(state)).expect("plain lab fields are infallible");
}

/// Original `0x526940`: count a move try; after the third, fail once the
/// `CAIMoveMinSecondsBeforeFail` SVar plus the Enter time is not ordered
/// above the world time (schedule 1.0 s and Mind +0x124). Returns the AL.
pub fn increment_tries<W: CompanyWorld>(
    state: &mut CompanyMoveState,
    min_seconds_before_fail_bits: u32,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    state.tries = state.tries.wrapping_add(1);
    if state.tries > 2 {
        let now = world.world_time()?;
        let deadline = b(min_seconds_before_fail_bits)?.add(&b(state.start_time_bits)?, p)?;
        if deadline.compare(&b(now)?) != Ordering::Greater {
            world.schedule(ONE)?;
            world.move_failed()?;
            return Ok(false);
        }
    }
    Ok(true)
}

/// Element return-to-slot boundaries (`0x52e9d0` / `0x52e900`).
pub trait SlotReturnWorld: NavigationWorld {
    /// `0x529c20` on the Element MoveState: virtual +0x1c (Leave) when its
    /// status (+0x0C, UAI +0x88) is nonzero.
    fn leave_move_state(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()>;
    /// MoveState virtual +0x0C (Think), the tail call of `0x52e9d0`.
    fn move_state_think(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()>;
    /// Mind virtual +0x180, the no-organization tail call of `0x52e9d0`.
    fn mind_virtual_180(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()>;
}

/// The Element's organization slot (Element +0x18) and organization center
/// (component +0xBC/+0xC0) plus slot offset (+0x78[slot]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotTarget {
    pub center_bits: [u32; 2],
    pub slot_offset_bits: [u32; 2],
}

fn slot_activation(goal: [u32; 2]) -> Activation {
    Activation {
        activation_mode: 0,
        destination_bits: goal,
        requested_speed_bits: 0,
        movement_mode: 0,
        auxiliary_bits: [0, 0],
    }
}

/// Original `0x52e900`: start an ordinary move to `goal` unless the MoveState
/// status (UAI +0x88) is already 2; then, if its destination differs from
/// `goal` (ordered equality) or its mode is nonzero, leave and restart.
pub fn move_to_slot<W: SlotReturnWorld>(
    state: &mut MoveState,
    actor: &mut ActorState,
    move_status: u32,
    goal: [u32; 2],
    world: &mut W,
) -> Result<()> {
    if move_status != 2 {
        crate::activate_move(state, actor, slot_activation(goal), world);
    }
    let same = f32::from_bits(state.destination_bits[0]) == f32::from_bits(goal[0])
        && f32::from_bits(state.destination_bits[1]) == f32::from_bits(goal[1]);
    if !same || state.mode != 0 {
        world.leave_move_state(state, actor)?;
        crate::activate_move(state, actor, slot_activation(goal), world);
    }
    Ok(())
}

/// Original `0x52e9d0`: with an organization, move to center + slot offset
/// (binary32 adds) and run MoveState Think; otherwise Mind virtual +0x180.
pub fn return_to_slot<W: SlotReturnWorld>(
    state: &mut MoveState,
    actor: &mut ActorState,
    move_status: u32,
    target: Option<SlotTarget>,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    let Some(target) = target else {
        return world.mind_virtual_180(state, actor);
    };
    let goal = [
        store(&b(target.center_bits[0])?.add(&b(target.slot_offset_bits[0])?, p)?)?,
        store(&b(target.center_bits[1])?.add(&b(target.slot_offset_bits[1])?, p)?)?,
    ];
    move_to_slot(state, actor, move_status, goal, world)?;
    world.move_state_think(state, actor)
}

/// Native binary32 0.25 (0xa033bc), 0.14645929 (0xa339a4), 0.5, 5.0 (0xa02b30).
const ARRIVAL_RADIUS_SQUARED: u32 = 0x3e80_0000;
const REPATH_RATIO: u32 = 0x3e15_f96d;
const HALF: u32 = 0x3f00_0000;
const SCATTER_RADIUS: u32 = 0x40a0_0000;

/// Original `0x52a1f0` (ecx = Mind, actor +0x20/+0x24): X difference stored to
/// binary32, Y extended; `dy*dy + dx*dx < 0.25` (strict, ordered).
pub fn arrived(goal: [u32; 2], actor_position: [u32; 2], p: Precision) -> Result<bool> {
    let dx = store(&b(goal[0])?.sub(&b(actor_position[0])?, p)?)?;
    let dy = b(goal[1])?.sub(&b(actor_position[1])?, p)?;
    let sum = dy.mul(&dy, p)?.add(&b(dx)?.mul(&b(dx)?, p)?, p)?;
    Ok(sum.compare(&b(ARRIVAL_RADIUS_SQUARED)?) == Ordering::Less)
}

/// Regiment member as read through the organization slot array (+0x28) and
/// its Element (+0x80).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    /// Index in the organization member array (iterator `0x4364c0` position).
    pub index: usize,
    /// Organization leader (+0x40) is this member.
    pub is_leader: bool,
    /// Element +0x1C.
    pub attached: bool,
    /// Element +0x18 slot.
    pub slot: i32,
    /// Actor +0x20/+0x24.
    pub position_bits: [u32; 2],
    /// Element exists and its +0x10 organization is this regiment's actor
    /// (the `0x523130` precondition).
    pub in_this_organization: bool,
}

/// Current formation (organization +0x74): none, oriented (+0xC -> +0x18),
/// scatter (+0x19) or a formation with neither byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormationKind {
    None,
    Oriented,
    Scatter,
    Plain,
}

/// The `0x53e1f0` request made by `0x51ea20`: the leader's `0x554350`
/// actor query with query +0x20 = target (ignored actor), +0x28 = Mind
/// virtual +0x98 result and required flags |= 0x20000000.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolveRequest {
    pub from_bits: [u32; 2],
    pub to_bits: [u32; 2],
    pub ignore_target: Option<u32>,
    pub query_word28: u32,
    /// Native Solve last argument: 2 when the Mind +0x220 state is current,
    /// 1 when the owner test `0x76b9a0` passes, else 0.
    pub mode: u32,
}

/// Target as seen by `0x5276b0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub position_bits: [u32; 2],
    /// The target is the regiment's own actor.
    pub is_self: bool,
}

/// Further services of the regiment move Think, in native order.
pub trait CompanyThinkWorld: CompanyWorld {
    /// `0x529e60` Think base (diagnostics/status).
    fn think_base(&mut self, state: &mut CompanyMoveState) -> Result<()>;
    /// `0x52a000` on this state; Mind virtual +0xB8 when current.
    fn is_current(&mut self, state: &mut CompanyMoveState) -> Result<bool>;
    fn mind_virtual_b8(&mut self) -> Result<()>;
    /// GTID lookup of the target (None when stale).
    fn target(&mut self, id: u32) -> Result<Option<Target>>;
    /// Mind virtual +0x140(target id), +0x144 (finished), +0xF4(target).
    fn target_lost(&mut self, id: u32) -> Result<()>;
    fn finished(&mut self) -> Result<()>;
    fn mind_virtual_f4(&mut self, id: u32) -> Result<()>;
    /// `0x51d7f0`: actor +0x164 group valid and `0x4ddee0` true.
    fn group_formation_active(&mut self) -> Result<bool>;
    /// `0x51fbc0` target in range (zone of control).
    fn target_in_range(&mut self, id: u32) -> Result<bool>;
    /// Target virtual +0x108 result, target +0xD0 nonnull, own actor virtual
    /// +0x138(result): true when the regiment should engage instead.
    fn engage_target(&mut self, id: u32) -> Result<bool>;
    /// Regiment actor +0x20/+0x24.
    fn own_position(&mut self) -> Result<[u32; 2]>;
    /// `0x52cae0` on the state path with the actor position.
    fn path_region_ok(&mut self, state: &mut CompanyMoveState, position: [u32; 2]) -> Result<bool>;
    /// Organization +0x40 leader: None when null, else `0x4cede0` moving.
    fn leader_moving(&mut self) -> Result<Option<bool>>;
    /// Organization +0x74 formation identity (0 = none).
    fn formation_word(&mut self) -> Result<u32>;
    /// Pop the front of Mind +0x2BC (pending member GTIDs): None when empty;
    /// Some(None) when the popped id is stale or the actor has flag bit0;
    /// Some(Some(member)) otherwise.
    fn pop_pending_member(&mut self) -> Result<Option<Option<Member>>>;
    /// `0x44dea0` on the actor's +0x164 group handle.
    fn group_handle_valid(&mut self) -> Result<bool>;
    /// `0x554350(leader, 0)`: the host prepares the leader's ordinary actor
    /// query (used by `solve`); a null leader is a native null read.
    fn prepare_leader_query(&mut self) -> Result<()>;
    /// Organization +0x40 leader position (+0x20/+0x24).
    fn leader_position(&mut self) -> Result<[u32; 2]>;
    /// Mind virtual +0x98.
    fn mind_virtual_98(&mut self) -> Result<u32>;
    /// `0x52a000` on the Mind +0x220 state.
    fn state_220_current(&mut self) -> Result<bool>;
    /// Actor virtual +0x108 nonnull (called twice) and `0x76b9a0` on it.
    fn owner_flag_76b9a0(&mut self) -> Result<bool>;
    /// `0x53e1f0` on the state path (see `crate::solver::solve`).
    fn solve(&mut self, path: &mut OwnedPath, request: SolveRequest) -> Result<bool>;
    /// `0x579a90`: owner +0x100 bit22 clears actor state 0x16.
    fn clear_owner_state_16(&mut self) -> Result<()>;
    /// `0x4eebe0` on the organization owner, then `0x57bef0`.
    fn set_organization_facing(&mut self, facing_bits: [u32; 2]) -> Result<()>;
    fn compute_slot_positions(&mut self) -> Result<()>;
    fn formation_kind(&mut self) -> Result<FormationKind>;
    /// Iterator `0x4364c0` (all members): next nonnull member after `after`.
    fn next_member(&mut self, after: Option<usize>) -> Result<Option<Member>>;
    /// Organization +0x78[slot] and regiment actor facing (+0x2C/+0x30).
    fn slot_offset(&mut self, slot: i32) -> Result<[u32; 2]>;
    fn own_facing(&mut self) -> Result<[u32; 2]>;
    /// Member Mind +0x100 (move path) and +0x10C (formation path) with the
    /// state speed and auxiliary vector.
    fn member_move_path(&mut self, member: Member, path: &OwnedPath, speed_bits: u32, auxiliary_bits: [u32; 2])
    -> Result<()>;
    fn member_formation_path(
        &mut self,
        member: Member,
        path: &OwnedPath,
        speed_bits: u32,
        auxiliary_bits: [u32; 2],
    ) -> Result<()>;
    /// Detached member move order (`0x4ca760`, `0x4d9b60`, `0x4ca2d0`).
    fn member_order(&mut self, member: Member, point_bits: [u32; 2], speed_bits: u32, auxiliary_bits: [u32; 2])
    -> Result<()>;
    /// `0x554ad0(point, direction, definition, 3, 0)`: free placement search
    /// updating `point` in place.
    fn place_near(&mut self, member: Member, point: &mut [u32; 2], direction_bits: [u32; 2]) -> Result<()>;
    /// One `0x85d840` draw of the world RNG (GWorld +0xD4).
    fn next_random(&mut self) -> Result<u32>;
}

fn unit_draw(draw: u32, p: Precision) -> Result<Binary> {
    let mut value = Binary::from_i32(draw as i32);
    if draw & 0x8000_0000 != 0 {
        value = value.add(&Binary::from_f64_bits(0x41f0_0000_0000_0000)?, p)?;
    }
    Ok(value.mul(&Binary::from_f64_bits(0x3df0_0000_0010_0000)?, p)?)
}

/// Original `0x409230` vector2 normalize (stores components; zero below 1e-4).
fn normalize2(v: &mut [u32; 2], p: Precision) -> Result<()> {
    let x = b(v[0])?;
    let y = b(v[1])?;
    let length = x.mul(&x, p)?.add(&y.mul(&y, p)?, p)?.sqrt(p)?;
    if length.compare(&b(EPSILON)?) != Ordering::Greater {
        *v = [0, 0];
        return Ok(());
    }
    let inverse = b(ONE)?.div(&length, p)?;
    v[0] = store(&inverse.mul(&x, p)?)?;
    v[1] = store(&inverse.mul(&y, p)?)?;
    Ok(())
}

fn empty_path() -> OwnedPath {
    OwnedPath { nodes: Vec::new(), kind1_count: 0, kind2_count: 0, dirty: true, word18: u32::MAX }
}

/// Original `0x523130`: send a slot-offset copy of the regiment path to a
/// member of this organization (Mind +0x10C). `shared` is the caller's copy
/// buffer (`0x525740`); without it a local copy is made.
pub fn send_member_path<W: CompanyThinkWorld>(
    state: &mut CompanyMoveState,
    member: Member,
    shared: Option<&mut OwnedPath>,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    if !member.in_this_organization {
        return Ok(());
    }
    let mut local = empty_path();
    let target = match shared {
        Some(path) => path,
        None => {
            local.assign_from(&state.path)?;
            &mut local
        }
    };
    let slot = world.slot_offset(member.slot)?;
    let facing = world.own_facing()?;
    offset_path_for_slot(target, slot, facing, &state.path, p)?;
    world.member_formation_path(member, target, state.speed_bits, state.auxiliary_bits)
}

fn detached_order<W: CompanyThinkWorld>(
    state: &mut CompanyMoveState,
    member: Member,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    let slot = world.slot_offset(member.slot)?;
    let first = state
        .path
        .nodes
        .first()
        .ok_or_else(|| Error::World("detached member order reads the first path node (native null read)".into()))?;
    let point = [
        store(&b(slot[0])?.add(&b(first.words[0])?, p)?)?,
        store(&b(slot[1])?.add(&b(first.words[1])?, p)?)?,
    ];
    world.member_order(member, point, state.speed_bits, state.auxiliary_bits)
}

/// Original `0x523240` (formation that is neither oriented nor scatter).
pub fn send_plain_member_path<W: CompanyThinkWorld>(
    state: &mut CompanyMoveState,
    member: Member,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    if member.is_leader {
        let path = state.path.clone();
        return world.member_move_path(member, &path, state.speed_bits, state.auxiliary_bits);
    }
    if !member.attached {
        return detached_order(state, member, p, world);
    }
    let slot = world.slot_offset(member.slot)?;
    let mut local = empty_path();
    translate_path(&mut local, &state.path, slot, p)?;
    world.member_move_path(member, &local, state.speed_bits, state.auxiliary_bits)
}

/// Original `0x5233f0` (scatter formation): attached members get the
/// slot-translated path with a new first node near their own position.
pub fn send_scatter_member_path<W: CompanyThinkWorld>(
    state: &mut CompanyMoveState,
    member: Member,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    if member.is_leader {
        let path = state.path.clone();
        return world.member_move_path(member, &path, state.speed_bits, state.auxiliary_bits);
    }
    if !member.attached {
        return detached_order(state, member, p, world);
    }
    let doubled = |draw: u32| -> Result<u32> {
        let centered = unit_draw(draw, p)?.sub(&b(HALF)?, p)?;
        store(&centered.add(&centered, p)?)
    };
    let first_draw = doubled(world.next_random()?)?;
    let second_draw = doubled(world.next_random()?)?;
    let mut direction = [second_draw, first_draw];
    normalize2(&mut direction, p)?;
    let radius = unit_draw(world.next_random()?, p)?.mul(&b(SCATTER_RADIUS)?, p)?;
    let scaled = [store(&b(direction[0])?.mul(&radius, p)?)?, store(&b(direction[1])?.mul(&radius, p)?)?];
    let slot = world.slot_offset(member.slot)?;
    let mut local = empty_path();
    translate_path(&mut local, &state.path, slot, p)?;
    let mut point = [
        store(&b(scaled[0])?.add(&b(member.position_bits[0])?, p)?)?,
        store(&b(scaled[1])?.add(&b(member.position_bits[1])?, p)?)?,
    ];
    world.place_near(member, &mut point, direction)?;
    let node = PathNode { words: [point[0], point[1], direction[0], direction[1], 1, 0, point[0], point[1]] };
    local.nodes.insert(0, node);
    local.kind1_count = local.kind1_count.wrapping_add(1);
    local.dirty = true;
    world.member_move_path(member, &local, state.speed_bits, state.auxiliary_bits)
}

/// Original `0x525740`: face the organization along the first path node,
/// recompute slots, send member paths by formation kind, remember the
/// formation and schedule 1.0 s.
pub fn dispatch_member_paths<W: CompanyThinkWorld>(
    state: &mut CompanyMoveState,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    world.clear_owner_state_16()?;
    let first = *state
        .path
        .nodes
        .first()
        .ok_or_else(|| Error::World("member dispatch reads the first path node (native null read)".into()))?;
    world.set_organization_facing([first.words[2], first.words[3]])?;
    world.compute_slot_positions()?;
    match world.formation_kind()? {
        FormationKind::Oriented => {
            let mut cursor = world.next_member(None)?;
            let mut shared = empty_path();
            shared.assign_from(&state.path)?;
            while let Some(member) = cursor {
                if member.is_leader {
                    let path = state.path.clone();
                    world.member_move_path(member, &path, state.speed_bits, state.auxiliary_bits)?;
                } else {
                    send_member_path(state, member, Some(&mut shared), p, world)?;
                }
                cursor = world.next_member(Some(member.index))?;
            }
        }
        FormationKind::Scatter => {
            let mut cursor = world.next_member(None)?;
            while let Some(member) = cursor {
                send_scatter_member_path(state, member, p, world)?;
                cursor = world.next_member(Some(member.index))?;
            }
        }
        FormationKind::None | FormationKind::Plain => {
            let mut cursor = world.next_member(None)?;
            while let Some(member) = cursor {
                send_plain_member_path(state, member, p, world)?;
                cursor = world.next_member(Some(member.index))?;
            }
        }
    }
    state.formation_word = world.formation_word()?;
    world.schedule(ONE)
}

/// Original `0x51ea20`: reset the path, plan from the organization leader to
/// the (recomputed) path destination, and schedule 1e-4 s on success.
pub fn plan_path<W: CompanyThinkWorld>(
    state: &mut CompanyMoveState,
    target: Option<u32>,
    extents: Extents,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    reset_path(&mut state.path);
    world.prepare_leader_query()?;
    let query_word28 = world.mind_virtual_98()?;
    let destination = state.destination_bits;
    set_destination(state, destination, extents, p, world)?;
    let mode = if world.state_220_current()? {
        2
    } else if world.owner_flag_76b9a0()? {
        1
    } else {
        0
    };
    let from = world.leader_position()?;
    let request =
        SolveRequest { from_bits: from, to_bits: state.path_destination_bits, ignore_target: target, query_word28, mode };
    let mut path = std::mem::replace(&mut state.path, empty_path());
    let solved = world.solve(&mut path, request);
    state.path = path;
    if !solved? {
        return Ok(false);
    }
    world.schedule(EPSILON)?;
    Ok(true)
}

/// Settings read by `0x5276b0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkSettings {
    /// SVar cache +0x190 `GroupSpeedMinimum`.
    pub group_speed_minimum_bits: u32,
    /// Lazily resolved SVar `CAIMoveMinSecondsBeforeFail`.
    pub min_seconds_before_fail_bits: u32,
    pub extents: Extents,
}

/// Complete regiment move Think `0x5276b0` (state vtable +0x0C).
pub fn think<W: CompanyThinkWorld>(
    state: &mut CompanyMoveState,
    settings: ThinkSettings,
    p: Precision,
    world: &mut W,
) -> Result<()> {
    world.think_base(state)?;
    if world.is_current(state)? {
        world.mind_virtual_b8()?;
    }
    let mut target = None;
    if state.target_id != 0 {
        let id = state.target_id;
        let Some(found) = world.target(id)? else {
            return world.target_lost(id);
        };
        if found.is_self {
            return world.finished();
        }
        if !world.group_formation_active()? && world.target_in_range(id)? {
            if world.engage_target(id)? {
                return world.mind_virtual_f4(id);
            }
            return world.finished();
        }
        target = Some(id);
        let mut goal = found.position_bits;
        if let Some(offset) = world.group_offset(state)? {
            goal = [
                store(&b(found.position_bits[0])?.add(&b(offset[0])?, p)?)?,
                store(&b(found.position_bits[1])?.add(&b(offset[1])?, p)?)?,
            ];
        }
        let own = world.own_position()?;
        let a = b(state.path_destination_bits[1])?.sub(&b(goal[1])?, p)?;
        let bx = b(state.path_destination_bits[0])?.sub(&b(goal[0])?, p)?;
        let c = b(own[1])?.sub(&b(goal[1])?, p)?;
        let d = b(own[0])?.sub(&b(goal[0])?, p)?;
        let moved = bx.mul(&bx, p)?.add(&a.mul(&a, p)?, p)?;
        let reach = d.mul(&d, p)?.add(&c.mul(&c, p)?, p)?.mul(&b(REPATH_RATIO)?, p)?;
        if reach.compare(&moved) != Ordering::Greater {
            reset_path(&mut state.path);
            state.tries = 0;
            set_destination(state, found.position_bits, settings.extents, p, world)?;
        }
    }
    let mut replan = state.path.nodes.is_empty();
    if !replan {
        let own = world.own_position()?;
        if world.path_region_ok(state, own)? {
            state.tries = 0;
            replan = true;
        }
    }
    let dispatch = if replan {
        if arrived(state.path_destination_bits, world.own_position()?, p)? {
            return world.finished();
        }
        if state.target_id == 0 && !increment_tries(state, settings.min_seconds_before_fail_bits, p, world)? {
            return Ok(());
        }
        if !plan_path(state, target, settings.extents, p, world)? {
            world.schedule(ONE)?;
            return world.move_failed();
        }
        !state.path.nodes.is_empty()
    } else if world.leader_moving()? == Some(true) {
        if state.formation_word == world.formation_word()? {
            while let Some(popped) = world.pop_pending_member()? {
                if let Some(member) = popped {
                    send_member_path(state, member, None, p, world)?;
                }
            }
            return world.schedule(ONE);
        }
        true
    } else {
        if arrived(state.path_destination_bits, world.own_position()?, p)? {
            return world.finished();
        }
        if world.group_handle_valid()?
            && b(state.speed_bits)?.compare(&b(settings.group_speed_minimum_bits)?) == Ordering::Less
        {
            state.tries = 0;
            return world.schedule(ONE);
        }
        increment_tries(state, settings.min_seconds_before_fail_bits, p, world)?
    };
    if dispatch {
        dispatch_member_paths(state, p, world)?;
    }
    Ok(())
}
