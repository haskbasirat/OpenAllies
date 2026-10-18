//! Original `KKC_UAIStateUnload::Think` (`52F440`, element state
//! `uai_unload` at Mind+34, vtable A34774 slot +C): a transporting unit waits
//! until its cargo is unloaded, then leaves the state.
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]).
//! 2. Unless the world time (`[B36534]+E0`) is strictly greater than the
//!    state+10 time (unordered counts as not greater), schedule 0.3 s, done.
//! 3. `58BCB0` on the actor transport component (+C0; null is a native null
//!    `this`). While its +40 count is below +3C (signed), state+10 becomes
//!    `definition+41C + time` (x87 add at the control-word precision, stored
//!    binary32) and 0.3 s is scheduled, done.
//! 4. An actor with flag 0x40 in +104 schedules 0.3 s, done.
//! 5. The state finishes ([`crate::pop`], `52A340`). A transport with the
//!    +4C byte orders the move state (Mind+7C) with
//!    `52DF60(2, transport+50/+54, 0, 0, [B279C0], [B279C4])`; otherwise the
//!    join state (Mind+164) is activated in mode 2 on the Mind's home object
//!    (`4D09A0`: Mind+28 resolved; [`super::uai_join::activate_join`]).
//!    Then 1e-4 s is scheduled.
use super::{
    IMMEDIATE_BITS, POINT_THREE_BITS, element, null,
    uai_join::{JoinActivateWorld, activate_join},
};
use crate::{
    Arena, MindId, Result, StateId, pop, think_base,
    x87::{n, num, store},
};
use aa_navigation::Activation;
use aa_numeric::Precision;

/// Services of `52F440` in native order.
pub trait UnloadThinkWorld: JoinActivateWorld {
    /// `[B36534]+E0` world time (binary32).
    fn world_time(&mut self) -> Result<u32>;
    /// Unload state+10 (binary32 next check time) and its write.
    fn unload_time(&mut self, state: StateId) -> Result<u32>;
    fn set_unload_time(&mut self, state: StateId, bits: u32) -> Result<()>;
    /// Actor +C0 (transport component), `None` when null.
    fn transport(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// `58BCB0` on the transport component.
    fn transport_update(&mut self, transport: u32) -> Result<()>;
    /// Transport +40 and +3C (signed counts), read after the update.
    fn transport_counts(&mut self, transport: u32) -> Result<[i32; 2]>;
    /// Actor definition +41C (binary32 unload interval).
    fn unload_interval(&mut self, mind: MindId) -> Result<u32>;
    /// Actor +104 flags.
    fn actor_flags(&mut self, mind: MindId) -> Result<u32>;
    /// Transport +4C byte and +50/+54 point.
    fn transport_byte4c(&mut self, transport: u32) -> Result<bool>;
    fn transport_point(&mut self, transport: u32) -> Result<[u32; 2]>;
    /// `52DF60` on the move state (Mind+7C); movement-owned
    /// (`aa_navigation::activate_move`). Its result is unused here.
    fn move_activate(
        &mut self,
        mind: MindId,
        request: Activation,
        arena: &mut Arena,
    ) -> Result<bool>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Complete `52F440` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl UnloadThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let time = world.world_time()?;
    let due = world.unload_time(state)?;
    if host(time) > host(due) {
        let transport = world
            .transport(mind)?
            .ok_or_else(|| null("58BCB0 on a null transport"))?;
        world.transport_update(transport)?;
        let [loaded, capacity] = world.transport_counts(transport)?;
        if loaded < capacity {
            let interval = world.unload_interval(mind)?;
            let time = world.world_time()?;
            let next = store(num(n(interval)?.add(&n(time)?, p))?)?;
            world.set_unload_time(state, next)?;
        } else if world.actor_flags(mind)? & 0x40 == 0 {
            pop(mind, arena, world)?;
            let transport = world
                .transport(mind)?
                .ok_or_else(|| null("transport +4C read"))?;
            if world.transport_byte4c(transport)? {
                let destination = world.transport_point(transport)?;
                let request = Activation {
                    activation_mode: 2,
                    destination_bits: destination,
                    movement_mode: 0,
                    requested_speed_bits: 0,
                    auxiliary_bits: world.null_point()?,
                };
                world.move_activate(mind, request, arena)?;
            } else {
                let home = arena.mind(mind)?.words_24_30[1];
                let home = world.resolve(home)?;
                let join = world.embedded_state(mind, element::JOIN)?;
                activate_join(join, 2, home, arena, world)?;
            }
            world.schedule(mind, IMMEDIATE_BITS, 0, arena)?;
            return Ok(wrong);
        }
    }
    world.schedule(mind, POINT_THREE_BITS, 0, arena)?;
    Ok(wrong)
}
