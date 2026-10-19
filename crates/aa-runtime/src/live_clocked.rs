//! The native world dispatcher over the same loaded actor, queue and source
//! owners. This binds existing kernels; it is not a readiness certificate.
//! Unported lifecycle leaves stop at their original boundary after its prefix.
use crate::{
    actor_world::{ActorStore, ObjectKey},
    clocked::{self, Effect, Predicate},
    live_registration,
    scheduler_owners::{Owners, Target},
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_orders::net::sync::Synchronizer;
use aa_simulation::NativeRng;
use aa_triggers::{manager::Manager, program::CoreProgram};
use aa_world::{
    ListKind, ObjectId, World,
    queues::{Delivery, Queues},
};
use std::cell::RefCell;

/// World+4 is a pointer, distinct from an unbound owner or a real zero counter.
pub enum Counter<'a> {
    Unbound,
    Null,
    Present(&'a mut u32),
}
/// Borrow native byte payloads without creating a second computed bool owner.
/// Native IsFogCheatEnabled tests each byte against zero;41C670 can retain128.
pub trait BoolSource {
    fn enabled(&self) -> bool;
}
impl BoolSource for u8 {
    fn enabled(&self) -> bool { *self != 0 }
}
impl BoolSource for bool {
    fn enabled(&self) -> bool { *self }
}
pub struct Fields<'a> {
    /// Actual SVarManager+1BC and World+275, retained outside dispatcher lab data.
    pub svar_fog_cheat: &'a dyn BoolSource,
    pub world_fog_cheat: &'a dyn BoolSource,
    pub auxiliary: Counter<'a>,
}
/// The same process Synchronizer A/B3655C. The actual thread/application gate
/// is evaluated only after its enabled and suspend checks have passed.
pub struct SyncLog<'a> {
    pub owner: &'a mut Synchronizer,
    pub gate: &'a mut dyn FnMut() -> Result<bool, String>,
}
/// The direct API is retained for a standalone owner. A live frame can instead
/// loan the one retained SyncA cell to both NetworkSeams and world callbacks.
pub enum SyncBinding<'a> {
    Direct(SyncLog<'a>),
    Shared {
        owner: &'a RefCell<Synchronizer>,
        gate: &'a mut dyn FnMut() -> Result<bool, String>,
    },
}
impl<'a> From<SyncLog<'a>> for SyncBinding<'a> {
    fn from(log: SyncLog<'a>) -> Self {
        Self::Direct(log)
    }
}
impl SyncBinding<'_> {
    /// Loan the same synchronizer and lazy gate for a shorter resource-load
    /// scope. Neither the log owner nor its runtime gate is duplicated.
    pub fn reborrow(&mut self)->SyncBinding<'_> {
        match self {
            Self::Direct(log)=>SyncBinding::Direct(SyncLog {owner:&mut *log.owner,gate:&mut *log.gate}),
            Self::Shared {owner,gate}=>SyncBinding::Shared {owner,gate:&mut **gate},
        }
    }
    pub fn direct_owner(&self) -> Option<&Synchronizer> {
        match self {
            Self::Direct(log) => Some(log.owner),
            Self::Shared { .. } => None,
        }
    }
    fn log_fog(&mut self, enabled: bool) -> aa_world::Result<()> {
        fn log(
            owner: &mut Synchronizer,
            gate: &mut dyn FnMut() -> Result<bool, String>,
            enabled: bool,
        ) -> aa_world::Result<()> {
            let open = if owner.enabled && owner.suspend < 1 {
                gate().map_err(fail)?
            } else {
                false
            };
            owner.log_bool(open, "IsFogCheatEnabled()", enabled);
            Ok(())
        }
        match self {
            Self::Direct(direct) => log(direct.owner, direct.gate, enabled),
            Self::Shared { owner, gate } => {
                let mut borrowed = owner
                    .try_borrow_mut()
                    .map_err(|e| fail(format!("same SyncA callback loan: {e}")))?;
                log(&mut borrowed, *gate, enabled)
            }
        }
    }
}
/// Native4F96D0's actual Player/Kingdom/Nation or observer music/audio owner.
/// It is invoked only on the first tick, without replacement time or RNG.
pub trait FirstTick {
    fn run(&mut self, actors: &ActorStore) -> Result<(), String>;
}
pub struct UnboundFirstTick;
impl FirstTick for UnboundFirstTick {
    fn run(&mut self, _: &ActorStore) -> Result<(), String> {
        Err("required native4F96D0 local Player+28/music audio owner is not bound".into())
    }
}

