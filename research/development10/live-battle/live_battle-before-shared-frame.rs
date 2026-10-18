//! Persistent live frame composition over one genuinely activated mission.
//! No raw Staged constructor is exposed. The actual dispatcher, queues, RNG,
//! ActorStore, spatial/registration/SVar/trigger owners move exactly once.
//! Native behavior remains in the existing proven driver and clocked kernels.
//!
//! Integration draft: do not export until battle_activation's actual sealed
//! caller-tail capability is implemented. No diagnostic ready mint is provided.
use crate::{
    actor_world::ActorStore,
    battle_activation::{ActivatedMission,ActiveApplicationOwner},
    catalog::Catalog,
    clocked::{self,ClockedState,StateParts},
    definitions::Bank,
    live_clocked,
    live_first_tick::{self,AudioOutput,MusicRequests},
    load_tail_world::SessionRead,
    load_world::{self,SpatialOwner},
    restoration::{Staged,KingdomEconomyLoad},
    scheduler_owners::Owners,
    settings::Settings,
    simulation::{self,Driver,NetworkSeams,WorldSeams},
    spatial_world::SpatialWorld,
};
use aa_orders::net::sync::Synchronizer;
use aa_rules::sources::V14Sources;
use aa_mapworld::restore::RestoreReport;
use aa_simulation::{FloatBits,NativeClock,NativeRng};
use aa_triggers::program::CoreProgram;
use aa_world::queues::Queues;
use std::{cell::RefCell,collections::BTreeMap,sync::Arc};

/// Immutable load evidence and remaining exact owners. These are moved, never
/// rebuilt from actor scans or used as an alternative live clock/queue owner.
pub struct LoadEvidence {
    pub resource_streams:Option<crate::map_resource_streams::Owner>,
    pub native_load_mode:Option<u32>,
    pub inactive_load:Option<crate::restore_lifecycle::InactiveLoad>,
    pub resource_source:Option<Arc<aa_rules::resources::ResourceCatalog>>,
    pub inventory:aa_mapworld::WorldInventory,
    pub kingdom_economy:BTreeMap<u32,KingdomEconomyLoad>,
    pub updatables:crate::updatables::Table<aa_world::queues::Owner>,
    pub report:RestoreReport,
    pub definition_text:crate::localization::Localization,
    pub live_text:crate::localization::Localization,
    pub updatables_chunk:Vec<u8>,
    pub saved_queue_load:Option<usize>,
}
struct Retained {
    actors:ActorStore,
    world:load_world::World,
    owners:Owners,
    definitions:Bank,
    settings:Settings,
    evidence:LoadEvidence,
}
enum FailedOwners {
    Activated(ActivatedMission),
    Transferred {staged:Staged,application:ActiveApplicationOwner},
    Parts {retained:Retained,clock:StateParts,application:ActiveApplicationOwner},
}
/// Failure preserves the exact graph and every committed ownership transfer.
pub struct BindFailure {error:String,owners:FailedOwners}
impl BindFailure {
    pub fn error(&self)->&str {&self.error}
    pub fn actors(&self)->&ActorStore {
        match &self.owners {
            FailedOwners::Activated(v)=>&v.staged().actors,
            FailedOwners::Transferred{staged,..}=>&staged.actors,
            FailedOwners::Parts{retained,..}=>&retained.actors,
        }
    }
}
impl std::fmt::Debug for BindFailure {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        f.debug_struct("BindFailure").field("error",&self.error).finish_non_exhaustive()
    }
}

