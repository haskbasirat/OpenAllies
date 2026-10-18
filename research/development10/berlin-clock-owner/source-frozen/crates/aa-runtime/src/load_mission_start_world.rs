//! Actual4FC600 tail ownership and the following managed-resource boundary.
//!
//! LoadedMap owns the same Staged graph by value. It is not a battle-ready flag:
//! resource return and StartMission remain subsequent ordered operations.
use crate::{catalog::Catalog, load_tail_world::SessionRead, restoration::Staged};
use aa_mapworld::{class_conversion as conversion, restore::RuntimeDefinition};
use aa_numeric::Precision;

fn requires_start_mission(load_mode: u32, started48: u32) -> Result<bool, String> {
    match load_mode {
        1 | 4 => Ok(started48 == 0),
        2 => Ok(false),
        _ => Err("post-resource task mode has no native mission branch".into()),
    }
}

/// A failed operation retains the actual graph and every preceding mutation.
pub struct Failure {
    pub staged: Staged,
    pub error: String,
}

pub struct LoadedMap {
    staged: Staged,
    resource: crate::map_resource_load::Owner,
    mission_attempted:bool,
}

/// Native post-load mission branch and same-resource assignment completed on
/// the SAME loaded graph. The constructor is private. Outer task/pre-load
/// application producers and later Goal SAI/application activation remain
/// separately owned operations; this is not an application-ready certificate.
pub struct StartedMission {
    loaded: LoadedMap,
}
impl StartedMission {
    pub fn staged(&self) -> &Staged {
        self.loaded.staged()
    }
    /// Requires the actual selected descriptor that constructed this returned
    /// resource. This preserves its source allocation through worker handoff.
    pub fn validate_selected_creator(
        &self, selected: &crate::resource_creator::Creator,
    ) -> Result<(), String> {
        self.loaded.resource.require_selected_origin(selected)
    }
    /// Requires the same sealed task inputs captured before worker execution.
    pub fn validate_task_inputs(
        &self, inputs: &crate::load_game_task::Inputs,
    ) -> Result<(), String> {
        self.loaded.resource.require_task_inputs(inputs)
    }
    /// Application activation consumes this once after its actual owner has
    /// completed. This transfer does not itself perform that activation.
    pub(crate) fn into_staged(mut self) -> Result<Staged, Self> {
        let Some(world) = self.loaded.staged.load_world.as_mut() else {
            return Err(self);
        };
        if world.resource_load.is_some() {
            return Err(self);
        }
        //Keep the actual resource/phase/prior-word/archive witness alive beside
        //the SAME mode Box through application activation and live frames.
        world.resource_load = Some(self.loaded.resource);
        Ok(self.loaded.staged)
    }
}

/// Runs the same existing native load-tail kernel, then the existing conversion
/// kernel over its ACTUAL queue. Positive conversion creation remains required.
/// Validation errors precede the first native operation and claim no SEH parity.
pub fn finish_load<S: SessionRead + ?Sized>(
    mut staged: Staged,
    catalog: &Catalog<'_>,
    session: &S,
    precision: Precision,
) -> Result<LoadedMap, Failure> {
    if let Err(error) = finish_load_inner(&mut staged, catalog, session, precision) {
        return Err(Failure { staged, error });
    }
    let Some(resource) = staged
        .load_world
        .as_mut()
        .and_then(|w| w.resource_load.take())
    else {
        return Err(Failure {
            staged,
            error: "load tail lost its actual MapResource owner".into(),
        });
    };
    Ok(LoadedMap { staged, resource,mission_attempted:false })
}

