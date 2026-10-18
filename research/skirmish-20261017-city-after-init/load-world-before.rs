//! Persistent owners for the immediate native saved-record lifecycle.
//!
//! Each callback borrows these owners and the actual record held outside its
//! Loading registry entry. The session is supplied by its application owner at
//! the call, never copied into map metadata. This type alone is not readiness.
use crate::{
    battle_terrain::{self, BattleTerrain}, catalog::Catalog,
    live_object_init::Record, live_placement::SettingsMode,
    live_registration, load_actor_init_world, load_object_init_world,
    load_spatial::LoadSpatial, load_world_globals::{PointShape, SessionPhase},
    load_zone_services, world_object_list::WorldObjectList,
};
use aa_actors::{ActorId, loading::{LoadKind, LoadShell}};
use aa_mapworld::{MapArchive, WorldInventory, bootstrap::DefinitionProof,
    header::MapHeader, restore::{RestoredComponent, RestoredRecord, RuntimeDefinition},
    restore_lifecycle::Context};
use aa_numeric::Precision;
use aa_rules::{native_source_requirements::NativeSourceRequirements, sources::V14Sources};
use aa_zones::tile::ZoneMap;
use std::sync::Arc;

type Result<T> = std::result::Result<T, String>;

/// No Clone/Default: the same list, index, zone map and Kingdom arrays move
/// from Catalog to Attempt to Staged. They are never reconstructed by GTID sort.
pub struct World {
    pub(crate) revision:crate::world_revision::ConstructedWorldRevision,
    pub(crate) header: MapHeader,
    pub(crate) terrain: BattleTerrain,
    pub(crate) spatial: SpatialOwner,
    pub(crate) zones: ZoneMap,
    pub(crate) zone_identities:crate::load_detection_activation::Identities,
    pub(crate) zone_globals:crate::load_zone_globals::State,
    pub(crate) variables:crate::load_view_variables::State,
    pub(crate) runtime_fields:crate::load_world_runtime_fields::State,
    pub(crate) mission_fields:crate::mission_fields::State,
    pub(crate) world_rules:crate::load_world_rules::Owner,
    pub(crate) configuration:crate::configure_world::State,
    pub(crate) kingdom_removal:crate::configure_kingdom_removal::State,
    pub(crate) creator_extents:Option<crate::world_creator_extents::State>,
    pub(crate) view_biome:Option<crate::vworld_resource_context::Biome>,
    pub(crate) objectives:crate::world_objectives::State,
    pub(crate) property_allocations:Option<crate::load_property_instances::State>,
    pub(crate) map_text:std::result::Result<crate::film_text::MapTextOrder,String>,
    pub(crate) sai_map:crate::load_sai_world::Map,
    pub(crate) objects: WorldObjectList<ActorId>,
    pub(crate) source_fields: NativeSourceRequirements,
    pub(crate) registration: live_registration::State,
    pub(crate) kingdoms: crate::load_kingdom_init_world::KingdomRegistryState,
    pub(crate) special:crate::load_special_object_init::State,
    pub(crate) sync_counters:crate::initial_world_sync::Counters,
    ///One retained command/movement/guard owner for this fresh application
    ///interval. Future World replacement must move it, not reset process guards.
    pub(crate) mind_runtime:crate::mind_world::Runtime,
    pub(crate) restrictions: crate::restriction_groups::LoadedRestrictionGroups,
    pub(crate) point: Arc<PointShape>,
    pub(crate) map_mode: Box<Option<u32>>,
    pub(crate) resource_load: Option<crate::map_resource_load::Owner>,
    pub(crate) global_settings_mode: SettingsMode,
    pub(crate) placement_clearance_bits: Option<u32>,
    /// Same singleton made by the verified native constructor-input producer.
    pub(crate) sun: crate::sun_state::State,
    sun_loaded: bool,
    pub(crate) misc:crate::saved_misc::State,
    misc_loaded:bool,
    pub(crate) tail:crate::load_tail_source::TailState,
    pub(crate) dispatcher:Option<aa_world::DispatcherState>,
    pub(crate) updatable_lifetime:crate::updatable_lifetime::State,
    pub(crate) presentation:crate::fx_manager::State<crate::fx_graph::Command>,
    pub(crate) trigger_manager:Option<aa_triggers::manager::Manager>,
    trigger_load_broadcast_pending:bool,
    source_files: Vec<aa_rules::resources::SourceIdentity>,
    precision: Precision,
}

impl World {
    pub fn revision(&self)->&crate::world_revision::ConstructedWorldRevision{&self.revision}
    pub(crate) fn split_initial_sync(&mut self)->(crate::initial_world_sync::Fields<'_>,&crate::world_revision::ConstructedWorldRevision,&mut SpatialOwner,&mut crate::mind_world::Runtime){
        (crate::initial_world_sync::Fields{
            svars:&self.variables,
            counters:&self.sync_counters,rules:&self.world_rules,mission:&self.mission_fields,
            runtime:&self.runtime_fields,misc:&self.misc,registration:&self.registration,
            objectives:&self.objectives,kingdoms:&self.kingdoms,
        },&self.revision,&mut self.spatial,&mut self.mind_runtime)
    }
    pub fn kingdoms(&self)->&crate::load_kingdom_init_world::KingdomRegistryState {&self.kingdoms}
    /// Bind after verified source completion, before the first TEAMS record.
    /// `duplicate_city_latch` is the actual process B3CC40 owner transferred to
    /// registration, not a per-map reset or a default inferred from this map.
    #[allow(clippy::too_many_arguments)]
    pub fn before_first_record(
        catalog: &Catalog<'_>, sources: &mut V14Sources,
        map: &MapArchive, inventory: &WorldInventory,
        point: Arc<PointShape>, sun:crate::sun_state::State, duplicate_city_latch: bool, precision: Precision,
        world_sink:Option<&mut dyn crate::load_sim_world::WorldConstructorSink>,
    ) -> Result<Self> {
        Self::before_first_record_inner(catalog,sources,map,inventory,point,sun,duplicate_city_latch,
            precision,world_sink,None,None)
    }