/// Same owners survive every frame and every service reborrow. Neither a
/// frame's return nor a new Services value resets either native failure latch.
pub struct Battle<'source> {
    application:ActiveApplicationOwner,
    state:ClockedState,
    retained:Retained,
    first_tick_source:live_first_tick::Source<'source>,
    trigger_program:Option<CoreProgram>,
    music:MusicRequests,
}
impl<'source> Battle<'source> {
    pub fn from_activated(
        mission:ActivatedMission,catalog:&Catalog<'source>,files:&mut V14Sources,
    )->Result<Self,BindFailure> {
        let source=match preflight(mission.staged(),catalog,files) {
            Ok(v)=>v,Err(error)=>return Err(BindFailure{error,owners:FailedOwners::Activated(mission)}),
        };
        let (mut staged,application)=mission.into_live_parts();
        let precision=application.simulation_precision();
        let transfer=(||->Result<(),String>{
            let world=staged.load_world.as_mut().ok_or("activated mission lost its World")?;
            application.require_active_world(world)?;
            match &world.spatial {
                SpatialOwner::Loading(Some(_))=>world.adopt_spatial(&mut staged.actors)?,
                SpatialOwner::Adopted(v) if v.error.is_none() && v.pending.is_empty()=>{},
                _=>return Err("live spatial transfer is partial or already in progress".into()),
            }
            if world.live_spatial()?.index().precision()!=precision || staged.queues.precision!=precision {
                return Err("actual simulation-thread precision differs from retained spatial/queue context".into());
            }
            Ok(())
        })();
        if let Err(error)=transfer {
            return Err(BindFailure{error,owners:FailedOwners::Transferred{staged,application}});
        }
        let Staged {resource_streams,load_world,native_load_mode,inactive_load,resource_source,inventory,
            kingdom_economy,triggers,actors,owners,updatables,definitions,clock,queues,rng,
            report,settings,definition_text,live_text,updatables_chunk,saved_queue_load}=staged;
        //Preflight required the real manager installation, not an empty Saved
        //proxy or a second Manager::from_saved at the activation boundary.
        debug_assert!(triggers.is_none());
        let mut world=load_world.expect("checked actual World before consuming mission");
        let dispatcher=world.dispatcher.take().expect("checked retained dispatcher before transfer");
        let native_world=aa_world::World::from_loaded_dispatcher(dispatcher,clock,precision);
        let retained=Retained {actors,world,owners,definitions,settings,evidence:LoadEvidence {
            resource_streams,native_load_mode,inactive_load,resource_source,inventory,kingdom_economy,updatables,
            report,definition_text,live_text,updatables_chunk,saved_queue_load,
        }};
        let state=match ClockedState::from_parts(StateParts {world:native_world,queues,rng}) {
            Ok(v)=>v,Err(failure)=>return Err(BindFailure {error:failure.reason.into(),
                owners:FailedOwners::Parts {retained,clock:failure.parts,application}}),
        };
        Ok(Self {application,state,retained,first_tick_source:source,
            trigger_program:None,music:MusicRequests::new()})
    }
    pub fn actors(&self)->&ActorStore {&self.retained.actors}
    pub fn spatial(&self)->Result<&SpatialWorld,String> {self.retained.world.live_spatial()}
    pub fn clock(&self)->NativeClock {self.state.world.clock}
    pub fn stopped(&self)->bool {self.state.stopped()}
    pub fn evidence(&self)->&LoadEvidence {&self.retained.evidence}
    pub fn music_requests(&self)->&MusicRequests {&self.music}
    /// A local loan, not a new owner or a self-reference. Network callbacks and
    /// world callbacks borrow this cell sequentially in Driver.frame order.
    pub fn borrow_frame(&mut self)->Frame<'_,'source> {Frame {battle:RefCell::new(self)}}
}
fn preflight<'source>(staged:&Staged,catalog:&Catalog<'source>,files:&mut V14Sources)
    ->Result<live_first_tick::Source<'source>,String> {
    let world=staged.load_world.as_ref().ok_or("activated mission has no actual World")?;
    world.runtime_fields.require_initialized()?;
    world.variables.require_published()?;
    world.variables.validate_in_bank(catalog,&staged.definitions)?;
    if world.dispatcher.is_none() {return Err("actual dispatcher owner was already moved".into());}
    if !world.trigger_manager.as_ref().is_some_and(|m|m.saved_payload_installed()) {
        return Err("actual trigger manager has not installed its saved payload".into());
    }
    if staged.triggers.is_some() {return Err("decoded triggers were not installed into the retained manager".into());}
    if staged.report.stopped_before.is_some() || staged.report.events.iter().any(|v|v.error.is_some()) {
        return Err("activated mission still has an incomplete record section".into());
    }
    live_first_tick::Source::bind(catalog,&staged.definitions,&staged.settings,files)
}

