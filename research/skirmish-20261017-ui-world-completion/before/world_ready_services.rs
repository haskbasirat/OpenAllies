//! One move-only application's current world-ready services. These are real
//! retained source-created variables and resource owners, not a ready receipt.
use crate::{
    application_mode::GameAppState, catalog::Catalog, film_process_metadata, film_variables,
    load_kingdom_glyph, load_view_variables::NativeOverrides, player_glyph, resource_depot,
};
pub use aa_rules::actors::ActorCatalog as SourceCatalog;
use aa_rules::{resources::SourceIdentity, sources::V14Sources};
use std::path::Path;

pub struct Application {
    application: GameAppState,
    depot: resource_depot::Application,
    film: film_variables::State,
    glyph: player_glyph::State,
    formatter: load_kingdom_glyph::Formatter,
    sources: Vec<SourceIdentity>,
    startup: crate::actor_creation_startup::StartupInputs,
    svar_inventory: crate::inactive_svar_dump::Inventory,
    frame_variables: crate::frame_variables::State,
}
///A failed initialization keeps the SAME application and all constructed
///prefix owners. It cannot be converted into an initialized/active bundle.
pub struct Failure {
    pub error: String,
    application: GameAppState,
    depot: resource_depot::Application,
    film: Option<film_variables::State>,
    glyph: Option<player_glyph::State>,
    startup: crate::actor_creation_startup::StartupInputs,
    svar_inventory: Option<crate::inactive_svar_dump::Inventory>,
    frame_variables: Option<crate::frame_variables::State>,
}
impl Failure {
    pub fn application(&self) -> &GameAppState {
        &self.application
    }
    pub fn depot(&self) -> &resource_depot::Application {
        &self.depot
    }
    pub fn film_variables(&self) -> Option<&film_variables::State> {
        self.film.as_ref()
    }
    pub fn glyph_variables(&self) -> Option<&player_glyph::State> {
        self.glyph.as_ref()
    }
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldReadyServiceFailure")
            .field("error", &self.error)
            .field("film_variables", &self.film.is_some())
            .field("glyph_variables", &self.glyph.is_some())
            .finish()
    }
}
///Disjoint loans of the actual retained fields. No owner is reconstructed at
///a selected-map, callback or frame boundary.
pub struct Parts<'a> {
    pub application: &'a mut GameAppState,
    pub depot: &'a mut resource_depot::Application,
    pub film: &'a mut film_variables::State,
    pub glyph: &'a mut player_glyph::State,
    pub formatter: &'a mut load_kingdom_glyph::Formatter,
    pub startup: &'a crate::actor_creation_startup::StartupInputs,
    pub svar_inventory: &'a crate::inactive_svar_dump::Inventory,
    pub frame_variables: &'a crate::frame_variables::State,
}
impl Application {
    ///Fresh explicit no-profile application. The portable output directory
    ///and embedded-only MAPSTRINGS choice are deliberate rewrite backend
    ///configuration, not claimed observations of another native process.
    pub fn from_fresh_sources(
        catalog: &Catalog<'_>,
        files: V14Sources,
        application: GameAppState,
        startup: crate::actor_creation_startup::StartupInputs,
        application_root: &Path,
        overrides: NativeOverrides,
    ) -> Result<Self, Failure> {
        let mut partial = Failure {
            error: String::new(),
            application,
            depot: resource_depot::Application::from_sources(files),
            film: None,
            glyph: None,
            startup,
            svar_inventory: None,
            frame_variables: None,
        };
        let result = (|| -> Result<(), String> {
            catalog.require_source_owner()?;
            if !matches!(overrides, NativeOverrides::ControlledNone) {
                return Err("world-ready application profile/console owner is unbound".into());
            }
            partial.svar_inventory = Some(
                crate::inactive_svar_dump::Inventory::from_captured_source(catalog)?,
            );
            partial.frame_variables = Some(crate::frame_variables::State::from_fresh_sources(
                catalog,
                partial.depot.sources_mut(),
                NativeOverrides::ControlledNone,
            )?);
            for expected in &catalog.actors.sources {
                let actual = partial
                    .depot
                    .sources_mut()
                    .read(&expected.path)
                    .map_err(|e| e.to_string())?;
                if actual.logical_path != expected.path
                    || actual.sha256 != expected.sha256
                    || actual.origin != expected.origin
                    || aa_rules::sources::sha256(&actual.bytes) != expected.sha256
                {
                    return Err(format!(
                        "world-ready source backend differs at {}",
                        expected.path
                    ));
                }
            }
            let process = film_process_metadata::State::from_fresh_sources(
                catalog,
                partial.depot.sources_mut(),
                NativeOverrides::ControlledNone,
            )?;
            partial
                .depot
                .attach_process_metadata(process)
                .map_err(|(_, error)| error)?;
            partial.film = Some(film_variables::State::from_fresh_sources(
                catalog,
                partial.depot.sources_mut(),
                NativeOverrides::ControlledNone,
            )?);
            partial.glyph = Some(player_glyph::State::from_fresh_sources(
                catalog,
                partial.depot.sources_mut(),
                NativeOverrides::ControlledNone,
            )?);
            partial
                .depot
                .configure_portable_work_depot(application_root)?;
            //A real retained UVar write; no default declaration is changed.
            partial
                .film
                .as_mut()
                .expect("constructed field")
                .assign_separate_map_strings_byte(0);
            Ok(())
        })();
        if let Err(error) = result {
            partial.error = error;
            return Err(partial);
        }
        Ok(Self {
            application: partial.application,
            depot: partial.depot,
            film: partial.film.expect("constructed field"),
            glyph: partial.glyph.expect("constructed field"),
            formatter: load_kingdom_glyph::Formatter::from_fresh_application(),
            sources: catalog.actors.sources.clone(),
            startup: partial.startup,
            svar_inventory: partial
                .svar_inventory
                .expect("constructed registration inventory"),
            frame_variables: partial
                .frame_variables
                .expect("constructed frame variables"),
        })
    }
    pub fn validate(&self, catalog: &Catalog<'_>) -> Result<(), String> {
        catalog.require_source_owner()?;
        if self.sources != catalog.actors.sources {
            return Err("world-ready services belong to another canonical source universe".into());
        }
        self.depot.process_metadata()?;
        self.frame_variables.validate(catalog)?;
        Ok(())
    }
    pub fn application(&self) -> &GameAppState {
        &self.application
    }
    pub fn startup(&self) -> &crate::actor_creation_startup::StartupInputs {
        &self.startup
    }
    pub fn frame_variables(&self) -> &crate::frame_variables::State {
        &self.frame_variables
    }
    pub fn frame_variables_mut(&mut self) -> &mut crate::frame_variables::State {
        &mut self.frame_variables
    }
    pub fn application_mut(&mut self) -> &mut GameAppState {
        &mut self.application
    }
    pub fn depot(&self) -> &resource_depot::Application {
        &self.depot
    }
    pub fn depot_mut(&mut self) -> &mut resource_depot::Application {
        &mut self.depot
    }
    pub fn header_constructor_counter(
        &self,
    ) -> Result<film_process_metadata::HeaderConstructorCounter, String> {
        Ok(self.depot.process_metadata()?.constructor_counter())
    }
    pub fn parts(&mut self, catalog: &Catalog<'_>) -> Result<Parts<'_>, String> {
        self.validate(catalog)?;
        Ok(Parts {
            application: &mut self.application,
            depot: &mut self.depot,
            film: &mut self.film,
            glyph: &mut self.glyph,
            formatter: &mut self.formatter,
            startup: &self.startup,
            svar_inventory: &self.svar_inventory,
            frame_variables: &self.frame_variables,
        })
    }
}