    ///The real selected-map path supplies the already constructed/read resource
    ///owner before76AA90. Its final+88 selection occurs after that constructor,
    ///before4FC600's mission words, RULES and TERRAIN loads.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn before_first_record_prepared(
        catalog: &Catalog<'_>, sources: &mut V14Sources,
        map: &MapArchive, inventory: &WorldInventory,
        point: Arc<PointShape>, sun:crate::sun_state::State, duplicate_city_latch: bool, precision: Precision,
        world_sink:Option<&mut dyn crate::load_sim_world::WorldConstructorSink>,
        prepared:(crate::map_resource_load::Owner,Box<Option<u32>>),
        selected:&crate::resource_creator::Creator,
    ) -> Result<Self> {
        Self::before_first_record_inner(catalog,sources,map,inventory,point,sun,duplicate_city_latch,
            precision,world_sink,Some(prepared),Some(selected))
    }

    #[allow(clippy::too_many_arguments)]
    fn before_first_record_inner(
        catalog: &Catalog<'_>, sources: &mut V14Sources,
        map: &MapArchive, inventory: &WorldInventory,
        point: Arc<PointShape>, sun:crate::sun_state::State, duplicate_city_latch: bool, precision: Precision,
        world_sink:Option<&mut dyn crate::load_sim_world::WorldConstructorSink>,
        prepared:Option<(crate::map_resource_load::Owner,Box<Option<u32>>)>,
        selected:Option<&crate::resource_creator::Creator>,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        let (mut resource_load,map_mode)=if let Some(prepared)=prepared {prepared} else {
            let archive=catalog.load_archive.try_borrow().map_err(|_|"load archive witness already borrowed")?.clone();
            if let Some(witness)=&archive {witness.validate_map(map)?;}
            crate::map_resource_load::Owner::fresh_loading_archive(
                catalog.native_load_mode.ok_or("load world requires actual MapResource+A8 mode owner")?,
                &catalog.actors.sources,archive)?
        };
        resource_load.validate_loading(map_mode.as_ref(),&catalog.actors.sources)?;
        resource_load.validate_archive(map)?;
        if *map_mode!=catalog.native_load_mode {
            return Err("prepared MapResource load mode differs from actual Catalog interval".into());
        }
        let revision=catalog.construct_world_revision()?;
        if let Some(sink)=world_sink {sink.store_world_76aa90(&revision)?;}
        //5126C0's receiver is the PARSED resource+88 at World Init. Copy its
        //raw words before5A2FCF optionally replaces+88 with selected settings.
        //Record-only diagnostics have no constructor input and keep it unbound.
        let creator_extents=if selected.is_some() {
            Some(crate::world_creator_extents::State::from_world_init_creator(resource_load.creator()?))
        }else {None};
        //GMap+24 retains the PARSED Creator's Biome before the selected
        //Creator replacement. Retain its canonical pointer-equivalent owner;
        //texture_subfolder_name is read lazily at43F0A0, not snapshotted here.
        let view_biome=if selected.is_some(){
            let bank=catalog.definition_bank.try_borrow().map_err(|e|e.to_string())?;
            Some(crate::vworld_resource_context::Biome::from_world_init_creator(catalog,&bank,resource_load.creator()?)?)
        }else{None};
        if let Some(selected)=selected {
            resource_load.select_creator_before_world_load(catalog,selected)?;
        }
        let mut runtime_fields=crate::load_world_runtime_fields::State::from_world_constructor();
        let mut mission_fields=crate::mission_fields::State::from_world_constructor();
        mission_fields.load_from_archive(map,&inventory.world)?;
        let source_pc = {
            let bank = catalog.definition_bank.try_borrow().map_err(|_| "source Bank is borrowed")?;
            let state = bank.combat_source()?;
            state.completed_means()?;
            if state.source_identities() != catalog.actors.sources {
                return Err("load world source Bank/catalog identities differ".into());
            }
            state.source_precision()
        };
        let header = inventory.header.as_ref().ok_or("load world requires MAP_HEADER")?.clone();
        let mut objectives=crate::world_objectives::State::from_world_constructor();
        objectives.bind_source(map,inventory.world.version)?;
        let mapstrings=map.decode_chunk("MAPSTRINGS",u64::MAX).map_err(|e|e.to_string())?;
        let map_text=catalog.with_current_text(|text|Ok(crate::film_text::MapTextOrder::bind(&mapstrings,text)))?;
        let mut variables=crate::load_view_variables::State::reset_from_fresh_source(catalog,
            crate::load_view_variables::NativeOverrides::ControlledNone)?;
        //4FC859 publishes this same reset current SVar owner before TERRAIN.
        let world_rules=if selected.is_some() {
            crate::load_world_rules::Owner::load_for_resource(catalog,inventory,&resource_load)?
        } else {
            //The retained no-selection record loader is a bounded diagnostic;
            //it cannot validate a completed Creator or start a live mission.
            crate::load_world_rules::Owner::load(catalog,inventory)?
        };
        let misc=crate::saved_misc::State::new(variables.constructor_game_speed(catalog)?);
        variables.publish_loaded_rules(catalog,&world_rules)?;
        //The constructor above binds the actual SVar+114 ->World+278 write.
        //Fresh WorldInit allocates+4 after that same GameSpeed producer.
        runtime_fields.initialize_fresh_auxiliary_4f27e0()?;
        let settings = catalog.settings.as_ref().ok_or("load world requires actual Settings")?;
        let source = battle_terrain::Source::bind(sources, catalog.actors, settings, source_pc)?;
        let terrain = BattleTerrain::load(map, &source)?;
        let spatial = LoadSpatial::new(&terrain.bound, precision).map_err(|e| e.to_string())?;
        let zones = load_zone_services::before_first_record(&terrain.bound, spatial.index(), &header)
            .map_err(|e| e.to_string())?;
        let zone_globals=crate::load_zone_globals::State::from_world_init(&zones).map_err(|e|e.to_string())?;
        let sai_map=crate::load_sai_world::Map::load(catalog,map,&terrain,spatial.index(),&zones,precision)?;
        let source_fields = catalog.actors.native_source_requirements(source_pc).map_err(|e| e.to_string())?;
        let mut updatable_lifetime=crate::updatable_lifetime::State::from_world_constructor();
        if inventory.world.version>10 {
            let table=catalog.updatables.try_borrow().map_err(|_|"50B470 actual saved table is borrowed")?;
            let table=table.as_ref().ok_or("50B470 actual saved table owner absent")?;
            let chunk=map.decode_chunk("UPDATABLES",u64::MAX).map_err(|v|v.to_string())?;
            updatable_lifetime.bind_saved_table(&chunk,table.len())?;
        }
        let teams = u32::try_from(inventory.teams.as_ref().ok_or("load world requires TEAMS")?.slots.len())
            .map_err(|e| e.to_string())?;
        let registration = live_registration::State::from_saved_team_count(teams, duplicate_city_latch)?;
        let kingdoms = crate::load_kingdom_init_world::KingdomRegistryState::from_inventory(catalog, inventory)?;
        let restrictions = crate::restriction_groups::LoadedRestrictionGroups::load(
            &map.decode_chunk("RESTRICTION_GROUPS", u64::MAX).map_err(|e|e.to_string())?, catalog,
            aa_mapworld::restriction_groups::LocalizedStorage::KeyOnly, source_pc,
        )?;
        if catalog.native_load_mode.is_none() {
            return Err("load world requires actual MapResource+A8 mode owner".into());
        }
        // The native World constructor publishes this same MapResource via
        // B36534/+28. Keep its mode allocation stable across every Rust move.
        let global_settings_mode=SettingsMode::SameWorld(
            crate::load_world_settings::SameWorldSettings::from_constructed_world(map_mode.as_ref())?);
        Ok(Self { revision,header, terrain, spatial:SpatialOwner::Loading(Some(spatial)), zones,
            zone_identities:crate::load_detection_activation::Identities::new(),
            zone_globals,variables,runtime_fields,mission_fields,world_rules,creator_extents,view_biome,objectives,map_text,
            configuration:crate::configure_world::State::from_world_constructor(selected.is_some()),
            kingdom_removal:crate::configure_kingdom_removal::State::new(),
            property_allocations:Some(crate::load_property_instances::State::from_world_constructor()),
            sai_map,objects:WorldObjectList::constructed(),
            source_fields, registration, kingdoms, special:crate::load_special_object_init::State::from_world_constructor(),
            sync_counters:crate::initial_world_sync::Counters::from_loaded_world(&inventory.world),restrictions, point,
            mind_runtime:crate::mind_world::Runtime::from_fresh_application_interval(),
            map_mode, resource_load:Some(resource_load), global_settings_mode,
            placement_clearance_bits:None, sun, sun_loaded:false, misc,misc_loaded:false,
            tail:crate::load_tail_source::TailState::from_world_constructor(),
            dispatcher:Some(aa_world::DispatcherState::from_native_constructor()),
            updatable_lifetime,
            presentation:crate::fx_manager::State::from_native_constructor(),
            trigger_manager:Some(aa_triggers::manager::Manager::from_native_constructor()),
            trigger_load_broadcast_pending:false,
            source_files:catalog.actors.sources.clone(), precision })
    }

    fn validate(&self, catalog:&Catalog<'_>, precision:Precision) -> Result<()> {
        catalog.require_source_owner()?;
        if self.source_files != catalog.actors.sources || self.precision != precision
            || *self.map_mode != catalog.native_load_mode {
            return Err("immediate load world source/precision/mode owner differs".into());
        }
        Ok(())
    }

    /// Require the real immediate load receipts before resource-tail callers
    /// may begin their own fallible post-load/conversion effects.
    pub(crate) fn validate_loaded_sections(&self,catalog:&Catalog<'_>,precision:Precision)->Result<()> {
        self.validate(catalog,precision)?;
        self.objectives.require_loaded()?;
        self.configuration.require_boundary()?;
        self.resource_load.as_ref().ok_or("MapResource load owner absent")?
            .validate_loading(self.map_mode.as_ref(),&catalog.actors.sources)?;
        if !self.sun_loaded || !self.misc_loaded {
            return Err("resource completion requires original SUN and MISC_DATA loads".into());
        }
        if !self.trigger_manager.as_ref().is_some_and(|m|m.saved_payload_installed())
            || self.trigger_load_broadcast_pending {
            return Err("resource completion requires original TRIGGERS and its presentation broadcast".into());
        }
        Ok(())
    }

    pub(crate) fn install_trigger_payload(&mut self,saved:aa_triggers::decode::SavedManager)->Result<()> {
        self.trigger_manager.as_mut().ok_or("actual trigger manager owner is absent")?
            .install_saved_on_fresh(saved).map_err(|e|e.to_string())?;
        //54D3D0 finishes with50D120("TriggerListUpdate"). Keep that boundary
        //on the same +130 message vector before SUN can begin.
        self.trigger_load_broadcast_pending=true;
        self.presentation.broadcast(&"TriggerListUpdate".encode_utf16().collect::<Vec<_>>())?;
        self.trigger_load_broadcast_pending=false;
        Ok(())
    }

    pub fn initialize(&mut self, catalog:&Catalog<'_>, context:&mut Context<'_>, session:&dyn SessionPhase) -> Result<u8> {
        let pc=context.precision;
        self.validate(catalog,pc)?;
        let ordinal=context.source_ordinal;
        let parts=context.split_initialize_parts()?;
        let owner=parts.held.id();
        // Kingdom Init mutates the native registry vectors themselves. Do not
        // alias those vectors through visibility services during that borrow.
        // Its reached Object gates need no memory pass; a newly reached pass
        // still reports its missing owner rather than receiving an empty list.
        if let RestoredRecord::Kingdom(kingdom)=parts.held.record_mut() {
            let mut zone_services=load_zone_services::Context::new(&self.point,session)
                .with_source_fields(catalog,&self.source_fields);
            let mut object=load_object_init_world::Context {
                terrain:&self.terrain,header:&self.header,spatial:self.spatial.loading_mut()?,zones:&mut self.zones,
                objects:&mut self.objects,source_fields:&self.source_fields,map_mode:&mut self.map_mode,
                global_settings_mode:self.global_settings_mode,
                placement_clearance_bits:self.placement_clearance_bits,zone_services:&mut zone_services,
            };
            let mut actor=ActorServices {object:&mut object,catalog,inventory:parts.inventory,world_version:None,rng:Some(&mut *parts.rng),zone_identities:&mut self.zone_identities,properties:self.property_allocations.as_mut().ok_or("actual property allocation owner absent during load")?,
                activation:Activation {globals:&self.zone_globals,variables:&mut self.variables,
                    phase:&self.dispatcher.as_ref().ok_or("load dispatcher owner absent")?.phase,presentation:&mut self.presentation}};
            let runtime=kingdom.runtime.as_mut().ok_or("Kingdom runtime is absent during Init")?;
            let derived=runtime.derived.as_mut().ok_or("Kingdom derived allocation is absent during Init")?;
            let mut record=Record {owner,shell:&mut runtime.base,definition:&runtime.definition,registry:parts.registry};
            let result=crate::load_kingdom_init_world::initialize_kingdom(catalog,&mut record,derived,
                crate::load_kingdom_init_world::Context {actor_services:&mut actor,
                    world_version:parts.world_version,registry_state:&mut self.kingdoms,
                    restrictions:Some(&self.restrictions),messages:parts.messages,source_ordinal:ordinal},pc)?;
            return Ok(u8::from(result));
        }
        let mut zone_services=load_zone_services::Context::with_kingdoms(&self.point,session,&self.kingdoms)
            .with_source_fields(catalog,&self.source_fields);
        let mut object=load_object_init_world::Context {
            terrain:&self.terrain,header:&self.header,spatial:self.spatial.loading_mut()?,zones:&mut self.zones,
            objects:&mut self.objects,source_fields:&self.source_fields,map_mode:&mut self.map_mode,
            global_settings_mode:self.global_settings_mode,
            placement_clearance_bits:self.placement_clearance_bits,zone_services:&mut zone_services,
        };
        let mut actor=ActorServices { object:&mut object, catalog,inventory:parts.inventory,world_version:None,rng:None,zone_identities:&mut self.zone_identities,properties:self.property_allocations.as_mut().ok_or("actual property allocation owner absent during load")?,
            activation:Activation {globals:&self.zone_globals,variables:&mut self.variables,
                phase:&self.dispatcher.as_ref().ok_or("load dispatcher owner absent")?.phase,presentation:&mut self.presentation} };
        let result=match parts.held.record_mut() {
            RestoredRecord::Team(team) => {
                let runtime=team.runtime.as_mut().ok_or("Team runtime is absent during Init")?;
                let mut record=Record {owner,shell:&mut runtime.base,definition:&runtime.definition,registry:parts.registry};
                crate::load_kingdom_init_world::initialize_team(catalog,&mut record,&mut object,
                    parts.messages,ordinal,pc)?
            }
            RestoredRecord::Object(entity) if matches!(entity.definition.native_class,6|7|9) => {
                use aa_mapworld::map_objects::SpecialObject;
                use crate::load_special_object_init::Derived;
                let derived=match entity.definition.native_class {
                    6=>match entity.special_object.as_mut() {
                        Some(SpecialObject::Area(area))=>Derived::Area(area),
                        _=>return Err("Area Init current derived allocation is absent".into()),
                    },
                    7=>match entity.special_object.as_mut() {
                        Some(SpecialObject::Camera(camera))=>Derived::Camera(camera),
                        _=>return Err("Camera Init current derived allocation is absent".into()),
                    },
                    9=>Derived::Doodad(entity.doodad.as_mut().ok_or("Doodad Init current derived allocation is absent")?),
                    _=>unreachable!(),
                };
                let mut record=Record {owner,shell:&mut entity.shell,definition:&entity.definition,registry:parts.registry};
                crate::load_special_object_init::initialize(catalog,&mut record,derived,&mut object,
                    &mut self.special,&*parts.world_version,parts.rng,pc).map_err(|e|e.to_string())?
            }
            record => {
                let (shell,definition)=record_parts(record)?;
                let class=definition.native_class;
                let mut record=Record {owner,shell,definition,registry:parts.registry};
                match class {
                    1 => initialize_thing(catalog,&record)?,
                    3 => {
                        actor.world_version=Some(&*parts.world_version);
                        actor.rng=Some(&mut *parts.rng);
                        load_actor_init_world::initialize(catalog,&mut record,pc,&mut actor).map_err(|e|e.to_string())?
                    },
                    2|6|7|9 => load_object_init_world::initialize(catalog,&mut record,&mut object,pc).map_err(|e|e.to_string())?,
                    _ => return Err("unsupported actual class for immediate Init".into()),
                }
            }
        };
        Ok(u8::from(result))
    }

    pub fn post_create(&mut self,catalog:&Catalog<'_>,context:&mut Context<'_>) -> Result<()> {
        self.validate(catalog,context.precision)?;
        let inventory=context.inventory;
        crate::load_post_create_world::post_create(catalog,context,&mut RequiredEffects {inventory,presentation:&mut self.presentation,variables:&mut self.variables,properties:self.property_allocations.as_mut().ok_or("actual property allocation owner absent during load")?})
    }
    /// Original GWorld+4C saved Load, using the same saved-ID table already
    /// populated by the records. Its progress never confers battle readiness.
    pub fn load_sun(&mut self,catalog:&Catalog<'_>,chunk:&aa_mapworld::Chunk,
        registrations:&mut Vec<aa_mapworld::restore_components::UpdatableRegistration>)->Result<()> {
        chunk.verify().map_err(|e|e.to_string())?;
        if chunk.directory.name!="SUN" || self.sun_loaded {
            return Err("SUN load requires its original chunk and not-yet-loaded singleton".into());
        }
        let used=self.sun.load(&chunk.bytes,&mut SunServices {catalog,registrations}).map_err(|e|e.to_string())?;
        if used!=chunk.bytes.len() {return Err("SUN chunk has trailing bytes after native Load".into());}
        self.sun_loaded=true;
        Ok(())
    }
    pub fn load_misc(&mut self,catalog:&Catalog<'_>,chunk:&aa_mapworld::Chunk,version:u32)->Result<()> {
        self.validate(catalog,self.precision)?;
        chunk.verify().map_err(|e|e.to_string())?;
        if chunk.directory.name!="MISC_DATA" || self.misc_loaded || !self.sun_loaded {
            return Err("MISC_DATA requires original chunk, completed SUN and not-yet-loaded owner".into());
        }
        let source_pc=catalog.definition_bank.try_borrow().map_err(|_|"MISC source Bank is borrowed")?.combat_source()?.source_precision();
        let used=self.misc.load(&chunk.bytes,version,&mut MiscServices {catalog,source_pc}).map_err(|e|e.to_string())?;
        if used!=chunk.bytes.len() {return Err("MISC_DATA has trailing bytes after native Load".into());}
        self.misc_loaded=true;
        Ok(())
    }
    pub fn publish_definition(&mut self,catalog:&Catalog<'_>,context:&mut Context<'_>) -> Result<()> {
        self.validate(catalog,context.precision)?;
        let inventory=context.inventory;
        crate::load_post_create_world::publish_definition_created_fx(catalog,context,&mut RequiredEffects {inventory,presentation:&mut self.presentation,variables:&mut self.variables,properties:self.property_allocations.as_mut().ok_or("actual property allocation owner absent during load")?})
    }
    pub fn register(&mut self,catalog:&Catalog<'_>,context:&mut Context<'_>,session:&dyn SessionPhase)->Result<()> {
        let pc=context.precision;
        self.validate(catalog,pc)?;
        let world=&context.inventory.world;
        let inventory=context.inventory;
        let (held,registry)=context.split_current()?;
        let owner=held.id();
        let mut effects=RegistrationEffects {grid:crate::load_actor_registration::GridContext {
            zones:&mut self.zones,spatial:self.spatial.loading()?,point:&self.point,session,world,
        },inventory};
        live_registration::register_restored(&mut self.registration,owner,held.record_mut(),registry,catalog,&mut effects,pc)
    }
    pub fn object_predicate(context:&mut Context<'_>) -> Result<bool> {
        let held=context.current_mut().ok_or("Object+B0 current allocation is null")?;
        let (shell,definition)=record_parts(held.record_mut())?;
        let (kind,object,actor)=match definition.native_class {
            1|5 => (LoadKind::Thing,false,false),
            2|6|7|9 => (LoadKind::Object,true,false),
            3|4 => (LoadKind::Actor,true,true),
            _ => return Err("Object+B0 actual constructed class is unsupported".into()),
        };
        if shell.kind!=kind || shell.object.is_some()!=object || shell.actor.is_some()!=actor {
            return Err("Object+B0 source class and constructed allocation differ".into());
        }
        Ok(object)
    }
    /// Transfer the same load index into the live placement owner. The
    /// adoption result is retained BEFORE propagating any partial-transfer
    /// error; no native node/cache is rebuilt or silently discarded.
    pub fn adopt_spatial(&mut self,actors:&mut crate::actor_world::ActorStore)->Result<()> {
        let loaded=match &mut self.spatial {
            SpatialOwner::Loading(owner)=>owner.take().ok_or("load index transfer already in progress")?,
            SpatialOwner::Adopted(_)=>return Err("load index has already been adopted".into()),
        };
        let adoption=crate::spatial_world::SpatialWorld::adopt_loaded(loaded.into_parts(),actors);
        let error=adoption.error.as_ref().map(ToString::to_string);
        self.spatial=SpatialOwner::Adopted(adoption);
        if let Some(error)=error {return Err(error);}
        Ok(())
    }
    pub fn live_spatial(&self)->Result<&crate::spatial_world::SpatialWorld> {
        match &self.spatial {
            SpatialOwner::Adopted(adoption) if adoption.error.is_none() && adoption.pending.is_empty()=>Ok(&adoption.world),
            _=>Err("live placement transfer has not completed".into()),
        }
    }
}

