//! Native Mind command ownership and dispatch (52A060, 52C320, 52BF30).
//! Commands have already passed the actor/selection authorization boundary.
//! The required storage and subtype handlers are live services, never defaults.
use crate::{Arena, Error, MindId, Result, StateId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Definition {
    /// Command definition+18. Mind is component slot4.
    pub component_slot: u32,
    /// Command definition+0C; distinct from the command registry ordinal.
    pub mind_type: u32,
    pub behavior_flags: u32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Context {
    /// Original diagnostic recursion word B3D170; wrapping, with no depth cap.
    pub depth: u32,
}
/// Recovered by executing CRT initializer9CF5D0 and its13 member thunks.
pub const HANDLER_SLOTS: [u32; 13] = [
    0xc0, 0xc4, 0xc8, 0xd0, 0xcc, 0xbc, 0xd4, 0xd8, 0xdc, 0xe0, 0xe4, 0xe8, 0xec,
];

pub trait World {
    /// Pure current command snapshots. A missing definition is an error; neither
    /// a registry ordinal nor a zero-filled command supplies a fallback type.
    fn definition(&self, command: u32) -> Result<Definition>;
    fn trailing(&self, command: u32) -> Result<bool>;
    /// Clone4D9CE0, including owned parameter data, returning one owned reference.
    /// IDs must be nonzero and unique among live commands.
    fn clone_owned(&mut self, command: u32) -> Result<u32>;
    /// Strong reference operations. On Err they must leave that reference
    /// unchanged, so callers retain ownership. They cannot mutate the Mind arena.
    fn retain(&mut self, command: u32) -> Result<()>;
    fn release(&mut self, command: u32) -> Result<()>;
    /// Actual subtype virtual identified by its recovered table slot. The handler
    /// may replace commands/stacks and recursively dispatch through this context.
    fn invoke(
        &mut self,
        mind: MindId,
        slot: u32,
        command: u32,
        arena: &mut Arena,
        context: &mut Context,
    ) -> Result<bool>;
    fn schedule(
        &mut self,
        mind: MindId,
        delay_bits: u32,
        data: u32,
        arena: &mut Arena,
    ) -> Result<()>;
    /// Current state's virtual+8, not metadata.discard_on_suspend.
    fn defers_rejected_command(&mut self, state: StateId, arena: &mut Arena) -> Result<bool>;
    /// Construct a native type6 Stop and deliver via actor4D9B60. Its return value
    /// is diagnostic only for52C320; it does not change that function's false.
    fn deliver_stop(&mut self, actor: u32, arena: &mut Arena) -> Result<bool>;
    fn actor_flags(&self, actor: u32) -> Result<u32>;
    /// Actual generation-checked group reference predicate44DEA0.
    fn has_live_group(&self, actor: u32) -> Result<bool>;
    /// Resolve4D1900 then notify4DE7A0 after the pending queue is exhausted.
    fn finish_group_commands(&mut self, actor: u32, arena: &mut Arena) -> Result<()>;
}

fn cleanup<T>(result: Result<T>, command: Option<u32>, world: &mut impl World) -> Result<T> {
    let released = match command {
        Some(id) => world.release(id),
        None => Ok(()),
    };
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(Error::World(format!(
            "temporary command {command:?} remains owned after release failure: {error}"
        ))),
        (Err(error), Err(release)) => Err(Error::World(format!(
            "{error}; temporary command {command:?} remains owned: {release}"
        ))),
    }
}

/// Assign a strong current-command reference, retaining the replacement before
/// releasing the old reference. The old reference remains installed on failure.
pub fn set_current(
    mind: MindId,
    command: Option<u32>,
    arena: &mut Arena,
    world: &mut impl World,
) -> Result<()> {
    let previous = arena.mind(mind)?.current_command;
    if command == previous {
        return Ok(());
    }
    if let Some(id) = command {
        if id == 0 {
            return Err(Error::World("null command identity".into()));
        }
        world.retain(id)?;
    }
    if let Some(id) = previous
        && let Err(error) = world.release(id)
    {
        return cleanup(Err(error), command, world);
    }
    arena.mind_mut(mind)?.current_command = command;
    Ok(())
}

/// 52A060. On a normal false return restore the previous current command. A
/// callback error preserves committed state and releases the temporary old hold.
pub fn dispatch(
    mind: MindId,
    command: u32,
    arena: &mut Arena,
    world: &mut impl World,
    context: &mut Context,
) -> Result<bool> {
    context.depth = context.depth.wrapping_add(1);
    let result = (|| {
        let definition = world.definition(command)?;
        if definition.component_slot != 4 {
            return Ok(false);
        }
        let slot = *HANDLER_SLOTS
            .get(definition.mind_type as usize)
            .ok_or_else(|| {
                Error::World(format!(
                    "unsupported native Mind command type {}",
                    definition.mind_type
                ))
            })?;
        let previous = arena.mind(mind)?.current_command;
        if let Some(id) = previous {
            world.retain(id)?;
        }
        let result = (|| {
            set_current(mind, Some(command), arena, world)?;
            let accepted = world.invoke(mind, slot, command, arena, context)?;
            if !accepted {
                set_current(mind, previous, arena, world)?;
            }
            Ok(accepted)
        })();
        cleanup(result, previous, world)
    })();
    context.depth = context.depth.wrapping_sub(1);
    result
}

/// 52C070. Retains then appends; a schedule failure leaves the command queued.
pub fn enqueue(
    mind: MindId,
    command: u32,
    arena: &mut Arena,
    world: &mut impl World,
) -> Result<()> {
    if command == 0 {
        return Err(Error::World("null command identity".into()));
    }
    arena
        .mind_mut(mind)?
        .pending_commands
        .try_reserve(1)
        .map_err(|_| Error::Allocation)?;
    world.retain(command)?;
    arena.mind_mut(mind)?.pending_commands.push_back(command);
    world.schedule(mind, 0x38d1b717, 0, arena)
}

/// 52C0D0. Release in head-to-tail order before dropping queue nodes. Zero nodes
/// represent already released references if a storage error interrupted cleanup.
pub fn clear_pending(mind: MindId, arena: &mut Arena, world: &mut impl World) -> Result<()> {
    for index in 0..arena.mind(mind)?.pending_commands.len() {
        let command = arena.mind(mind)?.pending_commands[index];
        if command != 0 {
            world.release(command)?;
            arena.mind_mut(mind)?.pending_commands[index] = 0;
        }
    }
    arena.mind_mut(mind)?.pending_commands.clear();
    Ok(())
}

/// 52A8D0. Discard suspended states without their exit callbacks, preserving the
/// active head node, and release the current command.
pub fn discard_suspended(mind: MindId, arena: &mut Arena, world: &mut impl World) -> Result<bool> {
    set_current(mind, None, arena, world)?;
    let removed = arena.mind(mind)?.stack.len() > 1;
    arena.mind_mut(mind)?.stack.truncate(1);
    Ok(removed)
}

/// 52C320. True means accepted immediately. Successfully queued or deferred
/// commands return false, exactly as the original, while retaining their effects.
pub fn process(
    mind: MindId,
    command: u32,
    arena: &mut Arena,
    world: &mut impl World,
    context: &mut Context,
) -> Result<bool> {
    let immediate_only = world.definition(command)?.behavior_flags & 0xfffffff0 == 0;
    let should_queue = world.trailing(command)? && !immediate_only;
    let copied = world.clone_owned(command)?;
    if copied == 0 {
        return Err(Error::World("command clone returned null".into()));
    }
    let mut owned = Some(copied);
    let result = (|| {
        if should_queue {
            enqueue(mind, copied, arena, world)?;
            return Ok(false);
        }
        clear_pending(mind, arena, world)?;
        if dispatch(mind, copied, arena, world, context)? {
            return Ok(true);
        }
        if let Some(Some(state)) = arena.mind(mind)?.stack.front().copied()
            && world.defers_rejected_command(state, arena)?
        {
            discard_suspended(mind, arena, world)?;
            if !immediate_only {
                enqueue(mind, copied, arena, world)?;
            }
            return Ok(false);
        }
        // The failed clone is destroyed before the original command's current
        // definition is re-read and before native Stop construction/delivery.
        cleanup(Ok(()), owned.take(), world)?;
        if world.definition(command)?.mind_type != 6 {
            let actor = arena.mind(mind)?.actor;
            world.deliver_stop(actor, arena)?;
        }
        Ok(false)
    })();
    cleanup(result, owned, world)
}

/// 52BF30 with queue pop52BEC0. Drains rejected pending commands until one is
/// accepted, or notifies the actor's live group after exhaustion. Actor flag6
/// prevents any mutation, including clearing the current command.
pub fn resume_pending(
    mind: MindId,
    arena: &mut Arena,
    world: &mut impl World,
    context: &mut Context,
) -> Result<bool> {
    let actor = arena.mind(mind)?.actor;
    if world.actor_flags(actor)? & (1 << 6) != 0 {
        return Ok(false);
    }
    set_current(mind, None, arena, world)?;
    while let Some(command) = arena.mind(mind)?.pending_commands.front().copied() {
        if command == 0 {
            return Err(Error::World("pending command node is null".into()));
        }
        // Native pop first retains a local reference, then removes the queue
        // node and releases its reference before dispatch.
        world.retain(command)?;
        arena.mind_mut(mind)?.pending_commands.pop_front();
        let result = (|| {
            if let Err(error) = world.release(command) {
                // The failed reference release leaves queue ownership intact.
                // Restore its node before cleaning up the independent local hold.
                arena.mind_mut(mind)?.pending_commands.push_front(command);
                return Err(error);
            }
            dispatch(mind, command, arena, world, context)
        })();
        if cleanup(result, Some(command), world)? {
            return Ok(true);
        }
    }
    if world.has_live_group(actor)? {
        world.finish_group_commands(actor, arena)?;
    }
    Ok(false)
}
