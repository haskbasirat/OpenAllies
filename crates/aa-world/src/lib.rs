//! Native outer-dispatch/list laboratory, not a complete world simulation.
//!
//! Ordering is bound to the original x86 oracle in research/phase2-world-order.
//! Numeric slots are intentional: leaf gameplay and scheduler internals remain
//! external callbacks. No wall-clock, command cadence, save format or parallel
//! actor execution is inferred. Callback errors/panics are not transactional.

use aa_numeric::Precision;
use aa_simulation::NativeClock;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

pub mod initialization;
pub mod immediate_destroy;
pub mod queues;

pub type ObjectId = u64;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("world list allocation failed")]
    Allocation,
    #[error("world list node identity exhausted")]
    NodeIdentityExhausted,
    #[error("callback invalidated a native traversal cursor")]
    InvalidatedCursor,
    #[error("unknown object flags for {0}")]
    UnknownObject(ObjectId),
    #[error("pending100 owner {0} has no current component target")]
    MissingComponent(ObjectId),
    #[error("required world callback failed: {0}")]
    Callback(String),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListKind {
    Pending100,
    Removal108,
    Retained110,
    Class178,
    Class188,
    Deferred1A4,
}

#[derive(Clone, Copy, Debug)]
struct Node {
    id: u64,
    value: ObjectId,
}

/// Callback-entry traces; predicates carry the returned value. No allocation
/// addresses or native free() instrumentation enter this semantic trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub phase: u32,
    pub tick: u32,
    pub operation: Operation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    FogCallback {
        enabled: bool,
    },
    RequestPhase {
        requested: u32,
    },
    FirstTickCallback {
        aux: Option<u32>,
    },
    DeferredPayload {
        object: ObjectId,
    },
    QueueUpdate {
        queue: u8,
    },
    Manager13CUpdate,
    Pending100Component {
        component: ObjectId,
    },
    AvailabilityLeaf {
        object: ObjectId,
    },
    RegistryUnlink {
        object: ObjectId,
    },
    RetainedUnlink {
        object: ObjectId,
    },
    ClassificationRemove {
        list: ListKind,
        object: ObjectId,
    },
    Actor8C {
        object: ObjectId,
        result: bool,
    },
    Actor90 {
        object: ObjectId,
    },
    ActorBC {
        object: ObjectId,
        result: bool,
    },
    ActorC0 {
        object: ObjectId,
        result: bool,
    },
    ActorF0 {
        object: ObjectId,
        argument: bool,
    },
    ActorE0 {
        object: ObjectId,
        argument: bool,
    },
    /// Entry of the removal request 0x4F6760.
    RequestRemoval {
        object: ObjectId,
        destroy: bool,
    },
    ActorB4 {
        object: ObjectId,
        result: bool,
    },
    /// Generation-checked GTID table lookup of actor+0x158 (0xB364C0).
    ResolveHandle {
        object: ObjectId,
        result: Option<ObjectId>,
    },
    Actor2C {
        object: ObjectId,
        argument: u32,
    },
}

