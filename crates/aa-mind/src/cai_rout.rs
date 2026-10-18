//! Original `KKC_CAIStateRout::Think` (`521240`, regiment state `cai_rout`
//! vtable A32AC8 slot +C): the routing (retreat) behaviour of an organization.
//!
//! Order: TAIState base Think (`529E60`); if the state is current (`52A000`)
//! and `RoutingTime` seconds have passed since it was entered (state+10), the
//! Mind's `+178` virtual ends the rout. Otherwise, when the embedded move state
//! (Mind+68) is not active (status != 2), a destination is chosen by `52B530`
//! (`crate::rout_select::select` with the world RNG): success activates the
//! move to it (`520440`) and schedules an immediate update, failure schedules
//! a retry in 2.0 s. While moving, the destination is copied into the current
//! command (`529F50`, when one exists); arrival within 0.5 units (`52A1F0`)
//! calls the Mind's `+144` virtual, otherwise the move state thinks.
use crate::{Arena, Result, StateId, is_current, think_base, within_arrival, x87::n};
use aa_numeric::Precision;
use std::cmp::Ordering;

/// binary32 2.0: retry delay after a failed destination search.
pub const RETRY_DELAY_BITS: u32 = 0x4000_0000;

/// Required services and live reads, called in native order.
pub trait RoutThinkWorld {
    /// `[B36534]+E0` game time (binary32).
    fn world_time(&mut self) -> Result<u32>;
    /// SVar `RoutingTime` (`[[B39AB0]+B0]`, binary32).
    fn routing_time(&mut self) -> Result<u32>;
    /// Rout state+10: game time recorded by `KKC_CAIStateRout::OnEnter` (51F190).
    fn entered_time(&mut self, state: StateId) -> Result<u32>;
    /// Mind virtual `+178` (routing time elapsed).
    fn end_rout(&mut self, arena: &mut Arena) -> Result<()>;
    /// Embedded move state (Mind+68) status word (+C).
    fn move_status(&mut self, arena: &Arena) -> Result<u32>;
    /// `52B530` on the world RNG (`4F02B0`); see `rout_select::select`.
    fn select_destination(&mut self, arena: &mut Arena) -> Result<Option<[u32; 2]>>;
    /// `520440(0, destination, 0, [B279C0], [B279C4])` on the move state.
    fn activate_move(&mut self, destination: [u32; 2], arena: &mut Arena) -> Result<()>;
    /// Mind Updatable (+8) virtual `+4(delay, 0)`.
    fn schedule(&mut self, delay_bits: u32, arena: &mut Arena) -> Result<()>;
    /// Mind+20 current command is present.
    fn has_current_command(&mut self, arena: &Arena) -> Result<bool>;
    /// `529F50`: current command flags |= 4 and +18/+1C = the move destination.
    fn mark_command_destination(&mut self, arena: &mut Arena) -> Result<()>;
    /// Move state +14/+18 destination (Mind+7C/+80).
    fn move_destination(&mut self, arena: &Arena) -> Result<[u32; 2]>;
    /// Mind actor +20/+24.
    fn actor_position(&mut self, arena: &Arena) -> Result<[u32; 2]>;
    /// Mind virtual `+144` (destination reached).
    fn reached(&mut self, arena: &mut Arena) -> Result<()>;
    /// Move state virtual `+C` (its Think).
    fn move_think(&mut self, arena: &mut Arena) -> Result<()>;
}

/// Complete `521240` (profiling/diagnostic calls excluded). Returns whether
/// the base Think reported a wrong activity (native one-time diagnostic).
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl RoutThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong_activity = think_base(state, arena)?;
    if is_current(state, arena)? {
        let now = world.world_time()?;
        let limit = world.routing_time()?;
        let entered = world.entered_time(state)?;
        let elapsed = crate::x87::num(n(now)?.sub(&n(entered)?, p))?;
        // `fcomp; test ah,1; jne`: end unless less-than (or unordered).
        if elapsed.compare(&n(limit)?) != Ordering::Less {
            world.end_rout(arena)?;
            return Ok(wrong_activity);
        }
    }
    if world.move_status(arena)? != 2 {
        match world.select_destination(arena)? {
            None => world.schedule(RETRY_DELAY_BITS, arena)?,
            Some(destination) => {
                world.activate_move(destination, arena)?;
                world.schedule(0, arena)?;
            }
        }
        return Ok(wrong_activity);
    }
    if world.has_current_command(arena)? {
        world.mark_command_destination(arena)?;
    }
    let destination = world.move_destination(arena)?;
    let position = world.actor_position(arena)?;
    if within_arrival(position, destination, p)? {
        world.reached(arena)?;
    } else {
        world.move_think(arena)?;
    }
    Ok(wrong_activity)
}

/// Services of `KKC_CAIStateRout::OnEnter` beyond the engage-list services.
pub trait RoutEnterWorld: crate::cai_engage::EngageWorld {
    /// `[B36534]+E0` game time (binary32).
    fn world_time(&mut self) -> Result<u32>;
    /// Rout state+10 (read back by the Think as `entered_time`).
    fn set_entered_time(&mut self, state: StateId, time: u32) -> Result<()>;
    /// `4D3FA0` on the organization (`529C30`).
    fn organization_halt(&mut self, arena: &mut Arena) -> Result<()>;
    /// Reference release of one pending command object (`52C0D0`); the
    /// object is destroyed by its owner when the count reaches zero.
    fn release_command(&mut self, command: u32) -> Result<()>;
}

/// Complete `51F190` (`KKC_CAIStateRout::OnEnter`, regiment rout state slot
/// +18) without its diagnostic log line: status +C = 2, entry time +10 = game
/// time, organization halt `4D3FA0`, engage target clear `51F040` on the
/// Mind's engage list (Mind+1CC) and `52C0D0` (release every pending command,
/// Mind+18/+1C, then empty the list; a null node (0) is skipped).
pub fn enter(
    state: StateId,
    targets: &mut crate::cai_engage::Targets,
    arena: &mut Arena,
    world: &mut impl RoutEnterWorld,
) -> Result<()> {
    let mind = arena
        .states
        .get(&state)
        .ok_or(crate::Error::MissingState(state.0))?
        .parent;
    arena
        .states
        .get_mut(&state)
        .ok_or(crate::Error::MissingState(state.0))?
        .status = 2;
    let now = world.world_time()?;
    world.set_entered_time(state, now)?;
    world.organization_halt(arena)?;
    crate::cai_engage::clear_targets(targets, world)?;
    let pending: Vec<u32> = arena
        .minds
        .get(&mind)
        .ok_or(crate::Error::MissingMind(mind.0))?
        .pending_commands
        .iter()
        .copied()
        .collect();
    for command in pending {
        if command != 0 {
            world.release_command(command)?;
        }
    }
    arena
        .minds
        .get_mut(&mind)
        .ok_or(crate::Error::MissingMind(mind.0))?
        .pending_commands
        .clear();
    Ok(())
}