fn finish_load_inner<S: SessionRead + ?Sized>(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    session: &S,
    precision: Precision,
) -> Result<(), String> {
    let world = staged
        .load_world
        .as_ref()
        .ok_or("load tail actual World is absent")?;
    world.validate_loaded_sections(catalog, precision)?;
    world
        .resource_load
        .as_ref()
        .ok_or("load tail actual MapResource companion is absent")?
        .validate_loading(world.map_mode.as_ref(), &catalog.actors.sources)?;
    world
        .resource_load
        .as_ref()
        .ok_or("load tail actual MapResource companion is absent")?
        .validate_creator(catalog, &staged.definitions)?;
    if !staged.report.lifecycle_detached.is_empty()
        || staged.report.lifecycle.iter().any(|e| {
            e.mode != aa_mapworld::restore_lifecycle::Mode::Immediate
                || !e.attempted
                || !e.completed
                || e.omitted
                || e.error.is_some()
                || e.calls.iter().any(|c| !c.completed || c.error.is_some())
        })
    {
        return Err("load tail requires actual completed immediate record callbacks".into());
    }
    crate::load_tail_binding::after_post_load(staged, catalog, session, precision)?;
    // Ownership transfer only: the completed load index and its exact nodes
    // become the placement owner used by conversion and mission preparation.
    // No native insertion, terrain query, or registration is repeated here.
    let world = staged.load_world.as_mut().ok_or("load tail lost actual World before spatial transfer")?;
    match &world.spatial {
        crate::load_world::SpatialOwner::Loading(Some(_)) => world.adopt_spatial(&mut staged.actors)?,
        crate::load_world::SpatialOwner::Adopted(_) => { world.live_spatial()?; },
        crate::load_world::SpatialOwner::Loading(None) => return Err("load tail spatial transfer is incomplete".into()),
    }
    if !staged.report.conversions.is_empty() {
        return Err(
            "4FD47C conversion tail requires real4F9C10 fresh creation; retained queue is nonempty"
                .into(),
        );
    }
    // Empty is a fact read from the same conversion owner. Running the kernel
    // takes its native no-iteration branch; no create callback is fabricated.
    let queue = std::mem::take(&mut staged.report.conversions);
    conversion::replace_converted(queue, &mut RequiredConversion).map_err(|e| e.error)?;
    staged.load_world.as_mut().ok_or("load tail lost actual World before temporary table retirement")?
        .updatable_lifetime.finish_map_load_50b500()?;
    Ok(())
}

impl LoadedMap {
    pub fn staged(&self) -> &Staged {
        &self.staged
    }

    ///Actual5A3ED0 task branch and ordered4FDCE0 mission prefix. On failure,
    ///the same loaded graph/resource and earlier effects are returned. A saved
    ///already-started World follows the native skip branch; an unstarted World
    ///cannot obtain StartedMission while any reached child remains required.
    pub fn start_mission<S:SessionRead+?Sized>(
        mut self,
        catalog: &Catalog<'_>,
        application: &crate::application_mode::GameAppState,
        startup: &crate::actor_creation_startup::StartupInputs,
        session:&S,
        sync: &mut crate::live_clocked::SyncLog<'_>,
        sim_sync:&mut crate::live_clocked::SyncLog<'_>,
        precision: Precision,
    ) -> Result<StartedMission, (Self, String)> {
        match self.start_mission_inner(catalog, application, startup, session, sync, sim_sync, precision) {
            Ok(()) => Ok(StartedMission { loaded: self }),
            Err(error) => Err((self, error)),
        }
    }