/// No implementation here substitutes for the original leaf routines. Callbacks
/// may enqueue work; traversal observes those mutations at the native boundaries.
/// A callback may call transition() recursively, but must not invalidate the
/// currently traversed node or an already cached successor. Such behavior was
/// not proven safe in the original and returns an error when detected here.
/// Every service is explicit. Errors stop at the failing boundary while
/// preserving prior effects; the host must not automatically retry that tick.
pub trait Callbacks {
    /// Actual native field/list owners can override these five operations. The
    /// defaults keep the standalone dispatcher laboratory's owned fields.
    fn fog_cheat_enabled(&mut self, world: &mut World) -> Result<bool> {
        Ok(world.svar_fog_cheat || world.world_fog_cheat)
    }
    fn increment_auxiliary_counter(&mut self, world: &mut World) -> Result<Option<u32>> {
        if let Some(value) = &mut world.auxiliary_counter {
            *value = value.wrapping_add(1);
        }
        Ok(world.auxiliary_counter)
    }
    fn pending_component_target(&mut self, world: &mut World, owner: ObjectId) -> Result<ObjectId> {
        world
            .pending100_targets
            .get(&owner)
            .copied()
            .ok_or(Error::MissingComponent(owner))
    }
    fn or_object_flags(&mut self, world: &mut World, object: ObjectId, bits: u32) -> Result<()> {
        *world
            .object_flags
            .get_mut(&object)
            .ok_or(Error::UnknownObject(object))? |= bits;
        Ok(())
    }
    fn remove_classification(
        &mut self,
        world: &mut World,
        list: ListKind,
        object: ObjectId,
    ) -> Result<()> {
        world.unlink_value(list, object);
        Ok(())
    }
    fn fog_callback(&mut self, world: &mut World, enabled: bool) -> Result<()>;
    fn first_tick_callback(&mut self, world: &mut World) -> Result<()>;
    fn deferred_payload(&mut self, world: &mut World, object: ObjectId) -> Result<()>;
    fn queue_update(&mut self, world: &mut World, queue: u8) -> Result<()>;
    fn manager_13c_update(&mut self, world: &mut World) -> Result<()>;
    fn pending_100_component(&mut self, world: &mut World, component: ObjectId) -> Result<()>;
    fn availability_leaf(&mut self, world: &mut World, object: ObjectId) -> Result<()>;
    fn registry_unlink(&mut self, world: &mut World, object: ObjectId) -> Result<()>;
    fn actor_8c(&mut self, world: &mut World, object: ObjectId) -> Result<bool>;
    fn actor_90(&mut self, world: &mut World, object: ObjectId) -> Result<()>;
    fn actor_bc(&mut self, world: &mut World, object: ObjectId) -> Result<bool>;
    fn actor_c0(&mut self, world: &mut World, object: ObjectId) -> Result<bool>;
    fn actor_f0(&mut self, world: &mut World, object: ObjectId, argument: bool) -> Result<()>;
    fn actor_e0(&mut self, world: &mut World, object: ObjectId, argument: bool) -> Result<()>;
    /// Actor virtual +0xB4, read by the immediate destroy path 0x4F5490.
    /// Only `request_removal` uses it; there is no default success.
    fn actor_b4(&mut self, _world: &mut World, _object: ObjectId) -> Result<bool> {
        Err(Error::Callback(
            "actor virtual +0xB4 is not provided".into(),
        ))
    }
    /// 0x4F5490: actor+0x158 handle (index low 16, generation high 16) looked
    /// up in the GTID table 0xB364C0; Some only for a matching live entry.
    fn resolve_handle(
        &mut self,
        _world: &mut World,
        _object: ObjectId,
    ) -> Result<Option<ObjectId>> {
        Err(Error::Callback(
            "GTID handle resolution is not provided".into(),
        ))
    }
    /// Actor virtual +0x2C, called with 3 by 0x4F5490.
    fn actor_2c(&mut self, _world: &mut World, _object: ObjectId, _argument: u32) -> Result<()> {
        Err(Error::Callback(
            "actor virtual +0x2C is not provided".into(),
        ))
    }
}

/// Move-only native4F7D00 phase/list owner during loading. It contains no
/// clock, so the saved clock stays with its single load owner until live bind.
pub struct DispatcherState {
    pub phase: u32,
    lists: [VecDeque<Node>; 6],
    next_node: u64,
    events: Vec<Event>,
}
impl DispatcherState {
    pub fn from_native_constructor() -> Self {
        Self {
            phase: 0,
            lists: std::array::from_fn(|_| VecDeque::new()),
            next_node: 0,
            events: Vec::new(),
        }
    }
}

