//! Native frame services over one genuinely started, SAI initialized World.
//! No raw Staged constructor is exposed. The actual dispatcher, queues, RNG,
//! ActorStore, spatial/registration/SVar/trigger owners move exactly once.
//! Native behavior remains in the existing proven driver and clocked kernels.
//!
//! This owns World services only. It grants no application activation receipt;
//! the concrete netcode7757F0 coordinator retains the product activation gate.
use crate::{
    actor_world::ActorStore,
    world_frame_inputs::WorldFrameInputs,
    restore_lifecycle::StartupOwner,
    catalog::Catalog,
    clocked::{self, ClockedState, StateParts},
    definitions::Bank,
    live_clocked,
    live_first_tick::{self, AudioOutput, MusicRequests},
    load_tail_world::SessionRead,
    load_world::{self, SpatialOwner},
    restoration::{KingdomEconomyLoad, Staged},
    scheduler_owners::Owners,
    settings::Settings,
    simulation::{self, WorldSeams, borrowed, shared_sim::SharedSim},
    spatial_world::SpatialWorld,
};
use aa_mapworld::restore::RestoreReport;
use aa_orders::net::sync::Synchronizer;
use aa_rules::sources::V14Sources;
use aa_simulation::{FloatBits, NativeClock, NativeRng};
use aa_triggers::program::CoreProgram;
use aa_world::queues::Queues;
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};
#[path="live_frame_services.rs"]
mod frame_services;