    fn start_mission_inner<S:SessionRead+?Sized>(
        &mut self,
        catalog: &Catalog<'_>,
        application: &crate::application_mode::GameAppState,
        startup: &crate::actor_creation_startup::StartupInputs,
        session:&S,
        sync: &mut crate::live_clocked::SyncLog<'_>,
        sim_sync:&mut crate::live_clocked::SyncLog<'_>,
        precision: Precision,
    ) -> Result<(), String> {
        if self.mission_attempted{return Err("mission startup previously failed; same partial owners retained and cannot be published by retrying the World48 skip branch".into())}
        self.mission_attempted=true;
        self.validate_resource_returned(catalog)?;
        self.staged.load_world.as_ref().ok_or("mission actual World absent before table-lifetime check")?
            .updatable_lifetime.require_returned()?;
        self.resource
            .validate_creator(catalog, &self.staged.definitions)?;
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or("mission actual World absent")?;
        world.mission_fields.require_loaded()?;
        let actual_pc = match &world.spatial {
            crate::load_world::SpatialOwner::Loading(Some(v)) => v.index().precision(),
            crate::load_world::SpatialOwner::Adopted(v) => v.world.index().precision(),
            _ => return Err("mission actual spatial owner is between transfers".into()),
        };
        if actual_pc != precision {
            return Err("mission arithmetic precision differs from the actual World".into());
        }
        //5A3ED0 tests actual World+48 before calling4FDCE0.4F02F0 then tests
        //the SAME MapResource at World+28; retained mode allocation/witness
        //validation above proves this load's unchanged-resource branch.
        //5A3E85..5A3EA2's mode2 task never calls StartMission. Modes1/4
        //use the explicit World48 conditional at5A3ED0/5A3F40 instead.
        if !requires_start_mission(self.resource.load_mode(), world.mission_fields.started48())? {
            return Ok(());
        }
        if (world.mission_fields.mode44() as i32) < 2 {
            return Err("4FDCE0 unverified map requires actual status/localization/4E4BC0 warning services before MergeKingdoms".into());
        }
        crate::merge_kingdoms_world::run(&mut self.staged, catalog, sync)?;
        let world = self.staged.load_world.as_ref().unwrap();
        //4A5BC0 walks current World+148, skipping holes in that actual vector.
        //4E6DB0's writes target these SAME held SpecialShell allocations.
        for id in world.kingdoms.all().iter().flatten() {
            let key = self
                .staged
                .actors
                .key(*id)
                .ok_or("mission Kingdom slot has no live GTID")?;
            let entity = self
                .staged
                .actors
                .entity_mut(key)
                .map_err(|e| e.to_string())?;
            let crate::actor_world::Source::Special(value) = &mut entity.source else {
                return Err("mission Kingdom slot is not its actual special allocation".into());
            };
            let crate::actor_world::SpecialSource::Kingdom(saved) = &value.source else {
                return Err("mission Kingdom slot is a different native class".into());
            };
            if saved.serialized_id != id.raw() {
                return Err("mission Kingdom GTID/source identity differs".into());
            }
            let shell = value
                .derived
                .as_mut()
                .ok_or("mission Kingdom derived owner absent")?;
            crate::kingdom_mission_start::start_existing(
                catalog,
                &self.staged.definitions,
                &world.variables,
                shell,
                precision,
            )?;
        }
        if application.mode() == 4 {
            return Err(
                "4FDCE0 editor mode requires actual938960/9389A0 before map preparation".into(),
            );
        }
        crate::map_prepare_world::run(&mut self.staged,catalog,startup,sync,sim_sync,precision).map_err(|e|e.to_string())?;
        let world=self.staged.load_world.as_mut().ok_or("mission World lost after map preparation")?;
        let visible=match world.variables.fog_integer(){0=>Some(true),1=>Some(false),_=>None};
        if let Some(visible)=visible {
            let phase=BorrowedPhase(session);
            let services=crate::load_zone_services::Context::with_kingdoms(&world.point,&phase,&world.kingdoms)
                .with_source_fields(catalog,&world.source_fields)
                .with_clock(&self.staged.clock);
            match &mut world.spatial {
                crate::load_world::SpatialOwner::Loading(Some(v))=>
                    crate::load_store_fog::reveal_all(catalog,&self.staged.definitions,&mut self.staged.actors,
                        &mut world.zones,&world.zone_globals,v.index(),&services,&world.kingdoms,visible),
                crate::load_world::SpatialOwner::Adopted(v)=>
                    crate::load_store_fog::reveal_all_published(catalog,&self.staged.definitions,&mut self.staged.actors,
                        &mut world.zones,&world.zone_globals,&mut v.world,self.staged.inactive_load.as_ref(),&services,&world.kingdoms,visible),
                _=>return Err("mission global Fog spatial owner is between transfers".into()),
            }.map_err(|e|e.to_string())?;
        }
        crate::mission_camera_world::run(&mut self.staged,catalog,session,precision)?;
        crate::mission_world_start::run(&mut self.staged,catalog,precision)?;
        crate::mission_finalize_world::run(&mut self.staged,catalog,precision)
    }

