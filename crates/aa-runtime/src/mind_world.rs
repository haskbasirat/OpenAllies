//! Live Mind lifecycle/command storage over the canonical ActorStore split.
//! Concrete field, identity, queue and simple-command ownership operations live
//! here. Unported state/world effects remain mandatory named services.
use crate::{
    actor_world::{ActiveComponent, ActorAccess, RuntimeComponent},
    scheduler_owners::{Owners, Target},
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_mind::{Arena, Error, MindId, Result, StateId, command, construct::Kind};
use aa_world::queues::{Owner, Queues};
use std::collections::BTreeMap;

fn fail(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
/// Storage-independent native529EC0 coordinator (also called by51D1B0).
/// The caller supplies the actual derived virtuals over its current owner;
/// this helper neither constructs actor storage nor substitutes world effects.
pub trait InitExistingWorld {
    fn reset_default(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    fn update_actor_state(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
}
pub fn init_existing(
    mind: MindId,
    arena: &mut Arena,
    world: &mut impl InitExistingWorld,
) -> Result<bool> {
    if arena.mind(mind)?.stack.is_empty() {
        world.reset_default(mind, arena)?;
    } else {
        world.update_actor_state(mind, arena)?;
    }
    Ok(true)
}
///52D7C0's single parent coordinator. A reached child error preserves the
///literal native guard-store prefix; final command clearing sees guard0.
pub(crate) trait ElementResetWorld {
    fn resetting(&mut self)->bool;
    fn store_resetting(&mut self,value:bool);
    fn resume(&mut self)->Result<bool>;
    fn activate_sleep(&mut self)->Result<()>;
    fn clear_current(&mut self)->Result<()>;
}
pub(crate) fn reset_element(w:&mut impl ElementResetWorld)->Result<()> {
    if w.resetting(){return Ok(());}
    w.store_resetting(true);
    if w.resume()? {w.store_resetting(false);return Ok(());}
    w.activate_sleep()?;
    w.store_resetting(false);
    w.clear_current()
}
/// The native command's plain fields, with stable definition references. This
/// type deliberately cannot represent a ParameterSet heap object: its copy and
/// destruction require the separately unported4C2290/4C2230 behavior.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlainCommand {
    /// Stable immutable registry identity for native command+8, separate from
    /// the projected dispatch fields below. No native pointer is dereferenced.
    pub definition_id: u32,
    /// The saved loader accepts a null definition. Dispatch must refuse it,
    /// rather than inventing a command type for the native null pointer.
    pub definition: Option<command::Definition>,
    pub flags: u32,
    pub word10: u32,
    pub object_definition: Option<u32>,
    pub position_bits: [u32; 2],
    pub auxiliary_bits: [u32; 2],
    pub scalar_bits: u32,
    pub parameter_definition: Option<u32>,
    pub extra_definition: Option<u32>,
    pub boolean: u8,
    pub trailing: u8,
}
struct StoredCommand {
    value: PlainCommand,
    references: u32,
}
#[derive(Default)]
pub struct Commands {
    next: u32,
    values: BTreeMap<u32, StoredCommand>,
}
impl Commands {
    pub(crate) fn is_pristine(&self) -> bool {
        self.next == 0 && self.values.is_empty()
    }
    /// Import one distinct allocation made by saved Mind Load. The source
    /// owner is moved; no extra retain is performed and gaps remain gaps.
    pub(crate) fn insert_restored(&mut self, id: u32, value: PlainCommand) -> Result<()> {
        if id == 0 || self.values.contains_key(&id) {
            return Err(fail("restored command identity is zero or repeats"));
        }
        self.values.insert(id, StoredCommand { value, references: 1 });
        self.next = self.next.max(id);
        Ok(())
    }
    /// Creates one owned reference. Portable identities are never native pointers
    /// or command registry ordinals, and no live identity is reused.
    pub fn allocate(&mut self, value: PlainCommand) -> Result<u32> {
        let id = self
            .next
            .checked_add(1)
            .ok_or_else(|| fail("command identities exhausted"))?;
        self.values.insert(
            id,
            StoredCommand {
                value,
                references: 1,
            },
        );
        self.next = id;
        Ok(id)
    }
    pub fn value(&self, id: u32) -> Result<&PlainCommand> {
        self.values
            .get(&id)
            .map(|v| &v.value)
            .ok_or_else(|| fail("live command identity missing"))
    }
    /// Mutate the one live command payload without replacing its identity or
    /// reference count. Native callbacks may change fields that callers reread.
    pub fn value_mut(&mut self, id: u32) -> Result<&mut PlainCommand> {
        self.values.get_mut(&id).map(|v| &mut v.value)
            .ok_or_else(|| fail("live command identity missing"))
    }
    pub fn references(&self, id: u32) -> Result<u32> {
        self.values
            .get(&id)
            .map(|v| v.references)
            .ok_or_else(|| fail("live command identity missing"))
    }
    pub fn clone_owned(&mut self, id: u32) -> Result<u32> {
        self.allocate(self.value(id)?.clone())
    }
    pub fn retain(&mut self, id: u32) -> Result<()> {
        let value = self
            .values
            .get_mut(&id)
            .ok_or_else(|| fail("retained command identity missing"))?;
        value.references = value
            .references
            .checked_add(1)
            .ok_or_else(|| fail("command reference count overflow"))?;
        Ok(())
    }
    pub fn release(&mut self, id: u32) -> Result<()> {
        let value = self
            .values
            .get_mut(&id)
            .ok_or_else(|| fail("released command identity missing"))?;
        if value.references == 1 {
            self.values.remove(&id);
        } else {
            value.references = value
                .references
                .checked_sub(1)
                .ok_or_else(|| fail("invalid zero command reference count"))?;
        }
        Ok(())
    }
}
#[derive(Default)]
pub struct Runtime {
    pub commands: Commands,
    pub movement: crate::movement_world::Runtime,
    /// One original B3D2D4 reentry guard shared by all Element Minds.
    element_resetting: bool,
    ///One process B3CF58 latch used by native51E3A0. This moves with the
    ///interactive application's canonical runtime; it is not reset per Mind.
    regiment_resetting: bool,
    ///B3D43C is zero-initialized process storage, not a command/World reset.
    ///Native Solve's disable_path_logging callback must mutate this SAME byte.
    path_log: u8,
    dispatch_depth: u32,
}
impl Runtime {
    ///Fresh interactive application interval only. Both reset latches are
    ///zero-initialized PE storage, NOT stores performed by GWorld's ctor.
    ///The owning application transfers this one runtime into its World and
    ///retains it across callbacks; a replacement World must move/reuse it.
    ///Evidence: skirmish-20261017-stored-mind-init/native-reset-guards.json.
    pub fn from_fresh_application_interval()->Self { Self::default() }
    pub(crate) fn regiment_resetting(&self)->bool { self.regiment_resetting }
    pub(crate) fn set_regiment_resetting(&mut self,value:bool) {self.regiment_resetting=value;}
    pub(crate) fn path_log(&self)->u8 {self.path_log}
    pub(crate) fn set_path_log(&mut self,value:u8) {self.path_log=value;}
    /// Move restored ownership into the one command runtime before issuing
    /// any live commands. A second bind or a non-pristine target is an error.
    pub fn bind_restored_commands(&mut self, actors: &mut crate::actor_world::ActorStore) -> Result<()> {
        if !self.commands.is_pristine() {
            return Err(fail("command runtime already allocated identities"));
        }
        let commands = actors.restored_commands.take()
            .ok_or_else(|| fail("no restored command arena is awaiting transfer"))?;
        self.commands = commands;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateBinding {
    pub actor: ActorId,
    pub mind: MindId,
    pub kind: Kind,
    pub offset: u32,
}
fn installed<'a>(
    actors: &'a ActorAccess<'_>,
    arena: &Arena,
    mind: MindId,
) -> Result<&'a aa_mind::construct::Installed> {
    let actor = ActorId::from_raw(arena.mind(mind)?.actor);
    match actors.component(actor, ComponentSlot::Mind).map_err(fail)? {
        Some(RuntimeComponent::Mind(value)) if value.mind_id == mind => Ok(value),
        _ => Err(fail("canonical actor Mind identity mismatch")),
    }
}
pub fn state_binding(
    actors: &ActorAccess<'_>,
    arena: &Arena,
    state: StateId,
) -> Result<StateBinding> {
    let mind = arena.state(state)?.parent;
    let value = installed(actors, arena, mind)?;
    let offset = value
        .state_bindings
        .iter()
        .find_map(|(&offset, &id)| (id == state).then_some(offset))
        .ok_or_else(|| fail("state identity is absent from its installed Mind"))?;
    Ok(StateBinding {
        actor: ActorId::from_raw(arena.mind(mind)?.actor),
        mind,
        kind: value.kind,
        offset,
    })
}
/// Actual Mind virtual+64: Element535E40, Regiment5247B0, Aircraft516AC0.
/// This tests the embedded Move status, not the current stack head.
pub fn moving(actors: &ActorAccess<'_>, arena: &Arena, actor: ActorId) -> Result<bool> {
    let Some(RuntimeComponent::Mind(value)) =
        actors.component(actor, ComponentSlot::Mind).map_err(fail)?
    else {
        return Err(fail("moving predicate requires a concrete Mind"));
    };
    let offset = match value.kind {
        Kind::Element => 0x7c,
        Kind::Regiment => 0x68,
        Kind::Aircraft => 0x34,
        Kind::Base => return Err(fail("abstract Mind moving predicate")),
    };
    let state = *value
        .state_bindings
        .get(&offset)
        .ok_or_else(|| fail("embedded Move binding missing"))?;
    if arena.state(state)?.parent != value.mind_id {
        return Err(fail("embedded Move parent mismatch"));
    }
    Ok(arena.state(state)?.status == 2)
}
fn state_bits(actors: &ActorAccess<'_>, actor: ActorId) -> Result<u64> {
    let key = actors
        .key(actor)
        .ok_or_else(|| fail("live actor identity missing"))?;
    let state = actors
        .object(key)
        .map_err(fail)?
        .components()
        .map_err(fail)?
        .state;
    Ok(u64::from(state.low) | (u64::from(state.high) << 32))
}
/// Actual Element metadata priorities from the original CRT initializers.
pub fn element_priority(offset: u32) -> Result<i32> {
    match offset {
        0x34 | 0x48 => Ok(3),
        0x68 => Ok(0),
        0x7c | 0xf8 | 0x10c | 0x12c | 0x148 | 0x164 | 0x178 | 0x188 => Ok(2),
        _ => Err(fail("unknown Element state metadata offset")),
    }
}
/// Services receive both disjoint live actor storage and the canonical Arena.
/// They may reenter the recovered lifecycle/command functions; they must not
/// replace the active state allocation. No default method claims success.
pub struct Context<'a, 'store> {
    pub actors: &'a mut ActorAccess<'store>,
    pub arena: &'a mut Arena,
    pub runtime: &'a mut Runtime,
    pub queues: &'a mut Queues,
    pub owners: &'a mut Owners,
}
pub trait Services {
    fn can_activate_other(
        &mut self,
        state: StateId,
        binding: StateBinding,
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
    fn enter_other(
        &mut self,
        state: StateId,
        binding: StateBinding,
        context: &mut Context<'_, '_>,
    ) -> Result<()>;
    fn leave_other(
        &mut self,
        state: StateId,
        binding: StateBinding,
        context: &mut Context<'_, '_>,
    ) -> Result<()>;
    fn actor_state_other(
        &mut self,
        state: StateId,
        binding: StateBinding,
        context: &mut Context<'_, '_>,
    ) -> Result<u8>;
    /// Actual4D64F0 including live state effects; raw bit assignment is invalid.
    fn set_actor_state(
        &mut self,
        actor: ActorId,
        index: u8,
        context: &mut Context<'_, '_>,
    ) -> Result<()>;
    fn rebuild_actions(&mut self, actor: ActorId, context: &mut Context<'_, '_>) -> Result<()>;
    /// Actual4CFC80, including Character/default-component effects.
    fn default_stance(&mut self, actor: ActorId, context: &mut Context<'_, '_>) -> Result<()>;
    fn reset_other(
        &mut self,
        mind: MindId,
        kind: Kind,
        context: &mut Context<'_, '_>,
    ) -> Result<()>;
    /// Complete derived Mind virtual+B8, not only a guessed actor state number.
    fn sync_state_other(
        &mut self,
        mind: MindId,
        kind: Kind,
        context: &mut Context<'_, '_>,
    ) -> Result<()>;
    fn facing(&self, actor: ActorId, actors: &ActorAccess<'_>) -> Result<[u32; 2]>;
    fn set_facing(
        &mut self,
        actor: ActorId,
        facing: [u32; 2],
        context: &mut Context<'_, '_>,
    ) -> Result<()>;
    /// Actor flags may already belong to the spatial store; this getter must
    /// route to that authoritative owner, never a stale constructor copy.
    fn actor_flags(&self, actor: ActorId, actors: &ActorAccess<'_>) -> Result<u32>;
    fn has_live_group(&self, actor: ActorId, actors: &ActorAccess<'_>) -> Result<bool>;
    fn finish_group_commands(
        &mut self,
        actor: ActorId,
        context: &mut Context<'_, '_>,
    ) -> Result<()>;
    /// The source command was admitted before this component boundary. Bind
    /// exact native subtype handlers; unsupported handlers must return errors.
    fn invoke_command(
        &mut self,
        mind: MindId,
        slot: u32,
        command: u32,
        command_context: &mut command::Context,
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
    fn deliver_stop(&mut self, actor: ActorId, context: &mut Context<'_, '_>) -> Result<bool>;
    fn defers_other(
        &mut self,
        state: StateId,
        binding: StateBinding,
        context: &mut Context<'_, '_>,
    ) -> Result<bool>;
}
pub struct World<'a, 'store, S> {
    pub actors: ActorAccess<'store>,
    pub runtime: &'a mut Runtime,
    pub queues: &'a mut Queues,
    pub owners: &'a mut Owners,
    pub services: &'a mut S,
}
struct ElementReset<'call,'world,'store,S>{world:&'call mut World<'world,'store,S>,arena:&'call mut Arena,mind:MindId}
impl<S:Services> ElementResetWorld for ElementReset<'_,'_,'_,S>{
    fn resetting(&mut self)->bool{self.world.runtime.element_resetting}
    fn store_resetting(&mut self,value:bool){self.world.runtime.element_resetting=value;}
    fn resume(&mut self)->Result<bool>{self.world.resume_pending_commands(self.mind,self.arena)}
    fn activate_sleep(&mut self)->Result<()> {
        let sleep=*installed(&self.world.actors,self.arena,self.mind)?.state_bindings.get(&0x68)
            .ok_or_else(||fail("Element Sleep68 binding missing"))?;
        //Native ignores this Boolean, including a genuine refusal.
        let _=aa_mind::activate(sleep,2,self.arena,self.world)?;Ok(())
    }
    fn clear_current(&mut self)->Result<()>{command::set_current(self.mind,None,self.arena,self.world)}
}
impl<S: Services> World<'_, '_, S> {
    ///Native52BF30 over the SAME command directory, stack and current dispatch
    ///depth. Fresh InitExisting and normal resets share this implementation.
    pub fn resume_pending_commands(&mut self,mind:MindId,arena:&mut Arena)->Result<bool> {
        let mut context=command::Context{depth:self.runtime.dispatch_depth};
        command::resume_pending(mind,arena,self,&mut context)
    }
    /// Native Mind virtual+24/52C320 after actual actor/selection authorization.
    /// Preserve the one Runtime B3D170 owner across synchronous nested dispatch;
    /// no independent top-level depth counter or command arena is created.
    pub fn process_command(&mut self, mind: MindId, id: u32, arena: &mut Arena) -> Result<bool> {
        installed(&self.actors, arena, mind)?;
        let mut context = command::Context { depth: self.runtime.dispatch_depth };
        command::process(mind, id, arena, self, &mut context)
    }
    pub fn owner(&self, mind: MindId, arena: &Arena) -> Result<Owner> {
        installed(&self.actors, arena, mind)?;
        let actor = ActorId::from_raw(arena.mind(mind)?.actor);
        let object = self
            .actors
            .key(actor)
            .ok_or_else(|| fail("Mind actor identity missing"))?;
        self.owners
            .get(Target::Component {
                object,
                slot: ComponentSlot::Mind as u8,
            })
            .map_err(fail)
    }
    fn active(&self, actor: ActorId, slot: ComponentSlot) -> Result<bool> {
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| fail("component owner is not live"))?;
        match self.actors.object(key).map_err(fail)?.active_component {
            ActiveComponent::None => Ok(false),
            ActiveComponent::Slot(current) => Ok(current == slot),
            ActiveComponent::UnresolvedNativeWord(_) => {
                Err(fail("active component reference is unresolved"))
            }
        }
    }
    pub fn schedule(
        &mut self,
        mind: MindId,
        delay_bits: u32,
        data: u32,
        arena: &mut Arena,
    ) -> Result<()> {
        let kind = installed(&self.actors, arena, mind)?.kind;
        if kind == Kind::Aircraft {
            return Err(fail(
                "Aircraft scheduling51C8B0 requires its flight service",
            ));
        }
        if kind == Kind::Element {
            let actor = ActorId::from_raw(arena.mind(mind)?.actor);
            if let Some(RuntimeComponent::Performer(p)) = self
                .actors
                .component(actor, ComponentSlot::Performer)
                .map_err(fail)?
                && self.active(ActorId::from_raw(p.state.owner), ComponentSlot::Performer)?
                && p.state.stage != 4
            {
                return Ok(());
            }
            let delay = aa_numeric::Binary::from_f32_bits(delay_bits).map_err(fail)?;
            if delay.compare(&aa_numeric::Binary::from_i32(0)).is_gt() {
                let actor = ActorId::from_raw(arena.mind(mind)?.actor);
                if let Some(RuntimeComponent::Mover(c)) = self
                    .actors
                    .component(actor, ComponentSlot::Mover)
                    .map_err(fail)?
                {
                    let mover_actor = c.header.owner;
                    let key = self
                        .actors
                        .key(mover_actor)
                        .ok_or_else(|| fail("Mover owner is not live"))?;
                    let owner = self
                        .owners
                        .get(Target::Component {
                            object: key,
                            slot: ComponentSlot::Mover as u8,
                        })
                        .map_err(fail)?;
                    if self.active(mover_actor, ComponentSlot::Mover)?
                        && !self.queues.has_owner(owner)
                    {
                        self.services.default_stance(
                            actor,
                            &mut Context {
                                actors: &mut self.actors,
                                arena,
                                runtime: self.runtime,
                                queues: self.queues,
                                owners: self.owners,
                            },
                        )?;
                    }
                }
            }
        }
        //50AD80 invokes virtual+10=529CD0, whose50B250 cancels all pending
        //entries before admitting the new one. World+208 is deferred queue4.
        let owner = self.owner(mind, arena)?;
        self.queues.cancel_owner(owner).map_err(fail)?;
        self.queues
            .add_after(4, owner, data, delay_bits)
            .map_err(fail)?;
        Ok(())
    }
    /// All three concrete Mind InitCreate vtables select438AC0 (true/ret4).
    /// This does not enter a state, publish the actor or initialize saved state.
    pub fn init_create(&self, mind: MindId, arena: &Arena) -> Result<bool> {
        if installed(&self.actors, arena, mind)?.kind == Kind::Base {
            return Err(fail(
                "abstract Mind InitCreate is outside concrete runtime domain",
            ));
        }
        Ok(true)
    }
    /// Element529EC0, also called by Regiment51D1B0. Aircraft517C60 requires its
    /// separate flight initialization and is explicitly outside this entry.
    pub fn init_load(&mut self, mind: MindId, arena: &mut Arena) -> Result<bool> {
        if installed(&self.actors, arena, mind)?.kind == Kind::Aircraft {
            return Err(fail(
                "Aircraft InitLoad517C60 requires flight initialization",
            ));
        }
        init_existing(mind, arena, self)
    }
    fn update_actor_state(&mut self, mind: MindId, arena: &mut Arena) -> Result<()> {
        let state = arena
            .mind(mind)?
            .stack
            .front()
            .copied()
            .flatten()
            .ok_or(Error::MissingActiveState)?;
        let binding = state_binding(&self.actors, arena, state)?;
        if binding.kind != Kind::Element {
            return self.services.sync_state_other(
                mind,
                binding.kind,
                &mut Context {
                    actors: &mut self.actors,
                    arena,
                    runtime: self.runtime,
                    queues: self.queues,
                    owners: self.owners,
                },
            );
        }
        let index = match binding.offset {
            0x34 => 12,
            0x48 | 0x7c => 8,
            0x68 => {
                if state_bits(&self.actors, binding.actor)? & (1u64 << 33) != 0 {
                    33
                } else {
                    12
                }
            }
            0xf8 | 0x12c => 4,
            0x164 => 7,
            0x178 => 9,
            _ => self.services.actor_state_other(
                state,
                binding,
                &mut Context {
                    actors: &mut self.actors,
                    arena,
                    runtime: self.runtime,
                    queues: self.queues,
                    owners: self.owners,
                },
            )?,
        };
        let actor = ActorId::from_raw(arena.mind(mind)?.actor);
        self.services.set_actor_state(
            actor,
            index,
            &mut Context {
                actors: &mut self.actors,
                arena,
                runtime: self.runtime,
                queues: self.queues,
                owners: self.owners,
            },
        )
    }
}

impl<S: Services> InitExistingWorld for World<'_, '_, S> {
    fn reset_default(&mut self, mind: MindId, arena: &mut Arena) -> Result<()> {
        aa_mind::World::reset_default(self, mind, arena)
    }
    fn update_actor_state(&mut self, mind: MindId, arena: &mut Arena) -> Result<()> {
        World::update_actor_state(self, mind, arena)
    }
}

impl<S: Services> aa_mind::World for World<'_, '_, S> {
    fn can_activate(&mut self, state: StateId, arena: &mut Arena) -> Result<bool> {
        let b = state_binding(&self.actors, arena, state)?;
        if b.kind != Kind::Element {
            return self.services.can_activate_other(
                state,
                b,
                &mut Context {
                    actors: &mut self.actors,
                    arena,
                    runtime: self.runtime,
                    queues: self.queues,
                    owners: self.owners,
                },
            );
        }
        if matches!(b.offset, 0x34 | 0x48) {
            return Ok(true);
        }
        let current = arena.mind(b.mind)?.stack.front().copied().flatten();
        let priority = if let Some(current) = current {
            let active = state_binding(&self.actors, arena, current)?;
            if active.mind != b.mind || active.kind != Kind::Element {
                return Err(fail("active state belongs to another Mind"));
            }
            element_priority(active.offset)?
        } else {
            0
        };
        // Sleep52DF50 compares the fixed value2; ordinary529FC0 compares the
        // candidate's original metadata+4. Status is not an activation gate.
        Ok((if b.offset == 0x68 {
            2
        } else {
            element_priority(b.offset)?
        }) >= priority)
    }
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        let b = state_binding(&self.actors, arena, state)?;
        if b.kind == Kind::Element {
            if matches!(b.offset, 0x34 | 0xf8 | 0x178) {
                arena.state_mut(state)?.status = 2; //5166C0
                return Ok(());
            }
            if b.offset == 0x12c {
                arena.state_mut(state)?.status = 2; //52D6C0
                let Some(RuntimeComponent::Mind(m)) = self
                    .actors
                    .component_mut(b.actor, ComponentSlot::Mind)
                    .map_err(fail)?
                else {
                    return Err(fail("canonical Mind missing"));
                };
                m.bytes.insert(0x144, 1);
                return Ok(());
            }
            if b.offset == 0x68 {
                arena.state_mut(state)?.status = 2; //52FC20
                if state_bits(&self.actors, b.actor)? & (1u64 << 33) == 0 {
                    self.services.default_stance(
                        b.actor,
                        &mut Context {
                            actors: &mut self.actors,
                            arena,
                            runtime: self.runtime,
                            queues: self.queues,
                            owners: self.owners,
                        },
                    )?;
                }
                return Ok(());
            }
        }
        self.services.enter_other(
            state,
            b,
            &mut Context {
                actors: &mut self.actors,
                arena,
                runtime: self.runtime,
                queues: self.queues,
                owners: self.owners,
            },
        )
    }
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        let b = state_binding(&self.actors, arena, state)?;
        if b.kind == Kind::Element && matches!(b.offset, 0x34 | 0x68) {
            arena.state_mut(state)?.status = 0; //4BDDA0
            return Ok(());
        }
        self.services.leave_other(
            state,
            b,
            &mut Context {
                actors: &mut self.actors,
                arena,
                runtime: self.runtime,
                queues: self.queues,
                owners: self.owners,
            },
        )
    }
    fn schedule(
        &mut self,
        mind: MindId,
        delay_bits: u32,
        data: u32,
        arena: &mut Arena,
    ) -> Result<()> {
        self.schedule(mind, delay_bits, data, arena)
    }
    fn reset_default(&mut self, mind: MindId, arena: &mut Arena) -> Result<()> {
        let kind = installed(&self.actors, arena, mind)?.kind;
        if kind != Kind::Element {
            return self.services.reset_other(
                mind,
                kind,
                &mut Context {
                    actors: &mut self.actors,
                    arena,
                    runtime: self.runtime,
                    queues: self.queues,
                    owners: self.owners,
                },
            );
        }
        reset_element(&mut ElementReset{world:self,arena,mind})
    }

    fn state_changed(&mut self, mind: MindId, arena: &mut Arena) -> Result<()> {
        match installed(&self.actors, arena, mind)?.kind {
            Kind::Element | Kind::Regiment => self.update_actor_state(mind, arena)?,
            Kind::Aircraft => {} //516AB0 calls only529D70.
            Kind::Base => {
                return Err(fail(
                    "abstract Mind state-changed callback is outside runtime domain",
                ));
            }
        }
        let actor = ActorId::from_raw(arena.mind(mind)?.actor);
        self.services.rebuild_actions(
            actor,
            &mut Context {
                actors: &mut self.actors,
                arena,
                runtime: self.runtime,
                queues: self.queues,
                owners: self.owners,
            },
        )
    }
}
impl<S: Services> command::World for World<'_, '_, S> {
    fn definition(&self, command: u32) -> Result<command::Definition> {
        self.runtime.commands.value(command)?.definition
            .ok_or_else(|| fail("saved command has a null ActorCommandData definition"))
    }
    fn trailing(&self, command: u32) -> Result<bool> {
        Ok(self.runtime.commands.value(command)?.trailing != 0)
    }
    fn clone_owned(&mut self, command: u32) -> Result<u32> {
        self.runtime.commands.clone_owned(command)
    }
    fn retain(&mut self, command: u32) -> Result<()> {
        self.runtime.commands.retain(command)
    }
    fn release(&mut self, command: u32) -> Result<()> {
        self.runtime.commands.release(command)
    }
    fn invoke(
        &mut self,
        mind: MindId,
        slot: u32,
        command: u32,
        arena: &mut Arena,
        command_context: &mut command::Context,
    ) -> Result<bool> {
        let concrete = installed(&self.actors, arena, mind)?;
        let element_stop =
            (concrete.kind == Kind::Element && slot == 0xd4).then(|| concrete.clone());
        let previous = self.runtime.dispatch_depth;
        self.runtime.dispatch_depth = command_context.depth;
        let result = if let Some(binding) = element_stop {
            // Only the installed immutable dispatch descriptor is snapshotted;
            // disable_command reads/mutates the one Arena and live actor fields.
            aa_mind::disable_command::apply(&binding, arena, self)
        } else {
            self.services.invoke_command(
                mind,
                slot,
                command,
                command_context,
                &mut Context {
                    actors: &mut self.actors,
                    arena,
                    runtime: self.runtime,
                    queues: self.queues,
                    owners: self.owners,
                },
            )
        };
        self.runtime.dispatch_depth = previous;
        result
    }
    fn schedule(
        &mut self,
        mind: MindId,
        delay_bits: u32,
        data: u32,
        arena: &mut Arena,
    ) -> Result<()> {
        self.schedule(mind, delay_bits, data, arena)
    }
    fn defers_rejected_command(&mut self, state: StateId, arena: &mut Arena) -> Result<bool> {
        let b = state_binding(&self.actors, arena, state)?;
        if b.kind == Kind::Element {
            element_priority(b.offset)?;
            return Ok(matches!(b.offset, 0x34 | 0x48));
        }
        self.services.defers_other(
            state,
            b,
            &mut Context {
                actors: &mut self.actors,
                arena,
                runtime: self.runtime,
                queues: self.queues,
                owners: self.owners,
            },
        )
    }
    fn deliver_stop(&mut self, actor: u32, arena: &mut Arena) -> Result<bool> {
        self.services.deliver_stop(
            ActorId::from_raw(actor),
            &mut Context {
                actors: &mut self.actors,
                arena,
                runtime: self.runtime,
                queues: self.queues,
                owners: self.owners,
            },
        )
    }
    fn actor_flags(&self, actor: u32) -> Result<u32> {
        self.services
            .actor_flags(ActorId::from_raw(actor), &self.actors)
    }
    fn has_live_group(&self, actor: u32) -> Result<bool> {
        self.services
            .has_live_group(ActorId::from_raw(actor), &self.actors)
    }
    fn finish_group_commands(&mut self, actor: u32, arena: &mut Arena) -> Result<()> {
        self.services.finish_group_commands(
            ActorId::from_raw(actor),
            &mut Context {
                actors: &mut self.actors,
                arena,
                runtime: self.runtime,
                queues: self.queues,
                owners: self.owners,
            },
        )
    }
}

impl<S: Services> aa_mind::disable_command::World for World<'_, '_, S> {
    fn element_leader(&self, actor: u32) -> Result<Option<u32>> {
        match self
            .actors
            .component(ActorId::from_raw(actor), ComponentSlot::Element)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Element(element)) => Ok(element.organization.map(ActorId::raw)),
            None => Ok(None),
            _ => Err(fail("Element slot has another component kind")),
        }
    }
    fn actor_facing(&self, actor: u32) -> Result<[u32; 2]> {
        self.services.facing(ActorId::from_raw(actor), &self.actors)
    }
    fn set_actor_facing(&mut self, actor: u32, facing: [u32; 2], arena: &mut Arena) -> Result<()> {
        self.services.set_facing(
            ActorId::from_raw(actor),
            facing,
            &mut Context {
                actors: &mut self.actors,
                arena,
                runtime: self.runtime,
                queues: self.queues,
                owners: self.owners,
            },
        )
    }
}