/// Immutable load evidence and remaining exact owners. These are moved, never
/// rebuilt from actor scans or used as an alternative live clock/queue owner.
pub struct LoadEvidence {
    pub resource_streams: Option<crate::map_resource_streams::Owner>,
    pub native_load_mode: Option<u32>,
    pub inactive_load: Option<crate::restore_lifecycle::InactiveLoad>,
    pub resource_source: Option<Arc<aa_rules::resources::ResourceCatalog>>,
    pub inventory: aa_mapworld::WorldInventory,
    pub kingdom_economy: BTreeMap<u32, KingdomEconomyLoad>,
    pub updatables: crate::updatables::Table<aa_world::queues::Owner>,
    pub report: RestoreReport,
    pub definition_text: crate::localization::Localization,
    pub live_text: crate::localization::Localization,
    pub updatables_chunk: Vec<u8>,
    pub saved_queue_load: Option<usize>,
}
/// One retained owner. It leaves its slot only while a synchronous
/// load-shaped callee runs on a temporary [`Staged`] assembled from the SAME
/// owners ([`Retained::with_staged`]); every owner is moved back before the
/// Retained is reachable again, so the empty state is never observable.
struct Lent<T>(Option<T>);
const LENT: &str = "retained owner is lent to a Staged creation loan";
impl<T> Lent<T> {
    fn new(value: T) -> Self {
        Self(Some(value))
    }
    fn take(&mut self) -> T {
        self.0.take().expect(LENT)
    }
    fn put(&mut self, value: T) {
        debug_assert!(self.0.is_none());
        self.0 = Some(value);
    }
}
impl<T> std::ops::Deref for Lent<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().expect(LENT)
    }
}
impl<T> std::ops::DerefMut for Lent<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut().expect(LENT)
    }
}
struct Retained {
    actors: Lent<ActorStore>,
    world: Lent<load_world::World>,
    owners: Lent<Owners>,
    definitions: Lent<Bank>,
    settings: Lent<Settings>,
    evidence: Lent<LoadEvidence>,
    /// Set once a census update swallowed an unported leaf. Such a World is no
    /// longer the native state and every later product/strict update refuses.
    census_contaminated: bool,
    /// 404F60-family debug-log calls of scheduled Updates, in call order.
    log: Vec<crate::live_updates::NativeLog>,
}
impl Retained {
    /// Run a load-shaped callee (fresh 4F9C10/4FA590 creation and its
    /// InitCreate/PostCreate/registration children) on a temporary Staged
    /// assembled by MOVING the same live owners: actors, World (with the live
    /// dispatcher phase/lists lent back from the clocked World), owners, Bank,
    /// settings, load evidence, queues and RNG. The authoritative clock is
    /// lent by value and must come back unchanged. Every owner is moved back
    /// before return, also when the callee fails (its committed effects stay).
    pub(crate) fn with_staged<R>(
        &mut self,
        state: &mut ClockedState,
        live_sai: crate::restore_lifecycle::LiveSai,
        call: impl FnOnce(&mut Staged) -> Result<R, String>,
    ) -> (Result<R, String>, crate::restore_lifecycle::LiveSai) {
        let placeholder = match Queues::new(state.queues.clock, state.queues.precision) {
            Ok(v) => v,
            Err(e) => return (Err(e.to_string()), live_sai),
        };
        if self.world.dispatcher.is_some() {
            return (Err("live World already holds a load dispatcher owner".into()), live_sai);
        }
        let mut world = self.world.take();
        world.dispatcher = Some(state.world.lend_dispatcher());
        let LoadEvidence {
            resource_streams,
            native_load_mode,
            inactive_load,
            resource_source,
            inventory,
            kingdom_economy,
            updatables,
            report,
            definition_text,
            live_text,
            updatables_chunk,
            saved_queue_load,
        } = self.evidence.take();
        let clock = state.world.clock;
        let mut staged = Staged {
            live_sai: Some(live_sai),
            resource_streams,
            load_world: Some(world),
            native_load_mode,
            inactive_load,
            resource_source,
            inventory,
            kingdom_economy,
            triggers: None,
            actors: self.actors.take(),
            owners: self.owners.take(),
            updatables,
            definitions: self.definitions.take(),
            clock,
            queues: std::mem::replace(&mut state.queues, placeholder),
            rng: state.rng,
            report,
            settings: self.settings.take(),
            definition_text,
            live_text,
            updatables_chunk,
            saved_queue_load,
        };
        let result = call(&mut staged);
        let Staged {
            resource_streams,
            load_world,
            native_load_mode,
            inactive_load,
            resource_source,
            inventory,
            kingdom_economy,
            triggers,
            actors,
            owners,
            updatables,
            definitions,
            clock: returned_clock,
            queues,
            rng,
            report,
            settings,
            definition_text,
            live_text,
            updatables_chunk,
            saved_queue_load,
            live_sai,
        } = staged;
        let live_sai = live_sai.expect("a Staged loan callee cannot remove the lent SAI");
        let mut world = load_world.expect("a Staged loan callee cannot remove the lent World");
        let dispatcher = world.dispatcher.take().expect("a Staged loan callee cannot remove the lent dispatcher");
        state.world.restore_dispatcher(dispatcher);
        self.world.put(world);
        self.actors.put(actors);
        self.owners.put(owners);
        self.definitions.put(definitions);
        self.settings.put(settings);
        self.evidence.put(LoadEvidence {
            resource_streams,
            native_load_mode,
            inactive_load,
            resource_source,
            inventory,
            kingdom_economy,
            updatables,
            report,
            definition_text,
            live_text,
            updatables_chunk,
            saved_queue_load,
        });
        state.queues = queues;
        state.rng = rng;
        let loan = if returned_clock != clock {
            Err("Staged creation loan changed the authoritative World clock".to_owned())
        } else if triggers.is_some() {
            Err("Staged creation loan installed a decoded trigger payload".to_owned())
        } else {
            Ok(())
        };
        let result = match (result, loan) {
            (Ok(v), Ok(())) => Ok(v),
            (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
            (Err(e), Err(l)) => Err(format!("{e}; {l}")),
        };
        (result, live_sai)
    }
}
/// One unported/failed leaf recorded (and skipped) by a census update.
#[derive(Clone, Debug, serde::Serialize)]
pub struct CensusEntry {
    pub tick: u32,
    pub site: String,
    pub error: String,
}
enum FailedOwners {
    Inputs(WorldFrameInputs),
    Transferred {
        staged: Staged,
        application: StartupOwner,
    },
    Parts {
        retained: Retained,
        clock: StateParts,
        application: StartupOwner,
    },
}
/// Failure preserves the exact graph and every committed ownership transfer.
pub struct BindFailure {
    error: String,
    owners: FailedOwners,
}
impl BindFailure {
    pub fn error(&self) -> &str {
        &self.error
    }
    pub fn actors(&self) -> &ActorStore {
        match &self.owners {
            FailedOwners::Inputs(v) => &v.staged().actors,
            FailedOwners::Transferred { staged, .. } => &staged.actors,
            FailedOwners::Parts { retained, .. } => &retained.actors,
        }
    }
}
impl std::fmt::Debug for BindFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

/// Same owners survive every frame and every service reborrow. Neither a
/// frame's return nor a new Services value resets either native failure latch.
pub struct World<'source> {
    application: StartupOwner,
    precision: aa_numeric::Precision,
    state: ClockedState,
    retained: Retained,
    first_tick_source: live_first_tick::Source<'source>,
    trigger_program: Option<CoreProgram>,
    music: MusicRequests,
}
///Read-only loans of the actual retained owners. This grants no activation or
///tick capability and contains no rebuilt actor, fog or definition snapshots.
pub struct PresentationParts<'a> {
    pub camera_flags:&'a crate::world_camera_flags::State,
    pub terrain:&'a crate::battle_terrain::BattleTerrain,
    pub actors:&'a ActorStore,
    pub spatial:&'a SpatialWorld,
    pub definitions:&'a Bank,
    pub settings:&'a Settings,
    pub variables:&'a crate::load_view_variables::State,
    pub zones:&'a aa_zones::tile::ZoneMap,
    pub kingdoms:&'a crate::load_kingdom_init_world::KingdomRegistryState,
    pub inventory:&'a aa_mapworld::WorldInventory,
    pub revision:&'a crate::world_revision::ConstructedWorldRevision,
    pub view_biome:Option<&'a crate::vworld_resource_context::Biome>,
    pub creator_extents:Option<&'a crate::world_creator_extents::State>,
    pub camera_target:&'a crate::load_tail_world::CameraState,
    pub triggers:Option<&'a aa_triggers::manager::Manager>,
    ///GWorld runtime fields; `.fog_cheat_word()` is GWorld+275.
    pub runtime_fields:&'a crate::load_world_runtime_fields::State,
    ///Completed source requirement bytes, e.g. `.byte(id, 0x16D)` ObjectData+16D.
    pub source_fields:&'a aa_rules::native_source_requirements::NativeSourceRequirements,
}
///Disjoint loans for the actual4F0750 view-publication stage. The same
///presentation queue, actor allocations and spatial owner remain retained;
///this exposes neither a second frame interval nor an activation constructor.
pub struct ViewBindingParts<'a> {
    pub actors:&'a mut ActorStore,
    pub spatial:&'a mut SpatialWorld,
    pub presentation:&'a mut crate::fx_manager::State<crate::fx_graph::Command>,
    pub kingdoms:&'a crate::load_kingdom_init_world::KingdomRegistryState,
    pub definitions:&'a Bank,
    pub settings:&'a Settings,
    pub inventory:&'a aa_mapworld::WorldInventory,
    pub revision:&'a crate::world_revision::ConstructedWorldRevision,
    pub clock:&'a NativeClock,
    pub world_phase:&'a u32,
    pub view_biome:Option<&'a crate::vworld_resource_context::Biome>,
}
impl<'source> World<'source> {
    pub fn from_sai_complete(
        inputs: WorldFrameInputs,
        catalog: &Catalog<'source>,
        files: &mut V14Sources,
        precision: aa_numeric::Precision,
    ) -> Result<Self, BindFailure> {
        if let Err(error)=inputs.require_sai_complete(catalog) {
            return Err(BindFailure {error,owners:FailedOwners::Inputs(inputs)});
        }
        let source = match preflight(inputs.staged(), catalog, files) {
            Ok(v) => v,
            Err(error) => {
                return Err(BindFailure {
                    error,
                    owners: FailedOwners::Inputs(inputs),
                });
            }
        };
        Self::from_bound(inputs,catalog,source,precision)
    }
    fn from_bound(
        inputs:WorldFrameInputs,catalog:&Catalog<'_>,source:live_first_tick::Source<'source>,
        precision:aa_numeric::Precision,
    )->Result<Self,BindFailure> {
        let (mut staged,application)=inputs.into_sai_complete_parts(catalog)
            .map_err(|(inputs,error)|BindFailure {error,owners:FailedOwners::Inputs(inputs)})?;
        let transfer = (|| -> Result<(), String> {
            let world = staged
                .load_world
                .as_mut()
                .ok_or("started World lost its World")?;
            world.mission_fields.require_loaded()?;
            world.configuration.require_boundary()?;
            if world.mission_fields.started48()==0 {return Err("frame World mission is unstarted".into())}
            match &world.spatial {
                SpatialOwner::Loading(Some(_)) => world.adopt_spatial(&mut staged.actors)?,
                SpatialOwner::Adopted(v) if v.error.is_none() && v.pending.is_empty() => {}
                _ => return Err("live spatial transfer is partial or already in progress".into()),
            }
            if world.live_spatial()?.index().precision() != precision
                || staged.queues.precision != precision
            {
                return Err("actual simulation-thread precision differs from retained spatial/queue context".into());
            }
            Ok(())
        })();
        if let Err(error) = transfer {
            return Err(BindFailure {
                error,
                owners: FailedOwners::Transferred {
                    staged,
                    application,
                },
            });
        }
        let Staged {
            resource_streams,
            load_world,
            native_load_mode,
            inactive_load,
            resource_source,
            inventory,
            kingdom_economy,
            triggers,
            actors,
            owners,
            updatables,
            definitions,
            clock,
            queues,
            rng,
            report,
            settings,
            definition_text,
            live_text,
            updatables_chunk,
            saved_queue_load,
            live_sai: _,
        } = staged;
        //Preflight required the real manager installation, not an empty Saved
        //proxy or a second Manager::from_saved at the activation boundary.
        debug_assert!(triggers.is_none());
        let mut world = load_world.expect("checked actual World before consuming mission");
        let dispatcher = world
            .dispatcher
            .take()
            .expect("checked retained dispatcher before transfer");
        let native_world = aa_world::World::from_loaded_dispatcher(dispatcher, clock, precision);
        let retained = Retained {
            actors: Lent::new(actors),
            world: Lent::new(world),
            owners: Lent::new(owners),
            definitions: Lent::new(definitions),
            settings: Lent::new(settings),
            census_contaminated: false,
            log: Vec::new(),
            evidence: Lent::new(LoadEvidence {
                resource_streams,
                native_load_mode,
                inactive_load,
                resource_source,
                inventory,
                kingdom_economy,
                updatables,
                report,
                definition_text,
                live_text,
                updatables_chunk,
                saved_queue_load,
            }),
        };
        let state = match ClockedState::from_parts(StateParts {
            world: native_world,
            queues,
            rng,
        }) {
            Ok(v) => v,
            Err(failure) => {
                return Err(BindFailure {
                    error: failure.reason.into(),
                    owners: FailedOwners::Parts {
                        retained,
                        clock: failure.parts,
                        application,
                    },
                });
            }
        };
        Ok(Self {
            application,
            precision,
            state,
            retained,
            first_tick_source: source,
            trigger_program: None,
            music: MusicRequests::new(),
        })
    }
    pub fn actors(&self) -> &ActorStore {
        &self.retained.actors
    }
    pub fn presentation_parts(&self)->Result<PresentationParts<'_>,String> {
        Ok(PresentationParts {camera_flags:&self.retained.world.camera_flags,actors:&self.retained.actors,spatial:self.retained.world.live_spatial()?,
            definitions:&self.retained.definitions,settings:&self.retained.settings,
            terrain:&self.retained.world.terrain,variables:&self.retained.world.variables,zones:&self.retained.world.zones,
            kingdoms:&self.retained.world.kingdoms,inventory:&self.retained.evidence.inventory,
            view_biome:self.retained.world.view_biome.as_ref(),revision:&self.retained.world.revision,
            creator_extents:self.retained.world.creator_extents.as_ref(),camera_target:&self.retained.world.tail.camera,
            triggers:self.retained.world.trigger_manager.as_ref(),
            runtime_fields:&self.retained.world.runtime_fields,source_fields:&self.retained.world.source_fields})
    }
    pub fn with_view_binding<R>(&mut self,call:impl FnOnce(ViewBindingParts<'_>)->Result<R,String>)->Result<R,String> {
        if self.state.stopped(){return Err("view publication refused after a failed World update".into());}
        let world=&mut *self.retained.world;
        let spatial=match &mut world.spatial {
            SpatialOwner::Adopted(v)if v.error.is_none()&&v.pending.is_empty()=>&mut v.world,
            _=>return Err("view publication requires the same complete adopted spatial owner".into()),
        };
        call(ViewBindingParts{actors:&mut self.retained.actors,spatial,
            presentation:&mut world.presentation,kingdoms:&world.kingdoms,
            definitions:&self.retained.definitions,settings:&self.retained.settings,
            inventory:&self.retained.evidence.inventory,revision:&world.revision,
            clock:&self.state.world.clock,world_phase:&self.state.world.phase,
            view_biome:world.view_biome.as_ref()})
    }
    pub fn spatial(&self) -> Result<&SpatialWorld, String> {
        self.retained.world.live_spatial()
    }
    pub fn clock(&self) -> NativeClock {
        self.state.world.clock
    }
    ///Read-only view of the SAME retained five native update queues
    ///(World+1AC/1C0/1D4/1E8/208). No copy is retained by this owner.
    pub fn queues(&self) -> &Queues {
        &self.state.queues
    }
    ///Read-only scheduler identity table of the retained graph.
    pub fn owners(&self) -> &Owners {
        &self.retained.owners
    }
    ///775D2F's same live World4F8AE0 initial synchronization. This consumes
    ///the authoritative RNG draw even when the actual logger is disabled.
    ///No detached Staged/clock or second synchronizer is introduced.
    pub fn synchronize_initial(&mut self,catalog:&Catalog<'_>,label:&str,binding:&mut live_clocked::SyncBinding<'_>)->Result<(),String>{
        crate::initial_world_sync::synchronize(&self.retained.world,&self.retained.actors,
            &self.retained.definitions,catalog,&self.state.world.clock,&mut self.state.rng,
            self.precision,label,binding)
    }
    pub fn stopped(&self) -> bool {
        self.state.stopped()
    }
    pub fn evidence(&self) -> &LoadEvidence {
        &self.retained.evidence
    }
    pub fn music_requests(&self) -> &MusicRequests {
        &self.music
    }
    /// Split the actual application/control owner from the World loan once.
    /// Order callbacks may borrow the World while the frame kernel holds its
    /// separate process-counter loan. Neither loan contains the whole Battle.
    pub fn borrow_frame(&mut self) -> Frame<'_, 'source> {
        let Self {
            application,
            precision,
            state,
            retained,
            first_tick_source,
            trigger_program,
            music,
        } = self;
        let (control,game_spy,presentation_timing,heartbeat_counter,network,sai_engine,sai_players)=application.frame_all_parts();
        Frame {
            precision:*precision,
            control: RefCell::new(control),
            game_spy,sai:RefCell::new((sai_engine,sai_players)),
            network,
            presentation_timing:RefCell::new(presentation_timing),
            heartbeat_counter:RefCell::new(heartbeat_counter),
            world: RefCell::new(WorldLoan {
                state,
                retained,
                first_tick_source,
                trigger_program,
                music,
            }),
        }
    }
}
impl World<'static> {
    ///Same concrete SAI-completed graph with an owning reference to its exact
    ///immutable source catalog, suitable for an ordinary application field.
    pub fn from_sai_complete_owned(
        inputs:WorldFrameInputs,catalog:&Catalog<'_>,files:&mut V14Sources,
        owner:std::sync::Arc<aa_rules::actors::ActorCatalog>,precision:aa_numeric::Precision,
    )->Result<Self,BindFailure> {
        if let Err(error)=inputs.require_sai_complete(catalog) {
            return Err(BindFailure {error,owners:FailedOwners::Inputs(inputs)});
        }
        let source=match preflight(inputs.staged(),catalog,files) {
            Ok(source)=>match source.into_shared(owner) {
                Ok(source)=>source,
                Err((_,error))=>return Err(BindFailure {error,owners:FailedOwners::Inputs(inputs)}),
            },
            Err(error)=>return Err(BindFailure {error,owners:FailedOwners::Inputs(inputs)}),
        };
        Self::from_bound(inputs,catalog,source,precision)
    }
}
fn preflight<'source>(
    staged: &Staged,
    catalog: &Catalog<'source>,
    files: &mut V14Sources,
) -> Result<live_first_tick::Source<'source>, String> {
    let world = staged
        .load_world
        .as_ref()
        .ok_or("started World has no actual World")?;
    world.runtime_fields.require_initialized()?;
    world.variables.require_published()?;
    world
        .variables
        .validate_in_bank(catalog, &staged.definitions)?;
    if world.dispatcher.is_none() {
        return Err("actual dispatcher owner was already moved".into());
    }
    if !world
        .trigger_manager
        .as_ref()
        .is_some_and(|m| m.saved_payload_installed())
    {
        return Err("actual trigger manager has not installed its saved payload".into());
    }
    if staged.triggers.is_some() {
        return Err("decoded triggers were not installed into the retained manager".into());
    }
    if staged.report.stopped_before.is_some()
        || staged.report.events.iter().any(|v| v.error.is_some())
    {
        return Err("started World still has an incomplete record section".into());
    }
    live_first_tick::Source::bind(catalog, &staged.definitions, &staged.settings, files)
}