    /// The normal resource-return boundary is AFTER4FC600 and BEFORE5A3ED0.
    /// Earlier close/flag effects remain committed if a later service fails.
    pub fn finish_resource_load(&mut self, catalog: &Catalog<'_>) -> Result<(), String> {
        let world = self
            .staged
            .load_world
            .as_mut()
            .ok_or("loaded map World owner is absent")?;
        self.resource
            .validate_loading(world.map_mode.as_ref(), &catalog.actors.sources)?;
        let streams = self
            .staged
            .resource_streams
            .as_mut()
            .ok_or("loaded map actual owned resource streams are absent")?;
        self.resource.close_owned_streams(streams)?;
        self.resource
            .finish_normal_return(world.map_mode.as_mut(), &catalog.actors.sources)
    }

    pub fn resource_prior_word(&self) -> u32 {
        self.resource.prior_word()
    }
    pub fn resource_load_mode(&self) -> u32 {
        self.resource.load_mode()
    }
    pub fn resource_flags(&self) -> u32 {
        self.resource.flags()
    }

    pub(crate) fn validate_resource_returned(&self, catalog: &Catalog<'_>) -> Result<(), String> {
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or("loaded map World owner is absent")?;
        self.resource
            .validate_returned(world.map_mode.as_ref(), &catalog.actors.sources)
    }
}
///A live borrowed Session interface, without copying its phase/state words.
struct BorrowedPhase<'a,S:?Sized>(&'a S);
impl<S:crate::load_world_globals::SessionPhase+?Sized> crate::load_world_globals::SessionPhase for BorrowedPhase<'_,S> {
    fn current_phase(&self)->Result<u32,String>{self.0.current_phase()}
    fn application_state(&self)->Result<u32,String>{self.0.application_state()}
}

struct RequiredConversion;
impl conversion::VariationSource<RuntimeDefinition> for RequiredConversion {
    type Error = String;
    fn is_object_data(&mut self, _: &RuntimeDefinition) -> Result<bool, String> {
        Err("positive conversion needs actual ObjectData dispatch".into())
    }
    fn variations_enabled(&mut self, _: &RuntimeDefinition) -> Result<bool, String> {
        Err("positive conversion needs actual ObjectData+108".into())
    }
    fn aspect_count(&mut self, _: &RuntimeDefinition) -> Result<i32, String> {
        Err("positive conversion needs actual ObjectData+110".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_post_resource_task_modes_and_loaded_start_word() {
        let f: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-mission-start/native-mission-prefix.json"
        ))
        .unwrap();
        for row in f["task_gates"].as_array().unwrap() {
            let c = &row["input"];
            let actual = requires_start_mission(
                c["mode"].as_u64().unwrap() as u32,
                c["started48"].as_u64().unwrap() as u32,
            )
            .unwrap();
            let called = row["output"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "4FDCE0_boundary");
            assert_eq!(actual, called, "{row}");
            assert_eq!(row["output"]["world_resource_unchanged"], true);
        }
        assert!(requires_start_mission(0, 0).is_err());
        assert!(requires_start_mission(3, 0).is_err());
    }
}
impl conversion::ReplacementWorld<RuntimeDefinition> for RequiredConversion {
    type Created = aa_actors::ActorId;
    fn create_thing(
        &mut self,
        _: &conversion::ObjectCreationParams<RuntimeDefinition>,
    ) -> Result<Option<Self::Created>, String> {
        Err("positive conversion needs actual4F9C10 creation/publication".into())
    }
}