pub struct World {
    pub phase: u32,
    pub clock: NativeClock,
    /// Explicit incoming x87 precision for4F9970's integer-times-step store.
    pub clock_precision: Precision,
    pub auxiliary_counter: Option<u32>,
    pub svar_fog_cheat: bool,
    pub world_fog_cheat: bool,
    /// Minimal opaque fields used by the dispatcher, not an actor registry.
    pub object_flags: BTreeMap<ObjectId, u32>,
    /// Native pending100 nodes contain owners whose +0x60 is read at dispatch.
    pub pending100_targets: BTreeMap<ObjectId, ObjectId>,
    lists: [VecDeque<Node>; 6],
    next_node: u64,
    events: Vec<Event>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            phase: 0,
            clock: NativeClock::stock(),
            clock_precision: Precision::Extended64,
            auxiliary_counter: None,
            svar_fog_cheat: false,
            world_fog_cheat: false,
            object_flags: BTreeMap::new(),
            pending100_targets: BTreeMap::new(),
            lists: std::array::from_fn(|_| VecDeque::new()),
            next_node: 0,
            events: Vec::new(),
        }
    }
}

impl ListKind {
    fn index(self) -> usize {
        match self {
            Self::Pending100 => 0,
            Self::Removal108 => 1,
            Self::Retained110 => 2,
            Self::Class178 => 3,
            Self::Class188 => 4,
            Self::Deferred1A4 => 5,
        }
    }
}