pub(crate) enum SpatialOwner {
    Loading(Option<LoadSpatial>),
    Adopted(crate::spatial_world::LoadAdoption),
}
impl SpatialOwner {
    fn loading(&self)->Result<&LoadSpatial> {
        match self {Self::Loading(Some(value))=>Ok(value),_=>Err("load index is no longer owned by record lifecycle".into())}
    }
    fn loading_mut(&mut self)->Result<&mut LoadSpatial> {
        match self {Self::Loading(Some(value))=>Ok(value),_=>Err("load index is no longer owned by record lifecycle".into())}
    }
}

fn record_parts(record:&mut RestoredRecord)->Result<(&mut LoadShell<DefinitionProof,RestoredComponent>,&RuntimeDefinition)> {
    match record {
        RestoredRecord::Team(e)=>e.runtime.as_mut().map(|r|(&mut r.base,&r.definition)),
        RestoredRecord::Kingdom(e)=>e.runtime.as_mut().map(|r|(&mut r.base,&r.definition)),
        RestoredRecord::Thing(e)|RestoredRecord::Object(e)|RestoredRecord::Actor(e)=>Some((&mut e.shell,&e.definition)),
        RestoredRecord::Loading=>None,
    }.ok_or_else(||"immediate lifecycle current owned allocation is missing".into())
}
/// GThing class1 constructor vtable+E8 is6A71D0 (AL=1, RET4).
/// Only that constructed source identity admits this constant native leaf.
fn initialize_thing(catalog:&Catalog<'_>,record:&Record<'_>)->Result<bool> {
    catalog.require_untranslated_actor_identity(&record.definition.proof.id)?;
    let proof=&record.definition.proof;
    let source=catalog.actors.find(&proof.id).ok_or("Thing Init canonical source is absent")?;
    if record.owner.is_null()
        || !record.registry.get(record.owner).is_some_and(|e|matches!(e.value,RestoredRecord::Loading))
        || record.definition.native_class!=1 || proof.category.runtime_class!=1
        || record.shell.kind!=LoadKind::Thing || record.shell.object.is_some() || record.shell.actor.is_some()
        || source.source!=proof.source || source.template_chain!=proof.template_chain
        || catalog.actors.native_category(&proof.id).map_err(|e|e.to_string())?!=proof.category
        || !crate::load_spatial::same_proof(&record.shell.thing.definition,proof) {
        return Err("Thing Init actual source/constructor/held allocation differs".into());
    }
    Ok(true)
}
struct Activation<'a> {
    globals:&'a crate::load_zone_globals::State,
    variables:&'a mut crate::load_view_variables::State,
    phase:&'a u32,
    presentation:&'a mut crate::fx_manager::State<crate::fx_graph::Command>,
}
struct ActorServices<'a,'b,'c,'d> { object:&'a mut load_object_init_world::Context<'b,'c>, catalog:&'a Catalog<'d>,inventory:&'a WorldInventory,world_version:Option<&'a u32>,rng:Option<&'a mut aa_simulation::NativeRng>,zone_identities:&'a mut crate::load_detection_activation::Identities,activation:Activation<'a>,properties:&'a mut crate::load_property_instances::State }
impl load_actor_init_world::Services for ActorServices<'_,'_,'_,'_> {
    fn supply_init_existing(&mut self,record:&mut Record<'_>,pc:Precision)->aa_actors::actor_creation::Result<bool>{
        let rng=self.rng.as_deref_mut().ok_or(aa_actors::actor_creation::Error::Unsupported("Supply Init requires actual load RNG owner"))?;
        let variables=&*self.activation.variables;
        let mut effects=self.activation.globals.bind(self.catalog,&*self.object.zones,
            self.object.spatial.index(),self.activation.phase,self.activation.presentation,variables)
            .map_err(|e|aa_actors::actor_creation::Error::World(e.to_string()))?
            .with_reveal(&mut *self.object.zone_services);
        crate::load_supply_init_world::init_existing(self.catalog,record,rng,pc,
            &mut crate::load_supply_init_world::Context{
                zones:&mut *self.object.zones,identities:&mut *self.zone_identities,effects:&mut effects,
                variables,settings_mode_owner:&*self.object.map_mode,global_settings:self.object.global_settings_mode,
            })
    }
    fn denizen_init_existing(&mut self,record:&mut Record<'_>,pc:Precision)->aa_actors::actor_creation::Result<bool>{
        let version=self.world_version;
        let rng=self.rng.as_deref_mut().ok_or(aa_actors::actor_creation::Error::Unsupported("Denizen Init requires actual load RNG owner"))?;
        crate::load_denizen_init_world::init_existing(self.catalog,record,rng,pc,||version.copied().ok_or(aa_actors::actor_creation::Error::Unsupported("Denizen Init actual World+2C owner")))
    }
    fn denizen_after_init(&mut self,record:&mut Record<'_>)->aa_actors::actor_creation::Result<()>{
        crate::load_denizen_init_world::after_init(self.catalog,record,&mut crate::load_denizen_init_world::RequiredZones,self.object.spatial.index().precision())
    }
    fn element_init_existing(&mut self,record:&mut Record<'_>,pc:Precision)->aa_actors::actor_creation::Result<bool> {
        let version=self.world_version;
        let rng=self.rng.as_deref_mut().ok_or(aa_actors::actor_creation::Error::Unsupported("Element Init requires actual load RNG owner"))?;
        crate::load_element_init_world::init_existing(self.catalog,record,rng,pc,||version.copied().ok_or(aa_actors::actor_creation::Error::Unsupported("Element Init actual World+2C owner")))
    }
    fn performer_init_existing(&mut self,record:&mut Record<'_>,pc:Precision)->aa_actors::actor_creation::Result<bool> {
        crate::load_performer_init_world::init_existing(self.catalog,self.inventory,record,pc)
    }
    fn global_property(&self)->aa_actors::actor_creation::Result<Option<u32>> {
        crate::load_property_instances::global(self.catalog).map_err(aa_actors::actor_creation::Error::World)
    }
    fn property_applies(&mut self,record:&mut Record<'_>,property:u32)->aa_actors::actor_creation::Result<bool> {
        crate::load_property_instances::applicable(self.catalog,record,&mut *self.properties,property)
            .map_err(aa_actors::actor_creation::Error::World)
    }
    fn properties_add(&mut self,record:&mut Record<'_>,property:u32)->aa_actors::actor_creation::Result<()> {
        let catalog=self.catalog;let source=record.owner;
        crate::load_property_instances::add(catalog,record,&mut *self.properties,property,0,Some(source),
            ||current_property_time(catalog),self.object.spatial.index().precision(),
            &mut PropertyEffects {variables:&mut *self.activation.variables,inventory:self.inventory,environment:catalog.runtime_environment})
            .map_err(aa_actors::actor_creation::Error::World)
    }
    fn existing_fx(&mut self,record:&mut Record<'_>)->aa_actors::actor_creation::Result<()> {
        let mut execution=self.catalog.load_execution.try_borrow_mut().map_err(|e|aa_actors::actor_creation::Error::World(e.to_string()))?;
        let execution=execution.as_deref_mut().ok_or(aa_actors::actor_creation::Error::Unsupported("Existing FX requires actual running resource worker"))?;
        crate::load_creation_fx::publish_existing(self.catalog,record,execution,&mut *self.activation.presentation)
            .map_err(aa_actors::actor_creation::Error::World)
    }
    fn refresh_4d1350(&mut self,record:&mut Record<'_>,pc:Precision)->aa_actors::actor_creation::Result<()> {
        crate::load_actor_action_refresh::refresh(self.catalog,self.inventory,record,pc)
    }
    fn performer_refresh(&mut self,record:&mut Record<'_>)->aa_actors::actor_creation::Result<()> {
        let current_environment=self.catalog.runtime_environment.unwrap_or(required_current_runtime_environment);
        crate::load_performer_value_refresh::refresh(self.catalog,record,
            &mut crate::load_performer_value_refresh::Context {variables:&mut *self.activation.variables,
                instances:Some(self.properties.instances()),current_environment},self.object.spatial.index().precision())
    }
    fn detection_after_init(&mut self,record:&mut Record<'_>)->aa_actors::actor_creation::Result<()> {
        let mut effects=self.activation.globals.bind(self.catalog,&*self.object.zones,
            self.object.spatial.index(),self.activation.phase,self.activation.presentation,self.activation.variables)
            .map_err(|e|aa_actors::actor_creation::Error::World(e.to_string()))?
            .with_reveal(&mut *self.object.zone_services);
        crate::load_detection_activation::after_init(self.catalog,record,&mut *self.object.zones,
            &mut *self.zone_identities,self.object.spatial.index().precision(),
            &mut effects)
            .map_err(|e|aa_actors::actor_creation::Error::World(e.to_string()))
    }
    fn scheduled_component_init_existing(&mut self,record:&mut Record<'_>,slot:aa_actors::components::ComponentSlot,
        pc:Precision)->aa_actors::actor_creation::Result<bool> {
        let version=self.world_version;
        let rng=self.rng.as_deref_mut().ok_or(aa_actors::actor_creation::Error::Unsupported(
            "scheduled Init requires the actual shared load RNG owner"))?;
        crate::load_detection_init_world::init_existing(self.catalog,record,slot,rng,pc,|| {
            version.copied().ok_or(aa_actors::actor_creation::Error::Unsupported(
                "scheduled Init requires the actual current World+2C owner"))
        })
    }
    fn refresh_4d4130(&mut self,record:&mut Record<'_>)->aa_actors::actor_creation::Result<()> {
        crate::load_organization_terrain_refresh::refresh(self.catalog,record,self.object.terrain,
            self.object.spatial.index().precision())
    }
    fn economy_init_existing(&mut self,record:&mut Record<'_>,pc:Precision)->aa_actors::actor_creation::Result<bool> {
        let rng=self.rng.as_deref_mut().ok_or(aa_actors::actor_creation::Error::Unsupported(
            "Economy Init requires the actual shared load RNG owner"))?;
        crate::load_economy_init_world::init_existing(self.catalog,record,rng,pc)
    }
    fn mind_init_existing(&mut self,record:&mut Record<'_>,pc:Precision)->aa_actors::actor_creation::Result<bool> {
        crate::load_mind_init_world::init_existing(self.catalog,self.inventory,record,pc,&mut RequiredEffects {inventory:self.inventory,presentation:&mut *self.activation.presentation,variables:&mut *self.activation.variables,properties:&mut *self.properties})
    }
    fn refresh_4d67f0(&mut self,record:&mut Record<'_>)->aa_actors::actor_creation::Result<()> {
        let catalog=self.catalog;
        let pc=self.object.spatial.index().precision();
        crate::load_actor_overlaps::refresh(catalog,record,self.object,pc,|| {
            catalog.load_clock.map(|clock|clock.begin.0).ok_or(aa_actors::actor_creation::Error::Unsupported(
                "actual GWorld+E0 clock owner is required for overlap expiration"))
        }).map(|_|())
    }
    fn world_version(&self)->aa_actors::actor_creation::Result<u32> {
        self.world_version.copied().ok_or(aa_actors::actor_creation::Error::Unsupported(
            "actual current World+2C borrow is required for this specialized Actor Init"))
    }
    fn organization_refresh(&mut self,record:&mut Record<'_>)->aa_actors::actor_creation::Result<()> {
        crate::load_organization_value_refresh::refresh(self.catalog,record,self.object.spatial.index().precision())
    }
    fn denizen_refresh(&mut self,record:&mut Record<'_>)->aa_actors::actor_creation::Result<()> {
        crate::load_denizen_value_refresh::refresh(self.catalog,record,self.object.spatial.index().precision())
    }
    fn object_init_null(&mut self,record:&mut Record<'_>,pc:Precision)->aa_actors::actor_creation::Result<bool> {
        load_object_init_world::initialize(self.catalog,record,self.object,pc)
    }
}
struct RequiredEffects<'a> {inventory:&'a WorldInventory,presentation:&'a mut crate::fx_manager::State<crate::fx_graph::Command>,
    variables:&'a mut crate::load_view_variables::State,properties:&'a mut crate::load_property_instances::State}
