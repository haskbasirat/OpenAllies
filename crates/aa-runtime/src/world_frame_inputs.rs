//! The genuine StartedMission's same graph, leased to concrete world-ready
//!services. This type alone grants no application activation or tick capability.
use crate::{
    actor_world::{ActorStore, ActorStoreWitness},
    catalog::Catalog,
    definitions::Bank,
    load_mission_start_world::StartedMission,
    load_world::SpatialOwner,
    localization::Localization,
    resource_creator::Creator,
    restoration::Staged,
    restore_lifecycle::StartupOwner,
    world_revision::ConstructedWorldRevision,
};
use aa_rules::resources::SourceIdentity;
use std::sync::Arc;
type Result<T> = std::result::Result<T, String>;

enum SaiPhase {
    Before,
    Running,
    Complete,
    Stopped(String),
}
///No Clone, Default, rawStaged constructor or caller-supplied ready flag.
pub struct WorldFrameInputs {
    staged: Staged,
    store: ActorStoreWitness,
    revision_origin: Arc<()>,
    revision: u32,
    sources: Vec<SourceIdentity>,
    sai: SaiPhase,
    application: Option<StartupOwner>,
}
///Disjoint loans only. NativeBeginPlaying mutates these actual owners; this
///is not a copied Session/World snapshot and it mints no completion receipt.
pub struct BeginPlayers<'a> {
    pub world: crate::world_revision::Witness,
    pub actors: &'a mut ActorStore,
    pub kingdom_slots: &'a [Option<aa_actors::ActorId>],
    pub live_text: &'a mut Localization,
    pub bank: &'a Bank,
    pub creator: &'a mut Creator,
    pub ai: AiPlayers<'a>,
    pub standalone: &'a crate::standalone_network::Standalone,
}
///Read-only graph fields reached by76C310. A retained MapTextOrder error is
///not read until the actual4738C0 operation inside the concrete output.
pub struct RecordingParts<'a> {
    pub bank: &'a Bank,
    pub actors: &'a ActorStore,
    pub live_text: &'a Localization,
    pub map_text: &'a std::result::Result<crate::film_text::MapTextOrder, String>,
}
///Disjoint actual owners for the capture roster to borrow the SAME retained
///Standalone while the caller lends the other World fields to5A26E0.
pub struct FilmCaptureParts<'a> {
    pub bank: &'a Bank,
    pub actors: &'a ActorStore,
    pub live_text: &'a mut Localization,
    pub world: &'a mut crate::load_world::World,
    pub ai_manager: &'a aa_mind::sai::manager::Sai,
    pub ai_players: &'a crate::load_sai_players::Arena,
    pub standalone: &'a crate::standalone_network::Standalone,
}
///Concrete loan of the sole application SAI/Arena pair. Network callers do
///not need a reverse runtime dependency or a second AI owner to call766960.
pub struct AiPlayers<'a> {
    manager: &'a mut aa_mind::sai::manager::Sai,
    profiles: &'a mut crate::load_sai_players::Arena,
}
impl AiPlayers<'_> {
    pub fn add_context<'a, 'source>(
        &'a mut self,
        catalog: &'a Catalog<'source>,
        bank: &'a Bank,
        actors: &'a ActorStore,
    ) -> crate::load_sai_players::AddContext<'a, 'source> {
        crate::load_sai_players::AddContext::new(catalog, bank, actors, self.manager, self.profiles)
    }
}
impl WorldFrameInputs {
    pub fn from_started(
        started: StartedMission,
    ) -> std::result::Result<Self, (StartedMission, String)> {
        let preflight = (|| {
            let staged = started.staged();
            if !matches!(staged.native_load_mode, Some(1 | 4)) {
                return Err("live world-ready requires actual normal/alternate mission task, not editor skip".to_owned());
            }
            let world = staged
                .load_world
                .as_ref()
                .ok_or("StartedMission World owner absent")?;
            world.mission_fields.require_loaded()?;
            if world.mission_fields.started48() == 0 {
                return Err(
                    "StartedMission World48 has not reached native mission completion".into(),
                );
            }
            staged
                .inactive_load
                .as_ref()
                .ok_or("StartedMission actual inactive application owner absent")?
                .observer_present()?;
            staged.definitions.combat_source()?.completed_means()?;
            Ok(())
        })();
        if let Err(error) = preflight {
            return Err((started, error));
        }
        let staged = started.into_staged().map_err(|value| {
            (
                value,
                "StartedMission lost its same returned MapResource companion".into(),
            )
        })?;
        let world = staged
            .load_world
            .as_ref()
            .expect("preflight retains private same World");
        let store = staged.actors.identity_witness();
        let revision_origin = Arc::clone(world.revision.owner_identity());
        let revision = world.revision.value();
        let sources = staged
            .definitions
            .combat_source()
            .expect("preflight retains same Bank")
            .source_identities()
            .to_vec();
        Ok(Self {
            staged,
            store,
            revision_origin,
            revision,
            sources,
            sai: SaiPhase::Before,
            application: None,
        })
    }
    pub fn staged(&self) -> &Staged {
        &self.staged
    }
    pub fn revision(&self) -> &ConstructedWorldRevision {
        &self
            .staged
            .load_world
            .as_ref()
            .expect("private retained World")
            .revision
    }
    fn validate(&self, catalog: &Catalog<'_>) -> Result<()> {
        catalog.require_source_owner()?;
        if !self.store.is_for(&self.staged.actors) {
            return Err("world-ready ActorStore allocation was replaced".into());
        }
        if self.sources != catalog.actors.sources {
            return Err("world-ready Catalog source universe differs".into());
        }
        let source = self.staged.definitions.combat_source()?;
        source.completed_means()?;
        if source.source_identities() != self.sources {
            return Err("world-ready same source Bank differs".into());
        }
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or("world-ready retained World absent")?;
        if world.revision.value() != self.revision
            || !Arc::ptr_eq(world.revision.owner_identity(), &self.revision_origin)
        {
            return Err("world-ready actual constructor revision owner differs".into());
        }
        let resource = world
            .resource_load
            .as_ref()
            .ok_or("world-ready actual returned resource absent")?;
        resource.validate_returned(world.map_mode.as_ref(), &self.sources)?;
        resource.validate_creator(catalog, &self.staged.definitions)
    }
    pub fn begin_players_parts(&mut self, catalog: &Catalog<'_>) -> Result<BeginPlayers<'_>> {
        self.validate(catalog)?;
        if !matches!(self.sai, SaiPhase::Before) {
            return Err("BeginPlaying owner is past its pre-SAI phase".into());
        }
        let staged = &mut self.staged;
        let world = staged
            .load_world
            .as_mut()
            .ok_or("BeginPlaying World absent")?;
        let (manager, profiles, standalone) = staged
            .inactive_load
            .as_mut()
            .ok_or("BeginPlaying application interval absent")?
            .pre_start_parts_with_network()?;
        let creator = world
            .resource_load
            .as_mut()
            .ok_or("BeginPlaying returned resource absent")?
            .creator_mut_for_world_ready(catalog, &staged.definitions)?;
        Ok(BeginPlayers {
            world: world.revision.witness(),
            actors: &mut staged.actors,
            kingdom_slots: world.kingdoms.majors(),
            live_text: &mut staged.live_text,
            bank: &staged.definitions,
            creator,
            ai: AiPlayers { manager, profiles },
            standalone,
        })
    }
    ///Same current World graph, lent only at the native pre-SAI film phase.
    ///The OS time input is sampled lazily by6FB1F0, not by this loan.
    pub fn film_capture_context<'a, 'assets, P, C>(
        &'a mut self,
        catalog: &'a Catalog<'assets>,
        application: &'a crate::application_mode::GameAppState,
        formatter: &'a mut crate::load_kingdom_glyph::Formatter,
        glyph_variables: &'a mut crate::player_glyph::State,
        players: &'a mut P,
        current_local_filetime: C,
    ) -> Result<crate::film_world_capture::Context<'a, 'assets, P, C>> {
        self.validate(catalog)?;
        if !matches!(self.sai, SaiPhase::Before) {
            return Err("film capture is outside native pre-SAI phase".into());
        }
        let staged = &mut self.staged;
        let (ai_manager, ai_players) = staged
            .inactive_load
            .as_mut()
            .ok_or("film capture actual inactive application owner absent")?
            .pre_start_parts()?;
        Ok(crate::film_world_capture::Context {
            catalog,
            bank: &staged.definitions,
            world: staged
                .load_world
                .as_mut()
                .ok_or("film capture World absent")?,
            actors: &staged.actors,
            live_text: &mut staged.live_text,
            application,
            ai_manager,
            ai_players,
            formatter,
            glyph_variables,
            players,
            current_local_filetime,
        })
    }
    pub fn recording_parts(&self, catalog: &Catalog<'_>) -> Result<RecordingParts<'_>> {
        self.validate(catalog)?;
        if !matches!(self.sai, SaiPhase::Before) {
            return Err("film preamble is outside native pre-SAI phase".into());
        }
        Ok(RecordingParts {
            bank: &self.staged.definitions,
            actors: &self.staged.actors,
            live_text: &self.staged.live_text,
            map_text: &self
                .staged
                .load_world
                .as_ref()
                .ok_or("film preamble World absent")?
                .map_text,
        })
    }
    pub fn film_capture_parts(&mut self, catalog: &Catalog<'_>) -> Result<FilmCaptureParts<'_>> {
        self.validate(catalog)?;
        if !matches!(self.sai, SaiPhase::Before) {
            return Err("film capture is outside native pre-SAI phase".into());
        }
        let staged = &mut self.staged;
        let (ai_manager, ai_players, standalone) = staged
            .inactive_load
            .as_mut()
            .ok_or("film capture actual inactive application owner absent")?
            .pre_start_parts_with_network()?;
        Ok(FilmCaptureParts {
            bank: &staged.definitions,
            actors: &staged.actors,
            live_text: &mut staged.live_text,
            world: staged
                .load_world
                .as_mut()
                .ok_or("film capture World absent")?,
            ai_manager,
            ai_players,
            standalone,
        })
    }
    ///Only this direct concrete7A0D40 return may complete this phase. The
    ///ready byte is never consulted as a completion certificate. Failed native
    ///prefixes remain in their same manager/map and cannot be retried here.
    pub fn initialize_sai(&mut self, catalog: &Catalog<'_>) -> Result<()> {
        self.validate(catalog)?;
        if !matches!(self.sai, SaiPhase::Before) {
            return Err("world-ready SAI startup already attempted".into());
        }
        let world = self.staged.load_world.as_ref().ok_or("SAI World absent")?;
        match &world.spatial {
            SpatialOwner::Adopted(value) if value.error.is_none() && value.pending.is_empty() => {}
            _ => return Err("SAI startup requires completed same spatial adoption".into()),
        }
        let mut inactive = self
            .staged
            .inactive_load
            .take()
            .ok_or("SAI actual inactive application owner absent")?;
        self.sai = SaiPhase::Running;
        let staged = &mut self.staged;
        let world = staged.load_world.as_mut().expect("private retained World");
        let SpatialOwner::Adopted(spatial) = &world.spatial else {
            unreachable!()
        };
        let outcome = inactive.with_startup_engine_and_players(|manager, profiles| {
            crate::load_sai_startup::initialize(
                manager,
                profiles,
                catalog,
                &staged.definitions,
                &staged.inventory,
                &mut staged.actors,
                &spatial.world,
                &world.kingdoms,
                &world.zones,
                &mut world.sai_map,
                &world.revision,
            )
        });
        match inactive.into_startup_owner() {
            Ok(application) => self.application = Some(application),
            Err(inactive) => {
                self.staged.inactive_load = Some(inactive);
                let error = "SAI startup did not retire its actual inactive interval".to_owned();
                self.sai = SaiPhase::Stopped(error.clone());
                return Err(error);
            }
        }
        match outcome {
            Ok(()) => {
                self.sai = SaiPhase::Complete;
                Ok(())
            }
            Err(error) => {
                self.sai = SaiPhase::Stopped(error.clone());
                Err(error)
            }
        }
    }
    pub fn require_sai_complete(&self, catalog: &Catalog<'_>) -> Result<()> {
        self.validate(catalog)?;
        match &self.sai {
            SaiPhase::Complete if self.application.is_some() => Ok(()),
            SaiPhase::Stopped(error) => Err(format!("SAI startup stopped: {error}")),
            _ => Err("same World7A0D40 has not completed".into()),
        }
    }
    ///Diagnostic access retains partial application state after an error;
    ///its presence is NOT an application/world-ready certificate.
    pub fn retired_application(&self) -> Option<&StartupOwner> {
        self.application.as_ref()
    }
    ///Current process-global warning text at native7738B0, after the genuine
    ///successful task/SAI path. This is distinct from local World+A0 warnings.
    pub fn success_warning_text(&self, catalog: &Catalog<'_>) -> Result<Vec<u16>> {
        self.require_sai_complete(catalog)?;
        self.application
            .as_ref()
            .ok_or("successful World application owner absent")?
            .success_warning_text()
    }
    pub fn into_sai_complete_parts(
        self,
        catalog: &Catalog<'_>,
    ) -> std::result::Result<(Staged, StartupOwner), (Self, String)> {
        if let Err(error) = self.require_sai_complete(catalog) {
            return Err((self, error));
        }
        Ok((
            self.staged,
            self.application
                .expect("private concrete SAI completion retains application"),
        ))
    }
}