/// Actual external siblings of the retained LocalBootstrap borrow. Only the
/// player-store LOAN is in a cell; the Host/Session owner is never copied or
/// wrapped wholesale. The same SyncA and actual output/gate are reused.
pub struct TickInputs<'a, S: SessionRead + ?Sized, O: AudioOutput + ?Sized> {
    pub session: &'a RefCell<S>,
    pub sync_a: &'a RefCell<Synchronizer>,
    pub sync_gate: &'a RefCell<&'a mut dyn FnMut() -> Result<bool, String>>,
    pub audio: &'a RefCell<&'a mut O>,
}
struct WorldLoan<'battle, 'source> {
    state: &'battle mut ClockedState,
    retained: &'battle mut Retained,
    first_tick_source: &'battle live_first_tick::Source<'source>,
    trigger_program: &'battle mut Option<CoreProgram>,
    music: &'battle mut MusicRequests,
}
pub struct Frame<'battle, 'source> {
    precision: aa_numeric::Precision,
    control: RefCell<&'battle mut crate::live_sim::FrameControl>,
    world: RefCell<WorldLoan<'battle, 'source>>,
    game_spy:&'battle crate::game_spy_manager_gate::State,
    network:&'battle crate::standalone_network::ApplicationNetwork,
    ///The application SAI manager [B36528] and its player arena, the SAME
    ///owners; a live creation loan moves them through its Staged and back.
    sai:RefCell<(&'battle mut aa_mind::sai::manager::Sai,&'battle mut crate::load_sai_players::Arena)>,
    presentation_timing:RefCell<&'battle mut crate::presentation_timing::State>,
    heartbeat_counter:RefCell<&'battle mut crate::heartbeat_process::Counter>,
}
/// Actual native-order callbacks borrow these same fields. No clock write or
/// second dispatcher is exposed. The decree adapter still performs its real
/// authorization, canonical-player/selection and native command dispatch.
pub struct OrderLoans<'a> {
    pub actors: &'a mut ActorStore,
    pub spatial: &'a mut SpatialWorld,
    pub queues: &'a mut Queues,
    pub rng: &'a mut NativeRng,
    pub definitions: &'a mut Bank,
    pub owners: &'a Owners,
    pub settings: &'a Settings,
    pub clock: NativeClock,
    pub mind_runtime:&'a mut crate::mind_world::Runtime,
    sync_fields:crate::initial_world_sync::Fields<'a>,
    revision:&'a crate::world_revision::ConstructedWorldRevision,
    sync_clock:&'a NativeClock,
    sync_precision:aa_numeric::Precision,
}
///Disjoint ordinary gameplay callback loans. This is the same retained graph
///as315's synchronization view, with mutable state/FX services when reached.
///Neither the authoritative clock nor its constructor identity is copied.
pub struct GameplayOrderLoans<'a>{
    pub actors:&'a mut ActorStore,
    pub spatial:&'a mut SpatialWorld,
    pub queues:&'a mut Queues,
    pub rng:&'a mut NativeRng,
    pub definitions:&'a mut Bank,
    pub owners:&'a mut Owners,
    pub mind_runtime:&'a mut crate::mind_world::Runtime,
    pub variables:&'a mut crate::load_view_variables::State,
    pub presentation:&'a mut crate::fx_manager::State<crate::fx_graph::Command>,
    pub inventory:&'a aa_mapworld::WorldInventory,
    pub zones:&'a aa_zones::tile::ZoneMap,
    pub clock:&'a NativeClock,
    pub revision:&'a crate::world_revision::ConstructedWorldRevision,
    pub precision:aa_numeric::Precision,
    pub terrain:&'a crate::battle_terrain::BattleTerrain,
    pub mission:&'a crate::mission_fields::State,
    pub phase:&'a u32,
    pub current_settings:crate::stored_actor_state_effects::CurrentSettings<'a>,
}
impl OrderLoans<'_>{
    ///The same currently retained World constructor identity. Pair this with
    ///the Session's Sim.require_created_world check; a numeric revision value
    ///or arbitrary presence boolean does not identify the published World.
    pub fn revision(&self)->&crate::world_revision::ConstructedWorldRevision{self.revision}
    ///Authoritative clock loan during a reentrant native command callback.
    ///The public clock value is diagnostic; timeline services use this owner.
    pub fn world_clock(&self)->&NativeClock{self.sync_clock}
    ///Native76D820 on the SAME published current World SVar allocation.
    ///This uses the retained Bank and the existing reentrant SyncA loan.
    pub fn synchronize_svars(&self,catalog:&Catalog<'_>,inventory:&crate::inactive_svar_dump::Inventory,binding:&mut live_clocked::SyncBinding<'_>)->Result<(),String>{
        inventory.synchronize_current(catalog,self.definitions,self.sync_fields.svars,binding)
    }
    ///Reentrant315/775BF0 synchronization borrows the same command interval,
    ///including the actual authoritative clock/RNG and disjoint World fields.
    pub fn synchronize_initial(&mut self,catalog:&Catalog<'_>,label:&str,binding:&mut live_clocked::SyncBinding<'_>)->Result<(),String>{
        let f=&self.sync_fields;
        crate::initial_world_sync::synchronize_fields(crate::initial_world_sync::Fields{
            svars:f.svars,
            counters:f.counters,rules:f.rules,mission:f.mission,runtime:f.runtime,
            misc:f.misc,registration:f.registration,objectives:f.objectives,kingdoms:f.kingdoms,
        },self.spatial,self.actors,self.definitions,catalog,self.sync_clock,self.rng,self.sync_precision,label,binding)
    }
}
impl Frame<'_, '_> {
    ///Only between native World update loans. A gameplay error keeps every
    ///committed command/component/queue effect in this retained World.
    pub fn with_gameplay_order<R>(&self,call:impl FnOnce(GameplayOrderLoans<'_>)->Result<R,String>)->Result<R,String>{
        let mut b=self.world.try_borrow_mut().map_err(|e|format!("actual gameplay World loan: {e}"))?;
        let WorldLoan{state,retained,..}=&mut *b;
        if state.stopped(){return Err("gameplay order refused after failed World update".into())}
        let world=&mut *retained.world;
        let spatial=match &mut world.spatial{
            SpatialOwner::Adopted(v)if v.error.is_none()&&v.pending.is_empty()=>&mut v.world,
            _=>return Err("actual gameplay spatial owner is incomplete".into()),
        };
        call(GameplayOrderLoans{actors:&mut retained.actors,spatial,queues:&mut state.queues,rng:&mut state.rng,
            definitions:&mut retained.definitions,owners:&mut retained.owners,mind_runtime:&mut world.mind_runtime,
            variables:&mut world.variables,presentation:&mut world.presentation,
            inventory:&retained.evidence.inventory,zones:&world.zones,clock:&state.world.clock,revision:&world.revision,precision:self.precision,
            terrain:&world.terrain,mission:&world.mission_fields,phase:&state.world.phase,
            current_settings:Some((world.global_settings_mode,world.map_mode.as_ref()))})
    }
    ///76F600 reads the distinct B40C9C owner, never the B3654C+BC gate.
    ///Only a completed SAME local selector proves its absence. Other choices
    ///remain required; this does not manufacture an online-manager word.
    pub fn end_condition_online_active(&self)->Result<bool,String>{
        self.network.standalone()?.online_manager_present()
    }
    ///Readonly current World fields after WorldServices released its update
    ///loan. Session callbacks borrow their separate owner; no World/clock is
    ///copied, reinstalled or turned into a second simulation interval.
    pub fn with_session_end_condition<R>(&self,call:impl FnOnce(
        &ActorStore,&crate::mission_fields::State,&crate::world_revision::ConstructedWorldRevision,
    )->Result<R,String>)->Result<R,String>{
        let b=self.world.try_borrow().map_err(|e|format!("actual end-condition World loan: {e}"))?;
        if b.state.stopped(){return Err("end-condition refused after failed World update".into())}
        call(&b.retained.actors,&b.retained.world.mission_fields,&b.retained.world.revision)
    }
    ///7515A0 borrows the SAME current actors/source Bank and definition-time
    ///text after WorldServices releases its loan. The caller validates the
    ///actual Sim+3C alias against this revision before reading Player28.
    pub fn with_version_ack_context<R>(&self,call:impl FnOnce(
        &ActorStore,&Bank,&crate::localization::Localization,&crate::world_revision::ConstructedWorldRevision,
    )->Result<R,String>)->Result<R,String>{
        let b=self.world.try_borrow().map_err(|e|format!("actual client-description World loan: {e}"))?;
        if b.state.stopped(){return Err("client description refused after failed World update".into())}
        call(&b.retained.actors,&b.retained.definitions,&b.retained.evidence.definition_text,&b.retained.world.revision)
    }
    ///Native reference readers run between order callbacks. They read the
    ///same registry/generations and source Bank, without retaining a snapshot
    ///or keeping this loan alive during a subsequent gameplay callback.
    pub fn with_order_data<R>(&self,call:impl FnOnce(
        &ActorStore,&Bank,&crate::world_revision::ConstructedWorldRevision,
    )->Result<R,String>)->Result<R,String>{
        let b=self.world.try_borrow().map_err(|e|format!("actual order reference World loan: {e}"))?;
        if b.state.stopped(){return Err("order reference reader refused after failed World update".into())}
        call(&b.retained.actors,&b.retained.definitions,&b.retained.world.revision)
    }
    ///Same current application manager/SAI fields, read afresh by each
    ///native logger gate; no per-frame bool or flags snapshot is returned.
    pub fn game_spy_manager_gate(&self)->&crate::game_spy_manager_gate::State{self.game_spy}
    ///SAI+5C..5E read afresh at each native logger gate.
    pub fn sai_sync_flags(&self)->Result<[u8;3],String>{
        Ok(self.sai.try_borrow().map_err(|e|format!("SAI+5C read during an SAI loan: {e}"))?.0.bytes_5c)
    }
    ///Move the application SAI manager and player arena out for one live
    ///creation loan. The constructor placeholder left in the slot carries the
    ///SAME +5C sync bytes, which creation children never write, so native
    ///logger gates evaluated during the loan read the actual values.
    fn lend_sai(&self)->Result<crate::restore_lifecycle::LiveSai,String>{
        let mut sai=self.sai.try_borrow_mut().map_err(|e|format!("SAI loan: {e}"))?;
        let mut lent=crate::restore_lifecycle::LiveSai::placeholders();
        lent.engine.bytes_5c=sai.0.bytes_5c;
        Ok(crate::restore_lifecycle::LiveSai{
            engine:std::mem::replace(&mut *sai.0,lent.engine),
            players:std::mem::replace(&mut *sai.1,lent.players),
        })
    }
    fn return_sai(&self,live:crate::restore_lifecycle::LiveSai)->Result<(),String>{
        let mut sai=self.sai.try_borrow_mut().map_err(|e|format!("SAI return: {e}"))?;
        if sai.0.bytes_5c!=live.engine.bytes_5c {
            *sai.0=live.engine;*sai.1=live.players;
            return Err("SAI+5C sync bytes changed during a creation loan".into());
        }
        *sai.0=live.engine;*sai.1=live.players;Ok(())
    }
    ///Disjoint loans of the same application-lifetime timing and heartbeat
    ///owners. Native service adapters use these during a frame; no fresh
    ///counter, sampled scalar mirror or second whole-World borrow is created.
    pub fn with_process_effects<R>(&self,call:impl FnOnce(&mut crate::presentation_timing::State,&mut crate::heartbeat_process::Counter)->Result<R,String>)->Result<R,String>{
        let mut timing=self.presentation_timing.try_borrow_mut().map_err(|e|format!("same presentation timing loan: {e}"))?;
        let mut counter=self.heartbeat_counter.try_borrow_mut().map_err(|e|format!("same heartbeat process loan: {e}"))?;
        call(&mut **timing,&mut **counter)
    }
    pub fn with_native_order<R>(
        &self,
        call: impl FnOnce(OrderLoans<'_>) -> Result<R, String>,
    ) -> Result<R, String> {
        let mut b = self
            .world
            .try_borrow_mut()
            .map_err(|e| format!("actual frame order loan: {e}"))?;
        let WorldLoan {
            state, retained, ..
        } = &mut *b;
        if state.stopped() {
            return Err("native order refused after partially committed world update".into());
        }
        let (sync_fields,revision,spatial_owner,mind_runtime)=retained.world.split_initial_sync();
        let spatial = match spatial_owner {
            SpatialOwner::Adopted(v) if v.error.is_none() && v.pending.is_empty() => &mut v.world,
            _ => return Err("actual live placement owner is incomplete".into()),
        };
        call(OrderLoans {
            actors: &mut retained.actors,
            spatial,
            queues: &mut state.queues,
            rng: &mut state.rng,
            definitions: &mut retained.definitions,
            owners: &retained.owners,
            settings: &retained.settings,
            clock: state.world.clock,
            mind_runtime,
            sync_fields,revision,sync_clock:&state.world.clock,sync_precision:self.precision,
        })
    }
    /// One native75CA70 frame on the actual Node/Sim/application siblings.
    /// No mutable diagnostic Driver or copied Timeline enters this path.
    /// The small cell contains only the same temporary Sim loan, allowing
    /// native order callbacks to reach that owner between kernel reads.
    pub fn run<H: borrowed::Host + ?Sized, S: SessionRead + ?Sized, O: AudioOutput + ?Sized>(
        &self,
        sim: &RefCell<&mut crate::live_sim::Sim>,
        now: FloatBits,
        host: &mut H,
        inputs: TickInputs<'_, S, O>,
    ) -> simulation::Result<simulation::Action> {
        self.run_inner(None,sim,now,host,inputs)
    }
    ///Actual Catalog loan for scheduled source-bound Mind callbacks. The
    ///legacy entry retains its precise unbound scheduled-Mind boundary.
    pub fn run_with_catalog<H:borrowed::Host+?Sized,S:SessionRead+?Sized,O:AudioOutput+?Sized>(
        &self,catalog:&Catalog<'_>,sim:&RefCell<&mut crate::live_sim::Sim>,now:FloatBits,
        host:&mut H,inputs:TickInputs<'_,S,O>,
    )->simulation::Result<simulation::Action>{self.run_inner(Some(catalog),sim,now,host,inputs)}
    ///Headless harness entry: exactly the catalog-bound WorldSeams::world_update
    ///body that 75CA70's world-tick branch reaches (752AE0 -> 76F790 -> GWorld
    ///4F9970), on the same retained owners, queues, RNG and failure latch.
    ///It does NOT evaluate the admin timeline gate 76AF90 or Sim+29 freeze and
    ///omits the 76F790 AVI/sync-log/41C180 prefix, 76F600 end check and 752AE0
    ///duration history. Product frames must use run_with_catalog; this entry
    ///exists so a loaded World's tick can be exercised before the view/task
    ///handshake unfreezes the Sim. It mints no readiness or activation.
    pub fn world_update_4f9970<S:SessionRead+?Sized,O:AudioOutput+?Sized>(
        &self,catalog:&Catalog<'_>,inputs:TickInputs<'_,S,O>,
    )->simulation::Result<NativeClock>{
        {
            let b=self.world.try_borrow().map_err(|e|seam(format!("frame loan remains held: {e}")))?;
            if b.state.stopped(){return Err(clocked::Error::Stopped.into())}
            if b.retained.census_contaminated{return Err(seam("World was advanced by a census update; it is no longer native state"))}
            if self.precision!=b.state.world.clock_precision{return Err(seam("actual application/World thread precisions differ"))}
        }
        WorldBorrow{frame:self,inputs,catalog:Some(catalog),census:None}.world_update()
    }
    ///Census variant of world_update_4f9970: every unported scheduled Update or
    ///world-effect leaf is recorded in `sink` and skipped instead of stopping the
    ///World, so one run inventories all reached gaps. The first use latches the
    ///World as contaminated: product and strict updates refuse it afterwards.
    ///Results after the first recorded entry are NOT native behaviour.
    pub fn world_update_census<S:SessionRead+?Sized,O:AudioOutput+?Sized>(
        &self,catalog:&Catalog<'_>,inputs:TickInputs<'_,S,O>,sink:&RefCell<Vec<CensusEntry>>,
    )->simulation::Result<NativeClock>{
        {
            let mut b=self.world.try_borrow_mut().map_err(|e|seam(format!("frame loan remains held: {e}")))?;
            if b.state.stopped(){return Err(clocked::Error::Stopped.into())}
            if self.precision!=b.state.world.clock_precision{return Err(seam("actual application/World thread precisions differ"))}
            b.retained.census_contaminated=true;
        }
        WorldBorrow{frame:self,inputs,catalog:Some(catalog),census:Some(sink)}.world_update()
    }
    ///Harness/diagnostic probe, not a native order entry: one fresh 4FA590
    ///Actor creation (4FA030/4F9C10 and its InitCreate/PostCreate/registration
    ///children) on the SAME live owners through a [`Retained::with_staged`]
    ///loan. Returns the created GTID; reached unported children are errors.
    pub fn probe_create_actor(
        &self,
        catalog: &Catalog<'_>,
        startup: &crate::actor_creation_startup::StartupInputs,
        ids: &str,
        position_bits: [u32; 3],
        direction_bits: [u32; 2],
        owner: u32,
    ) -> Result<Option<u32>, String> {
        let mut b = self.world.try_borrow_mut().map_err(|e| format!("creation probe World loan: {e}"))?;
        let WorldLoan { state, retained, .. } = &mut *b;
        if state.stopped() {
            return Err("creation probe refused after failed World update".into());
        }
        if retained.census_contaminated {
            return Err("World was advanced by a census update; it is no longer native state".into());
        }
        let precision = self.precision;
        let live_sai = self.lend_sai()?;
        let (result, live_sai) = retained.with_staged(state, live_sai, |staged| {
            let canonical = catalog.actors.find(ids).ok_or_else(|| format!("{ids} is not an Actor definition"))?.id.clone();
            let token = staged.definitions.intern("ActorDefinition", &canonical)?;
            let mut params = crate::map_actor_creation::Params::new(crate::map_replacement_queue::DefinitionRef {
                token,
                canonical_ids: canonical,
            });
            params.position_bits = position_bits;
            params.direction_bits = direction_bits;
            params.owner = owner;
            let key = crate::map_actor_creation::create(staged, catalog, &mut params, startup, precision)?;
            Ok(key.and_then(|k| staged.actors.id(k)).map(|id| id.raw()))
        });
        let returned = self.return_sai(live_sai);
        match (result, returned) {
            (Ok(v), Ok(())) => Ok(v),
            (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
            (Err(e), Err(r)) => Err(format!("{e}; {r}")),
        }
    }
    ///Retained 404F60-family debug-log calls made by scheduled Updates.
    pub fn native_log(&self)->Result<Vec<crate::live_updates::NativeLog>,String>{
        Ok(self.world.try_borrow().map_err(|e|format!("frame loan remains held: {e}"))?.retained.log.clone())
    }
    fn run_inner<'catalog_source,H:borrowed::Host+?Sized,S:SessionRead+?Sized,O:AudioOutput+?Sized>(
        &self,catalog:Option<&Catalog<'catalog_source>>,sim:&RefCell<&mut crate::live_sim::Sim>,now:FloatBits,
        host:&mut H,inputs:TickInputs<'_,S,O>,
    )->simulation::Result<simulation::Action>{
        let precision = self.precision;
        {
            let b = self
                .world
                .try_borrow()
                .map_err(|e| seam(format!("frame loan remains held: {e}")))?;
            if b.state.stopped() {
                return Err(clocked::Error::Stopped.into());
            }
            if b.retained.census_contaminated {
                return Err(seam("World was advanced by a census update; it is no longer native state"));
            }
            if precision != b.state.world.clock_precision {
                return Err(seam("actual application/World thread precisions differ"));
            }
            sim.try_borrow()
                .map_err(seam)?
                .require_created_world(&b.retained.world.revision)
                .map_err(seam)?;
        }
        let mut control = self.control.try_borrow_mut().map_err(seam)?;
        let mut same_sim = SharedSim::new(sim);
        let mut world = WorldBorrow {
            frame: self,
            inputs,
            catalog,
            census: None,
        };
        borrowed::Kernel {
            sim: &mut same_sim,
            control: control.control_loan(),
            precision,
        }
        .frame(now, host, &mut world)
    }
}
fn seam(error: impl std::fmt::Display) -> simulation::Error {
    simulation::Error::Seam(error.to_string())
}
struct WorldBorrow<'a, 'b, 'source, 'inputs, 'catalog_source, S: SessionRead + ?Sized, O: AudioOutput + ?Sized> {
    frame: &'a Frame<'b, 'source>,
    inputs: TickInputs<'inputs, S, O>,
    catalog:Option<&'a Catalog<'catalog_source>>,
    census:Option<&'a RefCell<Vec<CensusEntry>>>,
}
impl<S: SessionRead + ?Sized, O: AudioOutput + ?Sized> WorldSeams
    for WorldBorrow<'_, '_, '_, '_, '_, S, O>
{
    fn world_clock(&self) -> Option<NativeClock> {
        //Driver calls outside every completed synchronous callback loan. A
        //violating reentrant caller is outside this valid sequential domain;
        //never disguise its borrow violation as an absent native GWorld.
        Some(self.frame.world.borrow().state.world.clock)
    }
    fn world_update(&mut self) -> simulation::Result<NativeClock> {
        let session = self.inputs.session.try_borrow().map_err(seam)?;
        let mut output = self.inputs.audio.try_borrow_mut().map_err(seam)?;
        let mut b = self.frame.world.try_borrow_mut().map_err(seam)?;
        let WorldLoan {
            state,
            retained,
            first_tick_source,
            trigger_program,
            music,
        } = &mut *b;
        //76AC90 is Session+F8. The same Session loan stays immutable during
        //this supported CoreProgram interval; no phase-changing world opcode
        //is silently implemented by the manager-local evaluator.
        let phase = session.current_phase().map_err(seam)?;
        let program = trigger_program.get_or_insert_with(|| {
            CoreProgram::new(
                state.world.clock.begin.0,
                phase,
                state.world.clock_precision,
            )
        });
        program.application_mode = phase;
        let mut gate = || {
            let mut loan=self.inputs.sync_gate.try_borrow_mut().map_err(|e|e.to_string())?;
            (**loan)()
        };
        if let Some(catalog)=self.catalog {
            let tick=state.world.clock.tick;
            let mut services=frame_services::Services{catalog,retained,source:first_tick_source,
                program,music,session:&*session,output:&mut **output,sync_a:self.inputs.sync_a,gate:&mut gate,
                census:self.census.map(|sink|(sink,tick))};
            return Ok(state.step(&mut services)?);
        }
        let world =&mut *retained.world;
        let spatial = match &mut world.spatial {
            SpatialOwner::Adopted(v) if v.error.is_none() && v.pending.is_empty() => &mut v.world,
            _ => return Err(seam("actual live spatial owner is absent during update")),
        };
        let fields = world
            .runtime_fields
            .frame_fields(world.variables.cheat_fog_word())
            .map_err(seam)?;
        let mut first_tick = live_first_tick::Context {
            session: &*session,
            source: first_tick_source,
            bank: &retained.definitions,
            settings: &retained.settings,
            requests: music,
            output: &mut **output,
        };
        let mut services = live_clocked::Services {
            actors: &mut retained.actors,
            owners: &retained.owners,
            spatial,
            registration: &mut world.registration,
            fields,
            sync: live_clocked::SyncBinding::Shared {
                owner: self.inputs.sync_a,
                gate: &mut gate,
            },
            triggers: world
                .trigger_manager
                .as_mut()
                .ok_or_else(|| seam("retained trigger manager is absent"))?,
            trigger_program: program,
            first_tick: &mut first_tick,
            sun: None,
        };
        Ok(state.step(&mut services)?)
    }
}