/// Actual external siblings of the retained LocalBootstrap borrow. Only the
/// player-store LOAN is in a cell; the Host/Session owner is never copied or
/// wrapped wholesale. The same SyncA and actual output/gate are reused.
pub struct TickInputs<'a,S:SessionRead+?Sized,O:AudioOutput+?Sized> {
    pub session:&'a RefCell<S>,
    pub sync_a:&'a RefCell<Synchronizer>,
    pub sync_gate:&'a RefCell<&'a mut dyn FnMut()->Result<bool,String>>,
    pub audio:&'a RefCell<&'a mut O>,
}
pub struct Frame<'battle,'source> {battle:RefCell<&'battle mut Battle<'source>>}
/// Actual native-order callbacks borrow these same fields. No clock write or
/// second dispatcher is exposed. The decree adapter still performs its real
/// authorization, canonical-player/selection and native command dispatch.
pub struct OrderLoans<'a> {
    pub actors:&'a mut ActorStore,
    pub spatial:&'a mut SpatialWorld,
    pub queues:&'a mut Queues,
    pub rng:&'a mut NativeRng,
    pub definitions:&'a Bank,
    pub owners:&'a Owners,
    pub settings:&'a Settings,
    pub clock:NativeClock,
}
impl Frame<'_,'_> {
    pub fn with_native_order<R>(&self,call:impl FnOnce(OrderLoans<'_>)->Result<R,String>)->Result<R,String> {
        let mut b=self.battle.try_borrow_mut().map_err(|e|format!("actual frame order loan: {e}"))?;
        let Battle {application,state,retained,..}=&mut **b;
        if state.stopped() {return Err("native order refused after partially committed world update".into());}
        application.require_active_world(&retained.world)?;
        let spatial=match &mut retained.world.spatial {
            SpatialOwner::Adopted(v) if v.error.is_none() && v.pending.is_empty()=>&mut v.world,
            _=>return Err("actual live placement owner is incomplete".into()),
        };
        call(OrderLoans {actors:&mut retained.actors,spatial,queues:&mut state.queues,
            rng:&mut state.rng,definitions:&retained.definitions,owners:&retained.owners,
            settings:&retained.settings,clock:state.world.clock})
    }
    /// One native75CA70 frame; the existing driver authorizes at most one
    /// world/admin tick and retains its own failure latch. No raw step API.
    pub fn run<N:NetworkSeams,S:SessionRead+?Sized,O:AudioOutput+?Sized>(
        &self,driver:&mut Driver,now:FloatBits,network:&mut N,inputs:TickInputs<'_,S,O>,
    )->simulation::Result<simulation::Action> {
        {
            let b=self.battle.try_borrow_mut().map_err(|e|seam(format!("frame loan remains held: {e}")))?;
            if b.state.stopped() {return Err(clocked::Error::Stopped.into());}
            if driver.precision!=b.state.world.clock_precision {
                return Err(seam("actual Driver/World thread precisions differ"));
            }
            b.application.require_active_world(&b.retained.world).map_err(seam)?;
        }
        let mut world=WorldBorrow {frame:self,inputs};
        driver.frame(now,network,&mut world)
    }
}
fn seam(error:impl std::fmt::Display)->simulation::Error {simulation::Error::Seam(error.to_string())}
struct WorldBorrow<'a,'b,'source,'inputs,S:SessionRead+?Sized,O:AudioOutput+?Sized> {
    frame:&'a Frame<'b,'source>,inputs:TickInputs<'inputs,S,O>,
}
impl<S:SessionRead+?Sized,O:AudioOutput+?Sized> WorldSeams for WorldBorrow<'_,'_,'_,'_,S,O> {
    fn world_clock(&self)->Option<NativeClock> {
        //Driver calls outside every completed synchronous callback loan. A
        //violating reentrant caller is outside this valid sequential domain;
        //never disguise its borrow violation as an absent native GWorld.
        Some(self.frame.battle.borrow().state.world.clock)
    }
    fn world_update(&mut self)->simulation::Result<NativeClock> {
        let session=self.inputs.session.try_borrow().map_err(seam)?;
        let mut output=self.inputs.audio.try_borrow_mut().map_err(seam)?;
        let mut b=self.frame.battle.try_borrow_mut().map_err(seam)?;
        let Battle {application,state,retained,first_tick_source,trigger_program,music}=&mut **b;
        application.require_active_world(&retained.world).map_err(seam)?;
        //76AC90 is Session+F8. The same Session loan stays immutable during
        //this supported CoreProgram interval; no phase-changing world opcode
        //is silently implemented by the manager-local evaluator.
        let phase=session.current_phase().map_err(seam)?;
        let program=trigger_program.get_or_insert_with(||CoreProgram::new(
            state.world.clock.begin.0,phase,state.world.clock_precision));
        program.application_mode=phase;
        let world=&mut retained.world;
        let spatial=match &mut world.spatial {
            SpatialOwner::Adopted(v) if v.error.is_none() && v.pending.is_empty()=>&mut v.world,
            _=>return Err(seam("actual live spatial owner is absent during update")),
        };
        let fields=world.runtime_fields.frame_fields(world.variables.cheat_fog_word()).map_err(seam)?;
        let mut first_tick=live_first_tick::Context {
            session:&*session,source:first_tick_source,bank:&retained.definitions,
            settings:&retained.settings,requests:music,output:&mut **output,
        };
        let mut gate=|| {
            let mut loan=self.inputs.sync_gate.try_borrow_mut().map_err(|e|e.to_string())?;
            (**loan)()
        };
        let mut services=live_clocked::Services {
            actors:&mut retained.actors,owners:&retained.owners,spatial,
            registration:&mut world.registration,fields,
            sync:live_clocked::SyncBinding::Shared {owner:self.inputs.sync_a,gate:&mut gate},
            triggers:world.trigger_manager.as_mut().ok_or_else(||seam("retained trigger manager is absent"))?,
            trigger_program:program,first_tick:&mut first_tick,
        };
        Ok(state.step(&mut services)?)
    }
}
