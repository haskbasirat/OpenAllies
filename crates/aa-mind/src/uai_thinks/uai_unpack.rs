//! Original `KKC_UAIStateUnpack::Think` (`5372B0`, element state
//! `uai_unpack` at Mind+188, vtable A347E0 slot +C) with its inline helper
//! `52EBF0` (unpack position).
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]). An active move state (Mind+7C)
//!    Thinks instead (done).
//! 2. Without an organization (actor+80 -> +10): `575C80(&state+18)` on the
//!    actor pack component (+AC); when it fails, `tai_error_unpack|s`
//!    (`[A338F0]`) formatted with the actor name (actor virtual +24) is
//!    posted to the actor owner (actor virtual +108) with `40C230`.
//! 3. The organization leader (component +44) itself: `575C80(&state+18)`.
//! 4. Otherwise the point `L` = state+10/+14 is offered to `52EC20(actor, &L,
//!    &state+18)` (placement, not ported; it may move `L`). Failure ends.
//!    `L` equal to state+10/+14 (`4515F0`, unordered unequal) runs
//!    `575C80(&state+18)`. Else `52EBF0(&L)`: state+10/+14 = definition
//!    +538/+53C + `L` (x87 at the control-word precision, stored binary32),
//!    then an active move state takes that point as its destination (Mind+90/
//!    +94) or the move state is ordered there (`52DF60(0, point, 1, 0,
//!    [B279C0], [B279C4])`, movement-owned); the move state Thinks (done).
//! 5. Steps 2, 3 and the failure/equal exits of 4 end with Mind virtual
//!    +1B0 (no argument).
use super::{ThinkWorld, element, null, status_at};
use crate::{
    Arena, MindId, Result, StateId, think_base,
    x87::{n, num, store},
};
use aa_navigation::Activation;
use aa_numeric::Precision;

/// `40C230` message kind of the unpack error.
pub const ERROR_MESSAGE_KIND: u32 = 0x11;
/// Wide literal `A338F0`.
pub const UNPACK_ERROR_KEY: &str = "tai_error_unpack|s";

/// Services of `5372B0` in native order.
pub trait UnpackThinkWorld: ThinkWorld {
    /// Actor +80 -> +10 (organization), `None` when either is null.
    fn organization(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// Organization component (+7C) +44 (leader) is the actor itself.
    fn actor_is_leader(&mut self, mind: MindId, organization: u32) -> Result<bool>;
    /// Actor +AC (pack component), `None` when null.
    fn pack_component(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// `575C80(&point)` on the pack component.
    fn unpack_at(&mut self, component: u32, point: [u32; 2], arena: &mut Arena) -> Result<bool>;
    /// Unpack state +10/+14 (position) with its write, and +18/+1C (facing).
    fn unpack_position(&mut self, state: StateId) -> Result<[u32; 2]>;
    fn set_unpack_position(&mut self, state: StateId, point: [u32; 2]) -> Result<()>;
    fn unpack_facing(&mut self, state: StateId) -> Result<[u32; 2]>;
    /// `52EC20(actor, &point, &state+18)` on the unpack state (placement,
    /// not ported): `None` when it fails, else the (possibly moved) point.
    fn place(
        &mut self,
        state: StateId,
        point: [u32; 2],
        facing: [u32; 2],
        arena: &mut Arena,
    ) -> Result<Option<[u32; 2]>>;
    /// `4515F0`: binary32 points equal (unordered is unequal).
    fn points_equal(&mut self, a: [u32; 2], b: [u32; 2]) -> Result<bool>;
    /// Actor definition +538/+53C (binary32 offset).
    fn unpack_offset(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// Move state +14/+18 (Mind+90/+94) destination write.
    fn set_move_destination(&mut self, mind: MindId, point: [u32; 2]) -> Result<()>;
    /// `52DF60` on the move state (movement-owned); result unused here.
    fn move_activate(
        &mut self,
        mind: MindId,
        request: Activation,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Mind virtual +1B0 (no argument).
    fn mind_virtual_1b0(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Actor virtual +24 (its name), `4B9F40` display string for a key,
    /// actor virtual +108 (owner) and `40C230(B399B0, kind, actor, owner, 0,
    /// format, argument)`.
    fn actor_name(&mut self, mind: MindId, arena: &mut Arena) -> Result<String>;
    fn display_string(&mut self, key: &str, arena: &mut Arena) -> Result<String>;
    fn actor_owner(&mut self, mind: MindId, arena: &mut Arena) -> Result<u32>;
    fn post_message(
        &mut self,
        mind: MindId,
        kind: u32,
        owner: u32,
        format: &str,
        argument: &str,
        arena: &mut Arena,
    ) -> Result<()>;
}

fn unpack_here(
    state: StateId,
    mind: MindId,
    arena: &mut Arena,
    world: &mut impl UnpackThinkWorld,
) -> Result<bool> {
    let component = world
        .pack_component(mind)?
        .ok_or_else(|| null("575C80 on a null pack component"))?;
    let facing = world.unpack_facing(state)?;
    world.unpack_at(component, facing, arena)
}

/// Complete `5372B0` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl UnpackThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    if status_at(world, mind, element::MOVE, arena)? == 2 {
        world.embedded_think(mind, element::MOVE, arena)?;
        return Ok(wrong);
    }
    match world.organization(mind)? {
        None => {
            if !unpack_here(state, mind, arena, world)? {
                let name = world.actor_name(mind, arena)?;
                let format = world.display_string(UNPACK_ERROR_KEY, arena)?;
                let owner = world.actor_owner(mind, arena)?;
                world.post_message(mind, ERROR_MESSAGE_KIND, owner, &format, &name, arena)?;
            }
        }
        Some(organization) if world.actor_is_leader(mind, organization)? => {
            unpack_here(state, mind, arena, world)?;
        }
        Some(_) => {
            let position = world.unpack_position(state)?;
            let facing = world.unpack_facing(state)?;
            if let Some(point) = world.place(state, position, facing, arena)? {
                let position = world.unpack_position(state)?;
                if world.points_equal(position, point)? {
                    unpack_here(state, mind, arena, world)?;
                } else {
                    let [ox, oy] = world.unpack_offset(mind)?;
                    let y = num(n(oy)?.add(&n(point[1])?, p))?;
                    let x = store(num(n(ox)?.add(&n(point[0])?, p))?)?;
                    let target = [x, store(y)?];
                    world.set_unpack_position(state, target)?;
                    if status_at(world, mind, element::MOVE, arena)? == 2 {
                        world.set_move_destination(mind, target)?;
                    } else {
                        let request = Activation {
                            activation_mode: 0,
                            destination_bits: target,
                            movement_mode: 1,
                            requested_speed_bits: 0,
                            auxiliary_bits: world.null_point()?,
                        };
                        world.move_activate(mind, request, arena)?;
                    }
                    world.embedded_think(mind, element::MOVE, arena)?;
                    return Ok(wrong);
                }
            }
        }
    }
    world.mind_virtual_1b0(mind, arena)?;
    Ok(wrong)
}
