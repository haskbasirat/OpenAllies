use aa_numeric::Precision;
use aa_simulation::{NativeClock, NativeRng};
use aa_world::{
    ObjectId, World,
    initialization::ClockPlan,
    queues::{Clock, Delivery, Queues},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Initialization(#[from] aa_world::initialization::Error),
    #[error(transparent)]
    World(#[from] aa_world::Error),
    #[error(
        "world is stopped after a partially committed operation; restore or recreate it before continuing"
    )]
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Fog(bool),
    FirstTick,
    DeferredPayload(ObjectId),
    Manager,
    PendingComponent(ObjectId),
    Availability(ObjectId),
    RegistryUnlink(ObjectId),
    Actor90(ObjectId),
    ActorF0(ObjectId, bool),
    ActorE0(ObjectId, bool),
    /// Actor virtual +0x2C with its argument (0x4F5490 passes 3).
    Actor2C(ObjectId, u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Predicate {
    Retain,
    ClassBC,
    ClassC0,
    /// Actor virtual +0xB4 (0x4F5490 immediate destroy).
    ClassB4,
}

/// Actual implementations own registry/component data. Every leaf is required;
/// returning success for an unimplemented effect is not a valid implementation.
/// Only the runtime/admin owns world time. A service may mutate gameplay lists,
/// actors and schedules, but must not change either clock or precision. The
/// adapter checks this on return and stops the world on violation; effects
/// already produced by that invalid service cannot be rolled back.
pub trait Services {
    /// The laboratory defaults read its own fields. A live implementation must
    /// override all five methods with the same persistent native owners.
    fn fog_cheat_enabled(&mut self, world: &mut World) -> aa_world::Result<bool> {
        Ok(world.svar_fog_cheat || world.world_fog_cheat)
    }
    fn increment_auxiliary_counter(&mut self, world: &mut World) -> aa_world::Result<Option<u32>> {
        if let Some(value) = &mut world.auxiliary_counter {
            *value = value.wrapping_add(1);
        }
        Ok(world.auxiliary_counter)
    }
    fn pending_component_target(
        &mut self,
        world: &mut World,
        owner: ObjectId,
    ) -> aa_world::Result<ObjectId> {
        world
            .pending100_targets
            .get(&owner)
            .copied()
            .ok_or(aa_world::Error::MissingComponent(owner))
    }
    fn or_object_flags(
        &mut self,
        world: &mut World,
        object: ObjectId,
        bits: u32,
    ) -> aa_world::Result<()> {
        *world
            .object_flags
            .get_mut(&object)
            .ok_or(aa_world::Error::UnknownObject(object))? |= bits;
        Ok(())
    }
    fn remove_classification(
        &mut self,
        world: &mut World,
        list: aa_world::ListKind,
        object: ObjectId,
    ) -> aa_world::Result<()> {
        world.remove_classification_entry(list, object)
    }
    fn effect(
        &mut self,
        effect: Effect,
        world: &mut World,
        queues: &mut Queues,
        rng: &mut NativeRng,
    ) -> aa_world::Result<()>;
    fn predicate(
        &mut self,
        predicate: Predicate,
        object: ObjectId,
        world: &mut World,
        queues: &mut Queues,
        rng: &mut NativeRng,
    ) -> aa_world::Result<bool>;
    fn update_component(
        &mut self,
        event: Delivery,
        world: &mut World,
        queues: &mut Queues,
        rng: &mut NativeRng,
    ) -> aa_world::queues::Result<()>;
    /// 0x4F5490 GTID lookup of actor+0x158 (table 0xB364C0). Only removal
    /// requests use it; an implementation is required before destroying.
    fn resolve_handle(
        &mut self,
        _object: ObjectId,
        _world: &mut World,
        _queues: &mut Queues,
        _rng: &mut NativeRng,
    ) -> aa_world::Result<Option<ObjectId>> {
        Err(aa_world::Error::Callback(
            "runtime GTID handle resolution is not provided".into(),
        ))
    }
}

/// Native removal request 0x4F6760 issued from inside a service callback
/// (deaths and removals are requested while queue deliveries run). See
/// `aa_world::World::request_removal`; leaves dispatch through `services`.
pub fn request_removal<S: Services>(
    services: &mut S,
    world: &mut World,
    queues: &mut Queues,
    rng: &mut NativeRng,
    object: ObjectId,
    destroy: bool,
    session_mode: i32,
) -> aa_world::Result<bool> {
    let mut dispatch = Dispatch {
        queues,
        rng,
        services,
    };
    world.request_removal(object, destroy, session_mode, &mut dispatch)
}

pub struct Runtime<S> {
    pub world: World,
    pub queues: Queues,
    pub rng: NativeRng,
    pub services: S,
    stopped: bool,
}
/// The permanent live clock/queues/RNG/failure latch, separate from the actor
/// owners borrowed by Services for one frame. This avoids a self-referential
/// battle struct and never rebuilds the latch when a frame borrows services.
pub struct ClockedState {
    pub world: World,
    pub queues: Queues,
    pub rng: NativeRng,
    stopped: bool,
}
pub struct StateParts {
    pub world: World,
    pub queues: Queues,
    pub rng: NativeRng,
}
pub struct StateBindFailure {
    pub reason: &'static str,
    pub parts: StateParts,
}
/// One frame's loan; the permanent state and failure latch stay in the battle.
pub struct BorrowedState<'a, S> {
    state: &'a mut ClockedState,
    services: &'a mut S,
}
impl<S: Services> crate::simulation::WorldSeams for BorrowedState<'_, S> {
    fn world_clock(&self) -> Option<NativeClock> {
        Some(self.state.world.clock)
    }
    fn world_update(&mut self) -> crate::simulation::Result<NativeClock> {
        Ok(self.state.step(self.services)?)
    }
}
impl std::fmt::Debug for StateBindFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateBindFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}
impl ClockedState {
    pub fn from_parts(parts: StateParts) -> Result<Self, StateBindFailure> {
        if let Some(reason) = binding_error(&parts.world, &parts.queues) {
            return Err(StateBindFailure { reason, parts });
        }
        Ok(Self {
            world: parts.world,
            queues: parts.queues,
            rng: parts.rng,
            stopped: false,
        })
    }
    pub fn stopped(&self) -> bool {
        self.stopped
    }
    pub fn borrow<'a, S: Services>(&'a mut self, services: &'a mut S) -> BorrowedState<'a, S> {
        BorrowedState {
            state: self,
            services,
        }
    }
    pub fn step<S: Services>(&mut self, services: &mut S) -> Result<NativeClock, Error> {
        step_existing(
            &mut self.world,
            &mut self.queues,
            &mut self.rng,
            services,
            &mut self.stopped,
        )
    }
}
fn binding_error(world: &World, queues: &Queues) -> Option<&'static str> {
    let expected = Clock {
        begin_bits: world.clock.begin.0,
        end_bits: world.clock.end.0,
        step_bits: world.clock.step.0,
    };
    (queues.precision != world.clock_precision || queues.clock != expected)
        .then_some("existing native world and queue clock/precision differ")
}
fn step_existing<S: Services>(
    world: &mut World,
    queues: &mut Queues,
    rng: &mut NativeRng,
    services: &mut S,
    stopped: &mut bool,
) -> Result<NativeClock, Error> {
    if *stopped {
        return Err(Error::Stopped);
    }
    *stopped = true;
    world.step(&mut Dispatch {
        queues,
        rng,
        services,
    })?;
    *stopped = false;
    Ok(world.clock)
}
/// Owned parts of an existing native world. No Clone or extraction method can
/// accidentally replace its queue nodes or restart a stopped Runtime.
pub struct Parts<S> {
    pub world: World,
    pub queues: Queues,
    pub rng: NativeRng,
    pub services: S,
}
/// A failed binding returns every original owner, without dropping its prefix.
pub struct BindFailure<S> {
    pub reason: &'static str,
    pub parts: Parts<S>,
}
impl<S> std::fmt::Debug for BindFailure<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}
impl<S: Services> Runtime<S> {
    /// Move the already constructed/loaded world and its exact queues/RNG.
    /// This checks clock coherence only; map startup/readiness remains required.
    pub fn from_parts(parts: Parts<S>) -> Result<Self, BindFailure<S>> {
        if let Some(reason) = binding_error(&parts.world, &parts.queues) {
            return Err(BindFailure { reason, parts });
        }
        Ok(Self {
            world: parts.world,
            queues: parts.queues,
            rng: parts.rng,
            services: parts.services,
            stopped: false,
        })
    }
    /// Initial queue/clock state only. World actors/managers/ownership must be
    /// initialized through actual native services before the first step.
    pub fn new(
        plan: &ClockPlan,
        rng: NativeRng,
        services: S,
        precision: Precision,
    ) -> Result<Self, Error> {
        let clock = plan.new_clock();
        let queues = plan.initialize_queues(clock, precision)?;
        let mut world = World::default();
        world.clock = clock;
        world.clock_precision = precision;
        Ok(Self {
            world,
            queues,
            rng,
            services,
            stopped: false,
        })
    }
    pub fn stopped(&self) -> bool {
        self.stopped
    }
    /// Move the same owners and failure latch out of the legacy owned-service
    /// wrapper. Dropping the returned service releases any external loans.
    pub fn into_state(self) -> (ClockedState, S) {
        (
            ClockedState {
                world: self.world,
                queues: self.queues,
                rng: self.rng,
                stopped: self.stopped,
            },
            self.services,
        )
    }
    /// No render-time delta enters simulation. The native command/admin layer
    /// must authorize each call, retaining any debt when presentation stalls.
    pub fn step(&mut self) -> Result<NativeClock, Error> {
        step_existing(
            &mut self.world,
            &mut self.queues,
            &mut self.rng,
            &mut self.services,
            &mut self.stopped,
        )
    }
}