impl World {
    ///4F7D00's dispatcher/list constructor with the caller's actual clock.
    /// Live callbacks must supply the external fog/counter/actor/classification
    /// owners. No actor scan or copied classification/flag table is installed.
    pub fn from_native_constructor(clock: NativeClock, precision: Precision) -> Self {
        Self::from_loaded_dispatcher(DispatcherState::from_native_constructor(), clock, precision)
    }
    /// Lend the same phase/list/event owner back out as a DispatcherState for
    /// a synchronous load-shaped callee (fresh creation on a started World).
    /// The lists are moved, not copied; [`Self::restore_dispatcher`] must put
    /// the returned owner back before this World dispatches again.
    pub fn lend_dispatcher(&mut self) -> DispatcherState {
        DispatcherState {
            phase: self.phase,
            lists: std::mem::replace(&mut self.lists, std::array::from_fn(|_| VecDeque::new())),
            next_node: self.next_node,
            events: std::mem::take(&mut self.events),
        }
    }
    /// Return a lent owner, including any phase/list/event changes it made.
    pub fn restore_dispatcher(&mut self, state: DispatcherState) {
        let DispatcherState { phase, lists, next_node, events } = state;
        self.phase = phase;
        self.lists = lists;
        self.next_node = next_node;
        self.events = events;
    }
    /// Move the exact phase/list owner beside the one actual loaded clock.
    pub fn from_loaded_dispatcher(
        state: DispatcherState,
        clock: NativeClock,
        precision: Precision,
    ) -> Self {
        let DispatcherState {
            phase,
            lists,
            next_node,
            events,
        } = state;
        Self {
            phase,
            clock,
            clock_precision: precision,
            auxiliary_counter: None,
            svar_fog_cheat: false,
            world_fog_cheat: false,
            object_flags: BTreeMap::new(),
            pending100_targets: BTreeMap::new(),
            lists,
            next_node,
            events,
        }
    }
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    /// Consume the laboratory world to collect its trace. Requiring ownership
    /// prevents callbacks from draining a predicate event before it completes.
    pub fn into_events(self) -> Vec<Event> {
        self.events
    }
    pub fn list_values(&self, list: ListKind) -> Vec<ObjectId> {
        self.lists[list.index()].iter().map(|n| n.value).collect()
    }
    pub fn enqueue_back(&mut self, list: ListKind, object: ObjectId) -> Result<()> {
        self.insert(list, object, false)
    }
    pub fn enqueue_front(&mut self, list: ListKind, object: ObjectId) -> Result<()> {
        self.insert(list, object, true)
    }
    fn insert(&mut self, list: ListKind, value: ObjectId, front: bool) -> Result<()> {
        let next = self
            .next_node
            .checked_add(1)
            .ok_or(Error::NodeIdentityExhausted)?;
        let queue = &mut self.lists[list.index()];
        queue.try_reserve(1).map_err(|_| Error::Allocation)?;
        let node = Node {
            id: self.next_node,
            value,
        };
        if front {
            queue.push_front(node)
        } else {
            queue.push_back(node)
        }
        self.next_node = next;
        Ok(())
    }
    fn event(&mut self, operation: Operation) -> usize {
        let index = self.events.len();
        self.events.push(Event {
            phase: self.phase,
            tick: self.clock.tick,
            operation,
        });
        index
    }
    fn cursor(&self, list: ListKind, id: u64) -> Result<(ObjectId, Option<u64>)> {
        let queue = &self.lists[list.index()];
        let i = queue
            .iter()
            .position(|n| n.id == id)
            .ok_or(Error::InvalidatedCursor)?;
        Ok((queue[i].value, queue.get(i + 1).map(|n| n.id)))
    }
    fn unlink_node(&mut self, list: ListKind, id: u64) -> Result<()> {
        let queue = &mut self.lists[list.index()];
        let i = queue
            .iter()
            .position(|n| n.id == id)
            .ok_or(Error::InvalidatedCursor)?;
        queue.remove(i);
        Ok(())
    }
    fn unlink_value(&mut self, list: ListKind, object: ObjectId) {
        let queue = &mut self.lists[list.index()];
        if let Some(i) = queue.iter().position(|n| n.value == object) {
            queue.remove(i);
        }
    }
    /// Default laboratory storage for native4F1430's first class-list match.
    /// Live callbacks remove from their actual registration owner instead.
    pub fn remove_classification_entry(&mut self, list: ListKind, object: ObjectId) -> Result<()> {
        if !matches!(list, ListKind::Class178 | ListKind::Class188) {
            return Err(Error::Callback(
                "classification removal requires World+178/+188".into(),
            ));
        }
        self.unlink_value(list, object);
        Ok(())
    }
    fn retain<C: Callbacks + ?Sized>(
        &mut self,
        object: ObjectId,
        callbacks: &mut C,
    ) -> Result<bool> {
        let event = self.event(Operation::Actor8C {
            object,
            result: false,
        });
        let result = callbacks.actor_8c(self, object)?;
        self.events[event].operation = Operation::Actor8C { object, result };
        Ok(result)
    }
    fn availability_and_classification<C: Callbacks + ?Sized>(
        &mut self,
        object: ObjectId,
        callbacks: &mut C,
    ) -> Result<()> {
        self.event(Operation::AvailabilityLeaf { object });
        callbacks.availability_leaf(self, object)?;
        let event = self.event(Operation::ActorBC {
            object,
            result: false,
        });
        let bc = callbacks.actor_bc(self, object)?;
        self.events[event].operation = Operation::ActorBC { object, result: bc };
        let list = if bc {
            Some(ListKind::Class188)
        } else {
            let event = self.event(Operation::ActorC0 {
                object,
                result: false,
            });
            let c0 = callbacks.actor_c0(self, object)?;
            self.events[event].operation = Operation::ActorC0 { object, result: c0 };
            c0.then_some(ListKind::Class178)
        };
        if let Some(list) = list {
            self.event(Operation::ClassificationRemove { list, object });
            callbacks.remove_classification(self, list, object)?;
        }
        Ok(())
    }
    fn finalize<C: Callbacks + ?Sized>(
        &mut self,
        object: ObjectId,
        callbacks: &mut C,
    ) -> Result<()> {
        self.event(Operation::ActorF0 {
            object,
            argument: false,
        });
        callbacks.actor_f0(self, object, false)?;
        self.event(Operation::RegistryUnlink { object });
        callbacks.registry_unlink(self, object)?;
        self.event(Operation::ActorE0 {
            object,
            argument: true,
        });
        callbacks.actor_e0(self, object, true)
    }
    /// Exact outer phase transition 0x4F65B0, with external gameplay leaves.
    /// Does not implement the separate native request/dedup API at0x4F6760.
    pub fn transition<C: Callbacks + ?Sized>(
        &mut self,
        requested: u32,
        callbacks: &mut C,
    ) -> Result<()> {
        self.event(Operation::RequestPhase { requested });
        if self.phase == requested {
            return Ok(());
        }
        if matches!(self.phase, 1 | 4) {
            self.phase = 2;
            while let Some(node) = self.lists[ListKind::Pending100.index()].pop_front() {
                let component = callbacks.pending_component_target(self, node.value)?;
                self.event(Operation::Pending100Component { component });
                callbacks.pending_100_component(self, component)?;
            }
        }
        self.phase = 3;
        while let Some(node) = self.lists[ListKind::Removal108.index()].pop_front() {
            self.remove_now(node.value, callbacks)?;
        }
        self.phase = requested;
        Ok(())
    }
    /// 0x4F53D0(object, false): the per-object body of the +108 drain, also
    /// used directly by an immediate removal request.
    fn remove_now<C: Callbacks + ?Sized>(
        &mut self,
        object: ObjectId,
        callbacks: &mut C,
    ) -> Result<()> {
        self.availability_and_classification(object, callbacks)?;
        if self.phase != 6 {
            self.event(Operation::Actor90 { object });
            callbacks.actor_90(self, object)?;
            if self.retain(object, callbacks)? {
                self.event(Operation::ActorF0 {
                    object,
                    argument: true,
                });
                callbacks.actor_f0(self, object, true)?;
                callbacks.or_object_flags(self, object, 0x81)?;
                return self.enqueue_front(ListKind::Retained110, object);
            }
        }
        self.finalize(object, callbacks)
    }
    /// 0x4F5490: immediate destroy. A +0xB4 actor whose handle resolves acts
    /// on the resolved object instead of itself.
    fn destroy_now<C: Callbacks + ?Sized>(
        &mut self,
        object: ObjectId,
        callbacks: &mut C,
    ) -> Result<()> {
        immediate_destroy::run(&mut ImmediateCallbacks { world: self, callbacks }, object)
    }
    /// Native removal request 0x4F6760 (`session_mode` is session+0xE4).
    /// An object already on +108 is ignored (false, no flag change). `destroy`
    /// sets flag 2 first. Outside phases 3/6 (and phase 0 of mode 3) the
    /// object is PREPENDED to +108 (0x542A00), so the transition drains the
    /// newest request first; returns false. Otherwise it is removed now
    /// (0x4F53D0) or destroyed now (0x4F5490) and true is returned.
    pub fn request_removal<C: Callbacks + ?Sized>(
        &mut self,
        object: ObjectId,
        destroy: bool,
        session_mode: i32,
        callbacks: &mut C,
    ) -> Result<bool> {
        self.event(Operation::RequestRemoval { object, destroy });
        if self.lists[ListKind::Removal108.index()]
            .iter()
            .any(|n| n.value == object)
        {
            return Ok(false);
        }
        if destroy {
            callbacks.or_object_flags(self, object, 2)?;
        }
        let phase = self.phase;
        if (phase != 0 || session_mode != 3) && phase != 3 && phase != 6 {
            self.enqueue_front(ListKind::Removal108, object)?;
            return Ok(false);
        }
        if destroy {
            self.destroy_now(object, callbacks)?;
        } else {
            self.remove_now(object, callbacks)?;
        }
        Ok(true)
    }
    /// Advance one explicitly requested world tick. The native admin/decree
    /// boundary is separate and must not be inferred from render time or batch count.
    pub fn step<C: Callbacks + ?Sized>(&mut self, callbacks: &mut C) -> Result<()> {
        let enabled = callbacks.fog_cheat_enabled(self)?;
        self.event(Operation::FogCallback { enabled });
        callbacks.fog_callback(self, enabled)?;
        self.transition(1, callbacks)?;
        let auxiliary_counter = callbacks.increment_auxiliary_counter(self)?;
        if self.clock.tick == 0 {
            self.event(Operation::FirstTickCallback {
                aux: auxiliary_counter,
            });
            callbacks.first_tick_callback(self)?;
        }
        self.clock
            .advance_fragment_at_precision(self.clock_precision)?;
        let mut cursor = self.lists[ListKind::Deferred1A4.index()]
            .front()
            .map(|n| n.id);
        while let Some(id) = cursor {
            let (object, _) = self.cursor(ListKind::Deferred1A4, id)?;
            self.event(Operation::DeferredPayload { object });
            callbacks.deferred_payload(self, object)?;
            // Read next after the callback: newly linked tail nodes are visited.
            cursor = self.cursor(ListKind::Deferred1A4, id)?.1;
        }
        self.lists[ListKind::Deferred1A4.index()].clear();
        for queue in 0..4 {
            self.event(Operation::QueueUpdate { queue });
            callbacks.queue_update(self, queue)?;
        }
        self.transition(4, callbacks)?;
        self.event(Operation::Manager13CUpdate);
        callbacks.manager_13c_update(self)?;
        self.transition(5, callbacks)?;
        self.event(Operation::QueueUpdate { queue: 4 });
        callbacks.queue_update(self, 4)?;
        self.transition(0, callbacks)?;
        let mut cursor = self.lists[ListKind::Retained110.index()]
            .front()
            .map(|n| n.id);
        while let Some(id) = cursor {
            let (object, _) = self.cursor(ListKind::Retained110, id)?;
            let retained = self.retain(object, callbacks)?;
            // Native takes next after predicate, before deletion callbacks.
            let next = self.cursor(ListKind::Retained110, id)?.1;
            if !retained {
                self.event(Operation::RetainedUnlink { object });
                self.unlink_node(ListKind::Retained110, id)?;
                self.event(Operation::RegistryUnlink { object });
                callbacks.registry_unlink(self, object)?;
                self.availability_and_classification(object, callbacks)?;
                self.finalize(object, callbacks)?;
            }
            cursor = next;
        }
        Ok(())
    }
}

