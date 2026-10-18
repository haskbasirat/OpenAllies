//! Native tutorial SAI enable/disable and actor-command routing.
//! The real component command handler is required; this never bypasses actor
//! admission or parent routing to call a Mind directly.
use crate::startup::Object;
use serde::{Deserialize, Serialize};
#[derive(Debug, thiserror::Error)]
#[error("native AI control service failed: {0}")]
pub struct Error(pub String);
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shared(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node(pub u64);

/// Original4D9E20 stack command fields. Logical references replace addresses;
/// the field+34 strong reference is released by the caller after dispatch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub references: u32,
    pub definition: Option<Object>,
    pub flags: u32,
    pub words10_2c: [u32; 8],
    pub byte30: u8,
    pub shared34: Option<Shared>,
    pub word38: u32,
    pub byte3c: u8,
}
impl Command {
    pub fn new(definition: Option<Object>) -> Self {
        Self {
            references: 0,
            definition,
            flags: if definition.is_some() { 0x40 } else { 0 },
            words10_2c: [0; 8],
            byte30: 0,
            shared34: None,
            word38: 0,
            byte3c: 0,
        }
    }
}
pub trait World {
    fn flags8(&self, object: Object) -> Result<u32>;
    fn set_flags8(&mut self, object: Object, value: u32) -> Result<()>;
    /// Actor+154, not its Mind or actor ID.
    fn actor_sai_handle(&self, actor: Object) -> Result<Option<Shared>>;
    /// Reference reads/updates act on the actual owned object's native+4 word.
    /// On failure, a reference update must leave that word unchanged. A later
    /// destruction failure retains the already-zero word; the runtime must
    /// stop rather than retrying the release and decrementing it again.
    fn references(&self, shared: Shared) -> Result<u32>;
    fn set_references(&mut self, shared: Shared, value: u32) -> Result<()>;
    fn destroy_shared(&mut self, shared: Shared) -> Result<()>;
    /// Shared object+10 at the original reread point after the temporary retain.
    fn sai_target(&self, shared: Shared) -> Result<Option<Object>>;
    /// Exact target virtual+68(handle,0,0,0,0). May mutate live world fields.
    fn notify_disabled(
        &mut self,
        target: Object,
        handle: Shared,
        zero_arguments: [u32; 4],
    ) -> Result<()>;
    fn kingdom_head284(&self, kingdom: Object) -> Result<Option<Node>>;
    fn node_actor(&self, node: Node) -> Result<Option<Object>>;
    /// Reread only after command processing and temporary reference cleanup.
    fn node_next(&self, node: Node) -> Result<Option<Node>>;
    /// Actual command registry at globalB36530+298, indexed by ordinal6.
    fn command_definition(&self, ordinal: u32) -> Result<Option<Object>>;
    fn definition_component18(&self, definition: Object) -> Result<i32>;
    fn actor_component(&self, actor: Object, index: usize) -> Result<Option<Object>>;
    fn can_receive(
        &mut self,
        actor: Object,
        definition: Object,
        command: &mut Command,
    ) -> Result<bool>;
    fn process_component(&mut self, component: Object, command: &mut Command) -> Result<bool>;
    fn actor_virtual158(&mut self, actor: Object, command: &mut Command) -> Result<bool>;
    fn actor_virtual15c(&mut self, actor: Object, command: &mut Command) -> Result<Option<Object>>;
    /// Actual actor+7C component, then its+44 parent; null at either yieldsNone.
    fn organization_parent(&self, actor: Object) -> Result<Option<Object>>;
}
fn release(shared: Shared, world: &mut impl World) -> Result<()> {
    let count = world.references(shared)?.wrapping_sub(1);
    world.set_references(shared, count)?;
    if count == 0 {
        world.destroy_shared(shared)?;
    }
    Ok(())
}
fn cleanup<T>(result: Result<T>, shared: Option<Shared>, world: &mut impl World) -> Result<T> {
    let done = shared.map_or(Ok(()), |shared| release(shared, world));
    match (result, done) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e),
        (Err(e), Err(clean)) => Err(Error(format!(
            "{e}; temporary reference cleanup failed: {clean}"
        ))),
    }
}
///542080→79E2F0. Shared by the actor and Kingdom enable trigger wrappers.
pub fn enable(object: Option<Object>, world: &mut impl World) -> Result<bool> {
    let Some(object) = object else {
        return Ok(false);
    };
    let flags = world.flags8(object)? & !0x20;
    world.set_flags8(object, flags)?;
    Ok(true)
}
///79E980. Cache/retain the actual actor+154 handle before calling virtual+68;
/// callback replacement of actor+154 never changes which reference is released.
pub fn disable_actor_state(actor: Object, world: &mut impl World) -> Result<()> {
    let flags = world.flags8(actor)? | 0x20;
    world.set_flags8(actor, flags)?;
    let held = world.actor_sai_handle(actor)?;
    let Some(held) = held else { return Ok(()) };
    let count = world.references(held)?.wrapping_add(1);
    world.set_references(held, count)?;
    let result = (|| {
        if let Some(target) = world.sai_target(held)? {
            world.notify_disabled(target, held, [0; 4])?;
        }
        Ok(())
    })();
    cleanup(result, Some(held), world)
}
///Actual4D9B60. The found component pointer is cached before admission, and a
///rejected admission returnsfalse without attempting parent fallback. There is
///no arbitrary hop cap; the live hierarchy must obey the native ownership rules.
pub fn process(
    command: &mut Command,
    mut actor: Option<Object>,
    world: &mut impl World,
) -> Result<bool> {
    while let Some(current) = actor {
        let definition = command
            .definition
            .ok_or_else(|| Error("command has no resolved definition".into()))?;
        let index = world.definition_component18(definition)?;
        let index = usize::try_from(index)
            .ok()
            .filter(|&n| n < 31)
            .ok_or_else(|| {
                Error("command definition selects outside native31component table".into())
            })?;
        if let Some(component) = world.actor_component(current, index)? {
            //Native rereads command+8 for the admission argument.
            let definition = command
                .definition
                .ok_or_else(|| Error("command definition disappeared before admission".into()))?;
            if world.can_receive(current, definition, command)? {
                return world.process_component(component, command);
            }
            return Ok(false);
        }
        actor = if world.actor_virtual158(current, command)? {
            world.actor_virtual15c(current, command)?
        } else {
            world.organization_parent(current)?
        };
    }
    //The original only emits a diagnostic before returningfalse here.
    Ok(false)
}
/// Construct ordinal6, process through4D9B60 and release command+34 ownership.
/// The return is the actual component result; false may have committed effects.
pub fn stop(actor: Option<Object>, world: &mut impl World) -> Result<bool> {
    let mut command = Command::new(world.command_definition(6)?);
    let result = process(&mut command, actor, world);
    cleanup(result, command.shared34, world)
}
///5432C0. Every actual node gets a fresh command; its false result is ignored.
///Read node.next after callbacks, preserving native mutation/iteration order.
pub fn disable_kingdom(kingdom: Option<Object>, world: &mut impl World) -> Result<bool> {
    let Some(kingdom) = kingdom else {
        return Ok(false);
    };
    let flags = world.flags8(kingdom)? | 0x20;
    world.set_flags8(kingdom, flags)?;
    let mut node = world.kingdom_head284(kingdom)?;
    while let Some(current) = node {
        let actor = world.node_actor(current)?;
        stop(actor, world)?;
        node = world.node_next(current)?;
    }
    Ok(true)
}
///543380. No Stop is constructed if the required disable notification fails.
pub fn disable_actor(actor: Option<Object>, world: &mut impl World) -> Result<bool> {
    let Some(actor) = actor else { return Ok(false) };
    disable_actor_state(actor, world)?;
    stop(Some(actor), world)?;
    Ok(true)
}