struct Dispatch<'a, S> {
    queues: &'a mut Queues,
    rng: &'a mut NativeRng,
    services: &'a mut S,
}
impl<S: Services> Dispatch<'_, S> {
    fn synchronize_clock(&mut self, w: &World) {
        self.queues.clock = Clock {
            begin_bits: w.clock.begin.0,
            end_bits: w.clock.end.0,
            step_bits: w.clock.step.0,
        };
    }
    fn effect(&mut self, w: &mut World, e: Effect) -> aa_world::Result<()> {
        self.synchronize_clock(w);
        let before = w.clock;
        let precision = w.clock_precision;
        let result = self.services.effect(e, w, self.queues, self.rng);
        stable_clock(before, precision, w, self.queues).map_err(aa_world::Error::Callback)?;
        result
    }
    fn predicate(&mut self, w: &mut World, p: Predicate, id: ObjectId) -> aa_world::Result<bool> {
        self.synchronize_clock(w);
        let before = w.clock;
        let precision = w.clock_precision;
        let result = self.services.predicate(p, id, w, self.queues, self.rng);
        stable_clock(before, precision, w, self.queues).map_err(aa_world::Error::Callback)?;
        result
    }
}
struct QueueDispatch<'a, S> {
    world: &'a mut World,
    rng: &'a mut NativeRng,
    services: &'a mut S,
}
impl<S: Services> aa_world::queues::Callbacks for QueueDispatch<'_, S> {
    fn deliver(&mut self, q: &mut Queues, event: Delivery) -> aa_world::queues::Result<()> {
        let before = self.world.clock;
        let precision = self.world.clock_precision;
        let result = self
            .services
            .update_component(event, self.world, q, self.rng);
        stable_clock(before, precision, self.world, q)
            .map_err(aa_world::queues::Error::Callback)?;
        result
    }
}
fn stable_clock(
    before: NativeClock,
    precision: Precision,
    world: &World,
    queues: &Queues,
) -> Result<(), String> {
    if world.clock != before
        || world.clock_precision != precision
        || queues.precision != precision
        || queues.clock
            != (Clock {
                begin_bits: before.begin.0,
                end_bits: before.end.0,
                step_bits: before.step.0,
            })
    {
        return Err("runtime service changed the authoritative world/queue clock".into());
    }
    Ok(())
}
impl<S: Services> aa_world::Callbacks for Dispatch<'_, S> {
    fn fog_cheat_enabled(&mut self, world: &mut World) -> aa_world::Result<bool> {
        self.synchronize_clock(world);
        let before = world.clock;
        let precision = world.clock_precision;
        let result = self.services.fog_cheat_enabled(world);
        stable_clock(before, precision, world, self.queues).map_err(aa_world::Error::Callback)?;
        result
    }
    fn increment_auxiliary_counter(&mut self, world: &mut World) -> aa_world::Result<Option<u32>> {
        self.synchronize_clock(world);
        let before = world.clock;
        let precision = world.clock_precision;
        let result = self.services.increment_auxiliary_counter(world);
        stable_clock(before, precision, world, self.queues).map_err(aa_world::Error::Callback)?;
        result
    }
    fn pending_component_target(
        &mut self,
        world: &mut World,
        owner: ObjectId,
    ) -> aa_world::Result<ObjectId> {
        self.synchronize_clock(world);
        let before = world.clock;
        let precision = world.clock_precision;
        let result = self.services.pending_component_target(world, owner);
        stable_clock(before, precision, world, self.queues).map_err(aa_world::Error::Callback)?;
        result
    }
    fn or_object_flags(
        &mut self,
        world: &mut World,
        object: ObjectId,
        bits: u32,
    ) -> aa_world::Result<()> {
        self.synchronize_clock(world);
        let before = world.clock;
        let precision = world.clock_precision;
        let result = self.services.or_object_flags(world, object, bits);
        stable_clock(before, precision, world, self.queues).map_err(aa_world::Error::Callback)?;
        result
    }
    fn remove_classification(
        &mut self,
        world: &mut World,
        list: aa_world::ListKind,
        object: ObjectId,
    ) -> aa_world::Result<()> {
        self.synchronize_clock(world);
        let before = world.clock;
        let precision = world.clock_precision;
        let result = self.services.remove_classification(world, list, object);
        stable_clock(before, precision, world, self.queues).map_err(aa_world::Error::Callback)?;
        result
    }
    fn fog_callback(&mut self, w: &mut World, enabled: bool) -> aa_world::Result<()> {
        self.effect(w, Effect::Fog(enabled))
    }
    fn first_tick_callback(&mut self, w: &mut World) -> aa_world::Result<()> {
        self.effect(w, Effect::FirstTick)
    }
    fn deferred_payload(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<()> {
        self.effect(w, Effect::DeferredPayload(id))
    }
    fn queue_update(&mut self, w: &mut World, id: u8) -> aa_world::Result<()> {
        self.queues.clock = Clock {
            begin_bits: w.clock.begin.0,
            end_bits: w.clock.end.0,
            step_bits: w.clock.step.0,
        };
        let mut dispatch = QueueDispatch {
            world: w,
            rng: self.rng,
            services: self.services,
        };
        self.queues
            .update(id, &mut dispatch)
            .map_err(|e| aa_world::Error::Callback(e.to_string()))
    }
    fn manager_13c_update(&mut self, w: &mut World) -> aa_world::Result<()> {
        self.effect(w, Effect::Manager)
    }
    fn pending_100_component(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<()> {
        self.effect(w, Effect::PendingComponent(id))
    }
    fn availability_leaf(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<()> {
        self.effect(w, Effect::Availability(id))
    }
    fn registry_unlink(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<()> {
        self.effect(w, Effect::RegistryUnlink(id))
    }
    fn actor_8c(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        self.predicate(w, Predicate::Retain, id)
    }
    fn actor_90(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<()> {
        self.effect(w, Effect::Actor90(id))
    }
    fn actor_bc(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        self.predicate(w, Predicate::ClassBC, id)
    }
    fn actor_c0(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        self.predicate(w, Predicate::ClassC0, id)
    }
    fn actor_f0(&mut self, w: &mut World, id: ObjectId, value: bool) -> aa_world::Result<()> {
        self.effect(w, Effect::ActorF0(id, value))
    }
    fn actor_e0(&mut self, w: &mut World, id: ObjectId, value: bool) -> aa_world::Result<()> {
        self.effect(w, Effect::ActorE0(id, value))
    }
    fn actor_b4(&mut self, w: &mut World, id: ObjectId) -> aa_world::Result<bool> {
        self.predicate(w, Predicate::ClassB4, id)
    }
    fn actor_2c(&mut self, w: &mut World, id: ObjectId, argument: u32) -> aa_world::Result<()> {
        self.effect(w, Effect::Actor2C(id, argument))
    }
    fn resolve_handle(
        &mut self,
        w: &mut World,
        id: ObjectId,
    ) -> aa_world::Result<Option<ObjectId>> {
        self.synchronize_clock(w);
        let before = w.clock;
        let precision = w.clock_precision;
        let result = self.services.resolve_handle(id, w, self.queues, self.rng);
        stable_clock(before, precision, w, self.queues).map_err(aa_world::Error::Callback)?;
        result
    }
}