struct ImmediateCallbacks<'a, C: ?Sized> {
    world: &'a mut World,
    callbacks: &'a mut C,
}
impl<C: Callbacks + ?Sized> immediate_destroy::World for ImmediateCallbacks<'_, C> {
    type Object = ObjectId;
    type Error = Error;
    fn actor_b4(&mut self, object: ObjectId) -> Result<bool> {
        let event = self.world.event(Operation::ActorB4 { object, result: false });
        let result = self.callbacks.actor_b4(self.world, object)?;
        self.world.events[event].operation = Operation::ActorB4 { object, result };
        Ok(result)
    }
    fn resolve_handle(&mut self, object: ObjectId) -> Result<Option<ObjectId>> {
        let event = self.world.event(Operation::ResolveHandle { object, result: None });
        let result = self.callbacks.resolve_handle(self.world, object)?;
        self.world.events[event].operation = Operation::ResolveHandle { object, result };
        Ok(result)
    }
    fn actor_2c(&mut self, object: ObjectId, reason: u32) -> Result<()> {
        self.world.event(Operation::Actor2C { object, argument: reason });
        self.callbacks.actor_2c(self.world, object, reason)
    }
    fn or_object_flags(&mut self, object: ObjectId, flags: u32) -> Result<()> {
        self.callbacks.or_object_flags(self.world, object, flags)
    }
    fn availability_and_classification(&mut self, object: ObjectId) -> Result<()> {
        self.world.availability_and_classification(object, self.callbacks)
    }
    fn finalize(&mut self, object: ObjectId) -> Result<()> {
        self.world.finalize(object, self.callbacks)
    }
}