struct PropertyEffects<'a> {variables:&'a mut crate::load_view_variables::State,inventory:&'a WorldInventory,
    environment:Option<crate::load_performer_value_refresh::EnvironmentSampler>}
impl crate::load_property_instances::Effects for PropertyEffects<'_> {
    fn performer_added(&mut self,catalog:&Catalog<'_>,record:&mut Record<'_>,state:&mut crate::load_property_instances::State,pc:Precision)->Result<()> {
        crate::load_performer_property_refresh::refresh(catalog,record,
            &mut crate::load_performer_property_refresh::Context {variables:&mut *self.variables,
                properties:&*state,inventory:self.inventory,
                current_environment:self.environment.unwrap_or(required_current_runtime_environment)},pc)
            .map_err(|e|e.to_string())
    }
    fn minimum_multiplier(&mut self)->aa_body::Result<aa_body::F32> {
        Ok(aa_body::F32(self.variables.modifier_clamp_multiply_bits()))
    }
    fn numeric_changed(&mut self,catalog:&Catalog<'_>,fields:&mut crate::load_property_instances::Fields<'_>,modifier:&aa_body::property_effects::NumericModifier,
        properties:&mut aa_body::properties::PropertiesState,instances:&mut aa_body::property_effects::Instances)->aa_body::Result<()> {
        crate::load_property_numeric::changed(catalog,fields,modifier,properties,instances,&mut *self.variables)
    }
}
fn current_property_time(catalog:&Catalog<'_>)->Result<u32> {
    catalog.load_clock.map(|clock|clock.begin.0).ok_or("Property allocation requires actual World+E0 clock".into())
}
fn required_current_runtime_environment()->std::result::Result<aa_numeric::crt_pow::CrtEnvironment,String> {
    Err("Performer requires actual current-thread CRT sampler".into())
}
impl crate::load_mind_init_world::Effects for RequiredEffects<'_> {
    fn enter(&mut self,catalog:&Catalog<'_>,record:&mut Record<'_>,state:&'static aa_actors::state::Descriptor,
        _old:Option<&'static aa_actors::state::Descriptor>,pc:Precision)->aa_actors::actor_creation::Result<()> {
        //4D4A99 reads the old descriptor only inside the Organization child,
        // which this concrete adapter still refuses before any such read.
        let canonical=crate::live_object_init::Source::bind(catalog,record.definition)?;
        let source=catalog.actors.native_actor_state(state.name,canonical.source_precision())
            .map_err(|e|aa_actors::actor_creation::Error::World(e.to_string()))?;
        let inventory=self.inventory;
        crate::load_actor_registration::Reentry::state_reenter(
            &mut crate::load_actor_state_reentry::Context::with_fx(inventory,self),catalog,record,
            u32::from(state.index),source,pc)
    }
}
impl crate::load_post_create_world::Effects for RequiredEffects<'_> {
    fn initial_property(&mut self,catalog:&Catalog<'_>,record:&mut Record<'_>,property:crate::load_post_create_world::InitialProperty,pc:Precision)->Result<()> {
        crate::load_property_instances::initial(catalog,record,&mut *self.properties,&property,
            ||current_property_time(catalog),pc,&mut PropertyEffects {variables:&mut *self.variables,
                inventory:self.inventory,environment:catalog.runtime_environment})
    }
    fn created_fx(&mut self,catalog:&Catalog<'_>,record:&mut Record<'_>,_sources:&[Vec<aa_rules::resources::Provenance>],_pc:Precision)->Result<()> {
        let mut execution=catalog.load_execution.try_borrow_mut().map_err(|_|"load execution already borrowed")?;
        let execution=execution.as_deref_mut().ok_or("Created FX requires actual running resource worker")?;
        crate::load_creation_fx::publish(catalog,record,execution,self.presentation,1)
    }
    fn owner_add_actor(&mut self,catalog:&Catalog<'_>,record:&mut Record<'_>,owner:ActorId,pc:Precision)->Result<()> {
        crate::load_owner_add_actor::add_actor(catalog,self.inventory,record,owner,pc)
    }
}
struct SunServices<'a,'s> {
    catalog:&'a Catalog<'s>,
    registrations:&'a mut Vec<aa_mapworld::restore_components::UpdatableRegistration>,
}
impl crate::sun_state::World for SunServices<'_,'_> {
    fn cancel_scheduled(&mut self,head:&mut Option<u64>)->crate::sun_state::Result<()> {
        let owner=self.catalog.load_owners.try_borrow_mut().map_err(sun_error)?
            .restoring(aa_mapworld::restore_components::UpdatableTarget::Sun).map_err(sun_error)?;
        let mut queues=self.catalog.load_queues.try_borrow_mut().map_err(sun_error)?;
        let queues=queues.as_deref_mut().ok_or_else(||sun_error("Sun actual world queue owner is absent"))?;
        if *head!=queues.owner_head(owner).map(|id|id.raw()) {
            return Err(sun_error("Sun retained cancellation head and actual queue links differ"));
        }
        queues.cancel_owner(owner).map_err(sun_error)?;
        *head=None;
        Ok(())
    }
    fn table_word0(&mut self)->crate::sun_state::Result<i32> {
        aa_mapworld::restore::RestoreDefinitions::updatable_table_bound(self.catalog).map_err(sun_error)
    }
    fn slot_occupied(&mut self,id:i32)->crate::sun_state::Result<bool> {
        aa_mapworld::restore::RestoreDefinitions::updatable_slot_occupied(self.catalog,id).map_err(sun_error)
    }
    fn register_updatable(&mut self,id:i32)->crate::sun_state::Result<()> {
        use aa_mapworld::restore_components::{UpdatableRegistration,UpdatableTarget};
        aa_mapworld::restore::RestoreDefinitions::register_updatable(self.catalog,id,&UpdatableTarget::Sun).map_err(sun_error)?;
        self.registrations.push(UpdatableRegistration {serialized_id:id,target:UpdatableTarget::Sun});
        Ok(())
    }
}
fn sun_error(value:impl std::fmt::Display)->aa_actors::organization_state::LoadError {
    aa_actors::organization_state::LoadError::World(value.to_string())
}
struct MiscServices<'a,'s> {catalog:&'a Catalog<'s>,source_pc:Precision}
impl MiscServices<'_,'_> {
    fn reference(&self,domain:&str,ids:&[u16])->crate::saved_misc::Result<Option<u32>> {
        let definition=aa_mapworld::restore::RestoreDefinitions::resolve_named_definition(self.catalog,domain,ids,self.source_pc).map_err(sun_error)?;
        definition.map(|definition|self.catalog.definition_bank.try_borrow_mut().map_err(sun_error)?
            .intern(domain,&definition.id).map_err(sun_error)).transpose()
    }
}
impl crate::saved_misc::World for MiscServices<'_,'_> {
    fn name_list(&mut self,ids:&[u16])->crate::saved_misc::Result<Option<u32>> {self.reference("NameList",ids)}
    fn weather(&mut self,ids:&[u16])->crate::saved_misc::Result<Option<u32>> {self.reference("Weather",ids)}
    fn warn_unknown_name_list(&mut self,_:&[u16])->crate::saved_misc::Result<()> {
        Err(aa_actors::organization_state::LoadError::Unsupported("MISC unknown NameList native warning sink is unbound".into()))
    }
}
struct RegistrationEffects<'a> {grid:crate::load_actor_registration::GridContext<'a>,inventory:&'a WorldInventory}
impl live_registration::Effects for RegistrationEffects<'_> {
    fn register_actor(&mut self,id:ActorId,record:&mut RestoredRecord,registry:&mut aa_actors::IdentityRegistry<RestoredRecord>,catalog:&Catalog<'_>,pc:Precision)->Result<()> {
        let grid=crate::load_actor_registration::GridContext { zones:&mut *self.grid.zones,
            spatial:self.grid.spatial,point:self.grid.point,session:self.grid.session,world:self.grid.world };
        crate::load_actor_registration::register(catalog,id,record,registry,grid,&mut crate::load_actor_state_reentry::Context::new(self.inventory),pc).map_err(|e|e.to_string())
    }
    fn assert_duplicate_available_city(&mut self)->Result<bool> {
        Err("4F4C40 duplicate available-city assertion requires actual process diagnostic service".into())
    }
}

impl crate::load_actor_state_reentry::FxEffects for RequiredEffects<'_> {
    fn enter_fx(&mut self,catalog:&Catalog<'_>,record:&mut Record<'_>,matched:crate::fx_graph::MatchedState,
        _pc:Precision)->aa_actors::actor_creation::Result<()> {
        crate::load_actor_state_fx::publish(catalog,record,matched,&mut *self.presentation,&mut || {
            Ok(crate::fx_static_position::unset_position())
        }).map_err(aa_actors::actor_creation::Error::World)
    }
}
