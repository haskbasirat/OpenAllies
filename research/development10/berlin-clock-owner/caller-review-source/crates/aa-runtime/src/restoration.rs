//! A stopped, owned map restoration attempt. This joins the real source catalog,
//! localization, saved IDs, common component state, world clock and random stream.
//! It cannot be stepped or published as a battle until all later Load/PostLoad,
//! manager, visibility, actor and command services are implemented and verified.
use crate::{catalog::Catalog, localization::Localization, settings::Settings, updatables::Table};
use aa_actors::IdentityRegistry;
use aa_mapworld::{
    MapArchive, WorldInventory,
    restore::{RestoreDefinitions, RestoreMode, RestoreReport, RestoredRecord},
    restore_components::UpdatableTarget,
};
use aa_numeric::Precision;
use aa_rules::actors::ActorCatalog;
use aa_simulation::{FloatBits, NativeClock, NativeRng};
use aa_world::{
    initialization::ClockPlan,
    queues::{Clock, Queues},
};

/// Economy part of GKingdom::Load 0x4E8550 for one restored Kingdom: the
/// statistics objects it loads into +0x330 ("strategic", 0x58EE80) and
/// +0x344 ("tactical", 0x58DBE0), promoted at world start by
/// `KingdomStatistics::start_world` (0x4F1E10), and the v7+ property-record
/// dispositions (0x4E8A09..0x4E8C2F) in saved order.
#[derive(Clone, Debug)]
pub struct KingdomEconomyLoad {
    pub statistics: aa_economy::statistics::KingdomStatistics,
    pub properties: Vec<aa_economy::kingdom_load::LoadedPropertyRecord>,
    /// Kingdom +338 statistics snapshots in list order (4E8E20..4E8EC6).
    pub snapshots: Vec<StatisticsSnapshotEntry>,
}
/// One native head-to-tail snapshot list, preserving saved entries before
/// newly captured snapshots without imposing the old inline ten-resource cap.
#[derive(Clone, Debug)]
pub enum StatisticsSnapshotEntry {
    Loaded(StatisticsSnapshot),
    Live(aa_economy::statistics_live::Snapshot),
}
/// One 0x160-byte snapshot: constructor 58F360(kingdom +344) then Load
/// 58F2E0 (version 0, Statistics 58EE80 at +4, +10C..+130, +134,
/// +138..+15C).
#[derive(Clone, Debug)]
pub struct StatisticsSnapshot {
    /// +0: the kingdom's +344 relation statistics object at load time (the
    /// same object 4E3690 later promotes to +340), or null.
    pub references_relations: bool,
    pub statistics: aa_economy::statistics::Statistics,
    pub words_10c: [u32; 10],
    pub word_134: u32,
    pub words_138: [u32; 10],
}
/// Builds [`KingdomEconomyLoad`] for every restored Kingdom (keyed by its
/// saved ID). The Kingdom constructor leaves all four statistics pointers
/// null; only a v10+ Load (no +190 word) creates the loaded objects.
pub fn kingdom_economy(
    registry: &IdentityRegistry<RestoredRecord>,
    resource_count: usize,
    world_version: u32,
) -> Result<std::collections::BTreeMap<u32, KingdomEconomyLoad>, Error> {
    use aa_economy::{
        F32Bits,
        statistics::{KingdomStatistics, RelationStatistics, Statistics},
    };
    let mut out = std::collections::BTreeMap::new();
    for entry in registry.iter() {
        let RestoredRecord::Kingdom(kingdom) = &entry.value else {
            continue;
        };
        let payload = &kingdom.source.payload;
        let mut statistics = KingdomStatistics {
            active: None,
            loaded: None,
            relations: None,
            loaded_relations: None,
        };
        let mut snapshots = Vec::new();
        if let Some(ai) = &payload.ai {
            let mut loaded = Statistics::new(resource_count);
            let words: Vec<F32Bits> = ai.strategic.words.iter().map(|w| F32Bits(*w)).collect();
            loaded
                .load_saved(ai.strategic.version, &words)
                .map_err(|e| Error::Economy(format!("{e:?}")))?;
            statistics.loaded = Some(loaded);
            if let Some(tactical) = &ai.tactical {
                if tactical
                    .definition_groups
                    .iter()
                    .any(|g| !g.values.is_empty())
                    || tactical.handle_groups.iter().any(|g| !g.values.is_empty())
                {
                    return Err(Error::Economy(
                        "nonempty saved relation statistics need the definition/GTID key binding"
                            .into(),
                    ));
                }
                let empty = |n: usize| vec![Vec::new(); n];
                let mut relations = RelationStatistics::default();
                relations
                    .load_saved(
                        tactical.version,
                        &empty(tactical.definition_groups.len()),
                        &empty(tactical.handle_groups.len()),
                    )
                    .map_err(|e| Error::Economy(format!("{e:?}")))?;
                statistics.loaded_relations = Some(relations);
            }
            for entry in &ai.entries.values {
                // 58F360 leaves the inline Statistics (+4..) unset; a version-0
                // record keeps score 55 from that uninitialized allocation.
                if entry.state.version == 0 {
                    return Err(Error::Economy(
                        "version-0 snapshot statistics keep uninitialized heap words".into(),
                    ));
                }
                let mut snapshot = Statistics::new(resource_count);
                let words: Vec<F32Bits> = entry.state.words.iter().map(|w| F32Bits(*w)).collect();
                snapshot
                    .load_saved(entry.state.version, &words)
                    .map_err(|e| Error::Economy(format!("{e:?}")))?;
                let w = &entry.words_10c;
                if w.len() != 21 {
                    return Err(Error::Economy("snapshot tail is not 21 words".into()));
                }
                snapshots.push(StatisticsSnapshotEntry::Loaded(StatisticsSnapshot {
                    references_relations: statistics.loaded_relations.is_some(),
                    statistics: snapshot,
                    words_10c: w[0..10].try_into().expect("10 words"),
                    word_134: w[10],
                    words_138: w[11..21].try_into().expect("10 words"),
                }));
            }
        }
        let sources: Vec<Option<u32>> = payload
            .properties
            .as_ref()
            .map(|list| list.values.iter().map(|p| p.word_08).collect())
            .unwrap_or_default();
        out.insert(
            entry.id.raw(),
            KingdomEconomyLoad {
                snapshots,
                statistics,
                properties: aa_economy::kingdom_load::loaded_property_records(
                    world_version,
                    &sources,
                ),
            },
        );
    }
    Ok(out)
}
pub struct Attempt {
    pub resource_streams:Option<crate::map_resource_streams::Owner>,
    pub scheduler_owners:crate::scheduler_owners::Owners,
    pub load_world: Option<crate::load_world::World>,
    pub native_load_mode: Option<u32>,
    /// Owned fresh managed load interval, when explicitly supplied. Its SAI
    /// remains inactive; it does not certify per-record Init or publication.
    pub inactive_load: Option<crate::restore_lifecycle::InactiveLoad>,
    pub resource_source: Option<std::sync::Arc<aa_rules::resources::ResourceCatalog>>,
    pub inventory: WorldInventory,
    pub kingdom_economy: std::collections::BTreeMap<u32, KingdomEconomyLoad>,
    /// TriggerManager 54D3D0 over the map's TRIGGERS chunk, localized through
    /// MAPSTRINGS with the LanguageIDS application variable.
    pub triggers: Option<aa_triggers::decode::SavedManager>,
    pub registry: IdentityRegistry<RestoredRecord>,
    pub abilities: crate::saved_performer::AbilityArena,
    pub updatables: Table<UpdatableTarget>,
    pub definitions: crate::definitions::Bank,
    pub clock: NativeClock,
    pub queues: Queues,
    pub rng: NativeRng,
    pub report: RestoreReport,
    /// Source tables the later 4FC600 steps read.
    pub settings: Settings,
    /// Definition-time text (before MAPSTRINGS): ThingData `+10` names.
    pub definition_text: Localization,
    /// Same combined global/MAPSTRINGS table used by record restoration.
    pub live_text:Localization,
    /// The UPDATABLES chunk; its queue sections load after the records.
    pub updatables_chunk: Vec<u8>,
    /// Receipt for the native queue Load between SUN and MISC_DATA. The
    /// diagnostic codec path leaves this absent until Staged::post_load.
    pub saved_queue_load:Option<usize>,
}
/// Staged common ownership after moving restored component boxes into the live
/// arena. Partial Load remains partial; this type deliberately has no step API.
pub struct Staged {
    pub resource_streams:Option<crate::map_resource_streams::Owner>,
    pub load_world: Option<crate::load_world::World>,
    pub native_load_mode: Option<u32>,
    pub inactive_load: Option<crate::restore_lifecycle::InactiveLoad>,
    pub resource_source: Option<std::sync::Arc<aa_rules::resources::ResourceCatalog>>,
    pub inventory: WorldInventory,
    pub kingdom_economy: std::collections::BTreeMap<u32, KingdomEconomyLoad>,
    pub triggers: Option<aa_triggers::decode::SavedManager>,
    pub actors: crate::actor_world::ActorStore,
    pub owners: crate::scheduler_owners::Owners,
    pub updatables: Table<aa_world::queues::Owner>,
    pub definitions: crate::definitions::Bank,
    pub clock: NativeClock,
    pub queues: Queues,
    pub rng: NativeRng,
    pub report: RestoreReport,
    pub settings: Settings,
    pub definition_text: Localization,
    pub live_text:Localization,
    pub updatables_chunk: Vec<u8>,
    pub saved_queue_load:Option<usize>,
}
/// Explicit caller-owned source/lifecycle inputs for record restoration. The
/// default preserves standalone diagnostics with an unknown SAI lifetime and
/// no bound Resource metadata. No live resource arrays are created by this.
#[derive(Default)]
pub struct LoadBindings {
    pub native_load_mode: Option<u32>,
    pub inactive_load: Option<crate::restore_lifecycle::InactiveLoad>,
    pub resources: Option<std::sync::Arc<aa_rules::resources::ResourceCatalog>>,
}
/// What [`Staged::post_load`] did.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct PostLoadReport {
    /// UPDATABLES entries whose saved ID names no Updatable (the native
    /// null-slot skip of the queue Load).
    pub discarded_updates: usize,
    /// Objects registered when the pass began (no ported PostLoad creates
    /// or removes one, so each had its `+DC` called once).
    pub objects: usize,
    /// Logs and map warnings written by the PostLoads, in order.
    pub messages: Vec<crate::post_load_world::PostLoadMessage>,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Map(#[from] aa_mapworld::Error),
    #[error(transparent)]
    Identity(#[from] aa_actors::registry::RegistryError),
    #[error(transparent)]
    Updatable(#[from] crate::updatables::Error),
    #[error(transparent)]
    Localization(#[from] crate::localization::Error),
    #[error(transparent)]
    Settings(#[from] crate::settings::Error),
    #[error(transparent)]
    Initialization(#[from] aa_world::initialization::Error),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("map triggers: {0}")]
    Triggers(String),
    #[error("kingdom economy load: {0}")]
    Economy(String),
    #[error("staging restored records: {0}")]
    Staging(String),
    #[error(transparent)]
    Owners(#[from] crate::scheduler_owners::Error),
    #[error(transparent)]
    SavedQueues(#[from] crate::saved_queues::Error),
    #[error("PostLoad precondition: {0}")]
    PostLoadState(String),
    #[error(transparent)]
    PostLoad(#[from] aa_actors::post_load::Error),
    #[error("unported native load step: {0}")]
    Unported(&'static str),
    #[error("managed load lifecycle: {0}")]
    Lifecycle(String),
    #[error("map restoration: {0}")]
    Invalid(&'static str),
}
impl Attempt {
    /// Run each saved record's real immediate lifecycle against persistent
    /// map owners. This is the load path used to advance playable battle work;
    /// its result still requires successful PostLoad and WorldStart.
    #[allow(clippy::too_many_arguments)]
    pub fn load_immediate(
        map:&MapArchive,actors:&ActorCatalog,settings:Settings,text:Localization,
        precision:Precision,max_chunk_bytes:u64,mode:RestoreMode,bindings:LoadBindings,
        sources:&mut aa_rules::sources::V14Sources,
        session:&dyn crate::load_world_globals::SessionPhase,
        duplicate_city_latch:bool,
    )->Result<Self,Error> {
        Self::load_with_context_setup(map,actors,settings,text,precision,max_chunk_bytes,mode,
            bindings,Some(session),|catalog,inventory| {
                prepare_immediate_world(catalog,sources,map,inventory,duplicate_city_latch,precision,None)
            })
    }
    /// Actual owned read-only resource backend. Its one archive is borrowed
    /// throughout restoration and the same two stream views move to Staged.
    #[allow(clippy::too_many_arguments)]
    pub fn load_immediate_owned(
        streams:crate::map_resource_streams::Owner,actors:&ActorCatalog,settings:Settings,text:Localization,
        precision:Precision,max_chunk_bytes:u64,mode:RestoreMode,bindings:LoadBindings,
        sources:&mut aa_rules::sources::V14Sources,
        session:&dyn crate::load_world_globals::SessionPhase,duplicate_city_latch:bool,
    )->Result<Self,Error> {
        Self::load_immediate_owned_runtime(streams,actors,settings,text,precision,max_chunk_bytes,
            mode,bindings,sources,session,duplicate_city_latch,None,None,None)
    }
    /// The same owned load, called inside the application's actual resource
    /// worker. Both callbacks are borrowed from that running thread.
    #[allow(clippy::too_many_arguments)]
    pub fn load_immediate_owned_runtime(
        streams:crate::map_resource_streams::Owner,actors:&ActorCatalog,settings:Settings,text:Localization,
        precision:Precision,max_chunk_bytes:u64,mode:RestoreMode,bindings:LoadBindings,
        sources:&mut aa_rules::sources::V14Sources,
        session:&dyn crate::load_world_globals::SessionPhase,duplicate_city_latch:bool,
        execution:Option<&mut dyn crate::execution_context::ExecutionIds>,
        environment:Option<crate::load_performer_value_refresh::EnvironmentSampler>,
        world_sink:Option<&mut dyn crate::load_sim_world::WorldConstructorSink>,
    )->Result<Self,Error> {
        let witness=streams.witness();
        let map=streams.archive().map_err(Error::Lifecycle)?;
        let mut attempt=Self::load_with_runtime_setup(map,actors,settings,text,precision,max_chunk_bytes,mode,
            bindings,Some(session),execution,environment,None,None,None,|catalog,inventory| {
                *catalog.load_archive.try_borrow_mut().map_err(|_|"archive witness already borrowed")?=Some(witness);
                prepare_immediate_world(catalog,sources,map,inventory,duplicate_city_latch,precision,world_sink)
            })?;
        attempt.resource_streams=Some(streams);
        Ok(attempt)
    }
    /// Actual selected-resource load using the same completed source Catalog
    /// previously borrowed by the lobby. Resource constructor/header precede
    /// the World constructor; final current Creator selection precedes4FC600.
    /// A complete attempt still needs PostLoad, mission start and activation.
    #[allow(clippy::too_many_arguments)]
    pub fn load_selected_owned_runtime(
        streams:crate::map_resource_streams::Owner,catalog:Catalog<'_>,
        selected:&crate::resource_creator::Creator,precision:Precision,max_chunk_bytes:u64,
        mode:RestoreMode,bindings:LoadBindings,sources:&mut aa_rules::sources::V14Sources,
        session:&dyn crate::load_world_globals::SessionPhase,duplicate_city_latch:bool,
        execution:&mut dyn crate::execution_context::ExecutionIds,
        environment:crate::load_performer_value_refresh::EnvironmentSampler,
        world_sink:&mut dyn crate::load_sim_world::WorldConstructorSink,
        configuration_sync:crate::live_clocked::SyncBinding<'_>,
        configuration_sync_s:crate::live_clocked::SyncBinding<'_>,
        header_counter:&crate::film_process_metadata::HeaderConstructorCounter,
    )->Result<Self,Error> {
        let witness=streams.witness();
        let map=streams.archive().map_err(Error::Lifecycle)?;
        let mut attempt=Self::load_with_prepared_source_setup(map,catalog,precision,max_chunk_bytes,
            mode,bindings,session,Some(execution),Some(environment),Some(configuration_sync),Some(configuration_sync_s),|catalog,inventory| {
                *catalog.load_archive.try_borrow_mut().map_err(|_|"archive witness already borrowed")?=Some(witness.clone());
                let (mut resource,map_mode)=crate::map_resource_load::Owner::fresh_loading_archive(
                    catalog.native_load_mode.ok_or("selected resource task mode missing")?,
                    &catalog.actors.sources,Some(witness))?;
                resource.bind_fresh_factory_ids_5a28b0()?;
                resource.bind_header_constructor_counter(header_counter)?;
                resource.prepare_creator_header(catalog,map,Some(selected))?;
                let proof=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(catalog,sources)?;
                {
                    let bank=catalog.definition_bank.try_borrow().map_err(|_|"source Bank already borrowed")?;
                    let current=bank.combat_source()?;
                    current.completed_means()?;
                    if current.context().reference_startup()!=proof.context().reference_startup()
                        ||current.source_precision()!=proof.context().precision()
                        ||current.source_identities()!=catalog.actors.sources {
                        return Err("selected resource source context differs from same completed Bank".into());
                    }
                }
                let point=std::sync::Arc::new(crate::load_world_globals::PointShape::from_verified_startup_reference(catalog,&proof)?);
                let sun=crate::load_sun_reference::ConstructorInput::from_verified_startup_reference(catalog,&proof)?.create_sun(catalog)?;
                let world=crate::load_world::World::before_first_record_prepared(catalog,sources,map,
                    inventory,point,sun,duplicate_city_latch,precision,Some(world_sink),(resource,map_mode),selected)?;
                *catalog.load_world.try_borrow_mut().map_err(|_|"selected World preparation already borrowed")?=Some(world);
                Ok(())
            })?;
        attempt.resource_streams=Some(streams);
        Ok(attempt)
    }
    pub fn into_staged(mut self) -> Result<Staged, Error> {
        let mut actors = crate::actor_world::ActorStore::from_restored_with_abilities(
            self.registry,
            self.abilities,
        )
        .map_err(|e| Error::Staging(e.to_string()))?;
        if let Some(world)=self.load_world.as_mut() {
            actors.property_allocations=world.property_allocations.take()
                .ok_or(Error::Invalid("actual load property allocation owner already transferred"))?;
            //Move the loaded directory once into the runtime retained with
            //the same World before any fresh replacement can reset a Mind.
            world.mind_runtime.bind_restored_commands(&mut actors)
                .map_err(|e|Error::Staging(e.to_string()))?;
        }
        let mut owners = self.scheduler_owners;
        let updatables = owners.bind_saved_table_with_sun(&self.updatables, &actors,
            self.load_world.as_ref().map(|world|&world.sun))?;
        Ok(Staged {
            resource_streams:self.resource_streams,
            load_world: self.load_world,
            native_load_mode: self.native_load_mode,
            inactive_load: self.inactive_load,
            resource_source: self.resource_source,
            inventory: self.inventory,
            kingdom_economy: self.kingdom_economy,
            triggers: self.triggers,
            actors,
            owners,
            updatables,
            definitions: self.definitions,
            clock: self.clock,
            queues: self.queues,
            rng: self.rng,
            report: self.report,
            settings: self.settings,
            definition_text: self.definition_text,
            live_text:self.live_text,
            updatables_chunk: self.updatables_chunk,
            saved_queue_load:self.saved_queue_load,
        })
    }
    pub fn load(
        map: &MapArchive,
        actors: &ActorCatalog,
        settings: Settings,
        text: Localization,
        precision: Precision,
        max_chunk_bytes: u64,
    ) -> Result<Self, Error> {
        Self::load_with_mode(
            map,
            actors,
            settings,
            text,
            precision,
            max_chunk_bytes,
            RestoreMode::StopOnError,
        )
    }
    /// `RestoreMode::InspectPartial` is a diagnostic census of every source
    /// record's first unsupported operation. Its result is never publishable:
    /// records after a failure may observe earlier partial state.
    pub fn load_with_mode(
        map: &MapArchive,
        actors: &ActorCatalog,
        settings: Settings,
        text: Localization,
        precision: Precision,
        max_chunk_bytes: u64,
        mode: RestoreMode,
    ) -> Result<Self, Error> {
        Self::load_with_native_mode(
            map, actors, settings, text, precision, max_chunk_bytes, mode, None,
        )
    }
    /// Explicit original MapResource+A8 input. Existing callers keep None;
    /// mode-sensitive callbacks fail rather than assuming game/editor mode.
    /// InspectPartial remains diagnostic even when this input is supplied.
    pub fn load_with_native_mode(
        map: &MapArchive,
        actors: &ActorCatalog,
        settings: Settings,
        text: Localization,
        precision: Precision,
        max_chunk_bytes: u64,
        mode: RestoreMode,
        native_load_mode: Option<u32>,
    ) -> Result<Self, Error> {
        Self::load_with_bindings(map, actors, settings, text, precision, max_chunk_bytes, mode,
            LoadBindings { native_load_mode, ..LoadBindings::default() })
    }
    /// Restore record codecs/callbacks in an explicitly owned stopped load
    /// interval. This does not bypass missing immediate Init/PostCreate or
    /// authorize publication, task completion, world-ready or active SAI.
    pub fn load_with_bindings(
        map: &MapArchive,
        actors: &ActorCatalog,
        settings: Settings,
        text: Localization,
        precision: Precision,
        max_chunk_bytes: u64,
        mode: RestoreMode,
        bindings: LoadBindings,
    ) -> Result<Self, Error> {
        Self::load_with_source_setup(
            map, actors, settings, text, precision, max_chunk_bytes, mode, bindings,
            |_| Ok(()),
        )
    }
    /// Bind application-owned source state before the first saved record is
    /// constructed. The same definition Bank moves into this Attempt; this is
    /// not a second cache rebuilt after Load. The ordinary entry point above
    /// deliberately leaves optional source completion unavailable for diagnostic
    /// callers. A failed preparation returns before any record is registered.
    #[allow(clippy::too_many_arguments)]
    pub fn load_with_source_setup(
        map: &MapArchive,
        actors: &ActorCatalog,
        settings: Settings,
        text: Localization,
        precision: Precision,
        max_chunk_bytes: u64,
        mode: RestoreMode,
        bindings: LoadBindings,
        prepare: impl FnOnce(&Catalog<'_>) -> Result<(), String>,
    ) -> Result<Self, Error> {
        Self::load_with_context_setup(map,actors,settings,text,precision,max_chunk_bytes,mode,
            bindings,None,|catalog,_|prepare(catalog))
    }

    /// The application lends its actual Session during the native immediate
    /// lifecycle. Preparation owns the same pre-record world owners afterward
    /// moved into Attempt, including partial prefixes on a record error.
    #[allow(clippy::too_many_arguments)]
    pub fn load_with_context_setup(
        map:&MapArchive, actors:&ActorCatalog, settings:Settings, text:Localization,
        precision:Precision,max_chunk_bytes:u64,mode:RestoreMode,bindings:LoadBindings,
        session:Option<&dyn crate::load_world_globals::SessionPhase>,
        prepare:impl FnOnce(&Catalog<'_>,&WorldInventory)->Result<(),String>,
    )->Result<Self,Error> {
        Self::load_with_runtime_setup(map,actors,settings,text,precision,max_chunk_bytes,mode,
            bindings,session,None,None,None,None,None,prepare)
    }
    /// Ownership bridge from actual lobby/source setup. The whole Catalog,
    /// including its completed Bank and every cached source arena, is consumed.
    /// This grants no map-load or battle receipt; the caller must run the real
    /// resource/World preparation in native order through `prepare`.
    #[allow(clippy::too_many_arguments)]
    pub fn load_with_prepared_source_setup(
        map:&MapArchive,mut catalog:Catalog<'_>,precision:Precision,max_chunk_bytes:u64,
        mode:RestoreMode,bindings:LoadBindings,
        session:&dyn crate::load_world_globals::SessionPhase,
        execution:Option<&mut dyn crate::execution_context::ExecutionIds>,
        environment:Option<crate::load_performer_value_refresh::EnvironmentSampler>,
        configuration_sync:Option<crate::live_clocked::SyncBinding<'_>>,
        configuration_sync_s:Option<crate::live_clocked::SyncBinding<'_>>,
        prepare:impl FnOnce(&Catalog<'_>,&WorldInventory)->Result<(),String>,
    )->Result<Self,Error> {
        {
            let bank=catalog.definition_bank.try_borrow()
                .map_err(|_|Error::Lifecycle("prepared source Bank already borrowed".into()))?;
            let state=bank.combat_source().map_err(Error::Lifecycle)?;
            state.completed_means().map_err(Error::Lifecycle)?;
            if state.source_identities()!=catalog.actors.sources {
                return Err(Error::Lifecycle("prepared source identities differ from actual Catalog".into()));
            }
        }
        let actors=catalog.actors;
        if catalog.definition_text.is_none(){
            return Err(Error::Invalid("prepared source must retain its original definition-time text owner"));
        }
        let settings=catalog.settings.take().ok_or(Error::Invalid("prepared source Settings absent"))?;
        let text=catalog.localization.get_mut().take().ok_or(Error::Invalid("prepared source current text absent"))?;
        Self::load_with_runtime_setup(map,actors,settings,text,precision,max_chunk_bytes,mode,
            bindings,Some(session),execution,environment,configuration_sync,configuration_sync_s,Some(catalog),prepare)
    }
    #[allow(clippy::too_many_arguments)]
    fn load_with_runtime_setup(
        map:&MapArchive, actors:&ActorCatalog, settings:Settings, mut text:Localization,
        precision:Precision,max_chunk_bytes:u64,mode:RestoreMode,bindings:LoadBindings,
        session:Option<&dyn crate::load_world_globals::SessionPhase>,
        execution:Option<&mut dyn crate::execution_context::ExecutionIds>,
        environment:Option<crate::load_performer_value_refresh::EnvironmentSampler>,
        mut configuration_sync:Option<crate::live_clocked::SyncBinding<'_>>,
        mut configuration_sync_s:Option<crate::live_clocked::SyncBinding<'_>>,
        prepared_source:Option<Catalog<'_>>,
        prepare:impl FnOnce(&Catalog<'_>,&WorldInventory)->Result<(),String>,
    )->Result<Self,Error> {
        let LoadBindings { mut native_load_mode, inactive_load, resources } = bindings;
        if let Some(load) = &inactive_load {
            load.observer_present().map_err(Error::Lifecycle)?;
            let task_mode = load.native_load_mode();
            if native_load_mode.is_some_and(|actual| actual != task_mode) {
                return Err(Error::Lifecycle("managed task and native load mode disagree".into()));
            }
            native_load_mode = Some(task_mode);
        }
        let inventory = WorldInventory::decode(map, max_chunk_bytes)?;
        // Runtime never uses the inventory-only partial/census continuation.
        if inventory.header.is_none() || inventory.teams.is_none() || inventory.kingdoms.is_none() {
            return Err(Error::Invalid(
                "required source header/Team/Kingdom structure incomplete",
            ));
        }
        let (rate, _) = settings.unique_fixed("GameTicksPerGameSecond", precision)?;
        let plan = ClockPlan::from_tick_rate(rate, precision)?;
        let mut queues = plan.initialize_queues(plan.new_clock(), precision)?;
        let saved = &inventory.world;
        let clock = NativeClock {
            tick: saved.word_dc,
            begin: FloatBits(saved.begin_time_bits),
            end: FloatBits(saved.word_e4),
            step: FloatBits(saved.delta_bits),
        };
        aa_numeric::Binary::from_f32_bits(clock.begin.0)?;
        aa_numeric::Binary::from_f32_bits(clock.end.0)?;
        let step = aa_numeric::Binary::from_f32_bits(clock.step.0)?;
        if step.is_zero() || step.is_negative_nonzero() {
            return Err(Error::Invalid("saved world step must be positive"));
        }
        queues.clock = Clock {
            begin_bits: clock.begin.0,
            end_bits: clock.end.0,
            step_bits: clock.step.0,
        };
        let definition_text = text.clone();
        //The worker's5A2B90→5A2A40→5A19C0 repeats47EA40 on a
        //nonnull target Resource only when its loaded Header is not saved.
        //SourceTransfer may already carry the selected source's MAPSTRINGS;
        //478D20 removes that old map object before reading its replacement.
        let mapstrings=if !crate::map_resource_mode::world_rule_mode(
            inventory.header.as_ref().ok_or(Error::Invalid("required Header absent"))?.rules.mode,
        ).map_err(Error::Lifecycle)?.saved_state {
            text.replace_map_strings(
                ||map.decode_chunk("MAPSTRINGS",max_chunk_bytes).map_err(|e|e.to_string()),
                aa_rules::tgi::Limits::default(),
            )?
        }else{map.decode_chunk("MAPSTRINGS",max_chunk_bytes)?};
        // TriggerManager54D3D0 reads this MAPSTRINGS table after the dynamic
        // records. Resolve its actual language lazily at that reached boundary.
        let decode_triggers=|settings:&Settings| {
            let trigger_chunk=map.decode_chunk("TRIGGERS",max_chunk_bytes)?;
            let (language,_)=settings.language_ids()?;
            crate::trigger_world::load_map_triggers(&trigger_chunk.bytes,&mapstrings.bytes,language,
                usize::try_from(max_chunk_bytes).unwrap_or(usize::MAX))
                .map_err(|e|Error::Triggers(e.to_string()))
        };
        // Codec-only diagnostics historically decode independent chunks.
        // The immediate native path must preserve the later TRIGGERS boundary
        // and cannot manufacture a manager when record restoration stopped.
        let mut triggers=if session.is_none() {Some(decode_triggers(&settings)?)} else {None};
        let mut catalog=match prepared_source {
            Some(source)=>source.into_runtime_scope().map_err(|(error,_)|Error::Lifecycle(error))?,
            None=>Catalog::new(actors),
        };
        if !std::ptr::eq(catalog.actors,actors) {
            return Err(Error::Lifecycle("prepared load refers to another actual source Catalog".into()));
        }
        catalog.load_session = session;
        catalog.load_clock = Some(&clock);
        *catalog.load_execution.get_mut()=execution.map(|actual|Box::new(ExecutionLoan {actual}) as Box<dyn crate::execution_context::ExecutionIds>);
        catalog.runtime_environment=environment;
        if let Some(sync)=configuration_sync.as_mut() {catalog.bind_configuration_sync(sync.reborrow()).map_err(Error::Lifecycle)?;}
        if let Some(sync)=configuration_sync_s.as_mut() {catalog.bind_configuration_sync_s(sync.reborrow()).map_err(Error::Lifecycle)?;}
        *catalog.load_queues.get_mut()=Some(&mut queues);
        catalog.native_load_mode = native_load_mode;
        if let Some(load) = &inactive_load {
            catalog.bind_inactive_load(load).map_err(Error::Lifecycle)?;
        }
        if let Some(source) = &resources {
            catalog.bind_or_require_resource_catalog(source).map_err(Error::Economy)?;
        }
        catalog.replace_current_text(text);
        if catalog.definition_text.is_none(){catalog.definition_text = Some(definition_text);}
        catalog.settings = Some(settings);
        let updatables_chunk = map.decode_chunk("UPDATABLES", max_chunk_bytes)?;
        *catalog.updatables.get_mut() = Some(Table::from_chunk(&updatables_chunk)?);
        prepare(&catalog,&inventory).map_err(Error::Lifecycle)?;
        let mut registry = inventory.id_manager.restore_empty_registry()?;
        let mut rng = NativeRng::new(inventory.world.rng_state);
        let mut report = aa_mapworld::restore::restore_records_with_mode(
            &mut registry,
            &inventory,
            &catalog,
            &mut rng,
            precision,
            mode,
        );
        let mut saved_queue_load=None;
        if report.stopped_before.is_none() && report.events.iter().all(|e|e.error.is_none())
            && report.lifecycle.iter().all(|e|e.completed) {
            if triggers.is_none() {triggers=Some(decode_triggers(catalog.settings.as_ref()
                .ok_or(Error::Invalid("actual Settings disappeared before TRIGGERS"))?)?);}
            let mut load_world=catalog.load_world.try_borrow_mut().map_err(|_|Error::Lifecycle("SUN world is already borrowed".into()))?;
            if let Some(world)=load_world.as_mut() {
                world.install_trigger_payload(triggers.take().ok_or(Error::Invalid("decoded TRIGGERS payload disappeared"))?)
                    .map_err(Error::Lifecycle)?;
                let chunk=map.decode_chunk("SUN",max_chunk_bytes)?;
                world.load_sun(&catalog,&chunk,&mut report.updatable_registrations).map_err(Error::Lifecycle)?;
                // Native4FC600: SUN, deferred3/4 and periodic0/1/2 Loads,
                // then MISC_DATA. Keep pre-staging instance tokens so later
                // allocation moves cannot alter restored event identities.
                let table_borrow=catalog.updatables.try_borrow().map_err(|_|Error::Lifecycle("saved-ID table is borrowed during queue Load".into()))?;
                let table=table_borrow.as_ref().ok_or(Error::Invalid("saved-ID table is absent during queue Load"))?;
                let mut bound=Table::new(i32::try_from(table.len()).map_err(|_|Error::Invalid("saved-ID table count exceeds native format"))?)?;
                let mut owners=catalog.load_owners.try_borrow_mut().map_err(|_|Error::Lifecycle("Updatable owners are borrowed during queue Load".into()))?;
                for (id,target) in table.slots().iter().enumerate() {
                    if let Some(target)=target {bound.register(id as i32,owners.restoring(*target)?)?;}
                }
                let decoded=crate::saved_queues::decode(&updatables_chunk.bytes,inventory.world.version,&bound,|&owner|owner)?;
                let mut queues=catalog.load_queues.try_borrow_mut().map_err(|_|Error::Lifecycle("world queues are borrowed during queue Load".into()))?;
                saved_queue_load=Some(decoded.install(queues.as_deref_mut().ok_or(Error::Invalid("actual world queues are absent during queue Load"))?)?);
                drop(queues);
                drop(owners);
                drop(table_borrow);
                let chunk=map.decode_chunk("MISC_DATA",max_chunk_bytes)?;
                world.load_misc(&catalog,&chunk,inventory.world.version).map_err(Error::Lifecycle)?;
            }
        }
        // GKingdom::Load economy semantics (Kingdom records only; KINGDOMS
        // load before any dynamic record could raise GWorld+2C).
        let resource_count = catalog
            .resource_domain_count(precision)
            .map_err(Error::Economy)? as usize;
        let kingdom_economy =
            kingdom_economy(&registry, resource_count, inventory.world.version)?;
        let (load_world,scheduler_owners,definitions,abilities,settings,definition_text,live_text,updatables)=catalog.into_load_parts();
        let updatables=updatables.ok_or(Error::Invalid("saved Updatable table disappeared"))?;
        let settings=settings.ok_or(Error::Invalid("SVar source table disappeared"))?;
        let definition_text=definition_text.ok_or(Error::Invalid("definition-time text table disappeared"))?;
        let live_text=live_text.ok_or(Error::Invalid("actual live localization table disappeared"))?;
        Ok(Self {
            resource_streams:None,
            scheduler_owners,
            load_world,
            native_load_mode,
            inactive_load,
            resource_source: resources,
            inventory,
            kingdom_economy,
            triggers,
            registry,
            updatables,
            definitions,
            abilities,
            clock,
            queues,
            rng,
            report,
            settings,
            definition_text,
            live_text,
            updatables_chunk: updatables_chunk.bytes,
            saved_queue_load,
        })
    }
}

/// A borrowed adapter, not another captured ID or mutable execution owner.
pub(crate) struct ExecutionLoan<'a> {pub(crate) actual:&'a mut dyn crate::execution_context::ExecutionIds}
impl crate::execution_context::ExecutionIds for ExecutionLoan<'_> {
    fn current_os_thread_id(&mut self)->Result<u32,String>{self.actual.current_os_thread_id()}
    fn application_os_thread_id(&self)->u32{self.actual.application_os_thread_id()}
}
fn prepare_immediate_world(catalog:&Catalog<'_>,sources:&mut aa_rules::sources::V14Sources,
    map:&MapArchive,inventory:&WorldInventory,duplicate_city_latch:bool,precision:Precision,
    world_sink:Option<&mut dyn crate::load_sim_world::WorldConstructorSink>)->Result<(),String> {
    let proof=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(catalog,sources)?;
    crate::combat_source_completion::install(catalog,sources,proof.context(),proof.csv(),proof.globals())?;
    crate::combat_source_completion::compute(catalog)?;
    let point=std::sync::Arc::new(crate::load_world_globals::PointShape::from_verified_startup_reference(catalog,&proof)?);
    let sun=crate::load_sun_reference::ConstructorInput::from_verified_startup_reference(catalog,&proof)?.create_sun(catalog)?;
    let world=crate::load_world::World::before_first_record(catalog,sources,map,inventory,
        point,sun,duplicate_city_latch,precision,world_sink)?;
    *catalog.load_world.try_borrow_mut().map_err(|_|"immediate world is borrowed during preparation")?=Some(world);
    Ok(())
}
impl Staged {
    /// GWorld::Load `4FC600` after the record sections: the UPDATABLES queue
    /// Load (version >= 11: deferred queues 3 and 4, from version 22 also the
    /// periodic queues 0, 1, 2; gated on the version `4FC600` read at its
    /// start, not the raised GWorld `+2C`), then the PostLoad pass
    /// `4FD3B3..4FD3F1` (`aa_actors::post_load::post_load_pass`).
    ///
    /// Immediate loads already restored SUN, these queues and MISC_DATA in
    /// native order. Their queue receipt prevents a second Load from erasing
    /// events scheduled by later callbacks. The codec-only diagnostic path
    /// still loads its queues here. The staged world stays unpublishable because
    /// [`Self::init_for_loaded_map`] is unported.
    ///
    /// Preconditions: every record section completed (no stop, no failed
    /// record) and every object awaits its PostLoad. On a pass error the
    /// effects of earlier objects stay committed, as natively; the staged
    /// world must then be discarded.
    pub fn post_load(
        &mut self,
        actors: &ActorCatalog,
        precision: Precision,
    ) -> Result<PostLoadReport, Error> {
        use crate::actor_world::Source;
        use aa_mapworld::restore::LoadCompletion;
        if self.report.stopped_before.is_some()
            || self.report.events.iter().any(|e| e.error.is_some())
        {
            return Err(Error::PostLoadState(
                "the record sections did not complete".into(),
            ));
        }
        let keys: Vec<_> = self
            .actors
            .identities()
            .iter()
            .map(|entry| (entry.id, entry.value))
            .collect();
        for (id, key) in &keys {
            let entity = self
                .actors
                .entity(*key)
                .map_err(|e| Error::Staging(e.to_string()))?;
            let completion = match &entity.source {
                Source::Loaded(v) => v.completion,
                Source::Special(v) => v.completion,
                Source::New { .. } | Source::NewObject { .. } => {
                    return Err(Error::PostLoadState(format!(
                        "{:#x} is a fresh object",
                        id.raw()
                    )));
                }
            };
            if completion != LoadCompletion::AwaitingPostLoad {
                return Err(Error::PostLoadState(format!(
                    "{:#x} is {completion:?}, not awaiting PostLoad",
                    id.raw()
                )));
            }
        }
        let discarded_updates=match self.saved_queue_load {
            Some(discarded)=>discarded,
            None=> {
                let decoded=crate::saved_queues::decode(&self.updatables_chunk,
                    self.inventory.world.version,&self.updatables,|&owner|owner)?;
                let discarded=decoded.install(&mut self.queues)?;
                self.saved_queue_load=Some(discarded);
                discarded
            }
        };
        let mut report = PostLoadReport {
            discarded_updates,
            ..PostLoadReport::default()
        };
        let mut world = crate::post_load_world::World {
            actors: &mut self.actors,
            owners: &mut self.owners,
            queues: &mut self.queues,
            rng: &mut self.rng,
            definitions: &self.definitions,
            catalog: actors,
            definition_text: &self.definition_text,
            settings: &self.settings,
            world_version: self.report.world_version,
            precision,
            messages: &mut report.messages,
        };
        aa_actors::post_load::post_load_pass(&mut world)?;
        report.objects = keys.len();
        for (_, key) in keys {
            let entity = self
                .actors
                .entity_mut(key)
                .map_err(|e| Error::Staging(e.to_string()))?;
            match &mut entity.source {
                Source::Loaded(v) => v.completion = LoadCompletion::PostLoaded,
                Source::Special(v) => v.completion = LoadCompletion::PostLoaded,
                Source::New { .. } | Source::NewObject { .. } => {}
            }
        }
        Ok(report)
    }
    /// `4FD3F3..4FD42D`: `4F1E10` (only when map settings `+A8` is 1 or 4
    /// and `59E520`), VWorldState::InitForLoadedMap `4F5290`/`4F1C20`, the
    /// `[4610] InitForLoadedMap` log, then the class-conversion replacement
    /// tail (`4F9C10` per converted record). Not ported.
    pub fn init_for_loaded_map(&mut self) -> Result<(), Error> {
        Err(Error::Unported(
            "4F1E10 / InitForLoadedMap 4F5290, 4F1C20 and the conversion replacement tail",
        ))
    }
}