pub struct Services<'a> {
    pub actors: &'a mut ActorStore,
    pub owners: &'a Owners,
    pub spatial: &'a mut SpatialWorld,
    pub registration: &'a mut live_registration::State,
    pub fields: Fields<'a>,
    pub sync: SyncBinding<'a>,
    /// Actual World+13C loaded manager and its persistent evaluator diagnostics.
    pub triggers: &'a mut Manager,
    pub trigger_program: &'a mut CoreProgram,
    pub first_tick: &'a mut dyn FirstTick,
    /// World+4C Sun owner for bound world trigger opcodes; None keeps their
    /// explicit missing-service error.
    pub sun: Option<crate::live_updates::trigger_program::SunServices<'a>>,
}
fn fail(error: impl std::fmt::Display) -> aa_world::Error {
    aa_world::Error::Callback(format!("live native world: {error}"))
}
fn required(leaf: impl std::fmt::Display) -> aa_world::Error {
    fail(format!(
        "required {leaf} is not bound; preceding effects remain committed"
    ))
}
impl Services<'_> {
    fn key(&self, object: ObjectId) -> aa_world::Result<ObjectKey> {
        self.actors
            .checked_key(object)
            .ok_or_else(|| fail("current object arena identity is absent"))
    }
    fn registered_id(&self, object: ObjectId) -> aa_world::Result<ActorId> {
        self.actors
            .id(self.key(object)?)
            .ok_or_else(|| fail("object has no current GTID"))
    }
}
impl clocked::Services for Services<'_> {
    fn fog_cheat_enabled(&mut self, _: &mut World) -> aa_world::Result<bool> {
        Ok(self.fields.svar_fog_cheat.enabled() || self.fields.world_fog_cheat.enabled())
    }
    fn increment_auxiliary_counter(&mut self, _: &mut World) -> aa_world::Result<Option<u32>> {
        match &mut self.fields.auxiliary {
            Counter::Unbound => Err(required("World+4 auxiliary pointer")),
            Counter::Null => Ok(None),
            Counter::Present(value) => {
                **value = value.wrapping_add(1);
                Ok(Some(**value))
            }
        }
    }
    fn pending_component_target(
        &mut self,
        _: &mut World,
        owner: ObjectId,
    ) -> aa_world::Result<ObjectId> {
        let key = self.key(owner)?;
        let id = self.registered_id(owner)?;
        match self
            .actors
            .component(id, ComponentSlot::Body)
            .map_err(fail)?
        {
            Some(crate::actor_world::RuntimeComponent::Body(body)) if body.header.owner == id => {}
            Some(_) => return Err(fail("pending100 Body slot/class/owner differs")),
            None => return Err(aa_world::Error::MissingComponent(owner)),
        }
        self.owners
            .get(Target::Component {
                object: key,
                slot: ComponentSlot::Body as u8,
            })
            .map_err(fail)
    }
    fn or_object_flags(
        &mut self,
        _: &mut World,
        object: ObjectId,
        bits: u32,
    ) -> aa_world::Result<()> {
        if !matches!(bits, 2 | 0x81) {
            return Err(fail("native removal flag mask must be2 or81"));
        }
        let key = self.key(object)?;
        // Published actors own no second flags copy in ActorStore. Stage lookup
        // merely chooses the owner; failure of the spatial write is not hidden.
        if self.actors.staged_placement(key).is_ok() {
            self.actors.staged_placement_mut(key).map_err(fail)?.flags |= bits;
            Ok(())
        } else {
            self.spatial.or_removal_flags(key, bits).map_err(fail)
        }
    }
    fn remove_classification(
        &mut self,
        _: &mut World,
        list: ListKind,
        object: ObjectId,
    ) -> aa_world::Result<()> {
        let id = self.registered_id(object)?;
        self.registration
            .remove_classification(list, id)
            .map_err(fail)
    }
    fn effect(
        &mut self,
        effect: Effect,
        world: &mut World,
        _queues: &mut Queues,
        _: &mut NativeRng,
    ) -> aa_world::Result<()> {
        match effect {
            Effect::Fog(enabled) => self.sync.log_fog(enabled),
            Effect::FirstTick => self.first_tick.run(self.actors).map_err(fail),
            Effect::Manager => {
                self.trigger_program.time_bits = world.clock.begin.0;
                self.trigger_program.precision = world.clock_precision;
                let mut program = crate::live_updates::trigger_program::Program {
                    core: &mut *self.trigger_program,
                    sun: self.sun.as_mut(),
                    queues: &mut *_queues,
                    owners: self.owners,
                    now: world.clock.begin.0,
                };
                self.triggers
                    .update(&mut program)
                    .map(|_| ())
                    .map_err(fail)
            }
            Effect::DeferredPayload(_) => {
                Err(required("zone5147C0 deferred payload owner/effects"))
            }
            Effect::PendingComponent(_) => Err(required("Body561C20 pending100 callback")),
            Effect::Availability(_) => Err(required(
                "World4F4C40 current available-city/trigger effects",
            )),
            Effect::RegistryUnlink(_) => {
                Err(required("GTID50C1B0 actual registry unlink/lifetime"))
            }
            Effect::Actor90(_) => Err(required("Actor virtual+90 removal")),
            Effect::ActorF0(_, _) => Err(required("Actor virtual+F0 removal")),
            Effect::ActorE0(_, _) => Err(required("Actor virtual+E0 finalization")),
            Effect::Actor2C(_, _) => Err(required("Actor virtual+2C destruction")),
        }
    }
    fn predicate(
        &mut self,
        predicate: Predicate,
        object: ObjectId,
        _: &mut World,
        _: &mut Queues,
        _: &mut NativeRng,
    ) -> aa_world::Result<bool> {
        let key = self.key(object)?;
        let class = self
            .actors
            .object(key)
            .map_err(fail)?
            .definition
            .native_class;
        let offset = match predicate {
            Predicate::ClassBC => 0xbc,
            Predicate::ClassC0 => 0xc0,
            Predicate::ClassB4 => 0xb4,
            Predicate::Retain => return Err(required("Actor virtual+8C retain predicate")),
        };
        live_registration::class_predicate(class, offset).map_err(fail)
    }
    fn update_component(
        &mut self,
        event: Delivery,
        _: &mut World,
        queues: &mut Queues,
        rng: &mut NativeRng,
    ) -> aa_world::queues::Result<()> {
        crate::live_scheduler_dispatch::dispatch(
            event,
            &mut crate::live_scheduler_dispatch::Context {
                actors: self.actors,
                owners: self.owners,
                queues,
                rng,
                spatial: self.spatial,
            },
        )
    }
    fn resolve_handle(
        &mut self,
        object: ObjectId,
        _: &mut World,
        _: &mut Queues,
        _: &mut NativeRng,
    ) -> aa_world::Result<Option<ObjectId>> {
        let key = self.key(object)?;
        let actor = self
            .actors
            .object(key)
            .map_err(fail)?
            .actor
            .as_ref()
            .ok_or_else(|| required("Actor+158 GTID handle on non-Actor"))?;
        let handle = ActorId::from_raw(
            *actor
                .words
                .get(&0x158)
                .ok_or_else(|| required("initialized Actor+158"))?,
        );
        Ok(self.actors.key(handle).map(ObjectKey::raw))
    }
}
