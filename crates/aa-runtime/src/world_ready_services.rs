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
    loading: crate::application_loading::State,
    depot: resource_depot::Application,
    film: film_variables::State,
    glyph: player_glyph::State,
    formatter: load_kingdom_glyph::Formatter,
    sources: Vec<SourceIdentity>,
    startup: crate::actor_creation_startup::StartupInputs,
    svar_inventory: crate::inactive_svar_dump::Inventory,
    frame_variables: crate::frame_variables::State,
    view_inputs: Option<crate::view_startup_inputs::Inputs>,
}
///A failed initialization keeps the SAME application and all constructed
///prefix owners. It cannot be converted into an initialized/active bundle.
pub struct Failure {
    pub error: String,
    application: GameAppState,
    loading: crate::application_loading::State,
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
    ///Optional only before the actual source-created View inputs are installed.
    ///A reached consumer must require this same current AVar owner.
    pub orientation: Option<&'a crate::view_renderer_variables::State>,
}
///A disjoint loan of the same retained Loading allocation and view source
///owners. The established general Parts API is unchanged.
pub struct ViewStartupParts<'a> {
    pub loading: &'a mut crate::application_loading::State,
    pub inputs: &'a mut crate::view_startup_inputs::Inputs,
    pub depot: &'a mut resource_depot::Application,
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
            loading: crate::application_loading::State::from_native_constructor(),
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
            partial.depot.install_fresh_cache_variable_scope(catalog,overrides)?;
            {
                //Model5A94F0/5A9840 consume the actual completed canonical
                //Bone directory. Bind it once before the Bank/Resources move
                //to the worker and before any Model allocation is published.
                let bank=catalog.definition_bank.try_borrow()
                    .map_err(|_| "Model source-context installation Bank is already borrowed")?;
                partial.depot.prepare_model_source_context(catalog,&bank)?;
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
            loading: partial.loading,
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
            view_inputs: None,
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
    ///The SAME Application+E4 Loading allocation from40CB40→682DE0. This
    ///is neither the GameAppState returned by40D960 nor a page widget.
    pub fn loading(&self) -> &crate::application_loading::State { &self.loading }
    pub fn loading_mut(&mut self) -> &mut crate::application_loading::State { &mut self.loading }
    ///Install once after the actual completed source pass. A rejected incoming
    ///owner is returned whole; neither the existing owner nor its cells reset.
    pub fn install_view_inputs(
        &mut self,
        catalog: &Catalog<'_>,
        inputs: crate::view_startup_inputs::Inputs,
    ) -> Result<(), crate::view_startup_inputs::InstallFailure> {
        let checked = (|| -> Result<(), String> {
            if self.view_inputs.is_some() {
                return Err("application view startup inputs are already installed".into());
            }
            self.validate(catalog)?;
            let bank = catalog.definition_bank.try_borrow()
                .map_err(|_| "view startup installation source Bank is already borrowed")?;
            inputs.validate(catalog, &bank)
        })();
        if let Err(error) = checked {
            return Err(crate::view_startup_inputs::InstallFailure { inputs, error });
        }
        self.view_inputs = Some(inputs);
        Ok(())
    }
    pub fn view_inputs(&self) -> Result<&crate::view_startup_inputs::Inputs, String> {
        self.view_inputs.as_ref().ok_or_else(|| "application view startup inputs are unbound".into())
    }
    pub fn view_startup_parts(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
    ) -> Result<ViewStartupParts<'_>, String> {
        self.validate(catalog)?;
        self.view_inputs()?.validate(catalog, bank)?;
        Ok(ViewStartupParts {
            loading: &mut self.loading,
            inputs: self.view_inputs.as_mut().expect("validated retained owner"),
            depot: &mut self.depot,
        })
    }
    ///Actual5C3104 task publication binds the SAME application Loading17 and
    ///ResourceManager queue to the SAME World/VWorld AC allocation. The slot
    ///moves only after native5AED10 successfully adopted its queue reference.
    pub fn enqueue_view_resource_task<G>(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,
        world:&crate::world_revision::ConstructedWorldRevision,view:&crate::view_init_prefix::State<G>,
        slot:&mut Option<crate::view_resource_task::Task>)->Result<(),String>{
        self.validate(catalog)?;
        if bank.combat_source()?.source_identities()!=catalog.actors.sources.as_slice(){return Err("View task enqueue source Bank differs".into())}
        if !view.initialized_for(world)?{return Err("View resource task publication requires actual completed43F0A0 AD".into())}
        let task=slot.as_ref().ok_or("actual View task publication slot is null")?;
        task.bind_callback(world,view,&self.loading)?;
        self.depot.enqueue_view_task_5aed10(task)?;
        drop(slot.take());Ok(())
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
            orientation: self.view_inputs.as_ref().map(|inputs|inputs.orientation()),
        })
    }
}
